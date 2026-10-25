//! Phase 6 task 4, action 1 (subtask 4.1): ref-MC vs generated-MC
//! differential suite.
//!
//! Compares the brute-force reference model checker (`rfs::ref_mc` - an
//! independent implementation of `docs/SEMANTICS.md`, D-P6-5) against BOTH
//! generated exact-mode MC engines over:
//!   * every valid example whose state space fits the budget (the pinned
//!     eligible list below, measured 2026-10-24), and
//!   * 20 small random specs from `rfs::specgen` (pinned seeds).
//!
//! Per-run assertions (PHASE_6_TASK_4.md "Context" - different algorithms,
//! so the comparison is deliberate about what MUST agree):
//!   * status agreement: PassComplete iff PassComplete, Fail iff Fail,
//!     UnknownStateLimit iff UnknownStateLimit;
//!   * `states` EXACT (same reachable set - configuration identity was
//!     aligned in Phase 6 task 3);
//!   * `max_depth` agreement (both use level-synchronous BFS);
//!   * on failure: the engine's label is a member of ref_mc's minimal-depth
//!     violation-label set (`ref_mc::minimal_violation_labels`); for specs
//!     with a UNIQUE minimal label, label+step equality is additionally
//!     asserted (the observed ambiguous set is pinned below);
//!   * CE replay: the engine's counterexample must be a valid execution that
//!     genuinely exhibits the claimed violation at the claimed step, checked
//!     through ref_mc's own evaluators (`ref_mc::verify_counterexample`).
//!
//! `transitions` is deliberately NOT asserted cross-engine (traversal
//! accounting may differ; it IS asserted cross-backend by
//! backend_parity_test).
//!
//! Negative control (the "no false PASS" invariant of Phase 6): a tiny
//! `--max-states` on a large spec must yield UnknownStateLimit in ref_mc AND
//! both generated backends - never PassComplete.

use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::process::Command;
use std::sync::{LazyLock, Mutex};

/// gcc location (plan assumption, verified since Phase 2).
const GCC: &str = "/usr/bin/gcc";

/// State budget for eligible runs (measured 2026-10-24, debug toolchain).
const MAX_STATES: usize = 500_000;
const TIMEOUT_SEC: u64 = 60;

/// Pinned seeds of the 20 small random specs (`specgen::Bounds::small()`).
const RANDOM_SEEDS: [u64; 20] = [
    900_001, 900_002, 900_003, 900_004, 900_005, 900_006, 900_007, 900_008, 900_009, 900_010,
    900_011, 900_012, 900_013, 900_014, 900_015, 900_016, 900_017, 900_018, 900_019, 900_020,
];

/// Eligible-set exclusions: the two examples whose ref_mc run exceeds the
/// 60 s budget even with a generous state cap (measured 2026-10-24, debug
/// build): `19_guard_complex` (one BFS level runs for minutes) and
/// `39_endpoint_protocol` (> 60 s). All other valid examples qualify -
/// including the ~8-9 s benchmark pair `44_token_ring` / `45_elevator`.
const EXCLUDED: &[&str] = &["19_guard_complex.rfs", "39_endpoint_protocol.rfs"];

/// Observed AMBIGUOUS eligible specs (multiple labels tie at minimal depth),
/// measured during suite development; for these only set-membership + CE
/// replay are asserted (never label+step equality). Re-measure if examples
/// change.
const AMBIGUOUS: &[&str] = &[];

fn root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rustc() -> String {
    std::env::var("RUSTC").unwrap_or_else(|_| "rustc".to_string())
}

/// All valid examples except the pinned exclusions (dynamic enumeration, so
/// future additions are covered automatically).
fn eligible_examples() -> Vec<PathBuf> {
    let files = rfs::collect_rfs_files(root().join("examples/valid")).expect("collect .rfs files");
    let out: Vec<PathBuf> = files
        .into_iter()
        .filter(|p| {
            let name = p.file_name().unwrap().to_string_lossy().into_owned();
            !EXCLUDED.contains(&name.as_str())
        })
        .collect();
    assert!(
        out.len() >= 40,
        "eligible set shrank unexpectedly: {}",
        out.len()
    );
    out
}

