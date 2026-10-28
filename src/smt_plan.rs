//! Shared SMT bounded-mode spec (task 11): a backend-neutral encoding of a
//! module for k-step unrolling over z3 bitvectors. Both backends render this
//! SAME structure, so the constraint construction is identical in Rust and C
//! generated binaries (parity by construction, D-P4-12).
//!
//! v1 scope (documented limitations, rejected at codegen with shared errors):
//! - no assumptions, no on_error blocks;
//! - properties: top-level `always (phi)` / `eventually (phi)` only, where phi
//!   is a boolean combination of expressions over state/vars (no input/output
//!   references, no nested temporal operators);
//! - variables: bit<N> and bool only (no enum-typed vars);
//! - concrete payload fields: bit<N>/bool with N <= 64.
//! Docs: dev:mc-engine-overview, llm:mc-engine

use crate::ir::{Expr, Formula, Module};
use crate::mc_plan::{McPlan, ValueDomain};

/// Expression node over (state, vars, payload fields) with references already
/// resolved to indices. Mirrors the IR expression language.
#[derive(Debug, Clone)]
pub enum Se {
    /// Literal. `is_bool` distinguishes a bool literal (z3 Bool in boolean
    /// contexts) from an integer literal (IntLit, lifted by parent context).
    Lit {
        value: u64,
        is_bool: bool,
    },
    /// Current state (BV of `state_bits`).
    State,
    /// Variable index into [`SmtSpec::vars`].
    Var(usize),
    /// Concrete payload field: index into the CURRENT input class's concrete
    /// fields (see [`SmtTrans::class`]), with the field's bit width (1 for
    /// bool) and bool-ness captured at conversion time - both are
    /// class-relative, so they cannot be recovered from the spec alone.
    Field {
        idx: usize,
        width: u32,
        is_bool: bool,
    },
    Not(Box<Se>),
    BitNot(Box<Se>),
    Neg(Box<Se>),
    Bin(Sb, Box<Se>, Box<Se>),
}

/// Binary operators (subset of the IR `BinOp`, same names).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Sb {
    Plus,
    Minus,
    Star,
    ShiftLeft,
    ShiftRight,
    Eq,
    NotEq,
    Lt,
    Le,
    Gt,
    Ge,
    BitOr,
    BitXor,
    BitAnd,
    LogicOr,
    LogicAnd,
}

/// Property formula (boolean layer) over configuration expressions.
#[derive(Debug, Clone)]
pub enum Sf {
    Se(Se),
    Not(Box<Sf>),
    And(Box<Sf>, Box<Sf>),
    Or(Box<Sf>, Box<Sf>),
    Implies(Box<Sf>, Box<Sf>),
}

#[derive(Debug, Clone)]
pub struct SmtVar {
    pub name: String,
    /// Bit width (1 for bool vars).
    pub width: u32,
    /// True when the IR type is `bool` (vs `bit<1>`) - witness display only
    /// (true/false vs 0/1); both encode as 1-bit z3 values.
    pub is_bool: bool,
    /// Initial value (literal inits only, v1).
    pub init: u64,
}

/// One canonical input class = one event variant. Concrete fields become free
/// z3 variables per unrolled step; abstracted fields are substituted as their
/// representative literal during expression conversion.
#[derive(Debug, Clone)]
pub struct SmtClass {
    /// Display name for counterexample output (the plan's unique_name).
    pub name: String,
    /// Concrete fields only: (name, bit width; 1 for bool, is_bool - witness
    /// display only: true/false vs 0/1).
    pub fields: Vec<(String, u32, bool)>,
}

