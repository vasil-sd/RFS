//! Phase 6 task 4, action 5 (subtask 4.5): PSL translation checks.
//!
//! An INDEPENDENT recursive window evaluator for the monitorable PSL subset
//! (SEMANTICS.md 5.1-5.5), cross-checked against the generated online
//! monitors' verdicts over seeded random traces, for ALL property-bearing
//! examples (dynamic enumeration of `examples/valid`). For each example:
//!
//! 1. Generate the production artifact (property monitors ON) and read
//!    `PROPERTY_LABELS` - the exact set of properties codegen actually
//!    monitored (`eventually`/`until` are model-checking-only and skipped
//!    with a warning, SEMANTICS 5/6).
//! 2. Drive the real production FSM with action 3's seeded input script;
//!    parse the recorded trace (init + every S line carrying `st=`/`vars=` -
//!    committed, ASSUME-stopped and PROP-stopped alike) and the generated
//!    verdicts (M-lines: each property confirms at most once, sticky).
//! 3. For every monitored property, run the independent evaluator over the
//!    parsed trace; assert its first-confirmation sample AND trigger sample
//!    match the generated M-line exactly.
//!
//! The evaluator is a from-scratch recursive descent over the Formula/Expr
//! AST with its own bit-vector semantics (SEMANTICS 2.2) and its own
//! obligation scanning (window search, not a countdown automaton) - the
//! structural independence is the point: agreement validates the codegen
//! translation, not a shared implementation. Unsupported formula shapes
//! panic loudly rather than silently passing.
//!
//! ## `RFS_PSL_REF_TOOL` hook (availability-gated)
//!
//! If the environment variable `RFS_PSL_REF_TOOL` names an existing file,
//! then for every property-bearing example the test additionally invokes:
//!
//! ```text
//! $RFS_PSL_REF_TOOL <spec-file> <trace-file>
//! ```
//!
//! `<trace-file>` holds one canonical sample line per recorded sample (sample
//! 0 = init), in order:
//!
//! ```text
//! st={State} vars={v0,v1,...|-} in={Variant|f0,f1|-} out={V1|a,b;V2|c|-}
//! ```
//!
//! (empty var list / no input / no outputs render as `-`; values are raw:
//! bool 0/1, bit<N> decimal, enum = variant index). The tool must print one
//! line per property label it evaluates, in any order:
//!
//! ```text
//! PROP {label} none
//! PROP {label} step={k} trig={t}      (t = -1 for per-sample checks)
//! ```
//!
//! Labels the tool cannot evaluate are omitted. The test asserts every
//! printed verdict agrees with the independent evaluator's verdict for that
//! label and that the tool exits 0. If the variable is unset/empty or the
//! path does not exist, the hook skips silently.

use std::path::{Path, PathBuf};
use std::process::Command;

mod common;
use common::prod_driver;

/// Pinned random-trace parameters (task file: "seeded random traces").
const TRACE_SEED: u64 = 7_331;
const TRACE_STEPS: usize = 200;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn scratch() -> PathBuf {
    root().join("target").join("rfs_psl_translation")
}

// ---------------------------------------------------------------------------
// Sample model + parsing (full fidelity: state, vars, input, outputs).

#[derive(Debug, Clone)]
struct PslSample {
    /// Target state name (initial state for sample 0).
    state: String,
    /// Declaration-order raw var values.
    vars: Vec<u64>,
    /// (flattened variant name, raw field values); None at sample 0.
    input: Option<(String, Vec<u64>)>,
    /// Emitted outputs in declaration order.
    outputs: Vec<(String, Vec<u64>)>,
}

fn parse_name_fields(what: &str, s: &str) -> (String, Vec<u64>) {
    match s.split_once('|') {
        Some((n, f)) => (
            n.to_string(),
            f.split(',')
                .map(|x| {
                    x.parse::<u64>()
                        .unwrap_or_else(|e| panic!("bad {what} field value in [{s}]: {e}"))
                })
                .collect(),
        ),
        None => (s.to_string(), Vec::new()),
    }
}

