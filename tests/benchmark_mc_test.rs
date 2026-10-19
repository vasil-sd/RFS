//! Phase 4 benchmark MC suite (task 9): the three protocol-like benchmark
//! specs (43 sliding window, 44 token ring, 45 elevator) driven through BOTH
//! backends in exact mode - pass-complete statuses, deterministic metrics
//! identical across backends and pinned to the counts recorded in tasks 6/8,
//! inline FAIL variants (safety + liveness) with counterexample parity, and a
//! JSON report (`target/mc_benchmarks.json`) plus a per-backend summary table.
//!
//! Exact mode is seedless and deterministic (D-P4-7), so the pinned values are
//! exact engine outputs re-verified from both backends - not hand-computed.
//! Wall-clock times appear in the report and table as INFORMATIONAL data only:
//! they are never asserted on (the suite runs inside `cargo test` and must not
//! become a flaky timing gate; task 10 consumes the data for tuning).
//!
//! The JSON report is a development artifact: written under `target/`, never
//! checked in, and asserted on by parsed fields only (never byte-for-byte).

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

/// The Phase 4 benchmark set: one spec per protocol family, each carrying
/// safety + liveness properties and abstraction-stress payload fields.
const BENCH_SPECS: &[&str] = &[
    "43_sliding_window.rfs",
    "44_token_ring.rfs",
    "45_elevator.rfs",
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
    match name {
        "43_sliding_window.rfs" => Pin {
            states: 165,
            transitions: 330,
            max_depth: 22,
            monitors_bits: 4,
        },
        "44_token_ring.rfs" => Pin {
            states: 129_985,
            transitions: 519_940,
            max_depth: 276,
            monitors_bits: 10,
        },
        "45_elevator.rfs" => Pin {
            states: 11_098,
            transitions: 99_882,
            max_depth: 22,
            monitors_bits: 4,
        },
        other => panic!("no pin for benchmark spec {other}"),
    }
}

// ---------------------------------------------------------------------------
// Inline FAIL variants (action 2) - modified copies of the benchmark specs
// with a known violation. They live inline in this file on purpose: they are
// test inputs, not part of the valid-example set (no new example files).
// ---------------------------------------------------------------------------

/// `44_token_ring` with the Tick GRANT arm removed: a granted node's request
/// flag is never cleared, so the safety invariant `holder_no_request` is
/// violated at step 1. Safety takes precedence over liveness here - the
/// `req_granted_*` properties also fail on this path but are reported after.
const RING_NO_GRANT_SRC: &str = r#"
# FAIL VARIANT (benchmark_mc_test, task 9): the Tick grant arm is removed,
# so a granted node's request flag is never cleared -> holder_no_request violated.
#
# A single token circulates the ring; on every Tick it passes to the next
# node in ring order (bit<2> addition wraps 3 -> 0). A node with a pending
# request is granted when the token arrives at it: its flag clears and a
# Grant is emitted. Nodes may request only while they do not hold the token.
#
# Liveness "every request is eventually granted": nodes 1-3 request at
# startup (req init 14); req_granted_* check that each startup request is
# cleared, and flags are cleared ONLY on grant (safety invariant
# holder_no_request). The bounded property checks EVERY instance of node 1's
# requests (the environment may re-request at any time): the token needs at
# most 3 hops and env_tick bounds each hop to 5 samples.
#
# Abstraction stress: Request carries a bit<16> payload the protocol never
# inspects - input abstraction (D-P4-3) collapses it to one representative,
# so the state space is independent of its width. The token's carried frame
# ID (data: bit<8>) is a CONCRETE wide field: updated on every pass but never
# inspected, it multiplies the state space by 256 and shows the contrast.
module token_ring {
  enum State { Running }
  initial Running

  event Tick
  event Request(R(node: bit<2>, payload: bit<16>))

  output Grant(G(node: bit<2>, frame: bit<8>))
  output Pass(P(from: bit<2>, to: bit<2>))

