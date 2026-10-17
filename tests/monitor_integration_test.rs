//! Phase 3 monitor integration suite (task 16): end-to-end proof that the
//! online property monitors work in BOTH backends:
//!   (a) MC catches an injected bug with property-specific output,
//!   (b) the correct spec passes within bound (no false positives),
//!   (c) production debug monitors surface violations at the step API.
//!
//! Reference spec: `examples/valid/41_property_monitors.rfs`. The buggy
//! variant drops the Ack-handling transition, so a Fin can never be answered
//! and `fin_acked_bounded` must expire. All runs use FIXED seeds; the buggy
//! MC walk is in fact fully deterministic (exactly one allowed input per
//! state: Send(Fin) from Idle, then Tick forever in SentFin), so the asserted
//! step numbers are seed-independent. The production drivers (Rust + C) share
//! the same violating sequence and must report the same label/step values.

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
        .join(format!("rfs_monitor_integration_{name}"));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    dir
}

fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// gcc location (plan assumption 5, verified since Phase 2).
const GCC: &str = "/usr/bin/gcc";

fn compile_rust(src: &Path, bin: &Path) {
    let out = Command::new(rustc())
        .args(["--edition", "2021"])
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

fn compile_c(src: &Path, bin: &Path, defines: &[&str]) {
    let mut cmd = Command::new(GCC);
    cmd.arg("-Wall");
    for d in defines {
        cmd.arg(format!("-D{d}"));
    }
    let out = cmd
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

fn example41_source() -> String {
    std::fs::read_to_string(root().join("examples/valid/41_property_monitors.rfs"))
        .expect("read example 41")
}

/// Buggy variant of example 41: the Ack-handling transition is dropped, so a
/// Fin can never be answered and `fin_acked_bounded` must expire.
fn buggy_spec_source() -> String {
    let original = example41_source();
    const ACK_TRANSITION: &str = "  transition SentFin on Recv(Ack) {\n    next Acked;\n    emit Send(Ack);\n    set ack_received = true;\n  }\n";
    let buggy = original.replace(
        ACK_TRANSITION,
        "  # BUG (test variant): the Ack-handling transition is dropped.\n",
    );
    assert_ne!(
        buggy, original,
        "removing the Ack transition must change the spec"
    );
    assert!(!buggy.contains("transition SentFin on Recv(Ack)"));
    buggy
}

/// In-process codegen (same path as the CLI): parse `source`, run `command`
/// for `backend_name` with the given monitor flag, return the generated text.
fn generate(
    backend_name: &str,
    command: rfs::codegen::CodegenCommandKind,
    source: &str,
    with_property_monitors: bool,
) -> String {
    let ir = rfs::parse_source(source)
        .unwrap_or_else(|e| panic!("spec must parse ({backend_name}): {e}"));
    assert_eq!(ir.modules.len(), 1);
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors,
    };
    command
        .run(backend, &ir.modules[0], &options)
        .unwrap_or_else(|e| panic!("codegen {backend_name} must succeed: {e:?}"))
        .code
}

// ---------------------------------------------------------------------------
// Action 1: buggy spec -> MC detects the violation in BOTH backends
// ---------------------------------------------------------------------------

/// The property-specific counterexample fields printed by both MC runtimes
/// (identical wire format, see src/codegen_mc.rs).
struct McViolation {
    label: String,
    trigger_step: usize,
    violation_step: usize,
}

fn parse_mc_violation(stdout: &str) -> Option<McViolation> {
    let mut label = None;
    let mut trigger = None;
    let mut violation = None;
    for line in stdout.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("run ") {
            if let Some(l) = rest.split_once(" property violation: ") {
                label = Some(l.1.trim().to_string());
            }
        } else if let Some(n) = t.strip_prefix("trigger step: ") {
            trigger = n.parse().ok();
        } else if let Some(n) = t.strip_prefix("violation step: ") {
            violation = n.parse().ok();
        }
    }
    match (label, trigger, violation) {
        (Some(l), Some(t), Some(v)) => Some(McViolation {
            label: l,
            trigger_step: t,
            violation_step: v,
        }),
        _ => None,
    }
}

