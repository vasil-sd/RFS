//! Brute-force reference model checker - COMMON CORE + entry points
//! (Phase 6 task 3, D-P6-5 / D-P6-10; split per mode in Phase 16 task 1,
//! D-P16-1).
//!
//! Independent ground-truth engine for differential testing (Phase 6 tasks
//! 4.1/5.2) and user-facing rapid prototyping (`rfs mc-ref`). It implements
//! `docs/SEMANTICS.md` sections 2-9 directly, with DELIBERATELY DIFFERENT
//! data structures and algorithms from the generated engines:
//!
//! - its own expression evaluator and its own online monitor stepper compiled
//!   from the IR formula (same documented semantics as `src/monitor.rs`,
//!   different code);
//! - representative inputs computed independently from the IR (the rule of
//!   SEMANTICS.md section 9.3, reimplemented - not imported from
//!   `src/mc/plan.rs`);
//! - liveness decomposition into leaves + skeleton, consumed by the exact
//!   engine's SCC analysis in `ref_mc_exact.rs` (iterative Tarjan +
//!   self-loop check), different from the generated engine's BFS + stay-edge
//!   subgraph + Kosaraju construction.
//!
//! This file holds the COMMON CORE shared by every mode: values/typing, the
//! evaluator, signals/inputs/outputs, the monitor compiler + stepper,
//! representative inputs, transition effects, liveness decomposition,
//! formula/expression printing, the config/result types, report rendering
//! (text + JSON), and counterexample replay/verification. The exact-mode
//! engine (structured-key BFS + liveness SCC + minimal-violation labels)
//! lives in `ref_mc_exact.rs`; `run` here is the delegating entry point.
//!
//! Independence is about ALGORITHMS, not report formatting: per D-P6-10 this
//! module reuses `src/codegen/mc_concepts.rs` DATA TYPES (status taxonomy, JSON report
//! model) and its pure formatting/parsing helpers (`exact_summary_line`,
//! `parse_assume_selection`) so that `rfs mc-ref` output is directly
//! comparable - byte-for-byte where deterministic - with the generated MC
//! binaries' output. It mirrors the generated engines' DEFAULT exact mode:
//! deterministic first-match transition firing, seedless exhaustive search.
//!
//! Import list (independence proof, per task file "Other information"):
//! `std::{collections,time}`, `crate::lang::ir::*`, and
//! `crate::codegen::mc_concepts::{exact_summary_line, parse_assume_selection,
//! ExactStatus, FailReason, JsonCounterexample, JsonIo, JsonStep,
//! McJsonReport, EXACT_DEFAULT_MAX_STATES, EXACT_DEFAULT_TIMEOUT_SEC}`
//! (data types + shared report helpers only), plus the liveness helpers the
//! counterexample verification re-derives from `ref_mc_exact`
//! (`state_name`, `leaf_signals`, `leaf_bits`, `prod_delta`, `leaf_good`,
//! `skel_eval`). The unit tests additionally use `crate::json::json_rw::Json`.
//! NOT imported: `mc_plan`, `monitor`, `codegen::fsm`, `printer`, backend
//! modules - enforced by the `independence_import_audit` test over all
//! `ref_mc_*.rs` files.
//! Docs: dev:mc-engine-overview, dev:mc-engine-verify, dev:algorithms-mc-engine, dev:references-model-checking, llm:mc-engine

use std::collections::{HashMap, HashSet};

use crate::codegen::mc_concepts::{
    exact_summary_line, parse_assume_selection, ExactStatus, FailReason, JsonCounterexample,
    JsonIo, JsonStep, McJsonReport, ViolationSpec, EXACT_DEFAULT_MAX_STATES,
    EXACT_DEFAULT_TIMEOUT_SEC, FUZZ_DEFAULT_BOUND, FUZZ_DEFAULT_RUNS, FUZZ_DEFAULT_SEED,
};
use crate::lang::fold::{fold_const, is_const_expr};
use crate::lang::ir::*;

