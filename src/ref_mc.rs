//! Brute-force reference model checker (Phase 6 task 3, D-P6-5 / D-P6-10).
//!
//! Independent ground-truth engine for differential testing (Phase 6 tasks
//! 4.1/5.2) and user-facing rapid prototyping (`rfs mc-ref`). It implements
//! `docs/SEMANTICS.md` sections 2-9 directly, with DELIBERATELY DIFFERENT
//! data structures and algorithms from the generated engines:
//!
//! - plain `Vec` frontier + `HashMap<CfgKey, _>` visited set; configuration
//!   keys are structured tuples (state index, variable values, monitor
//!   persistent states) - NO bit-packing;
//! - its own expression evaluator and its own online monitor stepper compiled
//!   from the IR formula (same documented semantics as `src/monitor.rs`,
//!   different code);
//! - representative inputs computed independently from the IR (the rule of
//!   SEMANTICS.md section 9.3, reimplemented - not imported from
//!   `src/mc_plan.rs`);
//! - liveness via SCC analysis of its own product graph (iterative Tarjan +
//!   self-loop check), different from the generated engine's BFS + stay-edge
//!   subgraph + Kosaraju construction.
//!
//! Independence is about ALGORITHMS, not report formatting: per D-P6-10 this
//! module reuses `src/codegen_mc.rs` DATA TYPES (status taxonomy, JSON report
//! model) and its pure formatting/parsing helpers (`exact_summary_line`,
//! `parse_assume_selection`) so that `rfs mc-ref` output is directly
//! comparable - byte-for-byte where deterministic - with the generated MC
//! binaries' output. It mirrors the generated engines' DEFAULT exact mode:
//! deterministic first-match transition firing, seedless exhaustive search.
//!
//! Import list (independence proof, per task file "Other information"):
//! `std::{collections,time}`, `crate::ir::*`, and
//! `crate::codegen_mc::{exact_summary_line, parse_assume_selection,
//! ExactStatus, FailReason, JsonCounterexample, JsonIo, JsonStep,
//! McJsonReport, EXACT_DEFAULT_MAX_STATES, EXACT_DEFAULT_TIMEOUT_SEC}`
//! (data types + shared report helpers only). The unit tests additionally use
//! `crate::json_rw::Json`. NOT imported: `mc_plan`, `monitor`,
//! `codegen_fsm`, `printer`, backend modules - enforced by the
//! `independence_import_audit` test.

use std::collections::{HashMap, HashSet};
use std::time::Instant;

use crate::codegen_mc::{
    exact_summary_line, parse_assume_selection, ExactStatus, FailReason, JsonCounterexample,
    JsonIo, JsonStep, McJsonReport, EXACT_DEFAULT_MAX_STATES, EXACT_DEFAULT_TIMEOUT_SEC,
};
use crate::ir::*;

// ---------------------------------------------------------------------------
// Values (IR-level value representation)
// ---------------------------------------------------------------------------

/// One concrete value. The width/enum context lives in the declaring type;
/// `Bit` values are unsigned and always reduced mod 2^w by the evaluator.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Val {
    Bool(bool),
    Bit(u64),
    /// Enum variant name (the declaring enum is fixed by context).
    Enum(String),
}

impl Val {
    fn num(&self) -> Option<u64> {
        match self {
            Val::Bit(v) => Some(*v),
            Val::Bool(b) => Some(*b as u64),
            Val::Enum(_) => None,
        }
    }
}

/// Type of an expression subterm (this module's own typing, mirroring the
/// rules documented in SEMANTICS.md section 2.2).
#[derive(Debug, Clone, PartialEq, Eq)]
enum RType {
    Bool,
    Bit(u32),
    Enum(String),
    /// Integer literal (adapts to the other operand; bare literals are bit<8>).
    IntLit,
}

impl RType {
    /// Bit width for arithmetic/bitwise results (`IntLit` counts as bit<8>,
    /// matching the validator's rules).
    fn width(&self) -> Option<u32> {
        match self {
            RType::Bit(w) => Some(*w),
            RType::IntLit => Some(8),
            _ => None,
        }
    }
}

fn ir_to_rtype(ty: &Type) -> RType {
    match ty {
        Type::Bool => RType::Bool,
        Type::Bit { width } => RType::Bit(*width),
        Type::Enum { name } => RType::Enum(name.clone()),
    }
}

fn state_variants(module: &Module) -> &[String] {
    module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.as_slice())
        .unwrap_or_default()
}

/// Type of one expression (own implementation of the documented typing rules;
/// validated specs always type-check, so `Err` here marks an internal
/// invariant failure).
fn expr_type(e: &Expr, module: &Module, binds: &HashMap<String, Type>) -> Result<RType, String> {
    match e {
        Expr::Int { .. } => Ok(RType::IntLit),
        Expr::Bool { .. } => Ok(RType::Bool),
        Expr::Path { path } => {
            if path.is_empty() {
                return Err("empty path".into());
            }
            if path.len() == 1 {
                let name = &path[0];
                if name == "state" {
                    return Ok(RType::Enum("State".into()));
                }
                if let Some(ty) = binds.get(name) {
                    return Ok(ir_to_rtype(ty));
                }
                if let Some(v) = module.vars.iter().find(|v| &v.name == name) {
                    return Ok(ir_to_rtype(&v.ty));
                }
                if state_variants(module).iter().any(|s| s == name) {
                    return Ok(RType::Enum("State".into()));
                }
                return Err(format!("unknown identifier '{name}'"));
            }
            let enum_name = &path[0];
            let variant = path
                .get(1)
                .ok_or_else(|| "enum path too short".to_string())?;
            let variants = module
                .enums
                .iter()
                .find(|en| en.name == *enum_name)
                .map(|en| en.variants.clone())
                .ok_or_else(|| format!("unknown enum type '{enum_name}'"))?;
            if !variants.contains(variant) {
                return Err(format!("unknown variant '{variant}' in enum '{enum_name}'"));
            }
            if path.len() > 2 {
                return Err(format!("unsupported path length {}", path.len()));
            }
            Ok(RType::Enum(enum_name.clone()))
        }
        Expr::Unary { op, expr } => {
            let ty = expr_type(expr, module, binds)?;
            match op {
                UnaryOp::Not => {
                    if !matches!(ty, RType::Bool) {
                        return Err("operand of ! must be boolean".into());
                    }
                    Ok(RType::Bool)
                }
                UnaryOp::BitNot | UnaryOp::Neg => match ty {
                    RType::Bit(w) => Ok(RType::Bit(w)),
                    RType::IntLit => Ok(RType::Bit(8)),
                    _ => Err("bit_not/neg requires a bitvector operand".into()),
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = expr_type(left, module, binds)?;
            let r = expr_type(right, module, binds)?;
            binary_type(*op, &l, &r)
        }
    }
}

fn compatible(l: &RType, r: &RType) -> Result<(), String> {
    match (l, r) {
        (RType::IntLit, RType::Bit(_)) | (RType::Bit(_), RType::IntLit) => Ok(()),
        (a, b) if a == b => Ok(()),
        _ => Err(format!("type mismatch: {l:?} vs {r:?}")),
    }
}

fn bit_op_type(l: &RType, r: &RType) -> Result<RType, String> {
    let w = match (l, r) {
        (RType::Bit(a), RType::Bit(b)) => {
            if a != b {
                return Err(format!("bitwidth mismatch: bit<{a}> vs bit<{b}>"));
            }
            *a
        }
        (RType::IntLit, RType::Bit(b)) | (RType::Bit(b), RType::IntLit) => *b,
        (RType::IntLit, RType::IntLit) => 8,
        _ => return Err("bitwise/arithmetic operator requires bitvector operands".into()),
    };
    Ok(RType::Bit(w))
}

fn binary_type(op: BinOp, l: &RType, r: &RType) -> Result<RType, String> {
    match op {
        BinOp::LogicAnd | BinOp::LogicOr => {
            if !matches!(l, RType::Bool) || !matches!(r, RType::Bool) {
                return Err("logical operator requires boolean operands".into());
            }
            Ok(RType::Bool)
        }
        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            compatible(l, r)?;
            Ok(RType::Bool)
        }
        BinOp::Plus | BinOp::Minus | BinOp::Star => bit_op_type(l, r),
        BinOp::ShiftLeft | BinOp::ShiftRight => {
            let w = match l {
                RType::Bit(w) => *w,
                RType::IntLit => 8,
                _ => return Err("shift left operand must be a bitvector".into()),
            };
            if matches!(r, RType::Bool | RType::Enum(_)) {
                return Err("shift right operand must be a bitvector or integer literal".into());
            }
            Ok(RType::Bit(w))
        }
        BinOp::BitAnd | BinOp::BitOr => {
            if matches!(l, RType::Bool) && matches!(r, RType::Bool) {
                return Ok(RType::Bool);
            }
            bit_op_type(l, r)
        }
        BinOp::BitXor => bit_op_type(l, r),
    }
}

// ---------------------------------------------------------------------------
// Expression evaluation (own evaluator; wrapping semantics per SEMANTICS 2.2)
// ---------------------------------------------------------------------------

/// Evaluation environment: current/post-step control state, the variable
/// valuation, and the pattern bindings of the matched input.
#[derive(Clone)]
pub struct EvalCtx<'a> {
    pub module: &'a Module,
    /// Current (guard/assignment: OLD step) or post-step (property atoms)
    /// control state, as an enum value.
    pub state: Val,
    /// Module variable values (name -> value).
    pub vars: HashMap<String, Val>,
    /// Pattern binding values (bind name -> value).
    pub binds: HashMap<String, Val>,
    /// Types of the pattern bindings in scope (for typing).
    pub bind_types: HashMap<String, Type>,
}

fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// Evaluate an RFS expression to a concrete value.
fn eval_expr(e: &Expr, ctx: &EvalCtx) -> Result<Val, String> {
    let ty = expr_type(e, ctx.module, &ctx.bind_types)?;
    match e {
        Expr::Int { value } => Ok(Val::Bit(*value)),
        Expr::Bool { value } => Ok(Val::Bool(*value)),
        Expr::Path { path } => {
            if path.len() == 1 {
                let name = &path[0];
                if name == "state" {
                    return Ok(ctx.state.clone());
                }
                if let Some(v) = ctx.binds.get(name) {
                    return Ok(v.clone());
                }
                if let Some(v) = ctx.vars.get(name) {
                    return Ok(v.clone());
                }
                if state_variants(ctx.module).iter().any(|s| s == name) {
                    return Ok(Val::Enum(name.clone()));
                }
                return Err(format!("unknown identifier '{name}'"));
            }
            // Enum path: the variant name is the value (the enum type is fixed
            // by context; typing above already validated the path).
            Ok(Val::Enum(path[1].clone()))
        }
        Expr::Unary { op, expr } => {
            let v = eval_expr(expr, ctx)?;
            match op {
                UnaryOp::Not => match v {
                    Val::Bool(b) => Ok(Val::Bool(!b)),
                    other => Err(format!("! requires boolean, got {other:?}")),
                },
                UnaryOp::BitNot => {
                    let w = ty.width().ok_or("bit_not width")?;
                    let n = v.num().ok_or("bit_not operand not numeric")?;
                    Ok(Val::Bit(!n & mask(w) as u64))
                }
                UnaryOp::Neg => {
                    let w = ty.width().ok_or("neg width")?;
                    let n = v.num().ok_or("neg operand not numeric")?;
                    Ok(Val::Bit(((0u128.wrapping_sub(n as u128)) & mask(w)) as u64))
                }
            }
        }
        Expr::Binary { op, left, right } => {
            let l = eval_expr(left, ctx)?;
            let r = eval_expr(right, ctx)?;
            match op {
                BinOp::LogicAnd => Ok(Val::Bool(bool_of(&l)? && bool_of(&r)?)),
                BinOp::LogicOr => Ok(Val::Bool(bool_of(&l)? || bool_of(&r)?)),
                BinOp::Eq | BinOp::NotEq => {
                    let eq = val_eq(&l, &r)?;
                    Ok(Val::Bool(matches!(op, BinOp::Eq) == eq))
                }
                BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                    let a = l.num().ok_or("comparison operand not numeric")?;
                    let b = r.num().ok_or("comparison operand not numeric")?;
                    let c = match op {
                        BinOp::Lt => a < b,
                        BinOp::Le => a <= b,
                        BinOp::Gt => a > b,
                        _ => a >= b,
                    };
                    Ok(Val::Bool(c))
                }
                BinOp::Plus | BinOp::Minus | BinOp::Star => {
                    let w = ty.width().ok_or("arith width")?;
                    let a = l.num().ok_or("arith operand not numeric")?;
                    let b = r.num().ok_or("arith operand not numeric")?;
                    let res = match op {
                        BinOp::Plus => (a as u128).wrapping_add(b as u128),
                        BinOp::Minus => (a as u128).wrapping_sub(b as u128),
                        _ => (a as u128).wrapping_mul(b as u128),
                    };
                    Ok(Val::Bit((res & mask(w)) as u64))
                }
                BinOp::ShiftLeft | BinOp::ShiftRight => {
                    let w = expr_type(left, ctx.module, &ctx.bind_types)?
                        .width()
                        .ok_or("shift width")?;
                    let a = l.num().ok_or("shift operand not numeric")?;
                    let b = r.num().ok_or("shift amount not numeric")? as u32;
                    // SEMANTICS 2.2: shift by >= width yields 0; >> is logical.
                    let res = if b >= w {
                        0u128
                    } else if matches!(op, BinOp::ShiftLeft) {
                        (a as u128) << b
                    } else {
                        (a as u128) >> b
                    };
                    Ok(Val::Bit((res & mask(w)) as u64))
                }
                BinOp::BitAnd | BinOp::BitOr | BinOp::BitXor => {
                    if matches!(ty, RType::Bool) {
                        // On bool operands &/| act as logical connectives.
                        let a = bool_of(&l)?;
                        let b = bool_of(&r)?;
                        return Ok(Val::Bool(match op {
                            BinOp::BitAnd => a && b,
                            _ => a || b,
                        }));
                    }
                    let w = ty.width().ok_or("bitwise width")?;
                    let a = l.num().ok_or("bitwise operand not numeric")?;
                    let b = r.num().ok_or("bitwise operand not numeric")?;
                    let res = match op {
                        BinOp::BitAnd => a & b,
                        BinOp::BitOr => a | b,
                        _ => a ^ b,
                    };
                    Ok(Val::Bit(res & mask(w) as u64))
                }
            }
        }
    }
}

fn bool_of(v: &Val) -> Result<bool, String> {
    match v {
        Val::Bool(b) => Ok(*b),
        other => Err(format!("expected boolean, got {other:?}")),
    }
}

/// Full-value equality (enums by variant identity; typing guarantees the
/// operands are comparable).
fn val_eq(l: &Val, r: &Val) -> Result<bool, String> {
    match (l, r) {
        (Val::Bool(a), Val::Bool(b)) => Ok(a == b),
        (Val::Bit(a), Val::Bit(b)) => Ok(a == b),
        (Val::Enum(a), Val::Enum(b)) => Ok(a == b),
        _ => Err(format!("cannot compare {l:?} with {r:?}")),
    }
}

// ---------------------------------------------------------------------------
// Per-sample signals (property atoms) - own representation
// ---------------------------------------------------------------------------

/// A temporal-operator-free signal over one sample (own type; same domain as
/// the documented atom set of SEMANTICS.md section 5.1).
#[derive(Debug, Clone)]
enum Sig {
    True,
    /// Boolean RFS expression over the post-step state + variables.
    Var(Expr),
    InputIs(EventPattern),
    OutputHas(OutputPattern),
    Not(Box<Sig>),
    And(Box<Sig>, Box<Sig>),
    Or(Box<Sig>, Box<Sig>),
}

/// One flattened input event instance (variant index + field values).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct InputVal {
    /// Index into the flattened input variant list (declaration order).
    pub variant: usize,
    /// Field values in declaration order.
    pub fields: Vec<Val>,
}

/// One emitted output instance.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct OutputVal {
    pub variant: usize,
    pub fields: Vec<Val>,
}

/// One sample of a run (SEMANTICS.md section 4): position 0 is the initial
/// configuration (no input/outputs); later samples are post-step.
#[derive(Debug, Clone)]
pub struct Sample {
    pub input: Option<InputVal>,
    pub outputs: Vec<OutputVal>,
    /// Post-step control state index.
    pub to_state: usize,
    /// Post-step variable valuation (declaration order).
    pub vars: Vec<Val>,
}

impl Sample {
    fn eval_ctx<'a>(&self, module: &'a Module) -> EvalCtx<'a> {
        let mut vars = HashMap::new();
        for (v, val) in module.vars.iter().zip(self.vars.iter()) {
            vars.insert(v.name.clone(), val.clone());
        }
        let state = match &module.enums.iter().find(|e| e.name == "State") {
            Some(en) => Val::Enum(en.variants[self.to_state].clone()),
            None => Val::Enum("?".into()),
        };
        EvalCtx {
            module,
            state,
            vars,
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        }
    }
}

/// Evaluate a signal at one sample.
fn eval_sig(sig: &Sig, s: &Sample, module: &Module) -> Result<bool, String> {
    match sig {
        Sig::True => Ok(true),
        Sig::Var(e) => {
            let ctx = s.eval_ctx(module);
            bool_of(&eval_expr(e, &ctx)?)
        }
        Sig::InputIs(p) => match &s.input {
            Some(inp) => Ok(input_matches(module, p, inp)?),
            None => Ok(false), // sample 0: no input consumed
        },
        Sig::OutputHas(p) => Ok(s.outputs.iter().any(|o| output_matches(module, p, o))),
        Sig::Not(a) => Ok(!eval_sig(a, s, module)?),
        Sig::And(a, b) => Ok(eval_sig(a, s, module)? && eval_sig(b, s, module)?),
        Sig::Or(a, b) => Ok(eval_sig(a, s, module)? || eval_sig(b, s, module)?),
    }
}

// ---------------------------------------------------------------------------
// Pattern matching (own implementation of SEMANTICS.md section 3.1)
// ---------------------------------------------------------------------------

/// Flattened input/output variant (own reimplementation of the flattening
/// rules of SEMANTICS.md section 2.3).
#[derive(Debug, Clone)]
pub struct FlatVariant {
    pub group: String,
    pub variant: String,
    /// Collision-free flattened constructor name.
    pub unique_name: String,
    /// (field name, field type) in declaration order.
    pub fields: Vec<(String, Type)>,
}

fn flatten(groups: &[ConstructorGroup]) -> Vec<FlatVariant> {
    let mut out = Vec::new();
    let mut used: HashSet<String> = HashSet::new();
    for group in groups {
        for v in &group.variants {
            let (vname, fields) = match v {
                ConstructorVariant::Unit { name } => (name.clone(), vec![]),
                ConstructorVariant::Struct { name, fields } => (
                    name.clone(),
                    fields
                        .iter()
                        .map(|f| (f.name.clone(), f.ty.clone()))
                        .collect(),
                ),
            };
            let base = if group.name == vname && group.variants.len() == 1 {
                group.name.clone()
            } else {
                format!("{}_{}", group.name, vname)
            };
            let mut unique = base.clone();
            let mut n = 2;
            while !used.insert(unique.clone()) {
                unique = format!("{base}{n}");
                n += 1;
            }
            out.push(FlatVariant {
                group: group.name.clone(),
                variant: vname,
                unique_name: unique,
                fields,
            });
        }
    }
    out
}

/// Does `pattern` match the concrete input? Returns Ok(true/false); binding
/// names are ignored here (they constrain nothing).
fn input_matches(module: &Module, pattern: &EventPattern, inp: &InputVal) -> Result<bool, String> {
    let variants = flatten(&module.events);
    let vi = &variants[inp.variant];
    if vi.group != pattern.group {
        return Ok(false);
    }
    match &pattern.variant {
        None => Ok(true), // bare group pattern: every variant of the group
        Some(PatternVariant::Unit { name }) => Ok(vi.variant == *name),
        Some(PatternVariant::Struct { name, fields }) => {
            if vi.variant != *name || vi.fields.len() != fields.len() {
                return Ok(false);
            }
            for (((_, fty), pf), val) in vi.fields.iter().zip(fields.iter()).zip(inp.fields.iter())
            {
                let ok = match &pf.pattern {
                    PatternExpr::Wildcard => true,
                    PatternExpr::Bind { .. } => true,
                    PatternExpr::Int { value } => matches!(val, Val::Bit(v) if v == value),
                    PatternExpr::Bool { value } => matches!(val, Val::Bool(b) if b == value),
                    PatternExpr::EnumVariant { path } => enum_match(val, fty, path),
                };
                if !ok {
                    return Ok(false);
                }
            }
            Ok(true)
        }
    }
}

