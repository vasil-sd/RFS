//! Shared SMT code emitter (Phase 4 task 11): renders [`SmtSpec`] into
//! backend-specific source - Rust (z3 crate) and C (libz3 C API). Both
//! backends render the SAME spec with the SAME traversal order and the SAME
//! z3 operations, so constraint construction is identical in Rust and C
//! generated binaries (parity by construction, D-P4-12).
//!
//! The emitted spec section provides:
//! - constants (states, vars, classes, fields, transitions, properties);
//! - one expression-builder function per transition guard / assignment /
//!   property formula;
//! - function-pointer tables + an `ASGN_MAP` so the spec-independent engine
//!   (in the backend files) can loop over them.
//!
//! Leaf-variable convention (parity-critical): step variables are z3
//! CONSTANTS with fixed names (`s_<i>`, `v_<i>_<j>`, `ev_<i>`,
//! `fld_<i>_<g>`). The C builders receive them as pointers; the Rust builders
//! recreate them by name - z3 hash-consing returns the SAME node, so both
//! backends build structurally identical formulas.
//! Docs: dev:mc-engine-overview, dev:mc-engine-smt, dev:algorithms-mc-engine, llm:mc-engine

use crate::mc::smt_plan::{se_type, Sb, Se, SeType, Sf, SmtSpec};

/// Global concrete-field ordering: all classes in order, fields within a class
/// in order. Both emitters use this index for the `fld` step-variable arrays.
pub fn global_field_index(spec: &SmtSpec, class: usize, local: usize) -> usize {
    let mut g = 0;
    for (c, cls) in spec.classes.iter().enumerate() {
        if c == class {
            return g + local;
        }
        g += cls.fields.len();
    }
    unreachable!("class index out of range")
}

/// Whether an expression references step variables (state/vars/fields) -
/// decides whether the builder function's `i` parameter is used (`_i`
/// otherwise, so generated code compiles warning-free).
fn se_uses_step(se: &Se) -> bool {
    match se {
        Se::State | Se::Var(_) => true,
        Se::Field { .. } => true,
        Se::Lit { .. } => false,
        Se::Not(i) | Se::BitNot(i) | Se::Neg(i) => se_uses_step(i),
        Se::Cast { inner, .. } => se_uses_step(inner),
        Se::Bin(_, l, r) => se_uses_step(l) || se_uses_step(r),
    }
}

fn sf_uses_step(sf: &Sf) -> bool {
    match sf {
        Sf::Se(e) => se_uses_step(e),
        Sf::Not(i) => sf_uses_step(i),
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) => sf_uses_step(l) || sf_uses_step(r),
        // Atoms reference step-indexed named constants (ii_/oh_); every
        // temporal node shifts or expands over steps.
        Sf::InputIs { .. } | Sf::OutputHas { .. } => true,
        Sf::Next(i)
        | Sf::BoundedEventually { inner: i, .. }
        | Sf::BoundedAlways { inner: i, .. }
        | Sf::Eventually(i) => sf_uses_step(i),
        Sf::Until { left, right } => sf_uses_step(left) || sf_uses_step(right),
    }
}

/// True iff the formula references an input_is atom (the property fn must
/// name its `ii_steps` parameter without the underscore prefix).
fn sf_uses_input_atoms(sf: &Sf) -> bool {
    match sf {
        Sf::InputIs { .. } => true,
        Sf::Not(i) => sf_uses_input_atoms(i),
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) => {
            sf_uses_input_atoms(l) || sf_uses_input_atoms(r)
        }
        Sf::Next(i)
        | Sf::BoundedEventually { inner: i, .. }
        | Sf::BoundedAlways { inner: i, .. }
        | Sf::Eventually(i) => sf_uses_input_atoms(i),
        Sf::Until { left, right } => sf_uses_input_atoms(left) || sf_uses_input_atoms(right),
        _ => false,
    }
}

/// True iff the formula references an output_has atom.
fn sf_uses_output_atoms(sf: &Sf) -> bool {
    match sf {
        Sf::OutputHas { .. } => true,
        Sf::Not(i) => sf_uses_output_atoms(i),
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) => {
            sf_uses_output_atoms(l) || sf_uses_output_atoms(r)
        }
        Sf::Next(i)
        | Sf::BoundedEventually { inner: i, .. }
        | Sf::BoundedAlways { inner: i, .. }
        | Sf::Eventually(i) => sf_uses_output_atoms(i),
        Sf::Until { left, right } => sf_uses_output_atoms(left) || sf_uses_output_atoms(right),
        _ => false,
    }
}

/// True iff the formula contains a FULL-class temporal node (bounded_
/// eventually / until / nested always[N] / nested eventually) - such nodes
/// render horizon-truncated loops that reference `k`, so the builder fn must
/// name its `k` parameter without the underscore prefix.
fn sf_uses_horizon(sf: &Sf) -> bool {
    match sf {
        Sf::BoundedEventually { .. }
        | Sf::Until { .. }
        | Sf::BoundedAlways { .. }
        | Sf::Eventually(_) => true,
        Sf::Not(inner) | Sf::Next(inner) => sf_uses_horizon(inner),
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) => {
            sf_uses_horizon(l) || sf_uses_horizon(r)
        }
        Sf::Se(_) | Sf::InputIs { .. } | Sf::OutputHas { .. } => false,
    }
}

/// True iff the formula contains a `next` node (the property fn must name its
/// `alive` parameter without the underscore prefix - the D1 liveness gate
/// references `alive[step+1]`).
fn sf_uses_next(sf: &Sf) -> bool {
    match sf {
        Sf::Next(_) => true,
        Sf::Not(i) => sf_uses_next(i),
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) => sf_uses_next(l) || sf_uses_next(r),
        Sf::BoundedEventually { inner: i, .. }
        | Sf::BoundedAlways { inner: i, .. }
        | Sf::Eventually(i) => sf_uses_next(i),
        Sf::Until { left, right } => sf_uses_next(left) || sf_uses_next(right),
        _ => false,
    }
}

// ---------------------------------------------------------------------------
// Rust emission
// ---------------------------------------------------------------------------

struct RustRender {
    lines: Vec<String>,
    n: usize,
}

impl RustRender {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            n: 0,
        }
    }
    /// Push a binding line; returns the binding name. All bindings are OWNED
    /// z3 values (leaves are recreated constants - see module docs).
    fn push(&mut self, code: String) -> String {
        let name = format!("e{}", self.n);
        self.n += 1;
        self.lines.push(code);
        name
    }
}

/// Render an [`Se`] tree to sequential let-bindings. `expected` is the parent
/// context's bit width (lifts integer literals per the validator's rules);
/// `class` is the transition class for field resolution (None in properties,
/// where fields cannot occur).
/// Render one [`Se`] node as sequential let-bindings; returns the final
/// binding name. `expected` is the width hint for integer literals (validator
/// lifting rules); `class` resolves field indices; `bool_as_bv` selects the
/// representation of Bool-typed results: false = z3 Bool ast, true = BV(1)
/// ast (needed only at assignment top level, where the target var is a BV).
/// Render an [`Se`] tree to sequential let-bindings. `expected` is the parent
/// context's bit width (lifts integer literals per the validator's rules);
/// `class` is the transition class for field resolution (None in properties,
/// where fields cannot occur). `step` is the Rust expression evaluating to the
/// current step index: "i" renders the byte-identical inline form
/// (`format!("s_{i}")`); any other expression ("i + 1", "i + j") renders the
/// spaced form (`format!("s_{}", i + 1)`) - z3 hash-conses both to the same
/// step constants.
fn render_se_rust(
    se: &Se,
    spec: &SmtSpec,
    expected: Option<u32>,
    class: Option<usize>,
    bool_as_bv: bool,
    step: &str,
    r: &mut RustRender,
) -> String {
    // Step-leaf renderers: inline form for the base step (byte-identity),
    // spaced form for shifted steps.
    let state_leaf = |step: &str| -> String {
        if step == "i" {
            "BV::new_const(format!(\"s_{i}\"), STATE_BITS)".to_string()
        } else {
            format!("BV::new_const(format!(\"s_{{}}\", {step}), STATE_BITS)")
        }
    };
    let var_leaf = |j: usize, step: &str| -> String {
        if step == "i" {
            format!("BV::new_const(format!(\"v_{{i}}_{j}\"), VAR_WIDTHS[{j}])")
        } else {
            format!("BV::new_const(format!(\"v_{{}}_{j}\", {step}), VAR_WIDTHS[{j}])")
        }
    };
    let field_leaf = |g: usize, step: &str| -> String {
        if step == "i" {
            format!("BV::new_const(format!(\"fld_{{i}}_{g}\"), FIELD_WIDTHS[{g}])")
        } else {
            format!("BV::new_const(format!(\"fld_{{}}_{g}\", {step}), FIELD_WIDTHS[{g}])")
        }
    };
    match se {
        Se::Lit { value, is_bool } => {
            if *is_bool && !bool_as_bv {
                r.push(format!(
                    "let e{}: Bool = Bool::from_bool({});",
                    r.n,
                    *value != 0
                ))
            } else {
                // Task 3L: a folded full-precision literal that does not fit the
                // context width is masked (two's complement); an in-range literal
                // renders exactly as before (byte-identical pins preserved).
                let w = expected.unwrap_or(8);
                let m = if w >= 128 {
                    u128::MAX
                } else {
                    (1u128 << w) - 1
                };
                if *value <= m {
                    r.push(format!(
                        "let e{}: BV = BV::from_u64({}u64, {w});",
                        r.n, *value as u64
                    ))
                } else {
                    r.push(format!(
                        "let e{}: BV = BV::from_u64(({}u128 & {}) as u64, {w});",
                        r.n, value, m
                    ))
                }
            }
        }
        // Explicit conversion cast (task 3L): zero-extension / truncation of the
        // operand to `width` bits.
        Se::Cast { width, inner } => {
            let c = render_se_rust(inner, spec, Some(*width), class, false, step, r);
            let m = if *width >= 128 {
                u128::MAX
            } else {
                (1u128 << width) - 1
            };
            r.push(format!(
                "let e{}: BV = {c}.bvand(&BV::from_u64({}u64, {width}));",
                r.n, m as u64
            ))
        }
        Se::State => r.push(format!("let e{}: BV = {};", r.n, state_leaf(step))),
        Se::Var(j) => {
            let leaf = var_leaf(*j, step);
            if spec.vars[*j].is_bool && !bool_as_bv {
                let l = r.push(format!("let e{}: BV = {leaf};", r.n));
                r.push(format!(
                    "let e{}: Bool = {l}.eq(&BV::from_u64(1u64, 1));",
                    r.n
                ))
            } else {
                r.push(format!("let e{}: BV = {leaf};", r.n))
            }
        }
        Se::Field { idx, is_bool, .. } => {
            let g = global_field_index(spec, class.expect("field outside transition"), *idx);
            let leaf = field_leaf(g, step);
            if *is_bool && !bool_as_bv {
                let l = r.push(format!("let e{}: BV = {leaf};", r.n));
                r.push(format!(
                    "let e{}: Bool = {l}.eq(&BV::from_u64(1u64, 1));",
                    r.n
                ))
            } else {
                r.push(format!("let e{}: BV = {leaf};", r.n))
            }
        }
        Se::Not(inner) => {
            let c = render_se_rust(inner, spec, Some(1), class, false, step, r);
            let b = r.push(format!("let e{}: Bool = {c}.not();", r.n));
            finish_bool_rust(&b, bool_as_bv, r)
        }
        Se::BitNot(inner) | Se::Neg(inner) => {
            let w = se_type(inner, spec).width();
            let c = render_se_rust(inner, spec, Some(w), class, false, step, r);
            let m = if matches!(se, Se::BitNot(_)) {
                "bvnot"
            } else {
                "bvneg"
            };
            r.push(format!("let e{}: BV = {c}.{m}();", r.n))
        }
        Se::Bin(op, l, rr) => {
            let lt = se_type(l, spec);
            let rt = se_type(rr, spec);
            if produces_bool(*op, lt, rt) {
                let b = render_bool_bin_rust(*op, l, rr, spec, class, step, r);
                finish_bool_rust(&b, bool_as_bv, r)
            } else {
                render_bit_bin_rust(*op, l, rr, spec, class, step, r)
            }
        }
    }
}

/// Wrap a z3 Bool result as BV(1) when the context requires it (assignment
/// values). The ite is structurally identical in both backends.
fn finish_bool_rust(name: &str, bool_as_bv: bool, r: &mut RustRender) -> String {
    if !bool_as_bv {
        return name.to_string();
    }
    let one = r.push(format!("let e{}: BV = BV::from_u64(1u64, 1);", r.n));
    let zero = r.push(format!("let e{}: BV = BV::from_u64(0u64, 1);", r.n));
    r.push(format!("let e{}: BV = {name}.ite(&{one}, &{zero});", r.n))
}

/// Whether a binary op produces a Bool-typed result (validator rules).
fn produces_bool(op: Sb, lt: SeType, rt: SeType) -> bool {
    match op {
        Sb::LogicAnd | Sb::LogicOr => true,
        Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => true,
        Sb::BitAnd | Sb::BitOr => lt == SeType::Bool && rt == SeType::Bool,
        _ => false,
    }
}

