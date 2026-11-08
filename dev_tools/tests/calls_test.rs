//! Phase 12 task 5.6: port-equality tests for `calls extract` / `calls
//! classify`.
//!
//! The committed python pair (`dev_tools/analysis/extract_calls.py` +
//! `classify_calls.py`) is the ORACLE: for the same input, the Rust tool's
//! written output must be byte-identical to theirs.
//!
//! Two corpora are checked. The small committed fixture is fast and exercises
//! every kind, both payload forms, and the edge cases. The REAL repo-root
//! `commands.md` is the 12,596-call demonstration corpus.
//!
//! The python oracle is availability-gated: if `python3` is absent the tests
//! skip rather than fail (the same convention as the project's external-tool
//! e2e checks).

use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{Mutex, MutexGuard};

fn manifest_dir() -> &'static Path {
    Path::new(env!("CARGO_MANIFEST_DIR"))
}
fn repo_root() -> PathBuf {
    manifest_dir().parent().unwrap().to_path_buf()
}
fn analysis_dir() -> PathBuf {
    manifest_dir().join("analysis")
}
fn fixture_commands() -> PathBuf {
    manifest_dir().join("tests/fixtures/calls/commands.md")
}

/// Locate `python3` on PATH; None means the oracle is unavailable (skip).
fn python3() -> Option<PathBuf> {
    for dir in std::env::split_paths(&std::env::var_os("PATH")?) {
        let cand = dir.join("python3");
        if cand.is_file() {
            return Some(cand);
        }
    }
    None
}

fn run(cmd: &Path, args: &[&str]) -> (i32, String, String) {
    let out = Command::new(cmd)
        .args(args)
        .output()
        .expect("command must run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn rust_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs-dev")
}