// The counterexample verification re-derives the exact engine's liveness
// acceptance condition (D-P16-1: the helpers live with the engine).
use super::ref_mc_exact::{
    self, leaf_bits, leaf_good, leaf_signals, match_effects_all, prod_delta, skel_eval, state_name,
};
// The fuzz-mode engine (Phase 16 task 3) - same D-P16-1 delegation pattern.
use super::ref_mc_fuzz;
// The SMT-bounded executor (Phase 16 task 4) - feature-gated (in-process z3
// execution requires the optional `smt` feature; codegen of smt artifacts is
// NOT gated).
#[cfg(feature = "smt")]
use super::ref_mc_smt;

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
pub(crate) enum RType {
    Bool,
    Bit(u32),
    Enum(String),
    /// Full-precision integer constant (task 3L rule 1 - no width of its own;
    /// folded at i128 precision, never silently truncated).
    IntLit,
}

impl RType {
    /// Bit width for arithmetic/bitwise results. `IntLit` has NO width
    /// (full precision - task 3L rule 1): a constant subexpression is folded
    /// before any masking, so the eval arms that call this never see an
    /// `IntLit` type on a validated spec.
    fn width(&self) -> Option<u32> {
        match self {
            RType::Bit(w) => Some(*w),
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

pub(crate) fn state_variants(module: &Module) -> &[String] {
    module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.as_slice())
        .unwrap_or_default()
}

/// Type of one expression (own implementation of the documented typing rules;
/// validated specs always type-check, so `Err` here marks an internal
/// invariant failure). `pub(crate)` so the task-3L equivalence test can compare
/// it word-for-word against the validator's `type_expr`.
pub(crate) fn expr_type(
    e: &Expr,
    module: &Module,
    binds: &HashMap<String, Type>,
) -> Result<RType, String> {
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
        // Explicit conversion cast (task 3L): types as Bit(width); the operand
        // must be a bitvector - a constant expression is legal (the cast IS
        // the explicit conversion), bool/enum are rejected. WORD-FOR-WORD the
        // validator's Cast arm.
        Expr::Cast { width, expr } => {
            let ty = expr_type(expr, module, binds)?;
            match ty {
                RType::Bit(_) | RType::IntLit => Ok(RType::Bit(*width)),
                _ => Err("cast operand must be a bitvector".into()),
            }
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
                // A bare constant under ~ / - stays a full-precision constant
                // (no bit<8> default - task 3L rule 1).
                UnaryOp::BitNot | UnaryOp::Neg => match ty {
                    RType::Bit(w) => Ok(RType::Bit(w)),
                    RType::IntLit => Ok(RType::IntLit),
                    _ => Err("bit_not/neg requires a bitvector operand".into()),
                },
            }
        }
        Expr::Binary { op, left, right } => {
            let l = expr_type(left, module, binds)?;
            let r = expr_type(right, module, binds)?;
            binary_type(*op, left, right, &l, &r)
        }
    }
}

/// Comparison typing - WORD-FOR-WORD the validator's `compare_types` (task 3L):
/// equal types, or a constant meeting a bit<N> operand via rule 4. Warnings
/// are discarded here (the validator is the warning channel); errors propagate.
fn compatible(left: &Expr, right: &Expr, l: &RType, r: &RType) -> Result<(), String> {
    match (l, r) {
        (a, b) if a == b => Ok(()),
        (RType::IntLit, RType::Bit(b)) => {
            crate::lang::fold::const_fit(
                crate::lang::fold::fold_const(left)?,
                *b,
                &mut Vec::new(),
            )?;
            Ok(())
        }
        (RType::Bit(b), RType::IntLit) => {
            crate::lang::fold::const_fit(
                crate::lang::fold::fold_const(right)?,
                *b,
                &mut Vec::new(),
            )?;
            Ok(())
        }
        _ => Err(format!("type mismatch: {l:?} vs {r:?}")),
    }
}

