//! Benchmark MC suite (Phase 4 task 9, extended by Phase 6 task 6): the six
//! property-bearing benchmark specs (43 sliding window, 44 token ring, 45
//! elevator, 47 HTTP/2 stream, 48 USB control transfer, 49 CAN arbitration)
//! driven through BOTH backends in exact mode - pass-complete statuses,
//! deterministic metrics identical across backends and pinned to the counts
//! recorded in tasks 6/8 (Phase 4) and Phase 6 task 6, pinned FAIL variants
//! (safety, liveness, assumption relaxation - shared in
//! `tests/common/fail_variants.rs`) with counterexample parity and
//! ce_steps pins, in-process parse + codegen timing (informational), and a
//! JSON report (`target/mc_benchmarks.json`) plus per-backend summary tables.
//!
//! Exact mode is seedless and deterministic (D-P4-7), so the pinned values are
//! exact engine outputs re-verified from both backends - not hand-computed.
//! Wall-clock times, parse/codegen timings and peak_rss_kb appear in the
//! report and tables as INFORMATIONAL data only: they are never asserted on
//! (the suite runs inside `cargo test` and must not become a flaky timing or
//! machine-size gate; task 10 consumes the data for tuning). The single
//! exception is an order-of-magnitude sanity bound on peak_rss_kb.
//!
//! The JSON report is a development artifact: written under `target/`, never
//! checked in, and asserted on by parsed fields only (never byte-for-byte).
//!
//! The pinned FAIL variants live in `tests/common/fail_variants.rs` (shared
//! with `info_format_test`'s ce-trace walk - Phase 7 task 7), so each variant
//! text has exactly one home.

mod common;

use common::fail_variants::*;
use rfs::json_rw::Json;

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};
use std::time::Instant;

/// gcc location (plan assumption 5, verified since Phase 2).
const GCC: &str = "/usr/bin/gcc";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

// ---------------------------------------------------------------------------
// Benchmark specs + pinned metrics (recorded in tasks 6 and 8)
// ---------------------------------------------------------------------------

/// The benchmark set: one spec per protocol family, each carrying safety +
/// liveness properties and abstraction-stress payload fields. 43/44/45 are
/// the Phase 4 set; 47/48/49 were added in Phase 6 task 6 (v1.0 versioned).
const BENCH_SPECS: &[&str] = &[
    "43_sliding_window.rfs",
    "44_token_ring.rfs",
    "45_elevator.rfs",
    "47_http2_stream.rfs",
    "48_usb_control_transfer.rfs",
    "49_can_arbitration.rfs",
];

/// Pinned exact-mode metrics per benchmark spec (deterministic; recorded in
/// tasks 6 and 8 - the parity suite pins the same values for both backends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Pin {
    states: usize,
    transitions: usize,
    max_depth: usize,
    /// Encoded bit width of all property + assumption monitors (task 10
    /// statistics polish; deterministic - part of CONFIG_TOTAL_BYTES).
    monitors_bits: usize,
}

fn pin_for(name: &str) -> Pin {
    // Re-pinned after the config-key fix (obligation `remaining` packed only
    // while pending - ref_mc's mon_key_node semantics): these obligation-
    // bearing specs now count exactly ref_mc's configurations.
    match name {
        "43_sliding_window.rfs" => Pin {
            states: 94,
            transitions: 188,
            max_depth: 22,
            monitors_bits: 4,
        },
        "44_token_ring.rfs" => Pin {
            states: 26_113,
            transitions: 104_452,
            max_depth: 268,
            monitors_bits: 10,
        },
        "45_elevator.rfs" => Pin {
            states: 8_082,
            transitions: 72_738,
            max_depth: 22,
            monitors_bits: 4,
        },
        // Phase 6 task 6 (action 1): measured on both backends at pinning time.
        "47_http2_stream.rfs" => Pin {
            states: 434,
            transitions: 21_857,
            max_depth: 7,
            monitors_bits: 10,
        },
        "48_usb_control_transfer.rfs" => Pin {
            states: 104,
            transitions: 340,
            max_depth: 15,
            monitors_bits: 11,
        },
        "49_can_arbitration.rfs" => Pin {
            states: 240,
            transitions: 448,
            max_depth: 41,
            monitors_bits: 7,
        },
        other => panic!("no pin for benchmark spec {other}"),
    }
}

// ---------------------------------------------------------------------------
// Generation + compilation (cached: the suite's tests share the MC binaries)
// ---------------------------------------------------------------------------

static BIN_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