/// Parse the recorded trace out of an action-3 driver stdout: sample 0 = the
/// init line; every S line carrying ` st=A->B vars=...` contributes a sample
/// (committed, ASSUME-stopped and PROP-stopped alike - all record their
/// in/out/st/vars); UNEXPECTED lines record nothing.
fn parse_trace(out: &str) -> Vec<PslSample> {
    let mut samples = Vec::new();
    for line in out.lines() {
        if let Some(rest) = line.strip_prefix("init st=") {
            let (st, vars_s) = rest.split_once(" vars=").expect("init line shape");
            samples.push(PslSample {
                state: st.to_string(),
                vars: parse_vars(vars_s),
                input: None,
                outputs: Vec::new(),
            });
        } else if let Some(rest) = line.strip_prefix('S') {
            if !rest.starts_with(|c: char| c.is_ascii_digit()) {
                continue;
            }
            let in_i = rest.find(" in=").expect("in= part");
            let out_i = match rest.find(" out=") {
                Some(i) => i,
                None => continue, // UNEXPECTED
            };
            let st_i = rest.find(" st=").expect("st= part");
            let in_s = &rest[in_i + 4..out_i];
            // The output echo ends at the next marker: ` st=` (committed) or
            // ` ASSUME `/` PROP ` (stopped steps carry the status word before
            // the recorded sample).
            let after_out = &rest[out_i + 5..st_i];
            let out_s = after_out
                .find(" ASSUME ")
                .or_else(|| after_out.find(" PROP "))
                .map(|i| &after_out[..i])
                .unwrap_or(after_out);
            let tail = &rest[st_i + 4..];
            let (transition, vars_s) = tail.split_once(" vars=").expect("vars= part");
            let to = transition
                .rsplit_once("->")
                .map(|(_, t)| t)
                .unwrap_or(transition);
            let input = if in_s == "-" {
                None
            } else {
                Some(parse_name_fields("input", in_s))
            };
            let outputs = if out_s == "-" {
                Vec::new()
            } else {
                out_s
                    .split(';')
                    .map(|o| parse_name_fields("output", o))
                    .collect()
            };
            samples.push(PslSample {
                state: to.to_string(),
                vars: parse_vars(vars_s),
                input,
                outputs,
            });
        }
    }
    samples
}

fn parse_vars(s: &str) -> Vec<u64> {
    if s.is_empty() || s == "-" {
        Vec::new()
    } else {
        s.split(',')
            .map(|x| x.parse::<u64>().expect("var value"))
            .collect()
    }
}

/// Canonical sample lines for the `RFS_PSL_REF_TOOL` interface.
fn render_canonical(samples: &[PslSample]) -> String {
    let mut out = String::new();
    for s in samples {
        let vars = if s.vars.is_empty() {
            "-".to_string()
        } else {
            s.vars
                .iter()
                .map(|v| v.to_string())
                .collect::<Vec<_>>()
                .join(",")
        };
        let in_s = match &s.input {
            None => "-".to_string(),
            Some((n, f)) => {
                if f.is_empty() {
                    n.clone()
                } else {
                    format!(
                        "{}|{}",
                        n,
                        f.iter()
                            .map(|v| v.to_string())
                            .collect::<Vec<_>>()
                            .join(",")
                    )
                }
            }
        };
        let out_s = if s.outputs.is_empty() {
            "-".to_string()
        } else {
            s.outputs
                .iter()
                .map(|(n, f)| {
                    if f.is_empty() {
                        n.clone()
                    } else {
                        format!(
                            "{}|{}",
                            n,
                            f.iter()
                                .map(|v| v.to_string())
                                .collect::<Vec<_>>()
                                .join(",")
                        )
                    }
                })
                .collect::<Vec<_>>()
                .join(";")
        };
        out.push_str(&format!(
            "st={} vars={} in={} out={}\n",
            s.state, vars, in_s, out_s
        ));
    }
    out
}

// ---------------------------------------------------------------------------
// Independent expression evaluator (SEMANTICS 2.2: wrapping bit-vectors).

#[derive(Debug, Clone)]
enum Val {
    Bool(bool),
    Bit {
        width: u32,
        value: u64,
    },
    /// Enum value (enum name + variant index).
    Enum {
        enum_name: String,
        variant: usize,
    },
    /// Bare identifier not resolved against a variable (an enum-variant
    /// literal in a comparison).
    Ident(String),
}

