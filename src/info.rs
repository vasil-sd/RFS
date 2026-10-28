//! `rfs info` - static spec introspection (Phase 5 task 3).
//!
//! Cheap by design: no model checking, no reachability analysis - the command
//! parses + validates the spec and reports what the IR statically contains:
//! per module, states/enums, events/outputs with variants + field types,
//! variables with types + init, transition count, and properties/assumptions
//! each with their monitor-subset status (`monitored` / `skipped: <reason>`,
//! computed by the same R1-R4 check the codegen backends use).
//!
//! Output modes: human-readable text (default) or `--json` (a machine-readable
//! document rendered by the hand-rolled writer from task 2; stable key order,
//! all keys always present with null for absent values - a fixed schema for
//! IDE/tooling consumers).
//! Docs: dev:cli-commands-overview, llm:cli-commands

use crate::ir::{
    BinOp, Expr, Formula, Ir, Module, OnError, OutputExpr, PatternExpr, PatternVariant, Type,
    UnaryOp,
};
use crate::json_rw::Json;
use std::path::Path;

/// Options for the `info` command.
#[derive(Debug, Clone, Copy, Default)]
pub struct InfoOptions {
    /// Emit the machine-readable JSON document instead of text.
    pub json: bool,
}

/// Run `rfs info` on one spec file.
///
/// Returns the rendered output (text or JSON). `Err` carries the diagnostic
/// message for a parse/validation failure (the caller exits 1).
pub fn run_info(path: &Path, opts: &InfoOptions) -> Result<String, String> {
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(fmt) = crate::frontend::Format::from_extension(ext) else {
        return Err(format!(
            "input '{}' must have extension .rfs or .json",
            path.display()
        ));
    };
    let ir = match crate::frontend::convert_file_to_ir(path, fmt) {
        Ok(ir) => ir,
        Err(e) => return Err(format!("{}: {}", path.display(), e)),
    };
    if ir.modules.is_empty() {
        return Err(format!("no modules in {}", path.display()));
    }
    let errors = match crate::validator::validate_ir(&ir) {
        Ok(()) => Vec::new(),
        Err(e) => e,
    };
    if !errors.is_empty() {
        let msgs: Vec<String> = errors.iter().map(|e| e.message.clone()).collect();
        return Err(format!("{}: {}", path.display(), msgs.join("; ")));
    }
    if opts.json {
        Ok(render_json(&ir))
    } else {
        Ok(render_text(&ir))
    }
}

// ---------------------------------------------------------------------------
// Shared renderers (type/expr/formula/pattern -> string)
// ---------------------------------------------------------------------------

pub(crate) fn render_type(ty: &Type) -> String {
    match ty {
        Type::Bool => "bool".to_string(),
        Type::Bit { width } => format!("bit<{width}>"),
        Type::Enum { name } => format!("enum {name}"),
    }
}

fn render_unary_op(op: &UnaryOp) -> &'static str {
    match op {
        UnaryOp::Not => "!",
        UnaryOp::BitNot => "~",
        UnaryOp::Neg => "-",
    }
}

fn render_bin_op(op: &BinOp) -> &'static str {
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

/// Compact expression rendering (parens on compound nodes - readability over
/// precedence fidelity; `info` is a static view, not source regeneration).
pub(crate) fn render_expr(e: &Expr) -> String {
    match e {
        Expr::Int { value } => value.to_string(),
        Expr::Bool { value } => value.to_string(),
        Expr::Path { path } => path.join("."),
        Expr::Unary { op, expr } => format!("{}{}", render_unary_op(op), render_expr(expr)),
        Expr::Binary { op, left, right } => {
            format!(
                "({} {} {})",
                render_expr(left),
                render_bin_op(op),
                render_expr(right)
            )
        }
    }
}