/// Compile-and-cache under one lock so each key is built exactly once even
/// when test threads race for it.
fn cached_binary(key: &str, make: impl FnOnce(&Path) -> PathBuf) -> PathBuf {
    let mut cache = BIN_CACHE.lock().unwrap();
    if let Some(p) = cache.get(key) {
        return p.clone();
    }
    let dir = root().join("target").join("rfs_benchmarks");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = make(&dir);
    cache.insert(key.to_string(), path.clone());
    path
}

fn generate_from_source(backend_name: &str, src: &str, what: &str) -> String {
    let ir = rfs::parse_source(src)
        .unwrap_or_else(|e| panic!("{what} must parse ({backend_name}): {e}"));
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..rfs::codegen::BackendOptions::default()
    };
    rfs::codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir, &options)
        .unwrap_or_else(|e| panic!("codegen {backend_name} for {what} must succeed: {e:?}"))
        .code
}

/// The benchmark suite is a performance artifact as well as a correctness
/// one: Rust MC binaries are built with `-O` so the (informational) wall-clock
/// data matches task 8's sizing methodology.
fn compile_rust(src: &Path, bin: &Path) {
    let out = Command::new(rustc())
        .args(["--edition", "2021", "-O"])
        .arg(src)
        .arg("-o")
        .arg(bin)
        .output()
        .expect("run rustc on generated code");
    assert!(
        out.status.success(),
        "rustc failed on {}:\n{}",
        src.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn compile_c(src: &Path, bin: &Path) {
    let out = Command::new(GCC)
        .arg("-Wall")
        .arg(src)
        .arg("-o")
        .arg(bin)
        .output()
        .expect("run gcc on generated code");
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        out.status.success(),
        "gcc failed on {}:\n{stderr}",
        src.display()
    );
    // Hard bar (task 7 exit criterion): generated C compiles with `gcc -Wall`
    // without warnings.
    assert!(
        !stderr.lines().any(|l| l.contains("warning:")),
        "gcc -Wall emitted warnings on {} (generated C must be warning-free):\n{stderr}",
        src.display()
    );
}

fn bench_binary(stem: &str, backend_name: &str, source: &str) -> PathBuf {
    cached_binary(&format!("bench|{stem}|{backend_name}"), |dir| {
        let ext = if backend_name == "c" { "c" } else { "rs" };
        let src = dir.join(format!("{stem}_mc.{ext}"));
        std::fs::write(&src, generate_from_source(backend_name, source, stem))
            .expect("write MC source");
        let bin = dir.join(format!("{stem}_mc_{backend_name}"));
        if backend_name == "c" {
            compile_c(&src, &bin);
        } else {
            compile_rust(&src, &bin);
        }
        bin
    })
}

// ---------------------------------------------------------------------------
// Run + parse (identical wire format in both backends)
// ---------------------------------------------------------------------------

#[derive(Debug, Clone, PartialEq, Eq)]
struct BenchRun {
    exit_code: i32,
    /// `status=` field of the machine line.
    status: String,
    states: usize,
    transitions: usize,
    max_depth: usize,
    unexpected_terminations: Option<usize>,
    /// `monitor size:` line - encoded bit width of all monitors (task 10).
    monitors_bits: Option<usize>,
    memory_estimate_bytes: Option<usize>,
    /// Fail suffix fields (None on pass/unknown).
    reason: Option<String>,
    label: Option<String>,
    step: Option<usize>,
    /// Human-line counterexample fields.
    trigger_step: Option<usize>,
    violation_step: Option<usize>,
    deadlock_depth: Option<usize>,
    cycle_length: Option<usize>,
    /// Wall-clock of the run in ms. INFORMATIONAL ONLY - never asserted on.
    wall_ms: u128,
}

/// The deterministic projection (wall clock zeroed) - what parity compares.
fn det(run: &BenchRun) -> BenchRun {
    let mut r = run.clone();
    r.wall_ms = 0;
    r
}

/// One exact-mode run with explicit runtime args, keeping the raw stdout for
/// byte-identity comparisons (task 10B threads gate).
fn run_args_full(bin: &Path, args: &[&str]) -> (BenchRun, String) {
    // Exact mode is the default (D-P4-7): seedless, deterministic traversal.
    let start = Instant::now();
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run MC binary {:?}: {e}", bin));
    let wall_ms = start.elapsed().as_millis();
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    (
        parse_run(&stdout, out.status.code().unwrap_or(-1), wall_ms),
        stdout,
    )
}

fn run_parsed(bin: &Path) -> BenchRun {
    run_args_full(bin, &[]).0
}

/// Parse one exact-mode run's stdout into its deterministic fields.
fn parse_run(stdout: &str, exit_code: i32, wall_ms: u128) -> BenchRun {
    let mut status = String::new();
    let mut states = 0usize;
    let mut transitions = 0usize;
    let mut max_depth = 0usize;
    let mut reason = None;
    let mut label = None;
    let mut step = None;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("model-check summary: ") {
            for tok in rest.split_whitespace() {
                if let Some((k, v)) = tok.split_once('=') {
                    match k {
                        "status" => status = v.to_string(),
                        "states" => states = v.parse().unwrap_or(0),
                        "transitions" => transitions = v.parse().unwrap_or(0),
                        "max_depth" => max_depth = v.parse().unwrap_or(0),
                        "reason" => reason = Some(v.to_string()),
                        "label" => label = Some(v.to_string()),
                        "step" => step = Some(v.parse().unwrap_or(0)),
                        _ => {}
                    }
                }
            }
        }
    }
    let mut unexpected_terminations = None;
    let mut monitors_bits = None;
    let mut memory_estimate_bytes = None;
    let mut trigger_step = None;
    let mut violation_step = None;
    let mut deadlock_depth = None;
    let mut cycle_length = None;
    for line in stdout.lines() {
        let t = line.trim_start();
        if let Some(n) = t.strip_prefix("unexpected terminations: ") {
            unexpected_terminations = n.parse().ok();
        } else if let Some(rest) = t.strip_prefix("monitor size: ") {
            monitors_bits = rest.split_whitespace().next().and_then(|d| d.parse().ok());
        } else if let Some(rest) = t.strip_prefix("memory estimate: ") {
            memory_estimate_bytes = rest.split_whitespace().next().and_then(|d| d.parse().ok());
        } else if let Some(n) = t.strip_prefix("trigger step: ") {
            trigger_step = n.parse().ok();
        } else if let Some(n) = t.strip_prefix("violation step: ") {
            violation_step = n.parse().ok();
        } else if let Some(rest) = t.strip_prefix("deadlock: configuration at depth ") {
            deadlock_depth = rest.split_whitespace().next().and_then(|d| d.parse().ok());
        } else if let Some(n) = t.strip_prefix("cycle length: ") {
            cycle_length = n.parse().ok();
        }
    }
    BenchRun {
        exit_code,
        status,
        states,
        transitions,
        max_depth,
        unexpected_terminations,
        monitors_bits,
        memory_estimate_bytes,
        reason,
        label,
        step,
        trigger_step,
        violation_step,
        deadlock_depth,
        cycle_length,
        wall_ms,
    }
}

