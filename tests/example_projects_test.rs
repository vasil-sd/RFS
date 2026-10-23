//! Build + run verification for the Phase 5 task 12 example projects:
//! c_sync_handler (gcc -Wall, scenario parity with the Rust sync example),
//! fuzz_rust (the emitted skeleton compiles in its cargo-fuzz layout) and
//! fuzz_c (generation without clang; build gated on clang availability).

use std::path::PathBuf;

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rfs_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs")
}

fn cargo_bin() -> &'static str {
    env!("CARGO")
}

fn tool_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

/// The scenario data lines (byte-identical between the Rust and C handlers;
/// the header line is language-specific and not compared).
fn scenario_data_lines(stdout: &str) -> Vec<&str> {
    stdout
        .lines()
        .filter(|l| {
            l.starts_with("  ticks issued:")
                || l.starts_with("  frames sent")
                || l.starts_with("  acks received")
                || l.starts_with("  out-of-band")
                || l.starts_with("  max window")
        })
        .collect()
}

/// A gcc-style diagnostic references the source file (`handler.c:12:5:`).
/// The rfs codegen step prints its own `warning: property ...` diagnostics,
/// which are expected for spec 43 and must not trip the zero-warning bar.
fn has_gcc_diagnostic(stderr: &str) -> bool {
    stderr.contains("handler.c:") || stderr.contains("arq_fsm.c:")
}

#[test]
fn c_sync_handler_builds_wall_clean_and_matches_rust_scenario() {
    if !tool_on_path("gcc") || !tool_on_path("make") {
        eprintln!("note: gcc/make not available - c_sync_handler check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/c_sync_handler");
    // Rebuild from scratch via the project's own Makefile (RFS_BIN pinned to
    // this build's rfs binary).
    let out = std::process::Command::new("make")
        .current_dir(&dir)
        .arg("clean")
        .output()
        .expect("spawn make clean");
    assert!(out.status.success());
    let out = std::process::Command::new("make")
        .current_dir(&dir)
        .env("RFS_BIN", rfs_bin())
        .output()
        .expect("spawn make");
    assert!(
        out.status.success(),
        "make failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let build_stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        !has_gcc_diagnostic(&build_stderr),
        "gcc -Wall must be clean:\n{build_stderr}"
    );

    // Run the C handler.
    let out = std::process::Command::new(dir.join("c_sync_handler"))
        .output()
        .expect("run c_sync_handler");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let c_out = String::from_utf8_lossy(&out.stdout).to_string();

    // Run the Rust handler (nested cargo is safe: the parent build lock is
    // released while tests run - verified empirically in task 11).
    let out = std::process::Command::new(cargo_bin())
        .args(["run", "-q", "-p", "sync_handler"])
        .output()
        .expect("spawn cargo run sync_handler");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let rust_out = String::from_utf8_lossy(&out.stdout).to_string();

    // The scenario data lines must be byte-identical (C vs Rust parity).
    let c_lines = scenario_data_lines(&c_out);
    let r_lines = scenario_data_lines(&rust_out);
    assert_eq!(
        c_lines, r_lines,
        "C vs Rust scenario drift:\nC: {c_lines:?}\nRust: {r_lines:?}"
    );

    // And pinned to the expected values (guards against both drifting together).
    assert!(c_out.contains("  ticks issued:             19"));
    assert!(c_out.contains("  frames sent (in order):   [0, 1, 2, 3, 4, 5, 6, 7]"));
    assert!(c_out.contains("  acks received (in order): [0, 1, 2, 3, 4, 5, 6, 7]"));
    assert!(c_out.contains("  out-of-band acks dropped: 5"));
    assert!(c_out.contains("  max window observed:      4 (bound 4)"));

    let _ = std::process::Command::new("make")
        .current_dir(&dir)
        .arg("clean")
        .output();
}

#[test]
fn fuzz_rust_skeleton_compiles_in_cargo_fuzz_layout() {
    // Build the root crate first: its build.rs generates the adapted skeleton
    // into fuzz/fuzz_targets/arq.rs.
    let out = std::process::Command::new(cargo_bin())
        .args(["build", "-q", "-p", "fuzz_rust"])
        .output()
        .expect("spawn cargo build fuzz_rust");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    // `cargo check` in the detached fuzz workspace: type-checks the adapted
    // skeleton against the generated module. No fuzzer runtime needed (that
    // is link-time only - building a real fuzzer binary needs clang).
    let fuzz_dir = repo_root().join("crates/examples/fuzz_rust/fuzz");
    let out = std::process::Command::new(cargo_bin())
        .args(["check", "-q"])
        .current_dir(&fuzz_dir)
        .output()
        .expect("spawn cargo check in fuzz workspace");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr).to_string();
        // Registry/network failure -> skip with a note; compile error -> fail.
        if stderr.contains("no matching package")
            || stderr.contains("failed to download")
            || stderr.contains("network failure")
        {
            eprintln!("note: libfuzzer-sys not resolvable (offline?) - fuzz_rust check skipped");
            return;
        }
        panic!("cargo check failed in the fuzz workspace:\n{stderr}");
    }
}

#[test]
fn fuzz_c_generates_without_clang_and_builds_when_available() {
    if !tool_on_path("make") {
        eprintln!("note: make not available - fuzz_c check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/fuzz_c");
    // Generation only (no clang needed): the Makefile's generate target.
    let out = std::process::Command::new("make")
        .current_dir(&dir)
        .env("RFS_BIN", rfs_bin())
        .arg("generate")
        .output()
        .expect("spawn make generate");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );

    let skel =
        std::fs::read_to_string(dir.join("build/fuzz_target.c")).expect("skeleton generated");
    assert!(skel.contains("LLVMFuzzerTestOneInput"));
    assert!(skel.contains("#include \"rfs_generated_fsm.c\""));
    let fsm =
        std::fs::read_to_string(dir.join("build/rfs_generated_fsm.c")).expect("fsm generated");
    // v1.0 unified line-1 header (Phase 6, D-P6-3): identical text to the
    // Rust backend's line 1 - spec 43's whole-file canonical IR JSON hash.
    assert!(fsm.starts_with(
        "// Generated by RFS v1.0.0 | spec sha256:fe676005b8898509 | module sliding_window"
    ));

    // Build gated on clang availability (absent here: the gate + README path).
    if tool_on_path("clang") {
        let out = std::process::Command::new("make")
            .current_dir(&dir)
            .output()
            .expect("spawn make");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    } else {
        eprintln!(
            "note: clang not available - fuzz_c build check skipped (documented in README.md)"
        );
    }

    let _ = std::process::Command::new("make")
        .current_dir(&dir)
        .arg("clean")
        .output();
}