/// Enum-variant field pattern: the full path must name the field's enum type
/// and its variant (the generated engines compare the whole qualified value).
fn enum_match(val: &Val, fty: &Type, path: &[String]) -> bool {
    match (val, fty) {
        (Val::Enum(v), Type::Enum { name }) => path.len() == 2 && path[0] == *name && v == &path[1],
        _ => false,
    }
}

fn output_matches(module: &Module, pattern: &OutputPattern, out: &OutputVal) -> bool {
    let variants = flatten(&module.outputs);
    let vi = &variants[out.variant];
    if vi.group != pattern.group {
        return false;
    }
    match &pattern.variant {
        None => true,
        Some(PatternVariant::Unit { name }) => vi.variant == *name,
        Some(PatternVariant::Struct { name, fields }) => {
            vi.variant == *name
                && vi.fields.len() == fields.len()
                && vi
                    .fields
                    .iter()
                    .zip(fields.iter())
                    .zip(out.fields.iter())
                    .all(|(((_, fty), pf), val)| match &pf.pattern {
                        PatternExpr::Wildcard | PatternExpr::Bind { .. } => true,
                        PatternExpr::Int { value } => matches!(val, Val::Bit(v) if v == value),
                        PatternExpr::Bool { value } => matches!(val, Val::Bool(b) if b == value),
                        PatternExpr::EnumVariant { path } => enum_match(val, fty, path),
                    })
        }
    }
}

// ---------------------------------------------------------------------------
// Online monitor compiler + stepper (own implementation of SEMANTICS 5)
// ---------------------------------------------------------------------------

/// Anchor range: samples at which a check is evaluated (`to = None` =
/// unbounded from `from`). Own type (the semantics are documented in
/// SEMANTICS.md section 5).
#[derive(Debug, Clone, Copy)]
struct Range {
    from: u64,
    to: Option<u64>,
}

impl Range {
    fn all() -> Self {
        Self { from: 0, to: None }
    }
    fn contains(&self, s: u64) -> bool {
        s >= self.from && self.to.is_none_or(|t| s <= t)
    }
    /// Shift the anchor window by one sample (`next` semantics).
    fn shifted(&self) -> Range {
        Range {
            from: self.from + 1,
            to: self.to.map(|t| t + 1),
        }
    }
}

/// One node of a compiled monitor (own type; same shapes the documented
/// semantics require).
#[derive(Debug, Clone)]
enum Mon {
    /// `expr` must hold at every sample in `range`.
    Check {
        expr: Sig,
        range: Range,
    },
    /// `antecedent(i) -> consequent(i+1)` for anchors in `range`, with a
    /// one-sample delay register (starts false).
    Delayed {
        ant: Sig,
        cons: Sig,
        range: Range,
    },
    /// For every anchor i in `range` where `trigger(i)` holds, `body` must
    /// hold within `bound` samples (bound >= 1); bound == 0: at i itself.
    Obligation {
        bound: u64,
        trig: Sig,
        body: Sig,
        range: Range,
    },
    And(Vec<Mon>),
    Or(Vec<Mon>),
}

/// Compile a formula into a monitor. `Err` = outside the monitor subset
/// (R1-R4 + liveness operators) - identical classification to the generated
/// engines' compiler (the rules are documented in SEMANTICS.md section 5.5).
fn compile_formula(f: &Formula) -> Result<Mon, String> {
    compile_check(f, Range::all(), false)
}

fn compile_signal(f: &Formula) -> Option<Sig> {
    match f {
        Formula::Expr { expr } => Some(Sig::Var(expr.clone())),
        Formula::InputIs { pattern } => Some(Sig::InputIs(pattern.clone())),
        Formula::OutputHas { pattern } => Some(Sig::OutputHas(pattern.clone())),
        Formula::Not { inner } => Some(Sig::Not(Box::new(compile_signal(inner)?))),
        Formula::And { left, right } => Some(Sig::And(
            Box::new(compile_signal(left)?),
            Box::new(compile_signal(right)?),
        )),
        Formula::Or { left, right } => Some(Sig::Or(
            Box::new(compile_signal(left)?),
            Box::new(compile_signal(right)?),
        )),
        Formula::Implies { left, right } => Some(Sig::Or(
            Box::new(Sig::Not(Box::new(compile_signal(left)?))),
            Box::new(compile_signal(right)?),
        )),
        _ => None,
    }
}

fn compile_check(f: &Formula, range: Range, in_temporal: bool) -> Result<Mon, String> {
    match f {
        Formula::BoundedEventually { bound, inner } => {
            let body = compile_signal(inner).ok_or_else(|| {
                "bounded_eventually body must be temporal-operator-free".to_string()
            })?;
            let anchor = if in_temporal {
                range
            } else {
                Range {
                    from: 0,
                    to: Some(0),
                }
            };
            Ok(Mon::Obligation {
                bound: *bound,
                trig: Sig::True,
                body,
                range: anchor,
            })
        }
        Formula::Implies { left, right } => match right.as_ref() {
            Formula::BoundedEventually { bound, inner } => {
                let trig = compile_signal(left).ok_or_else(|| {
                    "implication antecedent must be temporal-operator-free".to_string()
                })?;
                let body = compile_signal(inner).ok_or_else(|| {
                    "bounded_eventually body must be temporal-operator-free".to_string()
                })?;
                let anchor = if in_temporal {
                    range
                } else {
                    Range {
                        from: 0,
                        to: Some(0),
                    }
                };
                Ok(Mon::Obligation {
                    bound: *bound,
                    trig,
                    body,
                    range: anchor,
                })
            }
            Formula::Next { inner } => {
                let ant = compile_signal(left).ok_or_else(|| {
                    "implication antecedent must be temporal-operator-free".to_string()
                })?;
                let cons = compile_signal(inner)
                    .ok_or_else(|| "next body must be temporal-operator-free".to_string())?;
                Ok(Mon::Delayed {
                    ant,
                    cons,
                    range: range.shifted(),
                })
            }
            _ => {
                let l = compile_signal(left).ok_or_else(|| {
                    "implication antecedent must be temporal-operator-free".to_string()
                })?;
                let r = compile_signal(right).ok_or_else(|| {
                    "implication consequent must be temporal-operator-free or bounded_eventually"
                        .to_string()
                })?;
                Ok(Mon::Check {
                    expr: Sig::Or(Box::new(Sig::Not(Box::new(l))), Box::new(r)),
                    range,
                })
            }
        },
        Formula::Always { inner } => compile_check(inner, range, true),
        Formula::BoundedAlways { bound, inner } => {
            if in_temporal {
                return Err("always[N] is not allowed under a temporal operator".into());
            }
            let n = *bound;
            if n < 1 {
                return Err("always[N] requires N >= 1".into());
            }
            compile_check(
                inner,
                Range {
                    from: 0,
                    to: Some(n - 1),
                },
                true,
            )
        }
        Formula::Next { inner } => {
            let s = compile_signal(inner)
                .ok_or_else(|| "next requires a temporal-operator-free body".to_string())?;
            Ok(Mon::Check {
                expr: s,
                range: range.shifted(),
            })
        }
        Formula::And { left, right } => Ok(Mon::And(vec![
            compile_check(left, range, in_temporal)?,
            compile_check(right, range, in_temporal)?,
        ])),
        Formula::Or { left, right } => Ok(Mon::Or(vec![
            compile_check(left, range, in_temporal)?,
            compile_check(right, range, in_temporal)?,
        ])),
        Formula::Not { inner } => {
            let s = compile_signal(inner).ok_or_else(|| {
                "negation of a temporal formula is outside the monitor subset".to_string()
            })?;
            Ok(Mon::Check {
                expr: Sig::Not(Box::new(s)),
                range,
            })
        }
        Formula::Eventually { .. } | Formula::Until { .. } => {
            Err("liveness operators (eventually/until) are outside the monitor subset".into())
        }
        _ => {
            let s = compile_signal(f)
                .ok_or_else(|| "formula is outside the monitor subset".to_string())?;
            Ok(Mon::Check { expr: s, range })
        }
    }
}

/// Runtime state of one monitor node (own type; mirrors the documented
/// pending-model semantics of SEMANTICS.md section 5.5).
#[derive(Debug, Clone)]
enum MonSt {
    Check {
        failed: bool,
    },
    Delayed {
        prev_ant: bool,
        failed: bool,
    },
    Obligation {
        pending: bool,
        remaining: u64,
        failed: bool,
        trigger_step: i64,
    },
    And {
        children: Vec<MonSt>,
        failed: bool,
        trigger_step: i64,
    },
    Or {
        children: Vec<MonSt>,
        failed: bool,
        trigger_step: i64,
    },
}

impl MonSt {
    fn trigger_step(&self) -> i64 {
        match self {
            MonSt::Obligation { trigger_step, .. }
            | MonSt::And { trigger_step, .. }
            | MonSt::Or { trigger_step, .. } => *trigger_step,
            _ => -1,
        }
    }
}

fn mon_init(spec: &Mon) -> MonSt {
    match spec {
        Mon::Check { .. } => MonSt::Check { failed: false },
        Mon::Delayed { .. } => MonSt::Delayed {
            prev_ant: false,
            failed: false,
        },
        Mon::Obligation { .. } => MonSt::Obligation {
            pending: false,
            remaining: 0,
            failed: false,
            trigger_step: -1,
        },
        Mon::And(children) => MonSt::And {
            children: children.iter().map(mon_init).collect(),
            failed: false,
            trigger_step: -1,
        },
        Mon::Or(children) => MonSt::Or {
            children: children.iter().map(mon_init).collect(),
            failed: false,
            trigger_step: -1,
        },
    }
}

/// Step one node on sample `s`. Returns the node's CURRENT-SAMPLE failure
/// status (per-sample, not sticky - essential for Or/And composition, where
/// disjuncts may fail and recover on different samples).
fn mon_step(
    st: &mut MonSt,
    spec: &Mon,
    s: u64,
    sample: &Sample,
    module: &Module,
) -> Result<bool, String> {
    match (st, spec) {
        (MonSt::Check { failed }, Mon::Check { expr, range }) => {
            let now = range.contains(s) && !eval_sig(expr, sample, module)?;
            if now {
                *failed = true;
            }
            Ok(now)
        }
        (MonSt::Delayed { prev_ant, failed }, Mon::Delayed { ant, cons, range }) => {
            if !*failed && range.contains(s) && *prev_ant && !eval_sig(cons, sample, module)? {
                *failed = true;
            }
            *prev_ant = eval_sig(ant, sample, module)?;
            Ok(*failed)
        }
        (
            MonSt::Obligation {
                pending,
                remaining,
                failed,
                trigger_step,
            },
            Mon::Obligation {
                bound,
                trig,
                body,
                range,
            },
        ) => {
            if !*failed {
                let a = range.contains(s) && eval_sig(trig, sample, module)?;
                let b = eval_sig(body, sample, module)?;
                if *bound == 0 {
                    if a && !b {
                        *failed = true;
                        *trigger_step = s as i64;
                    }
                } else {
                    // The documented pending model (re-trigger while pending is
                    // ignored: the pending obligation has the earliest deadline).
                    if !*pending && a {
                        *pending = true;
                        *remaining = *bound;
                        *trigger_step = s as i64;
                    }
                    if *pending {
                        if b {
                            *pending = false;
                        } else {
                            *remaining -= 1;
                            if *remaining == 0 {
                                *failed = true;
                            }
                        }
                    }
                }
            }
            Ok(*failed)
        }
        (
            MonSt::And {
                children,
                failed,
                trigger_step,
            },
            Mon::And(specs),
        ) => {
            let news: Vec<bool> = children
                .iter_mut()
                .zip(specs.iter())
                .map(|(c, sp)| mon_step(c, sp, s, sample, module))
                .collect::<Result<Vec<_>, _>>()?;
            let now = news.iter().any(|n| *n);
            if !*failed && now {
                *failed = true;
                *trigger_step = min_trigger(&news, children);
            }
            Ok(now)
        }
        (
            MonSt::Or {
                children,
                failed,
                trigger_step,
            },
            Mon::Or(specs),
        ) => {
            let news: Vec<bool> = children
                .iter_mut()
                .zip(specs.iter())
                .map(|(c, sp)| mon_step(c, sp, s, sample, module))
                .collect::<Result<Vec<_>, _>>()?;
            let now = !news.is_empty() && news.iter().all(|n| *n);
            if !*failed && now {
                *failed = true;
                *trigger_step = min_trigger(&news, children);
            }
            Ok(now)
        }
        _ => Err("monitor state/spec mismatch".into()),
    }
}

/// Earliest trigger step among the CURRENTLY failing children (-1 when none
/// carries one).
fn min_trigger(news: &[bool], children: &[MonSt]) -> i64 {
    news.iter()
        .zip(children.iter())
        .filter(|(n, _)| **n)
        .map(|(_, c)| c.trigger_step())
        .filter(|t| *t >= 0)
        .min()
        .unwrap_or(-1)
}

/// The persistent projection of one monitor's state - the part that belongs
/// in the configuration key (SEMANTICS.md section 9.2): obligation pending +
/// remaining (bound >= 1 only), delayed-check antecedent register. Preorder;
/// each entry is one u64 per persistent node.
fn mon_key(st: &MonSt, spec: &Mon) -> Vec<u64> {
    let mut out = Vec::new();
    mon_key_node(st, spec, &mut out);
    out
}

fn mon_key_node(st: &MonSt, spec: &Mon, out: &mut Vec<u64>) {
    match (st, spec) {
        (MonSt::Delayed { prev_ant, .. }, Mon::Delayed { .. }) => {
            out.push(*prev_ant as u64);
        }
        (
            MonSt::Obligation {
                pending, remaining, ..
            },
            Mon::Obligation { bound, .. },
        ) if *bound >= 1 => {
            // Injective encoding of (pending, remaining in 0..=bound).
            out.push(if *pending { 2 + 2 * remaining } else { 0 });
        }
        (MonSt::And { children, .. }, Mon::And(specs))
        | (MonSt::Or { children, .. }, Mon::Or(specs)) => {
            for (c, sp) in children.iter().zip(specs.iter()) {
                mon_key_node(c, sp, out);
            }
        }
        _ => {}
    }
}

/// Total persistent bits of one monitor (for the `monitor size` statistic -
/// the same widths the configuration encoding documents in SEMANTICS 9.2).
fn mon_bits(spec: &Mon) -> u32 {
    match spec {
        Mon::Delayed { .. } => 1,
        Mon::Obligation { bound, .. } if *bound >= 1 => 1 + ceil_log2(bound.saturating_add(1)),
        Mon::And(children) | Mon::Or(children) => children.iter().map(mon_bits).sum(),
        _ => 0,
    }
}

/// Smallest k with 2^k >= n (0 for n <= 1).
fn ceil_log2(n: u64) -> u32 {
    if n <= 1 {
        return 0;
    }
    let floor = 63 - n.leading_zeros();
    if n & (n - 1) == 0 {
        floor
    } else {
        floor + 1
    }
}

// ---------------------------------------------------------------------------
// Input abstraction: representative inputs (own reimplementation of
// SEMANTICS.md sections 9.3/9.4)
// ---------------------------------------------------------------------------

/// Per event variant: the set of field positions REFERENCED by guards, emit
/// arguments, assignment values (via used bindings), or `input_is` patterns
/// with constraining field patterns - the reference rule of section 9.3.
fn referenced_fields(module: &Module, variants: &[FlatVariant]) -> Vec<HashSet<usize>> {
    let mut refd: Vec<HashSet<usize>> = vec![HashSet::new(); variants.len()];

    // (a) Transitions: a bound field is referenced iff its binding name is
    // used in the guard, an emit argument, or an assignment value.
    for t in &module.transitions {
        let Some(PatternVariant::Struct { name, fields }) = &t.pattern.variant else {
            continue;
        };
        let Some(vi) = variants
            .iter()
            .position(|v| v.group == t.pattern.group && v.variant == *name)
        else {
            continue;
        };
        let mut bindings: Vec<(usize, String)> = Vec::new();
        for (i, pf) in fields.iter().enumerate() {
            if let PatternExpr::Bind { name } = &pf.pattern {
                bindings.push((i, name.clone()));
            }
        }
        if bindings.is_empty() {
            continue;
        }
        let used = used_identifiers(t);
        for (i, bname) in bindings {
            if i < variants[vi].fields.len() && used.contains(&bname) {
                refd[vi].insert(i);
            }
        }
    }

    // (b) Properties + assumptions: `input_is` patterns with CONSTRAINING
    // field patterns (wildcards and binds constrain nothing).
    let mut formulas: Vec<&Formula> = module.properties.iter().map(|p| &p.formula).collect();
    formulas.extend(module.assumptions.iter().map(|a| &a.formula));
    for f in formulas {
        collect_input_is_refs(f, variants, &mut refd);
    }

    refd
}

/// Single-segment path identifiers used in a transition's guard, emit args,
/// and assignment values.
fn used_identifiers(t: &Transition) -> HashSet<String> {
    let mut out = HashSet::new();
    if let Some(g) = &t.guard {
        collect_paths(g, &mut out);
    }
    for e in &t.emits {
        for a in &e.args {
            collect_paths(a, &mut out);
        }
    }
    for a in &t.assignments {
        collect_paths(&a.value, &mut out);
    }
    out
}

fn collect_paths(e: &Expr, out: &mut HashSet<String>) {
    match e {
        Expr::Path { path } if path.len() == 1 => {
            out.insert(path[0].clone());
        }
        Expr::Unary { expr, .. } => collect_paths(expr, out),
        Expr::Binary { left, right, .. } => {
            collect_paths(left, out);
            collect_paths(right, out);
        }
        _ => {}
    }
}

/// Walk a formula and mark fields constrained by `input_is` patterns.
fn collect_input_is_refs(f: &Formula, variants: &[FlatVariant], refd: &mut [HashSet<usize>]) {
    match f {
        Formula::InputIs { pattern } => {
            if let Some(PatternVariant::Struct { name, fields }) = &pattern.variant {
                if let Some(vi) = variants
                    .iter()
                    .position(|v| v.group == pattern.group && v.variant == *name)
                {
                    for (i, pf) in fields.iter().enumerate() {
                        let constraining = matches!(
                            pf.pattern,
                            PatternExpr::Int { .. }
                                | PatternExpr::Bool { .. }
                                | PatternExpr::EnumVariant { .. }
                        );
                        if constraining && i < variants[vi].fields.len() {
                            refd[vi].insert(i);
                        }
                    }
                }
            }
        }
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. } => collect_input_is_refs(inner, variants, refd),
        Formula::BoundedEventually { inner, .. } => collect_input_is_refs(inner, variants, refd),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => {
            collect_input_is_refs(left, variants, refd);
            collect_input_is_refs(right, variants, refd);
        }
        // Liveness bodies may constrain input fields too.
        Formula::Eventually { inner } => collect_input_is_refs(inner, variants, refd),
        Formula::Until { left, right } => {
            collect_input_is_refs(left, variants, refd);
            collect_input_is_refs(right, variants, refd);
        }
        _ => {}
    }
}

/// Full canonical domain of one field type (ascending: bool false < true;
/// bit<N> 0..=2^N-1; enum in declaration order).
fn field_domain(ty: &Type, module: &Module) -> Vec<Val> {
    match ty {
        Type::Bool => vec![Val::Bool(false), Val::Bool(true)],
        Type::Bit { width } => {
            let max = if *width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            (0..=max).map(Val::Bit).collect()
        }
        Type::Enum { name } => module
            .enums
            .iter()
            .find(|e| e.name == *name)
            .map(|e| e.variants.iter().cloned().map(Val::Enum).collect())
            .unwrap_or_default(),
    }
}

/// Representative (abstracted) value of one field type: bool -> false,
/// bit<N> -> 0, enum -> first declared variant.
fn representative_value(ty: &Type, module: &Module) -> Option<Val> {
    match ty {
        Type::Bool => Some(Val::Bool(false)),
        Type::Bit { .. } => Some(Val::Bit(0)),
        Type::Enum { name } => module
            .enums
            .iter()
            .find(|e| e.name == *name)
            .and_then(|e| e.variants.first().cloned())
            .map(Val::Enum),
    }
}