#[derive(Debug, Clone)]
pub struct SmtTrans {
    /// State index into [`SmtSpec::states`] this transition fires from.
    pub state: usize,
    pub next_state: usize,
    /// Input class (event variant) index.
    pub class: usize,
    /// Guard over (state, vars, payload fields); None = always enabled.
    pub guard: Option<Se>,
    /// Variable updates: (var index, expression). Vars absent here are
    /// unchanged by the transition.
    pub assignments: Vec<(usize, Se)>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SmtPropKind {
    Always,
    Eventually,
}

#[derive(Debug, Clone)]
pub struct SmtProp {
    pub name: String,
    pub kind: SmtPropKind,
    pub formula: Sf,
}

/// The complete backend-neutral SMT spec.
#[derive(Debug, Clone)]
pub struct SmtSpec {
    /// State names in declaration order (the module's `State` enum).
    pub states: Vec<String>,
    pub initial_state: usize,
    /// Bit width of the state encoding: max(1, ceil_log2(states.len())).
    pub state_bits: u32,
    pub vars: Vec<SmtVar>,
    pub classes: Vec<SmtClass>,
    /// Transitions in FIRING order (priority desc, declaration order for ties)
    /// - the same order the exact engine uses for first-match selection.
    pub transitions: Vec<SmtTrans>,
    pub properties: Vec<SmtProp>,
}

/// Context for converting one expression: name resolution follows the
/// validator's order exactly (state -> payload bindings -> vars -> state
/// variants).
struct ExprCtx {
    var_idx: std::collections::HashMap<String, usize>,
    /// Payload field name -> Some(idx into the class's concrete fields) or
    /// None (abstracted; the caller substitutes the representative literal).
    field: std::collections::HashMap<String, Option<usize>>,
    state_variant_idx: std::collections::HashMap<String, usize>,
}

/// ceil(log2(n)), minimum 1 (bit width of an index space of size n).
pub fn ceil_log2_pub(n: usize) -> u32 {
    if n <= 1 {
        return 1;
    }
    usize::BITS - (n - 1).leading_zeros()
}

fn ceil_log2(n: usize) -> u32 {
    ceil_log2_pub(n)
}

/// Build the shared SMT spec from a validated module + its MC plan. Returns a
/// shared error string for unsupported constructs (identical in both
/// backends, so codegen behavior is parity-safe).
pub fn build_smt_spec(module: &Module, plan: &McPlan) -> Result<SmtSpec, String> {
    if !module.assumptions.is_empty() {
        return Err(format!(
            "smt-bounded mode does not support assumptions (spec has {})",
            module.assumptions.len()
        ));
    }
    if module.on_error.is_some() {
        return Err("smt-bounded mode does not support on_error blocks".into());
    }

    // States: the module's `State` enum, declaration order.
    let state_enum = module
        .enums
        .iter()
        .find(|e| e.name == "State")
        .ok_or("module has no State enum")?;
    let states: Vec<String> = state_enum.variants.clone();
    let initial_state = match &module.initial_state {
        Some(name) => states
            .iter()
            .position(|s| s == name)
            .ok_or_else(|| format!("unknown initial state '{name}'"))?,
        None => 0,
    };

    // Vars: bit<N>/bool only (v1).
    let mut vars: Vec<SmtVar> = Vec::new();
    let mut var_idx: std::collections::HashMap<String, usize> = std::collections::HashMap::new();
    for v in &module.vars {
        let is_bool = matches!(v.ty, crate::ir::Type::Bool);
        let width = match &v.ty {
            crate::ir::Type::Bool => 1u32,
            crate::ir::Type::Bit { width } => *width,
            crate::ir::Type::Enum { name: _ } => {
                return Err(format!(
                    "smt-bounded mode does not support enum-typed variables (var '{}')",
                    v.name
                ))
            }
        };
        // Literal inits only (v1): no var/state references.
        let init = match &v.init {
            Expr::Int { value } => *value,
            Expr::Bool { value } => u64::from(*value),
            other => {
                return Err(format!(
                    "smt-bounded mode requires literal initial values (var '{}' has {:?})",
                    v.name, other
                ))
            }
        };
        if init >= (1u64 << width.min(64)) {
            return Err(format!(
                "initial value of var '{}' does not fit in {} bits",
                v.name, width
            ));
        }
        var_idx.insert(v.name.clone(), vars.len());
        vars.push(SmtVar {
            name: v.name.clone(),
            width,
            is_bool,
            init,
        });
    }

    // Input classes from the plan's abstraction table.
    let mut classes: Vec<SmtClass> = Vec::new();
    for vi in &plan.abstraction.variants {
        let mut fields: Vec<(String, u32, bool)> = Vec::new();
        for f in &vi.fields {
            if !f.concrete {
                continue; // abstracted: substituted as a literal during conversion
            }
            let is_bool = matches!(f.domain, Some(ValueDomain::Bool));
            let width = match &f.domain {
                Some(ValueDomain::Bool) => 1u32,
                Some(ValueDomain::Int { max }) => {
                    if *max == u64::MAX {
                        return Err(format!(
                            "smt-bounded mode does not support concrete payload fields wider than 64 bits (field '{}')",
                            f.name
                        ));
                    }
                    let w = ceil_log2((*max as usize) + 1);
                    if (1u64 << w) - 1 != *max {
                        return Err(format!(
                            "non-power-of-two domain for field '{}' (not a bit<N> type)",
                            f.name
                        ));
                    }
                    w
                }
                Some(ValueDomain::Enum { .. }) => {
                    return Err(format!(
                        "smt-bounded mode does not support enum-typed payload fields (field '{}')",
                        f.name
                    ))
                }
                None => unreachable!("concrete field has a domain"),
            };
            fields.push((f.name.clone(), width, is_bool));
        }
        classes.push(SmtClass {
            name: vi.unique_name.clone(),
            fields,
        });
    }

    // State-variant names for expression resolution.
    let mut state_variant_idx: std::collections::HashMap<String, usize> =
        std::collections::HashMap::new();
    for (i, s) in states.iter().enumerate() {
        state_variant_idx.insert(s.clone(), i);
    }

    // Transitions in firing order (priority desc, stable).
    let mut order: Vec<usize> = (0..module.transitions.len()).collect();
    order.sort_by_key(|&i| std::cmp::Reverse(module.transitions[i].priority.unwrap_or(0)));
    let mut transitions: Vec<SmtTrans> = Vec::new();
    for &ti in &order {
        let t = &module.transitions[ti];
        let state = states
            .iter()
            .position(|s| *s == t.state)
            .ok_or_else(|| format!("transition {} references unknown state '{}'", t.id, t.state))?;
        let next_state = states
            .iter()
            .position(|s| *s == t.next_state)
            .ok_or_else(|| {
                format!(
                    "transition {} references unknown next state '{}'",
                    t.id, t.next_state
                )
            })?;
        // Match the transition's pattern to an input class. A bare group
        // pattern (variant: None) names exactly one variant when the group is
        // single-variant; multi-variant wildcards are rejected in v1.
        let class = if t.pattern.variant.is_none() {
            let candidates: Vec<usize> = plan
                .abstraction
                .variants
                .iter()
                .enumerate()
                .filter(|(_, v)| v.group == t.pattern.group)
                .map(|(i, _)| i)
                .collect();
            if candidates.len() != 1 {
                return Err(format!(
                    "smt-bounded mode does not support wildcard event patterns over multi-variant groups (transition {} on '{}')",
                    t.id, t.pattern.group
                ));
            }
            candidates.into_iter().next().expect("one candidate")
        } else {
            plan.abstraction
                .variants
                .iter()
                .position(|v| {
                    v.group == t.pattern.group
                        && variant_name(&t.pattern.variant).as_deref() == Some(v.variant.as_str())
                })
                .ok_or_else(|| format!("transition {} has no matching input class", t.id))?
        };

        // Conversion context for this transition's payload fields.
        let mut field: std::collections::HashMap<String, Option<usize>> =
            std::collections::HashMap::new();
        for f in &plan.abstraction.variants[class].fields {
            field.insert(
                f.name.clone(),
                if f.concrete {
                    Some(fi_concrete(&classes[class], &f.name))
                } else {
                    None
                },
            );
        }
        let ctx = ExprCtx {
            var_idx: var_idx.clone(),
            field,
            state_variant_idx: state_variant_idx.clone(),
        };

        let guard = match &t.guard {
            Some(g) => Some(convert_expr(g, &ctx, &plan.abstraction.variants[class])?),
            None => None,
        };
        let mut assignments: Vec<(usize, Se)> = Vec::new();
        for a in &t.assignments {
            let vi = var_idx
                .get(&a.var)
                .copied()
                .ok_or_else(|| format!("transition {} assigns unknown var '{}'", t.id, a.var))?;
            assignments.push((
                vi,
                convert_expr(&a.value, &ctx, &plan.abstraction.variants[class])?,
            ));
        }
        transitions.push(SmtTrans {
            state,
            next_state,
            class,
            guard,
            assignments,
        });
    }

    // Properties: top-level always/eventually only (v1).
    let mut properties: Vec<SmtProp> = Vec::new();
    for p in &module.properties {
        let (kind, inner) = match &p.formula {
            Formula::Always { inner } => (SmtPropKind::Always, inner),
            Formula::Eventually { inner } => (SmtPropKind::Eventually, inner),
            other => {
                return Err(format!(
                    "smt-bounded mode supports top-level always/eventually properties only (property '{}' is {:?})",
                    p.name, formula_tag(other)
                ))
            }
        };
        // Property context: no payload bindings.
        let ctx = ExprCtx {
            var_idx: var_idx.clone(),
            field: std::collections::HashMap::new(),
            state_variant_idx: state_variant_idx.clone(),
        };
        let formula = convert_formula(inner, &ctx)?;
        properties.push(SmtProp {
            name: p.name.clone(),
            kind,
            formula,
        });
    }

    let state_bits = ceil_log2(states.len());
    Ok(SmtSpec {
        states,
        initial_state,
        state_bits,
        vars,
        classes,
        transitions,
        properties,
    })
}

fn variant_name(v: &Option<crate::ir::PatternVariant>) -> Option<String> {
    match v {
        Some(crate::ir::PatternVariant::Unit { name }) => Some(name.clone()),
        Some(crate::ir::PatternVariant::Struct { name, .. }) => Some(name.clone()),
        None => None,
    }
}

fn formula_tag(f: &Formula) -> &'static str {
    match f {
        Formula::InputIs { .. } => "input_is",
        Formula::OutputHas { .. } => "output_has",
        Formula::Expr { .. } => "expr",
        Formula::Not { .. } => "not",
        Formula::And { .. } => "and",
        Formula::Or { .. } => "or",
        Formula::Implies { .. } => "implies",
        Formula::Next { .. } => "next",
        Formula::Always { .. } => "always",
        _ => "temporal",
    }
}

