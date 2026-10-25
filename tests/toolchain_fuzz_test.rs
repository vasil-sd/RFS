//! Toolchain fuzzing - the SLOW compile-and-run layer (Phase 6 task 5,
//! D-P6-4). This is the CI-runnable half of the two-layer strategy; the fast
//! in-process libFuzzer layer lives in `fuzz/` (manual/nightly).
//!
//! Seeded deterministic suite: random VALID specs from the task-3 `specgen`
//! builder (pinned seeds, small bounds) are rendered to source and re-parsed
//! and validated (exercising printer + parser), then ALL six artifacts
//! (2 backends x production/test/mc) are generated and COMPILED (rustc /
//! gcc -Wall - compilation success IS the "no invalid target-language code"
//! check). The MC binaries run in exact mode (shared small `--max-states`
//! cap) AND fuzz mode; per spec we assert:
//!
//! - MC exit codes mirror the status taxonomy {0 pass-complete, 1 fail,
//!   2 unknown-state-limit} and fuzz-mode runs stay in {0, 1, 2};
//! - NO FALSE PASS: where `ref_mc` (independent ground truth, D-P6-5) says
//!   Fail, a generated engine must NOT say pass-complete - and full status
//!   agreement holds per the task 4.1 semantics;
//! - STABLE DIAGNOSTICS: each MC binary run twice -> stderr byte-identical.
//!
//! Documented asymmetry handled like Phase 6 task 4.1: an assumption already
//! violated at the initial configuration makes `ref_mc` report a clean error
//! while the generated binaries panic/abort - both naming the SAME label;
//! that pinned behavior is asserted instead of a status comparison.

use std::path::{Path, PathBuf};
use std::process::Command;

/// Pinned seed sets: 3 seeds x 10 specs (specgen stateful sequence per seed).
/// `24099` is the regression seed of plan correction 5a - its first spec is
/// the one the task-5 `codegen` fuzz target found (reproducible via
/// `rfs gen-spec --seed 24099 --size small`).
const SEEDS: [u64; 3] = [605_001, 605_002, 24099];
const SPECS_PER_SEED: usize = 10;

/// Shared exact-mode state cap for ref_mc AND the generated engines. With an
/// identical cap both sides either complete or report UnknownStateLimit
/// together (configuration counting is aligned per Phase 6 tasks 3/4), so
/// status agreement holds regardless of the actual space size - and the cap
/// keeps every run fast.
const MAX_STATES: usize = 5000;
const TIMEOUT_SEC: u64 = 60;
const FUZZ_RUNS: &str = "5";
const FUZZ_BOUND: &str = "5";

/// gcc location (plan assumption, verified since Phase 2).
const GCC: &str = "/usr/bin/gcc";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// Per-process scratch dir under `target/` (the established hygiene pattern
/// of the sibling suites - unique per process, cleaned by `cargo clean`).
fn scratch_dir() -> PathBuf {
    let dir = root()
        .join("target")
        .join("rfs_toolchain_fuzz")
        .join(format!("{}", std::process::id()));
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

// ---------------------------------------------------------------------------
// Compilation (success = no invalid target-language code)
// ---------------------------------------------------------------------------

fn compile_rust(tag: &str, backend_name: &str, kind: &str, src: &Path, out: &Path) {
    let mut cmd = Command::new(rustc());
    cmd.args(["--edition", "2021"]);
    if kind == "production" {
        // Library artifact (no main): compile as an rlib.
        cmd.arg("--crate-type").arg("lib");
    } else if kind == "test" {
        // Test artifact: libtest harness, like the `rfs test` command.
        cmd.arg("--test");
    }
    let status = cmd
        .arg(src)
        .arg("-o")
        .arg(out)
        .status()
        .unwrap_or_else(|e| panic!("[{tag}/{backend_name}/{kind}] run rustc: {e}"));
    assert!(
        status.success(),
        "[{tag}/{backend_name}/{kind}] rustc failed on generated code"
    );
}

fn compile_c(tag: &str, backend_name: &str, src: &Path, out: &Path, compile_only: bool) {
    let mut cmd = Command::new(GCC);
    cmd.arg("-Wall");
    if compile_only {
        // No main in the production artifact: compile-only (object file).
        cmd.arg("-c");
    }
    let status = cmd
        .arg(src)
        .arg("-o")
        .arg(out)
        .status()
        .unwrap_or_else(|e| panic!("[{tag}/{backend_name}] run gcc: {e}"));
    assert!(
        status.success(),
        "[{tag}/{backend_name}] gcc -Wall failed on generated code"
    );
}

// ---------------------------------------------------------------------------
// MC binary runs
// ---------------------------------------------------------------------------

struct Run {
    code: i32,
    stdout: String,
    stderr: String,
}

fn run_mc(bin: &Path, args: &[&str], tag: &str, backend_name: &str, mode: &str) -> Run {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("[{tag}/{backend_name}/{mode}] run MC binary: {e}"));
    let code = out
        .status
        .code()
        .unwrap_or_else(|| panic!("[{tag}/{backend_name}/{mode}] MC binary crashed (signal)"));
    Run {
        code,
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
        stderr: String::from_utf8_lossy(&out.stderr).to_string(),
    }
}

