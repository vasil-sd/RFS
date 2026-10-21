//! `rfs diagram` - plugin-based state-diagram and table generation (Phase 5
//! task 8).
//!
//! Mirrors the codegen backend pattern (`crate::codegen`): every output format
//! is a registered [`DiagramPlugin`] - static data + function pointers, one
//! module per format under `crate::diagrams`. Adding a new diagram format means
//! adding one module and one line to `ALL_DIAGRAMS`; no other code changes
//! (user requirement 2026-10-21).
//!
//! PRIMARY behaviour is pure-Rust text generation with NO external tool calls.
//! `--render svg` invokes the format's renderer (`dot` / `plantuml` / `mmdc`)
//! resolved through `crate::ext_tool`. Strict error semantics (user decision
//! 2026-10-21): a missing/unresolvable tool or a non-zero renderer exit is an
//! ERROR (exit 1) - no graceful degradation, because RFS runs in CI/CD
//! pipelines where silent fallbacks hide broken environments. Users who want a
//! fallback compose it themselves (try render, on failure generate text only,
//! or drive the `--print-script` output from their own wrapper).

use std::path::{Path, PathBuf};

use crate::diagrams;
use crate::ext_tool::{sh_quote, ExtTool, ResolvedTool, Script};
use crate::ir::{Ir, Module};

/// Pure render-plan construction for `--print-script` (argv + prelude steps,
/// no execution).
pub type PlanRenderFn = fn(&ResolvedTool, &Path, &Path) -> RenderPlan;

/// Execute the SVG renderer (strict error semantics; Err = diagnostic lines).
pub type RenderSvgFn = fn(&ResolvedTool, &Path, &Path) -> Result<(), Vec<String>>;

/// A registered diagram output format (plugin).
pub struct DiagramPlugin {
    /// CLI name: `dot`, `puml`, `mmd`, `table`, `md`.
    pub name: &'static str,
    /// One-line description shown by `--list`.
    pub description: &'static str,
    /// Source-file extension (without dot).
    pub ext: &'static str,
    /// Pure-Rust text generation (deterministic; no I/O, no tool calls).
    pub generate: fn(&Module) -> String,
    /// External tool used by `--render svg` (None = format has no renderer).
    pub svg_tool: Option<ExtTool>,
    /// Pure render-plan construction for `--print-script`.
    pub plan_render: Option<PlanRenderFn>,
    /// Execute the renderer (strict error semantics).
    pub render_svg: Option<RenderSvgFn>,
}

/// What `--print-script` needs from a renderer: prelude file-write steps plus
/// the final tool command line.
pub struct RenderPlan {
    /// (comment, rendered command line) pairs executed before the render call.
    pub prelude: Vec<(String, String)>,
    /// The rendered tool command line.
    pub cmdline: String,
}

/// The registry of all diagram formats (the plugin table).
const ALL_DIAGRAMS: &[&DiagramPlugin] = &[
    &diagrams::dot::DOT_PLUGIN,
    &diagrams::plantuml::PUML_PLUGIN,
    &diagrams::mermaid::MMD_PLUGIN,
    &diagrams::tables::TABLE_PLUGIN,
    &diagrams::tables::MD_PLUGIN,
];

pub static DIAGRAMS: &[&DiagramPlugin] = ALL_DIAGRAMS;

/// All registered formats as `(name, description)` pairs.
pub fn list_diagrams() -> Vec<(&'static str, &'static str)> {
    DIAGRAMS.iter().map(|d| (d.name, d.description)).collect()
}

/// Look up a format by CLI name.
pub fn get_diagram(name: &str) -> Option<&'static DiagramPlugin> {
    DIAGRAMS.iter().find(|d| d.name == name).copied()
}

/// Per-tool CLI overrides for `--render` (tool path + extra args).
#[derive(Debug, Clone, Default)]
pub struct ToolOverride {
    pub path: Option<String>,
    pub args: Option<String>,
}

/// Options for the `diagram` command.
#[derive(Debug, Clone)]
pub struct DiagramOptions {
    /// Plugin name (`dot|puml|mmd|table|md`); default `dot`.
    pub format: String,
    /// Output file (text mode). None = stdout.
    pub out: Option<PathBuf>,
    /// Render SVG via the format's external tool.
    pub render_svg: bool,
    /// Print a shell script instead of executing (task 3 semantics).
    pub print_script: bool,
    pub dot: ToolOverride,
    pub plantuml: ToolOverride,
    pub mermaid: ToolOverride,
}

impl Default for DiagramOptions {
    fn default() -> Self {
        Self {
            format: "dot".to_string(),
            out: None,
            render_svg: false,
            print_script: false,
            dot: ToolOverride::default(),
            plantuml: ToolOverride::default(),
            mermaid: ToolOverride::default(),
        }
    }
}

/// Result of a successful `rfs diagram` run.
pub struct DiagramResult {
    /// Text for stdout (None when written to a file).
    pub stdout: Option<String>,
    /// One-line note for debug logging.
    pub note: String,
}

fn tool_override(plugin: &DiagramPlugin, opts: &DiagramOptions) -> ToolOverride {
    match plugin.name {
        "dot" => opts.dot.clone(),
        "puml" => opts.plantuml.clone(),
        "mmd" => opts.mermaid.clone(),
        _ => ToolOverride::default(),
    }
}

