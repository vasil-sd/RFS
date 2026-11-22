//! In-process SMT-bounded executor for `rfs mc-ref --mode smt-bounded`
//! (Phase 16 task 4, D-P16-3): the THIRD consumer of the shared `SmtSpec`
//! encoding model - C emission (`smt_emit`), Rust emission (`smt_emit`), and
//! this in-process z3 execution. It shares the MODEL (`mc::plan` +
//! `mc::smt_plan`, the documented D-P16-3 exception), not the emission: own
//! Se->z3 conversion, own unrolling engine, own query construction - mirroring
//! the pinned conventions of SMT_ENGINE_RUST / SMT_ENGINE_C step for step
//! (fixed step-variable names/order, fixed constraint order, fixed query
//! order, identical model reading). An emitter bug and an executor bug stay
//! distinguishable; a model bug is caught by all three agreeing.
//!
//! Parity-critical conventions (mirrored from the generated engines):
//! - step variables: `s_{i}` (state, state_bits), `v_{i}_{j}` (per var,
//!   var width), `ev_{i}` (event class, only when N_CLASSES > 1),
//!   `fld_{i}_{g}` (class fields, global order);
//! - base constraints in fixed order: initial config at step 0 (state eq then
//!   per-var eq), then per step i < k-1: enabled_t = state_eq AND class_eq AND
//!   guard (firing order), fires_t = enabled_t AND NOT(any earlier enabled),
//!   any_fire, next-state ITE chain (last transition first), per-var ITE
//!   chains (unchanged when unassigned); alive_i = alive_{i-1} AND any_fire_i;
//! - queries in fixed order: safety properties in declaration order (viol =
//!   OR over i<k of alive_i AND NOT phi_i; sat -> FAIL at the first i where
//!   alive and not phi, read from the model), then liveness properties in
//!   declaration order (never = AND over i of NOT(alive_i AND phi_i); cyc =
//!   OR over a<b of config_a == config_b on state+vars; sat -> FAIL at pair.0
//!   with cycle length b-a, pair = first matching a<b from the model);
//! - exit codes: 1 fail, 2 unknown, 0 pass-within-bound.
//!
//! Docs: dev:mc-engine-overview, dev:mc-engine-smt, llm:mc-engine

use z3::{ast::Bool, ast::BV, with_z3_config, Config, Model, SatResult, Solver};

use crate::codegen::mc_concepts::{smt_summary_line, FailReason, SmtStatus};
use crate::codegen::AssumptionSelection;
use crate::lang::ir::Module;
use crate::mc::plan::McPlan;
use crate::mc::smt_plan::{
    build_smt_spec, ceil_log2_pub, se_type, Sb, Se, SeType, Sf, SmtPropKind, SmtSpec,
};

/// The outcome of one in-process SMT-bounded run: the exit code mirroring the
/// generated SMT binaries (0 pass-within-bound / 1 fail / 2 unknown) and the
/// full stdout (human lines + witness trace on fail + the summary line), each
/// line newline-terminated exactly as the generated engines print it.
#[derive(Debug)]
pub struct SmtOutcome {
    pub exit_code: i32,
    pub output: String,
    /// Stderr diagnostics (the per-property omission warnings when FULL
    /// properties are skipped without --full-properties).
    pub warnings: Vec<String>,
}

/// The z3 version this in-process executor is linked against (re-exported so
/// the parity suite can name both solver sides in its assertions - the z3
/// crate is an optional dependency, not reachable from integration tests).
pub fn z3_version() -> String {
    z3::full_version().to_string()
}

/// One step's z3 variables (the conversion context): the state BV, the per-var
/// BVs, and the per-global-field BVs at that step.
struct StepVars<'a> {
    s: &'a BV,
    v: &'a [BV],
    fld: &'a [BV],
}

/// Evaluation context for property formulas: the step variables PLUS the
/// per-step atom storage (k x N_ATOMS rows of precomputed input_is/output_has
/// expressions; the final row is all-false - no input/output is consumed at
/// the last unrolled step). Explicit storage, not name-shared constants:
/// z3_mk_fresh_const does not guarantee name-sharing across solver versions.
struct SfCtx<'a> {
    s_vars: &'a [BV],
    v_vars: &'a [Vec<BV>],
    fld_vars: &'a [Vec<BV>],
    ii_steps: &'a [Vec<Bool>],
    oh_steps: &'a [Vec<Bool>],
    /// Per-step path liveness (alive[i] = a transition fired at every step
    /// 0..i). Threaded so `Sf::Next` can be gated WEAK at trace death (D1 fix,
    /// SEMANTICS 8.3): a next-obligation whose successor config is frozen by an
    /// unhandled input is vacuously satisfied.
    alive: &'a [Bool],
    k: usize,
}

/// A converted expression node: a z3 Bool or a z3 bitvector (the emitter's
/// `bool_as_bv` context decides which at bool-typed leaves).
enum SeAst {
    B(Bool),
    V(BV),
}

fn b_and(a: &Bool, b: &Bool) -> Bool {
    Bool::and(&[a, b])
}

fn b_or(a: &Bool, b: &Bool) -> Bool {
    Bool::or(&[a, b])
}

fn read_bv(m: &Model, e: &BV) -> u64 {
    m.eval(e, true)
        .expect("model evaluation")
        .as_u64()
        .expect("numeral value")
}

fn read_bool(m: &Model, e: &Bool) -> bool {
    m.eval(e, true)
        .expect("model evaluation")
        .as_bool()
        .expect("bool value")
}

/// Global field index of a class-relative field (mirrors smt_emit's
/// `global_field_index` - the executor keeps its own copy per D-P16-3).
fn global_field_index(spec: &SmtSpec, class: usize, local: usize) -> usize {
    let mut g = 0;
    for (c, cls) in spec.classes.iter().enumerate() {
        if c == class {
            return g + local;
        }
        g += cls.fields.len();
    }
    unreachable!("class index out of range")
}

