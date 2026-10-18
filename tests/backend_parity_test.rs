//! Phase 3 backend-parity differential suite (task 17): the backend parity
//! principle made executable - Rust and C model checkers run on identical
//! specs/seeds must produce indistinguishable results, and both backends'
//! generated test executables must agree on every example.
//!
//! Scope: ALL valid examples carrying `prop`/`assume` declarations, enumerated
//! DYNAMICALLY (including `examples/valid/tla_adapted/`) so future additions
//! are covered automatically. Run parameters are pinned and SMALL
//! (`--bound 8 --runs 20 --seed 777`): the point is determinism + equality,
//! not coverage depth.
//!
//! Known recorded state (see PHASE_3_TASK_17_STATUS.md; the second bullet was
//! resolved by task 24):
//! - natural random-run violations at the pinned seed: `13_properties_safety`
//!   (`no_bad`, run 0) and `41_property_monitors` (`fin_acked_bounded`, run
//!   19) - a legitimate environment behavior, identical in both backends;
//! - all property-bearing examples' generated test executables pass in BOTH
//!   backends. (Task 17 had pinned `13_properties_safety` as failing in both:
//!   its representative-input walk ToBad -> Bad violated its own `no_bad`
//!   property while `test` codegen always includes monitors, D-P3-6. Task 24
//!   fixed the spec - a `Stay` self-loop declared first keeps the walk in
//!   Good; see PHASE_3_TASK_24_STATUS.md.)

use std::collections::{BTreeMap, HashMap};
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};

const BOUND: &str = "8";
const RUNS: &str = "20";
const SEED: &str = "777";
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
    let mut args: Vec<String> = vec![
        // Phase 4: exact mode is the default; these parity tests pin the
        // Phase 3 fuzz behavior (D-P4-7).
        "--mode".into(),
        "fuzz".into(),
        "--bound".into(),
        BOUND.into(),
        "--runs".into(),
        RUNS.into(),
        "--seed".into(),
        SEED.into(),
    ];
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

/// The property-specific counterexample fields printed by both MC runtimes.
/// `trigger_step` is None for plain per-sample checks (no obligation trigger).
#[derive(Debug, Clone, PartialEq, Eq)]
struct McViolation {
    run: usize,
    label: String,
    trigger_step: Option<usize>,
    violation_step: usize,
}

