//! Mermaid state diagrams (plugin for `rfs diagram --format mmd`).
//!
//! Generation is pure Rust; the emitted text uses ONLY syntax verified against
//! mermaid 12.2.0 (via mmdc) in this environment (PHASE_5_TASK_8.md records
//! the verification):
//!
//! - `stateDiagram-v2`, `[*]` start state, `A --> B: label` transitions.
//! - Mermaid has NO notes-on-links and NO dotted/dashed edges in state
//!   diagrams (both verified as unsupported), so ALL transition information
//!   goes into the multi-line edge label (literal two-char `\n`): event, then
//!   guard (if any), then one `emits X` line per emitted output. `on_error`
//!   fallbacks prefix the first label line with `[on_error]`.
//!
//! SVG rendering invokes the resolved `mmdc` binary (`RFS_MERMAID` /
//! `--mermaid` / `--mermaid-args`). Running as root, mmdc cannot launch
//! Chrome without a puppeteer config carrying `--no-sandbox`; this plugin
//! generates that JSON automatically (args overridable via env var
//! `RFS_MERMAID_PUPPETEER_ARGS`) - a temp file at execution time, an explicit
//! script step in `--print-script` mode.
//! Docs: dev:cli-commands-overview, llm:cli-commands

use std::path::Path;

use crate::diagram::{DiagramPlugin, RenderPlan};
use crate::diagrams::{emit_labels, event_label, guard_label, states};
use crate::ext_tool::{sh_quote, ResolvedTool};
use crate::ir::Module;

/// Default puppeteer browser args (root/container environments).
pub const DEFAULT_PUPPETEER_ARGS: &[&str] = &["--no-sandbox", "--disable-dev-shm-usage"];

/// Env var overriding the generated puppeteer browser args.
pub const PUPPETEER_ARGS_VAR: &str = "RFS_MERMAID_PUPPETEER_ARGS";

/// The effective puppeteer browser args (env override, else defaults).
pub fn puppeteer_args() -> Vec<String> {
    match std::env::var(PUPPETEER_ARGS_VAR) {
        Ok(v) if !v.trim().is_empty() => v.split_whitespace().map(String::from).collect(),
        _ => DEFAULT_PUPPETEER_ARGS
            .iter()
            .map(|s| s.to_string())
            .collect(),
    }
}

/// The puppeteer config JSON (compact, via the hand-rolled writer).
pub fn puppeteer_json() -> String {
    crate::json_rw::Json::obj(vec![(
        "args".to_string(),
        crate::json_rw::Json::arr(
            puppeteer_args()
                .into_iter()
                .map(crate::json_rw::Json::str)
                .collect(),
        ),
    )])
    .to_string_compact()
}

/// One transition (or on_error edge) with the full multi-line label.
fn push_transition(
    out: &mut String,
    from: &str,
    to: &str,
    event: &str,
    guard: Option<&str>,
    emits: &[String],
    on_error: bool,
) {
    let first = if on_error {
        format!("[{event}]")
    } else {
        event.to_string()
    };
    let mut label = first;
    if let Some(g) = guard {
        label.push_str(" \\n ");
        label.push_str(g);
    }
    for e in emits {
        label.push_str(&format!(" \\n emits {e}"));
    }
    out.push_str(&format!("  {from} --> {to}: {label}\n"));
}

pub fn generate(m: &Module) -> String {
    let mut out = String::new();
    out.push_str("stateDiagram-v2\n");
    if let Some(init) = &m.initial_state {
        out.push_str(&format!("  [*] --> {init}\n"));
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
    out
}

/// Relative puppeteer-config path used in `--print-script` mode (next to the
/// SVG output, deterministic).
pub fn script_pup_path(out: &Path) -> std::path::PathBuf {
    let stem = out
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("diagram");
    out.with_file_name(format!("{stem}.puppeteer.json"))
}

fn render_args(src: &Path, out: &Path, pup: &Path) -> Vec<String> {
    vec![
        "-i".to_string(),
        src.to_string_lossy().into_owned(),
        "-o".to_string(),
        out.to_string_lossy().into_owned(),
        "-p".to_string(),
        pup.to_string_lossy().into_owned(),
        "-q".to_string(),
    ]
}

pub fn plan_render(tool: &ResolvedTool, src: &Path, out: &Path) -> RenderPlan {
    let pup = script_pup_path(out);
    RenderPlan {
        prelude: vec![(
            "write mmdc puppeteer config (Chrome needs --no-sandbox when rfs runs as root)"
                .to_string(),
            format!(
                "printf '%s' {} > {}",
                sh_quote(&puppeteer_json()),
                sh_quote(&pup.display().to_string())
            ),
        )],
        cmdline: tool.render(&render_args(src, out, &pup)),
    }
}

pub fn render_svg(tool: &ResolvedTool, src: &Path, out: &Path) -> Result<(), Vec<String>> {
    // Temp puppeteer config (cleaned up afterwards, even on failure).
    let stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("diagram");
    let pup = std::env::temp_dir().join(format!(
        "rfs_mmdc_puppeteer_{}_{}.json",
        std::process::id(),
        stem
    ));
    if let Err(e) = std::fs::write(&pup, puppeteer_json()) {
        return Err(vec![format!(
            "failed to write mmdc puppeteer config {}: {e}",
            pup.display()
        )]);
    }
    let res = crate::diagrams::render_check(tool, &render_args(src, out, &pup));
    let _ = std::fs::remove_file(&pup);
    res
}

/// Pure-Rust SVG renderer (Phase 7 task 1): strict parse + layout of the
/// emitted stateDiagram-v2 subset via mermaid-rs-renderer. `render_strict`
/// returns diagnostics instead of a best-effort image, which doubles as our
/// negative-control material (strict error semantics, D-P7-2).
pub fn render_svg_internal(source: &str) -> Result<String, Vec<String>> {
    let opts = mermaid_rs_renderer::RenderOptions::default();
    match mermaid_rs_renderer::render_strict(source, opts) {
        Ok(svg) => Ok(svg),
        Err(e) => Err(vec![e.to_string()]),
    }
}

/// The registered plugin (one line in `crate::diagram::ALL_DIAGRAMS`).
pub const MMD_PLUGIN: DiagramPlugin = DiagramPlugin {
    name: "mmd",
    description: "Mermaid stateDiagram-v2 (all transition info in multi-line edge labels; SVG rendered in-process by default, mmdc via --renderer external)",
    ext: "mmd",
    generate,
    svg_tool: Some(crate::ext_tool::ExtTool::MERMAID),
    plan_render: Some(plan_render),
    render_svg: Some(render_svg),
    internal_render_svg: Some(render_svg_internal),
    internal_renderer_name: Some("mermaid-rs-renderer 0.3"),
};