/// The canonical input enumeration (section 9.4): event variants in
/// declaration order; within a variant, fields in declaration order; concrete
/// fields over their full domain, abstracted fields at their representative.
fn all_inputs(module: &Module) -> Vec<InputVal> {
    let variants = flatten(&module.events);
    let refd = referenced_fields(module, &variants);
    let mut out = Vec::new();
    for (vi, v) in variants.iter().enumerate() {
        // Per-field value lists (domain or single representative).
        let per_field: Vec<Vec<Val>> = v
            .fields
            .iter()
            .enumerate()
            .map(|(fi, (_, ty))| {
                if refd[vi].contains(&fi) {
                    field_domain(ty, module)
                } else {
                    vec![representative_value(ty, module).unwrap_or(Val::Bit(0))]
                }
            })
            .collect();
        // Cartesian product in declaration order (outermost first).
        let mut combos: Vec<Vec<Val>> = vec![vec![]];
        for vals in per_field {
            let base = std::mem::take(&mut combos);
            combos = base
                .into_iter()
                .flat_map(|prefix| {
                    vals.iter().cloned().map(move |v| {
                        let mut c = prefix.clone();
                        c.push(v);
                        c
                    })
                })
                .collect();
        }
        for fields in combos {
            out.push(InputVal {
                variant: vi,
                fields,
            });
        }
    }
    out
}

// ---------------------------------------------------------------------------
// Transition effects (deterministic first-match + on_error fallback)
// ---------------------------------------------------------------------------

/// One enabled effect of an input at a configuration.
struct Effect {
    /// The transition id that produced the effect (`usize::MAX` = on_error).
    pub tid: usize,
    pub next_state: usize,
    pub next_vars: Vec<Val>,
    pub outputs: Vec<OutputVal>,
}

/// Bindings of a matched pattern: (bind name -> field value), with the
/// binding types for expression typing.
fn pattern_bindings(
    module: &Module,
    t: &Transition,
    inp: &InputVal,
) -> Option<(HashMap<String, Val>, HashMap<String, Type>)> {
    let variants = flatten(&module.events);
    let vi = &variants[inp.variant];
    let fields = match &t.pattern.variant {
        Some(PatternVariant::Struct { name, fields }) if vi.variant == *name => fields,
        _ => return Some((HashMap::new(), HashMap::new())),
    };
    let mut vals = HashMap::new();
    let mut types = HashMap::new();
    for (i, pf) in fields.iter().enumerate() {
        if let PatternExpr::Bind { name } = &pf.pattern {
            if i < vi.fields.len() {
                vals.insert(name.clone(), inp.fields[i].clone());
                types.insert(name.clone(), vi.fields[i].1.clone());
            }
        }
    }
    Some((vals, types))
}

/// The FIRST matching transition for `input` at state `state_idx` in firing
/// order (effective priority descending, declaration order for ties), or the
/// `on_error` fallback when nothing matched - `None` = unexpected input.
fn match_effect(
    module: &Module,
    state_idx: usize,
    vars: &[Val],
    input: &InputVal,
) -> Option<Effect> {
    let states = state_variants(module).to_vec();
    let state_name = states.get(state_idx)?.clone();

    // Firing order for this state (own computation of section 3.4).
    let mut ts: Vec<&Transition> = module
        .transitions
        .iter()
        .filter(|t| t.state == state_name)
        .collect();
    ts.sort_by_key(|t| std::cmp::Reverse(t.priority.unwrap_or(0)));

    for t in &ts {
        if t.pattern.group != input_group(module, input)? {
            continue;
        }
        if !input_matches_variant(module, t, input) {
            continue;
        }
        let (binds, bind_types) = pattern_bindings(module, t, input).unwrap_or_default();
        let ctx = EvalCtx {
            module,
            state: Val::Enum(state_name.clone()),
            vars: var_map(module, vars),
            binds,
            bind_types,
        };
        if let Some(g) = &t.guard {
            // Validated specs always evaluate; an internal failure means "no
            // match" (the generated engines would not compile such a spec).
            if eval_expr(g, &ctx).ok().and_then(|v| bool_of(&v).ok()) == Some(false) {
                continue;
            }
        }
        // Emitted outputs (declaration order).
        let mut outputs = Vec::new();
        for o in &t.emits {
            let args: Vec<Val> = match o
                .args
                .iter()
                .map(|a| eval_expr(a, &ctx))
                .collect::<Result<Vec<_>, _>>()
            {
                Ok(v) => v,
                Err(_) => return None, // internal invariant (validated spec)
            };
            outputs.push(OutputVal {
                variant: output_variant_idx(module, o)?,
                fields: args,
            });
        }
        // Simultaneous updates: all RHS on OLD values.
        let mut next_vars = vars.to_vec();
        for a in &t.assignments {
            let v = eval_expr(&a.value, &ctx).ok()?;
            if let Some(pos) = module.vars.iter().position(|x| x.name == a.var) {
                next_vars[pos] = v;
            }
        }
        return Some(Effect {
            tid: t.id,
            next_state: state_index(module, &t.next_state)?,
            next_vars,
            outputs,
        });
    }

    // on_error fallback: no pattern bindings, variables carry over unchanged.
    if let Some(oe) = &module.on_error {
        let ctx = EvalCtx {
            module,
            state: Val::Enum(state_name.clone()),
            vars: var_map(module, vars),
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        };
        let mut outputs = Vec::new();
        for o in &oe.emits {
            let args: Vec<Val> = o
                .args
                .iter()
                .map(|a| eval_expr(a, &ctx))
                .collect::<Result<Vec<_>, _>>()
                .ok()?;
            outputs.push(OutputVal {
                variant: output_variant_idx(module, o)?,
                fields: args,
            });
        }
        return Some(Effect {
            tid: usize::MAX,
            next_state: state_index(module, &oe.next_state)?,
            next_vars: vars.to_vec(),
            outputs,
        });
    }
    None
}

fn input_group(module: &Module, input: &InputVal) -> Option<String> {
    let variants = flatten(&module.events);
    Some(variants[input.variant].group.clone())
}

fn input_matches_variant(module: &Module, t: &Transition, input: &InputVal) -> bool {
    // Group-level check is done by the caller; here: variant + field patterns.
    let variants = flatten(&module.events);
    let vi = &variants[input.variant];
    match &t.pattern.variant {
        None => true,
        Some(PatternVariant::Unit { name }) => vi.variant == *name,
        Some(PatternVariant::Struct { name, fields }) => {
            if vi.variant != *name || vi.fields.len() != fields.len() {
                return false;
            }
            for (((_, fty), pf), val) in
                vi.fields.iter().zip(fields.iter()).zip(input.fields.iter())
            {
                let ok = match &pf.pattern {
                    PatternExpr::Wildcard | PatternExpr::Bind { .. } => true,
                    PatternExpr::Int { value } => matches!(val, Val::Bit(v) if v == value),
                    PatternExpr::Bool { value } => matches!(val, Val::Bool(b) if b == value),
                    PatternExpr::EnumVariant { path } => enum_match(val, fty, path),
                };
                if !ok {
                    return false;
                }
            }
            true
        }
    }
}

fn var_map(module: &Module, vars: &[Val]) -> HashMap<String, Val> {
    module
        .vars
        .iter()
        .zip(vars.iter())
        .map(|(v, val)| (v.name.clone(), val.clone()))
        .collect()
}

fn state_index(module: &Module, name: &str) -> Option<usize> {
    state_variants(module).iter().position(|s| s == name)
}

fn output_variant_idx(module: &Module, o: &OutputExpr) -> Option<usize> {
    let variants = flatten(&module.outputs);
    match &o.variant {
        Some(v) => variants
            .iter()
            .position(|x| x.group == o.group && x.variant == *v),
        None => {
            // Bare group name: the single variant of a single-variant group.
            let g = module.outputs.iter().find(|g| g.name == o.group)?;
            if g.variants.len() != 1 {
                return None;
            }
            variants.iter().position(|x| x.group == o.group)
        }
    }
}

// ---------------------------------------------------------------------------
// Liveness decomposition (own reimplementation of the D-P4-9 rules,
// documented in SEMANTICS.md sections 6.2/6.3)
// ---------------------------------------------------------------------------

/// One leaf of a liveness property's top-level skeleton: a safety subformula
/// or an obligation (`eventually p` / weak `p until q`) with boolean bodies
/// over atomic propositions.
#[derive(Debug, Clone)]
enum Leaf {
    Safety(Formula),
    Eventually(Formula),
    Until { left: Formula, right: Formula },
}

/// Top-level boolean skeleton of a liveness property (all leaves replaced by
/// indices into [`LivProp::leaves`]).
#[derive(Debug, Clone)]
enum Skel {
    And(Box<Skel>, Box<Skel>),
    Or(Box<Skel>, Box<Skel>),
    Implies(Box<Skel>, Box<Skel>),
    Not(Box<Skel>),
    Leaf(usize),
}

/// A liveness-bearing, checkable property.
#[derive(Debug, Clone)]
struct LivProp {
    pub label: String,
    /// The formula text (own renderer; byte-identical to the generated
    /// engines' embedded string).
    pub formula_text: String,
    /// All leaves (safety + obligations) in first-appearance order.
    pub leaves: Vec<Leaf>,
    pub skeleton: Skel,
}

/// True iff `f` is a boolean formula over atomic propositions (no temporal
/// operators).
fn boolean_over_atoms(f: &Formula) -> bool {
    match f {
        Formula::Expr { .. } | Formula::InputIs { .. } | Formula::OutputHas { .. } => true,
        Formula::Not { inner } => boolean_over_atoms(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => boolean_over_atoms(left) && boolean_over_atoms(right),
        _ => false,
    }
}

/// True iff the formula contains an `eventually`/`until` operator anywhere.
fn contains_liveness(f: &Formula) -> bool {
    match f {
        Formula::Eventually { .. } | Formula::Until { .. } => true,
        Formula::Not { inner }
        | Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. } => contains_liveness(inner),
        Formula::And { left, right }
        | Formula::Or { left, right }
        | Formula::Implies { left, right } => contains_liveness(left) || contains_liveness(right),
        _ => false,
    }
}

/// Decompose a TOP-LEVEL formula of a liveness-bearing property. `Err` =
/// uncheckable (diagnostic; the property is skipped by every engine).
fn decompose_top(f: &Formula, leaves: &mut Vec<Leaf>) -> Result<Skel, String> {
    match f {
        Formula::And { left, right } => Ok(Skel::And(
            Box::new(decompose_top(left, leaves)?),
            Box::new(decompose_top(right, leaves)?),
        )),
        Formula::Or { left, right } => Ok(Skel::Or(
            Box::new(decompose_top(left, leaves)?),
            Box::new(decompose_top(right, leaves)?),
        )),
        Formula::Implies { left, right } => Ok(Skel::Implies(
            Box::new(decompose_top(left, leaves)?),
            Box::new(decompose_top(right, leaves)?),
        )),
        Formula::Not { inner } => Ok(Skel::Not(Box::new(decompose_top(inner, leaves)?))),
        Formula::Eventually { inner } => {
            if !boolean_over_atoms(inner) {
                return Err("liveness subformula must be a boolean formula over atomic propositions (no temporal operators inside `eventually`)".into());
            }
            let idx = leaves.len();
            leaves.push(Leaf::Eventually((**inner).clone()));
            Ok(Skel::Leaf(idx))
        }
        Formula::Until { left, right } => {
            if !boolean_over_atoms(left) || !boolean_over_atoms(right) {
                return Err("liveness subformula must be a boolean formula over atomic propositions (no temporal operators inside `until` operands)".into());
            }
            let idx = leaves.len();
            leaves.push(Leaf::Until {
                left: (**left).clone(),
                right: (**right).clone(),
            });
            Ok(Skel::Leaf(idx))
        }
        Formula::Next { inner }
        | Formula::Always { inner }
        | Formula::BoundedAlways { inner, .. }
        | Formula::BoundedEventually { inner, .. } => Err(if contains_liveness(inner) {
            "liveness operators nested under a temporal operator are not supported (deeper nesting)"
                .into()
        } else {
            "temporal operators are not supported in liveness properties; split the safety part into a separate property".into()
        }),
        Formula::Expr { .. } | Formula::InputIs { .. } | Formula::OutputHas { .. } => {
            let idx = leaves.len();
            leaves.push(Leaf::Safety(f.clone()));
            Ok(Skel::Leaf(idx))
        }
    }
}

/// Decompose all properties: `Some` iff liveness-bearing AND checkable.
fn decompose_properties(module: &Module) -> (Vec<Option<LivProp>>, Vec<String>) {
    let mut props = Vec::new();
    let mut diagnostics = Vec::new();
    for p in &module.properties {
        if !contains_liveness(&p.formula) {
            props.push(None);
            continue;
        }
        let mut leaves = Vec::new();
        match decompose_top(&p.formula, &mut leaves) {
            Ok(skeleton) => props.push(Some(LivProp {
                label: p.name.clone(),
                formula_text: print_formula_own(&p.formula),
                leaves,
                skeleton,
            })),
            Err(msg) => {
                diagnostics.push(format!("property '{}': {}", p.name, msg));
                props.push(None);
            }
        }
    }
    (props, diagnostics)
}

// ---------------------------------------------------------------------------
// Own formula -> text renderer (byte-identical to the generated engines'
// embedded formula strings; mirrors the documented grammar's round-trip form)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, Copy, PartialEq)]
enum Fctx {
    Top,
    ImpliesLeft,
    OrChild,
    ImpliesRight,
    AndChild,
    UntilLeft,
    UntilRight,
}

fn print_formula_own(f: &Formula) -> String {
    print_formula_ctx(f, Fctx::Top)
}

fn print_formula_ctx(f: &Formula, ctx: Fctx) -> String {
    let core = match f {
        Formula::Or { left, right } => format!(
            "{} || {}",
            print_formula_ctx(left, Fctx::OrChild),
            print_formula_ctx(right, Fctx::OrChild)
        ),
        Formula::And { left, right } => format!(
            "{} && {}",
            print_formula_ctx(left, Fctx::AndChild),
            print_formula_ctx(right, Fctx::AndChild)
        ),
        Formula::Implies { left, right } => format!(
            "{} -> {}",
            print_formula_ctx(left, Fctx::ImpliesLeft),
            print_formula_ctx(right, Fctx::ImpliesRight)
        ),
        Formula::Not { inner } => {
            let inner_text = match inner.as_ref() {
                Formula::Implies { .. }
                | Formula::Or { .. }
                | Formula::And { .. }
                | Formula::Until { .. } => {
                    format!("({})", print_formula_ctx(inner, Fctx::Top))
                }
                other => print_formula_ctx(other, Fctx::Top),
            };
            format!("!{inner_text}")
        }
        Formula::Expr { expr } => print_expr_own(expr),
        Formula::InputIs { pattern } => format!(
            "input_is({})",
            print_pattern_own(&pattern.group, &pattern.variant)
        ),
        Formula::OutputHas { pattern } => format!(
            "output_has({})",
            print_pattern_own(&pattern.group, &pattern.variant)
        ),
        Formula::Next { inner } => format!("next ({})", print_formula_ctx(inner, Fctx::Top)),
        Formula::Always { inner } => format!("always ({})", print_formula_ctx(inner, Fctx::Top)),
        Formula::BoundedAlways { bound, inner } => format!(
            "always[{}] ({})",
            bound,
            print_formula_ctx(inner, Fctx::Top)
        ),
        Formula::BoundedEventually { bound, inner } => format!(
            "bounded_eventually[{}] ({})",
            bound,
            print_formula_ctx(inner, Fctx::Top)
        ),
        Formula::Eventually { inner } => {
            format!("eventually ({})", print_formula_ctx(inner, Fctx::Top))
        }
        Formula::Until { left, right } => format!(
            "{} until {}",
            print_formula_ctx(left, Fctx::UntilLeft),
            print_formula_ctx(right, Fctx::UntilRight)
        ),
    };
    if needs_parens(f, ctx) {
        format!("({core})")
    } else {
        core
    }
}

fn needs_parens(f: &Formula, ctx: Fctx) -> bool {
    match f {
        Formula::Next { .. }
        | Formula::Always { .. }
        | Formula::BoundedAlways { .. }
        | Formula::BoundedEventually { .. }
        | Formula::Eventually { .. }
        | Formula::InputIs { .. }
        | Formula::OutputHas { .. } => false,
        Formula::Expr { expr } => {
            matches!(
                ctx,
                Fctx::OrChild | Fctx::AndChild | Fctx::UntilLeft | Fctx::UntilRight
            ) && expr_has_boolean_op(expr)
        }
        Formula::Not { .. } => matches!(ctx, Fctx::OrChild | Fctx::AndChild),
        Formula::Implies { .. } => matches!(
            ctx,
            Fctx::AndChild | Fctx::ImpliesLeft | Fctx::OrChild | Fctx::UntilLeft | Fctx::UntilRight
        ),
        Formula::And { .. } => matches!(ctx, Fctx::UntilLeft | Fctx::UntilRight),
        Formula::Or { .. } => matches!(
            ctx,
            Fctx::AndChild | Fctx::ImpliesLeft | Fctx::UntilLeft | Fctx::UntilRight
        ),
        Formula::Until { .. } => matches!(ctx, Fctx::UntilRight),
    }
}

fn expr_has_boolean_op(e: &Expr) -> bool {
    match e {
        Expr::Int { .. } | Expr::Bool { .. } | Expr::Path { .. } => false,
        Expr::Unary { expr, .. } => expr_has_boolean_op(expr),
        Expr::Binary { op, left, right } => {
            matches!(op, BinOp::LogicAnd | BinOp::LogicOr)
                || expr_has_boolean_op(left)
                || expr_has_boolean_op(right)
        }
    }
}

const PREC_LOGIC_OR: u8 = 1;
const PREC_LOGIC_AND: u8 = 2;
const PREC_EQUALITY: u8 = 3;
const PREC_RELATIONAL: u8 = 4;
const PREC_BIT_OR: u8 = 5;
const PREC_BIT_XOR: u8 = 6;
const PREC_BIT_AND: u8 = 7;
const PREC_SHIFT: u8 = 8;
const PREC_ADDITIVE: u8 = 9;
const PREC_MULTIPLICATIVE: u8 = 10;
const PREC_UNARY: u8 = 11;
const PREC_PRIMARY: u8 = 12;

fn bin_prec(op: BinOp) -> u8 {
    match op {
        BinOp::LogicOr => PREC_LOGIC_OR,
        BinOp::LogicAnd => PREC_LOGIC_AND,
        BinOp::Eq | BinOp::NotEq => PREC_EQUALITY,
        BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => PREC_RELATIONAL,
        BinOp::BitOr => PREC_BIT_OR,
        BinOp::BitXor => PREC_BIT_XOR,
        BinOp::BitAnd => PREC_BIT_AND,
        BinOp::ShiftLeft | BinOp::ShiftRight => PREC_SHIFT,
        BinOp::Plus | BinOp::Minus => PREC_ADDITIVE,
        BinOp::Star => PREC_MULTIPLICATIVE,
    }
}

fn bin_str(op: BinOp) -> &'static str {
    match op {
        BinOp::Plus => "+",
        BinOp::Minus => "-",
        BinOp::Star => "*",
        BinOp::ShiftLeft => "<<",
        BinOp::ShiftRight => ">>",
        BinOp::Eq => "==",
        BinOp::NotEq => "!=",
        BinOp::Lt => "<",
        BinOp::Le => "<=",
        BinOp::Gt => ">",
        BinOp::Ge => ">=",
        BinOp::BitOr => "|",
        BinOp::BitXor => "^",
        BinOp::BitAnd => "&",
        BinOp::LogicOr => "||",
        BinOp::LogicAnd => "&&",
    }
}

fn print_expr_own(e: &Expr) -> String {
    print_expr_min(e, 0)
}

fn print_expr_min(e: &Expr, min_prec: u8) -> String {
    let (core, prec) = match e {
        Expr::Int { value } => (value.to_string(), PREC_PRIMARY),
        Expr::Bool { value } => (value.to_string(), PREC_PRIMARY),
        Expr::Path { path } => (path.join("::"), PREC_PRIMARY),
        Expr::Unary { op, expr } => {
            let inner = print_expr_min(expr, PREC_UNARY);
            let inner = if matches!(expr.as_ref(), Expr::Binary { .. } | Expr::Unary { .. }) {
                format!("({inner})")
            } else {
                inner
            };
            let opstr = match op {
                UnaryOp::Not => "!",
                UnaryOp::BitNot => "~",
                UnaryOp::Neg => "-",
            };
            (format!("{opstr}{inner}"), PREC_UNARY)
        }
        Expr::Binary { op, left, right } => {
            let p = bin_prec(*op);
            let l = print_expr_min(left, p);
            let r = print_expr_min(right, p + 1);
            (format!("{l} {} {r}", bin_str(*op)), p)
        }
    };
    if prec < min_prec {
        format!("({core})")
    } else {
        core
    }
}

