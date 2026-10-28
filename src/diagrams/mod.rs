//! Diagram format plugins for `rfs diagram` (Phase 5 task 8).
//!
//! Each output format is a self-contained module here: one registration entry
//! (e.g. [`dot::DOT_PLUGIN`]) plus the pure-Rust generator and, where the
//! format has an external SVG renderer, its invocation functions. Adding a new
//! diagram format means adding a module and one line to the registry in
//! `crate::diagram` - no other code changes (user requirement 2026-10-21).
//!
//! All generators are deterministic: states in State-enum declaration order,
//! transitions in IR order, on_error edges last. Every format's output for a
//! pinned spec is byte-pinned in `tests/frontend_test.rs`.
//! Docs: dev:cli-commands-overview, llm:cli-commands

pub mod dot;
pub mod mermaid;
pub mod plantuml;
pub mod tables;

use crate::ext_tool::ResolvedTool;
use crate::ir::{EventPattern, Expr, Module, OutputExpr};

/// Event group + variant as a diagram label: `Go`, `Req(Value(x))`.
pub fn event_label(p: &EventPattern) -> String {
    crate::info::render_pattern_variant_group(&p.group, &p.variant)
}

/// Guard expression as a label (None when the transition has no guard).
pub fn guard_label(g: &Option<Expr>) -> Option<String> {
    g.as_ref().map(crate::info::render_expr)
}

/// Emitted outputs as labels: `Grant(G(node, frame))`.
pub fn emit_labels(emits: &[OutputExpr]) -> Vec<String> {
    emits.iter().map(crate::info::render_output).collect()
}

/// All states of a validated module: State-enum variants in declaration order
/// (the validator guarantees the enum exists and is non-empty once transitions
/// are present; an empty module simply yields no states).
pub fn states(m: &Module) -> Vec<&str> {
    m.enums
        .iter()
        .find(|e| e.name == "State")
        .map(|e| e.variants.iter().map(String::as_str).collect())
        .unwrap_or_default()
}

/// Execute a renderer tool with STRICT error semantics (user decision
/// 2026-10-21): any failure - spawn error or non-zero exit - becomes `Err`
/// with diagnostic lines naming the tool and carrying its captured stderr.
pub(crate) fn render_check(tool: &ResolvedTool, args: &[String]) -> Result<(), Vec<String>> {
    match tool.run(args) {
        Err(e) => Err(vec![format!("failed to run {}: {e}", tool.name)]),
        Ok(o) if !o.success() => Err(vec![format!(
            "{} exited with {}{}",
            tool.name,
            o.status,
            if o.stderr.trim().is_empty() {
                String::new()
            } else {
                format!(": {}", o.stderr.trim())
            }
        )]),
        Ok(_) => Ok(()),
    }
}