fn doc_status(doc: &rfs::json_rw::Json) -> String {
    doc.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>")
        .to_string()
}

// ---------------------------------------------------------------------------
// The per-spec flow
// ---------------------------------------------------------------------------

fn run_one(tag: &str, ir: &rfs::Ir) {
    // 1. Render -> re-parse -> validate (exercises printer + parser on every
    //    generated spec).
    let src = rfs::print_ir(ir);
    let parsed =
        rfs::parse_source(&src).unwrap_or_else(|e| panic!("[{tag}] re-parse of printed spec: {e}"));
    rfs::validator::validate_ir(&parsed)
        .unwrap_or_else(|e| panic!("[{tag}] validate of re-parsed spec: {e:?}"));

    let dir = scratch_dir();

    // 2. Generate ALL six artifacts (the MC artifact is baked with the JSON
    //    report so its stdout is machine-readable).
    for backend_name in ["rust", "c"] {
        let backend = rfs::codegen::get_backend(backend_name)
            .unwrap_or_else(|| panic!("[{tag}] backend {backend_name} registered"));
        let ext = if backend_name == "c" { "c" } else { "rs" };
        for (kind, command, options) in [
            (
                "production",
                rfs::codegen::CodegenCommandKind::Production,
                rfs::codegen::BackendOptions::default(),
            ),
            (
                "test",
                rfs::codegen::CodegenCommandKind::UnitTests,
                rfs::codegen::BackendOptions::default(),
            ),
            (
                "mc",
                rfs::codegen::CodegenCommandKind::ModelChecker,
                rfs::codegen::BackendOptions {
                    counterexample_format: rfs::codegen_mc::CounterexampleFormat::Json,
                    ..rfs::codegen::BackendOptions::default()
                },
            ),
        ] {
            let code = command
                .run(backend, &parsed, &options)
                .unwrap_or_else(|e| panic!("[{tag}/{backend_name}] {kind} codegen: {e:?}"));
            let src_path = dir.join(format!("{tag}_{backend_name}_{kind}.{ext}"));
            std::fs::write(&src_path, code.code.as_bytes())
                .unwrap_or_else(|e| panic!("[{tag}/{backend_name}] write {kind} source: {e}"));
            // 3. Compile (success = no invalid target-language code).
            let compile_only = kind == "production";
            let suffix = if backend_name == "rust" {
                if compile_only {
                    ".rlib"
                } else {
                    "_bin"
                }
            } else if compile_only {
                ".o"
            } else {
                "_bin"
            };
            let out_path = dir.join(format!("{tag}_{backend_name}_{kind}{suffix}"));
            if backend_name == "rust" {
                compile_rust(tag, backend_name, kind, &src_path, &out_path);
            } else {
                compile_c(tag, backend_name, &src_path, &out_path, compile_only);
            }
        }
    }

    // 4. ref_mc ground truth + generated-engine runs.
    let module = &parsed.modules[0];
    let cfg = rfs::ref_mc::RefConfig {
        max_states: MAX_STATES,
        timeout_sec: TIMEOUT_SEC,
        assume_arg: "all".to_string(),
    };
    match rfs::ref_mc::run(module, &cfg) {
        Ok(res) => check_status_agreement(tag, &res),
        Err(e) => check_sample0_assumption_abort(tag, &e),
    }
}