/// One `--counterexample-format json` run: stdout is the single compact JSON
/// document (D-P5-10 shape - same data as the text report, key order is the
/// cross-backend byte-parity contract).
fn run_json_mode(bin: &Path, extra_args: &[&str]) -> Json {
    let mut args: Vec<&str> = extra_args.to_vec();
    args.push("--counterexample-format");
    args.push("json");
    let out = Command::new(bin)
        .args(&args)
        .output()
        .unwrap_or_else(|e| panic!("run MC binary {:?}: {e}", bin));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    rfs::json_rw::parse(&stdout)
        .unwrap_or_else(|e| panic!("MC JSON report from {:?} must parse: {e}\n{stdout}", bin))
}

/// `statistics.peak_rss_kb` (always present; 0 on read failure - informational).
fn json_peak_rss_kb(doc: &Json) -> i64 {
    doc.get("statistics")
        .and_then(|s| s.get("peak_rss_kb"))
        .and_then(Json::as_int)
        .expect("peak_rss_kb in statistics")
}

/// `counterexample.ce_steps` (present iff the run failed).
fn json_ce_steps(doc: &Json) -> Option<i64> {
    doc.get("counterexample")
        .and_then(|c| c.get("ce_steps"))
        .and_then(Json::as_int)
}

// ---------------------------------------------------------------------------
// Action 1: pass-complete + deterministic metric parity on the benchmark set
// ---------------------------------------------------------------------------

