//! Phase 4 task 6 liveness engine integration suite (Rust MC backend):
//! end-to-end checks of the fixpoint analysis, lasso counterexamples,
//! truncated-mode soundness, and the `--nondet` strong-fairness filter.
//! The C mirror lands in task 7 (parity is checked there).
//!
//! All specs are inline (established pattern from monitor_integration):
//! correct variants must reach pass-complete; broken variants must produce a
//! liveness violation with a structurally valid lasso counterexample - the
//! trace is prefix + one cycle iteration and its final configuration equals
//! the configuration at the cycle start.

use std::path::{Path, PathBuf};
use std::process::Command;

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

/// Scratch dir under target/ (self-cleaning with `cargo clean`), removed first
/// so re-runs start from a clean slate. One dir per test: tests run in parallel.
fn scratch(name: &str) -> PathBuf {
    let dir = root()
        .join("target")
        .join(format!("rfs_liveness_mc_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// MC codegen for an inline spec; returns (code, warnings).
fn codegen_mc(src: &str) -> (String, Vec<String>) {
    let ir = rfs::parse_source(src).expect("inline spec must parse");
    assert_eq!(ir.modules.len(), 1);
    let backend =
        rfs::codegen::get_backend("rust").unwrap_or_else(|| panic!("rust backend registered"));
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 0,
        with_property_monitors: false,
        ..rfs::codegen::BackendOptions::default()
    };
    let out = rfs::codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir.modules[0], &options)
        .expect("MC codegen must succeed");
    (out.code, out.warnings)
}

fn build(src: &str, dir: &Path, name: &str) -> PathBuf {
    let (code, _warnings) = codegen_mc(src);
    let s = dir.join(format!("{name}.rs"));
    std::fs::write(&s, code).expect("write generated MC source");
    let b = dir.join(name);
    let out = Command::new(rustc())
        .args(["--edition", "2021"])
        .arg(&s)
        .arg("-o")
        .arg(&b)
        .output()
        .expect("run rustc on generated code");
    assert!(
        out.status.success(),
        "rustc failed on {}:\n{}",
        s.display(),
        String::from_utf8_lossy(&out.stderr)
    );
    b
}

fn run(bin: &Path, args: &[&str]) -> (i32, String) {
    let out = Command::new(bin)
        .args(args)
        .output()
        .expect("run MC binary");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).to_string(),
    )
}

/// One exact-mode run, fully parsed: summary-line fields plus the liveness
/// counterexample fields and the per-position (state, vars) trace.
#[derive(Debug)]
struct LivOut {
    exit_code: i32,
    status: String,
    reason: Option<String>,
    label: Option<String>,
    step: Option<usize>,
    formula: Option<String>,
    cycle_length: Option<usize>,
    note: Option<String>,
    /// Config at each trace position (0 = initial configuration).
    positions: Vec<(String, String)>,
    /// Inputs of the trace steps (position k uses inputs[k-1]).
    step_inputs: Vec<String>,
}

fn parse_liv(stdout: &str, exit_code: i32) -> LivOut {
    let mut status = String::new();
    let mut reason = None;
    let mut label = None;
    let mut step = None;
    for line in stdout.lines() {
        if let Some(rest) = line.strip_prefix("model-check summary: ") {
            for tok in rest.split_whitespace() {
                if let Some((k, v)) = tok.split_once('=') {
                    match k {
                        "status" => status = v.to_string(),
                        "reason" => reason = Some(v.to_string()),
                        "label" => label = Some(v.to_string()),
                        "step" => step = Some(v.parse().unwrap_or(0)),
                        _ => {}
                    }
                }
            }
        }
    }
    let mut formula = None;
    let mut cycle_length = None;
    let mut note = None;
    let mut positions: Vec<(String, String)> = Vec::new();
    let mut step_inputs: Vec<String> = Vec::new();
    for line in stdout.lines() {
        let t = line.trim_start();
        if let Some(v) = t.strip_prefix("formula: ") {
            formula = Some(v.to_string());
        } else if let Some(n) = t.strip_prefix("cycle length: ") {
            cycle_length = n.parse().ok();
        } else if let Some(v) = t.strip_prefix("note: ") {
            note = Some(v.to_string());
        } else if let Some(rest) = t.strip_prefix("initial state=") {
            // "initial state=A vars=Vars { ... }"
            let (st, vars) = rest.split_once(" vars=").expect("vars field");
            positions.push((st.to_string(), vars.to_string()));
        } else if let Some(rest) = t.strip_prefix("step ") {
            // "step 0 input=Go outputs=[] from=A to=B vars=Vars { ... }"
            let rest = rest.split_once(' ').map(|(_, r)| r).unwrap_or("");
            let input = rest
                .strip_prefix("input=")
                .map(|s| s.split_whitespace().next().unwrap_or("").to_string())
                .unwrap_or_default();
            let to = rest
                .rsplit_once(" to=")
                .and_then(|(_, v)| v.split_whitespace().next())
                .unwrap_or("")
                .to_string();
            let vars = rest
                .rsplit_once(" vars=")
                .map(|(_, v)| v.to_string())
                .unwrap_or_default();
            step_inputs.push(input);
            positions.push((to, vars));
        }
    }
    LivOut {
        exit_code,
        status,
        reason,
        label,
        step,
        formula,
        cycle_length,
        note,
        positions,
        step_inputs,
    }
}

