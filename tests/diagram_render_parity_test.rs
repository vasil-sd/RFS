//! Cross-renderer syntax parity suite (Phase 7 task 4, D-P7-10): every
//! diagram source rfs generates must parse + render WITHOUT SYNTAX ERRORS in
//! BOTH the in-process renderer and the format's external tool. The
//! comparison is syntax correctness ONLY - never visual output, bytes of the
//! SVG, or diagnostics text (user requirement; D-P7-10).
//!
//! Matrix:
//! - internal half: ALL valid examples x 3 formats (fast, pure Rust) - the
//!   generated text is fed to each in-process renderer directly.
//! - external half: a pinned representative subset (~4 specs exercising
//!   guards / emits / on_error / payloads) x 3 tools via `rfs diagram
//!   --renderer external`, availability-gated and serialized under a static
//!   mutex (mmdc must not run concurrently - TOOLCHAIN.md 6.4). The source
//!   file rfs writes next to the SVG is asserted byte-identical to the
//!   in-process generated text, so BOTH renderers provably parsed the SAME
//!   text.
//!
//! Plus: unresolvable-tools e2e for all three formats in one place (zero
//! external-dependency proof), `--print-script` internal-mode structural pins
//! (heredoc + in-process comment naming the crate, no tool line), and the
//! three library-level negative controls (corrupted text rejected by each
//! in-process renderer with diagnostics).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::Mutex;