#[test]
fn bench_mc_pass_complete_metrics_parity() {
    for name in BENCH_SPECS {
        let src = spec_source(name);
        let stem = name.strip_suffix(".rfs").expect("spec name");
        let re = run_parsed(&bench_binary(stem, "rust", &src));
        let ce = run_parsed(&bench_binary(stem, "c", &src));

        // Expected status: pass-complete in BOTH backends.
        assert_eq!(re.status, "pass-complete", "[{name}] rust backend");
        assert_eq!(ce.status, "pass-complete", "[{name}] c backend");
        assert_eq!(re.exit_code, 0, "[{name}] rust exit code");
        assert_eq!(ce.exit_code, 0, "[{name}] c exit code");

        // Sanity bounds: the exact counts recorded in tasks 6/8 (deterministic
        // exact mode - equality IS the bound), including the task 10 monitor
        // size stat.
        let pin = pin_for(name);
        for (be, r) in [("rust", &re), ("c", &ce)] {
            assert_eq!(
                (r.states, r.transitions, r.max_depth),
                (pin.states, pin.transitions, pin.max_depth),
                "[{name}] {be}: metrics drifted from the task 6/8 record"
            );
            assert_eq!(
                r.monitors_bits,
                Some(pin.monitors_bits),
                "[{name}] {be}: monitor size drifted from the task 10 record"
            );
        }

        // Deterministic metrics IDENTICAL across backends (every parsed field
        // except the informational wall clock).
        assert_eq!(
            det(&re),
            det(&ce),
            "[{name}] exact-mode results must be field-identical across backends"
        );
    }
}

// ---------------------------------------------------------------------------
// Action 2: inline FAIL variants - status/reason/label + counterexample parity
// ---------------------------------------------------------------------------

/// Pinned FAIL expectation (deterministic; re-verified from both backends).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct FailPin {
    reason: &'static str,
    label: &'static str,
    step: usize,
    states: usize,
    transitions: usize,
    max_depth: usize,
    violation_step: Option<usize>,
    cycle_length: Option<usize>,
    /// Counterexample length in steps (JSON report `counterexample.ce_steps`,
    /// Phase 6 task 6): deterministic, part of the parity contract.
    ce_steps: usize,
}

#[test]
fn bench_mc_fail_variants_both_backends() {
    // (stem, source, extra runtime args, pin). Phase 4: ring grant arm removed
    // (safety), elevator Idle go-up arm removed (liveness). Phase 6 task 6:
    // one-line-delta mutations of the new specs + assumption relaxation.
    let cases: Vec<(&str, String, &[&str], FailPin)> = vec![
        (
            "ring_no_grant",
            RING_NO_GRANT_SRC.to_string(),
            &[],
            FailPin {
                reason: "property-violation",
                label: "holder_no_request",
                step: 1,
                states: 1,
                transitions: 1,
                max_depth: 0,
                violation_step: Some(1),
                cycle_length: None,
                ce_steps: 1,
            },
        ),
        (
            "elev_noup",
            ELEV_NO_UP_SRC.to_string(),
            &[],
            // Re-pinned with the config-key fix (see pin_for).
            FailPin {
                reason: "liveness-violation",
                label: "up2_served",
                step: 0,
                states: 80,
                transitions: 720,
                max_depth: 6,
                violation_step: Some(0),
                // The canonical lasso cycle is shorter in the corrected (smaller)
                // config space.
                cycle_length: Some(2),
                ce_steps: 2,
            },
        ),
        // Safety variant: Open RstStream arm retargeted off Closed.
        (
            "http2_rst_fail",
            http2_rst_fail_src(),
            &[],
            FailPin {
                reason: "property-violation",
                label: "rst_goes_closed",
                step: 2,
                states: 34,
                transitions: 64,
                max_depth: 2,
                violation_step: Some(2),
                cycle_length: None,
                ce_steps: 2,
            },
        ),
        // Assumption relaxation: env_progress off, so the Error state becomes
        // reachable; the environment starves a partial transfer (Data(1) x3,
        // then Nak x4 into Error), where it can never complete - the bounded
        // obligation's countdown expires at step 16. Runs the UNMODIFIED spec
        // (stem = spec name reuses the pass run's binary).
        (
            "48_usb_control_transfer",
            spec_source("48_usb_control_transfer.rfs"),
            &["--assumes", "none"],
            FailPin {
                reason: "property-violation",
                label: "completes_after_setup",
                step: 16,
                states: 262,
                transitions: 1464,
                max_depth: 15,
                violation_step: Some(16),
                cycle_length: None,
                ce_steps: 16,
            },
        ),
        // Safety variant: passive-band entry counter update weakened (+1).
        (
            "can_passive_fail",
            can_passive_fail_src(),
            &[],
            FailPin {
                reason: "property-violation",
                label: "passive_band",
                step: 6,
                states: 15,
                transitions: 17,
                max_depth: 6,
                violation_step: Some(6),
                cycle_length: None,
                ce_steps: 6,
            },
        ),
    ];

    let mut table = String::from(
        "FAIL variants (exact mode) - pinned label/step/ce_steps, field-identical across backends\n",
    );
    for (stem, src, args, pin) in &cases {
        let re = run_args_full(&bench_binary(stem, "rust", src), args).0;
        let ce = run_args_full(&bench_binary(stem, "c", src), args).0;

        for (be, r) in [("rust", &re), ("c", &ce)] {
            assert_eq!(r.exit_code, 1, "[{stem}] {be}: failing MC exits 1");
            assert_eq!(r.status, "fail", "[{stem}] {be}");
            assert_eq!(r.reason.as_deref(), Some(pin.reason), "[{stem}] {be}");
            assert_eq!(r.label.as_deref(), Some(pin.label), "[{stem}] {be}");
            assert_eq!(r.step, Some(pin.step), "[{stem}] {be}");
            assert_eq!(
                (r.states, r.transitions, r.max_depth),
                (pin.states, pin.transitions, pin.max_depth),
                "[{stem}] {be}: fail metrics drifted from the record"
            );
            assert_eq!(r.violation_step, pin.violation_step, "[{stem}] {be}");
            assert_eq!(r.cycle_length, pin.cycle_length, "[{stem}] {be}");
        }

        // ce_steps pin from the JSON report (Phase 6 task 6): deterministic,
        // so it is asserted - and equal on both backends via the shared pin.
        let jr = run_json_mode(&bench_binary(stem, "rust", src), args);
        let jc = run_json_mode(&bench_binary(stem, "c", src), args);
        for (be, doc) in [("rust", &jr), ("c", &jc)] {
            assert_eq!(
                json_ce_steps(doc),
                Some(pin.ce_steps as i64),
                "[{stem}] {be}: ce_steps drifted from the record"
            );
            assert_eq!(
                doc.get("label").and_then(Json::as_str),
                Some(pin.label),
                "[{stem}] {be}: JSON label mismatch"
            );
        }

        // Counterexample fields (and all other deterministic fields) identical
        // across backends - the lasso for the liveness variant included.
        assert_eq!(
            det(&re),
            det(&ce),
            "[{stem}] FAIL counterexample must be field-identical across backends"
        );

        for (be, r) in [("rust", &re), ("c", &ce)] {
            table.push_str(&format!(
                "{:<24} {:<8} {:<18} {:<22} {:>5} {:>8} {:>9}\n",
                stem,
                be,
                r.reason.as_deref().unwrap_or("-"),
                r.label.as_deref().unwrap_or("-"),
                r.step.unwrap_or(0),
                pin.ce_steps,
                r.states
            ));
        }
    }
    eprintln!("{table}");
}