/// Exact + fuzz runs of BOTH generated engines vs the ref_mc verdict.
fn check_status_agreement(tag: &str, res: &rfs::ref_mc::RefResult) {
    let want_exit = match res.status {
        rfs::codegen_mc::ExactStatus::PassComplete => 0,
        rfs::codegen_mc::ExactStatus::Fail => 1,
        rfs::codegen_mc::ExactStatus::UnknownStateLimit => 2,
    };
    for (bi, backend_name) in ["rust", "c"].iter().enumerate() {
        let bin = scratch_dir().join(format!("{tag}_{backend_name}_mc_bin"));
        // Exact mode: status + exit code, twice for stderr stability.
        let r1 = run_mc(
            &bin,
            &["--max-states", &MAX_STATES.to_string()],
            tag,
            backend_name,
            "exact",
        );
        let r2 = run_mc(
            &bin,
            &["--max-states", &MAX_STATES.to_string()],
            tag,
            backend_name,
            "exact",
        );
        assert_eq!(
            r1.stderr, r2.stderr,
            "[{tag}/{backend_name}] exact-mode stderr must be stable across runs"
        );
        assert_eq!(
            r1.code, want_exit,
            "[{tag}/{backend_name}] exact exit code must mirror the status taxonomy"
        );
        let doc = rfs::json_rw::parse(r1.stdout.trim_end()).unwrap_or_else(|e| {
            panic!(
                "[{tag}/{backend_name}] MC JSON must strict-parse: {e}\n{}",
                r1.stdout
            )
        });
        let status = doc_status(&doc);
        // NO FALSE PASS (the Phase 6 invariant): a ref Fail must never be
        // reported as pass-complete by a generated engine.
        if matches!(res.status, rfs::codegen_mc::ExactStatus::Fail) {
            assert_ne!(
                status, "pass-complete",
                "[{tag}/{backend_name}] NO FALSE PASS: ref_mc says Fail"
            );
        }
        // Full status agreement (task 4.1 semantics).
        assert_eq!(
            status,
            res.status.wire(),
            "[{tag}/{backend_name}] exact status must agree with ref_mc"
        );
        // Fuzz mode: exit codes in {0, 1, 2}, stderr stable across two runs.
        let fuzz_args: Vec<String> = vec![
            "--mode".into(),
            "fuzz".into(),
            "--runs".into(),
            FUZZ_RUNS.into(),
            "--bound".into(),
            FUZZ_BOUND.into(),
            "--seed".into(),
            (24099 + bi as u64 * 1000).to_string(),
        ];
        let fargs: Vec<&str> = fuzz_args.iter().map(|s| s.as_str()).collect();
        let f1 = run_mc(&bin, &fargs, tag, backend_name, "fuzz");
        let f2 = run_mc(&bin, &fargs, tag, backend_name, "fuzz");
        assert!(
            matches!(f1.code, 0..=2),
            "[{tag}/{backend_name}] fuzz-mode exit code {code} outside {{0, 1, 2}}",
            code = f1.code
        );
        assert_eq!(
            f1.stderr, f2.stderr,
            "[{tag}/{backend_name}] fuzz-mode stderr must be stable across runs"
        );
    }
}

