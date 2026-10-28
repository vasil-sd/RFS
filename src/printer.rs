//! IR-to-RFS source printer (unparser) and round-trip helpers.
//!
//! The printer produces canonical formatted RFS v0 source from an IR document.
//! Printed output always parses back to an equivalent IR:
//!
//! - `rfs -> json -> rfs` is stable (`roundtrip_rfs`);
//! - `json -> rfs -> json` is stable (`roundtrip_json`).
//!
//! Notes:
//! - Printed modules group items by category (enums, initial, events, outputs,
//!   vars, transitions, properties, assumptions). The IR does not preserve the
//!   original interleaved item order, and item order is not semantically
//!   significant in RFS v0.
//! - A `PatternField` without a name whose pattern is a bind cannot be
//!   represented in RFS syntax; such JSON-only shapes are printed as a bare
//!   bind (which re-parses to a named field). Parser-produced IR never has this
//!   shape.
//! - The printer assumes parser-invariant IR shapes (unique constructor groups,
//!   valid variant names, ...); hand-crafted JSON that violates those invariants
//!   may not round-trip exactly.
//! Docs: dev:code-map-frontend

use crate::ir::*;
use std::fmt::Write as _;

/// Print an entire IR document as formatted RFS source.
pub fn print_ir(ir: &Ir) -> String {
    let mut out = String::new();
    for (i, module) in ir.modules.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        out.push_str(&print_module(module));
        out.push('\n');
    }
    out
}

/// Print a single module as formatted RFS source (no trailing newline).
pub fn print_module(module: &Module) -> String {
    let mut s = String::new();
    let header = match &module.version {
        Some(v) => format!("module {} version \"{}\" {{", module.name, v),
        None => format!("module {} {{", module.name),
    };
    let _ = writeln!(s, "{}", header);
    for e in &module.enums {
        let variants = e.variants.join(", ");
        let _ = writeln!(s, "  enum {} {{ {} }}", e.name, variants);
    }
    if let Some(init) = &module.initial_state {
        let _ = writeln!(s, "  initial {}", init);
    }
    for line in print_constructor_lines("event", &module.events) {
        let _ = writeln!(s, "  {}", line);
    }
    for line in print_constructor_lines("output", &module.outputs) {
        let _ = writeln!(s, "  {}", line);
    }
    for v in &module.vars {
        let _ = writeln!(
            s,
            "  var {}: {} init {}",
            v.name,
            print_type(&v.ty),
            print_expr(&v.init)
        );
    }
    for t in &module.transitions {
        s.push_str(&print_transition(t));
    }
    if let Some(oe) = &module.on_error {
        s.push_str(&print_on_error(oe));
    }
    for p in &module.properties {
        let _ = writeln!(s, "  prop {} : {} ;", p.name, print_formula(&p.formula));
    }
    for a in &module.assumptions {
        let _ = writeln!(s, "  assume {} : {} ;", a.name, print_formula(&a.formula));
    }
    let _ = write!(s, "}}");
    s
}

fn print_on_error(oe: &OnError) -> String {
    let mut s = String::from("  on_error {\n");
    let _ = writeln!(s, "    next {};", oe.next_state);
    for o in &oe.emits {
        let _ = writeln!(s, "    emit {};", print_output_expr(o));
    }
    s.push_str("  }\n");
    s
}

fn print_constructor_lines(kw: &str, groups: &[ConstructorGroup]) -> Vec<String> {
    let mut lines = Vec::new();
    for group in groups {
        for v in &group.variants {
            match v {
                ConstructorVariant::Unit { name } => {
                    if name == &group.name {
                        lines.push(format!("{} {}", kw, name));
                    } else {
                        lines.push(format!("{} {}({})", kw, group.name, name));
                    }
                }
                ConstructorVariant::Struct { name, fields } => {
                    let fs: Vec<String> = fields
                        .iter()
                        .map(|f| format!("{}: {}", f.name, print_type(&f.ty)))
                        .collect();
                    lines.push(format!(
                        "{} {}({}({}))",
                        kw,
                        group.name,
                        name,
                        fs.join(", ")
                    ));
                }
            }
        }
    }
    lines
}