fn parse_mc_violation(stdout: &str) -> Option<McViolation> {
    let mut run = None;
    let mut label = None;
    let mut trigger = None;
    let mut violation = None;
    for line in stdout.lines() {
        let t = line.trim_start();
        if let Some(rest) = t.strip_prefix("run ") {
            if let Some(l) = rest.split_once(" property violation: ") {
                run = l.0.parse().ok();
                label = Some(l.1.trim().to_string());
            }
        } else if let Some(n) = t.strip_prefix("trigger step: ") {
            trigger = n.parse().ok();
        } else if let Some(n) = t.strip_prefix("violation step: ") {
            violation = n.parse().ok();
        }
    }
    match (run, label, trigger, violation) {
        (Some(r), Some(l), t, Some(v)) => Some(McViolation {
            run: r,
            label: l,
            trigger_step: t,
            violation_step: v,
        }),
        _ => None,
    }
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

// ---------------------------------------------------------------------------
// Action 1: same-seed MC runs over all property-bearing examples
// ---------------------------------------------------------------------------

#[test]
fn mc_same_seed_runs_parity_all_property_bearing_examples() {
    for spec in property_bearing_examples() {
        let name = example_name(&spec);
        let rust = run_mc(&spec, "rust", &[]);
        let c = run_mc(&spec, "c", &[]);
        assert_eq!(
            rust.exit_code, c.exit_code,
            "[{name}] exit codes must match (same spec + seed)"
        );
        if rust.exit_code == Some(0) {
            let rs = parse_mc_summary(&rust.stdout)
                .unwrap_or_else(|| panic!("[rust {name}] summary line missing: {}", rust.stdout));
            let cs = parse_mc_summary(&c.stdout)
                .unwrap_or_else(|| panic!("[c {name}] summary line missing: {}", c.stdout));
            assert_eq!(
                rs, cs,
                "[{name}] summary fields must be identical (same spec + seed)"
            );
        } else {
            // Violation path: both backends must have reported a violation;
            // field-by-field comparison happens in the Action 2 test.
            assert!(
                parse_mc_violation(&rust.stdout).is_some()
                    && parse_mc_violation(&c.stdout).is_some(),
                "[{name}] non-zero exit without a property-violation report"
            );
        }
    }
}

// ---------------------------------------------------------------------------
// Action 2: violation parity (natural + forced)
// ---------------------------------------------------------------------------

#[test]
fn mc_violation_parity_natural_and_forced() {
    // Natural violations under the pinned parameters: compare field by field
    // whenever observed, and pin the currently-observed set so a silent change
    // (new violating example, LCG/param drift) fails loudly.
    let mut natural: BTreeMap<String, McViolation> = BTreeMap::new();
    for spec in property_bearing_examples() {
        let name = example_name(&spec);
        let rust = run_mc(&spec, "rust", &[]);
        let c = run_mc(&spec, "c", &[]);
        match (
            parse_mc_violation(&rust.stdout),
            parse_mc_violation(&c.stdout),
        ) {
            (Some(rv), Some(cv)) => {
                assert_eq!(
                    rv, cv,
                    "[{name}] natural violation must be field-identical across backends"
                );
                natural.insert(name, rv);
            }
            (None, None) => {}
            other => panic!("[{name}] violation observed in only ONE backend: {other:?}"),
        }
    }
    // Pinned at --bound 8 --runs 20 --seed 777 (deterministic):
    assert_eq!(
        natural.len(),
        2,
        "natural-violation set changed: {natural:?}"
    );
    assert_eq!(natural["13_properties_safety.rfs"].label, "no_bad");
    assert_eq!(
        natural["41_property_monitors.rfs"].label,
        "fin_acked_bounded"
    );

    // Forced violation in BOTH backends: example 41, property fin_acked_bounded
    // (its bounded obligation can be forced to expire; the other properties of
    // this FSM are unviolable - e.g. Closed is only reachable via an Ack that
    // sets ack_received). Compare the counterexample field by field.
    let examples = property_bearing_examples();
    let ex41 = examples
        .iter()
        .find(|p| example_name(p) == "41_property_monitors.rfs")
        .expect("example 41 present");
    const FORCED_LABEL: &str = "fin_acked_bounded";
    let forced_args = vec!["--violate".to_string(), FORCED_LABEL.to_string()];
    let rust = run_mc(ex41, "rust", &forced_args);
    let c = run_mc(ex41, "c", &forced_args);
    assert_eq!(
        rust.exit_code,
        Some(1),
        "[rust] forced violation must exit 1: {}",
        rust.stdout
    );
    assert_eq!(
        c.exit_code,
        Some(1),
        "[c] forced violation must exit 1: {}",
        c.stdout
    );
    let rv = parse_mc_violation(&rust.stdout)
        .unwrap_or_else(|| panic!("[rust] forced counterexample missing: {}", rust.stdout));
    let cv = parse_mc_violation(&c.stdout)
        .unwrap_or_else(|| panic!("[c] forced counterexample missing: {}", c.stdout));
    assert_eq!(
        rv.label, FORCED_LABEL,
        "forced label must be the requested one"
    );
    assert_eq!(
        rv, cv,
        "forced violation must be field-identical across backends"
    );
    // Pinned (deterministic at the pinned seed): same values as the natural
    // 41-violation - trigger at sample 1, deadline at sample 1 + 5 - 1 = 5.
    assert_eq!(rv.trigger_step, Some(1));
    assert_eq!(rv.violation_step, 5);
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