fn print_pattern_own(group: &str, variant: &Option<PatternVariant>) -> String {
    match variant {
        None => group.to_string(),
        Some(PatternVariant::Unit { name }) => format!("{group}({name})"),
        Some(PatternVariant::Struct { name, fields }) => {
            let fs: Vec<String> = fields.iter().map(print_pattern_field_own).collect();
            format!("{group}({name}({}))", fs.join(", "))
        }
    }
}

fn print_pattern_field_own(pf: &PatternField) -> String {
    match (&pf.name, &pf.pattern) {
        (Some(n), PatternExpr::Bind { name }) if n == name => n.clone(),
        (Some(n), _) => format!("{n}: {}", print_pattern_expr_own(&pf.pattern)),
        (None, _) => print_pattern_expr_own(&pf.pattern),
    }
}

fn print_pattern_expr_own(pe: &PatternExpr) -> String {
    match pe {
        PatternExpr::Wildcard => "_".to_string(),
        PatternExpr::Bind { name } => name.clone(),
        PatternExpr::Int { value } => value.to_string(),
        PatternExpr::Bool { value } => value.to_string(),
        PatternExpr::EnumVariant { path } => path.join("::"),
    }
}

// ---------------------------------------------------------------------------
// Public API
// ---------------------------------------------------------------------------

/// Configuration of one reference model-checking run.
#[derive(Debug, Clone)]
pub struct RefConfig {
    /// State cap (`--max-states`); a hit yields `UnknownStateLimit`.
    pub max_states: usize,
    /// Wall-clock timeout in seconds (0 = off; NOT parity-guaranteed).
    pub timeout_sec: u64,
    /// Assumption selection wire string: `all`, `none`, or a comma-separated
    /// label list (unknown labels silently dropped - same as the generated
    /// binaries' `--assumes`).
    pub assume_arg: String,
}

impl Default for RefConfig {
    fn default() -> Self {
        Self {
            max_states: EXACT_DEFAULT_MAX_STATES,
            timeout_sec: EXACT_DEFAULT_TIMEOUT_SEC,
            assume_arg: "all".to_string(),
        }
    }
}

/// One recorded input/output of a counterexample step.
#[derive(Debug, Clone)]
pub struct IoRec {
    /// Flattened constructor name (the generated `Input::`/`Output::` variant).
    pub variant: String,
    /// (field name, value) in declaration order.
    pub fields: Vec<(String, Val)>,
}

/// One step of a counterexample trace.
#[derive(Debug, Clone)]
pub struct StepRec {
    pub input: IoRec,
    pub outputs: Vec<IoRec>,
    /// State variant names (no `State::` prefix - added by renderers).
    pub from_state: String,
    pub to_state: String,
    /// Post-step variable values (declaration order).
    pub vars: Vec<Val>,
}

/// The counterexample data of a failed run (kind mirrors [`JsonCounterexample`]).
#[derive(Debug, Clone)]
pub enum CeData {
    /// Property violation (incl. sample-0): the full trace steps.
    Trace {
        /// The sample that started the failing obligation (None for plain
        /// per-sample failures).
        trigger_step: Option<usize>,
        steps: Vec<StepRec>,
    },
    /// Liveness violation: prefix + one cycle iteration; `cycle_start` is the
    /// number of PREFIX steps (the reported violation step).
    Lasso {
        formula_text: String,
        cycle_start: usize,
        note: String,
        steps: Vec<StepRec>,
    },
    /// Deadlock: the terminal configuration at `depth`, reached via `steps`.
    Deadlock { depth: usize, steps: Vec<StepRec> },
}

/// The result of one reference model-checking run. All fields needed by BOTH
/// report renderings (text + JSON) live here; rendering is pure over this.
#[derive(Debug, Clone)]
pub struct RefResult {
    pub status: ExactStatus,
    /// Configurations enqueued (including the initial one).
    pub states: usize,
    /// Enabled effects processed (success + pruned + violation; unexpected
    /// inputs do NOT count).
    pub transitions: usize,
    /// Greatest depth reached.
    pub max_depth: usize,
    /// (reason, label, step) - present iff status is Fail.
    pub fail: Option<(FailReason, String, usize)>,
    pub unexpected_terminations: usize,
    /// First unexpected termination (config id, input index), if any.
    pub first_unexpected: Option<(usize, usize)>,
    /// Encoded bit width of all property + assumption monitors.
    pub monitor_size_bits: usize,
    /// `states * (config bytes + 32)` - the same estimate formula the
    /// generated engines use (their config byte size is computed from the
    /// documented encoding widths of SEMANTICS.md section 9.2).
    pub memory_estimate_bytes: usize,
    /// Some(elapsed seconds) when the run stopped at the timeout.
    pub timeout_secs: Option<u64>,
    /// The persistent monitor-state projection at the deadlocked terminal
    /// configuration (Some iff status is Fail with reason Deadlock). Lets an
    /// external verifier re-derive the SEMANTICS 8.4 "no surviving successor"
    /// judgment, which depends on assumption-pruning state carried by the
    /// configuration but not printed in the report.
    pub deadlock_mons: Option<Vec<Vec<u64>>>,
    /// Warnings for formulas outside the monitor subset / uncheckable
    /// liveness (stderr diagnostics of `rfs mc-ref`).
    pub skip_warnings: Vec<String>,
    /// Initial configuration (state variant name + variable values).
    pub initial_state: String,
    pub initial_vars: Vec<Val>,
    /// Present iff status is Fail.
    pub ce: Option<CeData>,
}

/// One BFS node: a configuration with its monitor runtime states.
#[derive(Clone)]
struct BNode {
    state: usize,
    vars: Vec<Val>,
    /// Monitor states in aggregate order (properties, then selected
    /// assumptions).
    mons: Vec<MonSt>,
}

/// Structured configuration key (no bit-packing - deliberate).
#[derive(Hash, Eq, PartialEq)]
struct CfgKey {
    state: usize,
    vars: Vec<u64>,
    /// Per monitor: the persistent projection (one u64 per persistent node).
    mons: Vec<Vec<u64>>,
}

/// Encode one value for the configuration key. Variable entries are typed by
/// position, so encoding is injective per variable: enums encode as their
/// variant index, bools as 0/1, bits as the value.
fn val_key(v: &Val) -> u64 {
    match v {
        Val::Bool(b) => *b as u64,
        Val::Bit(x) => *x,
        Val::Enum(_) => 0, // replaced below with the variant index
    }
}

fn bnode_key(n: &BNode, module: &Module, mon_specs: &[&Mon]) -> CfgKey {
    let vars = n
        .vars
        .iter()
        .zip(module.vars.iter())
        .map(|(val, vd)| match (val, &vd.ty) {
            (Val::Enum(name), Type::Enum { name: en }) => module
                .enums
                .iter()
                .find(|e| e.name == *en)
                .and_then(|e| e.variants.iter().position(|v| v == name))
                .unwrap_or(0) as u64,
            _ => val_key(val),
        })
        .collect();
    CfgKey {
        state: n.state,
        vars,
        mons: n
            .mons
            .iter()
            .zip(mon_specs.iter())
            .map(|(st, sp)| mon_key(st, sp))
            .collect(),
    }
}

/// Run the brute-force reference model checker on ONE validated module.
///
/// Mirrors the generated engines' DEFAULT exact mode (deterministic
/// first-match firing, seedless exhaustive level-synchronous BFS). Returns
/// `Err` for internal invariant failures (e.g. an initializer that references
/// another variable - such specs do not produce working generated binaries
/// either) and for an assumption already violated at the initial
/// configuration (the generated binaries panic there; a CLI tool reports a
/// clean error instead, exit code 1).
pub fn run(module: &Module, cfg: &RefConfig) -> Result<RefResult, String> {
    let t0 = Instant::now();

    // --- Monitors + liveness decomposition (own compilers). ---
    let mut skip_warnings = Vec::new();
    let mut prop_mons: Vec<(String, Mon)> = Vec::new();
    for p in &module.properties {
        if contains_liveness(&p.formula) {
            continue; // liveness properties are checked post-BFS, not monitored
        }
        match compile_formula(&p.formula) {
            Ok(mon) => prop_mons.push((p.name.clone(), mon)),
            Err(_) => skip_warnings.push(format!(
                "property '{}' is outside the monitor subset; skipped",
                p.name
            )),
        }
    }
    let known: Vec<&str> = module.assumptions.iter().map(|a| a.name.as_str()).collect();
    let selected = parse_assume_selection(&cfg.assume_arg, &known);
    let mut assume_mons: Vec<(String, Mon)> = Vec::new();
    for a in &module.assumptions {
        if !selected.iter().any(|x| x == &a.name) {
            continue;
        }
        match compile_formula(&a.formula) {
            Ok(mon) => assume_mons.push((a.name.clone(), mon)),
            Err(_) => skip_warnings.push(format!(
                "assume '{}' is outside the monitor subset; skipped",
                a.name
            )),
        }
    }
    let (liv_props_opt, liv_diagnostics) = decompose_properties(module);
    for d in &liv_diagnostics {
        skip_warnings.push(d.clone());
    }
    let liv_props: Vec<LivProp> = liv_props_opt.into_iter().flatten().collect();

    // --- Configuration encoding widths (documented formulas, SEMANTICS 9.2)
    //     - used for the `monitor size` + `memory estimate` statistics only.
    let state_count = state_variants(module).len();
    let state_bits = ceil_log2(state_count as u64);
    let vars_bits: u32 = module
        .vars
        .iter()
        .map(|v| match &v.ty {
            Type::Bool => 1,
            Type::Bit { width } => *width,
            Type::Enum { name } => ceil_log2(
                module
                    .enums
                    .iter()
                    .find(|e| e.name == *name)
                    .map(|e| e.variants.len() as u64)
                    .unwrap_or(0),
            ),
        })
        .sum();
    let monitors_bits: u32 = prop_mons
        .iter()
        .chain(assume_mons.iter())
        .map(|(_, m)| mon_bits(m))
        .sum();
    let total_bytes = ((state_bits + vars_bits + monitors_bits) as usize).div_ceil(8);

    // --- Initial configuration. ---
    let states = state_variants(module).to_vec();
    let q0 = module
        .initial_state
        .as_ref()
        .and_then(|s| states.iter().position(|x| x == s))
        .ok_or_else(|| "module has no initial state".to_string())?;
    let mut init_vars: Vec<Val> = Vec::new();
    for v in &module.vars {
        let ctx = EvalCtx {
            module,
            state: Val::Enum(states[q0].clone()),
            vars: HashMap::new(),
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        };
        match eval_expr(&v.init, &ctx) {
            Ok(val) => init_vars.push(val),
            Err(_) => {
                return Err(format!(
                    "variable '{}' initializer is not a constant expression (unsupported)",
                    v.name
                ))
            }
        }
    }

    let all_mons: Vec<(String, Mon)> = prop_mons
        .iter()
        .cloned()
        .chain(assume_mons.iter().cloned())
        .collect();
    let n_props = all_mons.len() - assume_mons.len();
    let mon_specs: Vec<&Mon> = all_mons.iter().map(|(_, m)| m).collect();

    // Monitor states of the initial configuration; step sample 0.
    let mut mons: Vec<MonSt> = mon_specs.iter().copied().map(mon_init).collect();
    let sample0 = Sample {
        input: None,
        outputs: Vec::new(),
        to_state: q0,
        vars: init_vars.clone(),
    };
    // Assumptions first (the generated constructors panic on a sample-0
    // assumption violation; here: a clean error).
    for (i, (label, spec)) in all_mons.iter().enumerate().skip(n_props) {
        if mon_step(&mut mons[i], spec, 0, &sample0, module)? {
            return Err(format!(
                "assumption '{label}' violated at initial configuration (sample 0)"
            ));
        }
    }
    for (i, (label, spec)) in all_mons.iter().enumerate().take(n_props) {
        if mon_step(&mut mons[i], spec, 0, &sample0, module)? {
            // Sample-0 property violation: FAIL with the initial trace only.
            return Ok(RefResult {
                status: ExactStatus::Fail,
                states: 1,
                transitions: 0,
                max_depth: 0,
                fail: Some((FailReason::PropertyViolation, label.clone(), 0)),
                unexpected_terminations: 0,
                deadlock_mons: None,
                first_unexpected: None,
                monitor_size_bits: monitors_bits as usize,
                memory_estimate_bytes: total_bytes + 32,
                timeout_secs: None,
                skip_warnings,
                initial_state: states[q0].clone(),
                initial_vars: init_vars,
                ce: Some(CeData::Trace {
                    trigger_step: {
                        let t = mons[i].trigger_step();
                        (t >= 0).then_some(t as usize)
                    },
                    steps: Vec::new(),
                }),
            });
        }
    }

    // --- Level-synchronous BFS (SEMANTICS 9.5). ---
    let inputs = all_inputs(module);
    let in_variants = flatten(&module.events);
    let out_variants = flatten(&module.outputs);

    let node0 = BNode {
        state: q0,
        vars: init_vars.clone(),
        mons,
    };
    let mut visited: HashMap<CfgKey, usize> = HashMap::new();
    let mut nodes: Vec<BNode> = vec![node0];
    let mut parent: Vec<Option<(usize, usize, usize)>> = vec![None]; // (parent cid, iidx, tid)
    let mut depth: Vec<usize> = vec![0];
    let mut adj: Vec<Vec<(usize, usize, usize)>> = vec![Vec::new()]; // (iidx, tid, target)
    let mut complete: Vec<bool> = vec![false];
    visited.insert(bnode_key(&nodes[0], module, &mon_specs), 0);

    let mut frontier: Vec<usize> = vec![0];
    let mut max_depth = 0usize;
    let mut transitions = 0usize;
    let mut unexpected = 0usize;
    let mut first_unexpected: Option<(usize, usize)> = None;
    #[derive(PartialEq)]
    enum St {
        Running,
        Pass,
        StateLimit,
        Timeout,
        Fail,
    }
    let mut status = St::Running;
    let mut fail: Option<(FailReason, String, usize)> = None;
    let mut fail_path: Vec<(usize, usize, usize)> = Vec::new();
    let mut fail_trigger: Option<usize> = None;
    // Persistent monitor projection of the deadlocked terminal configuration
    // (SEMANTICS 8.4 deadlock depends on assumption-pruning state).
    let mut deadlock_mons: Option<Vec<Vec<u64>>> = None;

    while !frontier.is_empty() && status == St::Running {
        let mut next_frontier: Vec<usize> = Vec::new();
        'level: for &cid in &frontier {
            // Fine-grained deadline check (per configuration): ONE BFS level
            // can exceed the wall-clock budget on its own (e.g.
            // 19_guard_complex: one level runs for minutes), so checking only
            // between levels would let a single run blow past --timeout-sec.
            if cfg.timeout_sec > 0 && t0.elapsed().as_secs() >= cfg.timeout_sec {
                status = St::Timeout;
                break 'level;
            }
            let d = depth[cid];
            let node = nodes[cid].clone();
            let mut any_successor = false;
            for (iidx, input) in inputs.iter().enumerate() {
                let Some(eff) = match_effect(module, node.state, &node.vars, input) else {
                    // Unexpected input: this path terminates (SEMANTICS 8.3).
                    unexpected += 1;
                    if first_unexpected.is_none() {
                        first_unexpected = Some((cid, iidx));
                    }
                    continue;
                };
                transitions += 1;
                let sample = Sample {
                    input: Some(input.clone()),
                    outputs: eff.outputs.clone(),
                    to_state: eff.next_state,
                    vars: eff.next_vars.clone(),
                };
                // Step the monitors at sample d + 1 (assumptions first: a
                // pruned successor is never reported as a property violation).
                let mut mons = node.mons.clone();
                let mut pruned = false;
                for (mi, (_, spec)) in all_mons.iter().enumerate().skip(n_props) {
                    if mon_step(&mut mons[mi], spec, (d + 1) as u64, &sample, module)? {
                        pruned = true;
                        break;
                    }
                }
                let mut violation: Option<(String, usize, Option<usize>)> = None;
                if !pruned {
                    for (mi, (label, spec)) in all_mons.iter().enumerate().take(n_props) {
                        if mon_step(&mut mons[mi], spec, (d + 1) as u64, &sample, module)? {
                            violation = Some((label.clone(), d + 1, {
                                let t = mons[mi].trigger_step();
                                (t >= 0).then_some(t as usize)
                            }));
                            break;
                        }
                    }
                }
                if let Some((label, step, trigger)) = violation {
                    fail = Some((FailReason::PropertyViolation, label, step));
                    status = St::Fail;
                    fail_trigger = trigger;
                    let mut p = path_to(&parent, cid);
                    p.push((cid, iidx, eff.tid));
                    fail_path = p;
                    break 'level;
                }
                if pruned {
                    continue;
                }
                any_successor = true;
                let next = BNode {
                    state: eff.next_state,
                    vars: eff.next_vars,
                    mons,
                };
                let key = bnode_key(&next, module, &mon_specs);
                let tcid = match visited.get(&key).copied() {
                    Some(x) => x,
                    None => {
                        if nodes.len() >= cfg.max_states {
                            status = St::StateLimit;
                            break 'level;
                        }
                        visited.insert(key, nodes.len());
                        parent.push(Some((cid, iidx, eff.tid)));
                        depth.push(d + 1);
                        if d + 1 > max_depth {
                            max_depth = d + 1;
                        }
                        adj.push(Vec::new());
                        complete.push(false);
                        nodes.push(next);
                        next_frontier.push(nodes.len() - 1);
                        nodes.len() - 1
                    }
                };
                // Graph edge: recorded for every success (incl. to visited).
                adj[cid].push((iidx, eff.tid, tcid));
            }
            if status != St::Running {
                break;
            }
            complete[cid] = true;
            if !any_successor {
                // Deadlock (SEMANTICS 8.4): no successor at all.
                fail = Some((FailReason::Deadlock, "-".into(), d));
                status = St::Fail;
                fail_path = path_to(&parent, cid);
                deadlock_mons = Some(
                    nodes[cid]
                        .mons
                        .iter()
                        .zip(mon_specs.iter())
                        .map(|(st, sp)| mon_key(st, sp))
                        .collect(),
                );
                break;
            }
        }
        frontier = next_frontier;
    }
    if status == St::Running {
        status = St::Pass;
    }

    // --- Liveness analysis (own algorithm) on the completed or truncated
    //     graph - only when no safety failure / deadlock was found. A witness
    //     inside a truncated subgraph is SOUND (all its edges are real).
    let mut liv_result: Option<(usize, LivWitness)> = None;
    if status != St::Fail && !liv_props.is_empty() {
        for (pi, prop) in liv_props.iter().enumerate() {
            if let Some(w) = liv_analyze(module, &nodes, &adj, &inputs, prop) {
                liv_result = Some((pi, w));
                break;
            }
        }
        if let Some((pi, w)) = &liv_result {
            status = St::Fail;
            fail = Some((
                FailReason::LivenessViolation,
                liv_props[*pi].label.clone(),
                w.cycle_start,
            ));
            fail_path = w.lasso.clone(); // (src config, iidx, tid) edges
        }
    }

    // --- Assemble the result + counterexample data. ---
    let states_n = nodes.len();
    let mem_est = states_n * (total_bytes + 32);
    let status_out = match status {
        St::Pass => ExactStatus::PassComplete,
        St::Fail => ExactStatus::Fail,
        _ => ExactStatus::UnknownStateLimit,
    };
    let timeout_secs = matches!(status, St::Timeout).then(|| t0.elapsed().as_secs());

    let ce = match &fail {
        Some((FailReason::PropertyViolation, _, _)) => Some(CeData::Trace {
            trigger_step: fail_trigger,
            steps: trace_steps(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                &fail_path,
            ),
        }),
        Some((FailReason::Deadlock, _, d)) => Some(CeData::Deadlock {
            depth: *d,
            steps: trace_steps(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                &fail_path,
            ),
        }),
        Some((FailReason::LivenessViolation, _, _)) => {
            let (pi, w) = liv_result.as_ref().expect("liveness witness present");
            let prop = &liv_props[*pi];
            let steps = trace_steps(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                &w.lasso,
            );
            let note = liv_note(
                module,
                &nodes,
                &inputs,
                &in_variants,
                &out_variants,
                prop,
                w,
            );
            Some(CeData::Lasso {
                formula_text: prop.formula_text.clone(),
                cycle_start: w.cycle_start,
                note,
                steps,
            })
        }
        None => None,
    };

    Ok(RefResult {
        status: status_out,
        states: states_n,
        transitions,
        max_depth,
        fail,
        unexpected_terminations: unexpected,
        first_unexpected,
        deadlock_mons,
        monitor_size_bits: monitors_bits as usize,
        memory_estimate_bytes: mem_est,
        timeout_secs,
        skip_warnings,
        initial_state: states[q0].clone(),
        initial_vars: init_vars,
        ce,
    })
}

