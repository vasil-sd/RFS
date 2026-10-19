//! Phase 4 backend-parity differential suite (task 4, D-P4-14): the backend
//! parity principle made executable - Rust and C model checkers run in EXACT
//! mode on identical specs must produce indistinguishable results, and both
//! backends' generated test executables must agree on every example.
//!
//! Scope: ALL valid examples carrying `prop`/`assume` declarations, enumerated
//! DYNAMICALLY (including `examples/valid/tla_adapted/`) so future additions
//! are covered automatically. Exact mode is seedless and deterministic: the
//! traversal order pins which violation/deadlock is reported first, so the
//! expected VALUES below were re-pinned from actual engine output (task 4,
//! action 2) - they deliberately differ from the Phase 3 fuzz-era pins.
//!
//! Fuzz mode stays covered by the codegen flag-sensitivity tests and task 2's
//! spot check; this suite no longer drives `--mode fuzz`.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};

/// gcc location (plan assumption 5, verified since Phase 2).
const GCC: &str = "/usr/bin/gcc";

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// All valid examples carrying `prop `/`assume ` declarations (dynamic set).
fn property_bearing_examples() -> Vec<PathBuf> {
    let files = rfs::collect_rfs_files(root().join("examples/valid")).expect("collect .rfs files");
    let out: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            let src = std::fs::read_to_string(p).unwrap_or_else(|e| panic!("read {:?}: {e}", p));
            src.lines().any(|l| {
                let t = l.trim_start();
                t.starts_with("prop ") || t.starts_with("assume ")
            })
        })
        .collect();
    assert!(
        !out.is_empty(),
        "dynamic enumeration found no property-bearing examples"
    );
    // Anchor both traversal levels (top-level + tla_adapted subdirectory).
    let names: Vec<String> = out
        .iter()
        .map(|p| p.file_name().unwrap().to_string_lossy().into_owned())
        .collect();
    assert!(
        names.contains(&"41_property_monitors.rfs".to_string()),
        "top-level examples covered: {names:?}"
    );
    assert!(
        names.contains(&"33_german_transfer_tla.rfs".to_string()),
        "tla_adapted subdirectory covered: {names:?}"
    );
    out
}

fn example_name(spec: &Path) -> String {
    spec.file_name().unwrap().to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Generation + compilation (cached: tests 1 and 2 share the MC binaries)
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
    let dir = root().join("target").join("rfs_backend_parity");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = make(&dir);
    cache.insert(key.to_string(), path.clone());
    path
}

fn generate(backend_name: &str, command: rfs::codegen::CodegenCommandKind, spec: &Path) -> String {
    let src = std::fs::read_to_string(spec).unwrap_or_else(|e| panic!("read {:?}: {e}", spec));
    let ir = rfs::parse_source(&src)
        .unwrap_or_else(|e| panic!("{:?} must parse ({backend_name}): {e}", spec));
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..rfs::codegen::BackendOptions::default()
    };
    command
        .run(backend, &ir.modules[0], &options)
        .unwrap_or_else(|e| panic!("codegen {backend_name} {:?} must succeed: {e:?}", spec))
        .code
}

fn compile_rust(src: &Path, bin: &Path, as_test: bool) {
    let mut cmd = Command::new(rustc());
    cmd.args(["--edition", "2021"]);
    if as_test {
        // The `test` command emits a single file with a #[cfg(test)] module;
        // --test builds the harness executable from it.
        cmd.arg("--test");
    }
    let out = cmd
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
    assert!(
        out.status.success(),
        "gcc failed on {}:\n{}",
        src.display(),
        String::from_utf8_lossy(&out.stderr)
    );
}

fn mc_binary(spec: &Path, backend_name: &str) -> PathBuf {
    let stem = spec.file_stem().unwrap().to_string_lossy().into_owned();
    cached_binary(&format!("mc|{stem}|{backend_name}"), |dir| {
        let ext = if backend_name == "c" { "c" } else { "rs" };
        let src = dir.join(format!("{stem}_mc.{ext}"));
        std::fs::write(
            &src,
            generate(
                backend_name,
                rfs::codegen::CodegenCommandKind::ModelChecker,
                spec,
            ),
        )
        .expect("write MC source");
        let bin = dir.join(format!("{stem}_mc_{backend_name}"));
        if backend_name == "c" {
            compile_c(&src, &bin);
        } else {
            compile_rust(&src, &bin, false);
        }
        bin
    })
}

fn test_binary(spec: &Path, backend_name: &str) -> PathBuf {
    let stem = spec.file_stem().unwrap().to_string_lossy().into_owned();
    cached_binary(&format!("test|{stem}|{backend_name}"), |dir| {
        let ext = if backend_name == "c" { "c" } else { "rs" };
        let src = dir.join(format!("{stem}_test.{ext}"));
        std::fs::write(
            &src,
            generate(
                backend_name,
                rfs::codegen::CodegenCommandKind::UnitTests,
                spec,
            ),
        )
        .expect("write test source");
        let bin = dir.join(format!("{stem}_test_{backend_name}"));
        if backend_name == "c" {
            compile_c(&src, &bin);
        } else {
            compile_rust(&src, &bin, true);
        }
        bin
    })
}