// ---------------------------------------------------------------------------
// Action 3: JSON report (target/mc_benchmarks.json) + per-backend table
// ---------------------------------------------------------------------------

/// The timed set (Phase 6 task 6): the six benchmark specs plus the two
/// classic small examples as a parse/codegen throughput reference.
const TIMED_SPECS: &[&str] = &[
    "03_timeout_retry.rfs",
    "04_tcp_handshake_like.rfs",
    "43_sliding_window.rfs",
    "44_token_ring.rfs",
    "45_elevator.rfs",
    "47_http2_stream.rfs",
    "48_usb_control_transfer.rfs",
    "49_can_arbitration.rfs",
];

/// In-process parse + codegen wall clocks in ms. INFORMATIONAL ONLY - never
/// asserted on (CI machines vary; the data feeds task 10's tuning).
#[derive(Debug, Clone, Copy)]
struct CodegenTimes {
    parse_ms: u128,
    codegen_ms: u128,
}

fn time_parse_codegen(backend_name: &str, name: &str) -> CodegenTimes {
    let src = spec_source(name);
    let t0 = Instant::now();
    let ir =
        rfs::parse_source(&src).unwrap_or_else(|e| panic!("timed spec {name} must parse: {e}"));
    let parse_ms = t0.elapsed().as_millis();
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..rfs::codegen::BackendOptions::default()
    };
    let t1 = Instant::now();
    rfs::codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir, &options)
        .unwrap_or_else(|e| panic!("timed codegen {backend_name} for {name}: {e:?}"));
    let codegen_ms = t1.elapsed().as_millis();
    CodegenTimes {
        parse_ms,
        codegen_ms,
    }
}

fn codegen_times_json(t: &CodegenTimes) -> Json {
    Json::obj(vec![
        ("parse_ms".into(), Json::int(t.parse_ms as i64)),
        ("codegen_ms".into(), Json::int(t.codegen_ms as i64)),
    ])
}

/// One benchmark spec's results per backend: the text-mode run plus the
/// JSON-report document (keeps the entries tuple below clippy's complexity bar).
type SpecRuns = (HashMap<&'static str, BenchRun>, HashMap<&'static str, Json>);