/// Index of a concrete field within the class's concrete-fields list.
fn fi_concrete(class: &SmtClass, name: &str) -> usize {
    class
        .fields
        .iter()
        .position(|(n, _, _)| n == name)
        .expect("concrete field listed")
}

fn convert_expr(
    e: &Expr,
    ctx: &ExprCtx,
    variant: &crate::mc_plan::VariantInputSpec,
) -> Result<Se, String> {
    match e {
        Expr::Int { value } => Ok(Se::Lit {
            value: *value,
            is_bool: false,
        }),
        Expr::Bool { value } => Ok(Se::Lit {
            value: u64::from(*value),
            is_bool: true,
        }),
        Expr::Path { path } => {
            if path.len() != 1 {
                return Err(format!(
                    "smt-bounded mode does not support enum paths in expressions ({:?})",
                    path
                ));
            }
            let name = &path[0];
            if name == "state" {
                return Ok(Se::State);
            }
            if let Some(f) = ctx.field.get(name) {
                match f {
                    Some(idx) => {
                        let fs = variant
                            .fields
                            .iter()
                            .find(|fs| fs.name == *name)
                            .expect("field listed in class");
                        let (width, is_b) = match &fs.domain {
                            Some(crate::mc_plan::ValueDomain::Bool) => (1u32, true),
                            Some(crate::mc_plan::ValueDomain::Int { max }) => {
                                (ceil_log2((*max as usize) + 1), false)
                            }
                            _ => unreachable!("concrete field domain"),
                        };
                        return Ok(Se::Field {
                            idx: *idx,
                            width,
                            is_bool: is_b,
                        });
                    }
                    None => {
                        // Abstracted field: substitute the representative literal.
                        let spec = variant
                            .fields
                            .iter()
                            .find(|fs| fs.name == *name)
                            .ok_or_else(|| format!("unknown payload field '{name}'"))?;
                        return match &spec.representative {
                            Some(crate::codegen_fsm::RepValue::Bool(b)) => {
                                Ok(Se::Lit { value: u64::from(*b), is_bool: true })
                            }
                            Some(crate::codegen_fsm::RepValue::Int(v)) => {
                                Ok(Se::Lit { value: *v, is_bool: false })
                            }
                            other => Err(format!(
                                "smt-bounded mode cannot substitute representative {:?} for field '{name}'",
                                other.as_ref().map(|_| "enum")
                            )),
                        };
                    }
                }
            }
            if let Some(vi) = ctx.var_idx.get(name) {
                return Ok(Se::Var(*vi));
            }
            if let Some(si) = ctx.state_variant_idx.get(name) {
                return Ok(Se::Lit {
                    value: *si as u64,
                    is_bool: false,
                });
            }
            Err(format!("unknown identifier '{name}' in expression"))
        }
        Expr::Unary { op, expr } => {
            let inner = convert_expr(expr, ctx, variant)?;
            Ok(match op {
                crate::ir::UnaryOp::Not => Se::Not(Box::new(inner)),
                crate::ir::UnaryOp::BitNot => Se::BitNot(Box::new(inner)),
                crate::ir::UnaryOp::Neg => Se::Neg(Box::new(inner)),
            })
        }
        Expr::Binary { op, left, right } => {
            let l = convert_expr(left, ctx, variant)?;
            let r = convert_expr(right, ctx, variant)?;
            let sb = match op {
                crate::ir::BinOp::Plus => Sb::Plus,
                crate::ir::BinOp::Minus => Sb::Minus,
                crate::ir::BinOp::Star => Sb::Star,
                crate::ir::BinOp::ShiftLeft => Sb::ShiftLeft,
                crate::ir::BinOp::ShiftRight => Sb::ShiftRight,
                crate::ir::BinOp::Eq => Sb::Eq,
                crate::ir::BinOp::NotEq => Sb::NotEq,
                crate::ir::BinOp::Lt => Sb::Lt,
                crate::ir::BinOp::Le => Sb::Le,
                crate::ir::BinOp::Gt => Sb::Gt,
                crate::ir::BinOp::Ge => Sb::Ge,
                crate::ir::BinOp::BitOr => Sb::BitOr,
                crate::ir::BinOp::BitXor => Sb::BitXor,
                crate::ir::BinOp::BitAnd => Sb::BitAnd,
                crate::ir::BinOp::LogicOr => Sb::LogicOr,
                crate::ir::BinOp::LogicAnd => Sb::LogicAnd,
            };
            Ok(Se::Bin(sb, Box::new(l), Box::new(r)))
        }
    }
}