// ---------------------------------------------------------------------------
// Run + parse helpers (identical wire format in both backends)
// ---------------------------------------------------------------------------

struct McOutcome {
    exit_code: Option<i32>,
    stdout: String,
}

fn run_mc(spec: &Path, backend_name: &str, extra_args: &[String]) -> McOutcome {
    let bin = mc_binary(spec, backend_name);
    // Exact mode is the default (D-P4-7): seedless, deterministic traversal.
    let mut args: Vec<String> = Vec::new();
    args.extend_from_slice(extra_args);
    let out = Command::new(&bin)
        .args(&args)
        .output()
        .unwrap_or_else(|e| panic!("run MC binary {:?}: {e}", bin));
    McOutcome {
        exit_code: out.status.code(),
        stdout: String::from_utf8_lossy(&out.stdout).to_string(),
    }
}

/// One exact-mode run, fully parsed: status, summary-line fields and the
/// counterexample fields (D-P4-4 wire format; identical in both backends).
#[derive(Debug, Clone, PartialEq, Eq)]
struct McExact {
    exit_code: i32,
    /// `status=` field of the machine line.
    status: String,
    states: usize,
    transitions: usize,
    max_depth: usize,
    /// Fail suffix fields (None on pass/unknown).
    reason: Option<String>,
    label: Option<String>,
    step: Option<usize>,
    /// Human-line counterexample fields.
    trigger_step: Option<usize>,
    violation_step: Option<usize>,
    deadlock_depth: Option<usize>,
}

fn parse_exact(stdout: &str, exit_code: i32) -> McExact {
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
    let mut trigger_step = None;
    let mut violation_step = None;
    let mut deadlock_depth = None;
    for line in stdout.lines() {
        let t = line.trim_start();
        if let Some(n) = t.strip_prefix("trigger step: ") {
            trigger_step = n.parse().ok();
        } else if let Some(n) = t.strip_prefix("violation step: ") {
            violation_step = n.parse().ok();
        } else if let Some(rest) = t.strip_prefix("deadlock: configuration at depth ") {
            deadlock_depth = rest.split_whitespace().next().and_then(|d| d.parse().ok());
        }
    }
    McExact {
        exit_code,
        status,
        states,
        transitions,
        max_depth,
        reason,
        label,
        step,
        trigger_step,
        violation_step,
        deadlock_depth,
    }
}

// ---------------------------------------------------------------------------
// Action 1: exact-mode differential runs over all property-bearing examples
// ---------------------------------------------------------------------------

#[test]
fn mc_exact_runs_parity_all_property_bearing_examples() {
    for spec in property_bearing_examples() {
        let name = example_name(&spec);
        let rust = run_mc(&spec, "rust", &[]);
        let c = run_mc(&spec, "c", &[]);
        assert_eq!(
            rust.exit_code, c.exit_code,
            "[{name}] exit codes must match (same spec, deterministic traversal)"
        );
        let re = parse_exact(&rust.stdout, rust.exit_code.unwrap_or(-1));
        let ce = parse_exact(&c.stdout, c.exit_code.unwrap_or(-1));
        assert_eq!(
            re, ce,
            "[{name}] exact-mode results must be field-identical across backends\n[rust]\n{}\n[c]\n{}",
            rust.stdout, c.stdout
        );
    }
}

// ---------------------------------------------------------------------------
// Action 2: re-pinned exact-mode values (D-P4-14)
// ---------------------------------------------------------------------------

/// One pinned exact-mode expectation per property-bearing example.
#[derive(Debug, Clone, PartialEq, Eq)]
struct Pinned {
    exit_code: i32,
    status: &'static str,
    states: usize,
    transitions: usize,
    max_depth: usize,
    reason: Option<&'static str>,
    label: Option<&'static str>,
    step: Option<usize>,
    trigger_step: Option<usize>,
    violation_step: Option<usize>,
}