/// Steps from the root to configuration `cid`: (child config id, input idx,
/// transition id).
fn path_to(parent: &[Option<(usize, usize, usize)>], cid: usize) -> Vec<(usize, usize, usize)> {
    // Entries are (SOURCE config id, input index, transition id).
    let mut v = Vec::new();
    let mut c = cid;
    while let Some((p, i, t)) = parent[c] {
        v.push((p, i, t));
        c = p;
    }
    v.reverse();
    v
}

fn state_name(module: &Module, idx: usize) -> &str {
    &state_variants(module)[idx]
}

fn io_in(in_variants: &[FlatVariant], variant_idx: usize, input: &InputVal) -> IoRec {
    let vi = &in_variants[variant_idx];
    IoRec {
        variant: vi.unique_name.clone(),
        fields: vi
            .fields
            .iter()
            .map(|(n, _)| n.clone())
            .zip(input.fields.iter().cloned())
            .collect(),
    }
}

fn io_out(out_variants: &[FlatVariant], o: &OutputVal) -> IoRec {
    let vi = &out_variants[o.variant];
    IoRec {
        variant: vi.unique_name.clone(),
        fields: vi
            .fields
            .iter()
            .map(|(n, _)| n.clone())
            .zip(o.fields.iter().cloned())
            .collect(),
    }
}

