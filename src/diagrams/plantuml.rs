//! PlantUML state diagrams (plugin for `rfs diagram --format puml`).
//!
//! Generation is pure Rust; the emitted text uses ONLY syntax verified against
//! plantuml 1.2020.02 in this environment (PHASE_5_TASK_8.md records the
//! verification):
//!
//! - `@startuml` / `@enduml`, `[*]` start state, `A --> B : label` transitions.
//! - Edge label = `event \n guard` (literal two-char `\n`; guard line omitted
//!   when the transition has no guard) - the user-specified format.
//! - Emitted data goes in a NOTE ON LINK directly after the transition:
//!   one line per output, `note on link : emits X`. The position word is
//!   omitted because plantuml-core's parser rejects `note right on link`
//!   (Phase 7 task 2); real PlantUML accepts both forms.
//! - `on_error` fallback: dashed edge `A -[dashed]-> B : on_error` per state
//!   (the bracketed form is valid; `-dashed->` without brackets is NOT).
//!
//! SVG rendering is IN-PROCESS by default (Phase 7 task 2): the emitted text
//! is parsed + laid out + rendered by plantuml-core (pure Rust; git dependency,
//! rev-pinned in Cargo.toml). `--renderer external` invokes the resolved
//! `plantuml` binary instead (`RFS_PLANTUML` / `--plantuml` /
//! `--plantuml-args`). plantuml derives its output name from the input file,
//! so the source is written next to `--out *.svg` with the same stem
//! (orchestrated by `crate::diagram`).
//! Docs: dev:cli-commands-overview, llm:cli-commands

use std::path::Path;

use crate::diagram::{DiagramPlugin, RenderPlan};
use crate::diagrams::{emit_labels, event_label, guard_label, states};
use crate::ext_tool::{ExtTool, ResolvedTool};
use crate::ir::Module;

/// One transition (or on_error edge) plus its emitted-data note.
fn push_transition(
    out: &mut String,
    from: &str,
    to: &str,
    event: &str,
    guard: Option<&str>,
    emits: &[String],
    dashed: bool,
) {
    let arrow = if dashed { "-[dashed]->" } else { "-->" };
    let mut label = event.to_string();
    if let Some(g) = guard {
        label.push_str(" \\n ");
        label.push_str(g);
    }
    out.push_str(&format!("{from} {arrow} {to} : {label}\n"));
    for e in emits {
        out.push_str(&format!("note on link : emits {e}\n"));
    }
}

pub fn generate(m: &Module) -> String {
    let mut out = String::new();
    out.push_str("@startuml\n");
    out.push_str(&format!("title {}\n", m.name));
    if let Some(init) = &m.initial_state {
        out.push_str(&format!("[*] --> {init}\n"));
    }
    for t in &m.transitions {
        push_transition(
            &mut out,
            &t.state,
            &t.next_state,
            &event_label(&t.pattern),
            guard_label(&t.guard).as_deref(),
            &emit_labels(&t.emits),
            false,
        );
    }
    if let Some(on) = &m.on_error {
        for s in states(m) {
            push_transition(
                &mut out,
                s,
                &on.next_state,
                "on_error",
                None,
                &emit_labels(&on.emits),
                true,
            );
        }
    }
    out.push_str("@enduml\n");
    out
}

fn render_args(src: &Path) -> Vec<String> {
    // plantuml writes <input-stem>.svg next to the input; the orchestrator
    // guarantees that is exactly the requested --out path.
    vec!["-tsvg".to_string(), src.to_string_lossy().into_owned()]
}

pub fn plan_render(tool: &ResolvedTool, src: &Path, _out: &Path) -> RenderPlan {
    RenderPlan {
        prelude: Vec::new(),
        cmdline: tool.render(&render_args(src)),
    }
}

pub fn render_svg(tool: &ResolvedTool, src: &Path, _out: &Path) -> Result<(), Vec<String>> {
    // The exit status is the strict contract (user decision 2026-10-21);
    // plantuml derives its output name from the input, and the orchestrator
    // guarantees that is exactly `--out`.
    crate::diagrams::render_check(tool, &render_args(src))
}

/// Pure-Rust SVG renderer (Phase 7 task 2): parses + lays out + renders the
/// emitted PlantUML subset in-process via plantuml-core. Strict error
/// semantics: Err carries the pipeline's diagnostics (D-P7-2).
pub fn render_svg_internal(source: &str) -> Result<String, Vec<String>> {
    let opts = plantuml_core::RenderOptions::default();
    match plantuml_core::render(source, &opts) {
        Ok(svg) => Ok(svg),
        Err(e) => Err(vec![e.to_string()]),
    }
}

/// The registered plugin (one line in `crate::diagram::ALL_DIAGRAMS`).
pub const PUML_PLUGIN: DiagramPlugin = DiagramPlugin {
    name: "puml",
    description: "PlantUML state diagram (edge label 'event \\n guard', emitted data via note on link; SVG rendered in-process by default, plantuml via --renderer external)",
    ext: "puml",
    generate,
    svg_tool: Some(ExtTool::PLANTUML),
    plan_render: Some(plan_render),
    render_svg: Some(render_svg),
    internal_render_svg: Some(render_svg_internal),
    internal_renderer_name: Some("plantuml-core 0.2 (git rev 8066f61)"),
};