/// The lasso structure check: the trace is prefix + one cycle iteration and
/// its final configuration equals the configuration at the cycle start.
fn assert_lasso(out: &LivOut) {
    let s = out.step.expect("summary step field");
    let l = out.cycle_length.expect("cycle length field");
    assert!(l >= 1, "cycle must be non-empty");
    assert_eq!(
        out.positions.len(),
        s + l + 1,
        "trace must be prefix ({} positions) + one cycle iteration ({} steps)\n{}",
        s + 1,
        l,
        out.note.as_deref().unwrap_or("")
    );
    let (st_start, vars_start) = &out.positions[s];
    let (st_end, vars_end) = &out.positions[s + l];
    assert_eq!(
        st_start, st_end,
        "lasso must close: state at cycle start vs end"
    );
    assert_eq!(
        vars_start, vars_end,
        "lasso must close: vars at cycle start vs end"
    );
}

// ---------------------------------------------------------------------------
// Specs (inline; the broken variants are hand-run counterexample sources)
// ---------------------------------------------------------------------------

const PASS_SPEC: &str = r#"
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

const FAIL_EV_SPEC: &str = r#"
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

const FAIL_UNTIL_SPEC: &str = r#"
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

const FAIL_SKEL_SAFETY_SPEC: &str = r#"
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

const FAIL_INPUT_IS_SPEC: &str = r#"
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

const NONDET_DISC_SPEC: &str = r#"
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

const TRUNC_FAIL_SPEC: &str = r#"
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

const NEST_DIAG_SPEC: &str = r#"
module liv_nest_diag {
  enum State { A }
  initial A
  event Go
  var d: bool init false
  transition A on Go { next A; }
  prop nested : always (eventually (d)) ;
}
"#;

const SAFETY_PRECEDENCE_SPEC: &str = r#"
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

// ---------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------

#[test]
fn liveness_pass_complete() {
    let dir = scratch("pass");
    let bin = build(PASS_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 0, "{}", out);
    assert_eq!(r.status, "pass-complete");
    assert!(r.reason.is_none());
}

#[test]
fn liveness_eventually_violation() {
    let dir = scratch("fail_ev");
    let bin = build(FAIL_EV_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.status, "fail");
    assert_eq!(r.reason.as_deref(), Some("liveness-violation"));
    assert_eq!(r.label.as_deref(), Some("ev"));
    assert_eq!(r.formula.as_deref(), Some("eventually (d)"));
    assert_eq!(r.step, Some(0), "cycle starts at the initial configuration");
    assert_eq!(r.cycle_length, Some(2));
    assert_eq!(
        r.note.as_deref(),
        Some("eventually body never holds on the path")
    );
    assert_lasso(&r);
}

#[test]
fn liveness_until_violation() {
    let dir = scratch("fail_until");
    let bin = build(FAIL_UNTIL_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.reason.as_deref(), Some("liveness-violation"));
    assert_eq!(r.label.as_deref(), Some("p_until_q"));
    assert_eq!(r.formula.as_deref(), Some("p until q"));
    // The first not-p position is 1 (config B); the cycle starts there.
    assert_eq!(r.step, Some(1));
    assert_eq!(r.cycle_length, Some(2));
    assert_eq!(
        r.note.as_deref(),
        Some("until left fails at step 1 without right holding")
    );
    assert_lasso(&r);
}