fn mc_buggy_run(backend_name: &str, dir: &Path) -> McViolation {
    let kind = rfs::codegen::CodegenCommandKind::ModelChecker;
    if backend_name == "c" {
        let p = dir.join("mc.c");
        std::fs::write(
            &p,
            generate(backend_name, kind, &buggy_spec_source(), false),
        )
        .expect("write C MC source");
        compile_c(&p, &dir.join("mc"), &[]);
    } else {
        let p = dir.join("mc.rs");
        std::fs::write(
            &p,
            generate(backend_name, kind, &buggy_spec_source(), false),
        )
        .expect("write Rust MC source");
        compile_rust(&p, &dir.join("mc"));
    }
    // Bound 10 covers the deadline (sample 5) with margin; fixed seed per
    // task file. The walk is deterministic anyway (one allowed input/state).
    let out = Command::new(dir.join("mc"))
        .args(["--bound", "10", "--runs", "5", "--seed", "42"])
        .output()
        .expect("run generated MC binary");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(
        out.status.code(),
        Some(1),
        "[{backend_name}] buggy spec: MC must exit 1 on a property violation; stdout:\n{stdout}"
    );
    let v = parse_mc_violation(&stdout).unwrap_or_else(|| {
        panic!("[{backend_name}] property-specific counterexample missing; stdout:\n{stdout}")
    });
    assert!(
        stdout.contains("counterexample trace:"),
        "[{backend_name}] counterexample trace must be printed; stdout:\n{stdout}"
    );
    v
}

#[test]
fn mc_catches_injected_bug_both_backends() {
    let rust_dir = scratch("mc_bad_rust");
    let c_dir = scratch("mc_bad_c");
    let rust_v = mc_buggy_run("rust", &rust_dir);
    let c_v = mc_buggy_run("c", &c_dir);

    // Cross-backend parity: same property label, same trigger/deadline steps.
    assert_eq!(
        rust_v.label, c_v.label,
        "property label must match across backends"
    );
    assert_eq!(
        rust_v.trigger_step, c_v.trigger_step,
        "trigger step must match across backends"
    );
    assert_eq!(
        rust_v.violation_step, c_v.violation_step,
        "violation (deadline) step must match across backends"
    );

    // Pinned values: Send(Fin) at sample 1 triggers fin_acked_bounded (bound
    // 5); with the Ack transition gone the body can never hold, so the
    // violation is confirmed at sample 1 + 5 - 1 = 5.
    assert_eq!(rust_v.label, "fin_acked_bounded");
    assert_eq!(rust_v.trigger_step, 1);
    assert_eq!(rust_v.violation_step, 5);
}

// ---------------------------------------------------------------------------
// Action 2: correct spec passes within bound in both backends
// ---------------------------------------------------------------------------

/// The summary field names of a parsed summary line, in order.
fn summary_keys(v: &[(String, String)]) -> Vec<&str> {
    v.iter().map(|(k, _)| k.as_str()).collect()
}

/// Parse `model-check summary: k=v ...` into (key, value) pairs in order.
fn parse_mc_summary(stdout: &str) -> Option<Vec<(String, String)>> {
    let line = stdout
        .lines()
        .find(|l| l.starts_with("model-check summary: "))?;
    Some(
        line["model-check summary: ".len()..]
            .split_whitespace()
            .filter_map(|t| {
                t.split_once('=')
                    .map(|(k, v)| (k.to_string(), v.to_string()))
            })
            .collect(),
    )
}