/// Compact pattern rendering: `Go`, `Req(Value(x))`, wildcard `_`.
fn render_pattern_variant(v: &PatternVariant) -> String {
    match v {
        PatternVariant::Unit { name } => name.clone(),
        PatternVariant::Struct { name, fields } => {
            let inner: Vec<String> = fields
                .iter()
                .map(|f| render_pattern_expr(&f.pattern))
                .collect();
            format!("{name}({})", inner.join(", "))
        }
    }
}

fn render_pattern_expr(e: &PatternExpr) -> String {
    match e {
        PatternExpr::Wildcard => "_".to_string(),
        PatternExpr::Bind { name } => name.clone(),
        PatternExpr::Int { value } => value.to_string(),
        PatternExpr::Bool { value } => value.to_string(),
        PatternExpr::EnumVariant { path } => path.join("."),
    }
}

fn render_formula(f: &Formula) -> String {
    match f {
        Formula::InputIs { pattern } => format!(
            "input_is({})",
            render_pattern_variant_group(&pattern.group, &pattern.variant)
        ),
        Formula::OutputHas { pattern } => format!(
            "output_has({})",
            render_pattern_variant_group(&pattern.group, &pattern.variant)
        ),
        Formula::Expr { expr } => render_expr(expr),
        Formula::Not { inner } => format!("!{}", render_formula(inner)),
        Formula::And { left, right } => {
            format!("{} && {}", render_formula(left), render_formula(right))
        }
        Formula::Or { left, right } => {
            format!("{} || {}", render_formula(left), render_formula(right))
        }
        Formula::Implies { left, right } => {
            format!("{} -> {}", render_formula(left), render_formula(right))
        }
        Formula::Next { inner } => format!("next ({})", render_formula(inner)),
        Formula::Always { inner } => format!("always ({})", render_formula(inner)),
        Formula::BoundedAlways { bound, inner } => {
            format!("always[{bound}] ({})", render_formula(inner))
        }
        Formula::BoundedEventually { bound, inner } => {
            format!("bounded_eventually[{bound}] ({})", render_formula(inner))
        }
        Formula::Eventually { inner } => format!("eventually ({})", render_formula(inner)),
        Formula::Until { left, right } => {
            format!("{} until {}", render_formula(left), render_formula(right))
        }
    }
}

/// `Group` or `Group(Variant(...))`.
pub(crate) fn render_pattern_variant_group(
    group: &str,
    variant: &Option<PatternVariant>,
) -> String {
    match variant {
        None => group.to_string(),
        Some(v) => format!("{group}({})", render_pattern_variant(v)),
    }
}

/// Compact output-expression rendering for diagram labels: `Ack`,
/// `Grant(G(node, frame))`. Same compact style as [`render_expr`]; used by
/// `rfs diagram` (task 8) so emitted data reads identically in every format.
pub(crate) fn render_output(o: &OutputExpr) -> String {
    match &o.variant {
        None => o.group.clone(),
        Some(v) if o.args.is_empty() => format!("{}({v})", o.group),
        Some(v) => {
            let args: Vec<String> = o.args.iter().map(render_expr).collect();
            format!("{}({}({}))", o.group, v, args.join(", "))
        }
    }
}

/// Monitor-subset status of a property/assumption formula - the SAME R1-R4
/// check the codegen backends use for their "outside monitor subset; skipped"
/// warnings, so `info` never disagrees with generated code.
fn monitor_status(name: &str, is_assumption: bool, f: &Formula) -> String {
    match crate::monitor::compile_formula(name, is_assumption, f) {
        Ok(_) => "monitored".to_string(),
        Err(reason) => format!("skipped: {reason}"),
    }
}

// ---------------------------------------------------------------------------
// Text rendering
// ---------------------------------------------------------------------------

pub fn render_text(ir: &Ir) -> String {
    let mut out = String::new();
    for (i, m) in ir.modules.iter().enumerate() {
        if i > 0 {
            out.push('\n');
        }
        render_module_text(m, &mut out);
    }
    out
}