fn example_name(spec: &Path) -> String {
    spec.file_name().unwrap().to_string_lossy().into_owned()
}

// ---------------------------------------------------------------------------
// Generated-MC-binary generation + compilation (cached per test process)
// ---------------------------------------------------------------------------

static BIN_CACHE: LazyLock<Mutex<HashMap<String, PathBuf>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn cached_binary(key: &str, make: impl FnOnce(&Path) -> PathBuf) -> PathBuf {
    let mut cache = BIN_CACHE.lock().unwrap();
    if let Some(p) = cache.get(key) {
        return p.clone();
    }
    let dir = root().join("target").join("rfs_ref_mc_diff");
    std::fs::create_dir_all(&dir).expect("create scratch dir");
    let path = make(&dir);
    cache.insert(key.to_string(), path.clone());
    path
}

fn generate_mc_json(backend_name: &str, ir: &rfs::Ir) -> String {
    let backend = rfs::codegen::get_backend(backend_name)
        .unwrap_or_else(|| panic!("backend {backend_name} registered"));
    let options = rfs::codegen::BackendOptions {
        counterexample_format: rfs::codegen_mc::CounterexampleFormat::Json,
        ..rfs::codegen::BackendOptions::default()
    };
    rfs::codegen::CodegenCommandKind::ModelChecker
        .run(backend, ir, &options)
        .unwrap_or_else(|e| panic!("MC codegen ({backend_name}) must succeed: {e:?}"))
        .code
}

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

/// Cache/file tag: the example name without its `.rfs` extension (a dot in
/// the file name would derive an invalid Rust crate name).
fn tag_of(name: &str) -> String {
    name.trim_end_matches(".rfs").to_string()
}

fn mc_json_binary(tag: &str, backend_name: &str, ir: &rfs::Ir) -> PathBuf {
    let tag = tag_of(tag);
    cached_binary(&format!("mcjson|{tag}|{backend_name}"), |dir| {
        let ext = if backend_name == "c" { "c" } else { "rs" };
        let src = dir.join(format!("{tag}_mc_json.{ext}"));
        std::fs::write(&src, generate_mc_json(backend_name, ir)).expect("write MC source");
        let bin = dir.join(format!("{tag}_mc_json_{backend_name}"));
        if backend_name == "c" {
            compile_c(&src, &bin);
        } else {
            compile_rust(&src, &bin);
        }
        bin
    })
}

/// Run one generated MC binary (exact mode is the default) and return
/// (exit code, parsed JSON report document).
fn run_mc_json(bin: &Path, extra_args: &[&str]) -> (i32, rfs::json_rw::Json) {
    let out = Command::new(bin)
        .args(extra_args)
        .output()
        .unwrap_or_else(|e| panic!("run MC binary {:?}: {e}", bin));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let doc_text = stdout.strip_suffix('\n').unwrap_or(&stdout);
    let doc = rfs::json_rw::parse(doc_text)
        .unwrap_or_else(|e| panic!("MC JSON must strict-parse: {e}\n{stdout}"));
    (out.status.code().unwrap_or(-1), doc)
}

// ---------------------------------------------------------------------------
// ref_mc side (in-process)
// ---------------------------------------------------------------------------

fn ref_cfg() -> rfs::ref_mc::RefConfig {
    rfs::ref_mc::RefConfig {
        max_states: MAX_STATES,
        timeout_sec: TIMEOUT_SEC,
        assume_arg: "all".to_string(),
    }
}

/// The ref JSON document, rendered through the SAME unified report model as
/// the generated binaries (D-P6-10) - directly comparable.
fn ref_json_doc(module: &rfs::Module, res: &rfs::ref_mc::RefResult) -> rfs::json_rw::Json {
    rfs::ref_mc::to_json_report(res, module).to_json()
}

// ---------------------------------------------------------------------------
// JSON report field extraction
// ---------------------------------------------------------------------------