fn print_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "bool".to_string(),
        Type::Bit { width } => format!("bit<{}>", width),
        Type::Enum { name } => name.clone(),
    }
}

fn print_transition(t: &Transition) -> String {
    let mut s = String::new();
    let pattern = print_pattern(&t.pattern.group, &t.pattern.variant);
    match &t.guard {
        Some(guard) => {
            let _ = writeln!(
                s,
                "  transition {} on {} where {} {{",
                t.state,
                pattern,
                print_expr(guard)
            );
        }
        None => {
            let _ = writeln!(s, "  transition {} on {} {{", t.state, pattern);
        }
    }
    let _ = writeln!(s, "    next {};", t.next_state);
    for o in &t.emits {
        let _ = writeln!(s, "    emit {};", print_output_expr(o));
    }
    for a in &t.assignments {
        let _ = writeln!(s, "    set {} = {};", a.var, print_expr(&a.value));
    }
    if let Some(p) = t.priority {
        let _ = writeln!(s, "    priority {};", p);
    }
    let _ = writeln!(s, "  }}");
    s
}

// ---------------------------------------------------------------------------
// Patterns
// ---------------------------------------------------------------------------

fn print_pattern(group: &str, variant: &Option<PatternVariant>) -> String {
    match variant {
        None => group.to_string(),
        Some(PatternVariant::Unit { name }) => format!("{}({})", group, name),
        Some(PatternVariant::Struct { name, fields }) => {
            let fs: Vec<String> = fields.iter().map(print_pattern_field).collect();
            format!("{}({}({}))", group, name, fs.join(", "))
        }
    }
}

fn print_pattern_field(pf: &PatternField) -> String {
    match (&pf.name, &pf.pattern) {
        (Some(n), PatternExpr::Bind { name }) if n == name => n.clone(),
        (Some(n), _) => format!("{}: {}", n, print_pattern_expr(&pf.pattern)),
        (None, _) => print_pattern_expr(&pf.pattern),
    }
}

fn print_pattern_expr(pe: &PatternExpr) -> String {
    match pe {
        PatternExpr::Wildcard => "_".to_string(),
        PatternExpr::Bind { name } => name.clone(),
        PatternExpr::Int { value } => value.to_string(),
        PatternExpr::Bool { value } => value.to_string(),
        PatternExpr::EnumVariant { path } => path.join("::"),
    }
}

fn print_output_expr(o: &OutputExpr) -> String {
    match &o.variant {
        None => o.group.clone(),
        Some(v) if o.args.is_empty() => format!("{}({})", o.group, v),
        Some(v) => {
            let args: Vec<String> = o.args.iter().map(print_expr).collect();
            format!("{}({}({}))", o.group, v, args.join(", "))
        }
    }
}

// ---------------------------------------------------------------------------
// Expressions
// ---------------------------------------------------------------------------

/// Expression precedence levels, from loosest (1) to tightest.
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

/// Print an expression with correct parenthesization for top-level use.
pub fn print_expr(e: &Expr) -> String {
    print_expr_min(e, 0)
}

fn print_expr_min(e: &Expr, min_prec: u8) -> String {
    let (core, prec) = match e {
        Expr::Int { value } => (value.to_string(), PREC_PRIMARY),
        Expr::Bool { value } => (value.to_string(), PREC_PRIMARY),
        Expr::Path { path } => (path.join("::"), PREC_PRIMARY),
        Expr::Unary { op, expr } => {
            let inner = print_expr_min(expr, PREC_UNARY);
            // Parenthesize compound inners so hand-crafted JSON such as
            // neg(a + b) keeps its meaning.
            let inner = if matches!(expr.as_ref(), Expr::Binary { .. } | Expr::Unary { .. }) {
                format!("({})", inner)
            } else {
                inner
            };
            let opstr = match op {
                UnaryOp::Not => "!",
                UnaryOp::BitNot => "~",
                UnaryOp::Neg => "-",
            };
            (format!("{}{}", opstr, inner), PREC_UNARY)
        }
        Expr::Binary { op, left, right } => {
            let p = bin_prec(*op);
            // Left-associative operators: a same-precedence left child is
            // fine bare; a same-precedence right child needs parentheses.
            let l = print_expr_min(left, p);
            let r = print_expr_min(right, p + 1);
            (format!("{} {} {}", l, bin_str(*op), r), p)
        }
    };
    if prec < min_prec {
        format!("({})", core)
    } else {
        core
    }
}

