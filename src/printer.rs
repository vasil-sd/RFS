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
    let _ = writeln!(s, "module {} {{", module.name);
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
    for p in &module.properties {
        let _ = writeln!(s, "  prop {} : {} ;", p.name, print_formula(&p.formula));
    }
    for a in &module.assumptions {
        let _ = writeln!(s, "  assume {} : {} ;", a.name, print_formula(&a.formula));
    }
    let _ = write!(s, "}}");
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
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Fctx {
    Top,
    ImpliesLeft,
    OrChild,
    ImpliesRight,
    AndChild,
}

/// Whether a formula node must be parenthesized in the given context to
/// re-parse to an equal IR.
///
/// Background: the parser's `primary_formula` falls through to the full
/// expression grammar, so any bare boolean text (identifier/literal/`!` start)
/// is absorbed into a single `Expr` node that swallows following `&&`/`||`
/// operators. Consequently:
/// - `Formula::Or`/`Formula::And` only re-parse from parenthesized text;
/// - `Expr`/`Not` children must be parenthesized where a `||` or `&&`
///   operator follows (Or/And child positions);
/// - `Implies` must be parenthesized where `&&` follows or where another `->`
///   would re-associate it (And-child, Implies-left);
/// - temporal/atom keyword nodes are recognized as formula atoms and are safe
///   bare everywhere.
fn needs_parens(f: &Formula, ctx: Fctx) -> bool {
    match f {
        Formula::Next { .. }
        | Formula::Always { .. }
        | Formula::BoundedEventually { .. }
        | Formula::InputIs { .. }
        | Formula::OutputHas { .. } => false,
        Formula::Expr { .. } | Formula::Not { .. } => {
            matches!(ctx, Fctx::OrChild | Fctx::AndChild)
        }
        // An implication must be parenthesized where a boolean operator
        // follows: its right side would otherwise absorb the following
        // `||`/`&&` into one expression (Or/And child positions), or it could
        // be re-associated with an outer `->` (left of `->`).
        Formula::Implies { .. } => {
            matches!(ctx, Fctx::AndChild | Fctx::ImpliesLeft | Fctx::OrChild)
        }
        Formula::Or { .. } | Formula::And { .. } => true,
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
            // re-parses as `(!a) -> b`.
            let inner_text = match inner.as_ref() {
                Formula::Implies { .. } => format!("({})", print_formula_ctx(inner, Fctx::Top)),
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
        Formula::BoundedEventually { bound, inner } => format!(
            "bounded_eventually[{}] ({})",
            bound,
            print_formula_ctx(inner, Fctx::Top)
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
    let json = serde_json::to_string(&ir1).map_err(|e| format!("rfs -> json failed: {}", e))?;
    let ir2: Ir = serde_json::from_str(&json).map_err(|e| format!("json -> IR failed: {}", e))?;
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
    let ir1: Ir = serde_json::from_str(json).map_err(|e| format!("invalid RFS IR JSON: {}", e))?;
    let rfs1 = print_ir(&ir1);
    let ir2 = crate::parse_source(&rfs1)
        .map_err(|e| format!("re-parse of printed RFS failed: {}", e.message))?;
    if ir1 != ir2 {
        return Err("round-trip json -> rfs -> IR changed the IR".to_string());
    }
    let json2 = serde_json::to_string(&ir2).map_err(|e| format!("IR -> json failed: {}", e))?;
    let ir3: Ir = serde_json::from_str(&json2).map_err(|e| format!("json -> IR failed: {}", e))?;
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
            // Parser quirk: bare `a || b -> c` parses as `(a || b) -> c`
            // because the expression grammar absorbs `||` before `->` is seen.
            "state == A || state != A -> true",
            // Parenthesized left operand produces a formula-level Or node.
            "(state == A) || (state != A) -> true",
            "state == A && state != A -> false",
            "!(state == A)",
            "next (state == A)",
            "always (input_is(E) -> output_has(O))",
            "bounded_eventually[3] (state != A)",
            // Formula-level Or/And nodes (reachable only via explicit parens).
            "(state == A || state == B) -> true",
            "true -> (state == A && state == B)",
            "((state == A) || (state == B))",
            "((state == A) && (state == B))",
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
        let ir = parse_source("module m { enum State { A, B } initial A event E prop p : state == A || state != A -> true ; }").expect("parse");
        let text = print_ir(&ir);
        assert!(text.contains("prop p : state == A || state != A -> true ;"));
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
        let json = serde_json::to_string(&ir).expect("serialize");
        roundtrip_json(&json).expect("json -> rfs -> json round-trip");
    }
}