fn run_json(r: &BenchRun, peak_rss_kb: i64) -> Json {
    let opt = |v: Option<usize>| match v {
        Some(n) => Json::int(n as i64),
        None => Json::Null,
    };
    Json::obj(vec![
        ("status".into(), Json::Str(r.status.clone())),
        ("states".into(), Json::int(r.states as i64)),
        ("transitions".into(), Json::int(r.transitions as i64)),
        ("max_depth".into(), Json::int(r.max_depth as i64)),
        (
            "unexpected_terminations".into(),
            opt(r.unexpected_terminations),
        ),
        ("monitors_bits".into(), opt(r.monitors_bits)),
        ("memory_estimate_bytes".into(), opt(r.memory_estimate_bytes)),
        // Peak RSS in kB from the JSON-report run (informational; 0 on read
        // failure - see codegen_mc.rs). The order-of-magnitude sanity bound
        // is asserted at the call site, not here.
        ("peak_rss_kb".into(), Json::int(peak_rss_kb)),
        // u128 wall-clock ms; i64 covers 292 million years of runtime.
        ("wall_ms".into(), Json::int(r.wall_ms as i64)),
    ])
}

#[test]
fn bench_mc_json_report_and_table() {
    // Per spec x backend: the text-mode run (parsed fields) plus the JSON-
    // report-mode run (peak_rss_kb; same data as the text report).
    let mut entries: Vec<(String, SpecRuns)> = Vec::new();
    for name in BENCH_SPECS {
        let src = spec_source(name);
        let stem = name.strip_suffix(".rfs").expect("spec name");
        let rb = bench_binary(stem, "rust", &src);
        let cb = bench_binary(stem, "c", &src);
        let runs: HashMap<&str, BenchRun> = [("rust", run_parsed(&rb)), ("c", run_parsed(&cb))]
            .into_iter()
            .collect();
        let docs: HashMap<&str, Json> = [
            ("rust", run_json_mode(&rb, &[])),
            ("c", run_json_mode(&cb, &[])),
        ]
        .into_iter()
        .collect();

        for be in ["rust", "c"] {
            // peak_rss_kb: INFORMATIONAL, with the single order-of-magnitude
            // sanity bound allowed by the Phase 4 discipline (1 GB ceiling -
            // the largest benchmark here explores ~26k states).
            let rss = json_peak_rss_kb(&docs[be]);
            assert!(
                (0..1_000_000).contains(&rss),
                "[{name}] {be}: peak_rss_kb {rss} outside the sanity bound"
            );
            // Text and JSON modes carry the SAME data (D-P5-10): cross-check
            // the deterministic fields between the two renderings.
            let doc = &docs[be];
            assert_eq!(
                doc.get("status").and_then(Json::as_str),
                Some(runs[be].status.as_str()),
                "[{name}] {be}: JSON/text status mismatch"
            );
            for (key, want) in [
                ("states", runs[be].states as i64),
                ("transitions", runs[be].transitions as i64),
                ("max_depth", runs[be].max_depth as i64),
            ] {
                assert_eq!(
                    doc.get("metrics")
                        .and_then(|m| m.get(key))
                        .and_then(Json::as_int),
                    Some(want),
                    "[{name}] {be}: JSON/text metrics mismatch ({key})"
                );
            }
        }

        entries.push((name.to_string(), (runs, docs)));
    }

    // Parse + codegen timing: in-process lib calls over the timed set.
    // INFORMATIONAL ONLY - never asserted on.
    let timed: Vec<(&str, CodegenTimes, CodegenTimes)> = TIMED_SPECS
        .iter()
        .map(|&name| {
            (
                name,
                time_parse_codegen("rust", name),
                time_parse_codegen("c", name),
            )
        })
        .collect();
    let timing_json = timed
        .iter()
        .map(|(name, rt, ct)| {
            Json::obj(vec![
                ("spec".into(), Json::Str(name.to_string())),
                (
                    "backends".into(),
                    Json::obj(vec![
                        ("rust".into(), codegen_times_json(rt)),
                        ("c".into(), codegen_times_json(ct)),
                    ]),
                ),
            ])
        })
        .collect::<Vec<_>>();

    // Report: per spec x backend - status, deterministic metrics, peak_rss_kb
    // (informational), wall-clock (informational) - plus the codegen timing
    // section. Written under target/ - a development artifact.
    let specs = entries
        .iter()
        .map(|(name, (runs, docs))| {
            let backends = Json::obj(vec![
                (
                    "rust".into(),
                    run_json(&runs["rust"], json_peak_rss_kb(&docs["rust"])),
                ),
                (
                    "c".into(),
                    run_json(&runs["c"], json_peak_rss_kb(&docs["c"])),
                ),
            ]);
            Json::obj(vec![
                ("spec".into(), Json::Str(name.clone())),
                ("backends".into(), backends),
            ])
        })
        .collect::<Vec<_>>();
    let report = Json::obj(vec![
        ("suite".into(), Json::Str("benchmark_mc".into())),
        ("mode".into(), Json::Str("exact".into())),
        ("specs".into(), Json::arr(specs)),
        ("codegen_timing".into(), Json::arr(timing_json)),
    ]);
    let path = root().join("target").join("mc_benchmarks.json");
    std::fs::write(&path, report.to_string_pretty())
        .unwrap_or_else(|e| panic!("write {:?}: {e}", path));

    // Assert on PARSED fields (never byte-for-byte): read the file back and
    // check status + deterministic metrics match what the runs produced.
    let on_disk = rfs::json_rw::parse(&std::fs::read_to_string(&path).expect("read report"))
        .expect("report must be valid JSON");
    for (name, (runs, docs)) in &entries {
        for be in ["rust", "c"] {
            let r = &runs[be];
            let spec_entry = on_disk
                .get("specs")
                .and_then(Json::as_arr)
                .and_then(|a| {
                    a.iter()
                        .find(|s| s.get("spec").and_then(Json::as_str) == Some(name.as_str()))
                })
                .expect("report entry for spec");
            let j = spec_entry
                .get("backends")
                .and_then(|b| b.get(be))
                .expect("backend entry in report");
            assert_eq!(
                j.get("status").and_then(Json::as_str),
                Some(r.status.as_str()),
                "[{name}] {be} status in report"
            );
            assert_eq!(
                j.get("states").and_then(Json::as_int),
                Some(r.states as i64),
                "[{name}] {be} states in report"
            );
            assert_eq!(
                j.get("transitions").and_then(Json::as_int),
                Some(r.transitions as i64),
                "[{name}] {be} transitions in report"
            );
            assert_eq!(
                j.get("max_depth").and_then(Json::as_int),
                Some(r.max_depth as i64),
                "[{name}] {be} max_depth in report"
            );
            assert_eq!(
                j.get("peak_rss_kb").and_then(Json::as_int),
                Some(json_peak_rss_kb(&docs[be])),
                "[{name}] {be} peak_rss_kb in report (info only)"
            );
            assert!(
                j.get("wall_ms").is_some(),
                "[{name}] {be} wall_ms present (info only)"
            );
        }
    }

    // codegen_timing section: one entry per timed spec, both backends, both
    // fields present as non-negative ints (informational - values not pinned).
    let timing_entries = on_disk
        .get("codegen_timing")
        .and_then(Json::as_arr)
        .expect("report carries codegen_timing");
    assert_eq!(
        timing_entries.len(),
        TIMED_SPECS.len(),
        "codegen_timing entry count"
    );
    for e in timing_entries {
        let spec = e
            .get("spec")
            .and_then(Json::as_str)
            .expect("timing entry spec name");
        assert!(TIMED_SPECS.contains(&spec), "unknown timed spec {spec}");
        for be in ["rust", "c"] {
            for key in ["parse_ms", "codegen_ms"] {
                let v = e
                    .get("backends")
                    .and_then(|b| b.get(be))
                    .and_then(|t| t.get(key))
                    .and_then(Json::as_int);
                assert!(
                    v.is_some_and(|x| x >= 0),
                    "[{spec}] {be} {key} present (info only)"
                );
            }
        }
    }

    // Per-backend summary table to the test output (wall clock + peak RSS are
    // informational).
    let mut table = String::from(
        "benchmark MC suite (exact mode) - deterministic metrics identical across backends\n",
    );
    table.push_str(&format!(
        "{:<24} {:<8} {:<14} {:>9} {:>13} {:>9} {:>16} {:>18}\n",
        "spec",
        "backend",
        "status",
        "states",
        "transitions",
        "max_depth",
        "wall_ms (info)",
        "peak_rss_kb (info)"
    ));
    for (name, (runs, docs)) in &entries {
        for be in ["rust", "c"] {
            let r = &runs[be];
            table.push_str(&format!(
                "{:<24} {:<8} {:<14} {:>9} {:>13} {:>9} {:>16} {:>18}\n",
                name,
                be,
                r.status,
                r.states,
                r.transitions,
                r.max_depth,
                r.wall_ms,
                json_peak_rss_kb(&docs[be])
            ));
        }
    }
    eprintln!("{table}");

    // Parse + codegen timing table (in-process; informational - never asserted).
    let mut tt = String::from(
        "parse + codegen timing (in-process lib calls, informational - never asserted)\n",
    );
    tt.push_str(&format!(
        "{:<28} {:<8} {:>9} {:>12}\n",
        "spec", "backend", "parse_ms", "codegen_ms"
    ));
    for (name, rt, ct) in &timed {
        tt.push_str(&format!(
            "{:<28} {:<8} {:>9} {:>12}\n",
            name, "rust", rt.parse_ms, rt.codegen_ms
        ));
        tt.push_str(&format!(
            "{:<28} {:<8} {:>9} {:>12}\n",
            name, "c", ct.parse_ms, ct.codegen_ms
        ));
    }
    eprintln!("{tt}");
}