// ---------------------------------------------------------------------------
// Formulas
// ---------------------------------------------------------------------------

/// The parse context in which a formula node is printed. Each context matches
/// the parser grammar position that will re-parse the printed text:
///
/// - `Top`: start of a formula (prop/assume body, inside temporal parens);
///   parsed by `or_formula`.
/// - `ImpliesLeft`: left of `->`; parsed by `and_formula`.
/// - `OrChild`: child of `||`; parsed by `impl_formula`, followed by `||`.
/// - `ImpliesRight`: right of `->`; parsed by `impl_formula`, followed by
///   `->` or nothing.
/// - `AndChild`: child of `&&`; parsed by `unary_formula`, followed by `&&`.
/// - `UntilLeft`: left operand of `until`; parsed by `neg_operand`, followed
///   by `until` (left-associative: a nested `Until` re-parses bare).
/// - `UntilRight`: right operand of `until`; parsed by a single
///   `neg_operand` (a nested `Until` must be parenthesized).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fctx {
    Top,
    ImpliesLeft,
    OrChild,
    ImpliesRight,
    AndChild,
    UntilLeft,
    UntilRight,
}

/// Whether an expression still carries formula-level boolean operators
/// (`&&`/`||`). After the Phase 3 grammar fix (task 23) the parser builds
/// `Formula::And`/`Formula::Or` for formula-level booleans, so such shapes
/// only survive from old JSON IR; printing them bare under a following
/// `&&`/`||` would change their meaning, so they keep their parentheses.
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

/// Whether a formula node must be parenthesized in the given context to
/// re-parse to an equal IR.
///
/// Background: since the Phase 3 grammar fix (task 23) `primary_formula`
/// falls through only to the NON-BOOLEAN expression level, so formula-level
/// `&&`/`||` are parsed by `and_formula`/`or_formula` into
/// `Formula::And`/`Formula::Or`, each operand independently. Consequently:
/// - `Formula::And` re-parses bare in every formula context (left-associative,
///   tighter than `||` and `->`); the only exception is as the direct inner of
///   `!`, handled by the `Not` printing arm;
/// - `Formula::Or` re-parses bare at top level, under another `||` (left-
///   associative), and right of `->`; it must be parenthesized where a `&&`
///   or `->` would otherwise bind tighter to its left operand (And-child,
///   Implies-left);
/// - a plain `Expr` child prints bare under `||`/`&&`; an `Expr` that still
///   carries `LogicAnd`/`LogicOr` (old JSON IR only) keeps parentheses;
/// - `Not` children must be parenthesized where a `||` or `&&` operator
///   follows (Or/And child positions);
/// - `Implies` must be parenthesized where a boolean operator follows or
///   where another `->` would re-associate it (And-child, Implies-left,
///   Or-child);
/// - temporal/atom keyword nodes are recognized as formula atoms and are safe
///   bare everywhere.
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
        // `!` binds tighter than `until`, so a Not operand re-parses bare in
        // until positions (`!a until b` = `(!a) until b`).
        Formula::Not { .. } => matches!(ctx, Fctx::OrChild | Fctx::AndChild),
        // An implication must be parenthesized where a boolean operator
        // follows: its right side would otherwise absorb the following
        // `||`/`&&` into one expression (Or/And child positions), or it could
        // be re-associated with an outer `->` (left of `->`). In until
        // positions any bare boolean connective would bind tighter than
        // `until` and change the grouping.
        Formula::Implies { .. } => matches!(
            ctx,
            Fctx::AndChild | Fctx::ImpliesLeft | Fctx::OrChild | Fctx::UntilLeft | Fctx::UntilRight
        ),
        // `&&` binds looser than `until`: bare in every context EXCEPT as an
        // until operand, where `a && b until c` would re-parse as
        // `a && (b until c)`.
        Formula::And { .. } => matches!(ctx, Fctx::UntilLeft | Fctx::UntilRight),
        Formula::Or { .. } => matches!(
            ctx,
            Fctx::AndChild | Fctx::ImpliesLeft | Fctx::UntilLeft | Fctx::UntilRight
        ),
        // Left-associative: a nested until on the LEFT re-parses bare
        // (`a until b until c` = `(a until b) until c`); on the RIGHT it must
        // be parenthesized.
        Formula::Until { .. } => matches!(ctx, Fctx::UntilRight),
    }
}