/// Run `rfs diagram` on one spec file.
///
/// Operates on the FIRST module of the spec (codegen convention). `Err`
/// carries the diagnostic message (the caller exits 1).
pub fn run_diagram(path: &Path, opts: &DiagramOptions) -> Result<DiagramResult, String> {
    // Load + validate (same contract as `rfs info`).
    let ext = path.extension().and_then(|e| e.to_str()).unwrap_or("");
    let Some(fmt) = crate::frontend::Format::from_extension(ext) else {
        return Err(format!(
            "input '{}' must have extension .rfs or .json",
            path.display()
        ));
    };
    let ir: Ir = match crate::frontend::convert_file_to_ir(path, fmt) {
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
    let module = &ir.modules[0];

    // Resolve the format plugin.
    let plugin = get_diagram(&opts.format).ok_or_else(|| {
        format!(
            "unknown diagram format '{}' (registered: {})",
            opts.format,
            list_diagrams()
                .iter()
                .map(|(n, _)| *n)
                .collect::<Vec<_>>()
                .join(", ")
        )
    })?;

    // Pure generation - no tool calls.
    let source = (plugin.generate)(module);

    // Validate the `--render svg` preconditions up front: they apply to BOTH
    // execution and `--print-script` modes.
    let render_ctx = if opts.render_svg {
        let preset = plugin.svg_tool.ok_or_else(|| {
            format!(
                "diagram format '{}' has no SVG renderer (use dot, puml or mmd)",
                plugin.name
            )
        })?;
        let out = opts
            .out
            .clone()
            .ok_or("--render svg requires --out FILE ending in .svg")?;
        if out.extension().and_then(|e| e.to_str()) != Some("svg") {
            return Err(format!(
                "--render svg requires --out FILE ending in .svg (got '{}')",
                out.display()
            ));
        }
        Some((preset, out))
    } else {
        None
    };

    if opts.print_script {
        let ctx = render_ctx.as_ref().map(|(p, o)| (p, o));
        return Ok(DiagramResult {
            stdout: Some(render_script(path, plugin, &source, ctx, opts)),
            note: format!("printed script for --format {}", plugin.name),
        });
    }

    if let Some((preset, out)) = render_ctx {
        // Source written next to the SVG with the same stem: plantuml derives
        // its output name from the input, so this keeps every format's render
        // target exactly at `out`.
        let src_path = out.with_extension(plugin.ext);
        std::fs::write(&src_path, &source)
            .map_err(|e| format!("failed to write {}: {e}", src_path.display()))?;
        let ovr = tool_override(plugin, opts);
        let resolved = preset.resolve(ovr.path.as_deref(), ovr.args.as_deref());
        plugin
            .render_svg
            .expect("svg_tool set implies render_svg set")(&resolved, &src_path, &out)
        .map_err(|errs| errs.join("; "))?;
        return Ok(DiagramResult {
            stdout: None,
            note: format!(
                "wrote {} and rendered {}",
                src_path.display(),
                out.display()
            ),
        });
    }

    match &opts.out {
        Some(f) => {
            std::fs::write(f, &source)
                .map_err(|e| format!("failed to write {}: {e}", f.display()))?;
            Ok(DiagramResult {
                stdout: None,
                note: format!("wrote {}", f.display()),
            })
        }
        None => {
            let note = format!(
                "emitted {} bytes of '{}' to stdout",
                source.len(),
                plugin.name
            );
            Ok(DiagramResult {
                stdout: Some(source),
                note,
            })
        }
    }
}

/// Build the `--print-script` output: a self-contained POSIX sh script that
/// performs exactly what the command would have done (task 3 Script builder).
/// `render_ctx` is the validated `(preset, out svg path)` pair when
/// `--render svg` was given (None otherwise).
fn render_script<'a>(
    spec: &'a Path,
    plugin: &'a DiagramPlugin,
    source: &'a str,
    render_ctx: Option<(&'a ExtTool, &'a PathBuf)>,
    opts: &'a DiagramOptions,
) -> String {
    let mut desc = format!("rfs diagram --format {}", plugin.name);
    if opts.render_svg {
        desc.push_str(" --render svg");
        if let Some(out) = &opts.out {
            desc.push_str(&format!(" --out {}", out.display()));
        }
    } else if let Some(out) = &opts.out {
        desc.push_str(&format!(" --out {}", out.display()));
    }
    desc.push(' ');
    desc.push_str(&spec.display().to_string());

    let mut script = Script::new(&desc, &spec.display().to_string());

    if let Some((preset, out)) = render_ctx {
        let ovr = tool_override(plugin, opts);
        let resolved = preset.resolve(ovr.path.as_deref(), ovr.args.as_deref());
        script = script.with_tool(resolved.clone());

        let src_path = out.with_extension(plugin.ext);
        // Heredoc with a quoted delimiter: no shell expansion of the content.
        let src_text = if source.ends_with('\n') {
            source.to_string()
        } else {
            format!("{source}\n")
        };
        let heredoc = format!(
            "cat > {} <<'__RFS_DIAGRAM_SOURCE__'\n{}__RFS_DIAGRAM_SOURCE__",
            sh_quote(&src_path.display().to_string()),
            src_text
        );
        script = script.step("write generated diagram source", &heredoc);

        let plan = plugin
            .plan_render
            .expect("svg_tool set implies plan_render set")(
            &resolved, &src_path, out
        );
        for (comment, cmdline) in plan.prelude {
            script = script.step(&comment, &cmdline);
        }
        script = script.step("render svg", &plan.cmdline);
    } else {
        script = script.step(
            "no external tools needed (text format)",
            &format!(
                "echo 'rfs diagram --format {} <spec> emits text; nothing to run'",
                plugin.name
            ),
        );
    }

    script.render()
}
