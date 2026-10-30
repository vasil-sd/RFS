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
//! Docs: dev:mc-engine-overview, dev:algorithms-mc-engine, llm:mc-engine

use crate::smt_plan::{se_type, Sb, Se, SeType, Sf, SmtSpec};

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
        Se::Bin(_, l, r) => se_uses_step(l) || se_uses_step(r),
    }
}

fn sf_uses_step(sf: &Sf) -> bool {
    match sf {
        Sf::Se(e) => se_uses_step(e),
        Sf::Not(i) => sf_uses_step(i),
        Sf::And(l, r) | Sf::Or(l, r) | Sf::Implies(l, r) => sf_uses_step(l) || sf_uses_step(r),
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
fn render_se_rust(
    se: &Se,
    spec: &SmtSpec,
    expected: Option<u32>,
    class: Option<usize>,
    bool_as_bv: bool,
    r: &mut RustRender,
) -> String {
    match se {
        Se::Lit { value, is_bool } => {
            if *is_bool && !bool_as_bv {
                r.push(format!(
                    "let e{}: Bool = Bool::from_bool({});",
                    r.n,
                    *value != 0
                ))
            } else {
                let w = expected.unwrap_or(8);
                r.push(format!("let e{}: BV = BV::from_u64({value}u64, {w});", r.n))
            }
        }
        Se::State => r.push(format!(
            "let e{}: BV = BV::new_const(format!(\"s_{{i}}\"), STATE_BITS);",
            r.n
        )),
        Se::Var(j) => {
            let leaf = format!("BV::new_const(format!(\"v_{{i}}_{j}\"), VAR_WIDTHS[{j}])");
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
            let leaf = format!("BV::new_const(format!(\"fld_{{i}}_{g}\"), FIELD_WIDTHS[{g}])");
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
            let c = render_se_rust(inner, spec, Some(1), class, false, r);
            let b = r.push(format!("let e{}: Bool = {c}.not();", r.n));
            finish_bool_rust(&b, bool_as_bv, r)
        }
        Se::BitNot(inner) | Se::Neg(inner) => {
            let w = se_type(inner, spec).width();
            let c = render_se_rust(inner, spec, Some(w), class, false, r);
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
                let b = render_bool_bin_rust(*op, l, rr, spec, class, r);
                finish_bool_rust(&b, bool_as_bv, r)
            } else {
                render_bit_bin_rust(*op, l, rr, spec, class, r)
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
            let bits = if *value == 0 {
                1
            } else {
                64 - value.leading_zeros()
            };
            w = w.max(bits);
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
    r: &mut RustRender,
) -> String {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    match op {
        Sb::LogicAnd | Sb::LogicOr => {
            let a = render_se_rust(l, spec, Some(1), class, false, r);
            let b = render_se_rust(rr, spec, Some(1), class, false, r);
            let m = if op == Sb::LogicAnd { "and" } else { "or" };
            r.push(format!("let e{}: Bool = b_{m}(&{a}, &{b});", r.n))
        }
        Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => {
            let (wl, wr, ext_l, ext_r) = cmp_widths_full(l, rr, spec);
            let a0 = render_se_rust(l, spec, Some(wl), class, false, r);
            let b0 = render_se_rust(rr, spec, Some(wr), class, false, r);
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
            let a = render_se_rust(l, spec, Some(1), class, false, r);
            let b = render_se_rust(rr, spec, Some(1), class, false, r);
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
            let a = render_se_rust(l, spec, Some(w), class, false, r);
            let b = render_se_rust(rr, spec, Some(w), class, false, r);
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
        Sb::ShiftLeft | Sb::ShiftRight => render_shift_rust(op, l, rr, spec, class, r),
        _ => unreachable!("bit-producing op only"),
    }
}

fn render_sf_rust(sf: &Sf, spec: &SmtSpec, r: &mut RustRender) -> String {
    match sf {
        Sf::Se(inner) => render_se_rust(inner, spec, Some(1), None, false, r),
        Sf::Not(inner) => {
            let c = render_sf_rust(inner, spec, r);
            r.push(format!("let e{}: Bool = {c}.not();", r.n))
        }
        Sf::And(l, rr) => {
            let a = render_sf_rust(l, spec, r);
            let b = render_sf_rust(rr, spec, r);
            r.push(format!("let e{}: Bool = b_and(&{a}, &{b});", r.n))
        }
        Sf::Or(l, rr) => {
            let a = render_sf_rust(l, spec, r);
            let b = render_sf_rust(rr, spec, r);
            r.push(format!("let e{}: Bool = b_or(&{a}, &{b});", r.n))
        }
        Sf::Implies(l, rr) => {
            let a = render_sf_rust(l, spec, r);
            let b = render_sf_rust(rr, spec, r);
            r.push(format!("let e{}: Bool = {a}.implies(&{b});", r.n))
        }
    }
}

fn rust_str_list(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| format!("{s:?}")).collect();
    format!("[{}]", inner.join(", "))
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
    let cnames: Vec<String> = spec.classes.iter().map(|c| c.name.clone()).collect();
    s.push_str(&format!(
        "const CLASS_NAMES: &[&str] = &{};\n",
        rust_str_list(&cnames)
    ));
    s.push_str(&format!(
        "const N_CLASSES: usize = {};\n",
        spec.classes.len()
    ));
    let cls_bits = crate::smt_plan::ceil_log2_pub(spec.classes.len());
    s.push_str(&format!("const CLS_BITS: u32 = {cls_bits};\n"));
    let fnames: Vec<String> = spec
        .classes
        .iter()
        .flat_map(|c| c.fields.iter().map(|(n, _, _)| n.clone()))
        .collect();
    s.push_str(&format!(
        "const FIELD_NAMES: &[&str] = &{};\n",
        rust_str_list(&fnames)
    ));
    s.push_str(&format!(
        "const FIELD_WIDTHS: &[u32] = &{:?};\n",
        spec.classes
            .iter()
            .flat_map(|c| c.fields.iter().map(|(_, w, _)| *w))
            .collect::<Vec<_>>()
    ));
    s.push_str(&format!(
        "const FIELD_IS_BOOL: &[bool] = &{:?};\n",
        spec.classes
            .iter()
            .flat_map(|c| c.fields.iter().map(|(_, _, b)| *b))
            .collect::<Vec<_>>()
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
        .map(|t| format!("({}, {}, {})", t.state, t.next_state, t.class))
        .collect();
    s.push_str(&format!(
        "/// Transitions in firing order: (state, next_state, class).\nconst TRANS: &[(usize, usize, usize)] = &[{}];\n",
        trans.join(", ")
    ));
    let pnames: Vec<String> = spec.properties.iter().map(|p| p.name.clone()).collect();
    s.push_str(&format!(
        "const PROP_NAMES: &[&str] = &{};\n",
        rust_str_list(&pnames)
    ));
    s.push_str(&format!(
        "/// 0 = always (safety), 1 = eventually (liveness).\nconst PROP_KINDS: &[u8] = &{:?};\n",
        spec.properties
            .iter()
            .map(|p| u8::from(p.kind == crate::smt_plan::SmtPropKind::Eventually))
            .collect::<Vec<_>>()
    ));

    s.push_str("\nuse z3::ast::{BV, Bool};\n\n");

    // Transition guards (builder takes the step index; leaves are recreated
    // constants - hash-consed to the engine's step variables).
    for (t, tr) in spec.transitions.iter().enumerate() {
        let mut r = RustRender::new();
        let uses_i = tr.guard.as_ref().is_some_and(se_uses_step);
        match &tr.guard {
            Some(g) => render_se_rust(g, spec, Some(1), Some(tr.class), false, &mut r),
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
                    Some(tr.class),
                    true,
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
    // Property formulas.
    for (p, prop) in spec.properties.iter().enumerate() {
        let mut r = RustRender::new();
        let uses_i = sf_uses_step(&prop.formula);
        render_sf_rust(&prop.formula, spec, &mut r);
        s.push_str(&format!(
            "fn expr_p{p}({q}: usize) -> Bool {{\n    {}\n    e{}\n}}\n\n",
            r.lines.join("\n    "),
            r.n - 1,
            q = if uses_i { "i" } else { "_i" }
        ));
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
        "fn prop_fns() -> Vec<fn(usize) -> Bool> {{ vec![{}] }}\n",
        props.join(", ")
    ));
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
            let bits = if v == 0 { 1 } else { 64 - v.leading_zeros() };
            wl.max(bits)
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
    r: &mut RustRender,
) -> String {
    let wl = se_type(l, spec).width();
    let w_r = shift_amt_width(rr, spec, wl);
    let w_c = w_r.max(wl);
    let a = render_se_rust(l, spec, None, class, false, r);
    let b = render_se_rust(rr, spec, Some(w_r), class, false, r);
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
    r: &mut CRender,
) -> String {
    let wl = se_type(l, spec).width();
    let w_r = shift_amt_width(rr, spec, wl);
    let w_c = w_r.max(wl);
    let a = render_se_c(l, spec, None, class, false, r);
    let b = render_se_c(rr, spec, Some(w_r), class, false, r);
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
    r: &mut CRender,
) -> String {
    match se {
        Se::Lit { value, is_bool } => {
            if *is_bool && !bool_as_bv {
                r.push(format!(
                    "Z3_ast e{} = Z3_mk_{}(c);",
                    r.n,
                    if *value != 0 { "true" } else { "false" }
                ))
            } else {
                let w = expected.unwrap_or(8);
                r.push(format!("Z3_ast e{} = z3_num(c, {w}, {value}ull);", r.n))
            }
        }
        Se::State => r.push(format!("Z3_ast e{} = s;", r.n)),
        Se::Var(j) => {
            let leaf = format!("v[{j}]");
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
            let leaf = format!("fld[{g}]");
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
            let c_ = render_se_c(inner, spec, Some(1), class, false, r);
            let b = r.push(format!("Z3_ast e{} = Z3_mk_not(c, {c_});", r.n));
            finish_bool_c(&b, bool_as_bv, r)
        }
        Se::BitNot(inner) | Se::Neg(inner) => {
            let w = se_type(inner, spec).width();
            let c_ = render_se_c(inner, spec, Some(w), class, false, r);
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
                let b = render_bool_bin_c(*op, l, rr, spec, class, r);
                finish_bool_c(&b, bool_as_bv, r)
            } else {
                render_bit_bin_c(*op, l, rr, spec, class, r)
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
    r: &mut CRender,
) -> String {
    let lt = se_type(l, spec);
    let rt = se_type(rr, spec);
    match op {
        Sb::LogicAnd | Sb::LogicOr => {
            let a = render_se_c(l, spec, Some(1), class, false, r);
            let b = render_se_c(rr, spec, Some(1), class, false, r);
            let m = if op == Sb::LogicAnd { "and" } else { "or" };
            let an = format!("ab{}", r.n);
            r.push(format!("Z3_ast {an}[2] = {{ {a}, {b} }};"));
            r.push(format!("Z3_ast e{} = Z3_mk_{m}(c, 2, {an});", r.n))
        }
        Sb::Eq | Sb::NotEq | Sb::Lt | Sb::Le | Sb::Gt | Sb::Ge => {
            let (wl, wr, ext_l, ext_r) = cmp_widths_full(l, rr, spec);
            let a0 = render_se_c(l, spec, Some(wl), class, false, r);
            let b0 = render_se_c(rr, spec, Some(wr), class, false, r);
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
            let a = render_se_c(l, spec, Some(1), class, false, r);
            let b = render_se_c(rr, spec, Some(1), class, false, r);
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
            let a = render_se_c(l, spec, Some(w), class, false, r);
            let b = render_se_c(rr, spec, Some(w), class, false, r);
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
        Sb::ShiftLeft | Sb::ShiftRight => render_shift_c(op, l, rr, spec, class, r),
        _ => unreachable!("bit-producing op only"),
    }
}

fn render_sf_c(sf: &Sf, spec: &SmtSpec, r: &mut CRender) -> String {
    match sf {
        Sf::Se(inner) => render_se_c(inner, spec, Some(1), None, false, r),
        Sf::Not(inner) => {
            let c_ = render_sf_c(inner, spec, r);
            r.push(format!("Z3_ast e{} = Z3_mk_not(c, {c_});", r.n))
        }
        Sf::And(l, rr) => {
            let a = render_sf_c(l, spec, r);
            let b = render_sf_c(rr, spec, r);
            r.push(format!(
                "Z3_ast e{} = Z3_mk_and(c, 2, (Z3_ast[]){{{a}, {b}}});",
                r.n
            ))
        }
        Sf::Or(l, rr) => {
            let a = render_sf_c(l, spec, r);
            let b = render_sf_c(rr, spec, r);
            r.push(format!(
                "Z3_ast e{} = Z3_mk_or(c, 2, (Z3_ast[]){{{a}, {b}}});",
                r.n
            ))
        }
        Sf::Implies(l, rr) => {
            let a = render_sf_c(l, spec, r);
            let b = render_sf_c(rr, spec, r);
            r.push(format!("Z3_ast e{} = Z3_mk_implies(c, {a}, {b});", r.n))
        }
    }
}

fn c_str_list(items: &[String]) -> String {
    let inner: Vec<String> = items.iter().map(|s| format!("{s:?}")).collect();
    format!("{{{}}}", inner.join(", "))
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
    let cnames: Vec<String> = spec.classes.iter().map(|c| c.name.clone()).collect();
    s.push_str(&format!(
        "static const char *CLASS_NAMES[] = {};\n",
        c_str_list(&cnames)
    ));
    s.push_str(&format!("#define N_CLASSES {0}u\n", spec.classes.len()));
    let cls_bits = crate::smt_plan::ceil_log2_pub(spec.classes.len());
    s.push_str(&format!("#define CLS_BITS {cls_bits}u\n"));
    let fnames: Vec<String> = spec
        .classes
        .iter()
        .flat_map(|c| c.fields.iter().map(|(n, _, _)| n.clone()))
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
        .flat_map(|c| c.fields.iter().map(|(_, w, _)| format!("{w}u")))
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
                .map(|(_, _, b)| (if *b { 1 } else { 0 }).to_string())
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
        .map(|t| format!("{{{0}u, {1}u, {2}u}}", t.state, t.next_state, t.class))
        .collect();
    s.push_str(&format!(
        "/* Transitions in firing order: (state, next_state, class). */\nstatic const struct {{ unsigned state, next_state, cls; }} TRANS[] = {{{}}};\n#define N_TRANS {1}u\n",
        trans.join(", "),
        spec.transitions.len()
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
            (if p.kind == crate::smt_plan::SmtPropKind::Eventually {
                1
            } else {
                0
            })
            .to_string()
        })
        .collect();
    s.push_str(&format!(
        "/* 0 = always (safety), 1 = eventually (liveness). */\nstatic const unsigned char PROP_KINDS[] = {{{}}};\n#define N_PROPS {1}u\n",
        kinds.join(", "),
        spec.properties.len()
    ));

    s.push_str("\nstatic Z3_ast z3_num(Z3_context c, unsigned w, uint64_t v) {\n");
    s.push_str("    return Z3_mk_unsigned_int64(c, v, Z3_mk_bv_sort(c, w));\n}\n\n");

    // Transition guards.
    for (t, tr) in spec.transitions.iter().enumerate() {
        let mut r = CRender::new();
        match &tr.guard {
            Some(g) => render_se_c(g, spec, Some(1), Some(tr.class), false, &mut r),
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
                    Some(tr.class),
                    true,
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
    // Property formulas.
    for (p, prop) in spec.properties.iter().enumerate() {
        let mut r = CRender::new();
        render_sf_c(&prop.formula, spec, &mut r);
        s.push_str(&format!(
            "static Z3_ast expr_p{p}(Z3_context c, Z3_ast s, const Z3_ast *v, const Z3_ast *fld) {{\n    (void)s; (void)v; (void)fld;\n    {}\n    return e{};\n}}\n\n",
            r.lines.join("\n    "),
            r.n - 1
        ));
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
        "static Z3_ast (*PROP_FNS[N_PROPS])(Z3_context, Z3_ast, const Z3_ast *, const Z3_ast *) = {{{}}};\n",
        props.join(", ")
    ));
    s
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::smt_plan::{SmtClass, SmtProp, SmtPropKind, SmtTrans, SmtVar};

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
                },
                SmtVar {
                    name: "flag".into(),
                    width: 1,
                    is_bool: true,
                    init: 0,
                },
                SmtVar {
                    name: "big".into(),
                    width: 16,
                    is_bool: false,
                    init: 256,
                },
            ],
            classes: vec![
                SmtClass {
                    name: "Go_F".into(),
                    fields: vec![
                        ("x".into(), 8, false),
                        ("y".into(), 16, false),
                        ("ok".into(), 1, true),
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
                    class: 0,
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
                },
                // t1: A->A on Noop where counter < 4; counter <<= 2; big >>= 1.
                SmtTrans {
                    state: 0,
                    next_state: 0,
                    class: 1,
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
                },
                // t2: B->B on Go_F; big <<= x (narrow amount); counter >>= y (wide amount).
                SmtTrans {
                    state: 1,
                    next_state: 1,
                    class: 0,
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
                },
                // t3: B->A on Noop; big -= 300 (oversized literal, arithmetic truncates).
                SmtTrans {
                    state: 1,
                    next_state: 0,
                    class: 1,
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
                },
            ],
            properties: vec![
                // counter < 256 (oversized literal comparison - must widen).
                SmtProp {
                    name: "counter_bounded".into(),
                    kind: SmtPropKind::Always,
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