  var holder: bit<2> init 0   # current token holder (ring position)
  var req: bit<8>    init 14  # pending request flags, one per node; only bits 0-3 used
  var data: bit<8>   init 0   # frame ID carried by the token, +1 each pass (concrete wide field)

  assume env_tick : always (bounded_eventually[5] (input_is(Tick))) ;

  # Safety: the current holder never has a pending request - flags are only
  # cleared on grant, at the granting step.
  prop holder_no_request : always (req & (1 << holder) == 0) ;

  # Liveness: every startup request is eventually granted (nodes 1-3).
  prop req_granted_1 : eventually (req & 2 == 0) ;
  prop req_granted_2 : eventually (req & 4 == 0) ;
  prop req_granted_3 : eventually (req & 8 == 0) ;

  # Liveness (bounded, every instance): node 1's request is granted within
  # 25 samples of being set.
  prop req_granted_1_bounded : always (req & 2 != 0 -> bounded_eventually[25] (req & 2 == 0)) ;

  # A node requests the token (only while it does not hold it). The payload
  # is never inspected - input abstraction collapses it to one value.
  transition Running on Request(R(node, payload)) where node != holder {
    next Running;
    set req = req | (1 << node);
  }

  # Token passes to the next node in ring order. If that node has a pending
  # request it is granted: flag clears and a Grant is emitted.
  transition Running on Tick {
    priority 1;
    next Running;
    emit Pass(P(holder, holder + 1));
    set holder = holder + 1;
    set data = data + 1;
  }
}
"#;

/// `45_elevator` with the Idle go-up arm removed: an up-call at a floor above
/// the current one can never be served, so the liveness property `up2_served`
/// is violated (lasso, cycle length 3 - the elevator idles at floor 0 forever
/// while the startup up-call at floor 2 stays pending). The safety invariant
/// `idle_no_local_call` still holds on the path.
const ELEV_NO_UP_SRC: &str = r#"
# FAIL VARIANT (benchmark_mc_test, task 9): the Idle go-up arm is removed,
# so an up-call above can never be served -> up2_served liveness violated.
#
# The elevator moves one floor per Tick. While moving it stops to serve any
# pending call at the current floor (both directions, one stop); it reverses
# direction only at the ends and waits (Idle) when no call is anywhere. A
# press at the floor the elevator is currently at is served immediately (one
# stop clears both call vectors there), so a press never leaves a pending
# local bit behind; calls pressed elsewhere are served on the next visit.
# Two calls wait at startup: an up-call at floor 2 and a down-call at floor 3.
#
# Liveness "every call is eventually served": up2_served / down3_served check
# the two startup calls (non-vacuous: both are pending at init). Per-instance
# bounded liveness (always (pending -> bounded_eventually[N] cleared)) lives in
# spec 44 instead: each such obligation adds ~N countdown states to every
# config where the bit is set, which blew this spec past the Phase 4 sizing
# target (1M+ states with two bound-80 obligations).
#
# Note: only ONE environment assumption (env_tick). Two independent
# bounded_eventually assumptions over disjoint event sets admit prefixes where
# both countdowns reach their deadline on the same sample - no single input can
# satisfy both, so every successor is pruned and exact MC reports a spurious
# deadlock. Kept out of this benchmark spec on purpose.
#
# Abstraction stress: CallUp/CallDown carry a bit<16> user_id the protocol
# never inspects - input abstraction (D-P4-3) collapses it to one
# representative, so the state space is independent of its width. The call
# vectors are CONCRETE wide fields (bit<8>, only bits 0-3 used): 16x16
# combinations multiply the state space.
module elevator {
  enum State { Idle, Up, Down }
  initial Idle

  event Tick
  event CallUp(F(floor: bit<2>, user_id: bit<16>))
  event CallDown(F(floor: bit<2>, user_id: bit<16>))

  output Serve(S(floor: bit<2>))

