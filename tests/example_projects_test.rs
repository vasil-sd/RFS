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

mod common;

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
    if !common::toolchain::available(rfs::cli::ext_tool::ExtTool::GCC)
        || !common::toolchain::available_env_or_path("RFS_MAKE", "make")
    {
        eprintln!("note: gcc/make not available - c_sync_handler check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/c_sync_handler");
    // Rebuild from scratch via the project's own Makefile (RFS_BIN pinned to
    // this build's rfs binary).
    let out = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .arg("clean")
        .output()
        .expect("spawn make clean");
    assert!(out.status.success());
    let out = std::process::Command::new(common::toolchain::make())
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

    let _ = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .arg("clean")
        .output();
}

// Phase 17 task 9: the documented gcc/clang choice. The generated C is
// compiler-agnostic - this leg rebuilds c_sync_handler under CLANG and pins
// the SAME warning-free build + scenario outcome as the gcc leg above.
#[test]
fn c_sync_handler_builds_under_clang_with_same_outcome() {
    if !common::toolchain::available(rfs::cli::ext_tool::ExtTool::CLANG)
        || !common::toolchain::available_env_or_path("RFS_MAKE", "make")
    {
        eprintln!("note: clang/make not available - c_sync_handler clang check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/c_sync_handler");
    // Rebuild from scratch under clang (CC=clang is the documented choice).
    let out = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .arg("clean")
        .output()
        .expect("spawn make clean");
    assert!(out.status.success());
    let out = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .env("RFS_BIN", rfs_bin())
        .env("CC", "clang")
        .output()
        .expect("spawn make CC=clang");
    assert!(
        out.status.success(),
        "make CC=clang failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // The zero-warning bar holds under clang too (a compiler diagnostic
    // references handler.c / arq_fsm.c; rfs's own codegen warnings do not).
    let build_stderr = String::from_utf8_lossy(&out.stderr).to_string();
    assert!(
        !has_gcc_diagnostic(&build_stderr),
        "clang -Wall must be clean:\n{build_stderr}"
    );

    // Run the clang-built handler: the SAME pinned scenario outcome as gcc.
    let out = std::process::Command::new(dir.join("c_sync_handler"))
        .output()
        .expect("run c_sync_handler (clang build)");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let clang_out = String::from_utf8_lossy(&out.stdout).to_string();
    assert!(clang_out.contains("  ticks issued:             19"));
    assert!(clang_out.contains("  frames sent (in order):   [0, 1, 2, 3, 4, 5, 6, 7]"));
    assert!(clang_out.contains("  acks received (in order): [0, 1, 2, 3, 4, 5, 6, 7]"));
    assert!(clang_out.contains("  out-of-band acks dropped: 5"));
    assert!(clang_out.contains("  max window observed:      4 (bound 4)"));

    let _ = std::process::Command::new(common::toolchain::make())
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
    if !common::toolchain::available_env_or_path("RFS_MAKE", "make") {
        eprintln!("note: make not available - fuzz_c check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/fuzz_c");
    // Generation only (no clang needed): the Makefile's generate target.
    let out = std::process::Command::new(common::toolchain::make())
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
    // Phase 14 task 4: the skeleton references the fsm through its header (the
    // Makefile stamps --fsm-file-header build/rfs_generated_fsm.h), not by
    // source-including the .c.
    assert!(skel.contains("#include \"build/rfs_generated_fsm.h\""));
    let fsm =
        std::fs::read_to_string(dir.join("build/rfs_generated_fsm.c")).expect("fsm generated");
    // v1.0 unified line-1 header (Phase 6, D-P6-3): identical text to the
    // Rust backend's line 1 - spec 43's whole-file canonical IR JSON hash.
    assert!(fsm.starts_with(
        "// Generated by RFS v1.0.0 | spec sha256:fe676005b8898509 | module sliding_window"
    ));

    // Build gated on clang availability (absent here: the gate + README path).
    if common::toolchain::available(rfs::cli::ext_tool::ExtTool::CLANG) {
        let out = std::process::Command::new(common::toolchain::make())
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

    let _ = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .arg("clean")
        .output();
}

// ---------------------------------------------------------------------------
// Phase 17 task 6: the three NEGATIVE example projects. Each runs the REAL rfs
// binary on the example's spec(s) and pins the failure output (the diagnostic
// IS the deliverable), then proves the fixed variant passes. No external tools
// are needed - these are pure CLI invocations of rfs_bin().
// ---------------------------------------------------------------------------

fn run_rfs(args: &[&str]) -> (i32, String) {
    let out = std::process::Command::new(rfs_bin())
        .args(args)
        .output()
        .expect("spawn rfs");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

#[test]
fn mc_deadlock_fail_reports_deadlock_and_fix_passes() {
    let dir = repo_root().join("crates/examples/mc_deadlock_fail");
    // The broken spec: terminal Established deadlocks - exit 1 + the pinned CE.
    let (code, out) = run_rfs(&[
        "mc-ref",
        dir.join("04_tcp_handshake_like.rfs").to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "deadlock spec must fail mc-ref:\n{out}");
    assert!(
        out.contains("status=fail") && out.contains("reason=deadlock"),
        "pinned summary line missing:\n{out}"
    );
    assert!(
        out.contains("to=State::Established"),
        "the CE must end in the terminal state:\n{out}"
    );
    // The fixed spec: Tick teardown out of Established - exit 0, PASS complete.
    let (code, out) = run_rfs(&["mc-ref", dir.join("fixed.rfs").to_str().unwrap()]);
    assert_eq!(code, 0, "fixed spec must pass mc-ref:\n{out}");
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
}

#[test]
fn mc_property_violation_fail_reports_rst_and_fix_passes() {
    let dir = repo_root().join("crates/examples/mc_property_violation_fail");
    // The mutated variant: rst_goes_closed violated at step 2 - exit 1 + pinned CE.
    let (code, out) = run_rfs(&[
        "mc-ref",
        dir.join("47_http2_stream_rst_fail.rfs").to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "mutated variant must fail mc-ref:\n{out}");
    assert!(
        out.contains("reason=property-violation label=rst_goes_closed step=2"),
        "pinned violation line missing:\n{out}"
    );
    assert!(
        out.contains("to=State::HalfClosedLocal"),
        "the CE must show the retargeted arm:\n{out}"
    );
    // The fixed variant (= the original spec 47): exit 0, PASS complete.
    let (code, out) = run_rfs(&["mc-ref", dir.join("fixed.rfs").to_str().unwrap()]);
    assert_eq!(code, 0, "fixed variant must pass mc-ref:\n{out}");
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
}

#[test]
fn invalid_spec_diagnosis_reports_duplicate_priority_and_fix_validates() {
    let dir = repo_root().join("crates/examples/invalid_spec_diagnosis");
    // Text mode: exit 1 + the pinned semantic diagnostic.
    let (code, out) = run_rfs(&["validate", dir.join("broken.rfs").to_str().unwrap()]);
    assert_eq!(code, 1, "broken spec must fail validation:\n{out}");
    assert!(
        out.contains("duplicate priority 5 for the same event pattern (transitions #0 and #1)"),
        "pinned diagnostic missing:\n{out}"
    );
    // JSON mode: exit 1 + ok=false + kind=semantic + line/col 0/0 (module-level).
    let (code, out) = run_rfs(&[
        "validate",
        "--json",
        dir.join("broken.rfs").to_str().unwrap(),
    ]);
    assert_eq!(code, 1, "broken spec must fail --json validation:\n{out}");
    assert!(
        out.contains("\"ok\": false"),
        "pinned ok=false missing:\n{out}"
    );
    assert!(
        out.contains("\"kind\": \"semantic\""),
        "pinned kind missing:\n{out}"
    );
    // The fixed spec: exit 0, valid.
    let (code, out) = run_rfs(&["validate", dir.join("fixed.rfs").to_str().unwrap()]);
    assert_eq!(code, 0, "fixed spec must validate:\n{out}");
    assert!(out.contains("valid"), "pinned valid line missing:\n{out}");
}

// ---------------------------------------------------------------------------
// Phase 17 task 7: the three new WORKSPACE-MEMBER example crates. Each leg
// builds the member and runs its test suite via nested cargo (safe: the
// parent build lock is released while tests run - verified empirically in
// Phase 5 task 11). monitors_production additionally runs its RELEASE suite:
// there the monitor code is compiled out, so the violating sequence must step
// all Ok (the zero-cost claim, pinned mechanically).
// ---------------------------------------------------------------------------

fn cargo_member(args: &[&str]) -> String {
    let out = std::process::Command::new(cargo_bin())
        .args(args)
        .output()
        .expect("spawn nested cargo");
    assert!(
        out.status.success(),
        "cargo {} failed: {}",
        args.join(" "),
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).to_string()
}

// Nested `cargo test` runs WITHOUT -q: quiet mode replaces the per-test
// lines with dots, and the legs pin those lines.
#[test]
fn no_std_fsm_builds_and_tests_the_nostd_artifact() {
    cargo_member(&["build", "-q", "-p", "no_std_fsm"]);
    let out = cargo_member(&["test", "-p", "no_std_fsm"]);
    assert!(
        out.contains("full_session_through_nostd_artifact ... ok"),
        "pinned no_std session test missing:\n{out}"
    );
}

#[test]
fn monitors_production_tests_debug_violation_and_release_quiet() {
    cargo_member(&["build", "-q", "-p", "monitors_production"]);
    let debug = cargo_member(&["test", "-p", "monitors_production"]);
    assert!(
        debug.contains("violating_session_surfaces_property_violation ... ok"),
        "pinned debug violation test missing:\n{debug}"
    );
    // Release: the monitors are compiled out - the same violating sequence
    // steps all Ok.
    let release = cargo_member(&["test", "-p", "monitors_production", "--release"]);
    assert!(
        release.contains("release_build_compiles_monitors_out ... ok"),
        "pinned release zero-cost test missing:\n{release}"
    );
}

#[test]
fn callback_minimal_adapters_exercises_both_adapters() {
    cargo_member(&["build", "-q", "-p", "callback_minimal_adapters"]);
    let out = cargo_member(&["test", "-p", "callback_minimal_adapters"]);
    assert!(
        out.contains("callback_completion_stops_after_drop ... ok"),
        "pinned callback completion test missing:\n{out}"
    );
    assert!(
        out.contains("minimal_composes_with_the_callback_driver ... ok"),
        "pinned minimal composition test missing:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// Phase 17 task 8: the four PLAIN-DIR example projects (script-driven, NOT
// workspace members - the c_sync_handler / fuzz_c convention). Each leg runs
// the example's real script/Makefile and asserts PINNED excerpts from the real
// captures. Legs needing a real tool gate on availability and skip with a note
// (never fail on absence).
// ---------------------------------------------------------------------------

/// Run a shell script in `dir`, returning (exit code, combined stdout+stderr).
fn run_script(dir: &std::path::Path, script: &str) -> (i32, String) {
    let out = std::process::Command::new("sh")
        .arg(script)
        .current_dir(dir)
        .output()
        .expect("spawn sh");
    (
        out.status.code().unwrap_or(-1),
        format!(
            "{}{}",
            String::from_utf8_lossy(&out.stdout),
            String::from_utf8_lossy(&out.stderr)
        ),
    )
}

/// A cold checkout's project-local rustup toolchain is not on PATH; the scripts
/// fall back to it, so its presence counts as "rustc available" for gating.
fn project_local_rustc_exists() -> bool {
    repo_root()
        .join("../rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc")
        .is_file()
}

#[test]
fn mc_walkthrough_generates_both_backends_and_pins_reports() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - mc_walkthrough check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/mc_walkthrough");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The PASSING spec: both backends report the same complete pass.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=8 transitions=12 max_depth=5"),
        "pinned PASS summary missing:\n{out}"
    );
    // The FAILING variant: the property violation + counterexample.
    assert!(
        out.contains("reason=property-violation label=ack_bounded step=2"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("trigger step: 1"),
        "pinned trigger step missing:\n{out}"
    );
    assert!(
        out.contains("violation step: 2"),
        "pinned violation step missing:\n{out}"
    );
    // The JSON counterexample document.
    assert!(
        out.contains("\"status\":\"fail\""),
        "pinned JSON status missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"ack_bounded\""),
        "pinned JSON label missing:\n{out}"
    );
    assert!(
        out.contains("\"kind\":\"trace\""),
        "pinned JSON ce kind missing:\n{out}"
    );
}

#[test]
fn json_schema_ci_validates_all_five_formats() {
    let dir = repo_root().join("crates/examples/json_schema_ci");
    // The script exits 0 both on success AND when it skips (python3/jsonschema
    // absent) - distinguish the two by the skip note.
    let (code, out) = run_script(&dir, "validate.sh");
    assert_eq!(code, 0, "validate.sh failed:\n{out}");
    if out.contains("check skipped") {
        eprintln!("note: python3 jsonschema not available - json_schema_ci check skipped");
        return;
    }
    for fmt in [
        "spec-json",
        "ce-trace",
        "mc-report",
        "validate-json",
        "info-json",
    ] {
        assert!(
            out.contains(&format!("{fmt}: VALID")),
            "pinned {fmt} VALID missing:\n{out}"
        );
    }
    assert!(
        out.contains("all five formats validated against their schemas"),
        "pinned summary line missing:\n{out}"
    );
}

#[test]
fn specgen_mutation_loop_pins_the_real_transcript() {
    // No external tools needed - gen-spec / validate / mc-ref are in-process.
    let dir = repo_root().join("crates/examples/specgen_mutation_loop");
    let (code, out) = run_script(&dir, "loop.sh");
    assert_eq!(code, 0, "loop.sh failed:\n{out}");
    // Seed determinism is proven, not assumed.
    assert!(
        out.contains("DETERMINISM: base.rfs and base_again.rfs are byte-identical"),
        "pinned determinism line missing:\n{out}"
    );
    // The base spec passes model checking.
    assert!(
        out.contains("PASS (complete)"),
        "pinned base PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=4 transitions=12 max_depth=1"),
        "pinned base summary missing:\n{out}"
    );
    // The mutation fails at the initial configuration.
    assert!(
        out.contains("reason=property-violation label=p2 step=0"),
        "pinned mutation FAIL missing:\n{out}"
    );
    assert!(
        out.contains("violation step: 0 (initial configuration)"),
        "pinned initial-config violation missing:\n{out}"
    );
}

#[test]
fn tool_resolution_proves_precedence_and_builds_under_both_compilers() {
    let dir = repo_root().join("crates/examples/tool_resolution");
    // run.sh: the # tools: proof lines (self-resolves rfs + compilers).
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // An explicit RFS_GCC env var wins (the script's deterministic default).
    assert!(
        out.contains("# tools: gcc=/usr/bin/gcc"),
        "pinned explicit-gcc line missing:\n{out}"
    );
    // A CLI flag beats the env var.
    assert!(
        out.contains("# tools: gcc=/bin/sh-fake"),
        "pinned flag-beats-env line missing:\n{out}"
    );
    // Nothing set falls through to bare PATH resolution.
    assert!(
        out.contains("# tools: gcc=gcc"),
        "pinned bare-path line missing:\n{out}"
    );
    // The Rust scenario resolved a rustc (an absolute path, or the bare name).
    assert!(
        out.contains("# tools: rustc="),
        "pinned rustc tools line missing:\n{out}"
    );

    // Makefile: build the generated C FSM under gcc (the default CC) ...
    if !common::toolchain::available_named("gcc")
        || !common::toolchain::available_env_or_path("RFS_MAKE", "make")
    {
        eprintln!("note: gcc/make not available - tool_resolution Makefile check skipped");
        return;
    }
    let rfs = rfs_bin();
    let out = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .arg("clean")
        .output()
        .expect("spawn make clean");
    assert!(out.status.success());
    let out = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .env("RFS", rfs)
        .output()
        .expect("spawn make (gcc)");
    assert!(
        out.status.success(),
        "make (gcc) failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    // ... and under clang (availability-gated - the documented alternative).
    if common::toolchain::available_named("clang") {
        let _ = std::process::Command::new(common::toolchain::make())
            .current_dir(&dir)
            .arg("clean")
            .output();
        let out = std::process::Command::new(common::toolchain::make())
            .current_dir(&dir)
            .env("RFS", rfs)
            .env("CC", "clang")
            .output()
            .expect("spawn make CC=clang");
        assert!(
            out.status.success(),
            "make CC=clang failed: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    } else {
        eprintln!("note: clang not available - tool_resolution clang leg skipped");
    }
    let _ = std::process::Command::new(common::toolchain::make())
        .current_dir(&dir)
        .arg("clean")
        .output();
}