/// Comparison operand widths (mirrors smt_emit's `cmp_widths_full` - the
/// validator lifting + exact-MC full-value semantics): integer literals lift
/// to the sibling's width; if a literal does not fit, BOTH sides widen to
/// hold it and the non-literal side is zero-extended. Returns per-side (render
/// width, extra zero bits); both sides end at the same final width.
fn cmp_widths_full(l: &Se, r: &Se, spec: &SmtSpec) -> (u32, u32, u32, u32) {
    let lt = se_type(l, spec);
    let rt = se_type(r, spec);
    let nw_l = if matches!(lt, SeType::IntLit) {
        8
    } else {
        lt.width()
    };
    let nw_r = if matches!(rt, SeType::IntLit) {
        8
    } else {
        rt.width()
    };
    let mut w = nw_l.max(nw_r);
    for side in [l, r] {
        if let Se::Lit {
            value,
            is_bool: false,
        } = side
        {
            let bits = if *value == 0 {
                1
            } else {
                64 - value.leading_zeros()
            };
            w = w.max(bits);
        }
    }
    let (wl, el) = if matches!(lt, SeType::IntLit) {
        (w, 0)
    } else {
        (nw_l, w - nw_l)
    };
    let (wr, er) = if matches!(rt, SeType::IntLit) {
        (w, 0)
    } else {
        (nw_r, w - nw_r)
    };
    (wl, wr, el, er)
}

/// Shift-amount width (mirrors smt_emit's `shift_amt_width`): a bitvector
/// amount keeps its width; an integer literal lifts to at least the left
/// operand's width.
fn shift_amt_width(r: &Se, spec: &SmtSpec, wl: u32) -> u32 {
    match se_type(r, spec) {
        SeType::Bit(wr) => wr,
        SeType::IntLit => {
            let v = match r {
                Se::Lit { value, .. } => *value,
                _ => 0,
            };
            let bits = if v == 0 { 1 } else { 64 - v.leading_zeros() };
            wl.max(bits)
        }
        _ => 8,
    }
}

/// Zero-extend a bitvector by `ext` high zero bits (the emitter's widen).
fn widen(v: &BV, ext: u32) -> BV {
    if ext == 0 {
        v.clone()
    } else {
        BV::from_u64(0u64, ext).concat(v)
    }
}

/// Convert one shared expression to a z3 AST at step `i` (mirrors smt_emit's
/// `render_se_rust` rule for rule): `expected` is the parent context's bit
/// width (literal rendering), `class` resolves field indices, `bool_as_bv`
/// selects BV(1) over z3 Bool at bool-typed leaves.
fn se_to_z3(
    se: &Se,
    spec: &SmtSpec,
    sv: &StepVars<'_>,
    class: Option<usize>,
    expected: Option<u32>,
    bool_as_bv: bool,
) -> SeAst {
    match se {
        Se::Lit { value, is_bool } => {
            if *is_bool && !bool_as_bv {
                SeAst::B(Bool::from_bool(*value != 0))
            } else {
                let w = expected.unwrap_or(8);
                SeAst::V(BV::from_u64(*value, w))
            }
        }
        Se::State => SeAst::V(sv.s.clone()),
        Se::Var(j) => {
            let leaf = sv.v[*j].clone();
            if spec.vars[*j].is_bool && !bool_as_bv {
                SeAst::B(leaf.eq(&BV::from_u64(1u64, 1)))
            } else {
                SeAst::V(leaf)
            }
        }
        Se::Field { idx, .. } => {
            let g = global_field_index(spec, class.expect("field outside transition"), *idx);
            let leaf = sv.fld[g].clone();
            // Field bool-ness is captured at conversion time (class-relative).
            let is_bool = spec
                .classes
                .iter()
                .flat_map(|c| c.fields.iter())
                .nth(g)
                .map(|f| f.2)
                .unwrap_or(false);
            if is_bool && !bool_as_bv {
                SeAst::B(leaf.eq(&BV::from_u64(1u64, 1)))
            } else {
                SeAst::V(leaf)
            }
        }
        Se::Not(inner) => {
            let c = se_to_z3(inner, spec, sv, class, Some(1), false);
            let b = match c {
                SeAst::B(b) => b,
                SeAst::V(_) => unreachable!("Not of a non-bool expression"),
            };
            finish_bool(b, bool_as_bv)
        }
        Se::BitNot(inner) | Se::Neg(inner) => {
            let w = se_type(inner, spec).width();
            let c = se_to_z3(inner, spec, sv, class, Some(w), false);
            let v = match c {
                SeAst::V(v) => v,
                SeAst::B(_) => unreachable!("bit_not/neg of a bool expression"),
            };
            if matches!(se, Se::BitNot(_)) {
                SeAst::V(v.bvnot())
            } else {
                SeAst::V(v.bvneg())
            }
        }
        Se::Bin(op, l, r) => {
            let lt = se_type(l, spec);
            let rt = se_type(r, spec);
            match op {
                Sb::LogicAnd | Sb::LogicOr => {
                    let a = bool_of(se_to_z3(l, spec, sv, class, Some(1), false));
                    let b = bool_of(se_to_z3(r, spec, sv, class, Some(1), false));
                    SeAst::B(if *op == Sb::LogicAnd {
                        b_and(&a, &b)
                    } else {
                        b_or(&a, &b)
                    })
                }
                Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => {
                    let (wl, wr, ext_l, ext_r) = cmp_widths_full(l, r, spec);
                    let a0 = se_to_z3(l, spec, sv, class, Some(wl), false);
                    let b0 = se_to_z3(r, spec, sv, class, Some(wr), false);
                    if lt == SeType::Bool && rt == SeType::Bool {
                        // Bool ordering encodings (RFS allows comparisons on bools).
                        let a = bool_of(a0);
                        let b = bool_of(b0);
                        let out = match op {
                            Sb::Eq => a.eq(&b),
                            Sb::NotEq => a.eq(&b).not(),
                            Sb::Lt => b_and(&a.not(), &b),
                            Sb::Le => b_and(&a, &b.not()).not(),
                            Sb::Gt => b_and(&b.not(), &a),
                            Sb::Ge => b_and(&b, &a.not()).not(),
                            _ => unreachable!("non-comparison op in comparison arm"),
                        };
                        SeAst::B(out)
                    } else {
                        let a = widen(&bv_of(a0), ext_l);
                        let b = widen(&bv_of(b0), ext_r);
                        let out = match op {
                            Sb::Eq => a.eq(&b),
                            Sb::NotEq => a.ne(&b),
                            Sb::Lt => a.bvult(&b),
                            Sb::Le => a.bvule(&b),
                            Sb::Gt => a.bvugt(&b),
                            Sb::Ge => a.bvuge(&b),
                            _ => unreachable!("non-comparison op in comparison arm"),
                        };
                        SeAst::B(out)
                    }
                }
                Sb::Plus | Sb::Minus | Sb::Star | Sb::BitAnd | Sb::BitOr | Sb::BitXor => {
                    let w = match (lt, rt) {
                        (SeType::Bit(a), SeType::Bit(b)) => {
                            debug_assert_eq!(a, b);
                            a
                        }
                        (SeType::IntLit, t) | (t, SeType::IntLit) => t.width(),
                        _ => 8,
                    };
                    let a = bv_of(se_to_z3(l, spec, sv, class, Some(w), false));
                    let b = bv_of(se_to_z3(r, spec, sv, class, Some(w), false));
                    let out = match op {
                        Sb::Plus => a.bvadd(&b),
                        Sb::Minus => a.bvsub(&b),
                        Sb::Star => a.bvmul(&b),
                        Sb::BitAnd => a.bvand(&b),
                        Sb::BitOr => a.bvor(&b),
                        _ => a.bvxor(&b),
                    };
                    SeAst::V(out)
                }
                Sb::ShiftLeft | Sb::ShiftRight => {
                    let wl = se_type(l, spec).width();
                    let w_r = shift_amt_width(r, spec, wl);
                    let w_c = w_r.max(wl);
                    let a = bv_of(se_to_z3(l, spec, sv, class, None, false));
                    let b = bv_of(se_to_z3(r, spec, sv, class, Some(w_r), false));
                    let b_c = widen(&b, if w_c == w_r { 0 } else { w_c - w_r });
                    let glit = BV::from_u64(wl as u64, w_c);
                    let cond = b_c.bvult(&glit);
                    let amt = if w_c == wl {
                        b_c
                    } else {
                        b_c.extract(wl - 1, 0)
                    };
                    let shifted = if *op == Sb::ShiftLeft {
                        a.bvshl(&amt)
                    } else {
                        a.bvlshr(&amt)
                    };
                    SeAst::V(cond.ite(&shifted, &BV::from_u64(0u64, wl)))
                }
            }
        }
    }
}

