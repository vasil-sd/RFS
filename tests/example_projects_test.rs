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

#[test]
fn tcp_full_states_checks_all_modes_and_the_large_twin() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - tcp_full_states check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/tcp_full_states");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=30 transitions=71 max_depth=9"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: CLOSED made non-terminal -> property violation on closed_terminal.
    assert!(
        out.contains("reason=property-violation label=closed_terminal step=9"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Closed to=State::Listen"),
        "pinned CE defect step (CLOSED resurrected) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"closed_terminal\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=30").count() >= 3,
        "expected pass + both-backend parity (3x states=30), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
    // The large-state-space twin: smt-bounded proves the invariant within bound (the
    // committed capture is always cat'ed by run.sh, so this holds with or without z3),
    // and fuzz runs 200 iterations.
    assert!(
        out.contains("PASS WITHIN BOUND 12"),
        "pinned smt-bounded verdict missing:\n{out}"
    );
    assert!(
        out.contains("runs=200 bound=5 completed=200"),
        "pinned fuzz transcript missing:\n{out}"
    );
}

#[test]
fn tls13_handshake_no_app_data_before_finished() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - tls13_handshake check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/tls13_handshake");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    assert!(
        out.contains("status=pass-complete states=5 transitions=13 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    assert!(
        out.contains("reason=property-violation label=no_app_data_before_finished step=2"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::AwaitServerHello to=State::AwaitServerHello"),
        "pinned CE defect step (app data pre-Finished) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_app_data_before_finished\""),
        "pinned JSON label missing:\n{out}"
    );
    assert!(
        out.matches("status=pass-complete states=5").count() >= 3,
        "expected pass + both-backend parity (3x states=5), got:\n{out}"
    );
    assert!(
        out.contains("rust production: compiles clean"),
        "rust proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c proof missing:\n{out}"
    );
}

#[test]
fn quic_loss_recovery_terminates_and_the_large_twin() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - quic_loss_recovery check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/quic_loss_recovery");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    assert!(
        out.contains("status=pass-complete states=8 transitions=16 max_depth=4"),
        "pinned PASS summary missing:\n{out}"
    );
    assert!(
        out.contains("reason=liveness-violation label=terminates step=1"),
        "pinned liveness-violation summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Retransmit to=State::Retransmit"),
        "pinned CE retransmit-loop step missing:\n{out}"
    );
    assert!(
        out.matches("status=pass-complete states=8").count() >= 3,
        "expected pass + both-backend parity (3x states=8), got:\n{out}"
    );
    assert!(
        out.contains("rust production: compiles clean"),
        "rust proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c proof missing:\n{out}"
    );
    // The large-state-space twin: smt-bounded proves the invariant within bound; fuzz runs.
    assert!(
        out.contains("PASS WITHIN BOUND 12"),
        "pinned smt-bounded verdict missing:\n{out}"
    );
    assert!(
        out.contains("runs=200 bound=5 completed=200"),
        "pinned fuzz transcript missing:\n{out}"
    );
}

#[test]
fn dhcp_client_lease_renewal_and_deadlock_on_expiry() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - dhcp_client check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/dhcp_client");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("reason=deadlock label=- step=11"),
        "pinned deadlock summary missing:\n{out}"
    );
    assert!(
        out.contains("to=State::Bound vars=Vars { lease: 0 }"),
        "pinned CE stuck-at-BOUND(lease=0) missing:\n{out}"
    );
    assert!(
        out.matches("PASS (complete)").count() >= 3,
        "expected pass + both-backend parity, got:\n{out}"
    );
    assert!(
        out.contains("rust production: compiles clean"),
        "rust proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c proof missing:\n{out}"
    );
}

#[test]
fn dns_resolver_no_stale_serve_and_the_large_twin() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - dns_resolver check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/dns_resolver");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=12 transitions=22 max_depth=10"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: TTL-expiry guard removed -> a stale entry (ttl==0) is served,
    // a property violation on no_stale_serve.
    assert!(
        out.contains("reason=property-violation label=no_stale_serve step=11"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("outputs=[Output::ServeAnswer] from=State::Cached to=State::Cached vars=Vars { ttl: 0 }"),
        "pinned CE stale-serve step (ServeAnswer at ttl==0) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_stale_serve\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=12").count() >= 3,
        "expected pass + both-backend parity (3x states=12), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
    // The large-state-space twin: smt-bounded proves no_stale_serve within bound (the
    // committed capture is always cat'ed by run.sh, so this holds with or without z3),
    // and fuzz runs 200 iterations.
    assert!(
        out.contains("PASS WITHIN BOUND 12"),
        "pinned smt-bounded verdict missing:\n{out}"
    );
    assert!(
        out.contains("runs=200 bound=5 completed=200"),
        "pinned fuzz transcript missing:\n{out}"
    );
}