/// Print a formula with correct parenthesization for top-level use.
pub fn print_formula(f: &Formula) -> String {
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
            // `->` is right-associative: a nested implies on the right
            // round-trips bare (`a -> b -> c` re-parses as `a -> (b -> c)`).
            print_formula_ctx(right, Fctx::ImpliesRight)
        ),
        Formula::Not { inner } => {
            // The inner text always starts a fresh formula after `!`. A nested
            // implication must be parenthesized because bare `!a -> b`
            // re-parses as `(!a) -> b`; likewise a nested `&&`/`||` group,
            // because bare `!a && b` re-parses as `(!a) && b`; and a nested
            // `until`, because bare `!a until b` re-parses as `(!a) until b`
            // (`!` binds tighter than `until`).
            let inner_text = match inner.as_ref() {
                Formula::Implies { .. }
                | Formula::Or { .. }
                | Formula::And { .. }
                | Formula::Until { .. } => format!("({})", print_formula_ctx(inner, Fctx::Top)),
                other => print_formula_ctx(other, Fctx::Top),
            };
            format!("!{}", inner_text)
        }
        Formula::Expr { expr } => print_expr(expr),
        Formula::InputIs { pattern } => {
            format!(
                "input_is({})",
                print_pattern(&pattern.group, &pattern.variant)
            )
        }
        Formula::OutputHas { pattern } => format!(
            "output_has({})",
            print_pattern(&pattern.group, &pattern.variant)
        ),
        // Inside the temporal parentheses the inner text is a full formula.
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
        // The parentheses are mandatory by the grammar (`eventually` is a
        // primary alternative: `"eventually" "(" formula ")"`).
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
        format!("({})", core)
    } else {
        core
    }
}

// ---------------------------------------------------------------------------
// Round-trip helpers
// ---------------------------------------------------------------------------

/// Verify `rfs -> json -> rfs` round-trip stability for RFS source text.
///
/// Returns the parsed IR and the re-printed RFS text. The re-printed text
/// parses back to an IR equal to the original parse.
pub fn roundtrip_rfs(source: &str) -> Result<(Ir, String), String> {
    let ir1 = crate::parse_source(source).map_err(|e| e.message)?;
    let json = ir1
        .to_json()
        .map_err(|e| format!("rfs -> json failed: {}", e))?
        .to_string();
    let value = crate::json_rw::parse(&json).map_err(|e| format!("json parse failed: {}", e))?;
    let ir2 = Ir::from_json(&value).map_err(|e| format!("json -> IR failed: {}", e))?;
    if ir1 != ir2 {
        return Err("round-trip rfs -> json -> IR changed the IR".to_string());
    }
    let rfs2 = print_ir(&ir2);
    let ir3 = crate::parse_source(&rfs2)
        .map_err(|e| format!("re-parse of printed RFS failed: {}", e.message))?;
    if ir1 != ir3 {
        return Err("round-trip rfs -> json -> rfs -> IR changed the IR".to_string());
    }
    Ok((ir1, rfs2))
}