fn render_module_text(m: &Module, out: &mut String) {
    out.push_str(&format!("module {}", m.name));
    if let Some(v) = &m.version {
        out.push_str(&format!(" (version {v})"));
    }
    out.push('\n');

    if let Some(s) = &m.initial_state {
        out.push_str(&format!("  initial state: {s}\n"));
    }

    section_enums(m, out);
    section_groups("events", &m.events, out);
    section_groups("outputs", &m.outputs, out);
    section_vars(m, out);
    out.push_str(&format!("  transitions: {}\n", m.transitions.len()));
    if let Some(on) = &m.on_error {
        render_on_error_text(on, out);
    }
    section_props("properties", &m.properties, false, out);
    section_props("assumptions", &m.assumptions, true, out);
}

fn section_enums(m: &Module, out: &mut String) {
    out.push_str("  enums:");
    if m.enums.is_empty() {
        out.push_str(" (none)\n");
        return;
    }
    out.push('\n');
    for e in &m.enums {
        out.push_str(&format!("    {}: {}\n", e.name, e.variants.join(", ")));
    }
}

fn section_groups(label: &str, groups: &[crate::ir::ConstructorGroup], out: &mut String) {
    out.push_str(&format!("  {label}:"));
    if groups.is_empty() {
        out.push_str(" (none)\n");
        return;
    }
    out.push('\n');
    for g in groups {
        let variants: Vec<String> = g
            .variants
            .iter()
            .map(|v| match v {
                crate::ir::ConstructorVariant::Unit { name } => name.clone(),
                crate::ir::ConstructorVariant::Struct { name, fields } => {
                    let inner: Vec<String> = fields
                        .iter()
                        .map(|f| format!("{}: {}", f.name, render_type(&f.ty)))
                        .collect();
                    format!("{name}({})", inner.join(", "))
                }
            })
            .collect();
        out.push_str(&format!("    {}: {}\n", g.name, variants.join(", ")));
    }
}

fn section_vars(m: &Module, out: &mut String) {
    out.push_str("  variables:");
    if m.vars.is_empty() {
        out.push_str(" (none)\n");
        return;
    }
    out.push('\n');
    for v in &m.vars {
        out.push_str(&format!(
            "    {}: {} = {}\n",
            v.name,
            render_type(&v.ty),
            render_expr(&v.init)
        ));
    }
}

fn render_on_error_text(on: &OnError, out: &mut String) {
    let emits = if on.emits.is_empty() {
        String::new()
    } else {
        let list: Vec<String> = on
            .emits
            .iter()
            .map(|e| match &e.variant {
                None => e.group.clone(),
                Some(v) => format!("{}({v})", e.group),
            })
            .collect();
        format!("; emits {}", list.join(", "))
    };
    out.push_str(&format!("  on error: next {}{emits}\n", on.next_state));
}

fn section_props(
    label: &str,
    props: &[crate::ir::Property],
    is_assumption: bool,
    out: &mut String,
) {
    out.push_str(&format!("  {label}:"));
    if props.is_empty() {
        out.push_str(" (none)\n");
        return;
    }
    out.push('\n');
    for p in props {
        let status = monitor_status(&p.name, is_assumption, &p.formula);
        out.push_str(&format!(
            "    {}: {} - {}\n",
            p.name,
            render_formula(&p.formula),
            status
        ));
    }
}

// ---------------------------------------------------------------------------
// JSON rendering (stable schema: all keys always present, null when absent)
// ---------------------------------------------------------------------------

pub fn render_json(ir: &Ir) -> String {
    let modules: Vec<Json> = ir.modules.iter().map(module_json).collect();
    Json::obj(vec![("modules".to_string(), Json::arr(modules))]).to_string_pretty()
}