fn convert_formula(f: &Formula, ctx: &ExprCtx) -> Result<Sf, String> {
    match f {
        Formula::Expr { expr } => {
            // No payload context in properties: use an empty variant view.
            let variant = crate::mc_plan::VariantInputSpec {
                group: String::new(),
                variant: String::new(),
                unique_name: String::new(),
                fields: Vec::new(),
            };
            Ok(Sf::Se(convert_expr(expr, ctx, &variant)?))
        }
        Formula::Not { inner } => Ok(Sf::Not(Box::new(convert_formula(inner, ctx)?))),
        Formula::And { left, right } => Ok(Sf::And(
            Box::new(convert_formula(left, ctx)?),
            Box::new(convert_formula(right, ctx)?),
        )),
        Formula::Or { left, right } => Ok(Sf::Or(
            Box::new(convert_formula(left, ctx)?),
            Box::new(convert_formula(right, ctx)?),
        )),
        Formula::Implies { left, right } => Ok(Sf::Implies(
            Box::new(convert_formula(left, ctx)?),
            Box::new(convert_formula(right, ctx)?),
        )),
        other => Err(format!(
            "smt-bounded mode property formulas may use expr/not/and/or/implies only (found {})",
            formula_tag(other)
        )),
    }
}

/// Type of an [`Se`] node after name resolution. Mirrors
/// `validator::ExprType` with Enum already resolved to concrete widths;
/// `IntLit` is kept distinct (as in the validator) so binary-op rules can lift
/// it to a sibling's width.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SeType {
    Bool,
    Bit(u32),
    IntLit,
}