struct Ctx<'a> {
    module: &'a rfs::Module,
    samples: &'a [PslSample],
    flat_inputs: Vec<prod_driver::FlatVariant>,
    flat_outputs: Vec<prod_driver::FlatVariant>,
}

impl<'a> Ctx<'a> {
    fn new(module: &'a rfs::Module, samples: &'a [PslSample]) -> Self {
        Ctx {
            module,
            samples,
            flat_inputs: prod_driver::flattened_inputs(module),
            flat_outputs: prod_driver::flattened_outputs(module),
        }
    }

    fn n(&self) -> usize {
        self.samples.len()
    }

    fn mask(w: u32) -> u64 {
        if w >= 64 {
            u64::MAX
        } else {
            (1u64 << w) - 1
        }
    }

    fn var_index(&self, name: &str) -> Option<usize> {
        self.module.vars.iter().position(|v| v.name == name)
    }

    fn enum_variants(&self, name: &str) -> Option<&[String]> {
        self.module
            .enums
            .iter()
            .find(|e| e.name == name)
            .map(|e| e.variants.as_slice())
    }

    fn state_index(&self, name: &str) -> usize {
        prod_driver::state_variants(self.module)
            .iter()
            .position(|s| s == name)
            .unwrap_or_else(|| panic!("unknown state {name}"))
    }
}

fn eval_expr(ctx: &Ctx, e: &rfs::Expr, s: &PslSample) -> Val {
    use rfs::Expr::*;
    match e {
        Int { value } => Val::Bit {
            width: 8,
            value: *value,
        },
        Bool { value } => Val::Bool(*value),
        Path { path } => {
            let name = &path[0];
            if name == "state" {
                Val::Enum {
                    enum_name: "State".into(),
                    variant: ctx.state_index(&s.state),
                }
            } else if let Some(i) = ctx.var_index(name) {
                match &ctx.module.vars[i].ty {
                    rfs::Type::Bool => Val::Bool(s.vars[i] != 0),
                    rfs::Type::Bit { width } => Val::Bit {
                        width: *width,
                        value: s.vars[i] & Ctx::mask(*width),
                    },
                    rfs::Type::Enum { name: en } => Val::Enum {
                        enum_name: en.clone(),
                        variant: s.vars[i] as usize,
                    },
                }
            } else {
                Val::Ident(name.clone())
            }
        }
        Unary { op, expr } => {
            let v = eval_expr(ctx, expr, s);
            match (*op, &v) {
                (rfs::UnaryOp::Not, Val::Bool(b)) => Val::Bool(!b),
                (rfs::UnaryOp::BitNot, Val::Bit { width, value }) => Val::Bit {
                    width: *width,
                    value: (!value) & Ctx::mask(*width),
                },
                (rfs::UnaryOp::Neg, Val::Bit { width, value }) => Val::Bit {
                    width: *width,
                    value: (Ctx::mask(*width).wrapping_add(1).wrapping_sub(*value))
                        & Ctx::mask(*width),
                },
                _ => panic!("unary op on non-matching value: {v:?}"),
            }
        }
        Binary { op, left, right } => {
            let l = eval_expr(ctx, left, s);
            let r = eval_expr(ctx, right, s);
            binop(ctx, *op, l, r)
        }
    }
}

fn bits_of(v: Val) -> (u32, u64) {
    match v {
        Val::Bit { width, value } => (width, value),
        other => panic!("bitvector op on non-bitvector: {other:?}"),
    }
}

fn binop(ctx: &Ctx, op: rfs::BinOp, l: Val, r: Val) -> Val {
    use rfs::BinOp::*;
    match op {
        LogicAnd | LogicOr => {
            let (a, b) = match (l, r) {
                (Val::Bool(a), Val::Bool(b)) => (a, b),
                (x, y) => panic!("logical op on non-bool: {x:?} vs {y:?}"),
            };
            Val::Bool(match op {
                LogicAnd => a && b,
                _ => a || b,
            })
        }
        Eq | NotEq | Lt | Le | Gt | Ge => Val::Bool(cmp(ctx, op, l, r)),
        _ => arith_or_bit(op, l, r),
    }
}