fn module_json(m: &Module) -> Json {
    let enums: Vec<Json> = m
        .enums
        .iter()
        .map(|e| {
            Json::obj(vec![
                ("name".to_string(), Json::Str(e.name.clone())),
                (
                    "variants".to_string(),
                    Json::arr(e.variants.iter().cloned().map(Json::Str).collect()),
                ),
            ])
        })
        .collect();

    let groups = |gs: &[crate::ir::ConstructorGroup]| -> Json {
        Json::arr(
            gs.iter()
                .map(|g| {
                    let variants: Vec<Json> = g
                        .variants
                        .iter()
                        .map(|v| match v {
                            crate::ir::ConstructorVariant::Unit { name } => Json::obj(vec![
                                ("name".to_string(), Json::Str(name.clone())),
                                ("fields".to_string(), Json::Null),
                            ]),
                            crate::ir::ConstructorVariant::Struct { name, fields } => {
                                let f: Vec<Json> = fields
                                    .iter()
                                    .map(|fd| {
                                        Json::obj(vec![
                                            ("name".to_string(), Json::Str(fd.name.clone())),
                                            ("type".to_string(), Json::Str(render_type(&fd.ty))),
                                        ])
                                    })
                                    .collect();
                                Json::obj(vec![
                                    ("name".to_string(), Json::Str(name.clone())),
                                    ("fields".to_string(), Json::arr(f)),
                                ])
                            }
                        })
                        .collect();
                    Json::obj(vec![
                        ("name".to_string(), Json::Str(g.name.clone())),
                        ("variants".to_string(), Json::arr(variants)),
                    ])
                })
                .collect(),
        )
    };

    let vars: Vec<Json> = m
        .vars
        .iter()
        .map(|v| {
            Json::obj(vec![
                ("name".to_string(), Json::Str(v.name.clone())),
                ("type".to_string(), Json::Str(render_type(&v.ty))),
                ("init".to_string(), Json::Str(render_expr(&v.init))),
            ])
        })
        .collect();

    let props = |ps: &[crate::ir::Property], is_assumption: bool| -> Json {
        Json::arr(
            ps.iter()
                .map(|p| {
                    Json::obj(vec![
                        ("name".to_string(), Json::Str(p.name.clone())),
                        ("formula".to_string(), Json::Str(render_formula(&p.formula))),
                        (
                            "monitor_status".to_string(),
                            Json::Str(monitor_status(&p.name, is_assumption, &p.formula)),
                        ),
                    ])
                })
                .collect(),
        )
    };

    let on_error = match &m.on_error {
        None => Json::Null,
        Some(on) => Json::obj(vec![
            ("next_state".to_string(), Json::Str(on.next_state.clone())),
            (
                "emits".to_string(),
                Json::arr(
                    on.emits
                        .iter()
                        .map(|e| {
                            Json::obj(vec![
                                ("group".to_string(), Json::Str(e.group.clone())),
                                (
                                    "variant".to_string(),
                                    match &e.variant {
                                        None => Json::Null,
                                        Some(v) => Json::Str(v.clone()),
                                    },
                                ),
                            ])
                        })
                        .collect(),
                ),
            ),
        ]),
    };

    Json::obj(vec![
        ("name".to_string(), Json::Str(m.name.clone())),
        (
            "version".to_string(),
            match &m.version {
                None => Json::Null,
                Some(v) => Json::Str(v.clone()),
            },
        ),
        (
            "initial_state".to_string(),
            match &m.initial_state {
                None => Json::Null,
                Some(s) => Json::Str(s.clone()),
            },
        ),
        ("enums".to_string(), Json::arr(enums)),
        ("events".to_string(), groups(&m.events)),
        ("outputs".to_string(), groups(&m.outputs)),
        ("vars".to_string(), Json::arr(vars)),
        (
            "transitions".to_string(),
            Json::int(m.transitions.len() as i64),
        ),
        ("on_error".to_string(), on_error),
        ("properties".to_string(), props(&m.properties, false)),
        ("assumptions".to_string(), props(&m.assumptions, true)),
    ])
}