/// Arithmetic / bitwise operand typing - WORD-FOR-WORD the validator's
/// `arith_bit_type` (task 3L): equal widths; a constant meeting a bit<N>
/// operand goes through rule 4; two constants form a full-precision constant.
fn bit_op_type(left: &Expr, right: &Expr, l: &RType, r: &RType) -> Result<RType, String> {
    match (l, r) {
        (RType::Bit(a), RType::Bit(b)) => {
            if a != b {
                return Err(format!("bitwidth mismatch: bit<{a}> vs bit<{b}>"));
            }
            Ok(RType::Bit(*a))
        }
        (RType::IntLit, RType::Bit(b)) => {
            crate::lang::fold::const_fit(
                crate::lang::fold::fold_const(left)?,
                *b,
                &mut Vec::new(),
            )?;
            Ok(RType::Bit(*b))
        }
        (RType::Bit(b), RType::IntLit) => {
            crate::lang::fold::const_fit(
                crate::lang::fold::fold_const(right)?,
                *b,
                &mut Vec::new(),
            )?;
            Ok(RType::Bit(*b))
        }
        (RType::IntLit, RType::IntLit) => Ok(RType::IntLit),
        _ => Err("bitwise/arithmetic operator requires bitvector operands".into()),
    }
}

fn binary_type(
    op: BinOp,
    left: &Expr,
    right: &Expr,
    l: &RType,
    r: &RType,
) -> Result<RType, String> {
    match op {
        BinOp::LogicAnd | BinOp::LogicOr => {
            if !matches!(l, RType::Bool) || !matches!(r, RType::Bool) {
                return Err("logical operator requires boolean operands".into());
            }
            Ok(RType::Bool)
        }
        BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
            compatible(left, right, l, r)?;
            Ok(RType::Bool)
        }
        BinOp::Plus | BinOp::Minus | BinOp::Star => bit_op_type(left, right, l, r),
        BinOp::ShiftLeft | BinOp::ShiftRight => {
            // Task 3L: result = shifted value's width; a constant value shifted
            // by a VARIABLE amount takes the shift amount's width (refinement);
            // two constants form a full-precision constant. The constant-value
            // case goes through rule 4 exactly like the validator's shift_type.
            if matches!(r, RType::Bool | RType::Enum(_)) {
                return Err("shift right operand must be a bitvector or integer literal".into());
            }
            match (l, r) {
                (RType::Bit(a), _) => Ok(RType::Bit(*a)),
                (RType::IntLit, RType::Bit(b)) => {
                    crate::lang::fold::const_fit(
                        crate::lang::fold::fold_const(left)?,
                        *b,
                        &mut Vec::new(),
                    )?;
                    Ok(RType::Bit(*b))
                }
                (RType::IntLit, RType::IntLit) => Ok(RType::IntLit),
                _ => Err("shift left operand must be a bitvector".into()),
            }
        }
        BinOp::BitAnd | BinOp::BitOr => {
            if matches!(l, RType::Bool) && matches!(r, RType::Bool) {
                return Ok(RType::Bool);
            }
            bit_op_type(left, right, l, r)
        }
        BinOp::BitXor => bit_op_type(left, right, l, r),
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

pub(crate) fn mask(w: u32) -> u128 {
    if w >= 128 {
        u128::MAX
    } else {
        (1u128 << w) - 1
    }
}

/// Evaluate an RFS expression to a concrete value.
pub(crate) fn eval_expr(e: &Expr, ctx: &EvalCtx) -> Result<Val, String> {
    let ty = expr_type(e, ctx.module, &ctx.bind_types)?;
    match e {
        // Task 3L: a const-const comparison folds both sides at full precision
        // and compares SIGNED (u64 two's-complement would misorder negatives).
        Expr::Binary { op, left, right }
            if matches!(
                op,
                BinOp::Eq | BinOp::NotEq | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge
            ) && is_const_expr(left)
                && is_const_expr(right) =>
        {
            let a = fold_const(left)?;
            let b = fold_const(right)?;
            let c = match op {
                BinOp::Eq => a == b,
                BinOp::NotEq => a != b,
                BinOp::Lt => a < b,
                BinOp::Le => a <= b,
                BinOp::Gt => a > b,
                _ => a >= b,
            };
            Ok(Val::Bool(c))
        }
        // Task 3L: a maximal constant subexpression folds at full precision;
        // the parent masks to the context width (two's-complement in 64 bits
        // masked to N bits is the correct N-bit two's complement).
        _ if is_const_expr(e) => {
            let v = fold_const(e)?;
            Ok(Val::Bit(v as u64))
        }
        // Explicit conversion cast (task 3L): evaluate the operand, mask to width.
        Expr::Cast { width, expr } => {
            let n = eval_expr(expr, ctx)?
                .num()
                .ok_or("cast operand not numeric")?;
            Ok(Val::Bit((n as u128 & mask(*width)) as u64))
        }
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
                    // Task 3L: the result width is the EXPRESSION's type - the
                    // shifted value's width, or the shift amount's width when
                    // the value is a constant (the refinement). A bare IntLit
                    // has no width of its own, so the left operand's type must
                    // not be used here.
                    let w = ty.width().ok_or("shift width")?;
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

pub(crate) fn bool_of(v: &Val) -> Result<bool, String> {
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
pub(crate) enum Sig {
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
pub(crate) fn eval_sig(sig: &Sig, s: &Sample, module: &Module) -> Result<bool, String> {
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

pub(crate) fn flatten(groups: &[ConstructorGroup]) -> Vec<FlatVariant> {
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
pub(crate) struct Range {
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
pub(crate) enum Mon {
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
pub(crate) fn compile_formula(f: &Formula) -> Result<Mon, String> {
    compile_check(f, Range::all(), false)
}

pub(crate) fn compile_signal(f: &Formula) -> Option<Sig> {
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
pub(crate) enum MonSt {
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
    pub(crate) fn trigger_step(&self) -> i64 {
        match self {
            MonSt::Obligation { trigger_step, .. }
            | MonSt::And { trigger_step, .. }
            | MonSt::Or { trigger_step, .. } => *trigger_step,
            _ => -1,
        }
    }
}

pub(crate) fn mon_init(spec: &Mon) -> MonSt {
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
pub(crate) fn mon_step(
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
pub(crate) fn mon_key(st: &MonSt, spec: &Mon) -> Vec<u64> {
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
pub(crate) fn mon_bits(spec: &Mon) -> u32 {
    match spec {
        Mon::Delayed { .. } => 1,
        Mon::Obligation { bound, .. } if *bound >= 1 => 1 + ceil_log2(bound.saturating_add(1)),
        Mon::And(children) | Mon::Or(children) => children.iter().map(mon_bits).sum(),
        _ => 0,
    }
}

/// Smallest k with 2^k >= n (0 for n <= 1).
pub(crate) fn ceil_log2(n: u64) -> u32 {
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

/// The fuzz-mode input enumeration (D-P16-4 parity): ONE representative input
/// per event variant, in declaration order - exactly the generated fuzz
/// runtimes' `ALL_INPUTS` (one representative value per field, NOT the full
/// domain Cartesian product that exact mode's `all_inputs` enumerates). The
/// fuzz strategy randomizes over this list; matching it is what makes the
/// seeded draw sequence parity-hold across all three engines.
pub(crate) fn fuzz_inputs(module: &Module) -> Vec<InputVal> {
    let variants = flatten(&module.events);
    variants
        .iter()
        .enumerate()
        .map(|(vi, v)| InputVal {
            variant: vi,
            fields: v
                .fields
                .iter()
                .map(|(_, ty)| representative_value(ty, module).unwrap_or(Val::Bit(0)))
                .collect(),
        })
        .collect()
}

/// The canonical input enumeration (section 9.4): event variants in
/// declaration order; within a variant, fields in declaration order; concrete
/// fields over their full domain, abstracted fields at their representative.
pub(crate) fn all_inputs(module: &Module) -> Vec<InputVal> {
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
pub(crate) struct Effect {
    /// The transition id that produced the effect (`usize::MAX` = on_error).
    pub tid: usize,
    pub next_state: usize,
    pub next_vars: Vec<Val>,
    pub outputs: Vec<OutputVal>,
}

/// Bindings of a matched pattern: (bind name -> field value), with the
/// binding types for expression typing.
pub(crate) fn pattern_bindings(
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
pub(crate) fn match_effect(
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

pub(crate) fn input_group(module: &Module, input: &InputVal) -> Option<String> {
    let variants = flatten(&module.events);
    Some(variants[input.variant].group.clone())
}

pub(crate) fn input_matches_variant(module: &Module, t: &Transition, input: &InputVal) -> bool {
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

pub(crate) fn var_map(module: &Module, vars: &[Val]) -> HashMap<String, Val> {
    module
        .vars
        .iter()
        .zip(vars.iter())
        .map(|(v, val)| (v.name.clone(), val.clone()))
        .collect()
}

pub(crate) fn state_index(module: &Module, name: &str) -> Option<usize> {
    state_variants(module).iter().position(|s| s == name)
}

pub(crate) fn output_variant_idx(module: &Module, o: &OutputExpr) -> Option<usize> {
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
pub(crate) enum Leaf {
    Safety(Formula),
    Eventually(Formula),
    Until { left: Formula, right: Formula },
}

/// Top-level boolean skeleton of a liveness property (all leaves replaced by
/// indices into [`LivProp::leaves`]).
#[derive(Debug, Clone)]
pub(crate) enum Skel {
    And(Box<Skel>, Box<Skel>),
    Or(Box<Skel>, Box<Skel>),
    Implies(Box<Skel>, Box<Skel>),
    Not(Box<Skel>),
    Leaf(usize),
}

/// A liveness-bearing, checkable property.
#[derive(Debug, Clone)]
pub(crate) struct LivProp {
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
pub(crate) fn contains_liveness(f: &Formula) -> bool {
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
pub(crate) fn decompose_properties(module: &Module) -> (Vec<Option<LivProp>>, Vec<String>) {
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
        Expr::Cast { expr, .. } => expr_has_boolean_op(expr),
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
        // Explicit conversion cast (task 3L): `bit<N>(operand)`, tightest binding.
        Expr::Cast { width, expr } => {
            let inner = if matches!(expr.as_ref(), Expr::Path { path } if path.len() == 1) {
                print_expr_min(expr, PREC_PRIMARY)
            } else {
                format!("({})", print_expr_min(expr, 0))
            };
            (format!("bit<{}>({})", width, inner), PREC_PRIMARY)
        }
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
    /// `--nondet`: fire EVERY enabled transition as its own successor (the
    /// generated engines' nondet mode) instead of only the first match.
    /// Liveness then requires STRONGLY FAIR bad cycles (D-P4-11).
    pub nondet: bool,
}

impl Default for RefConfig {
    fn default() -> Self {
        Self {
            max_states: EXACT_DEFAULT_MAX_STATES,
            timeout_sec: EXACT_DEFAULT_TIMEOUT_SEC,
            assume_arg: "all".to_string(),
            nondet: false,
        }
    }
}

/// Configuration for one fuzz-mode run (the generated fuzz binaries' flag
/// surface, D-P16-4): same names, defaults and wire formats.
#[derive(Debug, Clone)]
pub struct FuzzConfig {
    /// `--bound N`: steps per run (default 5).
    pub bound: usize,
    /// `--runs N`: number of runs (default 20).
    pub runs: usize,
    /// `--seed N`: the LCG seed (default 12345).
    pub seed: u64,
    /// `--assumes` wire string: `all`, `none`, or comma-separated labels.
    pub assume_arg: String,
    /// `--violate label:prob:percent%,...` parsed directives (default empty).
    pub violate: Vec<ViolationSpec>,
    /// `--unexpected-prob P`: probability of injecting an unexpected input per
    /// step (default 0.0).
    pub unexpected_prob: f64,
}

impl Default for FuzzConfig {
    fn default() -> Self {
        Self {
            bound: FUZZ_DEFAULT_BOUND,
            runs: FUZZ_DEFAULT_RUNS,
            seed: FUZZ_DEFAULT_SEED,
            assume_arg: "all".to_string(),
            violate: Vec::new(),
            unexpected_prob: 0.0,
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

/// Exact-mode entry point (D-P16-1): delegates to the exact engine in
/// `ref_mc_exact.rs`. Kept here so that `rfs::mc::ref_mc::run` remains the
/// documented public entry for the CLI and the differential suites.
pub fn run(module: &Module, cfg: &RefConfig) -> Result<RefResult, String> {
    ref_mc_exact::run(module, cfg)
}

/// Fuzz-mode entry point (D-P16-1): delegates to the seeded engine in
/// `ref_mc_fuzz.rs`. Returns the report (summary line after all runs, or the
/// per-run violation report that terminated a run) - the CLI maps it to the
/// 0/1 exit contract.
pub fn run_fuzz(module: &Module, cfg: &FuzzConfig) -> Result<ref_mc_fuzz::FuzzReport, String> {
    ref_mc_fuzz::run(module, cfg)
}

/// SMT-bounded entry point (D-P16-1): delegates to the in-process z3 executor
/// in `ref_mc_smt.rs` (feature-gated - codegen of smt artifacts is NOT gated,
/// in-process execution IS). `full_properties` enables the FULL property class
/// (the generated binaries' --full-properties); `full_assumptions` is the
/// DISTINCT gate for the FULL assumption class (task 3C - expensive/cheap is
/// chosen independently for properties vs assumptions); `assumes_arg` is the
/// `--assumes` wire string (`all` / `none` / comma-separated labels). Returns
/// the outcome (exit code 0/1/2 + full stdout + stderr warnings) - the CLI
/// prints it and maps the exit code.
#[cfg(feature = "smt")]
pub fn run_smt(
    module: &Module,
    bound: usize,
    full_properties: bool,
    full_assumptions: bool,
    assumes_arg: &str,
) -> Result<ref_mc_smt::SmtOutcome, String> {
    ref_mc_smt::run(
        module,
        bound,
        full_properties,
        full_assumptions,
        assumes_arg,
    )
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

pub(crate) fn render_vars(module: &Module, vars: &[Val]) -> String {
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
pub(crate) fn render_input(io: &IoRec, variants: &[FlatVariant]) -> String {
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
pub(crate) fn render_output(io: &IoRec, variants: &[FlatVariant]) -> String {
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

pub(crate) fn render_outputs(outs: &[IoRec], variants: &[FlatVariant]) -> String {
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
    use crate::json::json_rw::Json;

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
        // The in-process reference MC is not a standalone binary: it does not
        // self-report RSS. 0 keeps the field present (structural unification
        // with the generated engines) and the document deterministic.
        peak_rss_kb: 0,
        counterexample,
    }
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
pub(crate) fn initial_config(module: &Module) -> Result<(usize, Vec<Val>), String> {
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
fn json_val(j: &crate::json::json_rw::Json, what: &str) -> Result<Val, String> {
    match j {
        crate::json::json_rw::Json::Bool(b) => Ok(Val::Bool(*b)),
        crate::json::json_rw::Json::Int(n) => Ok(Val::Bit(*n as u64)),
        crate::json::json_rw::Json::Str(s) => Ok(Val::Enum(
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

/// Replay a CE's input sequence through ref_mc's OWN transition logic, one
/// sample per step (sample 0 = the initial configuration).
///
/// Deterministic (`nondet == false`): each step takes the FIRST matching effect
/// (the generated engines' default exact mode), so the replay is unambiguous.
/// Nondet (`nondet == true`): a step's input may enable SEVERAL effects; the CE
/// took one specific one, so the replay selects the enabled effect whose
/// (to_state, vars, outputs) matches the CE's claim for that step - and errors
/// if the claim is not an enabled effect at all. This keeps the verification a
/// genuine ground-truth check (an invalid nondet CE is still rejected) while
/// following the actual path the engine took.
pub fn step_trace(
    module: &Module,
    steps: &[JsonStep],
    nondet: bool,
) -> Result<Vec<Sample>, String> {
    let (q0, mut vars) = initial_config(module)?;
    let mut state = q0;
    let mut samples = vec![Sample {
        input: None,
        outputs: Vec::new(),
        to_state: state,
        vars: vars.clone(),
    }];
    for (k, step) in steps.iter().enumerate() {
        let input = io_to_input(module, &step.input)?;
        if nondet {
            // The claim must be ONE of the enabled effects (not just the first).
            let effects = match_effects_all(module, state, &vars, &input);
            let claimed_to = ce_state_name(&step.to_state);
            let to_idx = state_index(module, claimed_to)
                .ok_or_else(|| format!("CE step {}: unknown to_state '{}'", k + 1, claimed_to))?;
            let mut claimed_vars = vars.clone();
            for (name, j) in &step.vars {
                let pos = module
                    .vars
                    .iter()
                    .position(|v| &v.name == name)
                    .ok_or_else(|| format!("CE step {}: unknown var '{name}'", k + 1))?;
                claimed_vars[pos] = json_val(j, &format!("CE step {} var {name}", k + 1))?;
            }
            let claimed_outputs: Vec<OutputVal> = step
                .outputs
                .iter()
                .map(|jio| io_to_output(module, jio))
                .collect::<Result<_, _>>()?;
            let matched = effects.iter().any(|e| {
                e.next_state == to_idx
                    && e.next_vars == claimed_vars
                    && e.outputs == claimed_outputs
            });
            if !matched {
                return Err(format!(
                    "CE step {}: claim is not an enabled effect (nondet)",
                    k + 1
                ));
            }
            samples.push(Sample {
                input: Some(input),
                outputs: claimed_outputs,
                to_state: to_idx,
                vars: claimed_vars.clone(),
            });
            state = to_idx;
            vars = claimed_vars;
        } else {
            let eff = match_effect(module, state, &vars, &input).ok_or_else(|| {
                format!(
                    "CE step {} is not a valid execution (no enabled transition)",
                    k + 1
                )
            })?;
            samples.push(Sample {
                input: Some(input),
                outputs: eff.outputs,
                to_state: eff.next_state,
                vars: eff.next_vars.clone(),
            });
            state = eff.next_state;
            vars = eff.next_vars;
        }
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
    initial_vars: &[(String, crate::json::json_rw::Json)],
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
    vars: &[(String, crate::json::json_rw::Json)],
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
            let samples = step_trace(module, steps, cfg.nondet)?;
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
            let all_steps: Vec<JsonStep> = prefix.iter().chain(cycle.iter()).cloned().collect();
            let samples = step_trace(module, &all_steps, cfg.nondet)?;
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

    /// Task 3L: the validator's `type_expr` and this engine's `expr_type`
    /// must agree WORD-FOR-WORD on every expression form - a drift between
    /// the two is a bug (the third implementation, smt_plan's `se_type`, is
    /// pinned by the same corpus end-to-end through smt-bounded runs).
    #[test]
    fn typing_equivalence_validator_vs_ref_mc() {
        let m = module(
            "module m { enum State { A } initial A event E output O \
             enum Mode { Run, Stop } \
             var x: bit<8> init 0 var y: bit<3> init 0 var k: bit<3> init 0 \
             var b: bool init false var md: Mode init Mode::Run \
             transition A on E { next A; emit O; } }",
        );
        let env = crate::lang::validator::Env::for_module(&m);
        let binds: HashMap<String, Type> = HashMap::new();

        // (expression source, expected validator type)
        let corpus: &[(&str, &str)] = &[
            ("5", "IntLit"),
            ("-5", "IntLit"),
            ("~5", "IntLit"),
            ("200 * 200", "IntLit"),
            ("(300 & 255)", "IntLit"),
            ("1 << 4", "IntLit"),
            ("x + 5", "Bit(8)"),
            ("5 + x", "Bit(8)"),
            ("x + y", "ERR"), // bitwidth mismatch: bit<8> vs bit<3>
            ("y + y", "Bit(3)"),
            ("x & 255", "Bit(8)"),
            ("x == 300", "ERR"), // rule 4: positive out-of-range constant
            ("x == 255", "Bool"),
            ("x < y", "ERR"),
            ("b && b", "Bool"),
            ("!b", "Bool"),
            ("~x", "Bit(8)"),
            ("-x", "Bit(8)"),
            ("x << k", "Bit(8)"), // value's width
            ("1 << k", "Bit(3)"), // refinement: amount's width
            ("k << x", "Bit(3)"), // value's width (k)
            ("bit<4>(x)", "Bit(4)"),
            ("bit<8>(y)", "Bit(8)"),
            ("bit<2>(5 + 1)", "Bit(2)"),
            ("bit<4> x + y", "ERR"), // (bit<4> x) + y: bit<4> vs bit<3>
            ("bit<4>(true)", "ERR"), // cast operand must be a bitvector
            ("bit<4>(Mode::Run)", "ERR"),
            ("md == Mode::Run", "Bool"),
            ("state == A", "Bool"),
        ];
        for &(src, want) in corpus {
            // Guard position holds a pure Expr (no formula-level operators to
            // confuse the extraction); parsing does not type-check.
            let expr = parse_source(&format!(
                "module m {{ enum State {{ A }} initial A event E output O \
                 enum Mode {{ Run, Stop }} var x: bit<8> init 0 var y: bit<3> init 0 \
                 var k: bit<3> init 0 var b: bool init false var md: Mode init Mode::Run \
                 transition A on E where {src} {{ next A; emit O; }} }}"
            ))
            .unwrap_or_else(|err| panic!("parse '{src}': {err}"))
            .modules
            .pop()
            .unwrap()
            .transitions[0]
                .guard
                .clone()
                .expect("guard present");
            let v = crate::lang::validator::type_expr(&expr, &env, &mut Vec::new());
            let r = expr_type(&expr, &m, &binds);
            match want {
                "ERR" => {
                    assert!(v.is_err(), "validator must reject '{src}'");
                    assert!(r.is_err(), "ref_mc must reject '{src}'");
                }
                _ => {
                    let vt = v.unwrap_or_else(|e| panic!("validator '{src}': {e}"));
                    let rt = r.unwrap_or_else(|e| panic!("ref_mc '{src}': {e}"));
                    let vname = match vt {
                        crate::lang::validator::ExprType::Bool => "Bool".to_string(),
                        crate::lang::validator::ExprType::Bit(w) => format!("Bit({w})"),
                        crate::lang::validator::ExprType::IntLit => "IntLit".to_string(),
                        crate::lang::validator::ExprType::Enum(n) => format!("Enum({n})"),
                    };
                    let rname = match rt {
                        RType::Bool => "Bool".to_string(),
                        RType::Bit(w) => format!("Bit({w})"),
                        RType::IntLit => "IntLit".to_string(),
                        RType::Enum(n) => format!("Enum({n})"),
                    };
                    assert_eq!(vname, want, "validator '{src}'");
                    assert_eq!(rname, want, "ref_mc '{src}' (drift from validator)");
                }
            }
        }
    }

    fn cfg() -> RefConfig {
        RefConfig::default()
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
        // D-P16-2, scoped per file: reads EVERY src/mc/ref_mc_*.rs. Forbidden
        // for all of them: the plan/monitor/codegen/backend modules. Documented
        // exceptions: ref_mc_smt.rs MAY import crate::mc::plan AND
        // crate::mc::smt_plan (the shared encoding model chain, D-P16-3);
        // ref_mc_fuzz.rs MAY import crate::utils::rng (one RNG family by
        // construction - not a forbidden module, noted for completeness).
        let dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/mc");
        let files: Vec<String> = std::fs::read_dir(&dir)
            .expect("read src/mc")
            .flatten()
            .map(|e| e.file_name().to_string_lossy().into_owned())
            .filter(|n| n.starts_with("ref_mc_") && n.ends_with(".rs"))
            .collect();
        assert!(!files.is_empty(), "no ref_mc_*.rs files found under src/mc");
        for name in &files {
            let src = std::fs::read_to_string(dir.join(name)).expect("read file");
            let forbidden: &[&str] = if name == "ref_mc_smt.rs" {
                &["monitor", "codegen::fsm", "printer", "backends"]
            } else {
                &["mc::plan", "monitor", "codegen::fsm", "printer", "backends"]
            };
            for forbidden in forbidden {
                assert!(
                    !src.lines().any(|l| {
                        let t = l.trim();
                        (t.starts_with("use crate::") || t.contains("crate::"))
                            && t.contains(forbidden)
                            && !t.starts_with("//!")
                    }),
                    "{name} must not import '{forbidden}'"
                );
            }
        }
    }
}