fn bool_of(a: SeAst) -> Bool {
    match a {
        SeAst::B(b) => b,
        SeAst::V(_) => unreachable!("expected a bool-typed expression"),
    }
}

fn bv_of(a: SeAst) -> BV {
    match a {
        SeAst::V(v) => v,
        SeAst::B(_) => unreachable!("expected a bitvector-typed expression"),
    }
}

/// The emitter's `finish_bool`: render a z3 Bool as BV(1) when the parent
/// context is bitvector-typed.
fn finish_bool(b: Bool, bool_as_bv: bool) -> SeAst {
    if !bool_as_bv {
        SeAst::B(b)
    } else {
        SeAst::V(b.ite(&BV::from_u64(1u64, 1), &BV::from_u64(0u64, 1)))
    }
}

/// Convert one property formula to a z3 Bool at step `i` (mirrors smt_emit's
/// `render_sf_rust`: a bare Se renders as a bool at width 1, no fields).
/// Evaluate a property formula at step `step` of the k-step unrolling.
/// `next` shifts to step+1; input_is/output_has atoms reference the engine's
/// pre-defined named constants (false at the final step); the FULL-class
/// temporal nodes expand quantifier-free over the horizon (clipped at k -
/// sound: clipping can only drop anchors, never create a false FAIL).
fn sf_to_z3(sf: &Sf, spec: &SmtSpec, ctx: &SfCtx, step: usize) -> Bool {
    let sv = StepVars {
        s: &ctx.s_vars[step],
        v: &ctx.v_vars[step],
        fld: &ctx.fld_vars[step],
    };
    match sf {
        Sf::Se(inner) => bool_of(se_to_z3(inner, spec, &sv, None, Some(1), false)),
        Sf::Not(inner) => sf_to_z3(inner, spec, ctx, step).not(),
        Sf::And(l, r) => b_and(&sf_to_z3(l, spec, ctx, step), &sf_to_z3(r, spec, ctx, step)),
        Sf::Or(l, r) => b_or(&sf_to_z3(l, spec, ctx, step), &sf_to_z3(r, spec, ctx, step)),
        Sf::Implies(l, r) => sf_to_z3(l, spec, ctx, step).implies(&sf_to_z3(r, spec, ctx, step)),
        // Atoms: explicit per-step storage (false at the final unrolled step
        // - no input/output consumed there). NOT name-shared constants:
        // z3_mk_fresh_const does not guarantee name-sharing across versions.
        Sf::InputIs { id, .. } => ctx.ii_steps[step][*id].clone(),
        Sf::OutputHas { id, .. } => ctx.oh_steps[step][*id].clone(),
        // next: the body holds one step later - WEAK at trace death (D1 fix):
        // if the path died at step+1 (an unhandled input froze the config into a
        // self-loop), the obligation is vacuously satisfied, matching exact mode
        // SEMANTICS 8.3 (a terminated path has no successor to check). Without
        // this gate the body was evaluated against the frozen artifact -> false FAILs.
        Sf::Next(inner) => b_or(
            &ctx.alive[step + 1].not(),
            &sf_to_z3(inner, spec, ctx, step + 1),
        ),
        // FULL class: bounded_eventually[N] - f within N steps of the anchor.
        Sf::BoundedEventually { bound, inner } => {
            let mut d = Bool::from_bool(false);
            for j in 0..(*bound).min(ctx.k.saturating_sub(step)) {
                d = b_or(&d, &sf_to_z3(inner, spec, ctx, step + j));
            }
            d
        }
        // FULL class: weak until - (some j: right(j) with left before it) OR
        // (left forever - horizon-truncated, sound).
        Sf::Until { left, right } => {
            let mut right_holds = Bool::from_bool(false);
            for j in step..ctx.k {
                // Weak until: p is required only STRICTLY before the first q
                // (SEMANTICS 6.1 - a sample where both hold satisfies it).
                let mut prefix = Bool::from_bool(true);
                for i in step..j {
                    prefix = b_and(&prefix, &sf_to_z3(left, spec, ctx, i));
                }
                right_holds = b_or(
                    &right_holds,
                    &b_and(&sf_to_z3(right, spec, ctx, j), &prefix),
                );
            }
            let mut left_forever = Bool::from_bool(true);
            for i in step..ctx.k {
                left_forever = b_and(&left_forever, &sf_to_z3(left, spec, ctx, i));
            }
            b_or(&right_holds, &left_forever)
        }
        // FULL class: nested always[N] (unbounded nesting uses bound = MAX).
        Sf::BoundedAlways { bound, inner } => {
            let mut c = Bool::from_bool(true);
            for j in 0..(*bound).min(ctx.k.saturating_sub(step)) {
                c = b_and(&c, &sf_to_z3(inner, spec, ctx, step + j));
            }
            c
        }
        // FULL class: nested eventually - f at some later step.
        Sf::Eventually(inner) => {
            let mut d = Bool::from_bool(false);
            for j in step..ctx.k {
                d = b_or(&d, &sf_to_z3(inner, spec, ctx, j));
            }
            d
        }
    }
}