fn doc_status(doc: &rfs::json_rw::Json) -> String {
    doc.get("status")
        .and_then(|v| v.as_str())
        .unwrap_or("<missing>")
        .to_string()
}

fn doc_fail(doc: &rfs::json_rw::Json) -> Option<(rfs::codegen_mc::FailReason, String, usize)> {
    let reason = doc.get("reason")?.as_str()?;
    let label = doc.get("label")?.as_str()?.to_string();
    let step = doc.get("step")?.as_int()? as usize;
    let r = match reason {
        "property-violation" => rfs::codegen_mc::FailReason::PropertyViolation,
        "liveness-violation" => rfs::codegen_mc::FailReason::LivenessViolation,
        "deadlock" => rfs::codegen_mc::FailReason::Deadlock,
        other => panic!("unknown reason wire name '{other}'"),
    };
    Some((r, label, step))
}

/// One JSON input/output record -> [`rfs::codegen_mc::JsonIo`].
fn json_io(j: &rfs::json_rw::Json, what: &str) -> rfs::codegen_mc::JsonIo {
    let obj = j
        .as_obj()
        .unwrap_or_else(|| panic!("{what}: not an object"));
    let mut variant = String::new();
    let mut fields: Vec<(String, rfs::json_rw::Json)> = Vec::new();
    for (k, v) in obj {
        match k.as_str() {
            "variant" => variant = v.as_str().expect("variant is a string").to_string(),
            "fields" => fields = v.as_obj().expect("fields is an object").to_vec(),
            other => panic!("{what}: unexpected key '{other}'"),
        }
    }
    rfs::codegen_mc::JsonIo { variant, fields }
}

fn json_step(j: &rfs::json_rw::Json) -> rfs::codegen_mc::JsonStep {
    let obj = j.as_obj().expect("step is an object");
    let mut input = None;
    let mut outputs: Vec<rfs::json_rw::Json> = Vec::new();
    let mut from_state = String::new();
    let mut to_state = String::new();
    let mut vars: Vec<(String, rfs::json_rw::Json)> = Vec::new();
    for (k, v) in obj {
        match k.as_str() {
            "input" => input = Some(v.clone()),
            "outputs" => outputs = v.as_arr().expect("outputs is an array").clone(),
            "from_state" => from_state = v.as_str().expect("string").to_string(),
            "to_state" => to_state = v.as_str().expect("string").to_string(),
            "vars" => vars = v.as_obj().expect("vars is an object").to_vec(),
            other => panic!("step: unexpected key '{other}'"),
        }
    }
    rfs::codegen_mc::JsonStep {
        input: json_io(&input.expect("input present"), "step.input"),
        outputs: outputs
            .iter()
            .map(|o| json_io(o, "step.outputs[i]"))
            .collect(),
        from_state,
        to_state,
        vars,
    }
}