/// A private temp dir for this test process (shared by its tests; each test
/// uses a distinct sub-path so parallel tests cannot collide).
fn temp(tag: &str) -> PathBuf {
    let d = std::env::temp_dir().join(format!("rfs-dev-calls-{tag}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&d);
    std::fs::create_dir_all(&d).unwrap();
    d
}

/// Run BOTH extractors on `commands` and assert the written JSONL is
/// byte-identical. Returns the shared row count.
fn assert_extract_equal(commands: &Path, tag: &str) -> usize {
    let py = python3().unwrap();
    let d = temp(tag);
    let py_out = d.join("py.jsonl");
    let rs_out = d.join("rs.jsonl");
    let script = analysis_dir().join("extract_calls.py");
    let (code, _so, se) = run(
        &py,
        &[
            script.to_str().unwrap(),
            commands.to_str().unwrap(),
            py_out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "python extract failed: {se}");
    let (code, _so, se) = run(
        rust_bin().as_ref(),
        &[
            "calls",
            "extract",
            commands.to_str().unwrap(),
            "--out",
            rs_out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "rust extract failed: {se}");
    let py_bytes = std::fs::read(&py_out).unwrap();
    let rs_bytes = std::fs::read(&rs_out).unwrap();
    assert_eq!(
        py_bytes,
        rs_bytes,
        "extract JSONL differs on {} ({} vs {} bytes)",
        commands.display(),
        py_bytes.len(),
        rs_bytes.len()
    );
    py_bytes.iter().filter(|&&b| b == b'\n').count()
}

/// The python classifier has no --out: it always rewrites the four committed
/// analysis TSVs in place. Two classify tests would therefore race on both the
/// backup files and the live writes, so they serialize on this lock for the
/// whole test. Holding it also makes the fixed-name TsvGuard below safe.
static CLASSIFY_LOCK: Mutex<()> = Mutex::new(());
fn classify_lock() -> MutexGuard<'static, ()> {
    CLASSIFY_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Back up the four analysis TSVs before the python classifier overwrites them
/// and restore them on drop, so the test leaves the tree exactly as it found
/// it. Safe only while `classify_lock()` is held (no concurrent guard).
struct TsvGuard {
    dir: PathBuf,
}
impl TsvGuard {
    fn new() -> Self {
        let dir = analysis_dir();
        for kind in ["terminal", "edit_file", "python", "misc"] {
            let src = dir.join(format!("{kind}_categories.tsv"));
            if src.is_file() {
                std::fs::copy(&src, src.with_file_name(format!("{kind}.bak"))).unwrap();
            }
        }
        TsvGuard { dir }
    }
}
impl Drop for TsvGuard {
    fn drop(&mut self) {
        for kind in ["terminal", "edit_file", "python", "misc"] {
            let bak = self.dir.join(format!("{kind}.bak"));
            let live = self.dir.join(format!("{kind}_categories.tsv"));
            if bak.is_file() {
                std::fs::rename(&bak, &live).unwrap();
            } else if live.exists() {
                let _ = std::fs::remove_file(&live);
            }
        }
    }
}

/// Run BOTH classifiers for `kind` over the same inventory and assert the TSVs
/// are byte-identical.
fn assert_classify_equal(inv: &Path, kind: &str, tag: &str) {
    let py = python3().unwrap();
    let d = temp(&format!("{tag}-{kind}"));
    let rs_out = d.join("rs.tsv");
    // python writes to analysis/<kind>_categories.tsv (no --out); read it back.
    let script = analysis_dir().join("classify_calls.py");
    let (code, _so, se) = run(
        &py,
        &[
            script.to_str().unwrap(),
            "--kind",
            kind,
            inv.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "python classify {kind} failed: {se}");
    let py_bytes = std::fs::read(analysis_dir().join(format!("{kind}_categories.tsv"))).unwrap();
    let (code, _so, se) = run(
        rust_bin().as_ref(),
        &[
            "calls",
            "classify",
            inv.to_str().unwrap(),
            "--kind",
            kind,
            "--out",
            rs_out.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "rust classify {kind} failed: {se}");
    let rs_bytes = std::fs::read(&rs_out).unwrap();
    assert_eq!(
        py_bytes,
        rs_bytes,
        "classify {kind} TSV differs ({} vs {} bytes)",
        py_bytes.len(),
        rs_bytes.len()
    );
}

// ---------------------------------------------------------------------------
// Fixture corpus (fast; every kind + both payload forms + edge cases)
// ---------------------------------------------------------------------------

#[test]
fn extract_port_equality_on_fixture() {
    let Some(_) = python3() else {
        eprintln!("SKIP: python3 not on PATH - the port-equality oracle is unavailable");
        return;
    };
    let rows = assert_extract_equal(&fixture_commands(), "fixture");
    assert!(rows >= 30, "fixture should carry ~31 rows, got {rows}");
}

#[test]
fn classify_port_equality_on_fixture_all_kinds() {
    let Some(_) = python3() else {
        eprintln!("SKIP: python3 not on PATH - the port-equality oracle is unavailable");
        return;
    };
    let _lock = classify_lock();
    let _guard = TsvGuard::new();
    let d = temp("fixture-classify");
    let inv = d.join("calls.jsonl");
    // Build the fixture inventory once with the RUST extractor (already proven
    // byte-equal to python), then classify it with BOTH.
    let (code, _so, se) = run(
        rust_bin().as_ref(),
        &[
            "calls",
            "extract",
            fixture_commands().to_str().unwrap(),
            "--out",
            inv.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "rust extract of fixture failed: {se}");
    for kind in ["terminal", "edit_file", "python", "misc"] {
        assert_classify_equal(&inv, kind, "fixture");
    }
}

// ---------------------------------------------------------------------------
// Real commands.md (the 12,596-call demonstration corpus)
// ---------------------------------------------------------------------------

#[test]
fn extract_port_equality_on_real_commands_md() {
    let Some(_) = python3() else {
        eprintln!("SKIP: python3 not on PATH - the port-equality oracle is unavailable");
        return;
    };
    let commands = repo_root().join("commands.md");
    assert!(
        commands.is_file(),
        "repo-root commands.md must exist for the demonstration corpus"
    );
    let rows = assert_extract_equal(&commands, "real");
    // The Phase 12 baseline is 12,596 recorded calls; a different count means
    // commands.md changed since the analysis - still a valid equality check,
    // but flag it so the demonstration numbers stay honest.
    assert_eq!(
        rows, 12596,
        "commands.md no longer carries the 12,596-call Phase 12 baseline"
    );
}

#[test]
fn classify_port_equality_on_real_terminal() {
    let Some(_) = python3() else {
        eprintln!("SKIP: python3 not on PATH - the port-equality oracle is unavailable");
        return;
    };
    let _lock = classify_lock();
    let _guard = TsvGuard::new();
    let d = temp("real-classify");
    let inv = d.join("calls.jsonl");
    let (code, _so, se) = run(
        rust_bin().as_ref(),
        &[
            "calls",
            "extract",
            repo_root().join("commands.md").to_str().unwrap(),
            "--out",
            inv.to_str().unwrap(),
        ],
    );
    assert_eq!(code, 0, "rust extract of real commands.md failed: {se}");
    // terminal is the largest + most rule-dense kind - the demonstration target.
    assert_classify_equal(&inv, "terminal", "real");
}