/// Comparison operand widths (validator lifting + exact-MC full-value
/// semantics): integer literals lift to the sibling's width; if a literal
/// does not fit, BOTH sides widen to hold it and the non-literal side is
/// zero-extended (exact-MC compares full values: `counter < 256` on a
/// bit<8> is always true). Arithmetic ops need no such widening - truncating
/// the literal is equivalent there (ring homomorphism). Returns per-side
/// (render width, extra zero bits); both sides end at the same final width.
fn cmp_widths_full(l: &Se, rr: &Se, spec: &SmtSpec) -> (u32, u32, u32, u32) {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
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
    for side in [l, rr] {
        if let Se::Lit {
            value,
            is_bool: false,
        } = side
        {
            // Task 3L: literal values are u128 (full-precision folded constants).
            let bits = if *value == 0 {
                1
            } else {
                128 - value.leading_zeros()
            };
            w = w.max(bits as u32);
        }
    }
    // Literals render directly at the final width (no extension); other
    // sides render at their natural width and zero-extend the difference.
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

/// Zero-extend a rendered operand when the comparison widened past its
/// natural width (literals render directly at the final width instead).
fn widen_rust(name: &str, ext: u32, r: &mut RustRender) -> String {
    if ext == 0 {
        return name.to_string();
    }
    let z = r.push(format!("let e{}: BV = BV::from_u64(0u64, {});", r.n, ext));
    r.push(format!("let e{}: BV = {z}.concat(&{name});", r.n))
}

/// C mirror of [`widen_rust`].
fn widen_c(name: &str, ext: u32, r: &mut CRender) -> String {
    if ext == 0 {
        return name.to_string();
    }
    let z = r.push(format!("Z3_ast e{} = z3_num(c, {}, 0ull);", r.n, ext));
    r.push(format!("Z3_ast e{} = Z3_mk_concat(c, {z}, {name});", r.n))
}

/// Bool-producing binary ops: children render as z3 Bools.
fn render_bool_bin_rust(
    op: Sb,
    l: &Se,
    rr: &Se,
    spec: &SmtSpec,
    class: Option<usize>,
    step: &str,
    r: &mut RustRender,
) -> String {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    match op {
        Sb::LogicAnd | Sb::LogicOr => {
            let a = render_se_rust(l, spec, Some(1), class, false, step, r);
            let b = render_se_rust(rr, spec, Some(1), class, false, step, r);
            let m = if op == Sb::LogicAnd { "and" } else { "or" };
            r.push(format!("let e{}: Bool = b_{m}(&{a}, &{b});", r.n))
        }
        Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => {
            let (wl, wr, ext_l, ext_r) = cmp_widths_full(l, rr, spec);
            let a0 = render_se_rust(l, spec, Some(wl), class, false, step, r);
            let b0 = render_se_rust(rr, spec, Some(wr), class, false, step, r);
            let a = widen_rust(&a0, ext_l, r);
            let b = widen_rust(&b0, ext_r, r);
            if lt == SeType::Bool && rt == SeType::Bool {
                // Bool ordering encodings (RFS allows comparisons on bools).
                match op {
                    Sb::Eq => r.push(format!("let e{}: Bool = {a}.eq(&{b});", r.n)),
                    Sb::NotEq => {
                        let t = r.push(format!("let e{}: Bool = {a}.eq(&{b});", r.n));
                        r.push(format!("let e{}: Bool = {t}.not();", r.n))
                    }
                    Sb::Lt => {
                        let na = r.push(format!("let e{}: Bool = {a}.not();", r.n));
                        r.push(format!("let e{}: Bool = b_and(&{na}, &{b});", r.n))
                    }
                    Sb::Le => {
                        let nb = r.push(format!("let e{}: Bool = {b}.not();", r.n));
                        let ab = r.push(format!("let e{}: Bool = b_and(&{a}, &{nb});", r.n));
                        r.push(format!("let e{}: Bool = {ab}.not();", r.n))
                    }
                    Sb::Gt => {
                        let nb = r.push(format!("let e{}: Bool = {b}.not();", r.n));
                        r.push(format!("let e{}: Bool = b_and(&{nb}, &{a});", r.n))
                    }
                    Sb::Ge => {
                        let na = r.push(format!("let e{}: Bool = {a}.not();", r.n));
                        let ba = r.push(format!("let e{}: Bool = b_and(&{b}, &{na});", r.n));
                        r.push(format!("let e{}: Bool = {ba}.not();", r.n))
                    }
                    _ => unreachable!("comparison op only"),
                }
            } else {
                // Unsigned bitvector comparisons.
                match op {
                    Sb::Eq => r.push(format!("let e{}: Bool = {a}.eq(&{b});", r.n)),
                    Sb::NotEq => {
                        let t = r.push(format!("let e{}: Bool = {a}.eq(&{b});", r.n));
                        r.push(format!("let e{}: Bool = {t}.not();", r.n))
                    }
                    Sb::Lt => r.push(format!("let e{}: Bool = {a}.bvult(&{b});", r.n)),
                    Sb::Le => r.push(format!("let e{}: Bool = {a}.bvule(&{b});", r.n)),
                    Sb::Gt => r.push(format!("let e{}: Bool = {a}.bvugt(&{b});", r.n)),
                    Sb::Ge => r.push(format!("let e{}: Bool = {a}.bvuge(&{b});", r.n)),
                    _ => unreachable!("comparison op only"),
                }
            }
        }
        Sb::BitAnd | Sb::BitOr => {
            // Both operands are Bool (checked by produces_bool): logical ops.
            let a = render_se_rust(l, spec, Some(1), class, false, step, r);
            let b = render_se_rust(rr, spec, Some(1), class, false, step, r);
            let m = if op == Sb::BitAnd { "and" } else { "or" };
            r.push(format!("let e{}: Bool = b_{m}(&{a}, &{b});", r.n))
        }
        _ => unreachable!("bool-producing op only"),
    }
}

/// Bit(w)-producing binary ops (arithmetic, bitwise, shifts).
fn render_bit_bin_rust(
    op: Sb,
    l: &Se,
    rr: &Se,
    spec: &SmtSpec,
    class: Option<usize>,
    step: &str,
    r: &mut RustRender,
) -> String {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    match op {
        Sb::Plus | Sb::Minus | Sb::Star | Sb::BitAnd | Sb::BitOr | Sb::BitXor => {
            let w = match (lt, rt) {
                (SeType::Bit(a), SeType::Bit(b)) => {
                    debug_assert_eq!(a, b);
                    a
                }
                (SeType::IntLit, t) | (t, SeType::IntLit) => t.width(),
                _ => 8,
            };
            let a = render_se_rust(l, spec, Some(w), class, false, step, r);
            let b = render_se_rust(rr, spec, Some(w), class, false, step, r);
            let m = match op {
                Sb::Plus => "bvadd",
                Sb::Minus => "bvsub",
                Sb::Star => "bvmul",
                Sb::BitAnd => "bvand",
                Sb::BitOr => "bvor",
                _ => "bvxor",
            };
            r.push(format!("let e{}: BV = {a}.{m}(&{b});", r.n))
        }
        Sb::ShiftLeft | Sb::ShiftRight => render_shift_rust(op, l, rr, spec, class, step, r),
        _ => unreachable!("bit-producing op only"),
    }
}

/// Render a property formula at the step named by `step` (a Rust expression:
/// "i" for the base step, "i + 1" under next, "i + j" inside FULL-class
/// expansion loops). Temporal nodes emit their quantifier-free expansions with
/// horizon clipping (`k` is in scope in every property/expansion function).
fn render_sf_rust(sf: &Sf, spec: &SmtSpec, step: &str, r: &mut RustRender) -> String {
    let shifted = |step: &str| -> String {
        if step == "i" {
            "i + 1".to_string()
        } else {
            format!("({step}) + 1")
        }
    };
    match sf {
        Sf::Se(inner) => render_se_rust(inner, spec, Some(1), None, false, step, r),
        Sf::Not(inner) => {
            let c = render_sf_rust(inner, spec, step, r);
            r.push(format!("let e{}: Bool = {c}.not();", r.n))
        }
        Sf::And(l, rr) => {
            let a = render_sf_rust(l, spec, step, r);
            let b = render_sf_rust(rr, spec, step, r);
            r.push(format!("let e{}: Bool = b_and(&{a}, &{b});", r.n))
        }
        Sf::Or(l, rr) => {
            let a = render_sf_rust(l, spec, step, r);
            let b = render_sf_rust(rr, spec, step, r);
            r.push(format!("let e{}: Bool = b_or(&{a}, &{b});", r.n))
        }
        Sf::Implies(l, rr) => {
            let a = render_sf_rust(l, spec, step, r);
            let b = render_sf_rust(rr, spec, step, r);
            r.push(format!("let e{}: Bool = {a}.implies(&{b});", r.n))
        }
        // Atoms: the engine passes the per-step atom storage (k x N_ATOMS);
        // step i's row is false at the final unrolled step (no input/output
        // consumed there). Explicit indexing - NOT name-shared constants, which
        // z3_mk_fresh_const does not guarantee to share across solver versions.
        Sf::InputIs { id, .. } => r.push(format!(
            "let e{}: Bool = ii_steps[{step}][{id}].clone();",
            r.n
        )),
        Sf::OutputHas { id, .. } => r.push(format!(
            "let e{}: Bool = oh_steps[{step}][{id}].clone();",
            r.n
        )),
        // next: the body holds one step later - WEAK at trace death (D1 fix):
        // if the path died at step+1 (an unhandled input froze the config), the
        // obligation is vacuously satisfied (alive[step+1] false -> true). The
        // `alive` parameter is in scope in every property/expansion fn.
        Sf::Next(inner) => {
            let b = render_sf_rust(inner, spec, &shifted(step), r);
            let ns = if step == "i" {
                "i + 1".to_string()
            } else {
                format!("({step}) + 1")
            };
            r.push(format!(
                "let e{}: Bool = b_or(&alive[{ns}].not(), &{b});",
                r.n
            ))
        }
        // FULL class: bounded_eventually[N] - f within N steps of the anchor
        // (horizon-clipped; clipping drops anchors, never creates false FAILs).
        Sf::BoundedEventually { bound, inner } => {
            let acc = r.push(format!("let mut e{}: Bool = Bool::from_bool(false);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            r.push(format!(
                "for {jv} in 0..{bound}.min(k.saturating_sub({step})) {{"
            ));
            let body_step = if step == "i" {
                format!("i + {jv}")
            } else {
                format!("({step}) + {jv}")
            };
            let b = render_sf_rust(inner, spec, &body_step, r);
            r.push(format!("{acc} = b_or(&{acc}, &{b});"));
            r.push("}".into());
            acc
        }
        // FULL class: weak until - (some j: right(j) with left before it) OR
        // (left forever - horizon-truncated, sound).
        Sf::Until { left, right } => {
            let rh = r.push(format!("let mut e{}: Bool = Bool::from_bool(false);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            let iv = format!("i{}", r.n);
            r.n += 1;
            r.push(format!("for {jv} in {step}..k {{"));
            let pf = r.push(format!("let mut e{}: Bool = Bool::from_bool(true);", r.n));
            // Weak until: p only strictly before the first q (SEMANTICS 6.1).
            r.push(format!("for {iv} in {step}..{jv} {{"));
            let lb = render_sf_rust(left, spec, &iv, r);
            r.push(format!("{pf} = b_and(&{pf}, &{lb});"));
            r.push("}".into());
            let rb = render_sf_rust(right, spec, &jv, r);
            r.push(format!("{rh} = b_or(&{rh}, &b_and(&{rb}, &{pf}));"));
            r.push("}".into());
            let lf = r.push(format!("let mut e{}: Bool = Bool::from_bool(true);", r.n));
            let iv2 = format!("i{}", r.n);
            r.n += 1;
            r.push(format!("for {iv2} in {step}..k {{"));
            let lb2 = render_sf_rust(left, spec, &iv2, r);
            r.push(format!("{lf} = b_and(&{lf}, &{lb2});"));
            r.push("}".into());
            r.push(format!("let e{}: Bool = b_or(&{rh}, &{lf});", r.n))
        }
        // FULL class: nested always[N] (unbounded nesting uses bound = MAX).
        Sf::BoundedAlways { bound, inner } => {
            let acc = r.push(format!("let mut e{}: Bool = Bool::from_bool(true);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            r.push(format!(
                "for {jv} in 0..{bound}.min(k.saturating_sub({step})) {{"
            ));
            let body_step = if step == "i" {
                format!("i + {jv}")
            } else {
                format!("({step}) + {jv}")
            };
            let b = render_sf_rust(inner, spec, &body_step, r);
            r.push(format!("{acc} = b_and(&{acc}, &{b});"));
            r.push("}".into());
            acc
        }
        // FULL class: nested eventually - f at some later step.
        Sf::Eventually(inner) => {
            let acc = r.push(format!("let mut e{}: Bool = Bool::from_bool(false);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            r.push(format!("for {jv} in {step}..k {{"));
            let b = render_sf_rust(inner, spec, &jv, r);
            r.push(format!("{acc} = b_or(&{acc}, &{b});"));
            r.push("}".into());
            acc
        }
    }
}

fn rust_str_list(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| format!("{s:?}")).collect();
    format!("[{}]", inner.join(", "))
}

/// One Rust string literal (variant names are identifiers - no escaping
/// surprises; task 3D enum display tables).
fn rust_str_lit(s: &str) -> String {
    format!("{s:?}")
}

/// Emit the full Rust spec section (constants + builder functions + tables).
pub fn emit_rust_spec(spec: &SmtSpec) -> String {
    let mut s = String::new();
    s.push_str("// ===== SMT bounded-mode spec (task 11) - rendered from the shared plan;\n");
    s.push_str("// identical in both backends (parity by construction, D-P4-12) =====\n");
    s.push_str(&format!("const STATE_BITS: u32 = {};\n", spec.state_bits));
    s.push_str(&format!(
        "const INITIAL_STATE: u64 = {};\n",
        spec.initial_state as u64
    ));
    s.push_str(&format!(
        "const STATES: &[&str] = &{};\n",
        rust_str_list(&spec.states)
    ));
    let vnames: Vec<String> = spec.vars.iter().map(|v| v.name.clone()).collect();
    s.push_str(&format!(
        "const VAR_NAMES: &[&str] = &{};\n",
        rust_str_list(&vnames)
    ));
    s.push_str(&format!(
        "const VAR_WIDTHS: &[u32] = &{:?};\n",
        spec.vars.iter().map(|v| v.width).collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "const VAR_INITS: &[u64] = &{:?};\n",
        spec.vars.iter().map(|v| v.init).collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "const VAR_IS_BOOL: &[bool] = &{:?};\n",
        spec.vars.iter().map(|v| v.is_bool).collect::<Vec<_>>()
    ));
    // Enum-var witness display (task 3D): concatenated variant names + per-var
    // offset/length (length 0 = not an enum - print the raw index as before).
    let mut ven: Vec<String> = Vec::new();
    let mut voff: Vec<String> = Vec::new();
    let mut vlen: Vec<String> = Vec::new();
    for v in &spec.vars {
        voff.push(ven.len().to_string());
        vlen.push(v.enum_variants.len().to_string());
        for name in &v.enum_variants {
            ven.push(rust_str_lit(name));
        }
    }
    s.push_str(&format!(
        "/// Enum-var variant names (concatenated; task 3D).\nconst VAR_ENUM_NAMES: &[&str] = &[{}];\nconst VAR_ENUM_OFF: &[usize] = &[{}];\nconst VAR_ENUM_LEN: &[usize] = &[{}];\n",
        ven.join(", "),
        voff.join(", "),
        vlen.join(", ")
    ));
    let cnames: Vec<String> = spec.classes.iter().map(|c| c.name.clone()).collect();
    s.push_str(&format!(
        "const CLASS_NAMES: &[&str] = &{};\n",
        rust_str_list(&cnames)
    ));
    s.push_str(&format!(
        "const N_CLASSES: usize = {};\n",
        spec.classes.len()
    ));
    let cls_bits = crate::mc::smt_plan::ceil_log2_pub(spec.classes.len());
    s.push_str(&format!("const CLS_BITS: u32 = {cls_bits};\n"));
    let fnames: Vec<String> = spec
        .classes
        .iter()
        .flat_map(|c| c.fields.iter().map(|(n, _, _, _)| n.clone()))
        .collect();
    s.push_str(&format!(
        "const FIELD_NAMES: &[&str] = &{};\n",
        rust_str_list(&fnames)
    ));
    s.push_str(&format!(
        "const FIELD_WIDTHS: &[u32] = &{:?};\n",
        spec.classes
            .iter()
            .flat_map(|c| c.fields.iter().map(|(_, w, _, _)| *w))
            .collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "const FIELD_IS_BOOL: &[bool] = &{:?};\n",
        spec.classes
            .iter()
            .flat_map(|c| c.fields.iter().map(|(_, _, b, _)| *b))
            .collect::<Vec<_>>()
    ));
    // Enum-field witness display (task 3D): same pattern as the vars.
    let mut fen: Vec<String> = Vec::new();
    let mut foff: Vec<String> = Vec::new();
    let mut flen: Vec<String> = Vec::new();
    for c in &spec.classes {
        for (_, _, _, ev) in &c.fields {
            foff.push(fen.len().to_string());
            flen.push(ev.len().to_string());
            for name in ev {
                fen.push(rust_str_lit(name));
            }
        }
    }
    s.push_str(&format!(
        "/// Enum-field variant names (concatenated; task 3D).\nconst FIELD_ENUM_NAMES: &[&str] = &[{}];\nconst FIELD_ENUM_OFF: &[usize] = &[{}];\nconst FIELD_ENUM_LEN: &[usize] = &[{}];\n",
        fen.join(", "),
        foff.join(", "),
        flen.join(", ")
    ));
    // (class, local field index) per global field - witness display.
    let mut fc: Vec<String> = Vec::new();
    for (ci, cls) in spec.classes.iter().enumerate() {
        for fi in 0..cls.fields.len() {
            fc.push(format!("({ci}, {fi})"));
        }
    }
    s.push_str(&format!(
        "const FIELD_POS: &[(usize, usize)] = &[{}];\n",
        fc.join(", ")
    ));
    let trans: Vec<String> = spec
        .transitions
        .iter()
        .map(|t| format!("({}, {})", t.state, t.next_state))
        .collect();
    s.push_str(&format!(
        "/// Transitions in firing order: (state, next_state).\nconst TRANS: &[(usize, usize)] = &[{}];\n",
        trans.join(", ")
    ));
    // Per-transition input classes (task 3D): concatenated class lists. A
    // named-variant transition has exactly one; a bare-group wildcard has all
    // of the group's variants (enabled_t ORs over them).
    let mut cls_all: Vec<String> = Vec::new();
    let mut cls_off: Vec<String> = Vec::new();
    let mut cls_len: Vec<String> = Vec::new();
    for t in &spec.transitions {
        cls_off.push(cls_all.len().to_string());
        cls_len.push(t.classes.len().to_string());
        for &c in &t.classes {
            cls_all.push(c.to_string());
        }
    }
    s.push_str(&format!(
        "/// Per-transition input classes (concatenated; task 3D).\nconst TRANS_CLASSES: &[usize] = &[{}];\nconst TRANS_CLS_OFF: &[usize] = &[{}];\nconst TRANS_CLS_LEN: &[usize] = &[{}];\n",
        cls_all.join(", "),
        cls_off.join(", "),
        cls_len.join(", ")
    ));
    let pnames: Vec<String> = spec.properties.iter().map(|p| p.name.clone()).collect();
    s.push_str(&format!(
        "const PROP_NAMES: &[&str] = &{};\n",
        rust_str_list(&pnames)
    ));
    s.push_str(&format!(
        "/// 0 = always (safety), 1 = eventually (liveness), 2 = bounded_\n/// eventually (full class), 3 = until (full class).\nconst PROP_KINDS: &[u8] = &{:?};\n",
        spec.properties
            .iter()
            .map(|p| match p.kind {
                crate::mc::smt_plan::SmtPropKind::Always => 0u8,
                crate::mc::smt_plan::SmtPropKind::Eventually => 1,
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(_) => 2,
                crate::mc::smt_plan::SmtPropKind::Until => 3,
            })
            .collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "/// always[N] anchor clip (0 = unbounded).\nconst PROP_BOUNDS: &[usize] = &{:?};\n",
        spec.properties
            .iter()
            .map(|p| p.bound.unwrap_or(0))
            .collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "/// Maximum `next` nesting depth (properties are checked only at\n/// steps i with i + depth < k).\nconst PROP_NEXT_DEPTH: &[usize] = &{:?};\n",
        spec.properties
            .iter()
            .map(|p| p.next_depth)
            .collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "/// FULL class (checked only with --full-properties).\nconst PROP_FULL: &[bool] = &{:?};\n",
        spec.properties.iter().map(|p| p.full).collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "/// bounded_eventually[N] horizon N (0 = not a bounded_eventually).\nconst PROP_BE_BOUND: &[usize] = &{:?};\n",
        spec.properties
            .iter()
            .map(|p| match p.kind {
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(n) => n,
                _ => 0,
            })
            .collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "const N_INPUT_ATOMS: usize = {};\nconst N_OUTPUT_ATOMS: usize = {};\n",
        spec.input_atoms.len(),
        spec.output_atoms.len()
    ));

    s.push_str("\nuse z3::ast::{BV, Bool};\n\n");

    // Transition guards (builder takes the step index; leaves are recreated
    // constants - hash-consed to the engine's step variables).
    for (t, tr) in spec.transitions.iter().enumerate() {
        let mut r = RustRender::new();
        let uses_i = tr.guard.as_ref().is_some_and(se_uses_step);
        match &tr.guard {
            Some(g) => render_se_rust(g, spec, Some(1), Some(tr.classes[0]), false, "i", &mut r),
            None => r.push("let e0: Bool = Bool::from_bool(true);".into()),
        };
        s.push_str(&format!(
            "fn expr_t{t}_guard({p}: usize) -> Bool {{\n    {}\n    e{}\n}}\n\n",
            r.lines.join("\n    "),
            r.n - 1,
            p = if uses_i { "i" } else { "_i" }
        ));
    }
    // Transition assignments (per var; `asgn_dummy` fills unused table slots -
    // never called, the engine checks ASGN_MAP first). Emitted only when some
    // slot needs it (warning-free generated code).
    let dummy_needed = spec
        .transitions
        .iter()
        .any(|t| (0..spec.vars.len()).any(|j| !t.assignments.iter().any(|(vi, _)| *vi == j)));
    if dummy_needed {
        s.push_str("fn asgn_dummy(_i: usize) -> BV { BV::from_u64(0u64, 1) }\n\n");
    }
    for (t, tr) in spec.transitions.iter().enumerate() {
        for j in 0..spec.vars.len() {
            if let Some((_, expr)) = tr.assignments.iter().find(|(vi, _)| *vi == j) {
                let mut r = RustRender::new();
                let uses_i = se_uses_step(expr);
                render_se_rust(
                    expr,
                    spec,
                    Some(spec.vars[j].width),
                    Some(tr.classes[0]),
                    true,
                    "i",
                    &mut r,
                );
                s.push_str(&format!(
                    "fn expr_t{t}_asgn_{j}({p}: usize) -> BV {{\n    {}\n    e{}\n}}\n\n",
                    r.lines.join("\n    "),
                    r.n - 1,
                    p = if uses_i { "i" } else { "_i" }
                ));
            }
        }
    }
    // Property formulas. Every property fn takes (step, horizon) so FULL-class
    // expansions can reference `k`; default-class bodies never do. Properties
    // the engine routes to expand_p{p} (FULL class + top-level
    // bounded_eventually, whose check IS the expansion) get a dummy body here.
    for (p, prop) in spec.properties.iter().enumerate() {
        let routed = prop.full
            || matches!(
                prop.kind,
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(_)
            );
        if routed {
            s.push_str(&format!(
                "fn expr_p{p}(_i: usize, _k: usize, _alive: &[Bool], _ii_steps: &[Vec<Bool>], _oh_steps: &[Vec<Bool>]) -> Bool {{ Bool::from_bool(true) }}\n\n"
            ));
            continue;
        }
        let mut r = RustRender::new();
        let uses_i = sf_uses_step(&prop.formula);
        render_sf_rust(&prop.formula, spec, "i", &mut r);
        // Atom parameters are named (no underscore) only when the body
        // references them - unused names would warn.
        let ii_name = if sf_uses_input_atoms(&prop.formula) {
            "ii_steps"
        } else {
            "_ii_steps"
        };
        let oh_name = if sf_uses_output_atoms(&prop.formula) {
            "oh_steps"
        } else {
            "_oh_steps"
        };
        // The alive parameter is named (no underscore) only when the body has a
        // next node (the D1 gate references it) - unused names would warn.
        let alive_name = if sf_uses_next(&prop.formula) {
            "alive"
        } else {
            "_alive"
        };
        s.push_str(&format!(
            "fn expr_p{p}({q}: usize, _k: usize, {alive_name}: &[Bool], {ii_name}: &[Vec<Bool>], {oh_name}: &[Vec<Bool>]) -> Bool {{\n    {}\n    e{}\n}}\n\n",
            r.lines.join("\n    "),
            r.n - 1,
            q = if uses_i { "i" } else { "_i" }
        ));
    }
    // Expansions for routed properties (FULL class + top-level
    // bounded_eventually): the whole formula rendered with the horizon `k`
    // in scope (temporal nodes emit horizon-truncated loops). Top-level BE
    // stores only its body in `formula` (the operator lives in `kind`), so
    // wrap it in a synthetic node to get the clipped OR loop.
    for (p, prop) in spec.properties.iter().enumerate() {
        let routed = prop.full
            || matches!(
                prop.kind,
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(_)
            );
        if !routed {
            continue;
        }
        let formula = match &prop.kind {
            crate::mc::smt_plan::SmtPropKind::BoundedEventually(n) => Sf::BoundedEventually {
                bound: *n,
                inner: Box::new(prop.formula.clone()),
            },
            _ => prop.formula.clone(),
        };
        let mut r = RustRender::new();
        // The renderer returns the final binding's name - loop-closing nodes
        // (BE/until) end with `}`, so r.n-1 would not be a binding.
        let fin = render_sf_rust(&formula, spec, "i", &mut r);
        let ii_name = if sf_uses_input_atoms(&formula) {
            "ii_steps"
        } else {
            "_ii_steps"
        };
        let oh_name = if sf_uses_output_atoms(&formula) {
            "oh_steps"
        } else {
            "_oh_steps"
        };
        let alive_name = if sf_uses_next(&formula) {
            "alive"
        } else {
            "_alive"
        };
        s.push_str(&format!(
            "fn expand_p{p}(i: usize, k: usize, {alive_name}: &[Bool], {ii_name}: &[Vec<Bool>], {oh_name}: &[Vec<Bool>]) -> Bool {{\n    {}\n    {fin}\n}}\n\n",
            r.lines.join("\n    ")
        ));
    }
    // input_is/output_has atom builders (called from the unrolling loop to
    // define the ii_/oh_ named constants). Specs without atoms get trivial
    // dummies - the engine's loops are compiled out by the N_*_ATOMS == 0
    // guards, but the functions must exist for the loop bodies to type-check.
    // With on_error the unrolling loop calls build_input_atom_ev instead, so
    // the plain builder would be dead code - emit it only without on_error.
    if spec.on_error.is_some() {
        // no plain input-atom builder (the _ev variant is emitted in the
        // on_error section below)
    } else if spec.input_atoms.is_empty() {
        // The ev parameter exists only for multi-class specs (task 3D) - the
        // call site passes it only then, keeping single-class codegen
        // byte-identical to before.
        let ev_param = if spec.classes.len() > 1 {
            ", _ev_vars: Option<&[BV]>"
        } else {
            ""
        };
        s.push_str(&format!(
            "fn build_input_atom(_a: usize, _i: usize, _fires: &[Bool], _fld_vars: &[Vec<BV>]{ev_param}) -> Bool {{\n    Bool::from_bool(false)\n}}\n\n"
        ));
    } else {
        let ev_param = if spec.classes.len() > 1 {
            ", ev_vars: Option<&[BV]>"
        } else {
            ""
        };
        s.push_str(&format!(
            "fn build_input_atom(a: usize, i: usize, fires: &[Bool], fld_vars: &[Vec<BV>]{ev_param}) -> Bool {{\n"
        ));
        for (a, arms) in spec.input_atoms.iter().enumerate() {
            // if / else-if chain: an if-without-else in statement position
            // must return (), so the dispatch is one expression.
            s.push_str(&format!(
                "    {} a == {a} {{\n",
                if a == 0 { "if" } else { "} else if" }
            ));
            s.push_str("        let mut val = Bool::from_bool(false);\n");
            for (ci, constraints) in arms {
                s.push_str("        {\n");
                s.push_str("            let mut cls_fire = Bool::from_bool(false);\n");
                // Transitions covering this arm's class (task 3D): a
                // multi-variant wildcard transition covers every class in its
                // `classes` - the index list is precomputed at emission.
                let covering: Vec<String> = spec
                    .transitions
                    .iter()
                    .enumerate()
                    .filter(|(_, tr)| tr.classes.contains(ci))
                    .map(|(t, _)| t.to_string())
                    .collect();
                if covering.is_empty() {
                    // ev_vars joins the suppression only on multi-class specs
                    // (the parameter exists only then).
                    let ev_sup = if spec.classes.len() > 1 {
                        ", ev_vars"
                    } else {
                        ""
                    };
                    s.push_str(&format!(
                        "            let _ = (i, fires, fld_vars{ev_sup}); // no transition covers this class\n"
                    ));
                } else {
                    for t in &covering {
                        s.push_str(&format!(
                            "            cls_fire = b_or(&cls_fire, &fires[{t}]);\n"
                        ));
                    }
                }
                // The actual-input class conjunct (task 3D): entailed by
                // fires[t] for single-class transitions, but a multi-variant
                // wildcard fires on ANY of its classes - without it every
                // covered arm would be true at once and input_is could hold
                // for the variant that did NOT come in (masking violations).
                let mut arm = if spec.classes.len() > 1 {
                    s.push_str(&format!(
                        "            let ccm: Bool = ev_vars.expect(\"event vars\")[i].eq(&BV::from_u64({ci}u64, CLS_BITS));\n"
                    ));
                    "b_and(&cls_fire, &ccm)".to_string()
                } else {
                    "cls_fire".to_string()
                };
                for (pos, (g, v, _isb)) in constraints.iter().enumerate() {
                    s.push_str(&format!(
                        "            let c{pos}: Bool = fld_vars[i][{g}].eq(&BV::from_u64({v}u64, FIELD_WIDTHS[{g}]));\n"
                    ));
                    arm = format!("b_and(&{arm}, &c{pos})");
                }
                s.push_str(&format!("            val = b_or(&val, &{arm});\n"));
                s.push_str("        }\n");
            }
            s.push_str("        val\n");
        }
        s.push_str("    } else {\n        Bool::from_bool(false)\n    }\n}\n\n");
    }
    if spec.output_atoms.is_empty() {
        s.push_str(
            "fn build_output_atom(_a: usize, _i: usize, _fires: &[Bool]) -> Bool {\n    Bool::from_bool(false)\n}\n\n",
        );
    } else {
        // Flatten every (transition, emit) pair into tables; each pair's
        // argument expressions become one step function returning a Vec<BV>,
        // dispatched through a function-pointer table.
        let mut pair_t: Vec<usize> = Vec::new();
        let mut pair_group: Vec<String> = Vec::new();
        let mut pair_variant: Vec<String> = Vec::new();
        let mut pair_argw: Vec<Vec<u32>> = Vec::new();
        for (t, tr) in spec.transitions.iter().enumerate() {
            for o in &tr.emits {
                pair_t.push(t);
                pair_group.push(o.group.clone());
                pair_variant.push(match &o.variant {
                    Some(v) => format!("Some(\"{v}\")"),
                    None => "None".to_string(),
                });
                pair_argw.push(o.args.iter().map(|a| se_type(a, spec).width()).collect());
            }
        }
        s.push_str(&format!("const EMIT_PAIR_T: &[usize] = &{:?};\n", pair_t));
        s.push_str(&format!(
            "const EMIT_PAIR_GROUP: &[&str] = &{};\n",
            rust_str_list(&pair_group)
        ));
        s.push_str(&format!(
            "const EMIT_PAIR_VARIANT: &[Option<&str>] = &[{}];\n",
            pair_variant.join(", ")
        ));
        s.push_str(&format!(
            "const EMIT_ARGW: &[&[u32]] = &[{}];\n",
            pair_argw
                .iter()
                .map(|row| format!("&{:?}", row))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        let mut arg_fns = String::new();
        let mut fn_names: Vec<String> = Vec::new();
        for (pi, tr) in spec.transitions.iter().enumerate() {
            for (oi, o) in tr.emits.iter().enumerate() {
                if o.args.is_empty() {
                    arg_fns.push_str("fn emit_args_dummy(_i: usize) -> Vec<BV> { Vec::new() }\n\n");
                    fn_names.push("emit_args_dummy".to_string());
                    continue;
                }
                let mut body = String::new();
                for arg in &o.args {
                    // Each argument renders with a FRESH renderer (bindings
                    // restart at e0), so the statements must be block-scoped
                    // or multi-argument emits would redefine e0.
                    let mut r = RustRender::new();
                    render_se_rust(
                        arg,
                        spec,
                        Some(se_type(arg, spec).width()),
                        Some(tr.classes[0]),
                        true,
                        "i",
                        &mut r,
                    );
                    body.push_str(&format!(
                        "    v.push({{{lines}    e{n}\n    }});\n",
                        lines = r.lines.join("\n    "),
                        n = r.n - 1
                    ));
                }
                arg_fns.push_str(&format!(
                    "fn emit_args_{pi}_{oi}(i: usize) -> Vec<BV> {{\n    let mut v: Vec<BV> = Vec::new();\n{body}    v\n}}\n\n",
                ));
                fn_names.push(format!("emit_args_{pi}_{oi}"));
            }
        }
        s.push_str(&arg_fns);
        s.push_str(&format!(
            "const EMIT_ARGS_FNS: &[fn(usize) -> Vec<BV>] = &[{}];\n",
            fn_names.join(", ")
        ));
        // Transition-outer structure mirrors the ref engine's operation order
        // exactly (fires[t] AND OR-over-emits match) - parity-critical: a
        // flattened pair loop would build an equivalent-but-differently-shaped
        // formula and could diverge on multi-emit transitions.
        s.push_str("fn build_output_atom(a: usize, i: usize, fires: &[Bool]) -> Bool {\n");
        for (a, (group, variant, constraints)) in spec.output_atoms.iter().enumerate() {
            // if / else-if chain (see build_input_atom).
            s.push_str(&format!(
                "    {} a == {a} {{\n",
                if a == 0 { "if" } else { "} else if" }
            ));
            s.push_str("        let mut val = Bool::from_bool(false);\n");
            s.push_str("        for t in 0..TRANS.len() {\n");
            s.push_str("            let mut emit_match = Bool::from_bool(false);\n");
            s.push_str("            for pi in 0..EMIT_PAIR_T.len() {\n");
            s.push_str("                if EMIT_PAIR_T[pi] != t { continue; }\n");
            // A group-level pattern (variant None) matches ANY variant of the
            // group - exact mode's output_matches rule. The relaxed `true`
            // check is emitted only when the group actually has variant emits
            // (unit-output groups keep the byte-identical `== None` form).
            let variant_check = match variant {
                Some(v) => format!("EMIT_PAIR_VARIANT[pi] == Some(\"{v}\")"),
                None => {
                    if spec.transitions.iter().any(|t| {
                        t.emits
                            .iter()
                            .any(|o| o.group == *group && o.variant.is_some())
                    }) {
                        "true".to_string()
                    } else {
                        "EMIT_PAIR_VARIANT[pi] == None".to_string()
                    }
                }
            };
            s.push_str(&format!(
                "                if EMIT_PAIR_GROUP[pi] == \"{group}\" && {variant_check} {{\n"
            ));
            s.push_str("                    let args = EMIT_ARGS_FNS[pi](i);\n");
            s.push_str("                    let mut m = Bool::from_bool(true);\n");
            for (pos, v, _isb) in constraints {
                s.push_str(&format!(
                    "                    m = b_and(&m, &args[{pos}].eq(&BV::from_u64({v}u64, EMIT_ARGW[pi][{pos}])));\n"
                ));
            }
            s.push_str("                    emit_match = b_or(&emit_match, &m);\n");
            s.push_str("                }\n            }\n");
            s.push_str("            val = b_or(&val, &b_and(&fires[t], &emit_match));\n");
            s.push_str("        }\n");
            s.push_str("        val\n");
        }
        s.push_str("    } else {\n        Bool::from_bool(false)\n    }\n}\n\n");
    }
    // ASGN_MAP[t][j]: does transition t assign var j?
    let mut map_rows = Vec::new();
    for t in 0..spec.transitions.len() {
        let row: Vec<String> = (0..spec.vars.len())
            .map(|j| {
                spec.transitions[t]
                    .assignments
                    .iter()
                    .any(|(vi, _)| *vi == j)
                    .to_string()
            })
            .collect();
        map_rows.push(format!("&[{}]", row.join(", ")));
    }
    s.push_str(&format!(
        "const ASGN_MAP: &[&[bool]] = &[{}];\n\n",
        map_rows.join(", ")
    ));
    // Function-pointer tables (the engine loops over these).
    let guards: Vec<String> = (0..spec.transitions.len())
        .map(|t| format!("expr_t{t}_guard"))
        .collect();
    s.push_str(&format!(
        "fn guard_fns() -> Vec<fn(usize) -> Bool> {{ vec![{}] }}\n\n",
        guards.join(", ")
    ));
    let mut asgn_rows = Vec::new();
    for t in 0..spec.transitions.len() {
        let row: Vec<String> = (0..spec.vars.len())
            .map(|j| {
                if spec.transitions[t]
                    .assignments
                    .iter()
                    .any(|(vi, _)| *vi == j)
                {
                    format!("expr_t{t}_asgn_{j}")
                } else {
                    "asgn_dummy".to_string()
                }
            })
            .collect();
        asgn_rows.push(format!("vec![{}]", row.join(", ")));
    }
    s.push_str(&format!(
        "fn asgn_fns() -> Vec<Vec<fn(usize) -> BV>> {{ vec![{}] }}\n\n",
        asgn_rows.join(", ")
    ));
    let props: Vec<String> = (0..spec.properties.len())
        .map(|p| format!("expr_p{p}"))
        .collect();
    s.push_str(&format!(
        "fn prop_fns() -> Vec<fn(usize, usize, &[Bool], &[Vec<Bool>], &[Vec<Bool>]) -> Bool> {{ vec![{}] }}\n",
        props.join(", ")
    ));
    // Expansion table: routed properties (FULL class + top-level
    // bounded_eventually) point at expand_p{p}; the rest are dummies (never
    // called - the engine dispatches on PROP_KINDS).
    let expands: Vec<String> = spec
        .properties
        .iter()
        .enumerate()
        .map(|(p, prop)| {
            if prop.full
                || matches!(
                    prop.kind,
                    crate::mc::smt_plan::SmtPropKind::BoundedEventually(_)
                )
            {
                format!("expand_p{p}")
            } else {
                "expand_dummy".to_string()
            }
        })
        .collect();
    // The dummy is emitted only when some slot needs it (warning-free code).
    if expands.iter().any(|e| e == "expand_dummy") {
        s.push_str("fn expand_dummy(_i: usize, _k: usize, _alive: &[Bool], _ii_steps: &[Vec<Bool>], _oh_steps: &[Vec<Bool>]) -> Bool { Bool::from_bool(true) }\n\n");
    }
    s.push_str(&format!(
        "fn expand_fns() -> Vec<fn(usize, usize, &[Bool], &[Vec<Bool>], &[Vec<Bool>]) -> Bool> {{ vec![{}] }}\n",
        expands.join(", ")
    ));

    // ---- on_error fallback (task 3C, D4) - emitted ONLY when the module has
    // one, so specs without it generate byte-identical code to before. ----
    if let Some(oe) = &spec.on_error {
        s.push_str(&format!(
            "/// on_error fallback (task 3C): catch-all lowest-priority transition\n/// from ANY state - no pattern bindings, no variable assignments.\nconst ON_ERROR_NEXT_STATE: usize = {};\n",
            oe.next_state
        ));
        // The fallback's emits: pair tables + argument step functions (same
        // structure as the transition emit tables).
        if oe.emits.is_empty() {
            s.push_str("const OE_EMIT_GROUP: &[&str] = &[];\nconst OE_EMIT_VARIANT: &[Option<&str>] = &[];\nconst OE_EMIT_ARGW: &[&[u32]] = &[];\n");
            s.push_str(
                "fn oe_emit_args_dummy(_i: usize) -> Vec<BV> { Vec::new() }\nconst OE_EMIT_ARGS_FNS: &[fn(usize) -> Vec<BV>] = &[oe_emit_args_dummy];\n\n",
            );
        } else {
            let groups: Vec<String> = oe.emits.iter().map(|o| o.group.clone()).collect();
            let variants: Vec<String> = oe
                .emits
                .iter()
                .map(|o| match &o.variant {
                    Some(v) => format!("Some(\"{v}\")"),
                    None => "None".to_string(),
                })
                .collect();
            let argws: Vec<Vec<u32>> = oe
                .emits
                .iter()
                .map(|o| o.args.iter().map(|a| se_type(a, spec).width()).collect())
                .collect();
            s.push_str(&format!(
                "const OE_EMIT_GROUP: &[&str] = &{};\n",
                rust_str_list(&groups)
            ));
            s.push_str(&format!(
                "const OE_EMIT_VARIANT: &[Option<&str>] = &[{}];\n",
                variants.join(", ")
            ));
            s.push_str(&format!(
                "const OE_EMIT_ARGW: &[&[u32]] = &[{}];\n",
                argws
                    .iter()
                    .map(|row| format!("&{:?}", row))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            let mut arg_fns = String::new();
            let mut fn_names: Vec<String> = Vec::new();
            for (oi, o) in oe.emits.iter().enumerate() {
                if o.args.is_empty() {
                    arg_fns
                        .push_str("fn oe_emit_args_dummy(_i: usize) -> Vec<BV> { Vec::new() }\n\n");
                    fn_names.push("oe_emit_args_dummy".to_string());
                    continue;
                }
                let mut body = String::new();
                for arg in &o.args {
                    let mut r = RustRender::new();
                    render_se_rust(
                        arg,
                        spec,
                        Some(se_type(arg, spec).width()),
                        None,
                        true,
                        "i",
                        &mut r,
                    );
                    body.push_str(&format!(
                        "    v.push({{{lines}    e{n}\n    }});\n",
                        lines = r.lines.join("\n    "),
                        n = r.n - 1
                    ));
                }
                arg_fns.push_str(&format!(
                    "fn oe_emit_args_{oi}(i: usize) -> Vec<BV> {{\n    let mut v: Vec<BV> = Vec::new();\n{body}    v\n}}\n\n",
                ));
                fn_names.push(format!("oe_emit_args_{oi}"));
            }
            s.push_str(&arg_fns);
            s.push_str(&format!(
                "const OE_EMIT_ARGS_FNS: &[fn(usize) -> Vec<BV>] = &[{}];\n\n",
                fn_names.join(", ")
            ));
        }
        // input_is evaluated against the ACTUAL input (ev_i + payload fields)
        // - with on_error an unmatched input is still consumed (exact mode
        // records it in the on_error sample, SEMANTICS 3.6). Specs without
        // input atoms get a trivial dummy (the engine's loops are compiled
        // out by the N_INPUT_ATOMS == 0 guards).
        if spec.input_atoms.is_empty() {
            s.push_str(
                "fn build_input_atom_ev(_a: usize, _i: usize, _ev_vars: Option<&[BV]>, _fld_vars: &[Vec<BV>]) -> Bool {\n    Bool::from_bool(false)\n}\n\n",
            );
        } else {
            s.push_str("fn build_input_atom_ev(a: usize, i: usize, ev_vars: Option<&[BV]>, fld_vars: &[Vec<BV>]) -> Bool {\n");
            for (a, arms) in spec.input_atoms.iter().enumerate() {
                s.push_str(&format!(
                    "    {} a == {a} {{\n",
                    if a == 0 { "if" } else { "} else if" }
                ));
                s.push_str("        let mut val = Bool::from_bool(false);\n");
                for (ci, constraints) in arms {
                    s.push_str("        {\n");
                    if spec.classes.len() > 1 {
                        s.push_str(&format!(
                        "            let mut arm: Bool = ev_vars.expect(\"event vars\")[i].eq(&BV::from_u64({ci}u64, CLS_BITS));\n"
                    ));
                    } else {
                        s.push_str("            let mut arm: Bool = Bool::from_bool(true);\n");
                    }
                    for (pos, (g, v, _isb)) in constraints.iter().enumerate() {
                        s.push_str(&format!(
                        "            let c{pos}: Bool = fld_vars[i][{g}].eq(&BV::from_u64({v}u64, FIELD_WIDTHS[{g}]));\n"
                    ));
                        s.push_str(&format!("            arm = b_and(&arm, &c{pos});\n"));
                    }
                    s.push_str("            val = b_or(&val, &arm);\n");
                    s.push_str("        }\n");
                }
                s.push_str("        val\n");
            }
            s.push_str("    } else {\n        Bool::from_bool(false)\n    }\n}\n\n");
        }
        // Whether the fallback's emits match an output_has atom definition at
        // step i (group-level patterns match ANY variant of the group - exact
        // mode's output_matches rule).
        if spec.output_atoms.is_empty() {
            s.push_str(
                "fn build_on_error_emit_match(_a: usize, _i: usize) -> Bool {\n    Bool::from_bool(false)\n}\n\n",
            );
        } else {
            s.push_str("fn build_on_error_emit_match(a: usize, i: usize) -> Bool {\n");
            for (a, (group, variant, constraints)) in spec.output_atoms.iter().enumerate() {
                s.push_str(&format!(
                    "    {} a == {a} {{\n",
                    if a == 0 { "if" } else { "} else if" }
                ));
                s.push_str("        let mut val = Bool::from_bool(false);\n");
                s.push_str("        for pi in 0..OE_EMIT_GROUP.len() {\n");
                s.push_str(&format!(
                    "            if OE_EMIT_GROUP[pi] == \"{group}\" && {} {{\n",
                    match variant {
                        Some(v) => format!("OE_EMIT_VARIANT[pi] == Some(\"{v}\")"),
                        None => "true".to_string(),
                    }
                ));
                s.push_str("                let args = OE_EMIT_ARGS_FNS[pi](i);\n");
                s.push_str("                let mut m = Bool::from_bool(true);\n");
                for (pos, v, _isb) in constraints {
                    s.push_str(&format!(
                    "                m = b_and(&m, &args[{pos}].eq(&BV::from_u64({v}u64, OE_EMIT_ARGW[pi][{pos}])));\n"
                ));
                }
                s.push_str("                val = b_or(&val, &m);\n");
                s.push_str("            }\n        }\n");
                s.push_str("        val\n");
            }
            s.push_str("    } else {\n        Bool::from_bool(false)\n    }\n}\n\n");
        }
    }

    // ---- Assumptions (task 3C, D3) - emitted ONLY when the module has any,
    // so specs without them generate byte-identical code to before. ----
    if !spec.assumptions.is_empty() {
        let anames: Vec<String> = spec.assumptions.iter().map(|a| a.name.clone()).collect();
        s.push_str(&format!(
            "/// Assumptions (task 3C): per-sample base constraints - exact-mode\n/// successor pruning expressed as SMT facts (SEMANTICS 7.x).\nconst ASSUME_NAMES: &[&str] = &{};\n",
            rust_str_list(&anames)
        ));
        s.push_str(&format!(
            "/// FULL class (asserted only with --full-assumptions).\nconst ASSUME_FULL: &[bool] = &{:?};\n",
            spec.assumptions.iter().map(|a| a.full).collect::<Vec<_>>()
        ));
        s.push_str(&format!(
            "/// always[N] sample clip (0 = all samples).\nconst ASSUME_BOUNDS: &[usize] = &{:?};\n",
            spec.assumptions.iter().map(|a| a.bound.unwrap_or(0)).collect::<Vec<_>>()
        ));
        s.push_str(&format!(
            "/// next_depth anchor clip: the last N anchors have no shifted horizon and are dropped (same clipping as properties).\nconst ASSUME_NEXT_DEPTH: &[usize] = &{:?};\n",
            spec.assumptions.iter().map(|a| a.next_depth).collect::<Vec<_>>()
        ));
        // One builder fn per assumption - same signature as the property fns
        // (FULL-class bodies render horizon loops that need k; next bodies
        // need the D1 alive gate).
        for (a, ass) in spec.assumptions.iter().enumerate() {
            let mut r = RustRender::new();
            let fin = render_sf_rust(&ass.formula, spec, "i", &mut r);
            let i_name = if sf_uses_step(&ass.formula) {
                "i"
            } else {
                "_i"
            };
            let k_name = if sf_uses_horizon(&ass.formula) {
                "k"
            } else {
                "_k"
            };
            let alive_name = if sf_uses_next(&ass.formula) {
                "alive"
            } else {
                "_alive"
            };
            let ii_name = if sf_uses_input_atoms(&ass.formula) {
                "ii_steps"
            } else {
                "_ii_steps"
            };
            let oh_name = if sf_uses_output_atoms(&ass.formula) {
                "oh_steps"
            } else {
                "_oh_steps"
            };
            s.push_str(&format!(
                "fn assume_f{a}({i_name}: usize, {k_name}: usize, {alive_name}: &[Bool], {ii_name}: &[Vec<Bool>], {oh_name}: &[Vec<Bool>]) -> Bool {{\n    {}\n    {fin}\n}}\n\n",
                r.lines.join("\n    ")
            ));
        }
        s.push_str(&format!(
            "fn assume_fns() -> Vec<fn(usize, usize, &[Bool], &[Vec<Bool>], &[Vec<Bool>]) -> Bool> {{ vec![{}] }}\n",
            (0..spec.assumptions.len()).map(|a| format!("assume_f{a}")).collect::<Vec<_>>().join(", ")
        ));
    }
    s
}

// ---------------------------------------------------------------------------
// C emission
// ---------------------------------------------------------------------------

struct CRender {
    lines: Vec<String>,
    n: usize,
}

impl CRender {
    fn new() -> Self {
        Self {
            lines: Vec::new(),
            n: 0,
        }
    }
    fn push(&mut self, code: String) -> String {
        let name = format!("e{}", self.n);
        self.n += 1;
        self.lines.push(code);
        name
    }
}

/// Width at which a shift amount is rendered. Exact-MC semantics use the
/// amount's FULL value (`amount >= wl -> 0`), so the z3 encoding must keep it
/// detectable: bitvectors keep their own width; integer literals lift to at
/// least `wl` and to enough bits to hold the value (no truncation).
fn shift_amt_width(rr: &Se, spec: &SmtSpec, wl: u32) -> u32 {
    match se_type(rr, spec) {
        SeType::Bit(wr) => wr,
        SeType::IntLit => {
            let v = match rr {
                Se::Lit { value, .. } => *value,
                _ => 0,
            };
            // Task 3L: literal values are u128 (full-precision folded constants).
            let bits = if v == 0 { 1 } else { 128 - v.leading_zeros() };
            wl.max(bits as u32)
        }
        _ => 8,
    }
}

/// Shift rendering (both backends share this algorithm): zero-extend the
/// amount to `w_c = max(amount width, wl)`, guard with `amount < wl`
/// (RFS: larger amounts yield 0), truncate the amount to `wl` bits for the
/// z3 shift op (lossless under the guard).
fn render_shift_rust(
    op: Sb,
    l: &Se,
    rr: &Se,
    spec: &SmtSpec,
    class: Option<usize>,
    step: &str,
    r: &mut RustRender,
) -> String {
    // Task 3L: the result width is the EXPRESSION's type - the shifted
    // value's width, or the shift amount's width when the value is a constant
    // (the refinement). The left operand's width would render a constant-value
    // shift at 8 bits and mismatch the context in the parent ite.
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    let wl = match (lt, rt) {
        (SeType::IntLit, SeType::Bit(b)) => b,
        (SeType::Bit(a), _) => a,
        _ => 8,
    };
    let w_r = shift_amt_width(rr, spec, wl);
    let w_c = w_r.max(wl);
    let a = render_se_rust(l, spec, Some(wl), class, false, step, r);
    let b = render_se_rust(rr, spec, Some(w_r), class, false, step, r);
    let b_c = if w_c == w_r {
        b.clone()
    } else {
        let z = r.push(format!(
            "let e{}: BV = BV::from_u64(0u64, {});",
            r.n,
            w_c - w_r
        ));
        r.push(format!("let e{}: BV = {z}.concat(&{b});", r.n))
    };
    let glit = r.push(format!("let e{}: BV = BV::from_u64({wl}u64, {w_c});", r.n));
    let cond = r.push(format!("let e{}: Bool = {b_c}.bvult(&{glit});", r.n));
    let amt = if w_c == wl {
        b_c.clone()
    } else {
        r.push(format!(
            "let e{}: BV = {b_c}.extract({}u32, 0);",
            r.n,
            wl - 1
        ))
    };
    let zero = r.push(format!("let e{}: BV = BV::from_u64(0u64, {wl});", r.n));
    let m = if op == Sb::ShiftLeft {
        "bvshl"
    } else {
        "bvlshr"
    };
    r.push(format!(
        "let e{}: BV = {cond}.ite(&{a}.{m}(&{amt}), &{zero});",
        r.n
    ))
}

/// C mirror of [`render_shift_rust`].
fn render_shift_c(
    op: Sb,
    l: &Se,
    rr: &Se,
    spec: &SmtSpec,
    class: Option<usize>,
    step: &str,
    r: &mut CRender,
) -> String {
    // Task 3L: the result width is the EXPRESSION's type - the shifted
    // value's width, or the shift amount's width when the value is a constant
    // (the refinement). The left operand's width would render a constant-value
    // shift at 8 bits and mismatch the context in the parent ite.
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    let wl = match (lt, rt) {
        (SeType::IntLit, SeType::Bit(b)) => b,
        (SeType::Bit(a), _) => a,
        _ => 8,
    };
    let w_r = shift_amt_width(rr, spec, wl);
    let w_c = w_r.max(wl);
    let a = render_se_c(l, spec, Some(wl), class, false, step, r);
    let b = render_se_c(rr, spec, Some(w_r), class, false, step, r);
    let b_c = if w_c == w_r {
        b.clone()
    } else {
        let z = r.push(format!("Z3_ast e{} = z3_num(c, {}, 0ull);", r.n, w_c - w_r));
        r.push(format!("Z3_ast e{} = Z3_mk_concat(c, {z}, {b});", r.n))
    };
    let glit = r.push(format!("Z3_ast e{} = z3_num(c, {w_c}, {wl}ull);", r.n));
    let cond = r.push(format!("Z3_ast e{} = Z3_mk_bvult(c, {b_c}, {glit});", r.n));
    let amt = if w_c == wl {
        b_c.clone()
    } else {
        r.push(format!(
            "Z3_ast e{} = Z3_mk_extract(c, {}u, 0u, {b_c});",
            r.n,
            wl - 1
        ))
    };
    let zero = r.push(format!("Z3_ast e{} = z3_num(c, {wl}, 0ull);", r.n));
    let m = if op == Sb::ShiftLeft {
        "bvshl"
    } else {
        "bvlshr"
    };
    r.push(format!(
        "Z3_ast e{} = Z3_mk_ite(c, {cond}, Z3_mk_{m}(c, {a}, {amt}), {zero});",
        r.n
    ))
}

/// C mirror of [`render_se_rust`]: same structure, z3 C API calls.
fn render_se_c(
    se: &Se,
    spec: &SmtSpec,
    expected: Option<u32>,
    class: Option<usize>,
    bool_as_bv: bool,
    step: &str,
    r: &mut CRender,
) -> String {
    // Base step ("i") uses the caller's arrays; any other expression renders
    // name-based fresh constants at that (runtime) step.
    let base = step == "i";
    match se {
        Se::Lit { value, is_bool } => {
            if *is_bool && !bool_as_bv {
                r.push(format!(
                    "Z3_ast e{} = Z3_mk_{}(c);",
                    r.n,
                    if *value != 0 { "true" } else { "false" }
                ))
            } else {
                // Task 3L: a folded full-precision literal that does not fit the
                // context width is masked (two's complement); an in-range literal
                // renders exactly as before (byte-identical pins preserved). C
                // widths cap at 64 (audit row C1).
                let w = expected.unwrap_or(8);
                let m = if w >= 64 { u64::MAX } else { (1u64 << w) - 1 };
                if *value <= m as u128 {
                    r.push(format!(
                        "Z3_ast e{} = z3_num(c, {w}, {}ull);",
                        r.n, *value as u64
                    ))
                } else {
                    r.push(format!(
                        "Z3_ast e{} = z3_num(c, {w}, ({}u128 & {}) as u64);",
                        r.n, value, m
                    ))
                }
            }
        }
        // Explicit conversion cast (task 3L): zero-extension / truncation of the
        // operand to `width` bits.
        Se::Cast { width, inner } => {
            let c = render_se_c(inner, spec, Some(*width), class, false, step, r);
            let m = if *width >= 64 {
                u64::MAX
            } else {
                (1u64 << width) - 1
            };
            r.push(format!(
                "Z3_ast e{} = Z3_mk_bvand(c, {c}, z3_num(c, {width}u, {}ull));",
                r.n, m
            ))
        }
        Se::State => {
            if base {
                r.push(format!("Z3_ast e{} = s;", r.n))
            } else {
                r.push(format!(
                    "char nm{}[32]; snprintf(nm{}, sizeof nm{}, \"s_%u\", {step}); Z3_ast e{} = Z3_mk_const(c, Z3_mk_string_symbol(c, nm{}), Z3_mk_bv_sort(c, STATE_BITS));",
                    r.n, r.n, r.n, r.n, r.n
                ))
            }
        }
        Se::Var(j) => {
            let leaf = if base {
                format!("v[{j}]")
            } else {
                let idx = r.n;
                r.push(format!(
                    "char nm{idx}[32]; snprintf(nm{idx}, sizeof nm{idx}, \"v_%u_{j}\", {step});"
                ));
                format!("Z3_mk_const(c, Z3_mk_string_symbol(c, nm{idx}), Z3_mk_bv_sort(c, VAR_WIDTHS[{j}]))")
            };
            if spec.vars[*j].is_bool && !bool_as_bv {
                let l = r.push(format!("Z3_ast e{} = {leaf};", r.n));
                r.push(format!(
                    "Z3_ast e{} = Z3_mk_eq(c, {l}, z3_num(c, 1, 1ull));",
                    r.n
                ))
            } else {
                r.push(format!("Z3_ast e{} = {leaf};", r.n))
            }
        }
        Se::Field { idx, is_bool, .. } => {
            let g = global_field_index(spec, class.expect("field outside transition"), *idx);
            let leaf = if base {
                format!("fld[{g}]")
            } else {
                let idx = r.n;
                r.push(format!(
                    "char nm{idx}[32]; snprintf(nm{idx}, sizeof nm{idx}, \"fld_%u_{g}\", {step});"
                ));
                format!("Z3_mk_const(c, Z3_mk_string_symbol(c, nm{idx}), Z3_mk_bv_sort(c, FIELD_WIDTHS[{g}]))")
            };
            if *is_bool && !bool_as_bv {
                let l = r.push(format!("Z3_ast e{} = {leaf};", r.n));
                r.push(format!(
                    "Z3_ast e{} = Z3_mk_eq(c, {l}, z3_num(c, 1, 1ull));",
                    r.n
                ))
            } else {
                r.push(format!("Z3_ast e{} = {leaf};", r.n))
            }
        }
        Se::Not(inner) => {
            let c_ = render_se_c(inner, spec, Some(1), class, false, step, r);
            let b = r.push(format!("Z3_ast e{} = Z3_mk_not(c, {c_});", r.n));
            finish_bool_c(&b, bool_as_bv, r)
        }
        Se::BitNot(inner) | Se::Neg(inner) => {
            let w = se_type(inner, spec).width();
            let c_ = render_se_c(inner, spec, Some(w), class, false, step, r);
            let m = if matches!(se, Se::BitNot(_)) {
                "bvnot"
            } else {
                "bvneg"
            };
            r.push(format!("Z3_ast e{} = Z3_mk_{m}(c, {c_});", r.n))
        }
        Se::Bin(op, l, rr) => {
            let lt = se_type(l, spec);
            let rt = se_type(rr, spec);
            if produces_bool(*op, lt, rt) {
                let b = render_bool_bin_c(*op, l, rr, spec, class, step, r);
                finish_bool_c(&b, bool_as_bv, r)
            } else {
                render_bit_bin_c(*op, l, rr, spec, class, step, r)
            }
        }
    }
}

/// C mirror of [`finish_bool_rust`].
fn finish_bool_c(name: &str, bool_as_bv: bool, r: &mut CRender) -> String {
    if !bool_as_bv {
        return name.to_string();
    }
    let one = r.push(format!("Z3_ast e{} = z3_num(c, 1, 1ull);", r.n));
    let zero = r.push(format!("Z3_ast e{} = z3_num(c, 1, 0ull);", r.n));
    r.push(format!(
        "Z3_ast e{} = Z3_mk_ite(c, {name}, {one}, {zero});",
        r.n
    ))
}

/// C mirror of [`render_bool_bin_rust`].
fn render_bool_bin_c(
    op: Sb,
    l: &Se,
    rr: &Se,
    spec: &SmtSpec,
    class: Option<usize>,
    step: &str,
    r: &mut CRender,
) -> String {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    match op {
        Sb::LogicAnd | Sb::LogicOr => {
            let a = render_se_c(l, spec, Some(1), class, false, step, r);
            let b = render_se_c(rr, spec, Some(1), class, false, step, r);
            let m = if op == Sb::LogicAnd { "and" } else { "or" };
            let an = format!("ab{}", r.n);
            r.push(format!("Z3_ast {an}[2] = {{ {a}, {b} }};"));
            r.push(format!("Z3_ast e{} = Z3_mk_{m}(c, 2, {an});", r.n))
        }
        Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => {
            let (wl, wr, ext_l, ext_r) = cmp_widths_full(l, rr, spec);
            let a0 = render_se_c(l, spec, Some(wl), class, false, step, r);
            let b0 = render_se_c(rr, spec, Some(wr), class, false, step, r);
            let a = widen_c(&a0, ext_l, r);
            let b = widen_c(&b0, ext_r, r);
            if lt == SeType::Bool && rt == SeType::Bool {
                match op {
                    Sb::Eq => r.push(format!("Z3_ast e{} = Z3_mk_eq(c, {a}, {b});", r.n)),
                    Sb::NotEq => {
                        let t = r.push(format!("Z3_ast e{} = Z3_mk_eq(c, {a}, {b});", r.n));
                        r.push(format!("Z3_ast e{} = Z3_mk_not(c, {t});", r.n))
                    }
                    Sb::Lt => {
                        let na = r.push(format!("Z3_ast e{} = Z3_mk_not(c, {a});", r.n));
                        let an = format!("ab{}", r.n);
                        r.push(format!("Z3_ast {an}[2] = {{ {na}, {b} }};"));
                        r.push(format!("Z3_ast e{} = Z3_mk_and(c, 2, {an});", r.n))
                    }
                    Sb::Le => {
                        let nb = r.push(format!("Z3_ast e{} = Z3_mk_not(c, {b});", r.n));
                        let an = format!("ab{}", r.n);
                        r.push(format!("Z3_ast {an}[2] = {{ {a}, {nb} }};"));
                        let t = r.push(format!("Z3_ast e{} = Z3_mk_and(c, 2, {an});", r.n));
                        r.push(format!("Z3_ast e{} = Z3_mk_not(c, {t});", r.n))
                    }
                    Sb::Gt => {
                        let nb = r.push(format!("Z3_ast e{} = Z3_mk_not(c, {b});", r.n));
                        let an = format!("ab{}", r.n);
                        r.push(format!("Z3_ast {an}[2] = {{ {nb}, {a} }};"));
                        r.push(format!("Z3_ast e{} = Z3_mk_and(c, 2, {an});", r.n))
                    }
                    Sb::Ge => {
                        let na = r.push(format!("Z3_ast e{} = Z3_mk_not(c, {a});", r.n));
                        let an = format!("ab{}", r.n);
                        r.push(format!("Z3_ast {an}[2] = {{ {b}, {na} }};"));
                        let t = r.push(format!("Z3_ast e{} = Z3_mk_and(c, 2, {an});", r.n));
                        r.push(format!("Z3_ast e{} = Z3_mk_not(c, {t});", r.n))
                    }
                    _ => unreachable!("comparison op only"),
                }
            } else {
                match op {
                    Sb::Eq => r.push(format!("Z3_ast e{} = Z3_mk_eq(c, {a}, {b});", r.n)),
                    Sb::NotEq => {
                        let t = r.push(format!("Z3_ast e{} = Z3_mk_eq(c, {a}, {b});", r.n));
                        r.push(format!("Z3_ast e{} = Z3_mk_not(c, {t});", r.n))
                    }
                    Sb::Lt => r.push(format!("Z3_ast e{} = Z3_mk_bvult(c, {a}, {b});", r.n)),
                    Sb::Le => r.push(format!("Z3_ast e{} = Z3_mk_bvule(c, {a}, {b});", r.n)),
                    Sb::Gt => r.push(format!("Z3_ast e{} = Z3_mk_bvugt(c, {a}, {b});", r.n)),
                    Sb::Ge => r.push(format!("Z3_ast e{} = Z3_mk_bvuge(c, {a}, {b});", r.n)),
                    _ => unreachable!("comparison op only"),
                }
            }
        }
        Sb::BitAnd | Sb::BitOr => {
            let a = render_se_c(l, spec, Some(1), class, false, step, r);
            let b = render_se_c(rr, spec, Some(1), class, false, step, r);
            let m = if op == Sb::BitAnd { "and" } else { "or" };
            let an = format!("ab{}", r.n);
            r.push(format!("Z3_ast {an}[2] = {{ {a}, {b} }};"));
            r.push(format!("Z3_ast e{} = Z3_mk_{m}(c, 2, {an});", r.n))
        }
        _ => unreachable!("bool-producing op only"),
    }
}

/// C mirror of [`render_bit_bin_rust`].
fn render_bit_bin_c(
    op: Sb,
    l: &Se,
    rr: &Se,
    spec: &SmtSpec,
    class: Option<usize>,
    step: &str,
    r: &mut CRender,
) -> String {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    match op {
        Sb::Plus | Sb::Minus | Sb::Star | Sb::BitAnd | Sb::BitOr | Sb::BitXor => {
            let w = match (lt, rt) {
                (SeType::Bit(a), SeType::Bit(b)) => {
                    debug_assert_eq!(a, b);
                    a
                }
                (SeType::IntLit, t) | (t, SeType::IntLit) => t.width(),
                _ => 8,
            };
            let a = render_se_c(l, spec, Some(w), class, false, step, r);
            let b = render_se_c(rr, spec, Some(w), class, false, step, r);
            let m = match op {
                Sb::Plus => "bvadd",
                Sb::Minus => "bvsub",
                Sb::Star => "bvmul",
                Sb::BitAnd => "bvand",
                Sb::BitOr => "bvor",
                _ => "bvxor",
            };
            r.push(format!("Z3_ast e{} = Z3_mk_{m}(c, {a}, {b});", r.n))
        }
        Sb::ShiftLeft | Sb::ShiftRight => render_shift_c(op, l, rr, spec, class, step, r),
        _ => unreachable!("bit-producing op only"),
    }
}

fn render_sf_c(sf: &Sf, spec: &SmtSpec, step: &str, r: &mut CRender) -> String {
    match sf {
        Sf::Se(inner) => render_se_c(inner, spec, Some(1), None, false, step, r),
        Sf::Not(inner) => {
            let c_ = render_sf_c(inner, spec, step, r);
            r.push(format!("Z3_ast e{} = Z3_mk_not(c, {c_});", r.n))
        }
        Sf::And(l, rr) => {
            let a = render_sf_c(l, spec, step, r);
            let b = render_sf_c(rr, spec, step, r);
            r.push(format!(
                "Z3_ast e{} = Z3_mk_and(c, 2, (Z3_ast[]){{{a}, {b}}});",
                r.n
            ))
        }
        Sf::Or(l, rr) => {
            let a = render_sf_c(l, spec, step, r);
            let b = render_sf_c(rr, spec, step, r);
            r.push(format!(
                "Z3_ast e{} = Z3_mk_or(c, 2, (Z3_ast[]){{{a}, {b}}});",
                r.n
            ))
        }
        Sf::Implies(l, rr) => {
            let a = render_sf_c(l, spec, step, r);
            let b = render_sf_c(rr, spec, step, r);
            r.push(format!("Z3_ast e{} = Z3_mk_implies(c, {a}, {b});", r.n))
        }
        // Atoms: the engine passes the per-step atom storage flattened
        // (k x N_ATOMS); explicit indexing - NOT name-shared constants, which
        // z3_mk_fresh_const does not guarantee to share across solver versions.
        Sf::InputIs { id, .. } => r.push(format!(
            "Z3_ast e{} = ii[({step}) * N_INPUT_ATOMS + {id}];",
            r.n
        )),
        Sf::OutputHas { id, .. } => r.push(format!(
            "Z3_ast e{} = oh[({step}) * N_OUTPUT_ATOMS + {id}];",
            r.n
        )),
        // next: the body holds one step later - WEAK at trace death (D1 fix):
        // if the path died at step+1 (an unhandled input froze the config), the
        // obligation is vacuously satisfied (alive[step+1] false -> true). The
        // `alive` parameter is in scope in every property/expansion fn.
        Sf::Next(inner) => {
            let ns = if step == "i" {
                "i + 1".to_string()
            } else {
                format!("({step}) + 1")
            };
            let b = render_sf_c(inner, spec, &ns, r);
            r.push(format!(
                "Z3_ast e{} = Z3_mk_or(c, 2, (Z3_ast[]){{ Z3_mk_not(c, alive[{ns}]), {b} }});",
                r.n
            ))
        }
        // FULL-class expansions (quantifier-free, horizon-truncated). The step
        // expression `step` is the anchor; loop variables are unique per node.
        Sf::BoundedEventually { bound, inner } => {
            let acc = r.push(format!("Z3_ast e{} = Z3_mk_false(c);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            let body_step = if step == "i" {
                format!("i + {jv}")
            } else {
                format!("({step}) + {jv}")
            };
            r.push(format!(
                "for (unsigned {jv} = 0; {jv} < {bound} && ({step}) + {jv} < k; {jv}++) {{"
            ));
            let b = render_sf_c(inner, spec, &body_step, r);
            r.push(format!("{acc} = Z3_mk_or(c, 2, (Z3_ast[]){{{acc}, {b}}});"));
            r.push("}".into());
            acc
        }
        Sf::BoundedAlways { bound, inner } => {
            let acc = r.push(format!("Z3_ast e{} = Z3_mk_true(c);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            let body_step = if step == "i" {
                format!("i + {jv}")
            } else {
                format!("({step}) + {jv}")
            };
            r.push(format!(
                "for (unsigned {jv} = 0; {jv} < {bound} && ({step}) + {jv} < k; {jv}++) {{"
            ));
            let b = render_sf_c(inner, spec, &body_step, r);
            r.push(format!(
                "{acc} = Z3_mk_and(c, 2, (Z3_ast[]){{{acc}, {b}}});"
            ));
            r.push("}".into());
            acc
        }
        Sf::Eventually(inner) => {
            let acc = r.push(format!("Z3_ast e{} = Z3_mk_false(c);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            r.push(format!("for (unsigned {jv} = {step}; {jv} < k; {jv}++) {{"));
            let b = render_sf_c(inner, spec, &jv, r);
            r.push(format!("{acc} = Z3_mk_or(c, 2, (Z3_ast[]){{{acc}, {b}}});"));
            r.push("}".into());
            acc
        }
        Sf::Until { left, right } => {
            let rh = r.push(format!("Z3_ast e{} = Z3_mk_false(c);", r.n));
            let jv = format!("j{}", r.n);
            r.n += 1;
            let iv = format!("i{}", r.n);
            r.n += 1;
            r.push(format!("for (unsigned {jv} = {step}; {jv} < k; {jv}++) {{"));
            let pf = r.push(format!("Z3_ast e{} = Z3_mk_true(c);", r.n));
            r.push(format!(
                // Weak until: p only strictly before the first q.
                "for (unsigned {iv} = {step}; {iv} < {jv}; {iv}++) {{"
            ));
            let lb = render_sf_c(left, spec, &iv, r);
            r.push(format!("{pf} = Z3_mk_and(c, 2, (Z3_ast[]){{{pf}, {lb}}});"));
            r.push("}".into());
            let rb = render_sf_c(right, spec, &jv, r);
            r.push(format!(
                "{rh} = Z3_mk_or(c, 2, (Z3_ast[]){{{rh}, Z3_mk_and(c, 2, (Z3_ast[]){{{rb}, {pf}}})}});"
            ));
            r.push("}".into());
            let lf = r.push(format!("Z3_ast e{} = Z3_mk_true(c);", r.n));
            let iv2 = format!("i{}", r.n);
            r.n += 1;
            r.push(format!(
                "for (unsigned {iv2} = {step}; {iv2} < k; {iv2}++) {{"
            ));
            let lb2 = render_sf_c(left, spec, &iv2, r);
            r.push(format!(
                "{lf} = Z3_mk_and(c, 2, (Z3_ast[]){{{lf}, {lb2}}});"
            ));
            r.push("}".into());
            let fin = r.push(format!(
                "Z3_ast e{} = Z3_mk_or(c, 2, (Z3_ast[]){{{rh}, {lf}}});",
                r.n
            ));
            fin
        }
    }
}

fn c_str_list(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| format!("{s:?}")).collect();
    format!("{{{}}}", inner.join(", "))
}

/// One C string literal (variant names are identifiers - no escaping
/// surprises; task 3D enum display tables).
fn c_str_lit(s: &str) -> String {
    format!("{s:?}")
}

/// Emit the full C spec section (constants + builder functions + tables).
pub fn emit_c_spec(spec: &SmtSpec) -> String {
    let mut s = String::new();
    // Includes come first: C requires them before first use (unlike Rust
    // `use` statements, which the engine section may carry).
    s.push_str("#include <z3.h>\n#include <stdint.h>\n#include <stdbool.h>\n#include <stdio.h>\n#include <stdlib.h>\n#include <string.h>\n");
    s.push_str("/* ===== SMT bounded-mode spec (task 11) - rendered from the shared plan;\n");
    s.push_str("   identical in both backends (parity by construction, D-P4-12) ===== */\n");
    s.push_str(&format!("#define STATE_BITS {0}u\n", spec.state_bits));
    s.push_str(&format!(
        "#define INITIAL_STATE {0}ull\n",
        spec.initial_state as u64
    ));
    s.push_str(&format!(
        "static const char *STATES[] = {};\n",
        c_str_list(&spec.states)
    ));
    let vnames: Vec<String> = spec.vars.iter().map(|v| v.name.clone()).collect();
    s.push_str(&format!("#define N_VARS {0}u\n", spec.vars.len()));
    s.push_str(&format!(
        "static const char *VAR_NAMES[] = {};\n",
        c_str_list(&vnames)
    ));
    // Zero-size arrays are invalid in C; a dummy slot keeps the definition
    // legal (the engine loops over N_VARS, never touching the dummy).
    let vw: Vec<String> = spec.vars.iter().map(|v| format!("{0}u", v.width)).collect();
    s.push_str(&format!(
        "static const unsigned VAR_WIDTHS[{}] = {{{}}};\n",
        if spec.vars.is_empty() {
            1
        } else {
            spec.vars.len()
        },
        if vw.is_empty() {
            "0u".to_string()
        } else {
            vw.join(", ")
        }
    ));
    s.push_str(&format!(
        "static const uint64_t VAR_INITS[] = {{{}}};\n",
        spec.vars
            .iter()
            .map(|v| format!("{0}ull", v.init))
            .collect::<Vec<_>>()
            .join(", ")
    ));
    let is_bools: Vec<String> = spec
        .vars
        .iter()
        .map(|v| (if v.is_bool { 1 } else { 0 }).to_string())
        .collect();
    s.push_str(&format!(
        "static const int VAR_IS_BOOL[] = {{{}}};\n",
        is_bools.join(", ")
    ));
    // Enum-var witness display (task 3D): concatenated variant names + per-var
    // offset/length (length 0 = not an enum - print the raw index as before).
    let mut ven: Vec<String> = Vec::new();
    let mut voff: Vec<String> = Vec::new();
    let mut vlen: Vec<String> = Vec::new();
    for v in &spec.vars {
        voff.push(format!("{}u", ven.len()));
        vlen.push(format!("{}u", v.enum_variants.len()));
        for name in &v.enum_variants {
            ven.push(c_str_lit(name));
        }
    }
    // A dummy name slot keeps the NAMES array non-empty when no var is an
    // enum (gcc -Warray-bounds warns on indexing a [0] array even under a
    // length guard; the dummy is never printed - LEN 0 skips it).
    s.push_str(&format!(
        "/* Enum-var variant names (concatenated; task 3D). */\nstatic const char *VAR_ENUM_NAMES[] = {{{}}};\nstatic const unsigned VAR_ENUM_OFF[{}] = {{{}}};\nstatic const unsigned VAR_ENUM_LEN[{}] = {{{}}};\n",
        if ven.is_empty() { "\"\"".to_string() } else { ven.join(", ") },
        if spec.vars.is_empty() { 1 } else { spec.vars.len() },
        if voff.is_empty() { "0u".to_string() } else { voff.join(", ") },
        if spec.vars.is_empty() { 1 } else { spec.vars.len() },
        if vlen.is_empty() { "0u".to_string() } else { vlen.join(", ") }
    ));
    let cnames: Vec<String> = spec.classes.iter().map(|c| c.name.clone()).collect();
    s.push_str(&format!(
        "static const char *CLASS_NAMES[] = {};\n",
        c_str_list(&cnames)
    ));
    s.push_str(&format!("#define N_CLASSES {0}u\n", spec.classes.len()));
    let cls_bits = crate::mc::smt_plan::ceil_log2_pub(spec.classes.len());
    s.push_str(&format!("#define CLS_BITS {cls_bits}u\n"));
    let fnames: Vec<String> = spec
        .classes
        .iter()
        .flat_map(|c| c.fields.iter().map(|(n, _, _, _)| n.clone()))
        .collect();
    s.push_str(&format!(
        "static const char *FIELD_NAMES[{}] = {};\n",
        if fnames.is_empty() { 1 } else { fnames.len() },
        if fnames.is_empty() {
            "{\"\"}".to_string()
        } else {
            c_str_list(&fnames)
        }
    ));
    let fw: Vec<String> = spec
        .classes
        .iter()
        .flat_map(|c| c.fields.iter().map(|(_, w, _, _)| format!("{w}u")))
        .collect();
    s.push_str(&format!(
        "static const unsigned FIELD_WIDTHS[{}] = {{{}}};\n",
        if fw.is_empty() { 1 } else { fw.len() },
        if fw.is_empty() {
            "0u".to_string()
        } else {
            fw.join(", ")
        }
    ));
    let fbools: Vec<String> = spec
        .classes
        .iter()
        .flat_map(|c| {
            c.fields
                .iter()
                .map(|(_, _, b, _)| (if *b { 1 } else { 0 }).to_string())
        })
        .collect();
    s.push_str(&format!("#define N_FIELDS {0}u\n", fbools.len()));
    s.push_str(&format!(
        "static const int FIELD_IS_BOOL[{}] = {{{}}};\n",
        if fbools.is_empty() { 1 } else { fbools.len() },
        if fbools.is_empty() {
            "0".to_string()
        } else {
            fbools.join(", ")
        }
    ));
    // Enum-field witness display (task 3D): same pattern as the vars.
    let mut fen: Vec<String> = Vec::new();
    let mut foff: Vec<String> = Vec::new();
    let mut flen: Vec<String> = Vec::new();
    for c in &spec.classes {
        for (_, _, _, ev) in &c.fields {
            foff.push(format!("{}u", fen.len()));
            flen.push(format!("{}u", ev.len()));
            for name in ev {
                fen.push(c_str_lit(name));
            }
        }
    }
    s.push_str(&format!(
        "/* Enum-field variant names (concatenated; task 3D). */\nstatic const char *FIELD_ENUM_NAMES[] = {{{}}};\nstatic const unsigned FIELD_ENUM_OFF[{}] = {{{}}};\nstatic const unsigned FIELD_ENUM_LEN[{}] = {{{}}};\n",
        if fen.is_empty() { "\"\"".to_string() } else { fen.join(", ") },
        if foff.is_empty() { 1 } else { foff.len() },
        if foff.is_empty() { "0u".to_string() } else { foff.join(", ") },
        if flen.is_empty() { 1 } else { flen.len() },
        if flen.is_empty() { "0u".to_string() } else { flen.join(", ") }
    ));
    let mut fc: Vec<String> = Vec::new();
    for (ci, cls) in spec.classes.iter().enumerate() {
        for fi in 0..cls.fields.len() {
            fc.push(format!("{{{ci}u, {fi}u}}"));
        }
    }
    s.push_str(&format!(
        "static const struct {{ unsigned c, f; }} FIELD_POS[] = {{{}}};\n",
        fc.join(", ")
    ));
    let trans: Vec<String> = spec
        .transitions
        .iter()
        .map(|t| format!("{{{0}u, {1}u}}", t.state, t.next_state))
        .collect();
    s.push_str(&format!(
        "/* Transitions in firing order: (state, next_state). */\nstatic const struct {{ unsigned state, next_state; }} TRANS[] = {{{}}};\n#define N_TRANS {1}u\n",
        trans.join(", "),
        spec.transitions.len()
    ));
    // Per-transition input classes (task 3D): concatenated class lists. A
    // named-variant transition has exactly one; a bare-group wildcard has all
    // of the group's variants (enabled_t ORs over them).
    let mut cls_all: Vec<String> = Vec::new();
    let mut cls_off: Vec<String> = Vec::new();
    let mut cls_len: Vec<String> = Vec::new();
    for t in &spec.transitions {
        cls_off.push(format!("{}u", cls_all.len()));
        cls_len.push(format!("{}u", t.classes.len()));
        for &c in &t.classes {
            cls_all.push(format!("{c}u"));
        }
    }
    s.push_str(&format!(
        "/* Per-transition input classes (concatenated; task 3D). */\nstatic const unsigned TRANS_CLASSES[] = {{{}}};\nstatic const unsigned TRANS_CLS_OFF[{}] = {{{}}};\nstatic const unsigned TRANS_CLS_LEN[{}] = {{{}}};\n",
        cls_all.join(", "),
        if cls_off.is_empty() { 1 } else { cls_off.len() },
        if cls_off.is_empty() { "0u".to_string() } else { cls_off.join(", ") },
        if cls_len.is_empty() { 1 } else { cls_len.len() },
        if cls_len.is_empty() { "0u".to_string() } else { cls_len.join(", ") }
    ));
    let pnames: Vec<String> = spec.properties.iter().map(|p| p.name.clone()).collect();
    s.push_str(&format!(
        "static const char *PROP_NAMES[] = {};\n",
        c_str_list(&pnames)
    ));
    let kinds: Vec<String> = spec
        .properties
        .iter()
        .map(|p| {
            match p.kind {
                crate::mc::smt_plan::SmtPropKind::Always => "0",
                crate::mc::smt_plan::SmtPropKind::Eventually => "1",
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(_) => "2",
                crate::mc::smt_plan::SmtPropKind::Until => "3",
            }
            .to_string()
        })
        .collect();
    s.push_str(&format!(
        "/* 0 = always (safety), 1 = eventually (liveness), 2 = bounded_eventually (full class), 3 = until (full class). */\nstatic const unsigned char PROP_KINDS[] = {{{}}};\n#define N_PROPS {1}u\n",
        kinds.join(", "),
        spec.properties.len()
    ));
    s.push_str(&format!(
        "/* always[N] anchor clip (0 = unbounded). */\nstatic const unsigned int PROP_BOUNDS[] = {{{}}};\n",
        spec.properties
            .iter()
            .map(|p| p.bound.unwrap_or(0).to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s.push_str(&format!(
        "/* Maximum `next` nesting depth (properties are checked only at steps i with i + depth < k). */\nstatic const unsigned int PROP_NEXT_DEPTH[] = {{{}}};\n",
        spec.properties
            .iter()
            .map(|p| p.next_depth.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s.push_str(&format!(
        "/* FULL class (checked only with --full-properties). */\nstatic const unsigned char PROP_FULL[] = {{{}}};\n",
        spec.properties
            .iter()
            .map(|p| p.full.to_string())
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s.push_str(&format!(
        "/* bounded_eventually[N] horizon N (0 = not a bounded_eventually). */\nstatic const unsigned int PROP_BE_BOUND[] = {{{}}};\n",
        spec.properties
            .iter()
            .map(|p| match p.kind {
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(n) => n.to_string(),
                _ => "0".to_string(),
            })
            .collect::<Vec<_>>()
            .join(", ")
    ));
    s.push_str(&format!(
        "#define N_INPUT_ATOMS {}u\n#define N_OUTPUT_ATOMS {}u\n",
        spec.input_atoms.len(),
        spec.output_atoms.len()
    ));

    s.push_str("\nstatic Z3_ast z3_num(Z3_context c, unsigned w, uint64_t v) {\n");
    s.push_str("    return Z3_mk_unsigned_int64(c, v, Z3_mk_bv_sort(c, w));\n}\n\n");

    // Transition guards.
    for (t, tr) in spec.transitions.iter().enumerate() {
        let mut r = CRender::new();
        match &tr.guard {
            Some(g) => render_se_c(g, spec, Some(1), Some(tr.classes[0]), false, "i", &mut r),
            None => r.push("Z3_ast e0 = Z3_mk_true(c);".into()),
        };
        s.push_str(&format!(
            "static Z3_ast expr_t{t}_guard(Z3_context c, Z3_ast s, const Z3_ast *v, const Z3_ast *fld) {{\n    (void)s; (void)v; (void)fld;\n    {}\n    return e{};\n}}\n\n",
            r.lines.join("\n    "),
            r.n - 1
        ));
    }
    // Transition assignments (`asgn_dummy` fills unused table slots - never
    // called, the engine checks ASGN_MAP first). Emitted only when some slot
    // needs it (warning-free generated code).
    let dummy_needed = spec
        .transitions
        .iter()
        .any(|t| (0..spec.vars.len()).any(|j| !t.assignments.iter().any(|(vi, _)| *vi == j)));
    if dummy_needed {
        s.push_str("static Z3_ast asgn_dummy(Z3_context c, Z3_ast s, const Z3_ast *v, const Z3_ast *fld) {\n");
        s.push_str("    (void)s; (void)v; (void)fld;\n    return z3_num(c, 1u, 0ull);\n}\n\n");
    }
    for (t, tr) in spec.transitions.iter().enumerate() {
        for j in 0..spec.vars.len() {
            if let Some((_, expr)) = tr.assignments.iter().find(|(vi, _)| *vi == j) {
                let mut r = CRender::new();
                render_se_c(
                    expr,
                    spec,
                    Some(spec.vars[j].width),
                    Some(tr.classes[0]),
                    true,
                    "i",
                    &mut r,
                );
                s.push_str(&format!(
                    "static Z3_ast expr_t{t}_asgn_{j}(Z3_context c, Z3_ast s, const Z3_ast *v, const Z3_ast *fld) {{\n    (void)s; (void)v; (void)fld;\n    {}\n    return e{};\n}}\n\n",
                    r.lines.join("\n    "),
                    r.n - 1
                ));
            }
        }
    }
    // Property formulas. Every property fn takes (step, horizon) so FULL-class
    // expansions can reference `k`; default-class bodies never do. FULL-class
    // properties get a dummy body here (never called - the engine routes them
    // to expand_p{p}, emitted below).
    for (p, prop) in spec.properties.iter().enumerate() {
        let routed = prop.full
            || matches!(
                prop.kind,
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(_)
            );
        if routed {
            s.push_str(&format!(
                "static Z3_ast expr_p{p}(Z3_context c, unsigned i, unsigned k, Z3_ast s, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *alive, const Z3_ast *ii, const Z3_ast *oh) {{\n    (void)c; (void)i; (void)k; (void)s; (void)v; (void)fld; (void)alive; (void)ii; (void)oh;\n    return Z3_mk_true(c);\n}}\n\n"
            ));
            continue;
        }
        let mut r = CRender::new();
        render_sf_c(&prop.formula, spec, "i", &mut r);
        // Atom parameters are named (no void-cast) only when the body
        // references them.
        let ii_name = if sf_uses_input_atoms(&prop.formula) {
            "ii"
        } else {
            "_ii"
        };
        let oh_name = if sf_uses_output_atoms(&prop.formula) {
            "oh"
        } else {
            "_oh"
        };
        // The alive parameter is named (no void-cast) only when the body has a
        // next node (the D1 gate references it).
        let alive_name = if sf_uses_next(&prop.formula) {
            "alive"
        } else {
            "_alive"
        };
        let voids = format!(
            "(void)s; (void)v; (void)fld; (void)k;{}{}{}",
            if ii_name == "_ii" { " (void)_ii;" } else { "" },
            if oh_name == "_oh" { " (void)_oh;" } else { "" },
            if alive_name == "_alive" {
                " (void)_alive;"
            } else {
                ""
            }
        );
        let void_i = if sf_uses_step(&prop.formula) {
            String::new()
        } else {
            "\n    (void)i;".to_string()
        };
        s.push_str(&format!(
            "static Z3_ast expr_p{p}(Z3_context c, unsigned i, unsigned k, Z3_ast s, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *{alive_name}, const Z3_ast *{ii_name}, const Z3_ast *{oh_name}) {{\n    {voids}{void_i}\n    {lines}\n    return e{n};\n}}\n\n",
            lines = r.lines.join("\n    "),
            n = r.n - 1
        ));
    }
    // Expansions for routed properties (FULL class + top-level
    // bounded_eventually): the whole formula rendered with the horizon `k`
    // in scope (temporal nodes emit horizon-truncated loops). Top-level BE
    // stores only its body in `formula` (the operator lives in `kind`), so
    // wrap it in a synthetic node to get the clipped OR loop.
    for (p, prop) in spec.properties.iter().enumerate() {
        let routed = prop.full
            || matches!(
                prop.kind,
                crate::mc::smt_plan::SmtPropKind::BoundedEventually(_)
            );
        if !routed {
            continue;
        }
        let formula = match &prop.kind {
            crate::mc::smt_plan::SmtPropKind::BoundedEventually(n) => Sf::BoundedEventually {
                bound: *n,
                inner: Box::new(prop.formula.clone()),
            },
            _ => prop.formula.clone(),
        };
        let mut r = CRender::new();
        // The renderer returns the final binding's name (see the Rust side).
        let fin = render_sf_c(&formula, spec, "i", &mut r);
        let ii_name = if sf_uses_input_atoms(&formula) {
            "ii"
        } else {
            "_ii"
        };
        let oh_name = if sf_uses_output_atoms(&formula) {
            "oh"
        } else {
            "_oh"
        };
        let alive_name = if sf_uses_next(&formula) {
            "alive"
        } else {
            "_alive"
        };
        let voids = format!(
            "(void)s; (void)v; (void)fld;{}{}{}",
            if ii_name == "_ii" { " (void)_ii;" } else { "" },
            if oh_name == "_oh" { " (void)_oh;" } else { "" },
            if alive_name == "_alive" {
                " (void)_alive;"
            } else {
                ""
            }
        );
        let void_i = if sf_uses_step(&formula) {
            String::new()
        } else {
            "\n    (void)i;".to_string()
        };
        s.push_str(&format!(
            "static Z3_ast expand_p{p}(Z3_context c, unsigned i, unsigned k, Z3_ast s, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *{alive_name}, const Z3_ast *{ii_name}, const Z3_ast *{oh_name}) {{\n    {voids}{void_i}\n    {lines}\n    return {fin};\n}}\n\n",
            lines = r.lines.join("\n    ")
        ));
    }
    // input_is/output_has atom builders (called from the unrolling loop to
    // define the ii_/oh_ named constants). Specs without atoms get trivial
    // dummies - the engine's loops are compiled out by the N_*_ATOMS == 0
    // guards, but the functions must exist for the loop bodies to compile.
    // With on_error the unrolling loop calls build_input_atom_ev instead, so
    // the plain builder would be an unused static (-Wall) - emit it only
    // without on_error.
    if spec.on_error.is_some() {
        // no plain input-atom builder (the _ev variant is emitted in the
        // on_error section below)
    } else if spec.input_atoms.is_empty() {
        // The ev parameter exists only for multi-class specs (task 3D) - the
        // call site passes it only then, keeping single-class codegen
        // byte-identical to before.
        let ev_param = if spec.classes.len() > 1 {
            ", const Z3_ast *ev"
        } else {
            ""
        };
        s.push_str(&format!(
            "static Z3_ast build_input_atom(Z3_context c, unsigned a, unsigned i, const Z3_ast *fld, const Z3_ast *fires{ev_param}) {{\n    (void)c; (void)a; (void)i; (void)fld; (void)fires{};\n    return Z3_mk_false(c);\n}}\n\n",
            if spec.classes.len() > 1 { "; (void)ev" } else { "" }
        ));
    } else {
        let ev_param = if spec.classes.len() > 1 {
            ", const Z3_ast *ev"
        } else {
            ""
        };
        s.push_str(&format!(
            "static Z3_ast build_input_atom(Z3_context c, unsigned a, unsigned i, const Z3_ast *fld, const Z3_ast *fires{ev_param}) {{\n"
        ));
        for (a, arms) in spec.input_atoms.iter().enumerate() {
            s.push_str(&format!("    if (a == {a}) {{\n"));
            s.push_str("        Z3_ast val = Z3_mk_false(c);\n");
            for (ci, constraints) in arms {
                s.push_str("        {\n");
                s.push_str("            Z3_ast cls_fire = Z3_mk_false(c);\n");
                // Transitions covering this arm's class (task 3D): a
                // multi-variant wildcard transition covers every class in its
                // `classes` - the index list is precomputed at emission.
                let covering: Vec<String> = spec
                    .transitions
                    .iter()
                    .enumerate()
                    .filter(|(_, tr)| tr.classes.contains(ci))
                    .map(|(t, _)| format!("{t}u"))
                    .collect();
                if covering.is_empty() {
                    // ev joins the suppression only on multi-class specs (the
                    // parameter exists only then).
                    let ev_sup = if spec.classes.len() > 1 {
                        " (void)ev;"
                    } else {
                        ""
                    };
                    s.push_str(&format!(
                        "            (void)fires;{ev_sup} // no transition covers this class\n"
                    ));
                } else {
                    for t in &covering {
                        s.push_str(&format!(
                            "            {{ Z3_ast pair[2] = {{ cls_fire, fires[{t}] }}; cls_fire = Z3_mk_or(c, 2, pair); }}\n"
                        ));
                    }
                }
                // The actual-input class conjunct (task 3D): entailed by
                // fires[t] for single-class transitions, but a multi-variant
                // wildcard fires on ANY of its classes - without it every
                // covered arm would be true at once and input_is could hold
                // for the variant that did NOT come in (masking violations).
                let mut arm = if spec.classes.len() > 1 {
                    s.push_str(&format!(
                        "            Z3_ast ccm = Z3_mk_eq(c, ev[i], z3_num(c, CLS_BITS, {ci}u));\n"
                    ));
                    "Z3_mk_and(c, 2, (Z3_ast[]){ cls_fire, ccm })".to_string()
                } else {
                    "cls_fire".to_string()
                };
                for (pos, (g, v, _isb)) in constraints.iter().enumerate() {
                    s.push_str(&format!(
                        "            Z3_ast c{pos} = Z3_mk_eq(c, fld[{g}], z3_num(c, FIELD_WIDTHS[{g}], {v}ull));\n"
                    ));
                    arm = format!("Z3_mk_and(c, 2, (Z3_ast[]){{ {arm}, c{pos} }})");
                }
                s.push_str(&format!(
                    "            val = Z3_mk_or(c, 2, (Z3_ast[]){{ val, {arm} }});\n"
                ));
                s.push_str("        }\n");
            }
            s.push_str("        return val;\n    }");
        }
        s.push_str("    return Z3_mk_false(c);\n}\n\n");
    }
    if spec.output_atoms.is_empty() {
        s.push_str(
            "static Z3_ast build_output_atom(Z3_context c, unsigned a, unsigned i, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *fires) {\n    (void)c; (void)a; (void)i; (void)v; (void)fld; (void)fires;\n    return Z3_mk_false(c);\n}\n\n",
        );
    } else {
        // Flatten every (transition, emit) pair into tables; each pair's
        // argument expressions become one step function filling an out array,
        // dispatched through a function-pointer table.
        let mut pair_t: Vec<usize> = Vec::new();
        let mut pair_group: Vec<String> = Vec::new();
        let mut pair_variant: Vec<Option<String>> = Vec::new();
        for (t, tr) in spec.transitions.iter().enumerate() {
            for o in &tr.emits {
                pair_t.push(t);
                pair_group.push(o.group.clone());
                pair_variant.push(o.variant.clone());
            }
        }
        s.push_str(&format!(
            "static const unsigned char EMIT_PAIR_T[] = {{{}}};\n#define N_EMIT_PAIRS {1}u\n",
            pair_t
                .iter()
                .map(|t| format!("{t}u"))
                .collect::<Vec<_>>()
                .join(", "),
            pair_t.len()
        ));
        s.push_str(&format!(
            "static const char *EMIT_PAIR_GROUP[] = {};\n",
            c_str_list(&pair_group)
        ));
        let variants: Vec<String> = pair_variant
            .iter()
            .map(|v| match v {
                Some(s) => format!("{s:?}"),
                None => "0".to_string(),
            })
            .collect();
        s.push_str(&format!(
            "/* NULL = any variant of the group. */\nstatic const char *EMIT_PAIR_VARIANT[] = {{{}}};\n",
            variants.join(", ")
        ));
        let mut argw_rows: Vec<String> = Vec::new();
        for (pi, tr) in spec.transitions.iter().enumerate() {
            for o in &tr.emits {
                s.push_str(&format!(
                    "static const unsigned int EMIT_ARGW_{pi}[] = {{{}}};\n",
                    o.args
                        .iter()
                        .map(|a| format!("{}u", se_type(a, spec).width()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                argw_rows.push(format!("EMIT_ARGW_{pi}"));
            }
        }
        s.push_str(&format!(
            "static const unsigned int *EMIT_ARGW[] = {{{}}};\n",
            argw_rows.join(", ")
        ));
        let mut fn_names: Vec<String> = Vec::new();
        for (pi, tr) in spec.transitions.iter().enumerate() {
            for (oi, o) in tr.emits.iter().enumerate() {
                if o.args.is_empty() {
                    s.push_str(
                        "static void emit_args_dummy(Z3_context c, unsigned i, const Z3_ast *v, const Z3_ast *fld, Z3_ast *out) {\n    (void)c; (void)i; (void)v; (void)fld; (void)out;\n}\n\n",
                    );
                    fn_names.push("emit_args_dummy".to_string());
                    continue;
                }
                let mut body = String::new();
                for (ai, arg) in o.args.iter().enumerate() {
                    let mut r = CRender::new();
                    render_se_c(
                        arg,
                        spec,
                        Some(se_type(arg, spec).width()),
                        Some(tr.classes[0]),
                        true,
                        "i",
                        &mut r,
                    );
                    body.push_str(&format!(
                        "    {{\n{}        out[{ai}] = e{};\n    }}\n",
                        r.lines.join("\n        "),
                        r.n - 1
                    ));
                }
                let voids = format!("(void)i; (void)v; (void)fld;");
                s.push_str(&format!(
                    "static void emit_args_{pi}_{oi}(Z3_context c, unsigned i, const Z3_ast *v, const Z3_ast *fld, Z3_ast *out) {{\n    {voids}\n{body}}}\n\n"
                ));
                fn_names.push(format!("emit_args_{pi}_{oi}"));
            }
        }
        s.push_str(&format!(
            "static void (*EMIT_ARGS_FNS[N_EMIT_PAIRS])(Z3_context, unsigned, const Z3_ast *, const Z3_ast *, Z3_ast *) = {{{}}};\n",
            fn_names.join(", ")
        ));
        let max_args = spec
            .transitions
            .iter()
            .flat_map(|t| t.emits.iter())
            .map(|o| o.args.len())
            .max()
            .unwrap_or(0)
            .max(1);
        // Transition-outer structure mirrors the ref engine's operation order
        // exactly (fires[t] AND OR-over-emits match) - parity-critical.
        s.push_str("static Z3_ast build_output_atom(Z3_context c, unsigned a, unsigned i, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *fires) {\n");
        for (a, (group, variant, constraints)) in spec.output_atoms.iter().enumerate() {
            s.push_str(&format!("    if (a == {a}) {{\n"));
            s.push_str("        Z3_ast val = Z3_mk_false(c);\n");
            s.push_str("        for (unsigned t = 0; t < N_TRANS; t++) {\n");
            s.push_str("            Z3_ast emit_match = Z3_mk_false(c);\n");
            s.push_str("            for (unsigned pi = 0; pi < N_EMIT_PAIRS; pi++) {\n");
            s.push_str("                if (EMIT_PAIR_T[pi] != t) continue;\n");
            // Group-level pattern relaxation (same rule as the Rust emitter -
            // exact mode's output_matches matches any variant of the group;
            // unit-output groups keep the byte-identical `== 0` form).
            let variant_check = match variant {
                Some(v) => format!("strcmp(EMIT_PAIR_VARIANT[pi], \"{v}\") == 0"),
                None => {
                    if spec.transitions.iter().any(|t| {
                        t.emits
                            .iter()
                            .any(|o| o.group == *group && o.variant.is_some())
                    }) {
                        "1".to_string()
                    } else {
                        "EMIT_PAIR_VARIANT[pi] == 0".to_string()
                    }
                }
            };
            s.push_str(&format!(
                "                if (strcmp(EMIT_PAIR_GROUP[pi], \"{group}\") == 0 && {variant_check} ) {{\n"
            ));
            s.push_str(&format!("                    Z3_ast args[{max_args}];\n"));
            s.push_str(&format!(
                "                    EMIT_ARGS_FNS[pi](c, i, v, fld, args);\n"
            ));
            s.push_str("                    Z3_ast m = Z3_mk_true(c);\n");
            for (pos, v, _isb) in constraints {
                s.push_str(&format!(
                    "                    Z3_ast c{pos} = Z3_mk_eq(c, args[{pos}], z3_num(c, EMIT_ARGW[pi][{pos}], {v}ull));\n"
                ));
                s.push_str(&format!(
                    "                    m = Z3_mk_and(c, 2, (Z3_ast[]){{ m, c{pos} }});\n"
                ));
            }
            s.push_str(&format!(
                "                    emit_match = Z3_mk_or(c, 2, (Z3_ast[]){{ emit_match, m }});\n"
            ));
            s.push_str("                }\n            }\n");
            s.push_str(&format!(
                "            val = Z3_mk_or(c, 2, (Z3_ast[]){{ val, Z3_mk_and(c, 2, (Z3_ast[]){{ fires[t], emit_match }}) }});\n"
            ));
            s.push_str("        }\n");
            s.push_str("        return val;\n    }");
        }
        s.push_str("    return Z3_mk_false(c);\n}\n\n");
    }
    // ASGN_MAP[t][j]: does transition t assign var j?
    let mut map_rows = Vec::new();
    for t in 0..spec.transitions.len() {
        let row: Vec<String> = (0..spec.vars.len())
            .map(|j| {
                if spec.transitions[t]
                    .assignments
                    .iter()
                    .any(|(vi, _)| *vi == j)
                {
                    "1".to_string()
                } else {
                    "0".to_string()
                }
            })
            .collect();
        map_rows.push(format!("{{{}}}", row.join(", ")));
    }
    s.push_str(&format!(
        "static const unsigned char ASGN_MAP[N_TRANS][{:?}] = {{{}}};\n\n",
        spec.vars.len(),
        map_rows.join(", ")
    ));
    // Function-pointer tables.
    let guards: Vec<String> = (0..spec.transitions.len())
        .map(|t| format!("expr_t{t}_guard"))
        .collect();
    s.push_str(&format!(
        "static Z3_ast (*GUARD_FNS[N_TRANS])(Z3_context, Z3_ast, const Z3_ast *, const Z3_ast *) = {{{}}};\n",
        guards.join(", ")
    ));
    let mut asgn_rows = Vec::new();
    for t in 0..spec.transitions.len() {
        let row: Vec<String> = (0..spec.vars.len())
            .map(|j| {
                if spec.transitions[t]
                    .assignments
                    .iter()
                    .any(|(vi, _)| *vi == j)
                {
                    format!("expr_t{t}_asgn_{j}")
                } else {
                    "asgn_dummy".to_string()
                }
            })
            .collect();
        asgn_rows.push(format!("{{{}}}", row.join(", ")));
    }
    s.push_str(&format!(
        "static Z3_ast (*ASGN_FNS[N_TRANS][{:?}])(Z3_context, Z3_ast, const Z3_ast *, const Z3_ast *) = {{{}}};\n",
        spec.vars.len(),
        asgn_rows.join(", ")
    ));
    let props: Vec<String> = (0..spec.properties.len())
        .map(|p| format!("expr_p{p}"))
        .collect();
    s.push_str(&format!(
        "static Z3_ast (*PROP_FNS[N_PROPS])(Z3_context, unsigned, unsigned, Z3_ast, const Z3_ast *, const Z3_ast *, const Z3_ast *, const Z3_ast *, const Z3_ast *) = {{{}}};\n",
        props.join(", ")
    ));
    // Expansion table: routed properties (FULL class + top-level
    // bounded_eventually) point at expand_p{p}; the rest are dummies (never
    // called - the engine dispatches on PROP_KINDS).
    let expands: Vec<String> = spec
        .properties
        .iter()
        .enumerate()
        .map(|(p, prop)| {
            if prop.full
                || matches!(
                    prop.kind,
                    crate::mc::smt_plan::SmtPropKind::BoundedEventually(_)
                )
            {
                format!("expand_p{p}")
            } else {
                "expand_dummy".to_string()
            }
        })
        .collect();
    // The dummy is emitted only when some slot needs it (warning-free code).
    if expands.iter().any(|e| e == "expand_dummy") {
        s.push_str("static Z3_ast expand_dummy(Z3_context c, unsigned i, unsigned k, Z3_ast s, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *alive, const Z3_ast *ii, const Z3_ast *oh) {\n    (void)c; (void)i; (void)k; (void)s; (void)v; (void)fld; (void)alive; (void)ii; (void)oh;\n    return Z3_mk_true(c);\n}\n\n");
    }
    s.push_str(&format!(
        "static Z3_ast (*EXPAND_FNS[N_PROPS])(Z3_context, unsigned, unsigned, Z3_ast, const Z3_ast *, const Z3_ast *, const Z3_ast *, const Z3_ast *, const Z3_ast *) = {{{}}};\n",
        expands.join(", ")
    ));

    // ---- on_error fallback (task 3C, D4) - emitted ONLY when the module has
    // one, so specs without it generate byte-identical code to before. ----
    if let Some(oe) = &spec.on_error {
        s.push_str(&format!(
            "/* on_error fallback (task 3C): catch-all lowest-priority transition\n * from ANY state - no pattern bindings, no variable assignments. */\nstatic const unsigned ON_ERROR_NEXT_STATE = {next}u;\n",
            next = oe.next_state
        ));
        // The tables are consumed only by build_on_error_emit_match, which is
        // a constant-false stub when the module has no output atoms - emit
        // them then and the statics would be unused (-Wall).
        if spec.output_atoms.is_empty() {
            // no tables; the stub consumer references nothing
        } else if oe.emits.is_empty() {
            s.push_str("static const char *OE_EMIT_GROUP[] = {0};\n#define N_OE_EMITS 0u\nstatic const char *OE_EMIT_VARIANT[] = {0};\nstatic const unsigned int *OE_EMIT_ARGW[] = {0};\n");
            s.push_str(
                "static void oe_emit_args_dummy(Z3_context c, unsigned i, const Z3_ast *v, const Z3_ast *fld, Z3_ast *out) {\n    (void)c; (void)i; (void)v; (void)fld; (void)out;\n}\nstatic void (*OE_EMIT_ARGS_FNS[1])(Z3_context, unsigned, const Z3_ast *, const Z3_ast *, Z3_ast *) = {oe_emit_args_dummy};\n\n",
            );
        } else {
            let groups: Vec<String> = oe.emits.iter().map(|o| o.group.clone()).collect();
            s.push_str(&format!(
                "static const char *OE_EMIT_GROUP[] = {};\n#define N_OE_EMITS {1}u\n",
                c_str_list(&groups),
                oe.emits.len()
            ));
            let variants: Vec<String> = oe
                .emits
                .iter()
                .map(|o| match &o.variant {
                    Some(v) => format!("{v:?}"),
                    None => "0".to_string(),
                })
                .collect();
            s.push_str(&format!(
                "/* NULL = unit variant. */\nstatic const char *OE_EMIT_VARIANT[] = {{{}}};\n",
                variants.join(", ")
            ));
            let mut fn_names: Vec<String> = Vec::new();
            for (oi, o) in oe.emits.iter().enumerate() {
                s.push_str(&format!(
                    "static const unsigned int OE_EMIT_ARGW_{oi}[] = {{{}}};\n",
                    o.args
                        .iter()
                        .map(|a| format!("{}u", se_type(a, spec).width()))
                        .collect::<Vec<_>>()
                        .join(", ")
                ));
                if o.args.is_empty() {
                    s.push_str(
                        "static void oe_emit_args_dummy(Z3_context c, unsigned i, const Z3_ast *v, const Z3_ast *fld, Z3_ast *out) {\n    (void)c; (void)i; (void)v; (void)fld; (void)out;\n}\n\n",
                    );
                    fn_names.push("oe_emit_args_dummy".to_string());
                    continue;
                }
                let mut body = String::new();
                for (ai, arg) in o.args.iter().enumerate() {
                    let mut r = CRender::new();
                    render_se_c(
                        arg,
                        spec,
                        Some(se_type(arg, spec).width()),
                        None,
                        true,
                        "i",
                        &mut r,
                    );
                    body.push_str(&format!(
                        "    {{\n{}        out[{ai}] = e{};\n    }}\n",
                        r.lines.join("\n        "),
                        r.n - 1
                    ));
                }
                let voids = "(void)i; (void)v; (void)fld;";
                s.push_str(&format!(
                    "static void oe_emit_args_{oi}(Z3_context c, unsigned i, const Z3_ast *v, const Z3_ast *fld, Z3_ast *out) {{\n    {voids}\n{body}}}\n\n"
                ));
                fn_names.push(format!("oe_emit_args_{oi}"));
            }
            s.push_str(&format!(
                "static const unsigned int *OE_EMIT_ARGW[] = {{{}}};\n",
                (0..oe.emits.len())
                    .map(|oi| format!("OE_EMIT_ARGW_{oi}"))
                    .collect::<Vec<_>>()
                    .join(", ")
            ));
            s.push_str(&format!(
                "static void (*OE_EMIT_ARGS_FNS[N_OE_EMITS])(Z3_context, unsigned, const Z3_ast *, const Z3_ast *, Z3_ast *) = {{{}}};\n",
                fn_names.join(", ")
            ));
        }
        // input_is evaluated against the ACTUAL input (ev_i + payload fields)
        // - with on_error an unmatched input is still consumed (exact mode
        // records it in the on_error sample, SEMANTICS 3.6).
        if spec.input_atoms.is_empty() {
            s.push_str(
                "static Z3_ast build_input_atom_ev(Z3_context c, unsigned a, unsigned i, const Z3_ast *ev, const Z3_ast *fld) {\n    (void)c; (void)a; (void)i; (void)ev; (void)fld;\n    return Z3_mk_false(c);\n}\n\n",
            );
        } else {
            s.push_str(
                "static Z3_ast build_input_atom_ev(Z3_context c, unsigned a, unsigned i, const Z3_ast *ev, const Z3_ast *fld) {\n",
            );
            for (a, arms) in spec.input_atoms.iter().enumerate() {
                s.push_str(&format!("    if (a == {a}) {{\n"));
                s.push_str("        Z3_ast val = Z3_mk_false(c);\n");
                for (ci, constraints) in arms {
                    s.push_str("        {\n");
                    if spec.classes.len() > 1 {
                        s.push_str(&format!(
                            "            Z3_ast arm = Z3_mk_eq(c, ev[i], z3_num(c, CLS_BITS, {ci}ull));\n"
                        ));
                    } else {
                        s.push_str("            Z3_ast arm = Z3_mk_true(c);\n");
                    }
                    for (pos, (g, v, _isb)) in constraints.iter().enumerate() {
                        s.push_str(&format!(
                            "            Z3_ast c{pos} = Z3_mk_eq(c, fld[{g}], z3_num(c, FIELD_WIDTHS[{g}], {v}ull));\n"
                        ));
                        s.push_str(&format!(
                            "            arm = Z3_mk_and(c, 2, (Z3_ast[]){{ arm, c{pos} }});\n"
                        ));
                    }
                    s.push_str("            val = Z3_mk_or(c, 2, (Z3_ast[]){ val, arm });\n");
                    s.push_str("        }\n");
                }
                s.push_str("        return val;\n    }");
            }
            s.push_str("    return Z3_mk_false(c);\n}\n\n");
        }
        // Whether the fallback's emits match an output_has atom definition at
        // step i (group-level patterns match ANY variant of the group - exact
        // mode's output_matches rule).
        if spec.output_atoms.is_empty() {
            s.push_str(
                "static Z3_ast build_on_error_emit_match(Z3_context c, unsigned a, unsigned i, const Z3_ast *v, const Z3_ast *fld) {\n    (void)c; (void)a; (void)i; (void)v; (void)fld;\n    return Z3_mk_false(c);\n}\n\n",
            );
        } else {
            let max_args = oe
                .emits
                .iter()
                .map(|o| o.args.len())
                .max()
                .unwrap_or(0)
                .max(1);
            s.push_str(
                "static Z3_ast build_on_error_emit_match(Z3_context c, unsigned a, unsigned i, const Z3_ast *v, const Z3_ast *fld) {\n",
            );
            for (a, (group, variant, constraints)) in spec.output_atoms.iter().enumerate() {
                s.push_str(&format!("    if (a == {a}) {{\n"));
                s.push_str("        Z3_ast val = Z3_mk_false(c);\n");
                s.push_str("        for (unsigned pi = 0; pi < N_OE_EMITS; pi++) {\n");
                s.push_str(&format!(
                    "            if (strcmp(OE_EMIT_GROUP[pi], \"{group}\") == 0 && {} ) {{\n",
                    match variant {
                        Some(v) => format!("strcmp(OE_EMIT_VARIANT[pi], \"{v}\") == 0"),
                        None => "1".to_string(),
                    }
                ));
                s.push_str(&format!("            {{ Z3_ast args[{max_args}];\n"));
                s.push_str("                OE_EMIT_ARGS_FNS[pi](c, i, v, fld, args);\n");
                s.push_str("                Z3_ast m = Z3_mk_true(c);\n");
                for (pos, v, _isb) in constraints {
                    s.push_str(&format!(
                        "                m = Z3_mk_and(c, 2, (Z3_ast[]){{ m, Z3_mk_eq(c, args[{pos}], z3_num(c, OE_EMIT_ARGW[pi][{pos}], {v}ull)) }});\n"
                    ));
                }
                s.push_str("                val = Z3_mk_or(c, 2, (Z3_ast[]){ val, m });\n            }\n        }\n        return val;\n    }");
            }
            s.push_str("    return Z3_mk_false(c);\n}\n\n");
        }
    }

    // ---- Assumptions (task 3C, D3) - emitted ONLY when the module has any,
    // so specs without them generate byte-identical code to before. ----
    if !spec.assumptions.is_empty() {
        let anames: Vec<String> = spec.assumptions.iter().map(|a| a.name.clone()).collect();
        s.push_str(&format!(
            "/* Assumptions (task 3C): per-sample base constraints - exact-mode\n * successor pruning expressed as SMT facts (SEMANTICS 7.x). */\nstatic const char *ASSUME_NAMES[] = {};\n#define N_ASSUME {1}u\n",
            c_str_list(&anames),
            spec.assumptions.len()
        ));
        s.push_str(&format!(
            "/* FULL class (asserted only with --full-assumptions). */\nstatic const unsigned char ASSUME_FULL[] = {{{}}};\n",
            spec.assumptions
                .iter()
                .map(|a| if a.full { "1" } else { "0" })
                .collect::<Vec<_>>()
                .join(", ")
        ));
        s.push_str(&format!(
            "/* always[N] sample clip (0 = all samples). */\nstatic const unsigned ASSUME_BOUNDS[] = {{{}}};\n",
            spec.assumptions
                .iter()
                .map(|a| format!("{}u", a.bound.unwrap_or(0)))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        s.push_str(&format!(
            "/* next_depth anchor clip: the last N anchors have no shifted horizon and are dropped (same clipping as properties). */\nstatic const unsigned ASSUME_NEXT_DEPTH[] = {{{}}};\n",
            spec.assumptions
                .iter()
                .map(|a| format!("{}u", a.next_depth))
                .collect::<Vec<_>>()
                .join(", ")
        ));
        // One builder fn per assumption - same signature as the property fns
        // (FULL-class bodies render horizon loops that need k; next bodies
        // need the D1 alive gate). Unused parameters get void-casts exactly
        // like expr_p{p} (warning-free generated code).
        for (a, ass) in spec.assumptions.iter().enumerate() {
            let mut r = CRender::new();
            let fin = render_sf_c(&ass.formula, spec, "i", &mut r);
            let ii_name = if sf_uses_input_atoms(&ass.formula) {
                "ii"
            } else {
                "_ii"
            };
            let oh_name = if sf_uses_output_atoms(&ass.formula) {
                "oh"
            } else {
                "_oh"
            };
            let alive_name = if sf_uses_next(&ass.formula) {
                "alive"
            } else {
                "_alive"
            };
            let voids = format!(
                "(void)s; (void)v; (void)fld;{}{}{}",
                if sf_uses_horizon(&ass.formula) {
                    String::new()
                } else {
                    " (void)k;".to_string()
                },
                if ii_name == "_ii" { " (void)_ii;" } else { "" },
                if oh_name == "_oh" { " (void)_oh;" } else { "" },
            );
            let void_alive = if alive_name == "_alive" {
                " (void)_alive;"
            } else {
                ""
            };
            let void_i = if sf_uses_step(&ass.formula) {
                String::new()
            } else {
                "\n    (void)i;".to_string()
            };
            s.push_str(&format!(
                "static Z3_ast assume_f{a}(Z3_context c, unsigned i, unsigned k, Z3_ast s, const Z3_ast *v, const Z3_ast *fld, const Z3_ast *{alive_name}, const Z3_ast *{ii_name}, const Z3_ast *{oh_name}) {{\n    {voids}{void_alive}{void_i}\n    {}\n    return {fin};\n}}\n\n",
                r.lines.join("\n    ")
            ));
        }
        s.push_str(&format!(
            "static Z3_ast (*ASSUME_FNS[N_ASSUME])(Z3_context, unsigned, unsigned, Z3_ast, const Z3_ast *, const Z3_ast *, const Z3_ast *, const Z3_ast *, const Z3_ast *) = {{{}}};\n",
            (0..spec.assumptions.len()).map(|a| format!("assume_f{a}")).collect::<Vec<_>>().join(", ")
        ));
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::mc::smt_plan::{SmtClass, SmtProp, SmtPropKind, SmtTrans, SmtVar};

    /// Minimal spec for renderer tests: 2 states, vars counter(bit<8>)/flag(bool)/big(bit<16>),
    /// classes Go_F(x:bit<8>, y:bit<16>, ok:bool) and Noop().
    fn test_spec() -> SmtSpec {
        SmtSpec {
            states: vec!["A".into(), "B".into()],
            initial_state: 0,
            state_bits: 1,
            vars: vec![
                SmtVar {
                    name: "counter".into(),
                    width: 8,
                    is_bool: false,
                    init: 0,
                    is_enum: false,
                    enum_variants: Vec::new(),
                },
                SmtVar {
                    name: "flag".into(),
                    width: 1,
                    is_bool: true,
                    init: 0,
                    is_enum: false,
                    enum_variants: Vec::new(),
                },
                SmtVar {
                    name: "big".into(),
                    width: 16,
                    is_bool: false,
                    init: 256,
                    is_enum: false,
                    enum_variants: Vec::new(),
                },
            ],
            classes: vec![
                SmtClass {
                    name: "Go_F".into(),
                    fields: vec![
                        ("x".into(), 8, false, Vec::new()),
                        ("y".into(), 16, false, Vec::new()),
                        ("ok".into(), 1, true, Vec::new()),
                    ],
                },
                SmtClass {
                    name: "Noop".into(),
                    fields: Vec::new(),
                },
            ],
            transitions: vec![
                // t0: A->B on Go_F where x > 10 && ok; counter += x; flag = true.
                SmtTrans {
                    state: 0,
                    next_state: 1,
                    classes: vec![0],
                    guard: Some(Se::Bin(
                        Sb::LogicAnd,
                        Box::new(Se::Bin(
                            Sb::Gt,
                            Box::new(Se::Field {
                                idx: 0,
                                width: 8,
                                is_bool: false,
                            }),
                            Box::new(Se::Lit {
                                value: 10,
                                is_bool: false,
                            }),
                        )),
                        Box::new(Se::Field {
                            idx: 2,
                            width: 1,
                            is_bool: true,
                        }),
                    )),
                    assignments: vec![
                        (
                            0,
                            Se::Bin(
                                Sb::Plus,
                                Box::new(Se::Var(0)),
                                Box::new(Se::Field {
                                    idx: 0,
                                    width: 8,
                                    is_bool: false,
                                }),
                            ),
                        ),
                        (
                            1,
                            Se::Lit {
                                value: 1,
                                is_bool: true,
                            },
                        ),
                    ],
                    emits: Vec::new(),
                },
                // t1: A->A on Noop where counter < 4; counter <<= 2; big >>= 1.
                SmtTrans {
                    state: 0,
                    next_state: 0,
                    classes: vec![1],
                    guard: Some(Se::Bin(
                        Sb::Lt,
                        Box::new(Se::Var(0)),
                        Box::new(Se::Lit {
                            value: 4,
                            is_bool: false,
                        }),
                    )),
                    assignments: vec![
                        (
                            0,
                            Se::Bin(
                                Sb::ShiftLeft,
                                Box::new(Se::Var(0)),
                                Box::new(Se::Lit {
                                    value: 2,
                                    is_bool: false,
                                }),
                            ),
                        ),
                        (
                            2,
                            Se::Bin(
                                Sb::ShiftRight,
                                Box::new(Se::Var(2)),
                                Box::new(Se::Lit {
                                    value: 1,
                                    is_bool: false,
                                }),
                            ),
                        ),
                    ],
                    emits: Vec::new(),
                },
                // t2: B->B on Go_F; big <<= x (narrow amount); counter >>= y (wide amount).
                SmtTrans {
                    state: 1,
                    next_state: 1,
                    classes: vec![0],
                    guard: None,
                    assignments: vec![
                        (
                            2,
                            Se::Bin(
                                Sb::ShiftLeft,
                                Box::new(Se::Var(2)),
                                Box::new(Se::Field {
                                    idx: 0,
                                    width: 8,
                                    is_bool: false,
                                }),
                            ),
                        ),
                        (
                            0,
                            Se::Bin(
                                Sb::ShiftRight,
                                Box::new(Se::Var(0)),
                                Box::new(Se::Field {
                                    idx: 1,
                                    width: 16,
                                    is_bool: false,
                                }),
                            ),
                        ),
                    ],
                    emits: Vec::new(),
                },
                // t3: B->A on Noop; big -= 300 (oversized literal, arithmetic truncates).
                SmtTrans {
                    state: 1,
                    next_state: 0,
                    classes: vec![1],
                    guard: None,
                    assignments: vec![(
                        2,
                        Se::Bin(
                            Sb::Minus,
                            Box::new(Se::Var(2)),
                            Box::new(Se::Lit {
                                value: 300,
                                is_bool: false,
                            }),
                        ),
                    )],
                    emits: Vec::new(),
                },
            ],
            properties: vec![
                // counter < 256 (oversized literal comparison - must widen).
                SmtProp {
                    name: "counter_bounded".into(),
                    kind: SmtPropKind::Always,
                    bound: None,
                    full: false,
                    next_depth: 0,
                    formula: Sf::Se(Se::Bin(
                        Sb::Lt,
                        Box::new(Se::Var(0)),
                        Box::new(Se::Lit {
                            value: 256,
                            is_bool: false,
                        }),
                    )),
                },
                // big >= 0 (fits).
                SmtProp {
                    name: "big_nonneg".into(),
                    kind: SmtPropKind::Eventually,
                    bound: None,
                    full: false,
                    next_depth: 0,
                    formula: Sf::Se(Se::Bin(
                        Sb::Ge,
                        Box::new(Se::Var(2)),
                        Box::new(Se::Lit {
                            value: 0,
                            is_bool: false,
                        }),
                    )),
                },
            ],
            input_atoms: Vec::new(),
            output_atoms: Vec::new(),
            assumptions: Vec::new(),
            on_error: None,
        }
    }

    /// Test helper: (left, right) operands of a Bin node.
    fn bin_ops(e: &Se) -> (&Se, &Se) {
        match e {
            Se::Bin(_, l, r) => (l, r),
            _ => panic!("not a bin"),
        }
    }

    #[test]
    fn cmp_widths_full_fits_and_widens() {
        let spec = test_spec();
        // counter(bit<8>) < 4: literal fits - no widening.
        let e = Se::Bin(
            Sb::Lt,
            Box::new(Se::Var(0)),
            Box::new(Se::Lit {
                value: 4,
                is_bool: false,
            }),
        );
        let (l, r) = bin_ops(&e);
        assert_eq!(cmp_widths_full(l, r, &spec), (8, 8, 0, 0));
        // counter < 256: literal needs 9 bits - left side widens by 1.
        let e = Se::Bin(
            Sb::Lt,
            Box::new(Se::Var(0)),
            Box::new(Se::Lit {
                value: 256,
                is_bool: false,
            }),
        );
        let (l, r) = bin_ops(&e);
        assert_eq!(cmp_widths_full(l, r, &spec), (8, 9, 1, 0));
        // 300 == counter: literal side renders at final width, ext 0.
        let e = Se::Bin(
            Sb::Eq,
            Box::new(Se::Lit {
                value: 300,
                is_bool: false,
            }),
            Box::new(Se::Var(0)),
        );
        let (l, r) = bin_ops(&e);
        assert_eq!(cmp_widths_full(l, r, &spec), (9, 8, 0, 1));
        // two literals: both render at the widened width.
        let e = Se::Bin(
            Sb::Eq,
            Box::new(Se::Lit {
                value: 5,
                is_bool: false,
            }),
            Box::new(Se::Lit {
                value: 300,
                is_bool: false,
            }),
        );
        let (l, r) = bin_ops(&e);
        assert_eq!(cmp_widths_full(l, r, &spec), (9, 9, 0, 0));
        // bool vs bool: width 1, no widening.
        let e = Se::Bin(
            Sb::Eq,
            Box::new(Se::Var(1)),
            Box::new(Se::Lit {
                value: 1,
                is_bool: true,
            }),
        );
        let (l, r) = bin_ops(&e);
        assert_eq!(cmp_widths_full(l, r, &spec), (1, 1, 0, 0));
    }

    #[test]
    fn shift_amt_width_rules() {
        let spec = test_spec();
        // bitvector amount keeps its width.
        let amt = Se::Field {
            idx: 0,
            width: 8,
            is_bool: false,
        };
        assert_eq!(shift_amt_width(&amt, &spec, 16), 8);
        // integer literal lifts to at least wl.
        let amt = Se::Lit {
            value: 1,
            is_bool: false,
        };
        assert_eq!(shift_amt_width(&amt, &spec, 8), 8);
        // oversized literal keeps enough bits to stay detectable (20 needs 5).
        let amt = Se::Lit {
            value: 20,
            is_bool: false,
        };
        assert_eq!(shift_amt_width(&amt, &spec, 4), 5);
        // zero literal still needs one bit.
        let amt = Se::Lit {
            value: 0,
            is_bool: false,
        };
        assert_eq!(shift_amt_width(&amt, &spec, 8), 8);
    }

    /// Normalize one rendered builder function body to a sequence of z3 op
    /// tokens (backend syntax stripped). Leaf variables map to the same token
    /// in both backends (`s`, `v_j`, `fld_g`).
    fn normalize_rust(body: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in body.lines() {
            let l = line.trim();
            if !l.starts_with("let e") {
                continue;
            }
            if let Some((_, tail)) = l.split_once(": BV = BV::from_u64(") {
                let inner = tail.trim_end_matches(';').trim_end_matches(')');
                let (v, w) = inner.split_once("u64, ").unwrap();
                out.push(format!("num{{{},{}}}", v, w));
            } else if let Some((_, tail)) = l.split_once(": Bool = Bool::from_bool(") {
                out.push(format!(
                    "bool{{{}}}",
                    tail.trim_end_matches(';').trim_end_matches(')')
                ));
            } else if l.contains("BV::new_const(format!(\"s_{i}\")") {
                out.push("leaf{s}".into());
            } else if let Some(p) = l.find("v_{i}_") {
                let j: String = l[p + 6..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                out.push(format!("leaf{{v_{j}}}"));
            } else if let Some(p) = l.find("fld_{i}_") {
                let g: String = l[p + 8..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                out.push(format!("leaf{{fld_{g}}}"));
            } else if l.ends_with(".not();") {
                out.push("not".into());
            } else if let Some(op) = [
                ".bvnot();",
                ".bvneg();",
                ".eq(&",
                ".bvult(&",
                ".bvule(&",
                ".bvugt(&",
                ".bvuge(&",
                ".concat(&",
                ".bvadd(&",
                ".bvsub(&",
                ".bvmul(&",
                ".bvand(&",
                ".bvor(&",
                ".bvxor(&",
            ]
            .iter()
            .find(|m| l.contains(**m))
            {
                let name = op
                    .trim_start_matches('.')
                    .trim_end_matches("();")
                    .trim_end_matches("(&");
                out.push(name.to_string());
            } else if l.contains(".ite(&") {
                // Shift lines nest the shift op inside the ite: emit both,
                // inner first (matches the C normalizer).
                if let Some(p) = [".bvshl(&", ".bvlshr(&"].iter().find_map(|m| l.find(*m)) {
                    let tail = &l[p..];
                    let end = tail.find('(').unwrap();
                    out.push(tail[1..end].to_string());
                    out.push("ite".into());
                } else {
                    out.push("ite".into());
                }
            } else if let Some(p) = l.find(".extract(") {
                let h: String = l[p + 9..]
                    .split('u')
                    .next()
                    .unwrap_or("")
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                out.push(format!("extract{{{h}}}"));
            } else if l.contains(": Bool = b_and(&") {
                out.push("and".into());
            } else if l.contains(": Bool = b_or(&") {
                out.push("or".into());
            }
        }
        out
    }

    fn normalize_c(body: &str) -> Vec<String> {
        let mut out = Vec::new();
        for line in body.lines() {
            let l = line.trim();
            if !l.starts_with("Z3_ast e") {
                continue;
            }
            if let Some((_, tail)) = l.split_once(" = z3_num(c, ") {
                let inner = tail.trim_end_matches(';').trim_end_matches(')');
                let (w, v) = inner.split_once(", ").unwrap();
                out.push(format!("num{{{},{}}}", v.trim_end_matches("ull"), w));
            } else if l.ends_with("= Z3_mk_true(c);") {
                out.push("bool{true}".into());
            } else if l.ends_with("= Z3_mk_false(c);") {
                out.push("bool{false}".into());
            } else if l.ends_with("= s;") {
                out.push("leaf{s}".into());
            } else if let Some(p) = l.find("= v[") {
                let j: String = l[p + 4..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                out.push(format!("leaf{{v_{j}}}"));
            } else if let Some(p) = l.find("= fld[") {
                let g: String = l[p + 6..]
                    .chars()
                    .take_while(|c| c.is_ascii_digit())
                    .collect();
                out.push(format!("leaf{{fld_{g}}}"));
            } else if let Some((_, tail)) = l.split_once("Z3_mk_") {
                let name = tail.split(['(', ';']).next().unwrap_or("").trim();
                match name {
                    "not" => out.push("not".into()),
                    "eq" => out.push("eq".into()),
                    "concat" => out.push("concat".into()),
                    "and" => out.push("and".into()),
                    "or" => out.push("or".into()),
                    "ite" => {
                        // Shift lines nest the shift op inside the ite: emit
                        // both, inner first (matches the Rust normalizer).
                        if let Some(p) = ["Z3_mk_bvshl(", "Z3_mk_bvlshr("]
                            .iter()
                            .find_map(|m| l.find(m))
                        {
                            let tail = &l[p..];
                            let end = tail.find('(').unwrap();
                            out.push(tail[6..end].to_string());
                        }
                        out.push("ite".into());
                    }
                    "extract" => {
                        let h: String = l
                            .split("Z3_mk_extract(c, ")
                            .nth(1)
                            .unwrap()
                            .chars()
                            .take_while(|c| c.is_ascii_digit())
                            .collect();
                        out.push(format!("extract{{{h}}}"));
                    }
                    n if n.starts_with("bv") => out.push(n.to_string()),
                    _ => {}
                }
            }
        }
        out
    }

    /// Extract one builder function body by name from a rendered spec.
    /// Generated functions are flat: the body ends at the first `\n}`.
    fn fn_body(src: &str, name: &str) -> String {
        let start = src.find(&format!("{name}(")).expect(name);
        let brace = src[start..].find('{').unwrap();
        let rest = &src[start + brace..];
        let end = rest[1..].find("\n}").unwrap() + 1;
        rest[..end].to_string()
    }

    /// Parity (D-P4-12): every builder function renders to the SAME sequence
    /// of z3 operations in Rust and C.
    #[test]
    fn rust_and_c_render_identical_op_sequences() {
        let spec = test_spec();
        let rust = emit_rust_spec(&spec);
        let c = emit_c_spec(&spec);
        for t in 0..4usize {
            for suffix in ["_guard", "_asgn_0", "_asgn_1", "_asgn_2"] {
                let name = format!("expr_t{t}{suffix}");
                if !rust.contains(&format!("fn {name}("))
                    || !c.contains(&format!("{name}(Z3_context"))
                {
                    continue; // not emitted for this (transition, var) pair
                }
                let rt = normalize_rust(&fn_body(&rust, &name));
                let ct = normalize_c(&fn_body(&c, &name));
                assert_eq!(rt, ct, "op sequence mismatch for {name}");
            }
        }
        for p in 0..2usize {
            let name = format!("expr_p{p}");
            let rt = normalize_rust(&fn_body(&rust, &name));
            let ct = normalize_c(&fn_body(&c, &name));
            assert_eq!(rt, ct, "op sequence mismatch for {name}");
        }
    }

    #[test]
    fn oversized_literal_comparison_widens() {
        let spec = test_spec();
        let rust = emit_rust_spec(&spec);
        // counter < 256: literal at width 9, counter zero-extended by 1.
        assert!(
            rust.contains("BV::from_u64(256u64, 9)"),
            "literal must widen to 9 bits"
        );
        let body = fn_body(&rust, "expr_p0");
        assert!(
            body.contains(".concat(&") && body.contains("bvult"),
            "widen + compare"
        );
        let c = emit_c_spec(&spec);
        let cbody = fn_body(&c, "expr_p0");
        assert!(
            cbody.contains("z3_num(c, 9, 256ull)")
                && cbody.contains("Z3_mk_concat")
                && cbody.contains("Z3_mk_bvult")
        );
    }

    #[test]
    fn shift_rendering_paths() {
        let spec = test_spec();
        let rust = emit_rust_spec(&spec);
        // t1: counter << 2 (literal fits wl=8): guard bvult(amt, 8), no concat/extract.
        let body = fn_body(&rust, "expr_t1_asgn_0");
        assert!(
            body.contains("BV::from_u64(2u64, 8)")
                && body.contains("bvult")
                && body.contains("bvshl")
        );
        assert!(
            !body.contains("concat") && !body.contains("extract"),
            "common case needs no widen/truncate"
        );
        // t2: big << x (x bit<8>, wl=16): zero-extend amount by 8.
        let body = fn_body(&rust, "expr_t2_asgn_2");
        assert!(
            body.contains("BV::from_u64(0u64, 8)") && body.contains(".concat(&"),
            "narrow amount must zero-extend"
        );
        // t2: counter >>= y (y bit<16>, wl=8): truncate amount to 8 bits.
        let body = fn_body(&rust, "expr_t2_asgn_0");
        assert!(
            body.contains(".extract(7u32, 0)"),
            "wide amount must truncate"
        );
        // t1: big >> 1 (literal fits wl=16).
        let body = fn_body(&rust, "expr_t1_asgn_2");
        assert!(body.contains("BV::from_u64(1u64, 16)") && body.contains("bvlshr"));
    }
}