/// Rebuild the trace steps of a path (each edge re-evaluated on its source
/// configuration - deterministic first-match, so effects are reproducible).
fn trace_steps(
    module: &Module,
    nodes: &[BNode],
    inputs: &[InputVal],
    in_variants: &[FlatVariant],
    out_variants: &[FlatVariant],
    path: &[(usize, usize, usize)],
) -> Vec<StepRec> {
    path.iter()
        .map(|(cid, iidx, _tid)| {
            let node = &nodes[*cid];
            let input = &inputs[*iidx];
            let eff = match_effect(module, node.state, &node.vars, input)
                .expect("deterministic replay: the recorded edge must still fire");
            StepRec {
                input: io_in(in_variants, input.variant, input),
                outputs: eff
                    .outputs
                    .iter()
                    .map(|o| io_out(out_variants, o))
                    .collect(),
                from_state: state_name(module, node.state).to_string(),
                to_state: state_name(module, eff.next_state).to_string(),
                vars: eff.next_vars,
            }
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Liveness analysis (own algorithm - SCC-based, per SEMANTICS.md section 6)
// ---------------------------------------------------------------------------

/// A liveness counterexample: `lasso` edges in order; the first
/// `cycle_start` are the prefix (from config 0), the rest form a cycle back
/// to the config at position `cycle_start`.
#[derive(Debug, Clone)]
pub struct LivWitness {
    pub lasso: Vec<(usize, usize, usize)>,
    pub cycle_start: usize,
}

/// Per-sample leaf atoms of one liveness property (safety leaves +
/// obligation bodies as signals).
struct LeafSig {
    kind: u8, // 0 safety, 1 eventually, 2 until
    a: Sig,
    b: Option<Sig>,
}

fn leaf_signals(prop: &LivProp) -> Vec<LeafSig> {
    prop.leaves
        .iter()
        .map(|l| match l {
            Leaf::Safety(f) => LeafSig {
                kind: 0,
                a: compile_signal(f).expect("decomposition guarantees boolean-over-atoms"),
                b: None,
            },
            Leaf::Eventually(body) => LeafSig {
                kind: 1,
                a: compile_signal(body).expect("decomposition guarantees boolean-over-atoms"),
                b: None,
            },
            Leaf::Until { left, right } => LeafSig {
                kind: 2,
                a: compile_signal(left).expect("decomposition guarantees boolean-over-atoms"),
                b: Some(
                    compile_signal(right).expect("decomposition guarantees boolean-over-atoms"),
                ),
            },
        })
        .collect()
}

/// Evaluate the skeleton over per-leaf satisfaction flags.
fn skel_eval(s: &Skel, t: &[bool]) -> bool {
    match s {
        Skel::And(l, r) => skel_eval(l, t) && skel_eval(r, t),
        Skel::Or(l, r) => skel_eval(l, t) || skel_eval(r, t),
        Skel::Implies(l, r) => !skel_eval(l, t) || skel_eval(r, t),
        Skel::Not(i) => !skel_eval(i, t),
        Skel::Leaf(i) => t[*i],
    }
}

/// Analyze one liveness property on the explored graph. Returns a lasso
/// witness iff violated (SEMANTICS.md section 6.4/6.6).
fn liv_analyze(
    module: &Module,
    nodes: &[BNode],
    adj: &[Vec<(usize, usize, usize)>],
    inputs: &[InputVal],
    prop: &LivProp,
) -> Option<LivWitness> {
    let leaves = leaf_signals(prop);
    let n = leaves.len();
    if n == 0 {
        return None;
    }
    let radices: Vec<usize> = leaves
        .iter()
        .map(|l| if l.kind == 2 { 3 } else { 2 })
        .collect();
    let mut places = vec![1usize; n];
    for k in (0..n - 1).rev() {
        places[k] = places[k + 1] * radices[k + 1];
    }
    let qcount: usize = radices.iter().product();

    // --- Position graph: position 0 = initial sample; one position per
    //     explored edge (edge labels are EDGE-based, SEMANTICS 6.5). ---
    let n_edges: usize = adj.iter().map(|e| e.len()).sum();
    let n_pos = 1 + n_edges;
    let mut pos_succ: Vec<Vec<usize>> = vec![Vec::new(); n_pos];
    // Position id of edge (cid, i) = 1 + sum_{c < cid} adj[c].len() + i.
    let mut prefix: Vec<usize> = vec![0usize; adj.len() + 1];
    for (c, es) in adj.iter().enumerate() {
        prefix[c + 1] = prefix[c] + es.len();
    }
    let pos_of_edge = |cid: usize, i: usize| 1 + prefix[cid] + i;
    // Inverse mapping: position -> (config id, adj index) of its edge.
    let mut pos_edge: Vec<Option<(usize, usize)>> = vec![None; n_pos];
    for cid in 0..adj.len() {
        for (i, _) in adj[cid].iter().enumerate() {
            pos_edge[pos_of_edge(cid, i)] = Some((cid, i));
        }
    }
    // Successors of position 0: outgoing edges of config 0. Successors of an
    // edge-position (u -> v): the outgoing edges of v.
    for (i, _) in adj[0].iter().enumerate() {
        pos_succ[0].push(pos_of_edge(0, i));
    }
    for cid in 0..adj.len() {
        for (i, &(_, _, v)) in adj[cid].iter().enumerate() {
            let p = pos_of_edge(cid, i);
            for (j, _) in adj[v].iter().enumerate() {
                pos_succ[p].push(pos_of_edge(v, j));
            }
        }
    }

    // --- Per-position label bits. ---
    let init_sample = Sample {
        input: None,
        outputs: Vec::new(),
        to_state: nodes[0].state,
        vars: nodes[0].vars.clone(),
    };
    let mut bits_at: Vec<u32> = vec![0u32; n_pos];
    bits_at[0] = leaf_bits(&leaves, &init_sample, module).ok()?;
    for cid in 0..adj.len() {
        let node = &nodes[cid];
        for (i, &(_iidx, _tid, _v)) in adj[cid].iter().enumerate() {
            let input = inputs[_iidx].clone();
            let eff = match_effect(module, node.state, &node.vars, &input)?;
            let sample = Sample {
                input: Some(input),
                outputs: eff.outputs.clone(),
                to_state: eff.next_state,
                vars: eff.next_vars.clone(),
            };
            bits_at[pos_of_edge(cid, i)] = leaf_bits(&leaves, &sample, module).ok()?;
        }
    }

    // --- Product reachability (BFS with parent pointers). ---
    let start_q = prod_delta(0, &places, &radices, &leaves, bits_at[0]);
    let mut pair_id: HashMap<(usize, usize), usize> = HashMap::new();
    let mut pair_pos: Vec<usize> = Vec::new(); // pair id -> position
    let mut pair_q: Vec<usize> = Vec::new();
    let mut prod_adj: Vec<Vec<usize>> = Vec::new();
    let mut parent_pair: Vec<Option<usize>> = Vec::new();
    let start_pid = 0usize;
    pair_id.insert((0, start_q), start_pid);
    pair_pos.push(0);
    pair_q.push(start_q);
    prod_adj.push(Vec::new());
    parent_pair.push(None);
    let mut queue: Vec<usize> = vec![start_pid];
    let mut qi = 0usize;
    while qi < queue.len() {
        let pid = queue[qi];
        qi += 1;
        let p = pair_pos[pid];
        let q = pair_q[pid];
        for &p2 in &pos_succ[p] {
            let q2 = prod_delta(q, &places, &radices, &leaves, bits_at[p2]);
            let key = (p2, q2);
            if let Some(&pid2) = pair_id.get(&key) {
                prod_adj[pid].push(pid2);
            } else {
                let npid = pair_pos.len();
                pair_id.insert(key, npid);
                pair_pos.push(p2);
                pair_q.push(q2);
                prod_adj.push(Vec::new());
                parent_pair.push(Some(pid));
                queue.push(npid);
                prod_adj[pid].push(npid);
            }
        }
    }

    // --- SCCs of the reachable product graph (iterative Tarjan). ---
    let npairs = pair_pos.len();
    let mut index_of = vec![usize::MAX; npairs];
    let mut lowlink = vec![0usize; npairs];
    let mut on_stack = vec![false; npairs];
    let mut scc_id = vec![usize::MAX; npairs];
    let mut counter = 0usize;
    let mut stack: Vec<usize> = Vec::new();
    let mut sccs: Vec<Vec<usize>> = Vec::new();
    for root in 0..npairs {
        if index_of[root] != usize::MAX {
            continue;
        }
        // Iterative DFS: (node, next-child-index).
        let mut work: Vec<(usize, usize)> = vec![(root, 0)];
        index_of[root] = counter;
        lowlink[root] = counter;
        counter += 1;
        stack.push(root);
        on_stack[root] = true;
        while let Some(&(node, ci)) = work.last() {
            if ci < prod_adj[node].len() {
                let w = prod_adj[node][ci];
                work.last_mut().unwrap().1 = ci + 1;
                if index_of[w] == usize::MAX {
                    index_of[w] = counter;
                    lowlink[w] = counter;
                    counter += 1;
                    stack.push(w);
                    on_stack[w] = true;
                    work.push((w, 0));
                } else if on_stack[w] {
                    lowlink[node] = lowlink[node].min(index_of[w]);
                }
            } else {
                work.pop();
                if let Some(&(parent_node, _)) = work.last() {
                    lowlink[parent_node] = lowlink[parent_node].min(lowlink[node]);
                }
                if lowlink[node] == index_of[node] {
                    let mut comp = Vec::new();
                    loop {
                        let w = stack.pop().unwrap();
                        on_stack[w] = false;
                        scc_id[w] = sccs.len();
                        comp.push(w);
                        if w == node {
                            break;
                        }
                    }
                    sccs.push(comp);
                }
            }
        }
    }

    // --- On-cycle pairs: non-trivial SCC members + self-loop singletons. ---
    let mut on_cycle = vec![false; npairs];
    for comp in &sccs {
        if comp.len() > 1 {
            for &p in comp {
                on_cycle[p] = true;
            }
        } else {
            let p = comp[0];
            if prod_adj[p].contains(&p) {
                on_cycle[p] = true;
            }
        }
    }

    // --- Find a falsifying limit vector (q ascending, discovery order). ---
    let kinds: Vec<u8> = leaves.iter().map(|l| l.kind).collect();
    for q in 0..qcount {
        let t: Vec<bool> = (0..n)
            .map(|k| leaf_good(kinds[k], (q / places[k]) % radices[k]))
            .collect();
        if skel_eval(&prop.skeleton, &t) {
            continue; // satisfied for this limit vector
        }
        let start = pair_id.get(&(0, start_q)).copied().unwrap_or(usize::MAX);
        let mut witness_pair: Option<usize> = None;
        for pid in start..npairs {
            if on_cycle[pid] && pair_q[pid] == q {
                witness_pair = Some(pid);
                break;
            }
        }
        let Some(wpid) = witness_pair else { continue };
        // Prefix: product path from the start pair to wpid (parent pointers).
        // Pairs P0..Pa with Pa = wpid; the edge taken by a product transition
        // X -> Y is the edge at position pos(Y), so the lasso's PREFIX edges
        // are the positions of P1..P_{a-1} (wpid's own edge starts the cycle).
        let mut prefix_pids: Vec<usize> = vec![wpid];
        let mut cur = wpid;
        while let Some(pp) = parent_pair[cur] {
            prefix_pids.push(pp);
            cur = pp;
        }
        prefix_pids.reverse();

        let mut lasso: Vec<(usize, usize, usize)> = Vec::new();
        for &pid in prefix_pids.iter().skip(1) {
            if pid == wpid {
                continue; // wpid's edge belongs to the cycle (below)
            }
            let (sc, si) = pos_edge[pair_pos[pid]].expect("prefix pair sits on an edge position");
            let (iidx, tid, _t) = adj[sc][si];
            lasso.push((sc, iidx, tid));
        }
        let cycle_start = lasso.len();

        // Cycle edges in order: wpid's own edge p_w=(u_w -> v_w) FIRST (after
        // the prefix we sit at u_w), then one edge per DFS descent. The
        // closing transition back to wpid re-takes p_w, so it is not recorded
        // again.
        {
            let (sc, si) = pos_edge[pair_pos[wpid]].expect("edge position");
            let (iidx, tid, _t) = adj[sc][si];
            lasso.push((sc, iidx, tid));
        }
        // DFS for the closing path back to wpid. GLOBAL visited pruning
        // (not just the current path): every reachable node is explored once,
        // so the search is O(V+E) - path-only pruning re-explores diamond
        // subgraphs exponentially and spins forever on small graphs (Phase 6
        // task 4 finding, specgen seed 900_006). Edges to wpid are checked
        // before the visited test, so any cycle through wpid is still found.
        let mut rec: Vec<usize> = Vec::new();
        let mut visited: HashSet<usize> = HashSet::new();
        visited.insert(wpid);
        let mut dfs: Vec<(usize, usize)> = vec![(wpid, 0)];
        let mut found = false;
        while let Some(&(at, ci)) = dfs.last() {
            if ci < prod_adj[at].len() {
                let w = prod_adj[at][ci];
                dfs.last_mut().unwrap().1 = ci + 1;
                if w == wpid {
                    // Closing transition re-takes p_w - already pushed first.
                    found = true;
                    break;
                }
                if visited.insert(w) {
                    rec.push(pair_pos[w]);
                    dfs.push((w, 0));
                }
            } else {
                let (_node, _) = dfs.pop().unwrap();
                if !dfs.is_empty() {
                    rec.pop();
                }
            }
        }
        if !found {
            continue; // should not happen (wpid is on a product cycle)
        }
        for &p in &rec {
            let (sc, si) = pos_edge[p].expect("cycle pair sits on an edge position");
            let (iidx, tid, _t) = adj[sc][si];
            lasso.push((sc, iidx, tid));
        }
        return Some(LivWitness { lasso, cycle_start });
    }
    None
}

/// Mixed-radix transition of the per-leaf progress automata on label bits.
/// Leaf states: safety 0=ok/1=bad; eventually 0=wait/1=done;
/// until 0=ok/1=sat/2=bad. All transitions are one-way, so every state
/// sequence on an infinite path is eventually constant (the limit vector).
fn prod_delta(
    q: usize,
    places: &[usize],
    radices: &[usize],
    leaves: &[LeafSig],
    bits: u32,
) -> usize {
    let mut out = 0usize;
    for (k, &r) in radices.iter().enumerate() {
        let s = (q / places[k]) % r;
        let a = (bits >> (2 * k)) & 1 == 1;
        let b = (bits >> (2 * k + 1)) & 1 == 1;
        let ns = match leaves[k].kind {
            0 => {
                if s == 1 || !a {
                    1
                } else {
                    0
                }
            }
            1 => {
                if s == 1 || a {
                    1
                } else {
                    0
                }
            }
            _ => match s {
                2 => 2,
                1 => 1,
                _ => {
                    if b {
                        1
                    } else if !a {
                        2
                    } else {
                        0
                    }
                }
            },
        };
        out = out * r + ns;
    }
    out
}

/// A leaf is satisfied on an infinite path iff its limit state is in the good
/// set.
fn leaf_good(kind: u8, s: usize) -> bool {
    match kind {
        0 => s == 0, // safety: never entered bad
        1 => s == 1, // eventually: done reached
        _ => s != 2, // until: ok (P-forever branch) or sat
    }
}

/// Label bits of one sample for the property's leaves: bit 2k = atom A,
/// bit 2k+1 = atom B (`until` right operand).
fn leaf_bits(leaves: &[LeafSig], s: &Sample, module: &Module) -> Result<u32, String> {
    let mut bits = 0u32;
    for (k, leaf) in leaves.iter().enumerate() {
        if eval_sig(&leaf.a, s, module)? {
            bits |= 1 << (2 * k);
        }
        if let Some(b) = &leaf.b {
            if eval_sig(b, s, module)? {
                bits |= 1 << (2 * k + 1);
            }
        }
    }
    Ok(bits)
}

/// Human-facing explanation of which leaves are falsified on the witness.
fn liv_note(
    module: &Module,
    nodes: &[BNode],
    inputs: &[InputVal],
    _in_variants: &[FlatVariant],
    _out_variants: &[FlatVariant],
    prop: &LivProp,
    w: &LivWitness,
) -> String {
    let leaves = leaf_signals(prop);
    let sample_at = |pos: usize| -> Result<Sample, String> {
        if pos == 0 {
            Ok(Sample {
                input: None,
                outputs: Vec::new(),
                to_state: nodes[0].state,
                vars: nodes[0].vars.clone(),
            })
        } else {
            let (cid, iidx, _tid) = w.lasso[pos - 1];
            let node = &nodes[cid];
            let input = inputs[iidx].clone();
            let eff = match_effect(module, node.state, &node.vars, &input)
                .ok_or_else(|| "lasso replay failed".to_string())?;
            Ok(Sample {
                input: Some(input),
                outputs: eff.outputs.clone(),
                to_state: eff.next_state,
                vars: eff.next_vars.clone(),
            })
        }
    };
    // Falsified leaves: those whose limit state is not good on the witness.
    let kinds: Vec<u8> = leaves.iter().map(|l| l.kind).collect();
    let radices: Vec<usize> = leaves
        .iter()
        .map(|l| if l.kind == 2 { 3 } else { 2 })
        .collect();
    let mut places = vec![1usize; leaves.len()];
    for k in (0..leaves.len() - 1).rev() {
        places[k] = places[k + 1] * radices[k + 1];
    }
    // Determine the limit vector from one cycle position (constant on the
    // cycle): run the automata over prefix + cycle and read the final states.
    let mut q = 0usize;
    q = prod_delta(
        q,
        &places,
        &radices,
        &leaves,
        leaf_bits(&leaves, &sample_at(0).unwrap(), module).unwrap(),
    );
    for pos in 1..=w.lasso.len() {
        let s = sample_at(pos).expect("valid lasso position");
        q = prod_delta(
            q,
            &places,
            &radices,
            &leaves,
            leaf_bits(&leaves, &s, module).expect("labels"),
        );
    }
    let falsified: Vec<usize> = (0..leaves.len())
        .filter(|&k| !leaf_good(kinds[k], (q / places[k]) % radices[k]))
        .collect();
    let mut parts: Vec<String> = Vec::new();
    for &k in &falsified {
        let leaf = &leaves[k];
        match leaf.kind {
            0 => {
                let p = (0..=w.lasso.len())
                    .find(|&pos| !eval_sig(&leaf.a, &sample_at(pos).unwrap(), module).unwrap())
                    .expect("safety leaf falsified on the witness");
                parts.push(format!("safety subformula false at step {p}"));
            }
            1 => parts.push("eventually body never holds on the path".to_string()),
            _ => {
                let b = leaf.b.as_ref().expect("until has a right operand");
                let mut seen_q = false;
                let mut p = None;
                for pos in 0..=w.lasso.len() {
                    let s = sample_at(pos).unwrap();
                    if !eval_sig(&leaf.a, &s, module).unwrap()
                        && !(seen_q || eval_sig(b, &s, module).unwrap())
                    {
                        p = Some(pos);
                        break;
                    }
                    seen_q |= eval_sig(b, &s, module).unwrap();
                }
                parts.push(format!(
                    "until left fails at step {} without right holding",
                    p.unwrap()
                ));
            }
        }
    }
    parts.join("; ")
}

// ---------------------------------------------------------------------------
// Report rendering (unified with the generated engines, D-P6-10)
// ---------------------------------------------------------------------------

/// Render one value in the Debug form the generated types print:
/// bool -> `true`/`false`; bit<N> -> decimal; enum -> `Enum::Variant`.
fn render_val(v: &Val, ty: &Type) -> String {
    match (v, ty) {
        (Val::Bool(b), _) => b.to_string(),
        (Val::Bit(n), _) => n.to_string(),
        (Val::Enum(name), Type::Enum { name: en }) => format!("{en}::{name}"),
        // Mismatches are internal invariant failures (validated specs).
        other => panic!("value/type mismatch in report rendering: {other:?}"),
    }
}

fn render_vars(module: &Module, vars: &[Val]) -> String {
    if module.vars.is_empty() {
        return "Vars".to_string();
    }
    let fields: Vec<String> = module
        .vars
        .iter()
        .zip(vars.iter())
        .map(|(v, val)| format!("{}: {}", v.name, render_val(val, &v.ty)))
        .collect();
    format!("Vars {{ {} }}", fields.join(", "))
}

/// Render one input in `Input::...` Debug form.
fn render_input(io: &IoRec, variants: &[FlatVariant]) -> String {
    if io.fields.is_empty() {
        return format!("Input::{}", io.variant);
    }
    let vi = variants
        .iter()
        .find(|v| v.unique_name == io.variant)
        .expect("known variant");
    let fields: Vec<String> = vi
        .fields
        .iter()
        .zip(io.fields.iter())
        .map(|((_, ty), (name, val))| format!("{name}: {}", render_val(val, ty)))
        .collect();
    format!("Input::{} {{ {} }}", io.variant, fields.join(", "))
}

/// Render one output in `Output::...` Debug form.
fn render_output(io: &IoRec, variants: &[FlatVariant]) -> String {
    if io.fields.is_empty() {
        return format!("Output::{}", io.variant);
    }
    let vi = variants
        .iter()
        .find(|v| v.unique_name == io.variant)
        .expect("known variant");
    let fields: Vec<String> = vi
        .fields
        .iter()
        .zip(io.fields.iter())
        .map(|((_, ty), (name, val))| format!("{name}: {}", render_val(val, ty)))
        .collect();
    format!("Output::{} {{ {} }}", io.variant, fields.join(", "))
}

fn render_outputs(outs: &[IoRec], variants: &[FlatVariant]) -> String {
    if outs.is_empty() {
        return "[]".to_string();
    }
    let items: Vec<String> = outs.iter().map(|o| render_output(o, variants)).collect();
    format!("[{}]", items.join(", "))
}

/// The full text report - byte-identical FORMAT to the generated exact-mode
/// binaries' default (text) report. `module` is required for value types.
pub fn render_text(res: &RefResult, module: &Module) -> String {
    let in_variants = flatten(&module.events);
    let out_variants = flatten(&module.outputs);
    render_text_impl(res, module, &in_variants, &out_variants)
}

fn render_text_impl(
    res: &RefResult,
    module: &Module,
    in_variants: &[FlatVariant],
    out_variants: &[FlatVariant],
) -> String {
    let mut out = String::new();
    match res.status {
        ExactStatus::PassComplete => out.push_str("PASS (complete)\n"),
        ExactStatus::Fail => out.push_str("FAIL\n"),
        ExactStatus::UnknownStateLimit => {
            if let Some(secs) = res.timeout_secs {
                out.push_str(&format!(
                    "UNKNOWN (state limit reached, timeout after {secs}s)\n"
                ));
            } else {
                out.push_str("UNKNOWN (state limit reached)\n");
            }
        }
    }
    let fail = res.fail.as_ref().map(|(r, l, s)| (*r, l.as_str(), *s));
    out.push_str(&exact_summary_line(
        res.status,
        res.states,
        res.transitions,
        res.max_depth,
        fail,
    ));
    out.push('\n');
    out.push_str(&format!(
        "  unexpected terminations: {}\n",
        res.unexpected_terminations
    ));
    if let Some((cid, iidx)) = res.first_unexpected {
        out.push_str(&format!(
            "  first unexpected: config {cid} input index {iidx}\n"
        ));
    }
    out.push_str(&format!("  monitor size: {} bits\n", res.monitor_size_bits));
    out.push_str(&format!(
        "  memory estimate: {} bytes\n",
        res.memory_estimate_bytes
    ));

    let Some((reason, label, step)) = &res.fail else {
        return out;
    };
    let ce = res.ce.as_ref().expect("fail carries a counterexample");
    match (ce, reason) {
        (
            CeData::Lasso {
                formula_text,
                cycle_start,
                note,
                steps,
            },
            FailReason::LivenessViolation,
        ) => {
            out.push_str(&format!("liveness violation: {label}\n"));
            out.push_str(&format!("  formula: {formula_text}\n"));
            out.push_str(&format!("  violation step: {step}\n"));
            out.push_str(&format!("  cycle length: {}\n", steps.len() - cycle_start));
            out.push_str(&format!("  note: {note}\n"));
        }
        (CeData::Deadlock { .. }, FailReason::Deadlock) => {
            out.push_str(&format!(
                "deadlock: configuration at depth {step} has no enabled transition\n"
            ));
        }
        (
            CeData::Trace {
                trigger_step,
                steps,
            },
            FailReason::PropertyViolation,
        ) => {
            if steps.is_empty() {
                out.push_str(&format!("property violation: {label}\n"));
                if let Some(t) = trigger_step {
                    out.push_str(&format!("  trigger step: {t} (initial configuration)\n"));
                }
                out.push_str(&format!(
                    "  violation step: {step} (initial configuration)\n"
                ));
            } else {
                out.push_str(&format!("property violation: {label}\n"));
                if let Some(t) = trigger_step {
                    out.push_str(&format!("  trigger step: {t}\n"));
                    let vars_at_t = if *t == 0 {
                        &res.initial_vars
                    } else {
                        &steps[*t - 1].vars
                    };
                    out.push_str(&format!(
                        "  vars at trigger step: {}\n",
                        render_vars(module, vars_at_t)
                    ));
                }
                out.push_str(&format!("  violation step: {step}\n"));
                let vars_at_s = if *step == 0 {
                    &res.initial_vars
                } else {
                    &steps[*step - 1].vars
                };
                out.push_str(&format!(
                    "  vars at violation step: {}\n",
                    render_vars(module, vars_at_s)
                ));
            }
        }
        _ => unreachable!("ce/reason mismatch"),
    }

    let steps: &[StepRec] = match ce {
        CeData::Trace { steps, .. }
        | CeData::Lasso { steps, .. }
        | CeData::Deadlock { steps, .. } => steps,
    };
    out.push_str("counterexample trace:\n");
    out.push_str(&format!(
        "  initial state=State::{} vars={}\n",
        res.initial_state,
        render_vars(module, &res.initial_vars)
    ));
    for (i, st) in steps.iter().enumerate() {
        out.push_str(&format!(
            "  step {} input={} outputs={} from=State::{} to=State::{} vars={}\n",
            i,
            render_input(&st.input, in_variants),
            render_outputs(&st.outputs, out_variants),
            st.from_state,
            st.to_state,
            render_vars(module, &st.vars)
        ));
    }
    out
}

/// Build the canonical JSON report document (same model + key order as the
/// generated engines' `--counterexample-format json` output).
pub fn to_json_report(res: &RefResult, module: &Module) -> McJsonReport {
    use crate::json_rw::Json;

    let val_json = |v: &Val, ty: &Type| -> Json {
        match (v, ty) {
            (Val::Bool(b), _) => Json::Bool(*b),
            (Val::Bit(n), _) => Json::Int(*n as i64), // bit<64> values above
            // i64::MAX cannot be
            // represented in the
            // shared i64 model; exact
            // MC does not enumerate
            // such input fields, so
            // this is unreachable for
            // byte-parity-relevant
            // specs (documented).
            (Val::Enum(name), Type::Enum { name: en }) => Json::Str(format!("{en}::{name}")),
            other => panic!("value/type mismatch in JSON report: {other:?}"),
        }
    };
    let vars_json = |vars: &[Val]| -> Vec<(String, Json)> {
        module
            .vars
            .iter()
            .zip(vars.iter())
            .map(|(v, val)| (v.name.clone(), val_json(val, &v.ty)))
            .collect()
    };
    let io_json = |io: &IoRec, variants: &[FlatVariant]| -> JsonIo {
        let vi = variants
            .iter()
            .find(|v| v.unique_name == io.variant)
            .expect("io record names a known variant");
        let fields = vi
            .fields
            .iter()
            .zip(io.fields.iter())
            .map(|((_, ty), (fname, val))| (fname.clone(), val_json(val, ty)))
            .collect();
        JsonIo {
            variant: io.variant.clone(),
            fields,
        }
    };
    let step_json = |s: &StepRec| -> JsonStep {
        JsonStep {
            input: io_json(&s.input, &flatten(&module.events)),
            outputs: s
                .outputs
                .iter()
                .map(|o| io_json(o, &flatten(&module.outputs)))
                .collect(),
            from_state: format!("State::{}", s.from_state),
            to_state: format!("State::{}", s.to_state),
            vars: vars_json(&s.vars),
        }
    };

    let counterexample = match (&res.fail, &res.ce) {
        (Some((FailReason::PropertyViolation, _, _)), Some(CeData::Trace { steps, .. })) => {
            Some(JsonCounterexample::Trace {
                initial_state: format!("State::{}", res.initial_state),
                initial_vars: vars_json(&res.initial_vars),
                steps: steps.iter().map(step_json).collect(),
            })
        }
        (
            Some((FailReason::LivenessViolation, _, _)),
            Some(CeData::Lasso {
                cycle_start, steps, ..
            }),
        ) => {
            let all: Vec<JsonStep> = steps.iter().map(step_json).collect();
            let (prefix, cycle) = all.split_at(*cycle_start);
            Some(JsonCounterexample::Lasso {
                initial_state: format!("State::{}", res.initial_state),
                initial_vars: vars_json(&res.initial_vars),
                prefix: prefix.to_vec(),
                cycle: cycle.to_vec(),
                cycle_length: cycle.len() as i64,
            })
        }
        (Some((FailReason::Deadlock, _, depth)), Some(CeData::Deadlock { steps, .. })) => {
            let (state, vars) = if steps.is_empty() {
                (
                    format!("State::{}", res.initial_state),
                    vars_json(&res.initial_vars),
                )
            } else {
                let last = &steps[steps.len() - 1];
                (format!("State::{}", last.to_state), vars_json(&last.vars))
            };
            Some(JsonCounterexample::DeadlockConfig {
                depth: *depth as i64,
                state,
                vars,
            })
        }
        _ => None,
    };

    McJsonReport {
        status: res.status,
        states: res.states as i64,
        transitions: res.transitions as i64,
        max_depth: res.max_depth as i64,
        fail: res
            .fail
            .as_ref()
            .map(|(r, l, s)| (*r, l.clone(), *s as i64)),
        unexpected_terminations: res.unexpected_terminations as i64,
        monitor_size_bits: res.monitor_size_bits as i64,
        memory_estimate_bytes: res.memory_estimate_bytes as i64,
        counterexample,
    }
}

// ---------------------------------------------------------------------------
// Minimal-violation labels (order-independent ground truth for task 4.1)
// ---------------------------------------------------------------------------

/// The sorted set of property labels violated at the GLOBAL MINIMAL depth,
/// independent of traversal order:
///
/// - safety violations count at their confirmation sample;
/// - liveness witnesses count at their cycle start position;
/// - deadlocks (label `-`) are excluded - they are covered by status
///   agreement in the differential suites.
///
/// The search is complete up to the first violation found in canonical order
/// (deeper violations cannot be shallower), so the set is exact for the
/// explored graph; with a truncating `max_states` it covers everything the
/// generated engines would have explored before stopping (sound: every label
/// returned has a real counterexample).
pub fn minimal_violation_labels(module: &Module, cfg: &RefConfig) -> Result<Vec<String>, String> {
    // Phase 1: canonical run to find the first violation in traversal order.
    let first = run(module, cfg)?;
    let Some((_, _, first_step)) = first.fail else {
        return Ok(Vec::new()); // pass-complete or unknown: no violations found
    };
    // Phase 2: re-run BFS but keep processing levels until level
    // `first_step - 1` is fully expanded, collecting every violation
    // confirmed at the minimal step.
    let mut labels: HashSet<String> = HashSet::new();
    collect_min_labels(module, cfg, first_step, &mut labels)?;

    // Liveness: witnesses from phase-1's graph (its cycle starts are exact
    // for the explored subgraph); include any at depth == the global min.
    let mut all: Vec<String> = labels.into_iter().collect();
    if let Some((FailReason::LivenessViolation, label, step)) = first.fail {
        if step <= first_step {
            all.push(label);
        }
    }
    // If the canonical run reported a liveness violation at `first_step` but
    // phase 2 found no safety tie, the liveness label still belongs.
    let mut seen: HashSet<String> = HashSet::new();
    let mut out: Vec<String> = all.into_iter().filter(|l| seen.insert(l.clone())).collect();
    out.sort();
    Ok(out)
}

/// BFS that does NOT stop at the first violation; collects labels of all
/// violations confirmed at step `target` (and records whether any shallower
/// violation exists - impossible by construction of `target`).
fn collect_min_labels(
    module: &Module,
    cfg: &RefConfig,
    target: usize,
    out: &mut HashSet<String>,
) -> Result<(), String> {
    let known: Vec<&str> = module.assumptions.iter().map(|a| a.name.as_str()).collect();
    let selected = parse_assume_selection(&cfg.assume_arg, &known);
    let mut prop_mons: Vec<(String, Mon)> = Vec::new();
    for p in &module.properties {
        if contains_liveness(&p.formula) {
            continue;
        }
        if let Ok(mon) = compile_formula(&p.formula) {
            prop_mons.push((p.name.clone(), mon));
        }
    }
    let mut assume_mons: Vec<(String, Mon)> = Vec::new();
    for a in &module.assumptions {
        if !selected.iter().any(|x| x == &a.name) {
            continue;
        }
        if let Ok(mon) = compile_formula(&a.formula) {
            assume_mons.push((a.name.clone(), mon));
        }
    }
    let all_mons: Vec<(String, Mon)> = prop_mons
        .iter()
        .cloned()
        .chain(assume_mons.iter().cloned())
        .collect();
    let n_props = prop_mons.len();
    let mon_specs: Vec<&Mon> = all_mons.iter().map(|(_, m)| m).collect();

    let states = state_variants(module).to_vec();
    let q0 = module
        .initial_state
        .as_ref()
        .and_then(|s| states.iter().position(|x| x == s))
        .ok_or_else(|| "module has no initial state".to_string())?;
    let mut init_vars: Vec<Val> = Vec::new();
    for v in &module.vars {
        let ctx = EvalCtx {
            module,
            state: Val::Enum(states[q0].clone()),
            vars: HashMap::new(),
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        };
        init_vars.push(eval_expr(&v.init, &ctx)?);
    }

    let inputs = all_inputs(module);
    // Step the monitors at sample 0 (as `run` does): countdowns and one-shot
    // obligations must start from the initial configuration. No NEW failure
    // can occur here when target > 0 (run() would have reported it).
    let sample0 = Sample {
        input: None,
        outputs: Vec::new(),
        to_state: q0,
        vars: init_vars.clone(),
    };
    let mut mons0: Vec<MonSt> = mon_specs.iter().copied().map(mon_init).collect();
    let mut failed_at_0: Vec<bool> = Vec::new();
    for (i, (_, spec)) in all_mons.iter().enumerate() {
        failed_at_0.push(mon_step(&mut mons0[i], spec, 0, &sample0, module)?);
    }
    let node0 = BNode {
        state: q0,
        vars: init_vars,
        mons: mons0,
    };
    // A sample-0 violation is the minimal one: collect EVERY monitorable
    // property whose monitor fails at sample 0 (run() reports only the first
    // in declaration order). Selected assumptions cannot be failing here -
    // run() would have returned Err for a sample-0 assumption violation.
    if target == 0 {
        for ((label, _), failed) in prop_mons.iter().zip(failed_at_0.iter()) {
            if *failed {
                out.insert(label.clone());
            }
        }
        return Ok(());
    }

    let mut visited: HashMap<CfgKey, usize> = HashMap::new();
    let mut nodes: Vec<BNode> = vec![node0];
    visited.insert(bnode_key(&nodes[0], module, &mon_specs), 0);
    let mut frontier: Vec<usize> = vec![0];
    let mut depth_of: Vec<usize> = vec![0];

    while !frontier.is_empty() {
        let level_depth = depth_of[frontier[0]];
        if level_depth + 1 > target {
            break; // all further violations are deeper than `target`
        }
        let mut next_frontier: Vec<usize> = Vec::new();
        for &cid in &frontier {
            let node = nodes[cid].clone();
            for input in inputs.iter() {
                let Some(eff) = match_effect(module, node.state, &node.vars, input) else {
                    continue;
                };
                let sample = Sample {
                    input: Some(input.clone()),
                    outputs: eff.outputs.clone(),
                    to_state: eff.next_state,
                    vars: eff.next_vars.clone(),
                };
                let mut mons = node.mons.clone();
                let mut pruned = false;
                for (mi, (_, spec)) in all_mons.iter().enumerate().skip(n_props) {
                    if mon_step(
                        &mut mons[mi],
                        spec,
                        (level_depth + 1) as u64,
                        &sample,
                        module,
                    )? {
                        pruned = true;
                        break;
                    }
                }
                if !pruned {
                    for (mi, (label, spec)) in all_mons.iter().enumerate().take(n_props) {
                        if mon_step(
                            &mut mons[mi],
                            spec,
                            (level_depth + 1) as u64,
                            &sample,
                            module,
                        )? && level_depth + 1 == target
                        {
                            out.insert(label.clone());
                        }
                    }
                }
                let next = BNode {
                    state: eff.next_state,
                    vars: eff.next_vars,
                    mons,
                };
                let key = bnode_key(&next, module, &mon_specs);
                if !visited.contains_key(&key) && nodes.len() < cfg.max_states {
                    visited.insert(key, nodes.len());
                    depth_of.push(level_depth + 1);
                    next_frontier.push(nodes.len());
                    nodes.push(next);
                } else if !visited.contains_key(&key) {
                    // State cap reached: deeper exploration is impossible; the
                    // target level was already fully processed above.
                    return Ok(());
                }
            }
        }
        frontier = next_frontier;
    }
    Ok(())
}

// ---------------------------------------------------------------------------
// Counterexample replay and verification (Phase 6 task 4.1)
//
// Public API for the differential suite: `step_trace` replays a fixed input
// sequence through ref_mc's OWN transition logic, and
// `verify_counterexample` validates a counterexample parsed from a generated
// engine's JSON report against ref_mc's independent evaluators (transition
// relation, monitor compilers, liveness product automaton). This is the
// cross-validation of task 4.1: the engine's CE must be a valid execution
// that genuinely exhibits the claimed violation at the claimed step.
// ---------------------------------------------------------------------------

/// The module's initial configuration: (state index, variable values).
fn initial_config(module: &Module) -> Result<(usize, Vec<Val>), String> {
    let states = state_variants(module).to_vec();
    let q0 = module
        .initial_state
        .as_ref()
        .and_then(|s| states.iter().position(|x| x == s))
        .ok_or_else(|| "module has no initial state".to_string())?;
    let mut init_vars: Vec<Val> = Vec::new();
    for v in &module.vars {
        let ctx = EvalCtx {
            module,
            state: Val::Enum(states[q0].clone()),
            vars: HashMap::new(),
            binds: HashMap::new(),
            bind_types: HashMap::new(),
        };
        init_vars.push(
            eval_expr(&v.init, &ctx)
                .map_err(|e| format!("variable '{}' initializer failed: {e}", v.name))?,
        );
    }
    Ok((q0, init_vars))
}

/// One JSON scalar of a counterexample report (field/variable value) to a
/// [`Val`]: Bool -> Bool, Int -> Bit, Str -> Enum variant name. The wire form
/// of an enum value is `EnumType::Variant` (see `to_json_report`); [`Val`]
/// carries the bare variant name.
fn json_val(j: &crate::json_rw::Json, what: &str) -> Result<Val, String> {
    match j {
        crate::json_rw::Json::Bool(b) => Ok(Val::Bool(*b)),
        crate::json_rw::Json::Int(n) => Ok(Val::Bit(*n as u64)),
        crate::json_rw::Json::Str(s) => Ok(Val::Enum(
            s.rsplit_once("::")
                .map(|(_, v)| v.to_string())
                .unwrap_or_else(|| s.clone()),
        )),
        other => Err(format!("{what}: unsupported JSON value {other:?}")),
    }
}

/// A counterexample input/output record to a concrete [`InputVal`]: the
/// variant must exist in the module's flattened constructors and the fields
/// must match the declaration (names, order, count).
fn io_to_input(module: &Module, io: &JsonIo) -> Result<InputVal, String> {
    let variants = flatten(&module.events);
    let vi = variants
        .iter()
        .position(|v| v.unique_name == io.variant)
        .ok_or_else(|| format!("unknown input variant '{}'", io.variant))?;
    if variants[vi].fields.len() != io.fields.len() {
        return Err(format!(
            "input {}: {} field(s) in CE, {} declared",
            io.variant,
            io.fields.len(),
            variants[vi].fields.len()
        ));
    }
    let fields = io
        .fields
        .iter()
        .zip(&variants[vi].fields)
        .map(|((name, j), (dname, _))| {
            if name != dname {
                return Err(format!(
                    "input {}: field '{name}' not declared (expected '{dname}')",
                    io.variant
                ));
            }
            json_val(j, &format!("input {} field {name}", io.variant))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(InputVal {
        variant: vi,
        fields,
    })
}

fn io_to_output(module: &Module, io: &JsonIo) -> Result<OutputVal, String> {
    let variants = flatten(&module.outputs);
    let vi = variants
        .iter()
        .position(|v| v.unique_name == io.variant)
        .ok_or_else(|| format!("unknown output variant '{}'", io.variant))?;
    if variants[vi].fields.len() != io.fields.len() {
        return Err(format!(
            "output {}: {} field(s) in CE, {} declared",
            io.variant,
            io.fields.len(),
            variants[vi].fields.len()
        ));
    }
    let fields = io
        .fields
        .iter()
        .zip(&variants[vi].fields)
        .map(|((name, j), (dname, _))| {
            if name != dname {
                return Err(format!(
                    "output {}: field '{name}' not declared (expected '{dname}')",
                    io.variant
                ));
            }
            json_val(j, &format!("output {} field {name}", io.variant))
        })
        .collect::<Result<Vec<_>, _>>()?;
    Ok(OutputVal {
        variant: vi,
        fields,
    })
}

/// Replay a fixed input sequence through the module using ref_mc's own
/// transition logic (deterministic first-match firing + `on_error` fallback).
/// Returns the sample sequence: sample 0 = initial configuration, sample k =
/// step k. `Err` when some input has no enabled transition and no `on_error`
/// (the generated production FSM would report UnexpectedEvent there).
pub fn step_trace(module: &Module, inputs: &[InputVal]) -> Result<Vec<Sample>, String> {
    let (q0, mut vars) = initial_config(module)?;
    let mut state = q0;
    let mut samples = vec![Sample {
        input: None,
        outputs: Vec::new(),
        to_state: state,
        vars: vars.clone(),
    }];
    for (k, input) in inputs.iter().enumerate() {
        let eff = match_effect(module, state, &vars, input).ok_or_else(|| {
            format!(
                "CE step {} is not a valid execution (no enabled transition)",
                k + 1
            )
        })?;
        samples.push(Sample {
            input: Some(input.clone()),
            outputs: eff.outputs,
            to_state: eff.next_state,
            vars: eff.next_vars.clone(),
        });
        state = eff.next_state;
        vars = eff.next_vars;
    }
    Ok(samples)
}

/// CE state strings on the wire carry the `State::` prefix (ref_mc's
/// prefixed form, byte-identical across engines); strip it for comparison
/// against bare variant names.
fn ce_state_name(s: &str) -> &str {
    s.strip_prefix("State::").unwrap_or(s)
}

/// The CE's claimed initial configuration must equal the module's.
fn check_initial(
    module: &Module,
    initial_state: &str,
    initial_vars: &[(String, crate::json_rw::Json)],
) -> Result<(), String> {
    let (q0, vars0) = initial_config(module)?;
    if state_name(module, q0) != ce_state_name(initial_state) {
        return Err(format!(
            "CE initial state '{initial_state}' != module initial '{}'",
            state_name(module, q0)
        ));
    }
    if initial_vars.len() != module.vars.len() {
        return Err(format!(
            "CE carries {} initial var(s), module declares {}",
            initial_vars.len(),
            module.vars.len()
        ));
    }
    for (pos, ((name, j), v)) in initial_vars.iter().zip(&vars0).enumerate() {
        if &module.vars[pos].name != name {
            return Err(format!(
                "CE initial vars: position {pos} claims '{name}', module declares '{}'",
                module.vars[pos].name
            ));
        }
        let dv = json_val(j, &format!("initial var {name}"))?;
        if &dv != v {
            return Err(format!(
                "CE initial var {name}: claimed {dv:?}, module has {v:?}"
            ));
        }
    }
    Ok(())
}

/// One replayed step must match the CE's claimed from/to state, variables and
/// outputs.
fn check_step_config(
    module: &Module,
    prev_state: usize,
    k: usize,
    step: &JsonStep,
    sample: &Sample,
) -> Result<(), String> {
    if state_name(module, prev_state) != ce_state_name(&step.from_state) {
        return Err(format!(
            "CE step {}: claims from_state='{}', replay is at '{}'",
            k + 1,
            step.from_state,
            state_name(module, prev_state)
        ));
    }
    if state_name(module, sample.to_state) != ce_state_name(&step.to_state) {
        return Err(format!(
            "CE step {}: claims to_state='{}', replay reaches '{}'",
            k + 1,
            step.to_state,
            state_name(module, sample.to_state)
        ));
    }
    if step.vars.len() != module.vars.len() {
        return Err(format!(
            "CE step {}: {} var(s) in CE, {} declared",
            k + 1,
            step.vars.len(),
            module.vars.len()
        ));
    }
    for (pos, ((name, j), v)) in step.vars.iter().zip(&sample.vars).enumerate() {
        if &module.vars[pos].name != name {
            return Err(format!(
                "CE step {}: position {pos} claims var '{name}', module declares '{}'",
                k + 1,
                module.vars[pos].name
            ));
        }
        let dv = json_val(j, &format!("CE step {} var {name}", k + 1))?;
        if &dv != v {
            return Err(format!(
                "CE step {}: var {name} claimed {dv:?}, replay has {v:?}",
                k + 1
            ));
        }
    }
    if step.outputs.len() != sample.outputs.len() {
        return Err(format!(
            "CE step {}: {} output(s) in CE, replay emits {}",
            k + 1,
            step.outputs.len(),
            sample.outputs.len()
        ));
    }
    for (jio, o) in step.outputs.iter().zip(&sample.outputs) {
        let dv = io_to_output(module, jio)?;
        if &dv != o {
            return Err(format!(
                "CE step {}: outputs diverge (claimed {:?}, replay {:?})",
                k + 1,
                dv,
                o
            ));
        }
    }
    Ok(())
}

/// Step the label's monitor over the replayed samples; the FIRST confirmed
/// failure must be exactly at `at_step`.
fn property_confirms_at(
    module: &Module,
    label: &str,
    samples: &[Sample],
    at_step: usize,
) -> Result<(), String> {
    let formula = module
        .properties
        .iter()
        .find(|p| p.name == label)
        .or_else(|| module.assumptions.iter().find(|a| a.name == label))
        .ok_or_else(|| format!("unknown label '{label}'"))?
        .formula
        .clone();
    if contains_liveness(&formula) {
        return Err(format!("label '{label}' is liveness (not monitorable)"));
    }
    let mon = compile_formula(&formula)
        .map_err(|e| format!("label '{label}' not in the monitor subset: {e}"))?;
    let mut st = mon_init(&mon);
    for (idx, s) in samples.iter().enumerate() {
        if idx > at_step {
            break;
        }
        if mon_step(&mut st, &mon, idx as u64, s, module)? {
            return if idx == at_step {
                Ok(())
            } else {
                Err(format!(
                    "label '{label}' confirmed at sample {idx}, CE claims {at_step}"
                ))
            };
        }
    }
    Err(format!(
        "label '{label}' never confirmed through sample {at_step}"
    ))
}

/// The lasso (prefix + one cycle lap) must falsify the claimed liveness
/// obligation: configuration closure, product-automaton closure, and a
/// falsifying limit vector (the same acceptance condition `liv_analyze`
/// uses for end components).
fn verify_lasso(
    module: &Module,
    label: &str,
    samples: &[Sample],
    prefix_len: usize,
    cycle_len: usize,
) -> Result<(), String> {
    let (liv_props, _) = decompose_properties(module);
    let prop = liv_props
        .into_iter()
        .flatten()
        .find(|p| p.label == label)
        .ok_or_else(|| format!("'{label}' is not a liveness property of the module"))?;
    // Configuration-level closure.
    if samples[prefix_len].to_state != samples[prefix_len + cycle_len].to_state
        || samples[prefix_len].vars != samples[prefix_len + cycle_len].vars
    {
        return Err("lasso cycle is not closed at the configuration level".to_string());
    }
    let leaves = leaf_signals(&prop);
    let n = leaves.len();
    if n == 0 {
        return Err(format!("'{label}' has no liveness leaves"));
    }
    let radices: Vec<usize> = leaves
        .iter()
        .map(|l| if l.kind == 2 { 3 } else { 2 })
        .collect();
    let mut places = vec![1usize; n];
    for k in (0..n - 1).rev() {
        places[k] = places[k + 1] * radices[k + 1];
    }
    // Position i <-> samples[i] (position 0 = initial configuration).
    let bits_at = |i: usize| leaf_bits(&leaves, &samples[i], module);
    let mut q = prod_delta(0, &places, &radices, &leaves, bits_at(0)?);
    for i in 1..=prefix_len {
        q = prod_delta(q, &places, &radices, &leaves, bits_at(i)?);
    }
    // One full cycle lap from the post-prefix product state; on the infinite
    // path P.C^omega the same lap repeats, so a genuine product cycle must be
    // a fixed point of the lap map (all leaf transitions are one-way).
    let q_after_lap = {
        let mut q2 = q;
        for i in (prefix_len + 1)..=(prefix_len + cycle_len) {
            q2 = prod_delta(q2, &places, &radices, &leaves, bits_at(i)?);
        }
        q2
    };
    let q_after_second_lap = {
        let mut q3 = q_after_lap;
        for i in (prefix_len + 1)..=(prefix_len + cycle_len) {
            q3 = prod_delta(q3, &places, &radices, &leaves, bits_at(i)?);
        }
        q3
    };
    if q_after_second_lap != q_after_lap {
        return Err(format!(
            "lasso cycle is not closed in the obligation product automaton (lap state {} -> {})",
            q_after_lap, q_after_second_lap
        ));
    }
    let q = q_after_lap;
    let kinds: Vec<u8> = leaves.iter().map(|l| l.kind).collect();
    let t: Vec<bool> = (0..n)
        .map(|k| leaf_good(kinds[k], (q / places[k]) % radices[k]))
        .collect();
    if skel_eval(&prop.skeleton, &t) {
        return Err(format!(
            "lasso does not falsify '{label}': the obligation holds on the cycle"
        ));
    }
    Ok(())
}

/// The inverse of [`mon_key`] at an enqueued configuration: no monitor there
/// can be in the failed state (a property failure ends the run; a failed
/// assumption prunes the successor before enqueue), so the persistent
/// projection fully determines the monitor state.
fn mon_from_key(spec: &Mon, key: &[u64], pos: &mut usize) -> Result<MonSt, String> {
    match spec {
        Mon::Check { .. } => Ok(MonSt::Check { failed: false }),
        Mon::Delayed { .. } => {
            let v = key
                .get(*pos)
                .copied()
                .ok_or("monitor projection too short")?;
            *pos += 1;
            Ok(MonSt::Delayed {
                prev_ant: v != 0,
                failed: false,
            })
        }
        Mon::Obligation { bound, .. } => {
            if *bound >= 1 {
                let code = key
                    .get(*pos)
                    .copied()
                    .ok_or("monitor projection too short")?;
                *pos += 1;
                // Encoding (mon_key_node): 0 = not pending, else 2 + 2*remaining.
                let (pending, remaining) = if code == 0 {
                    (false, 0)
                } else {
                    (true, (code - 2) / 2)
                };
                Ok(MonSt::Obligation {
                    pending,
                    remaining,
                    failed: false,
                    trigger_step: -1,
                })
            } else {
                // Bound 0 is decided at its anchor sample; a surviving
                // configuration carries no persistent state for it.
                Ok(MonSt::Obligation {
                    pending: false,
                    remaining: 0,
                    failed: false,
                    trigger_step: -1,
                })
            }
        }
        Mon::And(children) => {
            let mut sts = Vec::new();
            for c in children {
                sts.push(mon_from_key(c, key, pos)?);
            }
            Ok(MonSt::And {
                children: sts,
                failed: false,
                trigger_step: -1,
            })
        }
        Mon::Or(children) => {
            let mut sts = Vec::new();
            for c in children {
                sts.push(mon_from_key(c, key, pos)?);
            }
            Ok(MonSt::Or {
                children: sts,
                failed: false,
                trigger_step: -1,
            })
        }
    }
}

/// The claimed terminal configuration must be well-formed and a deadlock in
/// the SEMANTICS 8.4 sense: NO input has a surviving successor - unexpected
/// inputs terminate their path, and an enabled successor survives only if no
/// ACTIVE assumption monitor confirms a violation on it (pruning state comes
/// from `mons_proj`, the run's terminal monitor projection).
fn verify_deadlock(
    module: &Module,
    cfg: &RefConfig,
    depth: usize,
    state: &str,
    vars: &[(String, crate::json_rw::Json)],
    mons_proj: Option<&Vec<Vec<u64>>>,
) -> Result<(), String> {
    let states = state_variants(module).to_vec();
    let si = states
        .iter()
        .position(|s| s == ce_state_name(state))
        .ok_or_else(|| format!("deadlock CE: unknown state '{state}'"))?;
    if vars.len() != module.vars.len() {
        return Err(format!(
            "deadlock CE: {} var(s) in CE, {} declared",
            vars.len(),
            module.vars.len()
        ));
    }
    let mut vals: Vec<Val> = Vec::new();
    for (pos, (name, j)) in vars.iter().enumerate() {
        if &module.vars[pos].name != name {
            return Err(format!(
                "deadlock CE: unexpected var '{name}' at position {pos}"
            ));
        }
        let v = json_val(j, &format!("deadlock CE var {name}"))?;
        // Domain check against the declared type.
        match (&module.vars[pos].ty, &v) {
            (Type::Bool, Val::Bool(_)) => {}
            (Type::Bit { width }, Val::Bit(x)) => {
                if *width < 64 && *x >= 1u64 << width {
                    return Err(format!(
                        "deadlock CE: var {name} = {x} outside bit<{width}> domain"
                    ));
                }
            }
            (Type::Enum { name: en }, Val::Enum(vn)) => {
                let ok = module
                    .enums
                    .iter()
                    .find(|e| e.name == *en)
                    .map(|e| e.variants.iter().any(|x| x == vn))
                    .unwrap_or(false);
                if !ok {
                    return Err(format!(
                        "deadlock CE: var {name} = {vn} not a variant of {en}"
                    ));
                }
            }
            (ty, v) => {
                return Err(format!(
                    "deadlock CE: var {name} value {v:?} does not fit type {ty:?}"
                ))
            }
        }
        vals.push(v);
    }
    // Monitor setup identical to run() (same aggregate order: properties,
    // then selected assumptions).
    let known: Vec<&str> = module.assumptions.iter().map(|a| a.name.as_str()).collect();
    let selected = parse_assume_selection(&cfg.assume_arg, &known);
    let mut prop_mons: Vec<(String, Mon)> = Vec::new();
    for p in &module.properties {
        if contains_liveness(&p.formula) {
            continue;
        }
        if let Ok(mon) = compile_formula(&p.formula) {
            prop_mons.push((p.name.clone(), mon));
        }
    }
    let mut assume_mons: Vec<(String, Mon)> = Vec::new();
    for a in &module.assumptions {
        if !selected.iter().any(|x| x == &a.name) {
            continue;
        }
        if let Ok(mon) = compile_formula(&a.formula) {
            assume_mons.push((a.name.clone(), mon));
        }
    }
    let all_mons: Vec<(String, Mon)> = prop_mons
        .iter()
        .cloned()
        .chain(assume_mons.iter().cloned())
        .collect();
    let n_props = prop_mons.len();
    let proj =
        mons_proj.ok_or("deadlock CE: the run did not report a terminal monitor projection")?;
    if proj.len() != all_mons.len() {
        return Err(format!(
            "deadlock CE: {} monitor projections, {} monitors",
            proj.len(),
            all_mons.len()
        ));
    }
    let mut mons = Vec::new();
    for (key, (_, sp)) in proj.iter().zip(all_mons.iter()) {
        let mut pos = 0usize;
        mons.push(mon_from_key(sp, key, &mut pos)?);
    }
    // SEMANTICS 8.4: every input must be unexpected or pruned.
    for input in all_inputs(module) {
        let Some(eff) = match_effect(module, si, &vals, &input) else {
            continue; // unexpected: the path terminates (not a successor)
        };
        let sample = Sample {
            input: Some(input.clone()),
            outputs: eff.outputs,
            to_state: eff.next_state,
            vars: eff.next_vars,
        };
        // Assumptions first, at the successor's sample index (depth + 1) -
        // exactly as run() does; a confirmed violation prunes the successor.
        let mut mons_copy = mons.clone();
        let pruned = (0..mons_copy.len()).skip(n_props).any(|mi| {
            mon_step(
                &mut mons_copy[mi],
                &all_mons[mi].1,
                (depth + 1) as u64,
                &sample,
                module,
            )
            .unwrap_or(false)
        });
        if !pruned {
            return Err(format!(
                "configuration is not a deadlock: representative input variant {} has a surviving successor",
                input.variant
            ));
        }
    }
    Ok(())
}

/// Validate one counterexample parsed from a generated engine's JSON report
/// against ref_mc's independent evaluators. `reason`/`label`/`step` are the
/// summary-line fields of the same run; `ce` its counterexample document;
/// `deadlock_mons` the run's terminal monitor projection (`RefResult`
/// field, needed to re-derive the SEMANTICS 8.4 pruning judgment for
/// deadlock CEs). `Ok(())` = the CE is a valid execution that genuinely
/// exhibits the claimed violation at the claimed step (deadlock CEs:
/// well-formed deadlocked configuration).
pub fn verify_counterexample(
    module: &Module,
    cfg: &RefConfig,
    reason: FailReason,
    label: &str,
    step: usize,
    ce: &JsonCounterexample,
    deadlock_mons: Option<&Vec<Vec<u64>>>,
) -> Result<(), String> {
    match (reason, ce) {
        (
            FailReason::PropertyViolation,
            JsonCounterexample::Trace {
                initial_state,
                initial_vars,
                steps,
            },
        ) => {
            check_initial(module, initial_state, initial_vars)?;
            let inputs: Vec<InputVal> = steps
                .iter()
                .map(|s| io_to_input(module, &s.input))
                .collect::<Result<_, _>>()?;
            let samples = step_trace(module, &inputs)?;
            let q0 = initial_config(module)?.0;
            let mut prev = q0;
            for (k, s) in steps.iter().enumerate() {
                check_step_config(module, prev, k, s, &samples[k + 1])?;
                prev = samples[k + 1].to_state;
            }
            property_confirms_at(module, label, &samples, step)
        }
        (
            FailReason::LivenessViolation,
            JsonCounterexample::Lasso {
                initial_state,
                initial_vars,
                prefix,
                cycle,
                ..
            },
        ) => {
            check_initial(module, initial_state, initial_vars)?;
            if step != prefix.len() {
                return Err(format!(
                    "lasso CE: summary step {} != prefix length {}",
                    step,
                    prefix.len()
                ));
            }
            let inputs: Vec<InputVal> = prefix
                .iter()
                .chain(cycle.iter())
                .map(|s| io_to_input(module, &s.input))
                .collect::<Result<_, _>>()?;
            let samples = step_trace(module, &inputs)?;
            let q0 = initial_config(module)?.0;
            let mut prev = q0;
            for (k, s) in prefix.iter().chain(cycle.iter()).enumerate() {
                check_step_config(module, prev, k, s, &samples[k + 1])?;
                prev = samples[k + 1].to_state;
            }
            verify_lasso(module, label, &samples, prefix.len(), cycle.len())
        }
        (FailReason::Deadlock, JsonCounterexample::DeadlockConfig { depth, state, vars }) => {
            verify_deadlock(module, cfg, *depth as usize, state, vars, deadlock_mons)
        }
        _ => Err(format!(
            "reason {:?} does not match the counterexample kind",
            reason
        )),
    }
}

// ---------------------------------------------------------------------------
// Tests: hand-computed micro-specs + byte-pinned reports
// ---------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;

    /// Parse a single-module spec into its module.
    fn module(src: &str) -> Module {
        parse_source(src)
            .expect("parse")
            .modules
            .pop()
            .expect("module")
    }

    fn cfg() -> RefConfig {
        RefConfig::default()
    }

    // 1. Passing spec: A <-> B on Go.
    #[test]
    fn pass_spec_is_pass_complete() {
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::PassComplete);
        assert_eq!(r.states, 2);
        assert_eq!(r.transitions, 2);
        assert_eq!(r.max_depth, 1);
        assert_eq!(r.unexpected_terminations, 0);
        assert!(r.fail.is_none());
        assert!(r.ce.is_none());
    }

    // 2. Safety FAIL with known label + step.
    #[test]
    fn safety_fail_reports_label_and_step() {
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad event ToGood \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on ToGood { next Good; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::PropertyViolation, "no_bad".into(), 1))
        );
        // Canonical order: Stay first (self-loop), then ToBad -> violation at step 1.
        let CeData::Trace {
            steps,
            trigger_step,
        } = r.ce.as_ref().unwrap()
        else {
            panic!("expected trace CE");
        };
        assert_eq!(*trigger_step, None);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].from_state, "Good");
        assert_eq!(steps[0].to_state, "Bad");
        assert_eq!(steps[0].input.variant, "ToBad");
    }

    // 3. Liveness FAIL with a lasso (eventually over an unreachable state).
    #[test]
    fn liveness_fail_reports_lasso() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; } transition B on Go { next B; } \
             prop reaches_c : eventually (state == C) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::LivenessViolation, "reaches_c".into(), 1))
        );
        let CeData::Lasso {
            cycle_start,
            steps,
            note,
            formula_text,
        } = r.ce.as_ref().unwrap()
        else {
            panic!("expected lasso CE");
        };
        assert_eq!(*cycle_start, 1);
        assert_eq!(steps.len(), 2); // prefix A->B, cycle B->B
        assert_eq!(formula_text, "eventually (state == C)");
        assert_eq!(note, "eventually body never holds on the path");
    }

    // 4. Deadlock: terminal state with no outgoing transitions.
    #[test]
    fn deadlock_reports_terminal_configuration() {
        let m = module(
            "module m { enum State { A, T } initial A event Go \
             transition A on Go { next T; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(r.fail, Some((FailReason::Deadlock, "-".into(), 1)));
        let CeData::Deadlock { depth, steps } = r.ce.as_ref().unwrap() else {
            panic!("expected deadlock CE");
        };
        assert_eq!(*depth, 1);
        assert_eq!(steps.len(), 1);
        assert_eq!(steps[0].to_state, "T");
    }

    // 5. State-limit truncation: UnknownStateLimit, never a completeness claim.
    #[test]
    fn state_limit_is_unknown() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; } transition B on Go { next C; } \
             transition C on Go { next C; } }",
        );
        let mut c = cfg();
        c.max_states = 1;
        let r = run(&m, &c).expect("run");
        assert_eq!(r.status, ExactStatus::UnknownStateLimit);
        assert_eq!(r.states, 1);
        assert_eq!(r.transitions, 1); // the counted success that hit the cap
        assert!(r.fail.is_none());

        let mut c = cfg();
        c.max_states = 2;
        let r = run(&m, &c).expect("run");
        assert_eq!(r.status, ExactStatus::UnknownStateLimit);
        assert_eq!(r.states, 2);
    }

    // Sample-0 property violation.
    #[test]
    fn sample_zero_violation() {
        let m = module(
            "module m { enum State { A, B } initial B event Go \
             transition B on Go { next A; } \
             prop starts_a : always (state == A) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::PropertyViolation, "starts_a".into(), 0))
        );
        assert_eq!(r.states, 1);
        let CeData::Trace { steps, .. } = r.ce.as_ref().unwrap() else {
            panic!("expected trace CE");
        };
        assert!(steps.is_empty());
    }

    // Assumption pruning: the violating input is not explored.
    #[test]
    fn assumption_prunes_successor() {
        // A -Go-> B, B -Go-> B; the assumption forbids two consecutive Gos,
        // so B's self-loop successor is pruned on every visit.
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next B; } \
             assume single_go : always (input_is(Go) -> next (!input_is(Go))) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        // B's Go successor is pruned at every visit; B has no other input with
        // a successor (Go is the only event) -> deadlock at depth 1.
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(r.fail, Some((FailReason::Deadlock, "-".into(), 1)));

        // With assumptions off, B's self-loop is explored: pass-complete.
        let mut c = cfg();
        c.assume_arg = "none".into();
        let r = run(&m, &c).expect("run");
        assert_eq!(r.status, ExactStatus::PassComplete);
    }

    // Bounded-eventually obligation: deadline semantics (N=2, one-shot at 0).
    #[test]
    fn bounded_eventually_deadline() {
        // Top-level bounded_eventually[2]: the body must hold within samples
        // 0..=1. Sent is first emitted at step 2, so the countdown expires:
        // sample 0 triggers (remaining 2 -> 1), sample 1 decrements to 0 and
        // confirms the violation - before the emit can clear it.
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } \
             transition B on Go { next B; emit Sent; } \
             output Sent \
             prop sent_soon : bounded_eventually[2] (output_has(Sent)) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::Fail);
        assert_eq!(
            r.fail,
            Some((FailReason::PropertyViolation, "sent_soon".into(), 1))
        );
        // The violating successor is never enqueued.
        assert_eq!(r.states, 1);
        assert_eq!(r.transitions, 1);
        let CeData::Trace {
            trigger_step,
            steps,
        } = r.ce.as_ref().unwrap()
        else {
            panic!("expected trace CE");
        };
        assert_eq!(*trigger_step, Some(0)); // the one-shot obligation started at sample 0
        assert_eq!(steps.len(), 1);
    }

    // Unexpected inputs terminate paths but do not fail the model.
    #[test]
    fn unexpected_inputs_are_not_failures() {
        let m = module(
            "module m { enum State { A, B } initial A event Go event Other \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(r.status, ExactStatus::PassComplete);
        assert_eq!(r.unexpected_terminations, 2); // Other at A and at B
        assert_eq!(r.first_unexpected, Some((0, 1)));
    }

    // --- Byte-pinned reports (text + JSON) for the five outcomes above. ---

    #[test]
    fn pinned_pass_text_and_json() {
        let m = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "PASS (complete)\n\
             model-check summary: status=pass-complete states=2 transitions=2 max_depth=1\n\
             \x20 unexpected terminations: 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 66 bytes\n"
        );
        let doc = to_json_report(&r, &m).to_json_string();
        assert_eq!(
            doc,
            "{\"status\":\"pass-complete\",\"metrics\":{\"states\":2,\"transitions\":2,\"max_depth\":1},\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":0,\"memory_estimate_bytes\":66}}"
        );
    }

    #[test]
    fn pinned_safety_fail_text_and_json() {
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad event ToGood \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on ToGood { next Good; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "FAIL\n\
             model-check summary: status=fail states=1 transitions=2 max_depth=0 reason=property-violation label=no_bad step=1\n\
             \x20 unexpected terminations: 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 33 bytes\n\
             property violation: no_bad\n\
             \x20 violation step: 1\n\
             \x20 vars at violation step: Vars\n\
             counterexample trace:\n\
             \x20 initial state=State::Good vars=Vars\n\
             \x20 step 0 input=Input::ToBad outputs=[] from=State::Good to=State::Bad vars=Vars\n"
        );
        let doc = to_json_report(&r, &m).to_json_string();
        assert_eq!(
            doc,
            "{\"status\":\"fail\",\"metrics\":{\"states\":1,\"transitions\":2,\"max_depth\":0},\"reason\":\"property-violation\",\"label\":\"no_bad\",\"step\":1,\"statistics\":{\"unexpected_terminations\":0,\"monitor_size_bits\":0,\"memory_estimate_bytes\":33},\"counterexample\":{\"kind\":\"trace\",\"initial_state\":\"State::Good\",\"initial_vars\":{},\"steps\":[{\"input\":{\"variant\":\"ToBad\",\"fields\":{}},\"outputs\":[],\"from_state\":\"State::Good\",\"to_state\":\"State::Bad\",\"vars\":{}}]}}"
        );
    }

    #[test]
    fn pinned_deadlock_text() {
        let m = module(
            "module m { enum State { A, T } initial A event Go \
             transition A on Go { next T; } }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "FAIL\n\
             model-check summary: status=fail states=2 transitions=1 max_depth=1 reason=deadlock label=- step=1\n\
             \x20 unexpected terminations: 1\n\
             \x20 first unexpected: config 1 input index 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 66 bytes\n\
             deadlock: configuration at depth 1 has no enabled transition\n\
             counterexample trace:\n\
             \x20 initial state=State::A vars=Vars\n\
             \x20 step 0 input=Input::Go outputs=[] from=State::A to=State::T vars=Vars\n"
        );
    }

    #[test]
    fn pinned_liveness_text() {
        let m = module(
            "module m { enum State { A, B, C } initial A event Go \
             transition A on Go { next B; } transition B on Go { next B; } \
             prop reaches_c : eventually (state == C) ; }",
        );
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            render_text(&r, &m),
            "FAIL\n\
             model-check summary: status=fail states=2 transitions=2 max_depth=1 reason=liveness-violation label=reaches_c step=1\n\
             \x20 unexpected terminations: 0\n\
             \x20 monitor size: 0 bits\n\
             \x20 memory estimate: 66 bytes\n\
             liveness violation: reaches_c\n\
             \x20 formula: eventually (state == C)\n\
             \x20 violation step: 1\n\
             \x20 cycle length: 1\n\
             \x20 note: eventually body never holds on the path\n\
             counterexample trace:\n\
             \x20 initial state=State::A vars=Vars\n\
             \x20 step 0 input=Input::Go outputs=[] from=State::A to=State::B vars=Vars\n\
             \x20 step 1 input=Input::Go outputs=[] from=State::B to=State::B vars=Vars\n"
        );
    }

    #[test]
    fn minimal_labels_unique_and_tied() {
        // Unique minimal violation.
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad event ToGood \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on ToGood { next Good; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let labels = minimal_violation_labels(&m, &cfg()).expect("labels");
        assert_eq!(labels, vec!["no_bad".to_string()]);

        // No violations at all.
        let m2 = module(
            "module m { enum State { A, B } initial A event Go \
             transition A on Go { next B; } transition B on Go { next A; } }",
        );
        assert!(minimal_violation_labels(&m2, &cfg())
            .expect("labels")
            .is_empty());
    }

    // Timeout granularity: a SINGLE BFS level may exceed the wall-clock budget
    // (19_guard_complex: one level runs for minutes). The deadline check must
    // therefore fire mid-level, not only between levels - otherwise
    // --timeout-sec is silently ignored on such specs.
    #[test]
    fn timeout_fires_mid_level_on_giant_level() {
        // 1 state x bit<12> var; the event payload is referenced (assignment),
        // so it contributes its full domain: 4096 representative inputs. Level
        // 1 = 4095 configs x 4096 inputs (~1.7e7 transition steps) - one level
        // that takes several seconds on any host.
        let m = module(
            "module m { enum State { S } initial S \
             event E(P(p: bit<12>)) var x: bit<12> init 0 \
             transition S on E(P(p)) { next S; set x = p; } }",
        );
        let c = RefConfig {
            timeout_sec: 3,
            ..RefConfig::default()
        };
        let t0 = Instant::now();
        let r = run(&m, &c).expect("run");
        let elapsed = t0.elapsed().as_secs();
        assert_eq!(r.status, ExactStatus::UnknownStateLimit);
        assert!(r.timeout_secs.is_some(), "must record the timeout");
        assert!(
            elapsed <= 5,
            "timeout should fire shortly after the 3s budget (took {elapsed}s)"
        );
    }

    // Regression (Phase 6 task 4, correction 4a): a violation confirmed at
    // SAMPLE 0 must still yield the full minimal-label set - every
    // monitorable property failing at sample 0, not an empty set.
    #[test]
    fn minimal_labels_sample_zero_violation() {
        let m = module(
            "module m { enum State { S } initial S \
             event Stay \
             transition S on Stay { next S; } \
             prop p1 : true && false ; \
             prop p2 : always (false) ; }",
        );
        // run() reports the FIRST failing property in declaration order.
        let r = run(&m, &cfg()).expect("run");
        assert_eq!(
            r.fail.as_ref(),
            Some(&(FailReason::PropertyViolation, "p1".into(), 0))
        );
        // The minimal set carries BOTH labels (sorted).
        let labels = minimal_violation_labels(&m, &cfg()).expect("labels");
        assert_eq!(labels, vec!["p1".to_string(), "p2".to_string()]);
    }

    // --- Phase 6 task 4.1: counterexample replay + verification. ---

    /// Build one JSON step of a CE from bare names (the wire form prefixes
    /// states with `State::`).
    fn ce_step(input: &str, from: &str, to: &str, outputs: &[&str]) -> JsonStep {
        JsonStep {
            input: JsonIo {
                variant: input.into(),
                fields: vec![],
            },
            outputs: outputs
                .iter()
                .map(|o| JsonIo {
                    variant: (*o).into(),
                    fields: vec![],
                })
                .collect(),
            from_state: format!("State::{from}"),
            to_state: format!("State::{to}"),
            vars: vec![],
        }
    }

    #[test]
    fn verify_trace_ce_valid_and_tampered() {
        let m = module(
            "module m { enum State { Good, Bad } initial Good \
             event Stay event ToBad \
             transition Good on Stay { next Good; } \
             transition Good on ToBad { next Bad; } \
             transition Bad on Stay { next Bad; } \
             prop no_bad : always (state != Bad) ; }",
        );
        let ce = JsonCounterexample::Trace {
            initial_state: "State::Good".into(),
            initial_vars: vec![],
            steps: vec![ce_step("ToBad", "Good", "Bad", &[])],
        };
        // The genuine CE verifies.
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::PropertyViolation,
            "no_bad",
            1,
            &ce,
            None,
        )
        .expect("genuine trace CE must verify");
        // Tampered: the violation is confirmed at sample 1, not 2.
        let e = verify_counterexample(
            &m,
            &cfg(),
            FailReason::PropertyViolation,
            "no_bad",
            2,
            &ce,
            None,
        )
        .unwrap_err();
        assert!(e.contains("confirmed at sample 1"), "{e}");
        // Tampered: wrong target state.
        let mut bad = ce.clone();
        if let JsonCounterexample::Trace { steps, .. } = &mut bad {
            steps[0].to_state = "State::Good".into();
        }
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::PropertyViolation,
            "no_bad",
            1,
            &bad,
            None,
        )
        .expect_err("tampered to_state must be rejected");
        // Tampered: input that never fires from Good.
        let mut bad2 = ce.clone();
        if let JsonCounterexample::Trace { steps, .. } = &mut bad2 {
            steps[0].input.variant = "Stay".into();
        }
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::PropertyViolation,
            "no_bad",
            1,
            &bad2,
            None,
        )
        .expect_err("tampered input must be rejected (Bad not reached)");
    }

    #[test]
    fn verify_lasso_ce_valid_and_non_falsifying() {
        // C is unreachable: `eventually (state == C)` is genuinely violated
        // on the lasso A -Go-> B -Go-> B (repeating).
        let m = module(
            "module m { enum State { A, B, C } initial A \
             event Go \
             transition A on Go { next B; } \
             transition B on Go { next B; } \
             prop stuck : eventually (state == C) ; }",
        );
        let ce = JsonCounterexample::Lasso {
            initial_state: "State::A".into(),
            initial_vars: vec![],
            prefix: vec![ce_step("Go", "A", "B", &[])],
            cycle: vec![ce_step("Go", "B", "B", &[])],
            cycle_length: 1,
        };
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::LivenessViolation,
            "stuck",
            1,
            &ce,
            None,
        )
        .expect("genuine lasso must verify");
        // The SAME path satisfies `eventually (state == B)` - a CE claiming
        // that label on this lasso must be rejected.
        let m2 = module(
            "module m { enum State { A, B, C } initial A \
             event Go \
             transition A on Go { next B; } \
             transition B on Go { next B; } \
             prop ok : eventually (state == B) ; }",
        );
        verify_counterexample(
            &m2,
            &cfg(),
            FailReason::LivenessViolation,
            "ok",
            1,
            &ce,
            None,
        )
        .expect_err("non-falsifying lasso must be rejected");
    }

    #[test]
    fn verify_deadlock_ce_valid_and_non_deadlocked() {
        let m = module(
            "module m { enum State { S, T } initial S \
             event Go \
             transition S on Go { next T; } }",
        );
        // The run's terminal projection is the pruning-state input of the
        // SEMANTICS 8.4 re-derivation.
        let res = run(&m, &cfg()).expect("run");
        assert!(matches!(
            res.fail.as_ref(),
            Some((FailReason::Deadlock, _, _))
        ));
        let ce = JsonCounterexample::DeadlockConfig {
            depth: 1,
            state: "State::T".into(),
            vars: vec![],
        };
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::Deadlock,
            "-",
            1,
            &ce,
            res.deadlock_mons.as_ref(),
        )
        .expect("genuine deadlock CE must verify");
        let ce2 = JsonCounterexample::DeadlockConfig {
            depth: 0,
            state: "State::S".into(),
            vars: vec![],
        };
        let e = verify_counterexample(
            &m,
            &cfg(),
            FailReason::Deadlock,
            "-",
            0,
            &ce2,
            res.deadlock_mons.as_ref(),
        )
        .unwrap_err();
        assert!(e.contains("not a deadlock"), "{e}");
    }

    // Regression (Phase 6 task 4): SEMANTICS 8.4 deadlock includes assumption
    // PRUNING - a configuration whose enabled successors are all pruned by an
    // active assumption monitor is deadlocked even though transitions exist.
    #[test]
    fn verify_deadlock_ce_assumption_pruned() {
        // S1 has an enabled transition (Go -> S2), but the Tick assumption's
        // countdown expires on that successor, and there is no Tick transition
        // at S1 - so every successor is pruned or unexpected: deadlocked.
        let m = module(
            "module m { enum State { S0, S1, S2 } initial S0 \
             event Go event Tick \
             transition S0 on Go { next S1; } \
             transition S0 on Tick { next S0; } \
             transition S1 on Go { next S2; } \
             transition S2 on Go { next S2; } \
             assume tick_soon : always (bounded_eventually[3] (input_is(Tick))) ; }",
        );
        let res = run(&m, &cfg()).expect("run");
        assert_eq!(
            res.fail.as_ref(),
            Some(&(FailReason::Deadlock, "-".into(), 1))
        );
        let ce = JsonCounterexample::DeadlockConfig {
            depth: 1,
            state: "State::S1".into(),
            vars: vec![],
        };
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::Deadlock,
            "-",
            1,
            &ce,
            res.deadlock_mons.as_ref(),
        )
        .expect("assumption-pruned deadlock must verify");
        // A configuration with a genuinely surviving successor must be
        // rejected: S0's Tick successor keeps the countdown alive.
        let ce2 = JsonCounterexample::DeadlockConfig {
            depth: 0,
            state: "State::S0".into(),
            vars: vec![],
        };
        verify_counterexample(
            &m,
            &cfg(),
            FailReason::Deadlock,
            "-",
            0,
            &ce2,
            res.deadlock_mons.as_ref(),
        )
        .expect_err("S0 has a surviving successor (Tick)");
    }

    // Independence audit: the module must not import the shared engines'
    // search/evaluation code (task file "Other information").
    #[test]
    fn independence_import_audit() {
        let src = std::fs::read_to_string(concat!(env!("CARGO_MANIFEST_DIR"), "/src/ref_mc.rs"))
            .expect("read self");
        for forbidden in ["mc_plan", "monitor", "codegen_fsm", "printer", "backends"] {
            assert!(
                !src.lines().any(|l| {
                    let t = l.trim();
                    (t.starts_with("use crate::") || t.contains("crate::"))
                        && t.contains(forbidden)
                        && !t.starts_with("//!")
                }),
                "ref_mc.rs must not import '{forbidden}'"
            );
        }
    }
}