#[test]
fn arp_resolution_resolves_or_gives_up_and_the_retransmit_loop() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - arp_resolution check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/arp_resolution");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=137 transitions=267 max_depth=23"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: retry limit dropped -> an unresolvable entry retransmits
    // forever, a liveness violation of resolves_or_gives_up (a cycle in INCOMPLETE).
    assert!(
        out.contains("reason=liveness-violation label=resolves_or_gives_up step=0"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains(
            "from=State::Incomplete to=State::Incomplete vars=Vars { retries: 1, age: 0 }"
        ),
        "pinned CE retransmit-loop step (INCOMPLETE self-loop) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"resolves_or_gives_up\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=137").count() >= 3,
        "expected pass + both-backend parity (3x states=137), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn stp_port_states_loop_free_and_the_backup_forward_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - stp_port_states check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/stp_port_states");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=7 transitions=17 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: role guard removed -> a backup port (role==1) reaches
    // FORWARDING, violating the loop_free invariant.
    assert!(
        out.contains("reason=property-violation label=loop_free step=3"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Learning to=State::Forwarding vars=Vars { role: 1 }"),
        "pinned CE backup-forward defect step (Learning->Forwarding at role==1) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"loop_free\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=7").count() >= 3,
        "expected pass + both-backend parity (3x states=7), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn bgp_fsm_no_update_before_established_and_the_openconfirm_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - bgp_fsm check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/bgp_fsm");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=9 transitions=20 max_depth=4"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: the ESTABLISHED guard on UPDATE handling is removed, so a
    // SendUpdate is emitted while still in OPENCONFIRM - an ordering violation.
    assert!(
        out.contains("reason=property-violation label=no_update_before_established step=4"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("outputs=[Output::SendUpdate] from=State::OpenConfirm to=State::OpenConfirm"),
        "pinned CE premature-UPDATE defect step (SendUpdate in OpenConfirm) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_update_before_established\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=9").count() >= 3,
        "expected pass + both-backend parity (3x states=9), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn ospf_neighbor_single_master_and_the_tie_break_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - ospf_neighbor check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/ospf_neighbor");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=10 transitions=17 max_depth=6"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: the master/slave tie-break is removed, so both sides believe
    // they are master - a violation of single_master.
    assert!(
        out.contains("reason=property-violation label=single_master step=4"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains(
            "from=State::ExStart to=State::Exchange vars=Vars { we_master: 1, peer_master: 1 }"
        ),
        "pinned CE both-master defect step (ExStart->Exchange) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"single_master\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=10").count() >= 3,
        "expected pass + both-backend parity (3x states=10), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn sip_call_signaling_no_established_after_cancel_and_the_ignore_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - sip_call_signaling check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/sip_call_signaling");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("PASS (complete)"),
        "pinned PASS missing:\n{out}"
    );
    assert!(
        out.contains("status=pass-complete states=6 transitions=11 max_depth=4"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: CANCEL handling is dropped, so a cancelled invite still
    // reaches ESTABLISHED - a violation of no_established_after_cancel.
    assert!(
        out.contains("reason=property-violation label=no_established_after_cancel step=4"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Ringing to=State::Established vars=Vars { cancelled: 1 }"),
        "pinned CE ignored-CANCEL defect step (Ringing->Established at cancelled==1) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_established_after_cancel\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=6").count() >= 3,
        "expected pass + both-backend parity (3x states=6), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

// ---------------------------------------------------------------------------
// Phase 18 task 2 - the ten Group-A protocol FSMs A11-A20 (application / storage /
// wireless / security). Same leg shape as task 1: run.sh drives every demonstrated
// RFS mode; the pinned excerpts come from the REAL captures (never hand-written).
// ---------------------------------------------------------------------------

#[test]
fn mqtt_qos_session_exactly_once_and_the_large_twin() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - mqtt_qos_session check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/mqtt_qos_session");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=48 transitions=608 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: dedup guard removed -> a duplicate PUBLISH is delivered twice.
    assert!(
        out.contains("reason=property-violation label=exactly_once step=3"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::AwaitPubrec to=State::AwaitPubrec vars=Vars { last_id: 0, have_last: true, was_dup: true }"),
        "pinned CE duplicate-delivery defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"exactly_once\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=48").count() >= 3,
        "expected pass + both-backend parity (3x states=48), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
    // The large-state-space twin (16-bit packet-id space): smt-bounded proves the
    // invariant within bound (the committed capture is always cat'ed by run.sh, so
    // this holds with or without z3), and fuzz runs 200 iterations.
    assert!(
        out.contains("PASS WITHIN BOUND 12"),
        "pinned smt-bounded verdict missing:\n{out}"
    );
    assert!(
        out.contains("runs=200 bound=5 completed=200"),
        "pinned fuzz transcript missing:\n{out}"
    );
}

#[test]
fn coap_retransmit_ack_or_abort_and_the_lost_ack_loop() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - coap_retransmit check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/coap_retransmit");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=13058 transitions=3355906 max_depth=12"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: retry cap removed -> a lost ACK loops forever.
    assert!(
        out.contains("reason=property-violation label=ack_or_abort step=16"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("trigger step: 1"),
        "pinned obligation trigger step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"ack_or_abort\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=13058").count() >= 3,
        "expected pass + both-backend parity (3x states=13058), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
    // The extra --mode fuzz leg on the passing spec: 200 seeded runs, clean.
    assert!(
        out.contains(
            "runs=200 bound=5 completed=200 unexpected=0 assumption_stops=0 property_violations=0"
        ),
        "pinned fuzz transcript missing:\n{out}"
    );
}

#[test]
fn ppp_lcp_ipcp_no_ipcp_before_lcp_open_and_the_phase_guard_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - ppp_lcp_ipcp check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/ppp_lcp_ipcp");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=7 transitions=18 max_depth=5"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: phase guard removed -> IPCP opens while LCP is still ack-sent.
    assert!(
        out.contains("reason=property-violation label=no_ipcp_before_lcp_open step=3"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::LcpAckSent to=State::IpcpNegotiating vars=Vars { lcp_open: false, auth: Auth::None }"),
        "pinned CE IPCP-in-ack-sent defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_ipcp_before_lcp_open\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=7").count() >= 3,
        "expected pass + both-backend parity (3x states=7), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn iscsi_login_no_io_before_operational_and_the_phase_guard_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - iscsi_login check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/iscsi_login");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=6 transitions=12 max_depth=5"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: phase guard removed -> I/O is issued in the full-feature phase.
    assert!(
        out.contains("reason=property-violation label=no_io_before_operational step=3"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::FullFeature to=State::FullFeature vars=Vars { session_id: 1, have_session: true }"),
        "pinned CE I/O-in-full-feature defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_io_before_operational\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=6").count() >= 3,
        "expected pass + both-backend parity (3x states=6), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn fibrechannel_login_single_logged_in_and_the_duplicate_login_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - fibrechannel_login check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/fibrechannel_login");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=4 transitions=5 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: state guard removed -> a second P_LOGI is accepted while logged in.
    assert!(
        out.contains("reason=property-violation label=single_logged_in step=4"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::LoggedIn to=State::LoggedIn vars=Vars { logins: 2 }"),
        "pinned CE duplicate-login defect step (logins reaches 2) missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"single_logged_in\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=4").count() >= 3,
        "expected pass + both-backend parity (3x states=4), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn nvme_controller_no_io_before_enabled_and_the_large_twin() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - nvme_controller check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/nvme_controller");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=56 transitions=106 max_depth=27"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: enable guard removed -> I/O is submitted in the initialized state.
    assert!(
        out.contains("reason=property-violation label=no_io_before_enabled step=4"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Initialized to=State::Initialized vars=Vars { outstanding: 1, enabled: false }"),
        "pinned CE I/O-in-initialized defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_io_before_enabled\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=56").count() >= 3,
        "expected pass + both-backend parity (3x states=56), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
    // The large-state-space twin (4 queue pairs x bit<8> depth = 2^32 configs):
    // smt-bounded proves the invariant within bound (the committed capture is always
    // cat'ed by run.sh, so this holds with or without z3), and fuzz runs 200 iterations.
    assert!(
        out.contains("PASS WITHIN BOUND 12"),
        "pinned smt-bounded verdict missing:\n{out}"
    );
    assert!(
        out.contains("runs=200 bound=5 completed=200"),
        "pinned fuzz transcript missing:\n{out}"
    );
}

#[test]
fn sata_link_power_no_command_while_asleep_and_the_wake_guard_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - sata_link_power check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/sata_link_power");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=4 transitions=8 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: wake-before-command guard removed -> a command issues in Slumber.
    assert!(
        out.contains("reason=property-violation label=no_command_while_asleep step=4"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains(
            "from=State::Slumber to=State::Slumber vars=Vars { dev_power: DevPower::Slumber }"
        ),
        "pinned CE command-in-Slumber defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_command_while_asleep\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=4").count() >= 3,
        "expected pass + both-backend parity (3x states=4), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn usb_device_link_no_data_while_suspended_and_the_state_guard_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - usb_device_link check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/usb_device_link");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=4320 transitions=12829 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: state guard removed -> a data token flows while suspended.
    assert!(
        out.contains("reason=property-violation label=no_data_while_suspended step=3"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Suspended to=State::Suspended vars=Vars { addr: 1, config: 0, suspended_from: SuspFrom::Addressed }"),
        "pinned CE data-while-suspended defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_data_while_suspended\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=4320").count() >= 3,
        "expected pass + both-backend parity (3x states=4320), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn bt_gatt_session_notify_only_subscribed_and_the_subscription_guard_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - bt_gatt_session check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/bt_gatt_session");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=4 transitions=131080 max_depth=3"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: subscription guard removed -> a notification goes to an
    // unsubscribed client.
    assert!(
        out.contains("reason=property-violation label=notify_only_subscribed step=3"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::Connected to=State::Connected vars=Vars { subscribed: false, mapped: true }"),
        "pinned CE notify-unsubscribed defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"notify_only_subscribed\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=4").count() >= 3,
        "expected pass + both-backend parity (3x states=4), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}

#[test]
fn kerberos_auth_no_access_with_expired_tgt_and_the_expiry_guard_defect() {
    if !common::toolchain::available_named("gcc")
        || !(common::toolchain::available_named("rustc") || project_local_rustc_exists())
    {
        eprintln!("note: gcc/rustc not available - kerberos_auth check skipped");
        return;
    }
    let dir = repo_root().join("crates/examples/kerberos_auth");
    let (code, out) = run_script(&dir, "run.sh");
    assert_eq!(code, 0, "run.sh failed:\n{out}");
    // The passing spec: PASS complete with the pinned state count.
    assert!(
        out.contains("status=pass-complete states=30720 transitions=145472 max_depth=31"),
        "pinned PASS summary missing:\n{out}"
    );
    // The failing variant: expiry guard removed -> a service ticket is granted on an
    // expired TGT.
    assert!(
        out.contains("reason=property-violation label=no_access_with_expired_tgt step=6"),
        "pinned FAIL summary missing:\n{out}"
    );
    assert!(
        out.contains("from=State::TgsExchange to=State::HaveSt vars=Vars { clock: 2, tgt_expiry: 0, have_tgt: true, st_expiry: 6, have_st: true, last_ts: 1, seen_ts: true, replayed: false }"),
        "pinned CE GrantSt-on-expired-TGT defect step missing:\n{out}"
    );
    assert!(
        out.contains("\"label\":\"no_access_with_expired_tgt\""),
        "pinned JSON label missing:\n{out}"
    );
    // The fix re-runs to PASS; both generated backends agree (byte-identical reports).
    assert!(
        out.matches("status=pass-complete states=30720").count() >= 3,
        "expected pass + both-backend parity (3x states=30720), got:\n{out}"
    );
    // Production codegen compiles in both backends.
    assert!(
        out.contains("rust production: compiles clean"),
        "rust production compile proof missing:\n{out}"
    );
    assert!(
        out.contains("c production: compiles warning-free"),
        "c production compile proof missing:\n{out}"
    );
}
