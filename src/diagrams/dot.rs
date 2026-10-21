//! Graphviz DOT state diagrams (plugin for `rfs diagram --format dot`).
//!
//! Generation is pure Rust (no system dependency). SVG rendering invokes the
//! resolved `dot` binary (`RFS_DOT` / `--dot` / `--dot-args`, task 3 resolver).
//!
//! Layout: one node per State-enum variant (declaration order; the initial
//! state is marked filled), one edge per transition in IR order labeled
//! `Event [guard] -> next`, and - when the module declares `on_error` - one
//! dashed edge per state to the fallback target (it fires from ANY state).

use std::path::Path;

use crate::diagram::{DiagramPlugin, RenderPlan};
use crate::diagrams::{event_label, guard_label, states};
use crate::ext_tool::{ExtTool, ResolvedTool};
use crate::ir::Module;

/// DOT string escaping for labels: quote + escape backslash and double quote.
fn dot_str(s: &str) -> String {
    format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\""))
}

/// One transition's edge label: `Event [guard] -> next` (guard part omitted
/// when the transition has no guard).
pub fn transition_label(t: &crate::ir::Transition) -> String {
    let mut l = event_label(&t.pattern);
    if let Some(g) = guard_label(&t.guard) {
        l.push_str(" [");
        l.push_str(&g);
        l.push(']');
    }
    l.push_str(" -> ");
    l.push_str(&t.next_state);
    l
}

pub fn generate(m: &Module) -> String {
    let mut out = String::new();
    out.push_str(&format!("digraph {} {{\n", m.name));
    out.push_str("  rankdir=LR;\n");
    out.push_str("  node [shape=box];\n");
    for s in states(m) {
        if Some(s) == m.initial_state.as_deref() {
            out.push_str(&format!(
                "  {} [style=filled, fillcolor=\"#e0e0e0\"];\n",
                dot_str(s)
            ));
        } else {
            out.push_str(&format!("  {};\n", dot_str(s)));
        }
    }
    for t in &m.transitions {
        out.push_str(&format!(
            "  {} -> {} [label={}];\n",
            dot_str(&t.state),
            dot_str(&t.next_state),
            dot_str(&transition_label(t))
        ));
    }
    if let Some(on) = &m.on_error {
        for s in states(m) {
            out.push_str(&format!(
                "  {} -> {} [label={}, style=dashed];\n",
                dot_str(s),
                dot_str(&on.next_state),
                dot_str(&format!("on_error -> {}", on.next_state))
            ));
        }
    }
    out.push('}');
    out.push('\n');
    out
}

fn render_args(src: &Path, out: &Path) -> Vec<String> {
    vec![
        "-Tsvg".to_string(),
        src.to_string_lossy().into_owned(),
        "-o".to_string(),
        out.to_string_lossy().into_owned(),
    ]
}

pub fn plan_render(tool: &ResolvedTool, src: &Path, out: &Path) -> RenderPlan {
    RenderPlan {
        prelude: Vec::new(),
        cmdline: tool.render(&render_args(src, out)),
    }
}

pub fn render_svg(tool: &ResolvedTool, src: &Path, out: &Path) -> Result<(), Vec<String>> {
    crate::diagrams::render_check(tool, &render_args(src, out))
}

/// The registered plugin (one line in `crate::diagram::ALL_DIAGRAMS`).
pub const DOT_PLUGIN: DiagramPlugin = DiagramPlugin {
    name: "dot",
    description: "Graphviz DOT state diagram (pure Rust generation; SVG via the dot binary)",
    ext: "dot",
    generate,
    svg_tool: Some(ExtTool::DOT),
    plan_render: Some(plan_render),
    render_svg: Some(render_svg),
};