/// The input_is atom's defining expression at step i over the step's fire
/// predicates: OR over matching arms of (some transition of that class fired)
/// AND (the literal field constraints hold).
fn input_atom_expr(
    spec: &SmtSpec,
    arms: &[(usize, Vec<(usize, u64, bool)>)],
    fires: &[Bool],
    i: usize,
    fld_vars: &[Vec<BV>],
) -> Bool {
    let mut val = Bool::from_bool(false);
    for (ci, constraints) in arms {
        let mut cls_fire = Bool::from_bool(false);
        for (t, tr) in spec.transitions.iter().enumerate() {
            if tr.class == *ci {
                cls_fire = b_or(&cls_fire, &fires[t]);
            }
        }
        let mut arm = cls_fire;
        for (g, v, _isb) in constraints {
            let w = spec
                .classes
                .iter()
                .flat_map(|c| c.fields.iter())
                .nth(*g)
                .map(|f| f.1)
                .unwrap_or(1);
            arm = b_and(&arm, &fld_vars[i][*g].eq(&BV::from_u64(*v, w)));
        }
        val = b_or(&val, &arm);
    }
    val
}

/// The output_has atom's defining expression at step i: OR over transitions of
/// (it fired) AND (some emitted output matches the pattern - group/variant
/// statically, literal argument constraints evaluated at step i).
fn output_atom_expr(
    spec: &SmtSpec,
    def: &(String, Option<String>, Vec<(usize, u64, bool)>),
    fires: &[Bool],
    i: usize,
    s_vars: &[BV],
    v_vars: &[Vec<BV>],
    fld_vars: &[Vec<BV>],
) -> Bool {
    let (group, variant, constraints) = def;
    let sv = StepVars {
        s: &s_vars[i],
        v: &v_vars[i],
        fld: &fld_vars[i],
    };
    let mut val = Bool::from_bool(false);
    for (t, tr) in spec.transitions.iter().enumerate() {
        let mut emit_match = Bool::from_bool(false);
        for o in &tr.emits {
            if o.group != *group || o.variant.as_deref() != variant.as_deref() {
                continue;
            }
            let mut m = Bool::from_bool(true);
            for (pos, v, _isb) in constraints {
                match o.args.get(*pos) {
                    Some(arg) => {
                        let w = se_type(arg, spec).width();
                        let e = bv_of(se_to_z3(arg, spec, &sv, Some(tr.class), Some(w), true));
                        m = b_and(&m, &e.eq(&BV::from_u64(*v, w)));
                    }
                    None => {
                        m = Bool::from_bool(false);
                        break;
                    }
                }
            }
            emit_match = b_or(&emit_match, &m);
        }
        val = b_or(&val, &b_and(&fires[t], &emit_match));
    }
    val
}

/// Witness trace (pinned format - identical in both generated backends).
fn print_trace(
    out: &mut String,
    k: usize,
    m: &Model,
    spec: &SmtSpec,
    s_vars: &[BV],
    v_vars: &[Vec<BV>],
    ev_vars: Option<&[BV]>,
    fld_vars: &[Vec<BV>],
) {
    out.push_str(&format!("witness trace (solver model; bound k={k}):\n"));
    for i in 0..k {
        let si = read_bv(m, &s_vars[i]) as usize;
        let mut vs = String::new();
        for j in 0..spec.vars.len() {
            if j > 0 {
                vs.push_str(", ");
            }
            let val = read_bv(m, &v_vars[i][j]);
            if spec.vars[j].is_bool {
                vs.push_str(&format!(
                    "{}={}",
                    spec.vars[j].name,
                    if val != 0 { "true" } else { "false" }
                ));
            } else {
                vs.push_str(&format!("{}={}", spec.vars[j].name, val));
            }
        }
        let ev = if spec.classes.len() > 1 {
            let ci = read_bv(m, &ev_vars.expect("event vars")[i]) as usize;
            let mut fs = String::new();
            // Global field order; keep only this step's class fields.
            let mut g = 0;
            for (c, cls) in spec.classes.iter().enumerate() {
                for f in &cls.fields {
                    if c == ci {
                        if !fs.is_empty() {
                            fs.push_str(", ");
                        }
                        let val = read_bv(m, &fld_vars[i][g]);
                        if f.2 {
                            fs.push_str(&format!(
                                "{}={}",
                                f.0,
                                if val != 0 { "true" } else { "false" }
                            ));
                        } else {
                            fs.push_str(&format!("{}={}", f.0, val));
                        }
                    }
                    g += 1;
                }
            }
            format!("event={}({fs})", spec.classes[ci].name)
        } else {
            "event=-".to_string()
        };
        // The generated format is `vars={{{vs}}}` - no space before the
        // closing brace (byte-parity surface).
        out.push_str(&format!(
            "  step {i}: state={} vars={{{}}} {}\n",
            spec.states[si], vs, ev
        ));
    }
}

/// Run the in-process SMT-bounded check over the shared encoding model.
/// `bound` is the unrolling depth k (the generated binaries' `--bound`);
/// `full_properties` enables the FULL class (top-level bounded_eventually/
/// until, nested temporals) - without it those properties are omitted with a
/// per-property warning (mirrors the generated binaries' --full-properties).
pub fn run(module: &Module, bound: usize, full_properties: bool) -> Result<SmtOutcome, String> {
    // The model chain (D-P16-3): the same two calls both backends make.
    // Assumption selection is irrelevant - build_smt_spec rejects any spec
    // with assumptions (surfaced verbatim below), same as codegen.
    let plan = McPlan::compute(module, &AssumptionSelection::All);
    let spec = build_smt_spec(module, &plan)?;
    let k = bound.max(1);
    let cfg = Config::new();
    with_z3_config(&cfg, || run_unrolled(&spec, k, full_properties))
}