/// The documented sample-0 assumption-violation asymmetry (Phase 6 task 4.1):
/// ref_mc reports a clean error; the generated binaries panic/abort naming
/// the SAME label - in BOTH exact and fuzz mode (both construct the FSM, which
/// steps sample 0).
fn check_sample0_assumption_abort(tag: &str, err: &str) {
    let cap = "assumption '";
    let Some(start) = err.find(cap) else {
        panic!("[{tag}] unexpected ref_mc error (not a sample-0 assumption violation): {err}")
    };
    let rest = &err[start + cap.len()..];
    let end = rest
        .find('\'')
        .unwrap_or_else(|| panic!("[{tag}] malformed ref_mc error: {err}"));
    let label = &rest[..end];
    let max_states = MAX_STATES.to_string();
    for backend_name in ["rust", "c"] {
        let bin = scratch_dir().join(format!("{tag}_{backend_name}_mc_bin"));
        for (mode, args) in [
            ("exact", vec!["--max-states", &max_states]),
            (
                "fuzz",
                vec![
                    "--mode", "fuzz", "--runs", FUZZ_RUNS, "--bound", FUZZ_BOUND, "--seed", "1",
                ],
            ),
        ] {
            let out = Command::new(&bin)
                .args(&args)
                .output()
                .unwrap_or_else(|e| panic!("[{tag}/{backend_name}/{mode}] run MC binary: {e}"));
            assert!(
                !out.status.success(),
                "[{tag}/{backend_name}/{mode}] sample-0 assumption violation must fail the run"
            );
            let stderr = String::from_utf8_lossy(&out.stderr);
            assert!(
                stderr.contains(&format!("RFS assumption violated: {label}")),
                "[{tag}/{backend_name}/{mode}] panic/abort must name the violated assumption:\n{stderr}"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// The suite
// ---------------------------------------------------------------------------

#[test]
fn c_mc_trigger_step_zero_report_no_crash() {
    // Regression pin (plan correction 5b): when a fuzz-mode run reports a
    // property violation whose obligation started at SAMPLE 0 (trigger step
    // 0), the C report path must not index `steps[-1]`. The micro-spec below
    // is deterministic: `bounded_eventually[2] (output_has(Ping))` starts its
    // obligation at sample 0 and Ping is never emitted, so every fuzz run
    // confirms the violation with trigger step 0. Before the fix the generated
    // C MC binary SEGFAULTED here (ASan: SEGV READ in print_vars_debug); the
    // Rust backend was already correct (`vars_at` handles pos == 0).
    let src = "module trigger0 { enum State { A, B } initial A event Go output Ping transition A on Go { next B; } transition B on Go { next A; } prop p1 : bounded_eventually[2] (output_has(Ping)) ; }";
    let ir = rfs::parse_source(src).expect("micro-spec parses");
    let dir = scratch_dir();
    for backend_name in ["rust", "c"] {
        let backend = rfs::codegen::get_backend(backend_name)
            .unwrap_or_else(|| panic!("backend {backend_name} registered"));
        let code = rfs::codegen::CodegenCommandKind::ModelChecker
            .run(backend, &ir, &rfs::codegen::BackendOptions::default())
            .unwrap_or_else(|e| panic!("{backend_name} mc codegen: {e:?}"));
        let ext = if backend_name == "c" { "c" } else { "rs" };
        let src_path = dir.join(format!("trigger0_{backend_name}_mc.{ext}"));
        std::fs::write(&src_path, code.code.as_bytes()).expect("write source");
        let bin = dir.join(format!("trigger0_{backend_name}_mc_bin"));
        let status = if backend_name == "c" {
            Command::new(GCC)
                .arg("-Wall")
                .arg(&src_path)
                .arg("-o")
                .arg(&bin)
                .status()
                .expect("run gcc")
        } else {
            Command::new(rustc())
                .args(["--edition", "2021"])
                .arg(&src_path)
                .arg("-o")
                .arg(&bin)
                .status()
                .expect("run rustc")
        };
        assert!(
            status.success(),
            "{backend_name}: compile of generated MC failed"
        );
        let out = Command::new(&bin)
            .args([
                "--mode", "fuzz", "--runs", "3", "--bound", "4", "--seed", "1",
            ])
            .output()
            .expect("run MC binary");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{backend_name}: fuzz run must report the violation (exit 1), not crash"
        );
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert!(
            stdout.contains("trigger step: 0"),
            "{backend_name}: trigger-step-0 report line missing:\n{stdout}"
        );
        assert!(
            stdout.contains("vars at trigger step:"),
            "{backend_name}: vars-at-trigger-step line missing (the pre-fix C code \
             crashed rendering it):\n{stdout}"
        );
    }
}

#[test]
fn toolchain_fuzz_slow_suite() {
    let t0 = std::time::Instant::now();
    let mut count = 0usize;
    for &seed in &SEEDS {
        let mut gen = rfs::specgen::SpecGen::new(seed, rfs::specgen::Bounds::small());
        for i in 0..SPECS_PER_SEED {
            let ir = gen.generate();
            run_one(&format!("s{seed}n{i}"), &ir);
            count += 1;
        }
    }
    println!(
        "toolchain_fuzz: {count} specs x (6 artifacts + compile + MC runs + ref_mc) in {:?}",
        t0.elapsed()
    );
}