fn rfs_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs")
}

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn tmp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rfs_parity_{}_{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

/// All external-tool invocations (and every env mutation in this file)
/// serialize here: mmdc/Chrome must not run concurrently (TOOLCHAIN.md 6.4),
/// same pattern as frontend_test's DIAG_ENV_LOCK.
static TOOL_LOCK: Mutex<()> = Mutex::new(());

fn tool_lock() -> std::sync::MutexGuard<'static, ()> {
    TOOL_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn tool_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

fn set_env(pairs: &[(&str, Option<&str>)]) {
    for (k, v) in pairs {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
}

fn clear_tool_env() {
    set_env(&[
        ("RFS_DOT", None),
        ("RFS_DOT_ARGS", None),
        ("RFS_PLANTUML", None),
        ("RFS_PLANTUML_ARGS", None),
        ("RFS_MERMAID", None),
        ("RFS_MERMAID_ARGS", None),
    ]);
}

/// The pinned external-parity subset (D-P7-10): ~4 specs exercising guards /
/// emits / on_error / payloads.
const PINNED_PARITY_SPECS: &[&str] = &[
    // guards + event payloads (Syn/Ack fields)
    "examples/valid/04_tcp_handshake_like.rfs",
    // compound guard over payload fields + emit + set
    "examples/valid/19_guard_complex.rfs",
    // on_error fallback with emits
    "examples/valid/40_on_error_recovery.rfs",
    // rich payloads (Data(n: bit<4>)) + many outputs
    "examples/valid/48_usb_control_transfer.rfs",
];

/// All valid examples (dynamic set, incl. the tla_adapted subdirectory).
fn valid_examples() -> Vec<PathBuf> {
    let files = rfs::collect_rfs_files(root().join("examples/valid")).expect("collect examples");
    assert!(
        files.len() >= 50,
        "expected the full example set (50), got {}",
        files.len()
    );
    files
}

/// Generate the diagram source text in-process (text mode, no rendering).
fn generate_source(spec: &Path, format: &str) -> String {
    let opts = rfs::diagram::DiagramOptions {
        format: format.to_string(),
        ..Default::default()
    };
    rfs::diagram::run_diagram(spec, &opts)
        .unwrap_or_else(|e| panic!("generate {format} for {}: {e}", spec.display()))
        .stdout
        .expect("text mode emits stdout")
}

/// Feed the generated text to the format's in-process renderer (strict error
/// semantics: Err = diagnostics).
fn internal_render(format: &str, source: &str) -> String {
    let result = match format {
        "dot" => rfs::diagrams::dot::render_svg_internal(source),
        "puml" => rfs::diagrams::plantuml::render_svg_internal(source),
        "mmd" => rfs::diagrams::mermaid::render_svg_internal(source),
        _ => panic!("unknown format {format}"),
    };
    result.unwrap_or_else(|errs| panic!("internal {format} render failed: {}", errs.join("; ")))
}

// ---------------------------------------------------------------------------
// Internal half of the matrix: ALL valid examples x 3 formats.
// ---------------------------------------------------------------------------

fn internal_sweep(format: &str) {
    for f in valid_examples() {
        let source = generate_source(&f, format);
        let svg = internal_render(format, &source);
        assert!(
            svg.contains("<svg"),
            "internal {format} render on {}: no <svg",
            f.display()
        );
    }
}

#[test]
fn parity_internal_dot_all_valid_examples() {
    internal_sweep("dot");
}

#[test]
fn parity_internal_puml_all_valid_examples() {
    internal_sweep("puml");
}

#[test]
fn parity_internal_mmd_all_valid_examples() {
    internal_sweep("mmd");
}

// ---------------------------------------------------------------------------
// External half of the matrix: pinned subset x 3 tools (--renderer external).
// ---------------------------------------------------------------------------

fn external_matrix(format: &str, tool: &str, ext: &str) {
    if !tool_on_path(tool) {
        eprintln!("skipping: {tool} not on PATH (install per TOOLCHAIN.md section 6)");
        return;
    }
    let _g = tool_lock();
    clear_tool_env();
    let tmp = tmp_root(&format!("parity_ext_{ext}"));
    for spec_rel in PINNED_PARITY_SPECS {
        let spec = root().join(spec_rel);
        // The text the INTERNAL renderer parsed (internal half covers these
        // specs too) - the external tool must accept this SAME text.
        let expected_source = generate_source(&spec, format);
        let stem = spec.file_stem().and_then(|s| s.to_str()).unwrap_or("spec");
        let svg = tmp.join(format!("{ext}_{stem}.svg"));
        let out = Command::new(rfs_bin())
            .args([
                "diagram",
                "--format",
                format,
                "--renderer",
                "external",
                "--render",
                "svg",
                "--out",
            ])
            .arg(&svg)
            .arg(&spec)
            .output()
            .expect("run rfs");
        assert!(
            out.status.success(),
            "{tool} must parse the generated {format} source for {spec_rel}; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let src_path = svg.with_extension(ext);
        let written = std::fs::read_to_string(&src_path).expect("source written next to the SVG");
        assert_eq!(
            written, expected_source,
            "external tool must parse the SAME text the internal renderer parsed ({spec_rel})"
        );
        let svg_text = std::fs::read_to_string(&svg).expect("svg exists");
        assert!(
            svg_text.contains("<svg"),
            "{tool} produced no svg for {spec_rel}: {svg_text}"
        );
    }
    cleanup(&tmp);
}

#[test]
fn parity_external_dot_pinned_subset() {
    external_matrix("dot", "dot", "dot");
}

#[test]
fn parity_external_puml_pinned_subset() {
    external_matrix("puml", "plantuml", "puml");
}

#[test]
fn parity_external_mmd_pinned_subset() {
    external_matrix("mmd", "mmdc", "mmd");
}

// ---------------------------------------------------------------------------
// Zero-external-dependency proof: all three formats in one place.
// ---------------------------------------------------------------------------

/// With EVERY tool env var pointed at a nonexistent path, the DEFAULT
/// (internal) render must still succeed for dot, puml AND mmd - no external
/// binary is resolved or invoked on the internal path.
#[test]
fn parity_unresolvable_tools_internal_all_formats_exit_zero() {
    let _g = tool_lock();
    set_env(&[
        ("RFS_DOT", Some("/nonexistent/parity_dot")),
        ("RFS_PLANTUML", Some("/nonexistent/parity_plantuml")),
        ("RFS_MERMAID", Some("/nonexistent/parity_mmdc")),
    ]);
    let tmp = tmp_root("parity_unresolvable");
    // One spec with the full feature set (on_error + emits) for all three.
    let spec = root().join("examples/valid/40_on_error_recovery.rfs");
    for format in ["dot", "puml", "mmd"] {
        let svg = tmp.join(format!("{format}.svg"));
        let out = Command::new(rfs_bin())
            .args(["diagram", "--format", format, "--render", "svg", "--out"])
            .arg(&svg)
            .arg(&spec)
            .output()
            .expect("run rfs");
        assert!(
            out.status.success(),
            "{format}: internal rendering needs no external tool; stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let text = std::fs::read_to_string(&svg).expect("svg exists");
        assert!(text.contains("<svg"), "{format} svg: {text}");
    }
    set_env(&[
        ("RFS_DOT", None),
        ("RFS_PLANTUML", None),
        ("RFS_MERMAID", None),
    ]);
    cleanup(&tmp);
}

// ---------------------------------------------------------------------------
// `--print-script` in internal mode: heredoc + crate comment, no tool line.
// ---------------------------------------------------------------------------

fn print_script_internal_pinned(format: &str, ext: &str, crate_name: &str, tool_line: &str) {
    let tmp = tmp_root(&format!("parity_script_{ext}"));
    // Copy a real corpus spec so the heredoc carries non-trivial content.
    let src_spec = root().join("examples/valid/40_on_error_recovery.rfs");
    let spec = tmp.join("spec.rfs");
    std::fs::copy(&src_spec, &spec).expect("copy spec");
    let out = Command::new(rfs_bin())
        .args([
            "diagram",
            "--format",
            format,
            "--render",
            "svg",
            "--out",
            "build/diagram.svg",
            "--print-script",
        ])
        .arg(&spec)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let script = String::from_utf8_lossy(&out.stdout).into_owned();
    assert!(
        script.contains("<<'__RFS_DIAGRAM_SOURCE__'"),
        "heredoc writes the source: {script}"
    );
    assert!(
        script.contains("render svg (in-process, no external tool)"),
        "in-process comment step: {script}"
    );
    assert!(
        script.contains(&format!("to SVG in-process via {crate_name}")),
        "comment names the in-process crate: {script}"
    );
    assert!(
        !script.contains(tool_line),
        "no external tool invocation may appear: {script}"
    );
    cleanup(&tmp);
}

#[test]
fn parity_print_script_internal_dot_pinned() {
    print_script_internal_pinned("dot", "dot", "layout-rs 0.1.3", "dot -Tsvg");
}

#[test]
fn parity_print_script_internal_puml_pinned() {
    print_script_internal_pinned("puml", "puml", "plantuml-core", "plantuml -tsvg");
}

#[test]
fn parity_print_script_internal_mmd_pinned() {
    print_script_internal_pinned("mmd", "mmd", "mermaid-rs-renderer", "mmdc -i");
}

// ---------------------------------------------------------------------------
// Negative controls (consolidated): corrupted text is rejected by each
// in-process renderer with diagnostics (strict error semantics).
// ---------------------------------------------------------------------------

#[test]
fn parity_negative_control_dot_rejects_garbage() {
    let err = rfs::diagrams::dot::render_svg_internal("this is not dot at all\n")
        .expect_err("parse error must be detected");
    assert!(!err.is_empty(), "garbage must produce diagnostics");
}

#[test]
fn parity_negative_control_puml_rejects_garbage() {
    let err = rfs::diagrams::plantuml::render_svg_internal("this is not plantuml at all\n")
        .expect_err("parse error must be detected");
    assert!(!err.is_empty(), "garbage must produce diagnostics");
}

#[test]
fn parity_negative_control_mmd_rejects_garbage() {
    let err = rfs::diagrams::mermaid::render_svg_internal("this is not mermaid at all\n")
        .expect_err("parse error must be detected");
    assert!(!err.is_empty(), "garbage must produce diagnostics");
}