/// The counterexample object of a JSON report -> the shared report-model
/// enum (so `ref_mc::verify_counterexample` can consume it).
fn json_ce(doc: &rfs::json_rw::Json) -> rfs::codegen_mc::JsonCounterexample {
    let ce = doc
        .get("counterexample")
        .expect("fail report carries a counterexample");
    let obj = ce.as_obj().expect("counterexample is an object");
    let mut kind = String::new();
    let mut initial_state = String::new();
    let mut initial_vars: Vec<(String, rfs::json_rw::Json)> = Vec::new();
    let mut steps: Vec<rfs::json_rw::Json> = Vec::new();
    let mut prefix: Vec<rfs::json_rw::Json> = Vec::new();
    let mut cycle: Vec<rfs::json_rw::Json> = Vec::new();
    let mut cycle_length = 0i64;
    let mut depth = 0i64;
    let mut state = String::new();
    let mut vars: Vec<(String, rfs::json_rw::Json)> = Vec::new();
    for (k, v) in obj {
        match k.as_str() {
            "kind" => kind = v.as_str().expect("string").to_string(),
            "initial_state" => initial_state = v.as_str().expect("string").to_string(),
            "initial_vars" => initial_vars = v.as_obj().expect("object").to_vec(),
            "steps" => steps = v.as_arr().expect("array").clone(),
            "prefix" => prefix = v.as_arr().expect("array").clone(),
            "cycle" => cycle = v.as_arr().expect("array").clone(),
            "cycle_length" => cycle_length = v.as_int().expect("int"),
            "depth" => depth = v.as_int().expect("int"),
            "state" => state = v.as_str().expect("string").to_string(),
            "vars" => vars = v.as_obj().expect("object").to_vec(),
            other => panic!("counterexample: unexpected key '{other}'"),
        }
    }
    match kind.as_str() {
        "trace" => rfs::codegen_mc::JsonCounterexample::Trace {
            initial_state,
            initial_vars,
            steps: steps.iter().map(json_step).collect(),
        },
        "lasso" => rfs::codegen_mc::JsonCounterexample::Lasso {
            initial_state,
            initial_vars,
            prefix: prefix.iter().map(json_step).collect(),
            cycle: cycle.iter().map(json_step).collect(),
            cycle_length,
        },
        "deadlock-config" => {
            rfs::codegen_mc::JsonCounterexample::DeadlockConfig { depth, state, vars }
        }
        other => panic!("unknown CE kind '{other}'"),
    }
}

// ---------------------------------------------------------------------------
// The per-spec comparison (shared by examples and random specs)
// ---------------------------------------------------------------------------

