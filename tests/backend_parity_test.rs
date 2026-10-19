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
//!
//! Task 7 adds explicit liveness coverage: example 43 in exact mode must be
//! byte-identical across backends, and inline liveness variants (pass,
//! eventually/until/skeleton FAILs, input_is edge labels, --nondet fairness,
//! truncated-mode soundness, safety precedence) must agree on exit code and
//! full stdout - counterexample trace included.

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
        // Phase 4 benchmark specs (task 8). The token ring is the first spec
        // whose bit<2> arithmetic WRAPS (holder 3 -> 0): it exposed and pinned
        // the C backend's missing sub-byte modular mask on +,-,* (c_fsm.rs),
        // which previously let holder run past 3 in generated C.
        "44_token_ring.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 129985,
            transitions: 519940,
            max_depth: 276,
            reason: None,
            label: None,
            step: None,
            trigger_step: None,
            violation_step: None,
        },
        "45_elevator.rfs" => Pinned {
            exit_code: 0,
            status: "pass-complete",
            states: 11098,
            transitions: 99882,
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
    // Pinned current state: 9 pass-complete, 9 fail (1 property violation on
    // 13_properties_safety, 8 deadlocks at terminal states). Example 43 is the
    // first liveness-bearing spec (task 6): pass-complete in both backends;
    // examples 44/45 are the Phase 4 benchmark specs (task 8): pass-complete.
    assert_eq!((n_pass, n_fail), (9, 9), "pass/fail split drifted");
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

// ---------------------------------------------------------------------------
// Task 7: liveness parity coverage (C mirror of task 6's analysis)
//
// The inline specs below are the same ones used in tests/liveness_mc_test.rs
// (task 6, Rust only). Here BOTH backends must agree on exit code and
// byte-identical stdout - status, reason, label, step and the full lasso
// counterexample trace (prefix + one cycle iteration) included.
// ---------------------------------------------------------------------------

const LIV_PASS_SPEC: &str = r#"
module liv_pass {
  enum State { A, B }
  initial A
  event Go
  var d: bool init false
  transition A on Go { next B; set d = true; }
  transition B on Go { next A; }
  prop ev : eventually (d) ;
}
"#;

const LIV_FAIL_EV_SPEC: &str = r#"
module liv_fail_ev {
  enum State { A, B }
  initial A
  event Go
  var d: bool init false
  transition A on Go { next B; }
  transition B on Go { next A; }
  prop ev : eventually (d) ;
}
"#;

const LIV_FAIL_UNTIL_SPEC: &str = r#"
module liv_fail_until {
  enum State { A, B }
  initial A
  event Go
  var p: bool init true
  var q: bool init false
  transition A on Go { next B; set p = false; }
  transition B on Go { next A; }
  prop p_until_q : (p until q) ;
}
"#;

const LIV_FAIL_SKEL_SPEC: &str = r#"
module liv_fail_skel {
  enum State { A, B, C }
  initial A
  event Go
  var s: bool init true
  var d: bool init false
  transition A on Go { next B; set s = false; }
  transition B on Go { next C; set d = true; }
  transition C on Go { next B; }
  prop skel : (s && eventually (d)) ;
}
"#;

const LIV_FAIL_INPUT_IS_SPEC: &str = r#"
module liv_fail_input_is {
  enum State { A }
  initial A
  event Ping
  event Pong
  transition A on Ping { next A; }
  transition A on Pong { next A; }
  prop eventually_ping : eventually (input_is(Ping)) ;
}
"#;

const LIV_NONDET_DISC_SPEC: &str = r#"
module liv_nondet_disc {
  enum State { A, B }
  initial A
  event Stay
  event Esc
  var d: bool init false
  transition A on Stay { next A; }
  transition A on Esc { next B; set d = true; }
  transition B on Stay { next B; }
  transition B on Esc { next B; }
  prop ev_d : eventually (d) ;
}
"#;

const LIV_TRUNC_FAIL_SPEC: &str = r#"
module liv_trunc_fail {
  enum State { A, B, C }
  initial A
  event Go
  event Other
  var d: bool init false
  transition A on Go { next A; }
  transition A on Other { next B; }
  transition B on Go { next C; }
  transition C on Go { next C; }
  prop ev_d : eventually (d) ;
}
"#;

const LIV_SAFETY_PRECEDENCE_SPEC: &str = r#"
module liv_safety_precedence {
  enum State { A, B }
  initial A
  event Go
  var d: bool init false
  transition A on Go { next B; }
  transition B on Go { next A; }
  prop safe : always (d) ;
  prop ev : eventually (d) ;
}
"#;

/// MC codegen for an inline spec (the file-based twin of [`generate`]).
fn generate_inline(backend_name: &str, src: &str) -> String {
    let ir = rfs::parse_source(src)
        .unwrap_or_else(|e| panic!("inline liveness spec must parse ({backend_name}): {e}"));
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
        .run(backend, &ir.modules[0], &options)
        .unwrap_or_else(|e| panic!("MC codegen ({backend_name}) must succeed: {e:?}"))
        .code
}

fn liv_mc_binary(tag: &str, backend_name: &str, src: &str) -> PathBuf {
    cached_binary(&format!("mc|liv_{tag}|{backend_name}"), |dir| {
        let ext = if backend_name == "c" { "c" } else { "rs" };
        let s = dir.join(format!("liv_{tag}_mc.{ext}"));
        std::fs::write(&s, generate_inline(backend_name, src)).expect("write MC source");
        let b = dir.join(format!("liv_{tag}_mc_{backend_name}"));
        if backend_name == "c" {
            compile_c(&s, &b);
        } else {
            compile_rust(&s, &b, false);
        }
        b
    })
}

fn run_bin(bin: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(bin)
        .args(args)
        .output()
        .unwrap_or_else(|e| panic!("run MC binary {:?}: {e}", bin));
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

#[test]
fn mc_liveness_example_43_exact_parity() {
    // Explicit liveness coverage on the first liveness-bearing example:
    // exact-mode status, reason and full stdout must be identical across
    // backends (pass-complete, no counterexample).
    let spec = property_bearing_examples()
        .into_iter()
        .find(|p| p.file_name().and_then(|f| f.to_str()) == Some("43_sliding_window.rfs"))
        .expect("example 43 must be in the dynamic set");
    let rust = run_mc(&spec, "rust", &[]);
    let c = run_mc(&spec, "c", &[]);
    assert_eq!(rust.exit_code, Some(0), "{}", rust.stdout);
    assert_eq!(c.exit_code, Some(0), "{}", c.stdout);
    assert_eq!(
        rust.stdout, c.stdout,
        "example 43 exact-mode stdout must be byte-identical across backends"
    );
    let e = parse_exact(&rust.stdout, 0);
    assert_eq!(e.status, "pass-complete");
    assert!(
        e.reason.is_none() && e.label.is_none() && e.step.is_none(),
        "no counterexample expected on pass-complete: {e:?}"
    );
}

#[test]
fn mc_liveness_inline_variants_parity() {
    // Both backends must agree on exit code and byte-identical stdout for
    // every liveness variant - status, reason, label, step and the full
    // lasso counterexample trace (prefix + one cycle iteration) included.
    let variants: &[(&str, &str, &[&str])] = &[
        ("pass", LIV_PASS_SPEC, &[]),
        ("fail_ev", LIV_FAIL_EV_SPEC, &[]),
        ("fail_until", LIV_FAIL_UNTIL_SPEC, &[]),
        ("fail_skel", LIV_FAIL_SKEL_SPEC, &[]),
        ("fail_input_is", LIV_FAIL_INPUT_IS_SPEC, &[]),
        ("nondet_det", LIV_NONDET_DISC_SPEC, &[]),
        ("nondet_fair", LIV_NONDET_DISC_SPEC, &["--nondet"]),
        ("trunc_fail", LIV_TRUNC_FAIL_SPEC, &["--max-states", "2"]),
        ("trunc_unknown", LIV_FAIL_EV_SPEC, &["--max-states", "1"]),
        ("safety_prec", LIV_SAFETY_PRECEDENCE_SPEC, &[]),
    ];
    for (tag, src, args) in variants {
        let rust = run_bin(&liv_mc_binary(tag, "rust", src), args);
        let c = run_bin(&liv_mc_binary(tag, "c", src), args);
        assert_eq!(
            rust.0, c.0,
            "[liv {tag}] exit codes must match\n[rust]\n{}\n[c]\n{}",
            rust.1, c.1
        );
        assert_eq!(
            rust.1, c.1,
            "[liv {tag}] stdout must be byte-identical across backends"
        );
    }
}

#[test]
fn mc_liveness_counterexample_fields_pinned() {
    // Pinned lasso counterexample fields on the eventually-fail variant
    // (task 7 exit criterion: counterexample fields test-pinned): the cycle
    // starts at the initial configuration, has length 2, and the trace is
    // prefix + one cycle iteration.
    for (backend, bin) in [
        ("rust", liv_mc_binary("pin_ev", "rust", LIV_FAIL_EV_SPEC)),
        ("c", liv_mc_binary("pin_ev", "c", LIV_FAIL_EV_SPEC)),
    ] {
        let (code, out) = run_bin(&bin, &[]);
        assert_eq!(code, 1, "{backend}: {out}");
        assert!(out.contains("status=fail"), "{backend}: {out}");
        assert!(
            out.contains("reason=liveness-violation"),
            "{backend}: {out}"
        );
        assert!(out.contains("label=ev"), "{backend}: {out}");
        assert!(out.contains("step=0"), "{backend}: {out}");
        assert!(out.contains("liveness violation: ev"), "{backend}: {out}");
        assert!(out.contains("violation step: 0"), "{backend}: {out}");
        assert!(out.contains("cycle length: 2"), "{backend}: {out}");
    }
}

#[test]
fn mc_liveness_truncated_mode_sound() {
    // Bounded mode (task 7 exit criterion): with the bad cycle inside the
    // explored part both backends must report a sound FAIL; with it cut off
    // both must stay UNKNOWN - never a false pass-complete.
    let cases: &[(&str, &str, &[&str], &str)] = &[
        (
            "trunc_fail",
            LIV_TRUNC_FAIL_SPEC,
            &["--max-states", "2"],
            "fail",
        ),
        (
            "trunc_unknown",
            LIV_FAIL_EV_SPEC,
            &["--max-states", "1"],
            "unknown-state-limit",
        ),
    ];
    for (tag, src, args, want_status) in cases {
        for (backend, bin) in [
            ("rust", liv_mc_binary(tag, "rust", src)),
            ("c", liv_mc_binary(tag, "c", src)),
        ] {
            let (code, out) = run_bin(&bin, args);
            let e = parse_exact(&out, code);
            assert_eq!(e.status, *want_status, "[{backend} {tag}] {out}");
            assert_ne!(
                e.status, "pass-complete",
                "[{backend} {tag}] false pass-complete in truncated mode: {out}"
            );
        }
    }
}