#[test]
fn liveness_skeleton_safety_leaf_falsified() {
    let dir = scratch("fail_skel");
    let bin = build(FAIL_SKEL_SAFETY_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.reason.as_deref(), Some("liveness-violation"));
    assert_eq!(r.label.as_deref(), Some("skel"));
    assert_eq!(r.formula.as_deref(), Some("s && eventually (d)"));
    // The safety leaf breaks at step 1; the bad cycle (B<->C with d set)
    // starts at step 2.
    assert_eq!(r.step, Some(2));
    assert_eq!(r.cycle_length, Some(2));
    assert_eq!(r.note.as_deref(), Some("safety subformula false at step 1"));
    assert_lasso(&r);
}

#[test]
fn liveness_input_is_edge_labels() {
    let dir = scratch("fail_input_is");
    let bin = build(FAIL_INPUT_IS_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.reason.as_deref(), Some("liveness-violation"));
    assert_eq!(r.label.as_deref(), Some("eventually_ping"));
    // The witness cycle must use the Pong self-loop (the edge whose label
    // avoids Ping) - edge-based labeling, not node-based.
    assert_eq!(r.cycle_length, Some(1));
    assert_eq!(r.step_inputs, vec!["Pong".to_string()]);
    assert_lasso(&r);
}

#[test]
fn nondet_strong_fairness_filter() {
    let dir = scratch("nondet");
    let bin = build(NONDET_DISC_SPEC, &dir, "mc");
    // Deterministic mode: the Stay self-loop is a bad cycle -> FAIL.
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.reason.as_deref(), Some("liveness-violation"));
    // Nondet mode: the Stay self-loop is UNFAIR (Esc is also enabled at A
    // and never taken); every fair run takes Esc infinitely often, so the
    // property holds -> pass-complete.
    let (code, out) = run(&bin, &["--nondet"]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 0, "{}", out);
    assert_eq!(r.status, "pass-complete");
}

#[test]
fn truncated_analysis_sound_fail_and_unknown() {
    // Sound FAIL: the bad self-loop at A lies inside the explored part.
    let dir = scratch("trunc_fail");
    let bin = build(TRUNC_FAIL_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &["--max-states", "2"]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.status, "fail");
    assert_eq!(r.reason.as_deref(), Some("liveness-violation"));
    assert_lasso(&r);

    // UNKNOWN: with one state the bad cycle is cut off - never a false
    // pass-complete.
    let dir = scratch("trunc_unknown");
    let bin = build(FAIL_EV_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &["--max-states", "1"]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 2, "{}", out);
    assert_eq!(r.status, "unknown-state-limit");
    assert!(r.reason.is_none());
}

#[test]
fn deeper_nesting_codegen_diagnostic() {
    let (code, warnings) = codegen_mc(NEST_DIAG_SPEC);
    assert!(
        warnings.iter().any(|w| w.contains("deeper nesting")),
        "expected a deeper-nesting diagnostic, got {warnings:?}"
    );
    // The property is skipped (not a parse error): the binary compiles and
    // runs to pass-complete.
    let dir = scratch("nest_diag");
    let s = dir.join("mc.rs");
    std::fs::write(&s, code).expect("write generated MC source");
    let b = dir.join("mc");
    let out = Command::new(rustc())
        .args(["--edition", "2021"])
        .arg(&s)
        .arg("-o")
        .arg(&b)
        .output()
        .expect("run rustc on generated code");
    assert!(
        out.status.success(),
        "{}",
        String::from_utf8_lossy(&out.stderr)
    );
    let (code, out) = run(&b, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 0, "{}", out);
    assert_eq!(r.status, "pass-complete");
}

#[test]
fn safety_failure_takes_precedence_over_liveness() {
    // Both a monitor-checkable safety violation (at position 0) and a
    // liveness violation exist; the safety failure is reported and the
    // liveness path never runs.
    let dir = scratch("safety_prec");
    let bin = build(SAFETY_PRECEDENCE_SPEC, &dir, "mc");
    let (code, out) = run(&bin, &[]);
    let r = parse_liv(&out, code);
    assert_eq!(r.exit_code, 1, "{}", out);
    assert_eq!(r.status, "fail");
    assert_eq!(r.reason.as_deref(), Some("property-violation"));
    assert_eq!(r.label.as_deref(), Some("safe"));
}