fn compare_one(name: &str, ir: &rfs::Ir) {
    let module = &ir.modules[0];
    // --- ref_mc ground truth (in-process). ---
    let res = match rfs::ref_mc::run(module, &ref_cfg()) {
        Ok(r) => r,
        Err(e) => {
            // Documented asymmetry (ref_mc::run docs): an assumption already
            // violated at the initial configuration makes the generated
            // binaries panic/abort while ref_mc reports a clean error. Both
            // must name the SAME label; no report is emitted, so nothing
            // else is comparable.
            let cap = "assumption '";
            let Some(start) = e.find(cap) else {
                panic!("[{name}] ref_mc run failed: {e}")
            };
            let rest = &e[start + cap.len()..];
            let end = rest
                .find('\'')
                .unwrap_or_else(|| panic!("[{name}] malformed ref error: {e}"));
            let label = &rest[..end];
            for backend_name in ["rust", "c"] {
                let bin = mc_json_binary(name, backend_name, ir);
                let out = Command::new(&bin)
                    .output()
                    .unwrap_or_else(|e2| panic!("run MC binary {:?}: {e2}", bin));
                assert!(
                    !out.status.success(),
                    "[{name}/{backend_name}] sample-0 assumption violation must fail the run"
                );
                let err = String::from_utf8_lossy(&out.stderr);
                assert!(
                    err.contains(&format!("RFS assumption violated: {label}")),
                    "[{name}/{backend_name}] panic/abort must name the violated assumption:\n{err}"
                );
            }
            return;
        }
    };
    assert_ne!(
        res.status,
        rfs::codegen_mc::ExactStatus::UnknownStateLimit,
        "[{name}] ref_mc hit the state/timeout budget - update the eligible list"
    );

    for backend_name in ["rust", "c"] {
        let bin = mc_json_binary(name, backend_name, ir);
        let (exit, doc) = run_mc_json(&bin, &[]);

        // Status agreement (order-independent; must always hold).
        assert_eq!(
            doc_status(&doc),
            res.status.wire(),
            "[{name}/{backend_name}] status diverges: engine '{}' vs ref '{}'\n{}",
            doc_status(&doc),
            res.status.wire(),
            doc.to_string_compact()
        );
        assert_eq!(
            exit,
            res.status.exit_code(),
            "[{name}/{backend_name}] exit code {exit} != status exit {}",
            res.status.exit_code()
        );

        // Metrics: states EXACT (same reachable set); max_depth (both are
        // level-synchronous BFS). transitions is NOT asserted cross-engine.
        let metrics = doc.get("metrics").expect("[{name}] metrics object missing");
        assert_eq!(
            metrics.get("states").and_then(|v| v.as_int()),
            Some(res.states as i64),
            "[{name}/{backend_name}] states diverge"
        );
        assert_eq!(
            metrics.get("max_depth").and_then(|v| v.as_int()),
            Some(res.max_depth as i64),
            "[{name}/{backend_name}] max_depth diverges"
        );

        // Failure fields + CE replay.
        let engine_fail = doc_fail(&doc);
        match (engine_fail, &res.fail) {
            (None, None) => {}
            (Some((reason, label, step)), Some((rref, lref, sref))) => {
                assert_eq!(reason, *rref, "[{name}/{backend_name}] reason diverges");
                // Label ground truth: the minimal-depth violation set.
                if matches!(reason, rfs::codegen_mc::FailReason::PropertyViolation) {
                    let labels = rfs::ref_mc::minimal_violation_labels(module, &ref_cfg())
                        .unwrap_or_else(|e| panic!("[{name}] minimal labels: {e}"));
                    assert!(
                        !labels.is_empty() && labels.contains(&label),
                        "[{name}/{backend_name}] engine label '{label}' not in ref minimal set {labels:?}"
                    );
                    let ambiguous = AMBIGUOUS.contains(&name);
                    if labels.len() == 1 || !ambiguous {
                        // Unique minimal violation (or a pinned-unambiguous
                        // spec): exact label+step agreement.
                        assert_eq!(
                            &label, lref,
                            "[{name}/{backend_name}] label diverges: engine '{label}' vs ref '{lref}'"
                        );
                        assert_eq!(
                            step, *sref,
                            "[{name}/{backend_name}] step diverges: engine {step} vs ref {sref}"
                        );
                    }
                } else {
                    // Deadlock / liveness: label+step are part of the
                    // ground-truth identity (deadlock label is "-").
                    assert_eq!(&label, lref, "[{name}/{backend_name}] label diverges");
                    assert_eq!(step, *sref, "[{name}/{backend_name}] step diverges");
                }
                // CE replay through ref_mc's independent evaluators (the
                // deadlock CEs use the run's terminal monitor projection to
                // re-derive the SEMANTICS 8.4 pruning judgment).
                let ce = json_ce(&doc);
                rfs::ref_mc::verify_counterexample(
                    module,
                    &ref_cfg(),
                    reason,
                    &label,
                    step,
                    &ce,
                    res.deadlock_mons.as_ref(),
                )
                .unwrap_or_else(|e| panic!("[{name}/{backend_name}] CE replay failed: {e}"));
            }
            (a, b) => {
                panic!("[{name}/{backend_name}] fail presence diverges: engine {a:?} vs ref {b:?}")
            }
        }
    }
}

// ---------------------------------------------------------------------------
// Test 1: all eligible examples
// ---------------------------------------------------------------------------

#[test]
fn ref_mc_vs_generated_all_eligible_examples() {
    for spec in eligible_examples() {
        let name = example_name(&spec);
        let src = std::fs::read_to_string(&spec).unwrap_or_else(|e| panic!("read {spec:?}: {e}"));
        let ir = rfs::parse_source(&src).unwrap_or_else(|e| panic!("{name} must parse: {e}"));
        // Multi-module files (30_multi_module): the codegen command
        // convention drives the FIRST module only - ref and both engines do.
        compare_one(&name, &ir);
    }
}

// ---------------------------------------------------------------------------
// Test 2: 20 small random specs (specgen, pinned seeds)
// ---------------------------------------------------------------------------

#[test]
fn ref_mc_vs_generated_random_specs() {
    for seed in RANDOM_SEEDS {
        let mut gen = rfs::specgen::SpecGen::new(seed, rfs::specgen::Bounds::small());
        let ir = gen.generate();
        // specgen guarantees parseability (it renders via the printer) and
        // validity; re-assert both as a defense against drift.
        let src = rfs::print_ir(&ir);
        let reparsed = rfs::parse_source(&src).unwrap_or_else(|e| panic!("seed {seed}: {e}"));
        assert_eq!(reparsed, ir, "seed {seed}: printer round-trip drift");
        let name = format!("rand_{seed}");
        compare_one(&name, &ir);
    }
}