fn run_unrolled(spec: &SmtSpec, k: usize, full_properties: bool) -> Result<SmtOutcome, String> {
    let n_classes = spec.classes.len();
    let cls_bits = ceil_log2_pub(n_classes);
    // ---- Step variables (fixed names/order - parity-critical) ----
    let s_vars: Vec<BV> = (0..k)
        .map(|i| BV::new_const(format!("s_{i}").as_str(), spec.state_bits))
        .collect();
    let v_vars: Vec<Vec<BV>> = (0..k)
        .map(|i| {
            spec.vars
                .iter()
                .enumerate()
                .map(|(j, v)| BV::new_const(format!("v_{i}_{j}").as_str(), v.width))
                .collect()
        })
        .collect();
    let ev_vars: Option<Vec<BV>> = if n_classes > 1 {
        Some(
            (0..k)
                .map(|i| BV::new_const(format!("ev_{i}").as_str(), cls_bits))
                .collect(),
        )
    } else {
        None
    };
    let field_widths: Vec<u32> = spec
        .classes
        .iter()
        .flat_map(|c| c.fields.iter().map(|f| f.1))
        .collect();
    let fld_vars: Vec<Vec<BV>> = (0..k)
        .map(|i| {
            field_widths
                .iter()
                .enumerate()
                .map(|(g, &w)| BV::new_const(format!("fld_{i}_{g}").as_str(), w))
                .collect()
        })
        .collect();
    let step_vars = |i: usize| StepVars {
        s: &s_vars[i],
        v: &v_vars[i],
        fld: &fld_vars[i],
    };

    // ---- Base constraints: initial configuration + k-step unrolling ----
    let mut base: Vec<Bool> =
        vec![s_vars[0].eq(&BV::from_u64(spec.initial_state as u64, spec.state_bits))];
    for j in 0..spec.vars.len() {
        base.push(v_vars[0][j].eq(&BV::from_u64(spec.vars[j].init, spec.vars[j].width)));
    }
    let mut alive: Vec<Bool> = vec![Bool::from_bool(true)];
    // Per-step atom storage (see SfCtx): sample s's atoms describe the
    // transition that PRODUCED config_s (SEMANTICS 5.1 - "the transition just
    // taken"), so row 0 is all-false (no input/output at the initial sample)
    // and row i+1 is built from step i's fires in the loop below.
    let mut ii_steps: Vec<Vec<Bool>> = vec![vec![Bool::from_bool(false); spec.input_atoms.len()]];
    let mut oh_steps: Vec<Vec<Bool>> = vec![vec![Bool::from_bool(false); spec.output_atoms.len()]];
    for i in 0..k - 1 {
        // enabled_t = state match AND class match AND guard (firing order).
        let mut enableds: Vec<Bool> = Vec::new();
        for t in 0..spec.transitions.len() {
            let tr = &spec.transitions[t];
            let mut en = s_vars[i].eq(&BV::from_u64(tr.state as u64, spec.state_bits));
            if n_classes > 1 {
                en = b_and(
                    &en,
                    &ev_vars.as_ref().unwrap()[i].eq(&BV::from_u64(tr.class as u64, cls_bits)),
                );
            }
            if let Some(guard) = &tr.guard {
                en = b_and(
                    &en,
                    &bool_of(se_to_z3(
                        guard,
                        spec,
                        &step_vars(i),
                        Some(tr.class),
                        Some(1),
                        false,
                    )),
                );
            }
            enableds.push(en);
        }
        // fire_t = enabled_t AND no earlier transition enabled (first match).
        let mut fires: Vec<Bool> = Vec::new();
        let mut prefix_or: Option<Bool> = None;
        for t in 0..spec.transitions.len() {
            let fire = match &prefix_or {
                Some(p) => b_and(&p.not(), &enableds[t]),
                None => b_and(&enableds[t], &Bool::from_bool(true)),
            };
            // The and-true keeps the node fresh (no move out of the vec);
            // z3 rewrites it to `en` - the C backend does the same op.
            let new_prefix = match prefix_or {
                Some(p) => b_or(&p, &enableds[t]),
                None => b_and(&enableds[t], &Bool::from_bool(true)),
            };
            fires.push(fire);
            prefix_or = Some(new_prefix);
        }
        let any_fire = if fires.is_empty() {
            Bool::from_bool(false)
        } else {
            let mut acc: Option<Bool> = None;
            for f in &fires {
                acc = Some(match acc {
                    Some(a) => b_or(&a, f),
                    None => b_and(f, &Bool::from_bool(true)),
                });
            }
            acc.expect("non-empty fires")
        };
        alive.push(b_and(&alive[i], &any_fire));
        // input_is/output_has atoms for sample i+1: the transition fired at
        // step i IS the consumed input; its emits are that sample's outputs.
        let ii_row: Vec<Bool> = spec
            .input_atoms
            .iter()
            .enumerate()
            .map(|(_a, arms)| input_atom_expr(spec, arms, &fires, i, &fld_vars))
            .collect();
        ii_steps.push(ii_row);
        let oh_row: Vec<Bool> = spec
            .output_atoms
            .iter()
            .enumerate()
            .map(|(_a, def)| output_atom_expr(spec, def, &fires, i, &s_vars, &v_vars, &fld_vars))
            .collect();
        oh_steps.push(oh_row);
        if spec.transitions.is_empty() {
            // No transitions: deadlocked from step 0; state/vars frozen.
            base.push(s_vars[i].eq(&s_vars[i + 1]));
            for j in 0..spec.vars.len() {
                base.push(v_vars[i][j].eq(&v_vars[i + 1][j]));
            }
        } else {
            // Next state: ITE chain over firing transitions (last first).
            let mut chain: BV = BV::from_u64(
                spec.transitions[spec.transitions.len() - 1].next_state as u64,
                spec.state_bits,
            );
            for t in (0..spec.transitions.len() - 1).rev() {
                chain = fires[t].ite(
                    &BV::from_u64(spec.transitions[t].next_state as u64, spec.state_bits),
                    &chain,
                );
            }
            base.push(any_fire.ite(&chain, &s_vars[i]).eq(&s_vars[i + 1]));
            // Next vars: per-var ITE chains (unchanged when unassigned).
            for j in 0..spec.vars.len() {
                let mut acc: Option<BV> = None;
                for t in (0..spec.transitions.len()).rev() {
                    if let Some((_, expr)) = spec.transitions[t]
                        .assignments
                        .iter()
                        .find(|(vi, _)| *vi == j)
                    {
                        let e = bv_of(se_to_z3(
                            expr,
                            spec,
                            &step_vars(i),
                            Some(spec.transitions[t].class),
                            Some(spec.vars[j].width),
                            true,
                        ));
                        acc = Some(match acc {
                            Some(a) => fires[t].ite(&e, &a),
                            None => fires[t].ite(&e, &v_vars[i][j]),
                        });
                    } else if let Some(a) = acc {
                        acc = Some(fires[t].ite(&v_vars[i][j], &a));
                    }
                }
                let next_v = match acc {
                    Some(a) => any_fire.ite(&a, &v_vars[i][j]),
                    None => any_fire.ite(&v_vars[i][j], &v_vars[i][j]),
                };
                base.push(next_v.eq(&v_vars[i + 1][j]));
            }
        }
    }

    let ctx = SfCtx {
        s_vars: &s_vars,
        v_vars: &v_vars,
        fld_vars: &fld_vars,
        ii_steps: &ii_steps,
        oh_steps: &oh_steps,
        alive: &alive,
        k,
    };

    let mut out = String::new();
    let mut warnings: Vec<String> = Vec::new();

    // ---- Omissions: FULL properties without --full-properties ----
    let omitted: usize = spec
        .properties
        .iter()
        .filter(|p| p.full && !full_properties)
        .count();
    for prop in &spec.properties {
        if prop.full && !full_properties {
            warnings.push(format!(
                "warning: smt-bounded mode omits property '{}' by default - it uses bounded_eventually/until/nested temporal operators, which greatly affect solver time/memory at large bounds (O(k*N) formula growth). Pass --full-properties to include it.",
                prop.name
            ));
        }
    }

    // ---- Queries (fixed order: safety, then liveness; declaration order) ----
    for p in 0..spec.properties.len() {
        let prop = &spec.properties[p];
        if prop.full && !full_properties {
            continue; // omitted with a warning above
        }
        match prop.kind {
            SmtPropKind::Always => {
                // Anchor range: always[N] clips at N, next_depth at the shifted
                // horizon (sound - trailing anchors dropped, no false FAILs).
                let hi = k
                    .min(prop.bound.unwrap_or(k))
                    .min(k.saturating_sub(prop.next_depth));
                if hi == 0 {
                    continue; // no evaluable anchor within the bound
                }
                // viol_p = OR over i<hi: alive_i AND NOT phi_p(i).
                let mut viol: Option<Bool> = None;
                for i in 0..hi {
                    let phi = sf_to_z3(&prop.formula, spec, &ctx, i);
                    let bad = b_and(&alive[i], &phi.not());
                    viol = Some(match viol {
                        Some(v) => b_or(&v, &bad),
                        None => bad,
                    });
                }
                let solver = Solver::new();
                for b in &base {
                    solver.assert(b);
                }
                solver.assert(viol.expect("property formula"));
                match solver.check() {
                    SatResult::Sat => {
                        let m = solver.get_model().expect("model");
                        let mut vstep = 0usize;
                        for i in 0..hi {
                            if read_bool(&m, &alive[i])
                                && !read_bool(&m, &sf_to_z3(&prop.formula, spec, &ctx, i))
                            {
                                vstep = i;
                                break;
                            }
                        }
                        out.push_str("FAIL\n");
                        let mut summary = smt_summary_line(
                            SmtStatus::Fail,
                            k,
                            Some(FailReason::PropertyViolation),
                            Some(prop.name.as_str()),
                            Some(vstep),
                        );
                        if omitted > 0 {
                            summary.push_str(&format!(" properties_omitted={omitted}"));
                        }
                        out.push_str(&format!("{summary}\n"));
                        out.push_str(&format!("property violation: {}\n", prop.name));
                        out.push_str(&format!("  violation step: {vstep}\n"));
                        print_trace(
                            &mut out,
                            k,
                            &m,
                            spec,
                            &s_vars,
                            &v_vars,
                            ev_vars.as_deref(),
                            &fld_vars,
                        );
                        return Ok(SmtOutcome {
                            exit_code: 1,
                            output: out,
                            warnings,
                        });
                    }
                    SatResult::Unsat => {}
                    SatResult::Unknown => {
                        out.push_str(&format!("UNKNOWN (solver inconclusive within bound {k})\n"));
                        let mut summary = smt_summary_line(SmtStatus::Unknown, k, None, None, None);
                        if omitted > 0 {
                            summary.push_str(&format!(" properties_omitted={omitted}"));
                        }
                        out.push_str(&format!("{summary}\n"));
                        return Ok(SmtOutcome {
                            exit_code: 2,
                            output: out,
                            warnings,
                        });
                    }
                }
            }
            SmtPropKind::Eventually => {
                // never_p = phi_p holds at no alive step within the (next-
                // clipped) horizon.
                let hi = k.saturating_sub(prop.next_depth);
                let mut never: Option<Bool> = None;
                for i in 0..hi {
                    let phi = sf_to_z3(&prop.formula, spec, &ctx, i);
                    let n = b_and(&alive[i], &phi).not();
                    never = Some(match never {
                        Some(v) => b_and(&v, &n),
                        None => n,
                    });
                }
                // cycle = OR over a<b: identical (state, vars) - an infinite path.
                let mut cyc: Option<Bool> = None;
                for a in 0..k {
                    for b in (a + 1)..k {
                        let mut eq = s_vars[a].eq(&s_vars[b]);
                        for j in 0..spec.vars.len() {
                            eq = b_and(&eq, &v_vars[a][j].eq(&v_vars[b][j]));
                        }
                        cyc = Some(match cyc {
                            Some(c) => b_or(&c, &eq),
                            None => eq,
                        });
                    }
                }
                if let Some(c) = cyc {
                    let solver = Solver::new();
                    for b in &base {
                        solver.assert(b);
                    }
                    solver.assert(never.expect("liveness formula"));
                    solver.assert(&c);
                    match solver.check() {
                        SatResult::Sat => {
                            let m = solver.get_model().expect("model");
                            // Find the cycle pair from the model (first in order).
                            let cfgs: Vec<(u64, Vec<u64>)> = (0..k)
                                .map(|i| {
                                    let s = read_bv(&m, &s_vars[i]);
                                    let vs: Vec<u64> = (0..spec.vars.len())
                                        .map(|j| read_bv(&m, &v_vars[i][j]))
                                        .collect();
                                    (s, vs)
                                })
                                .collect();
                            let mut pair = (0usize, 1usize);
                            'outer: for a in 0..k {
                                for b in (a + 1)..k {
                                    if cfgs[a] == cfgs[b] {
                                        pair = (a, b);
                                        break 'outer;
                                    }
                                }
                            }
                            out.push_str("FAIL\n");
                            let mut summary = smt_summary_line(
                                SmtStatus::Fail,
                                k,
                                Some(FailReason::LivenessViolation),
                                Some(prop.name.as_str()),
                                Some(pair.0),
                            );
                            if omitted > 0 {
                                summary.push_str(&format!(" properties_omitted={omitted}"));
                            }
                            out.push_str(&format!("{summary}\n"));
                            out.push_str(&format!("liveness violation: {}\n", prop.name));
                            out.push_str(&format!("  violation step: {}\n", pair.0));
                            out.push_str(&format!("  cycle length: {}\n", pair.1 - pair.0));
                            print_trace(
                                &mut out,
                                k,
                                &m,
                                spec,
                                &s_vars,
                                &v_vars,
                                ev_vars.as_deref(),
                                &fld_vars,
                            );
                            return Ok(SmtOutcome {
                                exit_code: 1,
                                output: out,
                                warnings,
                            });
                        }
                        SatResult::Unsat => {}
                        SatResult::Unknown => {
                            out.push_str(&format!(
                                "UNKNOWN (solver inconclusive within bound {k})\n"
                            ));
                            let mut summary =
                                smt_summary_line(SmtStatus::Unknown, k, None, None, None);
                            if omitted > 0 {
                                summary.push_str(&format!(" properties_omitted={omitted}"));
                            }
                            out.push_str(&format!("{summary}\n"));
                            return Ok(SmtOutcome {
                                exit_code: 2,
                                output: out,
                                warnings,
                            });
                        }
                    }
                }
            }
            SmtPropKind::BoundedEventually(n) => {
                // One-shot from sample 0: f within [0, n-1]. A window that
                // exceeds the horizon (or is next-clipped short of it) is still
                // pending - no violation claim. `prop.formula` IS the body f
                // (the top-level temporal operator lives in `kind`). D2 fix: a
                // violation requires the path to SURVIVE to the deadline
                // (alive[hi-1]) - if it died earlier the deadline sample does not
                // exist and the obligation is unconfirmable (SEMANTICS 5.5). alive
                // is monotone-decreasing, so alive[hi-1] implies every earlier
                // alive[j]; the disjunction needs no per-disjunct gate. This also
                // makes the in-process encoding IDENTICAL to the generated one
                // (which previously gated no disjunct at all - a parity gap).
                let hi = n.min(k.saturating_sub(prop.next_depth));
                if hi < n {
                    continue; // pending within the bound
                }
                let mut sat = Bool::from_bool(false);
                for j in 0..hi {
                    sat = b_or(&sat, &sf_to_z3(&prop.formula, spec, &ctx, j));
                }
                let solver = Solver::new();
                for b in &base {
                    solver.assert(b);
                }
                // hi == 0 (bounded_eventually[0], an empty window) keeps the
                // pre-existing NOT-sat assertion; hi >= 1 adds the deadline gate.
                let mut viol = sat.not();
                if hi >= 1 {
                    viol = b_and(&alive[hi - 1], &viol);
                }
                solver.assert(viol);
                match solver.check() {
                    SatResult::Sat => {
                        let m = solver.get_model().expect("model");
                        // Confirmed at the obligation's deadline (n-1), as in
                        // exact mode.
                        let vstep = n - 1;
                        out.push_str("FAIL\n");
                        let mut summary = smt_summary_line(
                            SmtStatus::Fail,
                            k,
                            Some(FailReason::PropertyViolation),
                            Some(prop.name.as_str()),
                            Some(vstep),
                        );
                        if omitted > 0 {
                            summary.push_str(&format!(" properties_omitted={omitted}"));
                        }
                        out.push_str(&format!("{summary}\n"));
                        out.push_str(&format!("property violation: {}\n", prop.name));
                        out.push_str(&format!("  violation step: {vstep}\n"));
                        print_trace(
                            &mut out,
                            k,
                            &m,
                            spec,
                            &s_vars,
                            &v_vars,
                            ev_vars.as_deref(),
                            &fld_vars,
                        );
                        return Ok(SmtOutcome {
                            exit_code: 1,
                            output: out,
                            warnings,
                        });
                    }
                    SatResult::Unsat => {}
                    SatResult::Unknown => {
                        out.push_str(&format!("UNKNOWN (solver inconclusive within bound {k})\n"));
                        let mut summary = smt_summary_line(SmtStatus::Unknown, k, None, None, None);
                        if omitted > 0 {
                            summary.push_str(&format!(" properties_omitted={omitted}"));
                        }
                        out.push_str(&format!("{summary}\n"));
                        return Ok(SmtOutcome {
                            exit_code: 2,
                            output: out,
                            warnings,
                        });
                    }
                }
            }
            SmtPropKind::Until => {
                // Weak until over the horizon; a violation needs an infinite
                // path (cycle) on which the until formula is false - the same
                // liveness treatment as eventually.
                let phi = sf_to_z3(&prop.formula, spec, &ctx, 0);
                let mut cyc: Option<Bool> = None;
                for a in 0..k {
                    for b in (a + 1)..k {
                        let mut eq = s_vars[a].eq(&s_vars[b]);
                        for j in 0..spec.vars.len() {
                            eq = b_and(&eq, &v_vars[a][j].eq(&v_vars[b][j]));
                        }
                        cyc = Some(match cyc {
                            Some(c) => b_or(&c, &eq),
                            None => eq,
                        });
                    }
                }
                if let Some(c) = cyc {
                    let solver = Solver::new();
                    for b in &base {
                        solver.assert(b);
                    }
                    solver.assert(phi.not());
                    solver.assert(&c);
                    match solver.check() {
                        SatResult::Sat => {
                            let m = solver.get_model().expect("model");
                            let cfgs: Vec<(u64, Vec<u64>)> = (0..k)
                                .map(|i| {
                                    let s = read_bv(&m, &s_vars[i]);
                                    let vs: Vec<u64> = (0..spec.vars.len())
                                        .map(|j| read_bv(&m, &v_vars[i][j]))
                                        .collect();
                                    (s, vs)
                                })
                                .collect();
                            let mut pair = (0usize, 1usize);
                            'outer: for a in 0..k {
                                for b in (a + 1)..k {
                                    if cfgs[a] == cfgs[b] {
                                        pair = (a, b);
                                        break 'outer;
                                    }
                                }
                            }
                            out.push_str("FAIL\n");
                            let mut summary = smt_summary_line(
                                SmtStatus::Fail,
                                k,
                                Some(FailReason::LivenessViolation),
                                Some(prop.name.as_str()),
                                Some(pair.0),
                            );
                            if omitted > 0 {
                                summary.push_str(&format!(" properties_omitted={omitted}"));
                            }
                            out.push_str(&format!("{summary}\n"));
                            out.push_str(&format!("liveness violation: {}\n", prop.name));
                            out.push_str(&format!("  violation step: {}\n", pair.0));
                            out.push_str(&format!("  cycle length: {}\n", pair.1 - pair.0));
                            print_trace(
                                &mut out,
                                k,
                                &m,
                                spec,
                                &s_vars,
                                &v_vars,
                                ev_vars.as_deref(),
                                &fld_vars,
                            );
                            return Ok(SmtOutcome {
                                exit_code: 1,
                                output: out,
                                warnings,
                            });
                        }
                        SatResult::Unsat => {}
                        SatResult::Unknown => {
                            out.push_str(&format!(
                                "UNKNOWN (solver inconclusive within bound {k})\n"
                            ));
                            let mut summary =
                                smt_summary_line(SmtStatus::Unknown, k, None, None, None);
                            if omitted > 0 {
                                summary.push_str(&format!(" properties_omitted={omitted}"));
                            }
                            out.push_str(&format!("{summary}\n"));
                            return Ok(SmtOutcome {
                                exit_code: 2,
                                output: out,
                                warnings,
                            });
                        }
                    }
                }
            }
        }
    }
    // No violation found within the bound (incomplete - no claim).
    out.push_str(&format!(
        "PASS WITHIN BOUND {k} (incomplete - no completeness claim)\n"
    ));
    let mut summary = smt_summary_line(SmtStatus::PassWithinBound, k, None, None, None);
    if omitted > 0 {
        summary.push_str(&format!(" properties_omitted={omitted}"));
    }
    out.push_str(&format!("{summary}\n"));
    Ok(SmtOutcome {
        exit_code: 0,
        output: out,
        warnings,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lang::parser::parse_source;

    fn module(src: &str) -> Module {
        let ir = parse_source(src).expect("test spec parses");
        assert_eq!(ir.modules.len(), 1);
        ir.modules.into_iter().next().unwrap()
    }

    /// FAIL shape: the property is violated in the INITIAL configuration, so
    /// every bound >= 1 reports a property-violation at step 0.
    const FAIL_SRC: &str = r#"
module smt_ref_fail {
  enum State { A, B }
  initial A
  event E
  var x: bit<8> init 0
  transition A on E { next B; set x = 1; }
  prop p : always (x > 0) ;
}
"#;

    /// LIVENESS shape: the machine is stuck in A, so `eventually (state == B)`
    /// is violated - k-liveness must report a cycle.
    const LIV_SRC: &str = r#"
module smt_ref_liv {
  enum State { A, B }
  initial A
  event E
  transition A on E { next A; }
  prop reach_b : eventually (state == B) ;
}
"#;

    /// PASS shape: the property holds within any bound.
    const PASS_SRC: &str = r#"
module smt_ref_pass {
  enum State { A, B }
  initial A
  event E
  var x: bit<8> init 0
  transition A on E { next B; set x = 1; }
  transition B on E { next B; }
  prop p : always (x < 256) ;
}
"#;

    #[test]
    fn fail_at_initial_config() {
        let m = module(FAIL_SRC);
        for k in [1usize, 3, 5] {
            let out = run(&m, k, false).expect("smt run");
            assert_eq!(out.exit_code, 1, "bound {k}");
            assert!(
                out.output.starts_with(&format!(
                    "FAIL\nmodel-check summary: status=fail mode=smt-bounded bound={k} reason=property-violation label=p step=0\n"
                )),
                "bound {k}: {}",
                out.output
            );
            assert!(out.output.contains("  violation step: 0\n"), "bound {k}");
            assert!(
                out.output
                    .contains(&format!("witness trace (solver model; bound k={k}):\n")),
                "bound {k}: witness trace missing"
            );
        }
    }

    #[test]
    fn liveness_cycle_report() {
        let m = module(LIV_SRC);
        // Bound 1 has no a<b pair: the cycle query is vacuous -> pass.
        let out1 = run(&m, 1, false).expect("smt run");
        assert_eq!(out1.exit_code, 0, "bound 1: {}", out1.output);
        for k in [2usize, 4] {
            let out = run(&m, k, false).expect("smt run");
            assert_eq!(out.exit_code, 1, "bound {k}: {}", out.output);
            assert!(
                out.output.contains(&format!(
                    "model-check summary: status=fail mode=smt-bounded bound={k} reason=liveness-violation label=reach_b step=0\n"
                )),
                "bound {k}: {}",
                out.output
            );
            assert!(
                out.output.contains("liveness violation: reach_b\n"),
                "bound {k}"
            );
            assert!(out.output.contains("  cycle length: "), "bound {k}");
        }
    }

    #[test]
    fn pass_within_bound() {
        let m = module(PASS_SRC);
        for k in [1usize, 3, 10] {
            let out = run(&m, k, false).expect("smt run");
            assert_eq!(out.exit_code, 0, "bound {k}: {}", out.output);
            assert!(
                out.output == format!(
                    "PASS WITHIN BOUND {k} (incomplete - no completeness claim)\nmodel-check summary: status=pass-within-bound mode=smt-bounded bound={k}\n"
                ),
                "bound {k}: {}",
                out.output
            );
        }
    }

    #[test]
    fn rejection_errors_surface_verbatim() {
        // Assumptions are rejected by build_smt_spec (same error as codegen).
        let m = module(
            r#"
module smt_ref_assume {
  enum State { A }
  initial A
  event E
  assume a1 : always (true) ;
  transition A on E { next A; }
}
"#,
        );
        let err = run(&m, 3, false).expect_err("assumption spec must be rejected");
        assert!(
            err.starts_with("smt-bounded mode does not support assumptions"),
            "{err}"
        );
        // on_error blocks are rejected the same way.
        let m = module(
            r#"
module smt_ref_onerr {
  enum State { A }
  initial A
  event E
  transition A on E { next A; }
  on_error { next A; }
}
"#,
        );
        let err = run(&m, 3, false).expect_err("on_error spec must be rejected");
        assert_eq!(err, "smt-bounded mode does not support on_error blocks");
    }
}