// ---------------------------------------------------------------------------
// Task 10B action 3: --threads parity gate (byte-identity across N and
// backends) + --fast smoke test (soundness only - non-parity mode).
// ---------------------------------------------------------------------------

/// All eight workloads of the threads gate: the six benchmark specs plus all
/// five inline FAIL variants (safety + liveness counterexamples included; the
/// 48 assumption-relaxation run reuses the spec's own binary and carries its
/// `--assumes none` args through the gate).
fn threads_gate_workloads() -> Vec<(&'static str, String, &'static [&'static str])> {
    let mut out: Vec<(&'static str, String, &'static [&'static str])> = BENCH_SPECS
        .iter()
        .map(|name| {
            (
                name.strip_suffix(".rfs").expect("spec name"),
                spec_source(name),
                &[] as &[&str],
            )
        })
        .collect();
    out.push(("ring_no_grant", RING_NO_GRANT_SRC.to_string(), &[]));
    out.push(("elev_noup", ELEV_NO_UP_SRC.to_string(), &[]));
    out.push(("http2_rst_fail", http2_rst_fail_src(), &[]));
    out.push((
        "48_usb_control_transfer",
        spec_source("48_usb_control_transfer.rfs"),
        &["--assumes", "none"],
    ));
    out.push(("can_passive_fail", can_passive_fail_src(), &[]));
    out
}

#[test]
fn bench_mc_threads_parity() {
    // Determinism by construction (task 10B): canonical commit makes the
    // result a pure function of the spec, so EVERY run - any thread count,
    // either backend - must be byte-identical to rust --threads 1. This is
    // stronger than the field-level parity above: counterexample traces and
    // every printed line included. No pin changes are expected; a broken pin
    // here means a bug in the parallel path, not a re-pin.
    for (stem, src, extra) in threads_gate_workloads() {
        let mut ref_args: Vec<&str> = extra.to_vec();
        ref_args.push("--threads");
        ref_args.push("1");
        let ref_run = run_args_full(&bench_binary(stem, "rust", &src), &ref_args);
        for be in ["rust", "c"] {
            let bin = bench_binary(stem, be, &src);
            for n in ["2", "4"] {
                let mut args: Vec<&str> = extra.to_vec();
                args.push("--threads");
                args.push(n);
                let r = run_args_full(&bin, &args);
                assert_eq!(
                    (r.0.exit_code, r.1.as_str()),
                    (ref_run.0.exit_code, ref_run.1.as_str()),
                    "[{stem}] {be} --threads {n} must be byte-identical to rust --threads 1\n[ref]\n{}\n[got]\n{}",
                    ref_run.1, r.1
                );
            }
        }
    }
}

#[test]
fn bench_mc_fast_smoke() {
    // --fast is documented NON-PARITY (first found violation wins; which
    // configs were expanded before the abort is race-dependent). Assert only
    // the soundness contract: exit 1, status=fail, and a label that is
    // genuinely violated on the variant. The current labels are in fact
    // guaranteed: ring_no_grant has exactly one possible online violation
    // (holder_no_request - liveness is only detected after full BFS), and
    // elev_noup has none, so --fast never aborts early there and the
    // canonical liveness analysis reports up2_served.
    let cases: [(&str, &str, &str, &str); 2] = [
        (
            "ring_no_grant",
            RING_NO_GRANT_SRC,
            "property-violation",
            "holder_no_request",
        ),
        (
            "elev_noup",
            ELEV_NO_UP_SRC,
            "liveness-violation",
            "up2_served",
        ),
    ];
    for (stem, src, reason, label) in cases {
        for be in ["rust", "c"] {
            let bin = bench_binary(stem, be, src);
            let r = run_args_full(&bin, &["--threads", "4", "--fast"]).0;
            assert_eq!(r.exit_code, 1, "[{stem}] {be} --fast: failing MC exits 1");
            assert_eq!(r.status, "fail", "[{stem}] {be} --fast");
            assert_eq!(
                r.reason.as_deref(),
                Some(reason),
                "[{stem}] {be} --fast reason"
            );
            assert_eq!(
                r.label.as_deref(),
                Some(label),
                "[{stem}] {be} --fast label"
            );
        }
    }
}