// ---------------------------------------------------------------------------
// Test 3: negative control - no false PASS (complete)
// ---------------------------------------------------------------------------

#[test]
fn tiny_state_limit_never_pass_complete() {
    // 44_token_ring explores ~26k configurations; a cap of 1000 must stop all
    // three engines in UnknownStateLimit - never a completeness claim.
    let spec = root().join("examples/valid/44_token_ring.rfs");
    let src = std::fs::read_to_string(&spec).expect("read 44_token_ring");
    let ir = rfs::parse_source(&src).expect("parse");
    let module = &ir.modules[0];

    let res = rfs::ref_mc::run(
        module,
        &rfs::ref_mc::RefConfig {
            max_states: 1000,
            timeout_sec: TIMEOUT_SEC,
            assume_arg: "all".to_string(),
        },
    )
    .expect("ref run");
    assert_eq!(res.status, rfs::codegen_mc::ExactStatus::UnknownStateLimit);

    for backend_name in ["rust", "c"] {
        let bin = mc_json_binary("44_token_ring.rfs", backend_name, &ir);
        let (exit, doc) = run_mc_json(&bin, &["--max-states", "1000"]);
        assert_eq!(
            doc_status(&doc),
            "unknown-state-limit",
            "[{backend_name}] tiny cap must not claim completeness"
        );
        assert_ne!(doc_status(&doc), "pass-complete");
        assert_eq!(exit, 2);
    }
}

// ---------------------------------------------------------------------------
// Test 4: the rfs mc-ref CLI itself (exit codes + stdout vs in-process)
// ---------------------------------------------------------------------------

fn rfs_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs")
}

#[test]
fn mc_ref_cli_matches_in_process_and_generated_engine() {
    // One PASS spec and one FAIL spec pin all three relevant CLI behaviors:
    // exit codes mirror the generated binaries; JSON stdout is the unified
    // report (byte-comparable with the engines').
    let cases: [(&str, i32); 2] = [
        ("examples/valid/43_sliding_window.rfs", 0),
        ("examples/valid/13_properties_safety.rfs", 1),
    ];
    for (rel, want_exit) in cases {
        let path = root().join(rel);
        let src = std::fs::read_to_string(&path).expect("read spec");
        let ir = rfs::parse_source(&src).expect("parse");
        let module = &ir.modules[0];

        // In-process ground truth.
        let res = rfs::ref_mc::run(module, &ref_cfg()).expect("ref run");
        assert_eq!(want_exit, res.status.exit_code(), "pinned exit for {rel}");
        let want_doc = ref_json_doc(module, &res).to_string_compact();

        // The actual CLI binary.
        let out = Command::new(rfs_bin())
            .args(["mc-ref", "--counterexample-format", "json"])
            .arg(&path)
            .output()
            .expect("spawn rfs mc-ref");
        assert_eq!(
            out.status.code(),
            Some(want_exit),
            "rfs mc-ref {rel}: exit code diverges\nstderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout).to_string();
        assert_eq!(
            stdout.strip_suffix('\n').unwrap_or(&stdout),
            want_doc,
            "rfs mc-ref {rel}: CLI JSON diverges from in-process render"
        );

        // And it must agree with the generated engines' JSON on every field
        // the comparison above asserts (byte-identity is NOT required: the
        // traversal may report different-but-equivalent CEs on ambiguous
        // specs - 13 is unique, so here full byte agreement is expected and
        // pinned as the strongest form).
        for backend_name in ["rust", "c"] {
            let bin = mc_json_binary(
                path.file_name().unwrap().to_string_lossy().as_ref(),
                backend_name,
                &ir,
            );
            let (_, doc) = run_mc_json(&bin, &[]);
            assert_eq!(
                doc.to_string_compact(),
                want_doc,
                "[{rel}/{backend_name}] engine JSON diverges from ref_mc"
            );
        }
    }
}