fn mc_good_run(backend_name: &str, dir: &Path) -> Vec<(String, String)> {
    let kind = rfs::codegen::CodegenCommandKind::ModelChecker;
    let src = generate(backend_name, kind, &example41_source(), false);
    if backend_name == "c" {
        let p = dir.join("mc.c");
        std::fs::write(&p, src).expect("write C MC source");
        compile_c(&p, &dir.join("mc"), &[]);
    } else {
        let p = dir.join("mc.rs");
        std::fs::write(&p, src).expect("write Rust MC source");
        compile_rust(&p, &dir.join("mc"));
    }
    // Bound 4 is chosen so that NO property can be confirmed within the
    // horizon, for any seed or run count: fin_acked_bounded needs trigger +
    // 4 samples (sample 5) to expire; retries_bounded_window needs 6 ticks
    // (and only checks samples 0..9); the safety invariants are unreachable
    // in a violating form. Zero violations is therefore provable, not lucky.
    let out = Command::new(dir.join("mc"))
        .args(["--bound", "4", "--runs", "100", "--seed", "12345"])
        .output()
        .expect("run generated MC binary");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    assert_eq!(
        out.status.code(),
        Some(0),
        "[{backend_name}] correct spec: MC must exit 0 within bound; stdout:\n{stdout}"
    );
    assert!(
        !stdout.contains("property violation"),
        "[{backend_name}] no property violation may be reported; stdout:\n{stdout}"
    );
    parse_mc_summary(&stdout)
        .unwrap_or_else(|| panic!("[{backend_name}] summary line missing; stdout:\n{stdout}"))
}

#[test]
fn mc_correct_spec_passes_within_bound_both_backends() {
    let rust_dir = scratch("mc_good_rust");
    let c_dir = scratch("mc_good_c");
    let rust_s = mc_good_run("rust", &rust_dir);
    let c_s = mc_good_run("c", &c_dir);

    // Identical summary field sets (wire-format parity)...
    assert_eq!(
        summary_keys(&rust_s),
        summary_keys(&c_s),
        "summary field names must match across backends"
    );
    assert_eq!(
        summary_keys(&rust_s),
        vec![
            "runs",
            "bound",
            "completed",
            "unexpected",
            "assumption_stops",
            "property_violations"
        ]
    );
    // ...and zero property violations on the correct spec (no false positives).
    for (name, summary) in [("rust", &rust_s), ("c", &c_s)] {
        let pv = summary
            .iter()
            .find(|(k, _)| k == "property_violations")
            .expect("property_violations field");
        assert_eq!(
            pv.1, "0",
            "[{name}] correct spec must have zero property violations: {summary:?}"
        );
    }
}

// ---------------------------------------------------------------------------
// Actions 3 + 4: production debug monitors surface the violation at the step
// API (Rust StepError / C RFS_PROPERTY_VIOLATION), same values in both.
// ---------------------------------------------------------------------------

/// The violating sequence shared by both drivers: Fin, then four Ticks.
/// `fin_acked_bounded` triggers at sample 1 (bound 5) and expires at sample
/// 5, so steps 1-4 are Ok and step 5 surfaces the violation.
const RUST_DRIVER: &str = r#"
#[path = "fsm.rs"]
mod fsm;
use fsm::*;

fn main() {
    let mut f = Fsm::new();
    let seq = [Input::Send_Fin, Input::Tick, Input::Tick, Input::Tick, Input::Tick];
    for (i, inp) in seq.iter().enumerate() {
        match f.step(*inp) {
            Ok(_) => println!("step {} ok", i + 1),
            Err(e) => println!("step {} err {:?}", i + 1, e),
        }
    }
}
"#;

const C_DRIVER: &str = r#"
#include "fsm.c"
#include <stdio.h>

static const char* status_name(rfs_status_t s) {
    switch (s) {
        case RFS_OK: return "RFS_OK";
        case RFS_UNEXPECTED_EVENT: return "RFS_UNEXPECTED_EVENT";
        case RFS_ASSUMPTION_VIOLATION: return "RFS_ASSUMPTION_VIOLATION";
        case RFS_PROPERTY_VIOLATION: return "RFS_PROPERTY_VIOLATION";
    }
    return "?";
}
static rfs_policy_t stop_handler(void* user_data, const char* label) { (void)user_data; (void)label; return RFS_STOP; }