/// Verify `json -> rfs -> json` round-trip stability for an IR JSON document.
///
/// Returns the parsed IR and the printed RFS text. Re-parsing the printed RFS
/// yields an IR equal to the original JSON document.
pub fn roundtrip_json(json: &str) -> Result<(Ir, String), String> {
    let value = crate::json_rw::parse(json).map_err(|e| format!("invalid RFS IR JSON: {}", e))?;
    let ir1 = Ir::from_json(&value).map_err(|e| format!("invalid RFS IR JSON: {}", e))?;
    let rfs1 = print_ir(&ir1);
    let ir2 = crate::parse_source(&rfs1)
        .map_err(|e| format!("re-parse of printed RFS failed: {}", e.message))?;
    if ir1 != ir2 {
        return Err("round-trip json -> rfs -> IR changed the IR".to_string());
    }
    let json2 = ir2
        .to_json()
        .map_err(|e| format!("IR -> json failed: {}", e))?
        .to_string();
    let value2 = crate::json_rw::parse(&json2).map_err(|e| format!("json parse failed: {}", e))?;
    let ir3 = Ir::from_json(&value2).map_err(|e| format!("json -> IR failed: {}", e))?;
    if ir2 != ir3 {
        return Err("round-trip rfs -> json changed the IR".to_string());
    }
    Ok((ir1, rfs1))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::parse_source;

    const MODULE_VARS: &str = "var x: bit<8> init 0 var y: bit<8> init 3 var flag: bool init false";

    fn guard_module(expr: &str) -> String {
        format!(
            "module m {{ enum State {{ A, B }} initial A event E {} transition A on E where {} {{ next B; }} }}",
            MODULE_VARS, expr
        )
    }

    #[test]
    fn canonical_module_format() {
        let ir = parse_source(
            "module m { enum State { A, B } initial A event E var x: bit<8> init 0 transition A on E where x == 1 { next B; set x = x + 1; } prop p : always (state == A) ; }",
        )
        .expect("parse");
        let text = print_ir(&ir);
        assert_eq!(
            text,
            "module m {\n  enum State { A, B }\n  initial A\n  event E\n  var x: bit<8> init 0\n  transition A on E where x == 1 {\n    next B;\n    set x = x + 1;\n  }\n  prop p : always (state == A) ;\n}\n"
        );
    }

    #[test]
    fn expression_precedence_roundtrips() {
        let cases = [
            "x + y - x",
            "x - (y - x)",
            "(x + y) * 2",
            "x + y * 2",
            "x << 1 >> 2",
            "x << (1 >> 2)",
            "x & y | 1",
            "(x & y) | 1",
            "x ^ y & 3",
            "~x & 3",
            "-x + y",
            "x < y && y > 0",
            "x < y == true",
            "(x < y) == (y > x)",
            "!flag || flag && true",
            "x | (y ^ 1)",
        ];
        for c in cases {
            let src = guard_module(c);
            roundtrip_rfs(&src).unwrap_or_else(|e| panic!("round-trip failed for '{}': {}", c, e));
        }
    }

    #[test]
    fn expression_printing_is_exact() {
        // Left-associative chains stay bare; right-nested same-precedence gets parens.
        let ir = parse_source(&guard_module("x - y - x")).expect("parse");
        assert!(print_ir(&ir).contains("where x - y - x {"));

        let ir = parse_source(&guard_module("x - (y - x)")).expect("parse");
        assert!(print_ir(&ir).contains("where x - (y - x) {"));

        // Redundant parentheses are normalized away when precedence is
        // unambiguous; the printed form re-parses to the same IR.
        let ir = parse_source(&guard_module("(x & y) | 1")).expect("parse");
        assert!(print_ir(&ir).contains("where x & y | 1 {"));

        // A tighter operator on the right does not need parentheses:
        // `x | (y ^ 1)` normalizes to `x | y ^ 1` (same IR).
        let ir = parse_source(&guard_module("x | (y ^ 1)")).expect("parse");
        assert!(print_ir(&ir).contains("where x | y ^ 1 {"));

        // A looser operator on the right keeps its parentheses.
        let ir = parse_source(
            "module m { enum State { A, B } initial A event E var x: bit<8> init 0 var y: bit<8> init 3 var z: bit<8> init 7 transition A on E where x & (y | z) { next B; } }",
        )
        .expect("parse");
        assert!(print_ir(&ir).contains("where x & (y | z) {"));
    }

    #[test]
    fn formula_precedence_roundtrips() {
        let cases = [
            // Task 23 grammar: `->` binds tighter than `||`, so bare
            // `a || b -> c` parses as `a || (b -> c)`; the printer keeps the
            // implication parenthesized.
            "state == A || state != A -> true",
            // Parenthesized left operand produces a formula-level Or node
            // (now also reachable bare - task 23).
            "(state == A) || (state != A) -> true",
            "state == A && state != A -> false",
            "!(state == A)",
            "next (state == A)",
            "always (input_is(E) -> output_has(O))",
            "bounded_eventually[3] (state != A)",
            // Formula-level Or/And nodes (bare since task 23; parens still
            // accepted).
            "(state == A || state == B) -> true",
            "true -> (state == A && state == B)",
            "((state == A) || (state == B))",
            "((state == A) && (state == B))",
            // Task 23 mixed atom kinds (previously unparseable).
            "state == A && output_has(O)",
            "state == A || output_has(O)",
            "output_has(O) && state == A",
            "(state == A) && output_has(O)",
            "state == A && state != A || state == B",
            "!(state == A && state == B)",
            // Nested implications and mixed structures.
            "(state == A -> state == B) -> true",
            "state == A -> (state == B -> true)",
            "(state == A -> state == B) || true",
            "!state == A && state != B",
        ];
        for c in cases {
            let src = format!(
                "module m {{ enum State {{ A, B }} initial A event E output O prop p : {} ; }}",
                c
            );
            roundtrip_rfs(&src).unwrap_or_else(|e| panic!("round-trip failed for '{}': {}", c, e));
        }
    }

    #[test]
    fn formula_printing_keeps_implicit_precedence() {
        // Task 23 grammar: formula-level `&&` is tighter than `||`, and an
        // implication right of `||` must be parenthesized (bare it would
        // re-associate). Plain-expression operands print bare.
        let ir = parse_source("module m { enum State { A, B } initial A event E prop p1 : state == A && state != A || state == B ; prop p2 : state == A || state != A -> true ; }").expect("parse");
        let text = print_ir(&ir);
        assert!(text.contains("prop p1 : state == A && state != A || state == B ;"));
        assert!(text.contains("prop p2 : state == A || (state != A -> true) ;"));
    }

    #[test]
    fn formula_and_or_with_expr_children_print_minimal() {
        // Task 23: formula-level booleans with plain-expression operands print
        // without redundant parentheses and round-trip in both directions.
        let src = "module m { enum State { A, B } initial A prop p1 : state == A && state == B ; prop p2 : state == A || state == B ; prop p3 : !(state == A && state == B) ; }";
        let (ir, rfs2) = roundtrip_rfs(src).expect("rfs round-trip");
        assert!(
            rfs2.contains("prop p1 : state == A && state == B ;"),
            "p1 over-parenthesized: {}",
            rfs2
        );
        assert!(
            rfs2.contains("prop p2 : state == A || state == B ;"),
            "p2 over-parenthesized: {}",
            rfs2
        );
        // The Not arm keeps its group parenthesized (bare `!a && b` would
        // re-parse as `(!a) && b`).
        assert!(
            rfs2.contains("prop p3 : !(state == A && state == B) ;"),
            "p3 must keep inner parens: {}",
            rfs2
        );
        // json -> rfs direction.
        let json = ir.to_json().expect("serialize").to_string();
        roundtrip_json(&json).expect("json round-trip");
    }

    #[test]
    fn patterns_and_payloads_roundtrip() {
        let src = r#"
module m {
  enum State { A } initial A
  enum Mode { Normal, Fast }
  event Recv(Syn)
  event Recv(Ack(ack_seq: bit<4>))
  event Cmd(Value(op: bit<4>, b: bool))
  output Reply(Ok(code: bit<8>))
  transition A on Recv(Ack(ack_seq)) where ack_seq == 0 { next A; }
  transition A on Cmd(Value(0, true)) { next A; emit Reply(Ok(1)); }
  transition A on Recv(Syn) { next A; }
}
"#;
        roundtrip_rfs(src).expect("pattern module round-trip");
    }

    #[test]
    fn empty_and_multi_module_roundtrip() {
        roundtrip_rfs("").expect("empty source round-trips");
        let src = "module a { enum State { A } initial A }\nmodule b { enum State { X, Y } initial X event E transition X on E { next Y; } }";
        let (ir, text) = roundtrip_rfs(src).expect("multi-module round-trip");
        assert_eq!(ir.modules.len(), 2);
        assert!(text.contains("}\n\nmodule b {"));
    }

    #[test]
    fn json_to_rfs_roundtrip() {
        let ir = parse_source("module m { enum State { A, B } initial A event E transition A on E { next B; } prop p : always (state == A) ; }").expect("parse");
        let json = ir.to_json().expect("serialize").to_string();
        roundtrip_json(&json).expect("json -> rfs -> json round-trip");
    }

    // -- Task 5: liveness operator rendering (D-P4-8) -------------------------

    fn prop_module(formula: &str) -> String {
        format!(
            "module m {{ enum State {{ A }} initial A event E \
             var a: bool init false var b: bool init false var c: bool init false \
             prop p : {} ; }}",
            formula
        )
    }

    #[test]
    fn liveness_formula_roundtrips_both_directions() {
        // Every shape prints in canonical minimal-parenthesis form (the input
        // string IS the canonical output) and round-trips both directions:
        // rfs -> json -> rfs and json -> rfs -> json.
        let cases = [
            "eventually (a)",
            "eventually (a || b)",
            "eventually (always (a))",
            "a until b",
            "a until b until c",
            "!a until b",
            "!!a until b",
            "(a && b) until c",
            "a until (b || c)",
            "a && b until c",
            "eventually (a) && b",
            "!(a until b)",
            "a -> b until c",
        ];
        for c in cases {
            let src = prop_module(c);
            let (ir, text) = roundtrip_rfs(&src)
                .unwrap_or_else(|e| panic!("rfs round-trip failed for '{}': {}", c, e));
            assert!(
                text.contains(c),
                "canonical printing changed '{}' : {}",
                c,
                text
            );
            let json = ir.to_json().expect("serialize").to_string();
            roundtrip_json(&json)
                .unwrap_or_else(|e| panic!("json round-trip failed for '{}': {}", c, e));
        }
    }

    #[test]
    fn pre_liveness_ir_json_still_deserializes() {
        // D-P4-8: additive IR - a JSON document written before
        // eventually/until existed (no new variant kinds) must still
        // deserialize; the liveness variants were additive (Phase 6's
        // IR_VERSION bump is separate and versioned, see src/ir.rs).
        let ir = parse_source(
            "module m { enum State { A, B } initial A event E transition A on E { next B; } prop p : always (state == A -> next (state == B)) ; }",
        )
        .expect("parse");
        assert_eq!(ir.ir_version, crate::ir::IR_VERSION);
        let json = ir.to_json().expect("serialize").to_string();
        // The document contains no liveness variant kinds.
        assert!(!json.contains("\"eventually\"") && !json.contains("\"until\""));
        let value = crate::json_rw::parse(&json).expect("parse pre-liveness IR JSON");
        let back = Ir::from_json(&value).expect("pre-liveness IR JSON must still deserialize");
        assert_eq!(ir, back);
    }

    #[test]
    fn liveness_printing_exact_shapes() {
        // Left-associative until chains stay bare; right-nested until gets
        // parens; boolean groups in until positions get parens.
        let ir = parse_source(&prop_module("a until b until c")).expect("parse");
        assert!(print_ir(&ir).contains("prop p : a until b until c ;"));

        let ir = parse_source(&prop_module("!(a until b)")).expect("parse");
        assert!(print_ir(&ir).contains("prop p : !(a until b) ;"));

        let ir = parse_source(&prop_module("(a && b) until c")).expect("parse");
        assert!(print_ir(&ir).contains("prop p : (a && b) until c ;"));
    }
}