impl SeType {
    /// Bit width used for z3 sort selection (bool = 1 bit; IntLit defaults to
    /// 8 - only valid when no parent context applies).
    pub fn width(self) -> u32 {
        match self {
            SeType::Bool => 1,
            SeType::Bit(w) => w,
            SeType::IntLit => 8,
        }
    }
}

/// Bottom-up type of an expression node, computed with EXACTLY the validator's
/// rules (`type_expr`/`binary_type`/`bit_op_type`): integer literals default
/// to `Bit(8)` and are lifted by their parent per the binary-op rules. The
/// spec is already validated, so this never fails; both emitters use it to
/// pick z3 constructors (Bool vs bitvector) and to mask literals to the right
/// width.
pub fn se_type(se: &Se, spec: &SmtSpec) -> SeType {
    match se {
        Se::Lit { is_bool: true, .. } => SeType::Bool,
        Se::Lit { is_bool: false, .. } => SeType::IntLit,
        Se::State => SeType::Bit(spec.state_bits),
        Se::Var(i) => {
            if spec.vars[*i].is_bool {
                SeType::Bool
            } else {
                SeType::Bit(spec.vars[*i].width)
            }
        }
        Se::Field { width, is_bool, .. } => {
            if *is_bool {
                SeType::Bool
            } else {
                SeType::Bit(*width)
            }
        }
        Se::Not(_) => SeType::Bool,
        Se::BitNot(inner) | Se::Neg(inner) => match se_type(inner, spec) {
            // IntLit under bit_not/neg defaults to Bit(8) (validator rule).
            SeType::IntLit => SeType::Bit(8),
            t @ SeType::Bit(_) => t,
            _ => SeType::Bit(8),
        },
        Se::Bin(op, l, r) => {
            let lt = se_type(l, spec);
            let rt = se_type(r, spec);
            match op {
                Sb::LogicAnd | Sb::LogicOr => SeType::Bool,
                Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => SeType::Bool,
                Sb::Plus | Sb::Minus | Sb::Star | Sb::BitXor => bit_op_type(lt, rt),
                Sb::ShiftLeft | Sb::ShiftRight => match lt {
                    // Width comes from the LEFT operand; the right may be any
                    // width (validator rule).
                    SeType::IntLit => SeType::Bit(8),
                    t @ SeType::Bit(_) => t,
                    _ => SeType::Bit(8),
                },
                Sb::BitAnd | Sb::BitOr => {
                    if lt == SeType::Bool && rt == SeType::Bool {
                        SeType::Bool
                    } else {
                        bit_op_type(lt, rt)
                    }
                }
            }
        }
    }
}

/// Validator `bit_op_type`: (Bit(a),Bit(b)) requires a==b; IntLit lifts to the
/// other operand's width; (IntLit,IntLit) -> Bit(8).
fn bit_op_type(lt: SeType, rt: SeType) -> SeType {
    match (lt, rt) {
        (SeType::Bit(a), SeType::Bit(b)) => {
            debug_assert_eq!(a, b, "validated spec: equal widths");
            SeType::Bit(a)
        }
        (SeType::IntLit, SeType::Bit(b)) | (SeType::Bit(b), SeType::IntLit) => SeType::Bit(b),
        (SeType::IntLit, SeType::IntLit) => SeType::Bit(8),
        // Bool operands are rejected by the validator for these ops; a
        // validated spec never reaches this arm.
        _ => SeType::Bit(8),
    }
}