int main(void) {
    fsm_t f;
    fsm_new(&f);
    input_t seq[5] = {
        { .kind = INPUT_SEND_FIN },
        { .kind = INPUT_TICK },
        { .kind = INPUT_TICK },
        { .kind = INPUT_TICK },
        { .kind = INPUT_TICK },
    };
    for (int i = 0; i < 5; i++) {
        step_result_t r = fsm_step(&f, seq[i], stop_handler, 0);
        printf("step %d status=%s", i + 1, status_name(r.status));
        if (r.status == RFS_PROPERTY_VIOLATION) {
            printf(" prop_label=%s prop_step=%d prop_trigger_step=%d", r.prop_label, r.prop_step, r.prop_trigger_step);
        }
        printf("\n");
    }
    return 0;
}
"#;

#[test]
fn rust_production_debug_monitors_surface_property_violation() {
    let dir = scratch("rust_prod");
    // --with-property-monitors: monitors emitted, compile-gated to debug builds.
    std::fs::write(
        dir.join("fsm.rs"),
        generate(
            "rust",
            rfs::codegen::CodegenCommandKind::Production,
            &example41_source(),
            true,
        ),
    )
    .expect("write generated FSM");
    std::fs::write(dir.join("driver.rs"), RUST_DRIVER).expect("write driver");
    // NO -O: the StepError::PropertyViolation variant is #[cfg(debug_assertions)].
    compile_rust(&dir.join("driver.rs"), &dir.join("driver"));

    let out = Command::new(dir.join("driver"))
        .output()
        .expect("run Rust driver");
    assert!(
        out.status.success(),
        "Rust driver failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    for i in 1..=4 {
        assert!(
            stdout.contains(&format!("step {i} ok")),
            "step {i} must be Ok; stdout:\n{stdout}"
        );
    }
    // Pinned: same label/step the MC counterexample reports (Action 1).
    assert!(
        stdout.contains("step 5 err PropertyViolation { label: \"fin_acked_bounded\", step: 5 }"),
        "step 5 must surface the property violation; stdout:\n{stdout}"
    );
}

#[test]
fn c_production_monitors_surface_property_violation_parity() {
    let dir = scratch("c_prod");
    std::fs::write(
        dir.join("fsm.c"),
        generate(
            "c",
            rfs::codegen::CodegenCommandKind::Production,
            &example41_source(),
            true,
        ),
    )
    .expect("write generated FSM");
    std::fs::write(dir.join("driver.c"), C_DRIVER).expect("write driver");
    // -DRFS_PROPERTY_MONITORS activates the #ifdef-gated monitor code.
    compile_c(
        &dir.join("driver.c"),
        &dir.join("driver"),
        &["RFS_PROPERTY_MONITORS"],
    );

    let out = Command::new(dir.join("driver"))
        .output()
        .expect("run C driver");
    assert!(
        out.status.success(),
        "C driver failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    for i in 1..=4 {
        assert!(
            stdout.contains(&format!("step {i} status=RFS_OK")),
            "step {i} must be RFS_OK; stdout:\n{stdout}"
        );
    }
    // Parity with the Rust step API (Action 3): same label, step and trigger
    // sample as `PropertyViolation { label: "fin_acked_bounded", step: 5 }`
    // with trigger_step 1 (the MC counterexample values from Action 1).
    assert!(
        stdout.contains("step 5 status=RFS_PROPERTY_VIOLATION prop_label=fin_acked_bounded prop_step=5 prop_trigger_step=1"),
        "step 5 must report RFS_PROPERTY_VIOLATION with the Rust values; stdout:\n{stdout}"
    );
}