  var pos: bit<2>      init 0   # current floor (0 = bottom, 3 = top)
  var up_calls: bit<8> init 4   # pending up-calls, one bit per floor; only bits 0-3 used
  var down_calls: bit<8> init 8 # pending down-calls, one bit per floor; only bits 0-3 used

  assume env_tick : always (bounded_eventually[5] (input_is(Tick))) ;

  # Safety: the elevator never idles while a passenger waits at its floor
  # (holds at every sample: presses at the current floor are served in the
  # same step, and stops clear both call vectors there).
  prop idle_no_local_call : always (state == Idle -> up_calls & (1 << pos) == 0 && down_calls & (1 << pos) == 0) ;

  # Liveness: the two startup calls are eventually served.
  prop up2_served : eventually (up_calls & 4 == 0) ;
  prop down3_served : eventually (down_calls & 8 == 0) ;

  # Passengers press calls (the user_id is never inspected - input
  # abstraction collapses it to one value). A press at the floor the elevator
  # is currently at is served immediately: one stop clears BOTH call vectors
  # at that floor (and the pressing passenger boards) WITHOUT losing the
  # current state - staying in Up/Down is what makes up2_served / down3_served
  # hold against an environment that spams presses at the current floor
  # (any reset to Idle would let a press interrupt every ascent attempt). A
  # press elsewhere sets the pending bit; it is served on the next visit.
  transition Idle on CallUp(F(floor, user_id)) where floor == pos {
    priority 2;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Idle on CallUp(F(floor, user_id)) where floor != pos {
    priority 1;
    next Idle;
    set up_calls = up_calls | (1 << floor);
  }
  transition Up on CallUp(F(floor, user_id)) where floor == pos {
    priority 2;
    next Up;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Up on CallUp(F(floor, user_id)) where floor != pos {
    priority 1;
    next Up;
    set up_calls = up_calls | (1 << floor);
  }
  transition Down on CallUp(F(floor, user_id)) where floor == pos {
    priority 2;
    next Down;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Down on CallUp(F(floor, user_id)) where floor != pos {
    priority 1;
    next Down;
    set up_calls = up_calls | (1 << floor);
  }
  transition Idle on CallDown(F(floor, user_id)) where floor == pos {
    priority 2;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Idle on CallDown(F(floor, user_id)) where floor != pos {
    priority 1;
    next Idle;
    set down_calls = down_calls | (1 << floor);
  }
  transition Up on CallDown(F(floor, user_id)) where floor == pos {
    priority 2;
    next Up;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Up on CallDown(F(floor, user_id)) where floor != pos {
    priority 1;
    next Up;
    set down_calls = down_calls | (1 << floor);
  }
  transition Down on CallDown(F(floor, user_id)) where floor == pos {
    priority 2;
    next Down;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Down on CallDown(F(floor, user_id)) where floor != pos {
    priority 1;
    next Down;
    set down_calls = down_calls | (1 << floor);
  }

  # Moving up: stop to serve a call at the current floor (arrived with it
  # pending); otherwise keep going up. After serving the last demand the
  # elevator finishes its ascent to the top before reversing or waiting - a
  # documented simplification; at the top, reverse if a call waits below.
  transition Up on Tick where ((up_calls | down_calls) & (1 << pos)) != 0 {
    priority 3;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Up on Tick where pos < 3 {
    priority 2;
    next Up;
    set pos = pos + 1;
  }
  transition Up on Tick where pos == 3 && ((up_calls | down_calls) & ((1 << pos) - 1)) != 0 {
    priority 1;
    next Down;
  }
  transition Up on Tick {
    next Idle;
  }

  # Moving down: mirror image of the Up arms.
  transition Down on Tick where ((up_calls | down_calls) & (1 << pos)) != 0 {
    priority 3;
    next Idle;
    emit Serve(S(pos));
    set up_calls = up_calls & ~(1 << pos);
    set down_calls = down_calls & ~(1 << pos);
  }
  transition Down on Tick where pos > 0 {
    priority 2;
    next Down;
    set pos = pos - 1;
  }
  transition Down on Tick where pos == 0 && ((up_calls | down_calls) & ~((1 << (pos + 1)) - 1)) != 0 {
    priority 1;
    next Up;
  }
  transition Down on Tick {
    next Idle;
  }

  # Idle: head to the nearest demand (up first), or wait when no call is
  # anywhere. No serve arm here: state Idle never has a pending call at the
  # current floor (idle_no_local_call).
  # Call at some floor strictly above pos: floors >= pos minus the current
  # floor (built from `1 << pos` - `pos + 1` would wrap at the top floor).
  transition Idle on Tick where ((up_calls | down_calls) & ((1 << pos) - 1)) != 0 {
    priority 1;
    next Down;
  }
  transition Idle on Tick {
    next Idle;
  }
}
"#;

// ---------------------------------------------------------------------------
// Generation + compilation (cached: the three tests share the MC binaries)
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
        .run(backend, &ir.modules[0], &options)
        .unwrap_or_else(|e| panic!("codegen {backend_name} for {what} must succeed: {e:?}"))
        .code
}

fn spec_source(name: &str) -> String {
    let p = root().join("examples/valid").join(name);
    std::fs::read_to_string(&p).unwrap_or_else(|e| panic!("read {:?}: {e}", p))
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
}

#[test]
fn bench_mc_fail_variants_both_backends() {
    let cases: [(&str, &str, FailPin); 2] = [
        // Safety variant: ring grant arm removed.
        (
            "ring_no_grant",
            RING_NO_GRANT_SRC,
            FailPin {
                reason: "property-violation",
                label: "holder_no_request",
                step: 1,
                states: 1,
                transitions: 1,
                max_depth: 0,
                violation_step: Some(1),
                cycle_length: None,
            },
        ),
        // Liveness variant: elevator Idle go-up arm removed.
        (
            "elev_noup",
            ELEV_NO_UP_SRC,
            FailPin {
                reason: "liveness-violation",
                label: "up2_served",
                step: 0,
                states: 144,
                transitions: 1296,
                max_depth: 7,
                violation_step: Some(0),
                cycle_length: Some(3),
            },
        ),
    ];

    for (stem, src, pin) in cases {
        let re = run_parsed(&bench_binary(stem, "rust", src));
        let ce = run_parsed(&bench_binary(stem, "c", src));

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

        // Counterexample fields (and all other deterministic fields) identical
        // across backends - the lasso for the liveness variant included.
        assert_eq!(
            det(&re),
            det(&ce),
            "[{stem}] FAIL counterexample must be field-identical across backends"
        );
    }
}

// ---------------------------------------------------------------------------
// Action 3: JSON report (target/mc_benchmarks.json) + per-backend table
// ---------------------------------------------------------------------------

fn run_json(r: &BenchRun) -> serde_json::Value {
    serde_json::json!({
        "status": r.status,
        "states": r.states,
        "transitions": r.transitions,
        "max_depth": r.max_depth,
        "unexpected_terminations": r.unexpected_terminations,
        "monitors_bits": r.monitors_bits,
        "memory_estimate_bytes": r.memory_estimate_bytes,
        "wall_ms": r.wall_ms,
    })
}

#[test]
fn bench_mc_json_report_and_table() {
    let mut entries: Vec<(String, HashMap<&'static str, BenchRun>)> = Vec::new();
    for name in BENCH_SPECS {
        let src = spec_source(name);
        let stem = name.strip_suffix(".rfs").expect("spec name");
        let runs: HashMap<&str, BenchRun> = [
            ("rust", run_parsed(&bench_binary(stem, "rust", &src))),
            ("c", run_parsed(&bench_binary(stem, "c", &src))),
        ]
        .into_iter()
        .collect();
        entries.push((name.to_string(), runs));
    }

    // Report: per spec x backend - status, deterministic metrics, wall-clock
    // (informational). Written under target/ - a development artifact.
    let specs = entries
        .iter()
        .map(|(name, runs)| {
            let backends = serde_json::json!({
                "rust": run_json(&runs["rust"]),
                "c": run_json(&runs["c"]),
            });
            serde_json::json!({ "spec": name, "backends": backends })
        })
        .collect::<Vec<_>>();
    let report = serde_json::json!({
        "suite": "benchmark_mc",
        "mode": "exact",
        "specs": specs,
    });
    let path = root().join("target").join("mc_benchmarks.json");
    std::fs::write(
        &path,
        serde_json::to_string_pretty(&report).expect("serialize report"),
    )
    .unwrap_or_else(|e| panic!("write {:?}: {e}", path));

    // Assert on PARSED fields (never byte-for-byte): read the file back and
    // check status + deterministic metrics match what the runs produced.
    let on_disk: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&path).expect("read report"))
            .expect("report must be valid JSON");
    for (name, runs) in &entries {
        for be in ["rust", "c"] {
            let r = &runs[be];
            let j = &on_disk["specs"]
                .as_array()
                .and_then(|a| a.iter().find(|s| s["spec"] == *name))
                .expect("report entry for spec")["backends"][be];
            assert_eq!(j["status"], r.status, "[{name}] {be} status in report");
            assert_eq!(
                j["states"], r.states as u64,
                "[{name}] {be} states in report"
            );
            assert_eq!(
                j["transitions"], r.transitions as u64,
                "[{name}] {be} transitions in report"
            );
            assert_eq!(
                j["max_depth"], r.max_depth as u64,
                "[{name}] {be} max_depth in report"
            );
            assert!(
                j.get("wall_ms").is_some(),
                "[{name}] {be} wall_ms present (info only)"
            );
        }
    }

    // Per-backend summary table to the test output (wall clock informational).
    let mut table = String::from(
        "benchmark MC suite (exact mode) - deterministic metrics identical across backends\n",
    );
    table.push_str(&format!(
        "{:<24} {:<8} {:<14} {:>9} {:>13} {:>9} {:>16}\n",
        "spec", "backend", "status", "states", "transitions", "max_depth", "wall_ms (info)"
    ));
    for (name, runs) in &entries {
        for be in ["rust", "c"] {
            let r = &runs[be];
            table.push_str(&format!(
                "{:<24} {:<8} {:<14} {:>9} {:>13} {:>9} {:>16}\n",
                name, be, r.status, r.states, r.transitions, r.max_depth, r.wall_ms
            ));
        }
    }
    eprintln!("{table}");
}

// ---------------------------------------------------------------------------
// Task 10B action 3: --threads parity gate (byte-identity across N and
// backends) + --fast smoke test (soundness only - non-parity mode).
// ---------------------------------------------------------------------------

/// All five workloads of the threads gate: the three benchmark specs plus
/// both inline FAIL variants (safety + liveness counterexamples included).
fn threads_gate_workloads() -> Vec<(&'static str, String)> {
    let mut out: Vec<(&'static str, String)> = BENCH_SPECS
        .iter()
        .map(|name| {
            (
                name.strip_suffix(".rfs").expect("spec name"),
                spec_source(name),
            )
        })
        .collect();
    out.push(("ring_no_grant", RING_NO_GRANT_SRC.to_string()));
    out.push(("elev_noup", ELEV_NO_UP_SRC.to_string()));
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
    for (stem, src) in threads_gate_workloads() {
        let ref_run = run_args_full(&bench_binary(stem, "rust", &src), &["--threads", "1"]);
        for be in ["rust", "c"] {
            let bin = bench_binary(stem, be, &src);
            for (n, args) in [
                ("2", &["--threads", "2"][..]),
                ("4", &["--threads", "4"][..]),
            ] {
                let r = run_args_full(&bin, args);
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