fn pinned(name: &str) -> Pinned {
    // Re-pinned from ACTUAL exact-mode output (task 4, action 2). Exact BFS
    // reports the first bad event in deterministic traversal order - deadlocks
    // at terminal states now surface on several examples that the fuzz walk
    // never reported. Old fuzz-era pins: natural violations only on
    // 13_properties_safety (no_bad) and 41_property_monitors
    // (fin_acked_bounded, forced trigger=1/violation=5).
    match name {
        "04_tcp_handshake_like.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 4,
            transitions: 3,
            max_depth: 2,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(2),
            trigger_step: None,
            violation_step: None,
        },
        "13_properties_safety.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 1,
            transitions: 2,
            max_depth: 0,
            reason: Some("property-violation"),
            label: Some("no_bad"),
            step: Some(1),
            trigger_step: None,
            violation_step: Some(1),
        },
        "14_assumptions_tick.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 258,
            transitions: 258,
            max_depth: 257,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "15_bounded_eventually.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 3,
            transitions: 2,
            max_depth: 2,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(2),
            trigger_step: None,
            violation_step: None,
        },
        "16_next_property.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 2,
            transitions: 1,
            max_depth: 1,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(1),
            trigger_step: None,
            violation_step: None,
        },
        "24_event_no_payload_properties.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 1,
            transitions: 1,
            max_depth: 0,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "37_assumption_demo.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 2,
            transitions: 3,
            max_depth: 1,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "41_property_monitors.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 7,
            transitions: 6,
            max_depth: 3,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(3),
            trigger_step: None,
            violation_step: None,
        },
        "42_bounded_liveness_protocol.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 6,
            transitions: 5,
            max_depth: 3,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(2),
            trigger_step: None,
            violation_step: None,
        },
        // First liveness-bearing example (task 6): Rust checks the
        // eventually/until properties on the completed graph; C (until task 7)
        // skips them as out-of-subset. Both backends agree on status and the
        // deterministic BFS metrics because liveness adds no monitor state.
        "43_sliding_window.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 165,
            transitions: 330,
            max_depth: 22,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "31_traffic_light_tla.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 3,
            transitions: 3,
            max_depth: 2,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "32_tcp_endpoint_tla.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 11,
            transitions: 14,
            max_depth: 3,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(3),
            trigger_step: None,
            violation_step: None,
        },
        "33_german_transfer_tla.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 4,
            transitions: 3,
            max_depth: 2,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(2),
            trigger_step: None,
            violation_step: None,
        },
        "34_single_lane_bridge_tla.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 3,
            transitions: 4,
            max_depth: 1,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "35_bounded_buffer_tla.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 3,
            transitions: 4,
            max_depth: 2,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "36_echo_node_tla.rfs" => Pinned {
            exit_code: 1,
            status: "fail",
            states: 5,
            transitions: 4,
            max_depth: 2,
            reason: Some("deadlock"),
            label: Some("-"),
            step: Some(2),
            trigger_step: None,
            violation_step: None,
        },
        other => panic!(
            "no pinned exact-mode expectation for {other} - add one (re-pin from actual output)"
        ),
    }
}

#[test]
fn mc_exact_results_match_re_pinned_values() {
    let mut n_pass = 0usize;
    let mut n_fail = 0usize;
    for spec in property_bearing_examples() {
        let name = example_name(&spec);
        let rust = run_mc(&spec, "rust", &[]);
        let c = run_mc(&spec, "c", &[]);
        let pin = pinned(&name);
        for (backend, out) in [("rust", &rust), ("c", &c)] {
            assert_eq!(
                out.exit_code,
                Some(pin.exit_code),
                "[{backend} {name}] exit code drifted from the re-pinned value; stdout:\n{}",
                out.stdout
            );
            let e = parse_exact(&out.stdout, pin.exit_code);
            assert_eq!(e.status, pin.status, "[{backend} {name}] status drifted");
            assert_eq!(
                (e.states, e.transitions, e.max_depth),
                (pin.states, pin.transitions, pin.max_depth),
                "[{backend} {name}] summary fields drifted; stdout:\n{}",
                out.stdout
            );
            assert_eq!(
                (e.reason.as_deref(), e.label.as_deref(), e.step),
                (pin.reason, pin.label, pin.step),
                "[{backend} {name}] fail-suffix fields drifted; stdout:\n{}",
                out.stdout
            );
            assert_eq!(
                (e.trigger_step, e.violation_step),
                (pin.trigger_step, pin.violation_step),
                "[{backend} {name}] counterexample fields drifted; stdout:\n{}",
                out.stdout
            );
        }
        if pin.status == "pass-complete" {
            n_pass += 1;
        } else {
            n_fail += 1;
        }
    }
    // Pinned current state: 7 pass-complete, 9 fail (1 property violation on
    // 13_properties_safety, 8 deadlocks at terminal states). Example 43 is the
    // first liveness-bearing spec (task 6): pass-complete in both backends.
    assert_eq!((n_pass, n_fail), (7, 9), "pass/fail split drifted");
}

// ---------------------------------------------------------------------------
// Action 3: generated test executables both pass (identical results)
// ---------------------------------------------------------------------------

#[test]
fn generated_test_executables_parity_all_property_bearing_examples() {
    let mut failing: Vec<String> = Vec::new();
    for spec in property_bearing_examples() {
        let name = example_name(&spec);
        let rbin = test_binary(&spec, "rust");
        let cbin = test_binary(&spec, "c");
        let r = Command::new(&rbin)
            .output()
            .unwrap_or_else(|e| panic!("run rust test binary: {e}"));
        let c = Command::new(&cbin)
            .output()
            .unwrap_or_else(|e| panic!("run c test binary: {e}"));
        assert_eq!(
            r.status.success(),
            c.status.success(),
            "[{name}] pass/fail must match across backends (rust stdout: {}, c stdout: {})",
            String::from_utf8_lossy(&r.stdout),
            String::from_utf8_lossy(&c.stdout)
        );
        if !r.status.success() {
            failing.push(name);
        }
    }
    // Pinned current state: no example fails its own generated test in either
    // backend. (Task 24 resolved the former 13_properties_safety failure -
    // see PHASE_3_TASK_24_STATUS.md.)
    assert!(failing.is_empty(), "unexpected failing set: {failing:?}");
}