fn cmp(ctx: &Ctx, op: rfs::BinOp, l: Val, r: Val) -> bool {
    use rfs::BinOp::*;
    match op {
        Eq => eq_of(ctx, l, r),
        NotEq => !eq_of(ctx, l, r),
        Lt | Le | Gt | Ge => {
            let lt = lt_of(&l, &r);
            let eq = eq_of(ctx, l, r);
            match op {
                Lt => lt,
                Le => eq || lt,
                Gt => !(eq || lt),
                Ge => !lt,
                _ => unreachable!(),
            }
        }
        _ => unreachable!("cmp called with non-comparison op"),
    }
}

fn eq_of(ctx: &Ctx, l: Val, r: Val) -> bool {
    match (&l, &r) {
        (Val::Bool(a), Val::Bool(b)) => a == b,
        (
            Val::Bit {
                width: w1,
                value: v1,
            },
            Val::Bit {
                width: w2,
                value: v2,
            },
        ) => {
            let m = Ctx::mask((*w1).max(*w2));
            v1 & m == v2 & m
        }
        (Val::Enum { enum_name, variant }, Val::Ident(name))
        | (Val::Ident(name), Val::Enum { enum_name, variant }) => {
            let variants = ctx
                .enum_variants(enum_name)
                .unwrap_or_else(|| panic!("unknown enum {enum_name}"));
            let idx = variants
                .iter()
                .position(|x| x == name)
                .unwrap_or_else(|| panic!("variant {name} not in enum {enum_name}"));
            idx == *variant
        }
        (
            Val::Enum {
                enum_name: e1,
                variant: v1,
            },
            Val::Enum {
                enum_name: e2,
                variant: v2,
            },
        ) => e1 == e2 && v1 == v2,
        (x, y) => panic!("unsupported comparison operands: {x:?} vs {y:?}"),
    }
}

fn lt_of(l: &Val, r: &Val) -> bool {
    match (l, r) {
        (
            Val::Bit {
                width: w1,
                value: v1,
            },
            Val::Bit {
                width: w2,
                value: v2,
            },
        ) => {
            let m = Ctx::mask((*w1).max(*w2));
            (v1 & m) < (v2 & m)
        }
        (x, y) => panic!("ordering comparison on non-bitvector: {x:?} vs {y:?}"),
    }
}

fn arith_or_bit(op: rfs::BinOp, l: Val, r: Val) -> Val {
    use rfs::BinOp::*;
    let (w1, a) = bits_of(l);
    let (w2, b) = bits_of(r);
    match op {
        ShiftLeft | ShiftRight => {
            // The LEFT operand's width governs; amount >= width -> 0.
            if b >= w1 as u64 {
                Val::Bit {
                    width: w1,
                    value: 0,
                }
            } else if op == ShiftLeft {
                Val::Bit {
                    width: w1,
                    value: (a << b) & Ctx::mask(w1),
                }
            } else {
                Val::Bit {
                    width: w1,
                    value: a >> b,
                }
            }
        }
        _ => {
            let w = w1.max(w2);
            let m = Ctx::mask(w);
            let v = match op {
                Plus => (a & m).wrapping_add(b & m),
                Minus => (a & m).wrapping_sub(b & m),
                Star => {
                    let m128 = if w >= 64 { u128::MAX } else { (1u128 << w) - 1 };
                    ((a as u128 * b as u128) & m128) as u64
                }
                BitAnd => (a & m) & (b & m),
                BitOr => (a | b) & m,
                BitXor => (a ^ b) & m,
                _ => unreachable!("arith_or_bit called with non-arithmetic op"),
            };
            Val::Bit {
                width: w,
                value: v & m,
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Independent recursive window evaluator (SEMANTICS 5.1-5.5).

/// Temporal-operator-free formula (a per-sample signal).
fn is_signal(f: &rfs::Formula) -> bool {
    use rfs::Formula::*;
    match f {
        Expr { .. } | InputIs { .. } | OutputHas { .. } => true,
        Not { inner } => is_signal(inner),
        And { left, right } | Or { left, right } | Implies { left, right } => {
            is_signal(left) && is_signal(right)
        }
        Next { .. }
        | Always { .. }
        | BoundedAlways { .. }
        | BoundedEventually { .. }
        | Eventually { .. }
        | Until { .. } => false,
    }
}

fn sig_formula(ctx: &Ctx, f: &rfs::Formula, s: &PslSample) -> bool {
    use rfs::Formula::*;
    match f {
        Expr { expr } => matches!(eval_expr(ctx, expr, s), Val::Bool(true)),
        InputIs { pattern } => pattern_matches_input(ctx, pattern, s),
        OutputHas { pattern } => s.outputs.iter().any(|(n, f)| {
            match_pattern(
                ctx,
                &ctx.flat_outputs,
                &pattern.group,
                &pattern.variant,
                n,
                f,
            )
        }),
        Not { inner } => !sig_formula(ctx, inner, s),
        And { left, right } => sig_formula(ctx, left, s) && sig_formula(ctx, right, s),
        Or { left, right } => sig_formula(ctx, left, s) || sig_formula(ctx, right, s),
        Implies { left, right } => !sig_formula(ctx, left, s) || sig_formula(ctx, right, s),
        _ => panic!("temporal operator in signal position: {f:?}"),
    }
}

fn pattern_matches_input(ctx: &Ctx, pat: &rfs::EventPattern, s: &PslSample) -> bool {
    let Some((name, fields)) = &s.input else {
        return false;
    };
    match_pattern(
        ctx,
        &ctx.flat_inputs,
        &pat.group,
        &pat.variant,
        name,
        fields,
    )
}

fn match_pattern(
    ctx: &Ctx,
    flat: &[prod_driver::FlatVariant],
    group: &str,
    variant: &Option<rfs::PatternVariant>,
    name: &str,
    fields: &[u64],
) -> bool {
    let Some(fv) = flat.iter().find(|v| v.name == name && v.group == group) else {
        return false;
    };
    if let Some(pv) = variant {
        let pvn = match pv {
            rfs::PatternVariant::Unit { name } => name.as_str(),
            rfs::PatternVariant::Struct { name, .. } => name.as_str(),
        };
        if fv.variant_name != pvn {
            return false;
        }
        if let rfs::PatternVariant::Struct {
            name: _,
            fields: pf,
        } = pv
        {
            for (i, f) in pf.iter().enumerate() {
                match &f.pattern {
                    rfs::PatternExpr::Wildcard | rfs::PatternExpr::Bind { .. } => {}
                    rfs::PatternExpr::Int { value } => {
                        if fields.get(i).copied() != Some(*value) {
                            return false;
                        }
                    }
                    rfs::PatternExpr::Bool { value } => {
                        if fields.get(i).map(|x| *x != 0) != Some(*value) {
                            return false;
                        }
                    }
                    rfs::PatternExpr::EnumVariant { path } => {
                        let vn = &path[path.len() - 1];
                        let (Some((_fname, field_ty)), Some(fval)) =
                            (fv.fields.get(i), fields.get(i))
                        else {
                            return false;
                        };
                        match field_ty {
                            rfs::Type::Enum { name: en } => {
                                let variants = ctx
                                    .enum_variants(en)
                                    .unwrap_or_else(|| panic!("unknown enum {en}"));
                                let idx = variants
                                    .iter()
                                    .position(|x| x == vn)
                                    .unwrap_or_else(|| panic!("variant {vn} not in enum {en}"));
                                if *fval != idx as u64 {
                                    return false;
                                }
                            }
                            other => panic!("enum pattern on non-enum field: {other:?}"),
                        }
                    }
                }
            }
        }
    }
    true
}

/// First confirmation (sample index, trigger sample) within window [i..n) for
/// a TOP-LEVEL monitorable formula, or None if the monitor never confirms on
/// this trace. Trigger is -1 for per-sample-check confirmations.
fn confirm(ctx: &Ctx, f: &rfs::Formula, i: usize) -> Option<(usize, i64)> {
    use rfs::Formula::*;
    match f {
        Always { inner } => scan_anchors(ctx, i, ctx.n(), inner),
        BoundedAlways { bound, inner } => {
            let end = (i + *bound as usize).min(ctx.n());
            scan_anchors(ctx, i, end, inner)
        }
        BoundedEventually { bound, inner } => one_shot_obligation(ctx, i, *bound, inner),
        _ => panic!("top-level formula is not a monitorable obligation shape: {f:?}"),
    }
}

fn scan_anchors(ctx: &Ctx, i: usize, end: usize, body: &rfs::Formula) -> Option<(usize, i64)> {
    for j in i..end {
        if let Some(c) = confirm_body_at_anchor(ctx, body, j) {
            return Some(c);
        }
    }
    None
}

/// Confirm within window [anchor..n) for a body anchored at `anchor`.
fn confirm_body_at_anchor(ctx: &Ctx, body: &rfs::Formula, anchor: usize) -> Option<(usize, i64)> {
    use rfs::Formula::*;
    if is_signal(body) {
        (anchor..ctx.n())
            .find(|&j| !sig_formula(ctx, body, &ctx.samples[j]))
            .map(|j| (j, -1))
    } else {
        match body {
            Implies { left, right } => confirm_implied(ctx, left, right, anchor),
            BoundedEventually { bound, inner } => one_shot_obligation(ctx, anchor, *bound, inner),
            _ => panic!("unsupported temporal body shape: {body:?}"),
        }
    }
}

fn confirm_implied(
    ctx: &Ctx,
    left: &rfs::Formula,
    right: &rfs::Formula,
    anchor: usize,
) -> Option<(usize, i64)> {
    use rfs::Formula::*;
    match right {
        Next { inner } => {
            // For every check sample s >= max(anchor+1, 1): sig(left, s-1) ->
            // sig(inner, s) (one-sample delay; the register starts false).
            let start = anchor.saturating_add(1).max(1);
            (start..ctx.n())
                .find(|&s| {
                    sig_formula(ctx, left, &ctx.samples[s - 1])
                        && !sig_formula(ctx, inner, &ctx.samples[s])
                })
                .map(|s| (s, -1))
        }
        BoundedEventually { bound, inner } => scan_obligation(ctx, left, inner, *bound, anchor),
        _ => panic!("unsupported implication consequent: {right:?}"),
    }
}

/// Reactive obligation `trigger -> bounded_eventually[bound] body` anchored at
/// `anchor` (SEMANTICS 5.5): for every trigger sample j the body must hold
/// within [j, min(j+bound-1, n-1)]; re-triggers while pending are ignored and
/// a fresh countdown starts after the first clear; a still-pending obligation
/// at end-of-trace never confirms (its deadline sample does not exist).
fn scan_obligation(
    ctx: &Ctx,
    trigger: &rfs::Formula,
    body: &rfs::Formula,
    bound: u64,
    anchor: usize,
) -> Option<(usize, i64)> {
    let n = ctx.n();
    let mut j = anchor;
    while j < n {
        if sig_formula(ctx, trigger, &ctx.samples[j]) {
            let d = if bound == 0 {
                j
            } else {
                (j + bound as usize - 1).min(n - 1)
            };
            match (j..=d).find(|&k| sig_formula(ctx, body, &ctx.samples[k])) {
                Some(k) => j = k + 1,
                None => return Some((d, j as i64)),
            }
        } else {
            j += 1;
        }
    }
    None
}

/// One-shot obligation (top-level `bounded_eventually`, or a constant-true
/// trigger): pending from `anchor`; deadline min(anchor+bound-1, n-1)
/// (bound == 0: the anchor sample itself).
fn one_shot_obligation(
    ctx: &Ctx,
    anchor: usize,
    bound: u64,
    body: &rfs::Formula,
) -> Option<(usize, i64)> {
    let n = ctx.n();
    if anchor >= n {
        return None;
    }
    let d = if bound == 0 {
        anchor
    } else {
        (anchor + bound as usize - 1).min(n - 1)
    };
    match (anchor..=d).find(|&k| sig_formula(ctx, body, &ctx.samples[k])) {
        Some(_) => None,
        None => Some((d, anchor as i64)),
    }
}

// ---------------------------------------------------------------------------
// Generated-side extraction + the external reference-tool hook.

/// The `PROPERTY_LABELS` const of a generated Rust artifact - exactly the
/// properties codegen emitted monitors for.
fn extract_property_labels(rust_code: &str) -> Vec<String> {
    const PREFIX: &str = "pub const PROPERTY_LABELS: &[&str] = &[";
    let start = rust_code
        .find(PREFIX)
        .expect("PROPERTY_LABELS const in artifact")
        + PREFIX.len();
    let rest = &rust_code[start..];
    let close = rest.find(']').unwrap();
    rest[..close]
        .split(',')
        .map(|s| s.trim().trim_matches('"').to_string())
        .filter(|s| !s.is_empty())
        .collect()
}

/// Per-property generated verdicts from the driver's M-lines: each property
/// confirms at most once (sticky), so at most one M-line names it.
fn extract_generated_verdicts(
    out: &str,
    labels: &[String],
) -> std::collections::HashMap<String, Option<(usize, i64)>> {
    let mut map: std::collections::HashMap<String, Option<(usize, i64)>> =
        labels.iter().map(|l| (l.clone(), None)).collect();
    for line in out.lines() {
        let Some(rest) = line.strip_prefix('M') else {
            continue;
        };
        if !rest.starts_with(|c: char| c.is_ascii_digit()) || !rest.contains(" FAIL ") {
            continue;
        }
        let parts: Vec<&str> = rest.split_whitespace().collect();
        // ["M{k}", "FAIL", label, "step={k}", "trig={t}"]
        let (label, step_s, trig_s) = (parts[2], parts.get(3).copied(), parts.get(4).copied());
        let Some(step) = step_s
            .and_then(|s| s.strip_prefix("step="))
            .and_then(|s| s.parse::<usize>().ok())
        else {
            continue;
        };
        let trig = trig_s
            .and_then(|s| s.strip_prefix("trig="))
            .and_then(|s| s.parse::<i64>().ok())
            .unwrap_or(-1);
        if map.contains_key(label) {
            map.insert(label.to_string(), Some((step, trig)));
        }
    }
    map
}

/// The `RFS_PSL_REF_TOOL` hook: None unless the variable names an existing file.
fn ref_tool() -> Option<PathBuf> {
    let p = std::env::var("RFS_PSL_REF_TOOL").ok()?;
    if p.is_empty() {
        return None;
    }
    let pb = PathBuf::from(&p);
    pb.is_file().then_some(pb)
}

/// Run the external reference tool over one trace; returns its per-label
/// verdicts (label -> Option<(step, trig)>).
fn run_ref_tool(
    tool: &Path,
    spec: &Path,
    trace_text: &str,
    dir: &Path,
) -> Vec<(String, Option<(usize, i64)>)> {
    let trace_file = dir.join("ref_trace.txt");
    std::fs::write(&trace_file, trace_text).expect("write ref trace file");
    let out = Command::new(tool)
        .arg(spec)
        .arg(&trace_file)
        .output()
        .unwrap_or_else(|e| panic!("run RFS_PSL_REF_TOOL {tool:?}: {e}"));
    assert!(
        out.status.success(),
        "RFS_PSL_REF_TOOL exited {:?}:\n{}",
        out.status.code(),
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    stdout
        .lines()
        .filter_map(|l| {
            let rest = l.strip_prefix("PROP ")?;
            let (label, verdict) = rest.split_once(' ')?;
            let v = if verdict == "none" {
                None
            } else {
                let mut it = verdict.split_whitespace();
                let k = it.next()?.strip_prefix("step=")?.parse::<usize>().ok()?;
                let t = it.next()?.strip_prefix("trig=")?.parse::<i64>().ok()?;
                Some((k, t))
            };
            Some((label.to_string(), v))
        })
        .collect()
}

// ---------------------------------------------------------------------------
// Test.

#[test]
fn psl_translation_matches_generated_monitors_all_property_examples() {
    let files = rfs::collect_rfs_files(root().join("examples/valid")).expect("collect .rfs files");
    assert!(!files.is_empty(), "no valid examples found");

    let tool = ref_tool();
    let mut compared_props = 0usize;
    let mut confirmed_props = 0usize;
    let mut examples_with_monitors = 0usize;
    let mut skipped_all_liveness = 0usize;

    for spec in &files {
        let src = std::fs::read_to_string(spec).unwrap_or_else(|e| panic!("read {:?}: {e}", spec));
        let ir = rfs::parse_source(&src).unwrap_or_else(|e| panic!("{:?} must parse: {e}", spec));
        let module = &ir.modules[0];
        if module.properties.is_empty() {
            continue;
        }

        // Scratch tag (rustc crate-name safe).
        let tag: String = spec
            .strip_prefix(root())
            .expect("spec under root")
            .to_string_lossy()
            .chars()
            .map(|c| if c.is_ascii_alphanumeric() { c } else { '_' })
            .collect();

        // 1) Which properties did codegen actually monitor?
        let art = prod_driver::production_artifacts(&format!("psl_{tag}"), module, &scratch());
        let labels = extract_property_labels(&art.rust_code);
        if labels.is_empty() {
            skipped_all_liveness += 1;
            continue; // pure-liveness spec: no monitor to compare against
        }
        examples_with_monitors += 1;

        // 2) Drive the real FSM with the seeded script; parse trace + verdicts.
        let (rust_bin, _c_bin) =
            prod_driver::build_drivers(&format!("psl_{tag}"), module, &scratch());
        let script = prod_driver::input_script(module, TRACE_SEED, TRACE_STEPS);
        let (ec, out) = prod_driver::run_driver(&rust_bin, &script);
        assert_eq!(ec, 0, "[{}] driver exited {ec}\n{out}", tag);
        let samples = parse_trace(&out);
        // Parser-consistency invariant: the parsed sample count must equal
        // exactly what the driver recorded (init + every S line that carries
        // st=/vars= - committed, ASSUME-stopped and PROP-stopped alike).
        // Protocol specs driven by random input may commit very few steps
        // (most inputs are UNEXPECTED there), so no arbitrary length floor.
        let expected = 1 + out
            .lines()
            .filter(|l| {
                l.starts_with('S')
                    && l[1..].starts_with(|c: char| c.is_ascii_digit())
                    && l.contains(" st=")
                    && l.contains(" vars=")
            })
            .count();
        assert_eq!(
            samples.len(),
            expected,
            "[{}] parser captured {} samples but the driver recorded {expected}",
            tag,
            samples.len()
        );
        let gen = extract_generated_verdicts(&out, &labels);

        // 3) Independent evaluator per monitored property.
        let ctx = Ctx::new(module, &samples);
        for p in &module.properties {
            if !labels.contains(&p.name) {
                continue; // skipped by codegen (outside the monitor subset)
            }
            let mine = confirm(&ctx, &p.formula, 0);
            if mine.is_some() {
                confirmed_props += 1;
            }
            assert_eq!(
                mine, gen[&p.name],
                "[{}/{}] evaluator diverges from generated monitor\n[eval] {:?}\n[gen] {:?}",
                tag, p.name, mine, gen[&p.name]
            );
            compared_props += 1;
        }

        // 4) External reference tool (availability-gated).
        if let Some(tool) = &tool {
            let dir = scratch().join(&tag);
            std::fs::create_dir_all(&dir).expect("create hook dir");
            let verdicts = run_ref_tool(tool, spec, &render_canonical(&samples), &dir);
            for (label, v) in verdicts {
                let expected = confirm(
                    &ctx,
                    &module
                        .properties
                        .iter()
                        .find(|p| p.name == label)
                        .expect("hook named an unknown property")
                        .formula,
                    0,
                );
                assert_eq!(
                    v, expected,
                    "[{tag}] RFS_PSL_REF_TOOL disagrees with the independent evaluator on {label}:\n[tool] {v:?}\n[eval] {expected:?}"
                );
            }
        }
    }

    // Non-degeneracy: the enumeration must have covered a real slice of the
    // property-bearing corpus, and at least one confirmation must have fired
    // on live data so the violation path is exercised (pinned seed => stable).
    assert!(
        confirmed_props >= 1,
        "no property confirmed on any random trace - the violation path is untested"
    );
    assert!(
        examples_with_monitors >= 8,
        "only {examples_with_monitors} monitored examples found - enumeration degenerate?"
    );
    assert!(
        compared_props >= 20,
        "only {compared_props} properties compared - enumeration degenerate? (skipped all-liveness: {skipped_all_liveness})"
    );
}
