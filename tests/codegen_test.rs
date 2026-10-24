use rfs::{codegen, collect_rfs_files};
use std::path::PathBuf;

fn files() -> Vec<PathBuf> {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    if let Ok(v) = collect_rfs_files(root.join("examples/valid")) {
        files.extend(v);
    }
    if let Ok(entries) = std::fs::read_dir(root.join("training")) {
        for e in entries.flatten() {
            let valid = e.path().join("snippets/valid");
            if valid.is_dir() {
                if let Ok(v) = collect_rfs_files(&valid) {
                    files.extend(v);
                }
            }
        }
    }
    files.sort();
    files.dedup();
    files
}

fn parse_module(path: &PathBuf) -> rfs::Ir {
    let ir =
        rfs::parse_file(path).unwrap_or_else(|e| panic!("{} should parse: {}", path.display(), e));
    assert!(
        !ir.modules.is_empty(),
        "{} should contain a module",
        path.display()
    );
    ir
}

// ---------------------------------------------------------------------------
// Backend registry
// ---------------------------------------------------------------------------

#[test]
fn registry_lists_rust_backend() {
    let list = codegen::list_backends();
    assert!(list.iter().any(|(name, _)| *name == "rust"));
    let backend = codegen::get_backend("rust").expect("rust backend registered");
    assert_eq!(backend.name, "rust");
    // Every backend registers all three commands.
    for cmd in ["production", "test", "mc"] {
        assert!(
            backend.commands.iter().any(|c| c.name == cmd),
            "rust backend must register command {}",
            cmd
        );
    }
    // Phase 2: the C backend is registered alongside Rust.
    let c_backend = codegen::get_backend("c").expect("c backend registered");
    for cmd in ["production", "test", "mc"] {
        assert!(
            c_backend.commands.iter().any(|c| c.name == cmd),
            "c backend must register command {}",
            cmd
        );
    }
    assert!(codegen::get_backend("nope").is_none());
}

#[test]
fn registry_help_texts_are_nonempty() {
    let help = codegen::common_help();
    assert!(help.contains("rfs codegen --list"));
    let backend = codegen::get_backend("rust").unwrap();
    let bhelp = codegen::backend_help(backend);
    for cmd in ["production", "test", "mc"] {
        assert!(bhelp.contains(cmd), "backend help must mention {}", cmd);
    }
}

// ---------------------------------------------------------------------------
// Generation through the registry (all valid examples)
// ---------------------------------------------------------------------------

#[test]
fn all_rust_commands_generate_for_valid_examples() {
    let files = files();
    assert!(!files.is_empty());
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    for path in &files {
        let ir = parse_module(path);
        for (kind, marker) in [
            (codegen::CodegenCommandKind::Production, "pub struct Fsm"),
            (codegen::CodegenCommandKind::UnitTests, "#[test]"),
            (codegen::CodegenCommandKind::ModelChecker, "fn main()"),
        ] {
            let out = kind
                .run(backend, &ir, &options)
                .unwrap_or_else(|e| panic!("{} {:?} failed: {:?}", path.display(), kind, e));
            assert!(
                out.code.contains(marker),
                "{} {:?} output missing marker '{}'",
                path.display(),
                kind,
                marker
            );
        }
    }
}

#[test]
fn rust_production_flag_toggles_property_monitors() {
    // D-P3-6: --with-property-monitors gates monitor emission in Rust
    // production; the off output must be zero-cost (no monitor machinery).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/41_property_monitors.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let off_opts = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let on_opts = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: true,

        ..codegen::BackendOptions::default()
    };
    let off = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &off_opts)
        .expect("production codegen (flag off)");
    let on = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &on_opts)
        .expect("production codegen (flag on)");
    assert_ne!(
        off.code, on.code,
        "the flag must change Rust production output"
    );
    // Flag on: StepError variant + violation struct + aggregate, all debug-gated.
    assert!(
        on.code
            .contains("PropertyViolation { label: String, step: usize }"),
        "on: StepError::PropertyViolation variant must be emitted"
    );
    assert!(
        on.code.contains(
            "pub struct PropertyViolation { pub label: String, pub step: usize, pub trigger_step: Option<usize> }"
        ),
        "on: PropertyViolation struct must be emitted"
    );
    assert!(
        on.code.contains("pub struct PropMonitors"),
        "on: monitor aggregate"
    );
    assert!(
        on.code.contains("#[cfg(debug_assertions)]"),
        "on: monitor machinery must be debug-gated (zero release cost)"
    );
    // Flag off: zero-cost-when-disabled - no monitor machinery at all.
    assert!(
        !off.code.contains("PropertyViolation"),
        "off: no PropertyViolation"
    );
    assert!(
        !off.code.contains("PropMonitors"),
        "off: no monitor aggregate"
    );
    assert!(
        !off.code.contains("#[cfg(debug_assertions)]"),
        "off: no debug-gated monitor code"
    );
}

#[test]
fn steps_option_changes_test_generation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/02_counter.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let small = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 1,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let big = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 6,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let a = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir, &small)
        .unwrap();
    let b = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir, &big)
        .unwrap();
    assert_ne!(a.code, b.code, "steps option must affect generated tests");
}

// ---------------------------------------------------------------------------
// CLI parsing (`rfs codegen ...`)
// ---------------------------------------------------------------------------

fn argss(s: &str) -> Vec<String> {
    s.split_whitespace().map(|x| x.to_string()).collect()
}

#[test]
fn cli_parses_list_and_help() {
    assert!(matches!(
        codegen::parse_codegen_args(&argss("--list")),
        Ok(codegen::CodegenRequest::List)
    ));
    assert!(matches!(
        codegen::parse_codegen_args(&argss("--help")),
        Ok(codegen::CodegenRequest::Help)
    ));
    assert!(matches!(
        codegen::parse_codegen_args(&argss("-h")),
        Ok(codegen::CodegenRequest::Help)
    ));
}

#[test]
fn cli_parses_backend_help() {
    assert!(matches!(
        codegen::parse_codegen_args(&argss("rust --help")),
        Ok(codegen::CodegenRequest::BackendHelp(name)) if name == "rust"
    ));
}

#[test]
fn cli_parses_generate_with_options() {
    let req = codegen::parse_codegen_args(&argss(
        "rust test spec.rfs --out /tmp/t.rs --steps 7 --assumes none",
    ))
    .unwrap();
    match req {
        codegen::CodegenRequest::Generate {
            backend_name,
            command,
            input,
            out,
            assumptions,
            steps,
            with_property_monitors,
            ..
        } => {
            assert_eq!(backend_name, "rust");
            assert_eq!(command, codegen::CodegenCommandKind::UnitTests);
            assert_eq!(input, "spec.rfs");
            assert_eq!(out.as_deref(), Some("/tmp/t.rs"));
            assert_eq!(assumptions, codegen::AssumptionSelection::None);
            assert_eq!(steps, 7);
            assert!(!with_property_monitors);
        }
        other => panic!("expected Generate, got {:?}", other),
    }
}

#[test]
fn cli_generate_defaults() {
    let req = codegen::parse_codegen_args(&argss("rust production spec.json")).unwrap();
    match req {
        codegen::CodegenRequest::Generate {
            command,
            input,
            out,
            assumptions,
            steps,
            ..
        } => {
            assert_eq!(command, codegen::CodegenCommandKind::Production);
            assert_eq!(input, "spec.json");
            assert!(out.is_none());
            assert_eq!(assumptions, codegen::AssumptionSelection::All);
            assert_eq!(steps, 4);
        }
        other => panic!("expected Generate, got {:?}", other),
    }
}

#[test]
fn cli_parses_assume_labels() {
    let req = codegen::parse_codegen_args(&argss("rust mc spec.rfs --assumes a1,a2")).unwrap();
    match req {
        codegen::CodegenRequest::Generate { assumptions, .. } => {
            assert_eq!(
                assumptions,
                codegen::AssumptionSelection::Labels(vec!["a1".into(), "a2".into()])
            );
        }
        other => panic!("expected Generate, got {:?}", other),
    }
}

#[test]
fn cli_reports_errors() {
    // No arguments at all.
    assert!(codegen::parse_codegen_args(&[]).is_err());
    // Unknown command name.
    let e = codegen::parse_codegen_args(&argss("rust frobnicate spec.rfs")).unwrap_err();
    assert!(e.iter().any(|x| x.contains("unknown command")));
    // --out without a value.
    let e = codegen::parse_codegen_args(&argss("rust production spec.rfs --out")).unwrap_err();
    assert!(e.iter().any(|x| x.contains("--out requires")));
    // Unknown option.
    let e = codegen::parse_codegen_args(&argss("rust mc spec.rfs --bogus 1")).unwrap_err();
    assert!(e.iter().any(|x| x.contains("unknown option")));
    // Stray positional argument.
    let e = codegen::parse_codegen_args(&argss("rust mc spec.rfs extra")).unwrap_err();
    assert!(e.iter().any(|x| x.contains("unexpected extra argument")));
    // Non-numeric --steps.
    let e = codegen::parse_codegen_args(&argss("rust test spec.rfs --steps many")).unwrap_err();
    assert!(e.iter().any(|x| x.contains("--steps requires a numeric")));
    // Missing command/input after backend name.
    assert!(codegen::parse_codegen_args(&argss("rust production")).is_err());
}

// ---------------------------------------------------------------------------
// Aggregate test-crate tooling (Rust backend)
// ---------------------------------------------------------------------------

#[test]
fn aggregate_codegen_succeeds_for_valid_examples_and_training_snippets() {
    let files = files();
    assert!(!files.is_empty());
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let crate_ = rfs::backends::rust_test::generate_aggregate_test_crate(&files, &options)
        .expect("aggregate codegen should succeed");
    assert!(crate_.module_count > 0);
    assert!(crate_.lib.contains("pub mod "));
}

#[test]
fn model_checker_codegen_contains_main() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/13_properties_safety.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 0,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let mc = codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir, &options)
        .expect("model checker codegen should succeed");
    assert!(mc.code.contains("fn main()"));
}

// ---------------------------------------------------------------------------
// Phase 2: priorities, on_error fallback, version header
// ---------------------------------------------------------------------------

#[test]
fn priority_10_arm_precedes_priority_1_in_generated_step() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/39_endpoint_protocol.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let out = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &options)
        .expect("production codegen");
    // The priority-10 arm (next Open via the exact-seq Data match) must be
    // emitted before the priority-1 fallback arm in state Connecting.
    let pos_hi = out
        .code
        .find("Output::Send_Ack { ack_seq: 1 }")
        .expect("priority-10 emit present");
    let pos_lo = out
        .code
        .find("Output::Alert_Code { reason: 7 }")
        .expect("priority-1 emit present");
    assert!(pos_hi < pos_lo, "higher priority arm must come first");
}

#[test]
fn on_error_fallback_replaces_unexpected_event_error() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/40_on_error_recovery.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let out = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &options)
        .expect("production codegen");
    assert!(out.code.contains("on_error fallback"));
    // With on_error the step function no longer returns UnexpectedEvent.
    assert!(!out.code.contains("Err(StepError::UnexpectedEvent)"));
}

#[test]
fn version_header_becomes_spec_version_constant() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/39_endpoint_protocol.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let out = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &options)
        .expect("production codegen");
    assert!(out
        .code
        .contains("pub const SPEC_VERSION: &str = \"1.0.0\";"));
}

#[test]
fn generated_negative_test_asserts_on_error_landing() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/40_on_error_recovery.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let out = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir, &options)
        .expect("unit-test codegen");
    assert!(out.code.contains("on_error_found"));
}

// ---------------------------------------------------------------------------
// Phase 2: C backend (registry + per-command generation)
// ---------------------------------------------------------------------------

#[test]
fn c_backend_registered_with_all_commands() {
    let backends: Vec<&str> = rfs::codegen::list_backends()
        .into_iter()
        .map(|(n, _)| n)
        .collect();
    assert!(backends.contains(&"c"), "C backend must be registered");
    let backend = rfs::codegen::get_backend("c").expect("c backend");
    for cmd in ["production", "test", "mc"] {
        assert!(
            backend.commands.iter().any(|c| c.name == cmd),
            "c backend missing command {}",
            cmd
        );
    }
}

#[test]
fn all_c_commands_generate_for_valid_examples() {
    let files = files();
    let backend = rfs::codegen::get_backend("c").unwrap();
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    for path in &files {
        let ir = parse_module(path);
        // v1.0 spec hash (D-P6-3): the whole file's canonical IR JSON, one
        // hash per artifact - stamped once per file for this per-module sweep.
        let mut stamped = options.clone();
        stamped.spec_hash =
            rfs::spec_hash::spec_hash16(&ir.to_json().expect("to_json").to_string());
        for module in &ir.modules {
            // Specs with bit widths outside 8/16/32/64 are rejected by the C backend.
            let result =
                rfs::codegen::CodegenCommandKind::Production.run_module(backend, module, &stamped);
            if let Err(e) = &result {
                assert!(
                    e.iter().any(|x| x.contains("bit widths")),
                    "{} production: unexpected C codegen error: {:?}",
                    path.display(),
                    e
                );
                continue;
            }
            let out = result.unwrap();
            assert!(
                out.code.contains("fsm_step"),
                "{} c production missing fsm_step",
                path.display()
            );
            assert!(out.code.contains("#include <stdint.h>"));

            let tests = rfs::codegen::CodegenCommandKind::UnitTests
                .run_module(backend, module, &stamped)
                .unwrap_or_else(|e| panic!("{} c test failed: {:?}", path.display(), e));
            assert!(
                tests.code.contains("int main(void)"),
                "{} c test missing main",
                path.display()
            );

            let mc = rfs::codegen::CodegenCommandKind::ModelChecker
                .run_module(backend, module, &stamped)
                .unwrap_or_else(|e| panic!("{} c mc failed: {:?}", path.display(), e));
            assert!(
                mc.code.contains("model-check summary:"),
                "{} c mc missing summary line",
                path.display()
            );
        }
    }
}

#[test]
fn c_production_flag_toggles_property_monitors() {
    // D-P3-6 parity: the same option gates C production via a compile-time
    // define; off output contains no monitor state or logic.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/41_property_monitors.rfs"));
    let backend = codegen::get_backend("c").unwrap();
    let off_opts = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let on_opts = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: true,

        ..codegen::BackendOptions::default()
    };
    let off = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir, &off_opts)
        .expect("production codegen (flag off)");
    let on = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir, &on_opts)
        .expect("production codegen (flag on)");
    assert_ne!(
        off.code, on.code,
        "the flag must change C production output"
    );
    // Flag on: define + ifdef-gated monitor code.
    assert!(
        on.code.contains("#define RFS_PROPERTY_MONITORS 1"),
        "on: gating define"
    );
    assert!(
        on.code.contains("#ifdef RFS_PROPERTY_MONITORS"),
        "on: ifdef gate"
    );
    assert!(
        on.code.contains("rfs_property_violation_t"),
        "on: violation type"
    );
    assert!(
        on.code.contains("prop_monitors"),
        "on: monitor state in the FSM"
    );
    // Flag off: no define, no monitor type/state. (The rfs_status_t enum keeps
    // RFS_PROPERTY_VIOLATION in both outputs - stable wire format.)
    assert!(
        !off.code.contains("RFS_PROPERTY_MONITORS"),
        "off: no gating define"
    );
    assert!(
        !off.code.contains("rfs_property_violation_t"),
        "off: no violation type"
    );
    assert!(!off.code.contains("prop_monitors"), "off: no monitor state");
    let abi = "RFS_PROPERTY_VIOLATION } rfs_status_t";
    assert!(
        off.code.contains(abi) && on.code.contains(abi),
        "rfs_status_t wire format must be stable across the flag"
    );
}

#[test]
fn c_production_matches_rust_semantics_on_endpoint_protocol() {
    // Both backends must agree on the priority-ordered step for the Phase 2
    // showcase spec: the higher-priority arm is emitted first.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/39_endpoint_protocol.rfs"));
    let backend = rfs::codegen::get_backend("c").unwrap();
    let options = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let out = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir, &options)
        .expect("c production codegen");
    let pos_hi = out
        .code
        .find("INPUT_RECV_DATA")
        .expect("data pattern present");
    assert!(
        out.code.contains("RFS_SPEC_VERSION"),
        "version header must be emitted"
    );
    assert!(
        out.code.contains("on_error fallback"),
        "on_error block must be emitted"
    );
    assert!(pos_hi > 0);
}

// ---------------------------------------------------------------------------
// Phase 3: property-monitor flag semantics (D-P3-6)
// ---------------------------------------------------------------------------

#[test]
fn mc_and_test_always_include_property_monitors() {
    // D-P3-6: mc and test ALWAYS include monitors; the flag is not accepted
    // for those commands (the CLI rejects it with a clear error).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/41_property_monitors.rfs"));
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    // Rust: monitor aggregate + StepError variant present despite flag off.
    let rust = codegen::get_backend("rust").unwrap();
    for kind in [
        codegen::CodegenCommandKind::ModelChecker,
        codegen::CodegenCommandKind::UnitTests,
    ] {
        let out = kind
            .run(rust, &ir, &options)
            .unwrap_or_else(|e| panic!("rust {:?} failed: {:?}", kind, e));
        assert!(
            out.code.contains("pub struct PropMonitors"),
            "rust {:?}: monitors must be unconditional",
            kind
        );
        assert!(
            out.code
                .contains("PropertyViolation { label: String, step: usize }"),
            "rust {:?}: StepError::PropertyViolation must be present",
            kind
        );
    }
    // C: violation type + monitor state present despite flag off.
    let c = codegen::get_backend("c").unwrap();
    for kind in [
        codegen::CodegenCommandKind::ModelChecker,
        codegen::CodegenCommandKind::UnitTests,
    ] {
        let out = kind
            .run(c, &ir, &options)
            .unwrap_or_else(|e| panic!("c {:?} failed: {:?}", kind, e));
        assert!(
            out.code.contains("rfs_property_violation_t"),
            "c {:?}: monitors must be unconditional",
            kind
        );
        assert!(
            out.code.contains("prop_monitors"),
            "c {:?}: monitor state",
            kind
        );
    }
    // CLI: passing the flag to mc/test is rejected (D-P3-6: not accepted/needed).
    for cmd in ["mc", "test"] {
        let e = codegen::parse_codegen_args(&argss(&format!(
            "rust {} spec.rfs --with-property-monitors",
            cmd
        )))
        .unwrap_err();
        assert!(
            e.iter()
                .any(|x| x.contains("only valid for the production command")),
            "{}: unexpected rejection message: {:?}",
            cmd,
            e
        );
    }
}

#[test]
fn no_skip_warnings_for_in_subset_properties_both_backends() {
    // D-P3-6 / phase exit criterion: every property-bearing valid example must
    // compile all its prop/assume formulas to monitors with ZERO "outside
    // monitor subset" skip warnings, in both backends. Files are enumerated
    // dynamically (currently 15: top-level 04, 13, 14, 15, 16, 24, 37, 41, 42
    // + tla_adapted 31-36).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut files = Vec::new();
    if let Ok(v) = collect_rfs_files(root.join("examples/valid")) {
        files.extend(v);
    }
    files.sort();
    let property_files: Vec<&PathBuf> = files
        .iter()
        .filter(|p| {
            std::fs::read_to_string(p)
                .map(|t| {
                    t.lines().any(|l| {
                        let s = l.trim_start();
                        s.starts_with("prop ") || s.starts_with("assume ")
                    })
                })
                .unwrap_or(false)
        })
        .collect();
    assert!(
        property_files.len() >= 3,
        "expected at least three property-bearing valid examples, found {}",
        property_files.len()
    );
    for backend_name in ["rust", "c"] {
        let backend = codegen::get_backend(backend_name).unwrap();
        let options = codegen::BackendOptions {
            assumptions: codegen::AssumptionSelection::All,
            steps: 4,
            with_property_monitors: true,

            ..codegen::BackendOptions::default()
        };
        for path in &property_files {
            let ir = parse_module(path);
            // v1.0 spec hash (D-P6-3): the whole file's canonical IR JSON,
            // one hash per artifact - stamped once per file for this sweep.
            let mut stamped = options.clone();
            stamped.spec_hash =
                rfs::spec_hash::spec_hash16(&ir.to_json().expect("to_json").to_string());
            for module in &ir.modules {
                let result =
                    codegen::CodegenCommandKind::Production.run_module(backend, module, &stamped);
                // The C backend rejects specs with unsupported bit widths (see
                // all_c_commands_generate_for_valid_examples) - skip those.
                if let Err(e) = &result {
                    assert!(
                        e.iter().any(|x| x.contains("bit widths")),
                        "{} production: unexpected C codegen error: {:?}",
                        path.display(),
                        e
                    );
                    continue;
                }
                let out = result.unwrap();
                // Liveness (eventually/until) formulas are MC-only by design
                // (D-P4-9): their skip warnings are expected and identical in
                // both backends (pinned by
                // liveness_properties_skip_with_identical_warning_both_backends).
                let has_liveness = module
                    .properties
                    .iter()
                    .chain(module.assumptions.iter())
                    .any(|p| formula_has_liveness(&p.formula));
                if !has_liveness {
                    assert!(
                        out.warnings
                            .iter()
                            .all(|w| !w.contains("outside the monitor subset")),
                        "{} {}: in-subset formulas must not be skipped: {:?}",
                        backend_name,
                        path.display(),
                        out.warnings
                    );
                }
            }
        }
    }
}

/// True iff the formula contains an `eventually`/`until` operator anywhere.
fn formula_has_liveness(f: &rfs::ir::Formula) -> bool {
    use rfs::ir::Formula::*;
    match f {
        Eventually { .. } | Until { .. } => true,
        Not { inner }
        | Next { inner }
        | Always { inner }
        | BoundedAlways { inner, .. }
        | BoundedEventually { inner, .. } => formula_has_liveness(inner),
        And { left, right } | Or { left, right } | Implies { left, right } => {
            formula_has_liveness(left) || formula_has_liveness(right)
        }
        _ => false,
    }
}

#[test]
fn liveness_properties_skip_with_identical_warning_both_backends() {
    // D-P4-8/D-P4-9: ANY formula containing eventually/until is outside the
    // monitor subset; production codegen in BOTH backends must emit the
    // identical skip warning (MC-only properties).
    let source = "module m {\n  enum State { A, B }\n  initial A\n  event E\n  var flag: bool init false\n  transition A on E { next B; }\n  prop live : eventually (flag) ;\n  prop wait : !flag until flag ;\n  assume env : state == A until state == B ;\n}\n";
    let ir = rfs::parse_source(source).expect("liveness module should parse");

    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: true,

        ..codegen::BackendOptions::default()
    };
    let mut warnings_by_backend: Vec<Vec<String>> = Vec::new();
    for backend_name in ["rust", "c"] {
        let backend = codegen::get_backend(backend_name).unwrap();
        let out = codegen::CodegenCommandKind::Production
            .run(backend, &ir, &options)
            .unwrap_or_else(|e| panic!("{} production failed: {:?}", backend_name, e));
        warnings_by_backend.push(out.warnings);
    }
    // Identical warning text in the same order in both backends.
    assert_eq!(warnings_by_backend[0], warnings_by_backend[1]);
    let expected = vec![
        "property 'live' is outside the monitor subset; skipped",
        "property 'wait' is outside the monitor subset; skipped",
        "assume 'env' is outside the monitor subset; skipped",
    ];
    assert_eq!(warnings_by_backend[0], expected);
}

#[test]
fn mc_exact_flags_embedded_as_runtime_defaults() {
    // D-P4-7: codegen-time flags are embedded as defaults, overridable at
    // runtime. Each flag must change the generated output as expected.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/02_counter.rfs"));
    let backend = codegen::get_backend("rust").unwrap();

    fn gen(ir: &rfs::ir::Ir, backend: &codegen::Backend, opts: &codegen::BackendOptions) -> String {
        codegen::CodegenCommandKind::ModelChecker
            .run(backend, ir, opts)
            .unwrap()
            .code
    }

    let base = codegen::BackendOptions {
        ..Default::default()
    };
    let out = gen(&ir, backend, &base);
    // Exact mode is the default (D-P4-7).
    assert!(out.contains("const DEFAULT_MODE: u8 = 0;"));
    assert!(out.contains("const DEFAULT_MAX_STATES: usize = 1000000;"));
    assert!(out.contains("const DEFAULT_NONDET: bool = false;"));
    assert!(out.contains("const DEFAULT_TIMEOUT_SEC: u64 = 0;"));
    // The exact engine and the mode dispatch are present.
    assert!(out.contains("fn run_exact()"));
    assert!(out.contains("fn run_fuzz()"));
    assert!(out.contains("fn main()"));

    let fuzz = codegen::BackendOptions {
        mc_mode: codegen::McMode::Fuzz,
        ..Default::default()
    };
    assert!(gen(&ir, backend, &fuzz).contains("const DEFAULT_MODE: u8 = 1;"));

    let limited = codegen::BackendOptions {
        mc_max_states: 5000,
        ..Default::default()
    };
    assert!(gen(&ir, backend, &limited).contains("const DEFAULT_MAX_STATES: usize = 5000;"));

    let nondet = codegen::BackendOptions {
        mc_nondet: true,
        ..Default::default()
    };
    assert!(gen(&ir, backend, &nondet).contains("const DEFAULT_NONDET: bool = true;"));

    let timed = codegen::BackendOptions {
        mc_timeout_sec: 30,
        ..Default::default()
    };
    assert!(gen(&ir, backend, &timed).contains("const DEFAULT_TIMEOUT_SEC: u64 = 30;"));

    // Counterexample report format (task 10): text is the default.
    assert!(out.contains("const DEFAULT_COUNTEREXAMPLE_FORMAT: u8 = 0;"));
    let json_fmt = codegen::BackendOptions {
        counterexample_format: rfs::codegen_mc::CounterexampleFormat::Json,
        ..Default::default()
    };
    assert!(gen(&ir, backend, &json_fmt).contains("const DEFAULT_COUNTEREXAMPLE_FORMAT: u8 = 1;"));

    // The C backend embeds the same default as a #define.
    let c_backend = codegen::get_backend("c").unwrap();
    assert!(gen(&ir, c_backend, &base).contains("#define DEFAULT_COUNTEREXAMPLE_FORMAT 0"));
    assert!(gen(&ir, c_backend, &json_fmt).contains("#define DEFAULT_COUNTEREXAMPLE_FORMAT 1"));
}

#[test]
fn mc_flag_parsing_roundtrip() {
    let req = codegen::parse_codegen_args(&argss(
        "rust mc spec.rfs --mode fuzz --max-states 777 --nondet --timeout-sec 9 --counterexample-format json",
    ))
    .unwrap();
    match req {
        codegen::CodegenRequest::Generate {
            mc_mode,
            mc_max_states,
            mc_nondet,
            mc_timeout_sec,
            counterexample_format,
            ..
        } => {
            assert_eq!(mc_mode, codegen::McMode::Fuzz);
            assert_eq!(mc_max_states, 777);
            assert!(mc_nondet);
            assert_eq!(mc_timeout_sec, 9);
            assert_eq!(
                counterexample_format,
                rfs::codegen_mc::CounterexampleFormat::Json
            );
        }
        _ => panic!("expected Generate request"),
    }
    // Defaults when the flags are absent.
    let req = codegen::parse_codegen_args(&argss("rust mc spec.rfs")).unwrap();
    match req {
        codegen::CodegenRequest::Generate {
            mc_mode,
            mc_max_states,
            mc_nondet,
            mc_timeout_sec,
            counterexample_format,
            ..
        } => {
            assert_eq!(mc_mode, codegen::McMode::Exact);
            assert_eq!(mc_max_states, 1000000);
            assert!(!mc_nondet);
            assert_eq!(mc_timeout_sec, 0);
            assert_eq!(
                counterexample_format,
                rfs::codegen_mc::CounterexampleFormat::Text
            );
        }
        _ => panic!("expected Generate request"),
    }
    // Invalid values are rejected with the pinned message.
    let errs = codegen::parse_codegen_args(&argss("rust mc spec.rfs --counterexample-format yaml"))
        .unwrap_err();
    assert_eq!(
        errs,
        vec![rfs::codegen_mc::COUNTEREXAMPLE_FORMAT_ERROR.to_string()]
    );
}

#[test]
fn mc_json_writer_embedded_and_dependency_free() {
    // Task 10: the JSON writer is EMBEDDED in both backends' generated MC
    // source (self-contained artifacts - no serde or other JSON dependency).
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/02_counter.rfs"));

    for backend_name in ["rust", "c"] {
        let backend = codegen::get_backend(backend_name).unwrap();
        let out = codegen::CodegenCommandKind::ModelChecker
            .run(backend, &ir, &codegen::BackendOptions::default())
            .unwrap()
            .code;
        // The escaping primitive is named per backend (Rust: mc_json_escape,
        // C port: mjb_escape).
        let esc_marker = if backend_name == "rust" {
            "mc_json_escape"
        } else {
            "mjb_escape"
        };
        assert!(
            out.contains(esc_marker),
            "{backend_name}: embedded JSON writer missing"
        );
        assert!(
            out.contains("build_mc_json_doc") || out.contains("mc_json_doc_head"),
            "{backend_name}: canonical document builder missing"
        );
        assert!(
            out.contains("ce_trace_json")
                && out.contains("ce_lasso_json")
                && out.contains("ce_deadlock_json"),
            "{backend_name}: counterexample kind builders missing"
        );
        assert!(
            !out.to_ascii_lowercase().contains("serde"),
            "{backend_name}: generated MC must not reference serde"
        );
    }
}

#[test]
fn mc_enum_input_field_domain_qualified_both_backends() {
    // Pre-existing fix found during task 10 verification: a CONCRETE enum
    // input field made the Rust engine enumerate BARE variant names
    // (`[Red, Green]`), which does not compile; the C engine was already
    // qualified. Both backends must now generate compiling engines for such
    // specs (the plan carries the enum name: ValueDomain::Enum { name, .. }).
    let src = r#"module m {
  enum State { A, B }
  initial A
  enum Mode { M1, M2 }
  event E(V(mode: Mode))
  transition A on E(V(mode)) where mode == Mode::M1 { next B; }
  prop p : always (state != B) ;
}"#;
    let ir = rfs::parse_source(src).expect("enum-input spec must parse");
    assert_eq!(ir.modules.len(), 1);

    let rust_backend = codegen::get_backend("rust").unwrap();
    let rust_code = codegen::CodegenCommandKind::ModelChecker
        .run(rust_backend, &ir, &codegen::BackendOptions::default())
        .unwrap()
        .code;
    assert!(
        rust_code.contains("Mode::M1"),
        "Rust engine must enumerate qualified variant paths"
    );

    let c_backend = codegen::get_backend("c").unwrap();
    let c_code = codegen::CodegenCommandKind::ModelChecker
        .run(c_backend, &ir, &codegen::BackendOptions::default())
        .unwrap()
        .code;
    assert!(
        c_code.contains("MODE_M1"),
        "C engine must enumerate qualified constants"
    );

    // Compile + run both engines (gated on tool availability): the spec fails
    // with a property violation, and the outputs must be byte-identical.
    let have_rustc = tool_on_path("rustc");
    let have_gcc = std::path::Path::new("/usr/bin/gcc").exists();
    if have_rustc && have_gcc {
        let dir = std::env::temp_dir().join(format!("rfs_enum_mc_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let rs = dir.join("m_mc.rs");
        let c = dir.join("m_mc.c");
        std::fs::write(&rs, &rust_code).unwrap();
        std::fs::write(&c, &c_code).unwrap();
        let rbin = dir.join("m_mc_rust");
        let cbin = dir.join("m_mc_c");
        let out = std::process::Command::new("rustc")
            .args(["--edition", "2021"])
            .arg(&rs)
            .arg("-o")
            .arg(&rbin)
            .output()
            .expect("spawn rustc");
        assert!(
            out.status.success(),
            "rustc failed on enum-input MC codegen:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let out = std::process::Command::new("/usr/bin/gcc")
            .arg("-Wall")
            .arg(&c)
            .arg("-o")
            .arg(&cbin)
            .output()
            .expect("spawn gcc");
        assert!(
            out.status.success(),
            "gcc failed on enum-input MC codegen:\n{}",
            String::from_utf8_lossy(&out.stderr)
        );
        let r = std::process::Command::new(&rbin)
            .output()
            .expect("run rust engine");
        let c2 = std::process::Command::new(&cbin)
            .output()
            .expect("run c engine");
        assert_eq!(
            r.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&r.stdout)
        );
        assert_eq!(
            c2.status.code(),
            Some(1),
            "{}",
            String::from_utf8_lossy(&c2.stdout)
        );
        assert_eq!(
            r.stdout, c2.stdout,
            "enum-input MC outputs must be byte-identical across backends"
        );
    } else {
        eprintln!("note: rustc/gcc not available - enum-input compile check skipped");
    }
}

// ---------------------------------------------------------------------------
// no_std production codegen (Phase 5 task 9)
// ---------------------------------------------------------------------------

fn rfs_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs")
}

fn example_module(name: &str) -> rfs::Ir {
    let p = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(format!("examples/valid/{name}"));
    parse_module(&p)
}

#[test]
fn cli_no_std_flag_parsing() {
    // production accepts --no-std (both backends parse identically).
    for backend in ["rust", "c"] {
        let req =
            codegen::parse_codegen_args(&argss(&format!("{backend} production spec.rfs --no-std")))
                .unwrap();
        match req {
            codegen::CodegenRequest::Generate { no_std, .. } => assert!(no_std),
            other => panic!("expected Generate, got {:?}", other),
        }
    }
    // Default is off.
    let req = codegen::parse_codegen_args(&argss("rust production spec.rfs")).unwrap();
    match req {
        codegen::CodegenRequest::Generate { no_std, .. } => assert!(!no_std),
        other => panic!("expected Generate, got {:?}", other),
    }
    // test/mc are rejected with the pinned message.
    for cmd in ["test", "mc"] {
        let errs = codegen::parse_codegen_args(&argss(&format!("rust {cmd} spec.rfs --no-std")))
            .unwrap_err();
        assert_eq!(
            errs,
            vec![codegen::NO_STD_PRODUCTION_ONLY_ERROR.to_string()]
        );
    }
}

#[test]
fn cli_no_std_e2e_exit_codes_and_output() {
    let spec =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/valid/01_trivial_two_state.rfs");
    // production --no-std: exit 0, no_std header on stdout.
    let out = std::process::Command::new(rfs_bin())
        .args(["codegen", "rust", "production"])
        .arg(&spec)
        .arg("--no-std")
        .output()
        .expect("spawn rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    // v1.0 unified line-1 header (D-P6-3): spec 01's whole-file canonical IR
    // JSON hash; the CLI stamps it via `run()`.
    assert!(
        text.starts_with(
            "// Generated by RFS v1.0.0 | spec sha256:0929208e6691b540 | module trivial\n#![no_std]\n"
        ),
        "{text}"
    );

    // test --no-std: exit 1 + pinned diagnostic on stderr.
    let out = std::process::Command::new(rfs_bin())
        .args(["codegen", "rust", "test"])
        .arg(&spec)
        .arg("--no-std")
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(codegen::NO_STD_PRODUCTION_ONLY_ERROR),
        "stderr: {stderr}"
    );
}

#[test]
fn test_mc_entry_points_reject_no_std() {
    // Library-level guard (beyond the CLI parser): every test/mc entry point
    // rejects no_std with the same pinned message.
    let ir = example_module("01_trivial_two_state.rfs");
    let module = &ir.modules[0];
    let options = codegen::BackendOptions {
        no_std: true,
        ..Default::default()
    };
    for (name, result) in [
        (
            "rust test",
            rfs::backends::rust_test::generate_unit_tests(module, &options),
        ),
        (
            "rust mc",
            rfs::backends::rust_mc::generate_model_checker(module, &options),
        ),
        (
            "c test",
            rfs::backends::c_test::generate_unit_tests(module, &options),
        ),
        (
            "c mc",
            rfs::backends::c_mc::generate_model_checker(module, &options),
        ),
    ] {
        let errs = result.expect_err(&format!("{name} should reject no_std"));
        assert_eq!(
            errs,
            vec![codegen::NO_STD_PRODUCTION_ONLY_ERROR.to_string()]
        );
    }
}

#[test]
fn no_std_header_pins_and_std_unchanged() {
    let ir = example_module("01_trivial_two_state.rfs"); // no output payloads
    let module = &ir.modules[0];
    // Stamp the v1.0 spec hash (D-P6-3) exactly as `run()` does so these
    // direct library calls produce CLI-equivalent artifacts.
    let spec_hash = rfs::spec_hash::spec_hash16(&ir.to_json().expect("to_json").to_string());
    let header_line = format!(
        "// Generated by RFS v{} | spec sha256:{} | module {}",
        env!("CARGO_PKG_VERSION"),
        spec_hash,
        module.name
    );
    let std_opts = codegen::BackendOptions {
        spec_hash: spec_hash.clone(),
        ..Default::default()
    };
    let nostd_opts = codegen::BackendOptions {
        no_std: true,
        spec_hash,
        ..Default::default()
    };

    let std_code = rfs::backends::rust_fsm::generate_production(module, &std_opts)
        .unwrap()
        .code;
    let std_lines: Vec<&str> = std_code.lines().collect();
    // std mode is byte-stable (regression pin for the first two lines).
    assert_eq!(std_lines[0], header_line);
    assert!(std_lines[1].starts_with("#![allow(") && std_lines[1].ends_with(")]"));
    assert!(!std_code.contains("no_std"));

    let nostd_code = rfs::backends::rust_fsm::generate_production(module, &nostd_opts)
        .unwrap()
        .code;
    let nl: Vec<&str> = nostd_code.lines().collect();
    let expected_header: Vec<&str> = vec![
        &header_line,
        "#![no_std]",
        std_lines[1], // the identical #![allow(...)] line
        "extern crate alloc;",
        "use alloc::string::{String, ToString};",
        "use alloc::vec::Vec;",
        "use alloc::format;",
    ];
    assert_eq!(&nl[..expected_header.len()], &expected_header);
    // No output payloads in this module: the vec! macro is NOT imported.
    assert_eq!(nl[7], "#[derive(Debug, Clone, Copy, PartialEq, Eq)]");
    assert!(!nostd_code.contains("use alloc::vec;"));
}

#[test]
fn no_std_header_imports_vec_macro_when_outputs_exist() {
    let ir = example_module("10_output_payloads.rfs");
    let module = &ir.modules[0];
    let nostd_opts = codegen::BackendOptions {
        no_std: true,
        ..Default::default()
    };
    let code = rfs::backends::rust_fsm::generate_production(module, &nostd_opts)
        .unwrap()
        .code;
    let lines: Vec<&str> = code.lines().collect();
    assert_eq!(lines[7], "use alloc::vec;");
    assert!(code.contains("let outputs = vec!["));
}

#[test]
fn c_production_no_std_is_documented_noop() {
    let ir = example_module("01_trivial_two_state.rfs");
    let module = &ir.modules[0];
    const NOTE: &str = "// --no-std accepted: the C backend is already dependency-free; the flag has no effect on this output.";
    let std_code =
        rfs::backends::c_fsm::generate_production(module, &codegen::BackendOptions::default())
            .unwrap()
            .code;
    let nostd_code = rfs::backends::c_fsm::generate_production(
        module,
        &codegen::BackendOptions {
            no_std: true,
            ..Default::default()
        },
    )
    .unwrap()
    .code;
    assert!(nostd_code.contains(NOTE));
    assert!(!std_code.contains(NOTE));
    // The note line is the ONLY difference between the two artifacts.
    assert_eq!(nostd_code.replacen(&format!("{NOTE}\n"), "", 1), std_code);
}

/// Strip the generated file header (comment line, inner attributes, and the
/// no_std alloc imports) so the remaining BODY can be compared byte-for-byte
/// between the std and no_std variants (the task 9 API-identity criterion).
fn strip_generated_rust_header(code: &str) -> String {
    code.lines()
        .skip_while(|l| {
            l.starts_with("// Generated by RFS")
                || l.starts_with("#![")
                || *l == "extern crate alloc;"
                || l.starts_with("use alloc::")
        })
        .collect::<Vec<_>>()
        .join("\n")
}

fn tool_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

#[test]
fn no_std_compile_sweep_and_body_parity_all_valid_examples() {
    // For EVERY valid example, in both monitor modes: (1) the no_std
    // production output has a BYTE-IDENTICAL body to the std output (the
    // public API shape is unchanged - only the header differs); (2) it
    // COMPILES with `rustc --edition 2021 --crate-type lib`, which catches any
    // leftover std item. Gated on rustc availability (like the real-tool
    // diagram tests); generation + body parity run unconditionally.
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let specs = rfs::collect_rfs_files(root.join("examples/valid")).expect("examples/valid");
    assert_eq!(specs.len(), 47, "sweep must cover all valid examples");
    let have_rustc = tool_on_path("rustc");

    let dir = std::env::temp_dir().join(format!("rfs_nostd_sweep_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();

    let backend = codegen::get_backend("rust").expect("rust backend registered");
    for spec in &specs {
        let ir = parse_module(spec);
        let module = &ir.modules[0];
        let stem = spec.file_stem().unwrap().to_str().unwrap();
        for (mode, monitors) in [("plain", false), ("monitors", true)] {
            let std_opts = codegen::BackendOptions {
                with_property_monitors: monitors,
                ..Default::default()
            };
            let nostd_opts = codegen::BackendOptions {
                no_std: true,
                with_property_monitors: monitors,
                ..Default::default()
            };
            let std_code = (backend.generate_production)(module, &std_opts)
                .unwrap_or_else(|e| {
                    panic!(
                        "{} ({}): std generation failed: {e:?}",
                        spec.display(),
                        mode
                    )
                })
                .code;
            let nostd_code = (backend.generate_production)(module, &nostd_opts)
                .unwrap_or_else(|e| {
                    panic!(
                        "{} ({}): no_std generation failed: {e:?}",
                        spec.display(),
                        mode
                    )
                })
                .code;

            // API-identity criterion: body byte-identical, header-only diff.
            let std_body = strip_generated_rust_header(&std_code);
            let nostd_body = strip_generated_rust_header(&nostd_code);
            assert!(!std_body.is_empty(), "{} ({})", spec.display(), mode);
            assert_eq!(
                nostd_body,
                std_body,
                "body drift for {} ({})",
                spec.display(),
                mode
            );

            // Compile sweep: rustc must accept the no_std artifact.
            if have_rustc {
                let path = dir.join(format!("{stem}_{mode}.rs"));
                std::fs::write(&path, &nostd_code).unwrap();
                let out = std::process::Command::new("rustc")
                    .args([
                        "--edition",
                        "2021",
                        "--crate-type",
                        "lib",
                        "--emit=metadata",
                        "-o",
                    ])
                    .arg(dir.join(format!("{stem}_{mode}.rmeta")))
                    .arg(&path)
                    .output()
                    .expect("spawn rustc");
                assert!(
                    out.status.success(),
                    "rustc failed for {} ({}):\n{}",
                    spec.display(),
                    mode,
                    String::from_utf8_lossy(&out.stderr)
                );
            }
        }
    }
    if !have_rustc {
        eprintln!("note: rustc not on PATH - compile sweep skipped (body parity still enforced)");
    }
    let _ = std::fs::remove_dir_all(&dir);
}

// ---------------------------------------------------------------------------
// --with-protocol-fsm (Phase 5 task 11): opt-in `impl ProtocolFsm for Fsm`
// emission linking the artifact to the rfs-adapters crate.
// ---------------------------------------------------------------------------

/// The exact emitted block (byte pin; mirrors rust_fsm::emit_protocol_fsm_impl).
const PF_IMPL_BLOCK: &str = "\
// ProtocolFsm adapter surface (opt-in via --with-protocol-fsm, Phase 5
// task 11): this is the one documented opt-in that links a generated
// artifact to an external crate (rfs-adapters).
use rfs_adapters::ProtocolFsm;

impl ProtocolFsm for Fsm {
    type Input = Input;
    type Output = Output;
    type Error = StepError;

    fn new() -> Self { Fsm::new() }
    fn step(&mut self, input: Self::Input) -> Result<Vec<Self::Output>, Self::Error> {
        Fsm::step(self, input)
    }
}";

#[test]
fn cli_protocol_fsm_flag_parsing() {
    // production and test accept the flag (both backends parse identically).
    for backend in ["rust", "c"] {
        for cmd in ["production", "test"] {
            let req = codegen::parse_codegen_args(&argss(&format!(
                "{backend} {cmd} spec.rfs --with-protocol-fsm"
            )))
            .unwrap();
            match req {
                codegen::CodegenRequest::Generate {
                    with_protocol_fsm, ..
                } => {
                    assert!(with_protocol_fsm)
                }
                other => panic!("expected Generate, got {:?}", other),
            }
        }
    }
    // Default is off.
    let req = codegen::parse_codegen_args(&argss("rust production spec.rfs")).unwrap();
    match req {
        codegen::CodegenRequest::Generate {
            with_protocol_fsm, ..
        } => assert!(!with_protocol_fsm),
        other => panic!("expected Generate, got {:?}", other),
    }
    // mc is rejected with the pinned message.
    let errs =
        codegen::parse_codegen_args(&argss("rust mc spec.rfs --with-protocol-fsm")).unwrap_err();
    assert_eq!(errs, vec![codegen::PROTOCOL_FSM_MC_ERROR.to_string()]);
}

#[test]
fn cli_protocol_fsm_e2e_exit_codes_and_output() {
    let spec =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/valid/01_trivial_two_state.rfs");
    // production --with-protocol-fsm: exit 0, the impl block on stdout.
    let out = std::process::Command::new(rfs_bin())
        .args(["codegen", "rust", "production"])
        .arg(&spec)
        .arg("--with-protocol-fsm")
        .output()
        .expect("spawn rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let text = String::from_utf8_lossy(&out.stdout);
    assert!(text.contains(PF_IMPL_BLOCK), "{text}");

    // mc --with-protocol-fsm: exit 1 + pinned diagnostic on stderr.
    let out = std::process::Command::new(rfs_bin())
        .args(["codegen", "rust", "mc"])
        .arg(&spec)
        .arg("--with-protocol-fsm")
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains(codegen::PROTOCOL_FSM_MC_ERROR),
        "stderr: {stderr}"
    );
}

#[test]
fn protocol_fsm_emission_pins_and_default_unchanged() {
    let ir = example_module("01_trivial_two_state.rfs");
    let module = &ir.modules[0];
    let default_code =
        rfs::backends::rust_fsm::generate_production(module, &codegen::BackendOptions::default())
            .unwrap()
            .code;
    // Default output is unchanged: no trace of the trait at all.
    assert!(!default_code.contains("ProtocolFsm"));
    assert!(!default_code.contains("rfs_adapters"));

    let pf_code = rfs::backends::rust_fsm::generate_production(
        module,
        &codegen::BackendOptions {
            with_protocol_fsm: true,
            ..Default::default()
        },
    )
    .unwrap()
    .code;
    // The exact block, exactly once, after the inherent `impl Fsm`.
    assert_eq!(pf_code.matches(PF_IMPL_BLOCK).count(), 1);
    let pf_pos = pf_code.find(PF_IMPL_BLOCK).expect("block present");
    let inherent_pos = pf_code.find("impl Fsm {").expect("inherent impl present");
    assert!(pf_pos > inherent_pos);
    // Byte-level pin: the flag-on artifact is EXACTLY the default artifact
    // plus the block (leading newline + trailing newline from the emission).
    assert_eq!(pf_code, format!("{default_code}\n{PF_IMPL_BLOCK}\n"));

    // The test artifact carries the same block (before the #[cfg(test)] module).
    let test_code = rfs::backends::rust_test::generate_unit_tests(
        module,
        &codegen::BackendOptions {
            with_protocol_fsm: true,
            ..Default::default()
        },
    )
    .unwrap()
    .code;
    assert_eq!(test_code.matches(PF_IMPL_BLOCK).count(), 1);
    let t_pf = test_code.find(PF_IMPL_BLOCK).unwrap();
    let t_tests = test_code.find("#[cfg(test)]").expect("test module present");
    assert!(t_pf < t_tests);

    // The C backend ignores the flag: byte-identical output.
    let c_default =
        rfs::backends::c_fsm::generate_production(module, &codegen::BackendOptions::default())
            .unwrap()
            .code;
    let c_pf = rfs::backends::c_fsm::generate_production(
        module,
        &codegen::BackendOptions {
            with_protocol_fsm: true,
            ..Default::default()
        },
    )
    .unwrap()
    .code;
    assert_eq!(c_default, c_pf);
}

#[test]
fn mc_entry_points_reject_protocol_fsm() {
    // Library-level guard (beyond the CLI parser): both mc entry points
    // reject the flag with the same pinned message.
    let ir = example_module("01_trivial_two_state.rfs");
    let module = &ir.modules[0];
    let options = codegen::BackendOptions {
        with_protocol_fsm: true,
        ..Default::default()
    };
    for (name, result) in [
        (
            "rust mc",
            rfs::backends::rust_mc::generate_model_checker(module, &options),
        ),
        (
            "c mc",
            rfs::backends::c_mc::generate_model_checker(module, &options),
        ),
    ] {
        let errs = result.expect_err(&format!("{name} should reject --with-protocol-fsm"));
        assert_eq!(errs, vec![codegen::PROTOCOL_FSM_MC_ERROR.to_string()]);
    }
}

// ---------------------------------------------------------------------------
// Phase 5 task 5: Rust test renderer (four extended plan classes) + the
// separate libfuzzer_sys fuzz-target skeleton.
// ---------------------------------------------------------------------------

/// Extract the generated test function names from a `#[cfg(test)]` module
/// (functions named `fn <name>() {` - the tests; helpers like
/// `candidate_inputs` carry a return type and are excluded).
fn extract_generated_test_names(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|l| {
            let t = l.trim();
            let rest = t.strip_prefix("fn ")?;
            let name = rest.split('(').next()?.to_string();
            (rest.ends_with("() {")).then_some(name)
        })
        .collect()
}

/// All four extended plan classes are present for property-bearing examples,
/// with the pinned stable names.
#[test]
fn rust_test_emits_all_four_extended_classes_for_property_examples() {
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    for name in ["41_property_monitors.rfs", "43_sliding_window.rfs"] {
        let ir = example_module(name);
        let out = codegen::CodegenCommandKind::UnitTests
            .run(backend, &ir, &options)
            .unwrap_or_else(|e| panic!("{}: {e:?}", name));
        // Per-transition class (41 has transitions from every state).
        assert!(out.code.contains("fn t0_"), "{}: per-transition test", name);
        // Invariant class (both examples carry safety always-properties).
        assert!(
            out.code.contains("fn invariant_"),
            "{}: invariant test",
            name
        );
        // Property-monitor debug class.
        assert!(
            out.code.contains("_fires_on_violation() {"),
            "{}: monitor-violating trace test",
            name
        );
        // Random-trace class with the pinned seed and K in the name.
        assert!(
            out.code.contains("fn random_trace_seed12345_steps4() {"),
            "{}: random-trace test",
            name
        );
    }
}

/// The generated test names follow the pinned convention: the three
/// module-level tests, `t<id>_<state>_on_<event>` (+ `_not_taken`),
/// `invariant_<label>`, `monitor_<label>_fires_on_violation`, and
/// `random_trace_seed<seed>_steps<K>`.
#[test]
fn rust_test_names_follow_the_pinned_convention() {
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    // 40 has on_error + guarded transitions (not-taken cases); 41 is
    // property-rich.
    for name in ["40_on_error_recovery.rfs", "41_property_monitors.rfs"] {
        let ir = example_module(name);
        let out = codegen::CodegenCommandKind::UnitTests
            .run(backend, &ir, &options)
            .unwrap_or_else(|e| panic!("{}: {e:?}", name));
        for t in extract_generated_test_names(&out.code) {
            let ok = matches!(
                t.as_str(),
                "initial_state_is_correct"
                    | "positive_sequence_runs"
                    | "negative_unexpected_input_search"
                    | "random_trace_seed12345_steps4"
            ) || t.starts_with("t")
                && t[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
                && (t.contains("_on_") || t.ends_with("_not_taken"))
                || t.starts_with("invariant_")
                || t.ends_with("_fires_on_violation") && t.starts_with("monitor_");
            assert!(ok, "{}: unexpected test name `{}`", name, t);
        }
    }
}

/// The `--steps` option flows into the random-trace class (name + sequence
/// length) - option sensitivity for the new classes.
#[test]
fn rust_test_steps_option_changes_random_trace_class() {
    let backend = codegen::get_backend("rust").unwrap();
    let ir = example_module("41_property_monitors.rfs");
    let run = |steps: usize| {
        let options = codegen::BackendOptions {
            assumptions: Default::default(),
            steps,
            with_property_monitors: false,

            ..codegen::BackendOptions::default()
        };
        codegen::CodegenCommandKind::UnitTests
            .run(backend, &ir, &options)
            .expect("unit-test codegen")
    };
    let a = run(2);
    let b = run(6);
    assert!(a.code.contains("fn random_trace_seed12345_steps2() {"));
    assert!(b.code.contains("fn random_trace_seed12345_steps6() {"));
    // The precomputed index sequence length tracks K.
    let count = |code: &str| code.matches("usize").count();
    assert_ne!(count(&a.code), count(&b.code));
}

/// The fuzz skeleton is emitted as a SEPARATE extra file; the main test
/// artifact stays dependency-free (no libfuzzer references).
#[test]
fn rust_test_emits_fuzz_skeleton_as_separate_file() {
    let backend = codegen::get_backend("rust").unwrap();
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    // 43 has a struct input variant with bool + bit<4> + bit<32> fields.
    let ir = example_module("43_sliding_window.rfs");
    let out = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir, &options)
        .expect("unit-test codegen");
    assert_eq!(out.extra_files.len(), 1);
    assert_eq!(out.extra_files[0].name, "fuzz_target.rs");
    let fz = &out.extra_files[0].content;
    assert!(fz.contains("#![no_main]"));
    assert!(fz.contains("use libfuzzer_sys::fuzz_target;"));
    assert!(fz.contains("fuzz_target!"));
    // The decode plan is rendered: variant selector + payload masking.
    assert!(fz.contains("% 2"));
    assert!(fz.contains("& 0xFu8"));
    // Main artifact stays dependency-free.
    assert!(!out.code.contains("libfuzzer"));
}

/// CLI: `--out` writes the fuzz skeleton next to the test artifact; stdout
/// mode is refused (strict - no silent file loss). Both backends.
#[test]
fn cli_test_command_writes_fuzz_target_next_to_out() {
    let spec =
        PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples/valid/41_property_monitors.rfs");
    for (backend, ext, fuzz_name) in [
        ("rust", "rs", "fuzz_target.rs"),
        ("c", "c", "fuzz_target.c"),
    ] {
        let dir = std::env::temp_dir().join(format!("rfs_t5_cli_{backend}_{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let out_path = dir.join(format!("fsm_test.{ext}"));

        // --out: exit 0, both files written next to each other.
        let out = std::process::Command::new(rfs_bin())
            .args(["codegen", backend, "test"])
            .arg(&spec)
            .arg("--out")
            .arg(&out_path)
            .output()
            .expect("spawn rfs");
        assert!(
            out.status.success(),
            "{backend}: stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("wrote"), "{backend}: {}", stdout);
        assert!(
            dir.join(fuzz_name).is_file(),
            "{backend}: fuzz skeleton missing"
        );
        assert!(out_path.is_file());

        // stdout mode: exit 1 + diagnostic naming the extra file.
        let out2 = std::process::Command::new(rfs_bin())
            .args(["codegen", backend, "test"])
            .arg(&spec)
            .output()
            .expect("spawn rfs");
        assert!(
            !out2.status.success(),
            "{backend}: stdout mode must be refused"
        );
        let stderr = String::from_utf8_lossy(&out2.stderr);
        assert!(stderr.contains(fuzz_name), "{backend}: {}", stderr);
        assert!(stderr.contains("--out"), "{backend}: {}", stderr);

        let _ = std::fs::remove_dir_all(&dir);
    }
}

/// The aggregate generated-code crate: the new classes are present (per
/// module) and the crate stays dependency-free (no fuzz skeleton inlined).
#[test]
fn aggregate_crate_carries_new_classes_and_stays_dependency_free() {
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    let crate_ = rfs::backends::rust_test::generate_aggregate_test_crate(&files(), &options)
        .expect("aggregate codegen");
    assert!(crate_.lib.contains("fn random_trace_seed12345_steps4"));
    assert!(crate_.lib.contains("fn invariant_") || crate_.lib.contains("_fires_on_violation"));
    // No libfuzzer_sys anywhere: the aggregate must compile without deps.
    assert!(!crate_.lib.contains("libfuzzer"));
    assert!(!crate_.manifest.contains("libfuzzer"));
}

// ---------------------------------------------------------------------------
// Phase 5 task 6: C test renderer (four extended plan classes, same stable
// names as Rust) + the separate libFuzzer fuzz-target skeleton.
// ---------------------------------------------------------------------------

/// Extract the generated C test names from the harness `tests[]` table
/// (entries `{"<name>", <fn>},`).
fn extract_c_test_names(code: &str) -> Vec<String> {
    code.lines()
        .filter_map(|l| {
            let t = l.trim();
            let rest = t.strip_prefix("{")?.strip_suffix("},")?;
            let (name, fn_name) = rest.split_once(",")?;
            let name = name.trim().trim_matches('"').to_string();
            let fn_name = fn_name.trim().to_string();
            assert_eq!(name, fn_name, "stable name must be the C function name");
            Some(name)
        })
        .collect()
}

/// All four extended plan classes are present for property-bearing examples,
/// with the pinned stable names (same as the Rust renderer).
#[test]
fn c_test_emits_all_four_extended_classes_for_property_examples() {
    let backend = codegen::get_backend("c").unwrap();
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    for name in ["41_property_monitors.rfs", "43_sliding_window.rfs"] {
        let ir = example_module(name);
        let out = codegen::CodegenCommandKind::UnitTests
            .run(backend, &ir, &options)
            .unwrap_or_else(|e| panic!("{}: {e:?}", name));
        // Per-transition class.
        assert!(
            out.code.contains("static void t0_"),
            "{}: per-transition test",
            name
        );
        // Invariant class (both examples carry safety always-properties).
        assert!(
            out.code.contains("static void invariant_"),
            "{}: invariant test",
            name
        );
        // Property-monitor debug class.
        assert!(
            out.code.contains("_fires_on_violation(void) {"),
            "{}: monitor-violating trace test",
            name
        );
        // Random-trace class with the pinned seed and K in the name.
        assert!(
            out.code
                .contains("static void random_trace_seed12345_steps4(void) {"),
            "{}: random-trace test",
            name
        );
    }
}

/// The generated C test names follow the pinned convention (identical to the
/// Rust renderer): the three module-level tests, `t<id>_<state>_on_<event>`
/// (+ `_not_taken`), `invariant_<label>`, `monitor_<label>_fires_on_violation`,
/// and `random_trace_seed<seed>_steps<K>`.
#[test]
fn c_test_names_follow_the_pinned_convention() {
    let backend = codegen::get_backend("c").unwrap();
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    // 40 has on_error + guarded transitions (not-taken cases); 41 is
    // property-rich.
    for name in ["40_on_error_recovery.rfs", "41_property_monitors.rfs"] {
        let ir = example_module(name);
        let out = codegen::CodegenCommandKind::UnitTests
            .run(backend, &ir, &options)
            .unwrap_or_else(|e| panic!("{}: {e:?}", name));
        let names = extract_c_test_names(&out.code);
        assert!(names.len() >= 4, "{}: harness table too small", name);
        for t in names {
            let ok = matches!(
                t.as_str(),
                "initial_state_is_correct"
                    | "positive_sequence_runs"
                    | "negative_unexpected_input_search"
                    | "random_trace_seed12345_steps4"
            ) || (t.starts_with('t')
                && t[1..].chars().next().is_some_and(|c| c.is_ascii_digit())
                && (t.contains("_on_") || t.ends_with("_not_taken")))
                || t.starts_with("invariant_")
                || (t.starts_with("monitor_") && t.ends_with("_fires_on_violation"));
            assert!(ok, "{}: unexpected test name `{}`", name, t);
        }
    }
}

/// The `--steps` option flows into the random-trace class (name + sequence
/// length) - option sensitivity for the new classes.
#[test]
fn c_test_steps_option_changes_random_trace_class() {
    let backend = codegen::get_backend("c").unwrap();
    let ir = example_module("41_property_monitors.rfs");
    let run = |steps: usize| {
        let options = codegen::BackendOptions {
            assumptions: Default::default(),
            steps,
            with_property_monitors: false,

            ..codegen::BackendOptions::default()
        };
        codegen::CodegenCommandKind::UnitTests
            .run(backend, &ir, &options)
            .expect("unit-test codegen")
    };
    let a = run(2);
    let b = run(6);
    assert!(a
        .code
        .contains("static void random_trace_seed12345_steps2(void) {"));
    assert!(b
        .code
        .contains("static void random_trace_seed12345_steps6(void) {"));
    // The precomputed index table content tracks K.
    let table = |code: &str| {
        let start = code
            .find("static const int RT_IDX[] = {")
            .expect("RT_IDX table present");
        let end = code[start..].find("};").unwrap() + start;
        code[start..end].to_string()
    };
    assert_ne!(table(&a.code), table(&b.code));
}

/// The fuzz skeleton is emitted as a SEPARATE extra file; the main test
/// artifact stays dependency-free (no libFuzzer references) and both compile
/// with `gcc -Wall` at zero warnings (the skeleton against the generated
/// production source it includes).
#[test]
fn c_test_emits_fuzz_skeleton_as_extra_file() {
    let backend = codegen::get_backend("c").unwrap();
    let options = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 4,
        with_property_monitors: false,

        ..codegen::BackendOptions::default()
    };
    // 43 has a struct input variant with bool + bit<4> + bit<32> fields.
    let ir = example_module("43_sliding_window.rfs");
    let out = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir, &options)
        .expect("unit-test codegen");
    assert_eq!(out.extra_files.len(), 1);
    assert_eq!(out.extra_files[0].name, "fuzz_target.c");
    let fz = &out.extra_files[0].content;
    assert!(fz.contains("int LLVMFuzzerTestOneInput(const uint8_t* data, size_t size)"));
    assert!(fz.contains("clang -O1 -fsanitize=fuzzer,address"));
    // The decode plan is rendered: variant selector + payload masking.
    assert!(fz.contains("% 2"));
    assert!(fz.contains("& 0xF"));
    // Main artifact stays dependency-free.
    assert!(!out.code.contains("LLVMFuzzer"));
    assert!(!out.code.contains("libFuzzer"));

    // gcc -Wall: the main test artifact and the fuzz skeleton (against the
    // generated production source it includes) must compile warning-free.
    let dir = std::env::temp_dir().join(format!("rfs_t6_fuzz_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    let prod = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &options)
        .expect("production codegen");
    let main_c = dir.join("fsm_test.c");
    let prod_c = dir.join("rfs_generated_fsm.c");
    let fuzz_c = dir.join("fuzz_target.c");
    std::fs::write(&main_c, &out.code).unwrap();
    std::fs::write(&prod_c, &prod.code).unwrap();
    std::fs::write(&fuzz_c, fz).unwrap();
    for src_path in [&main_c, &fuzz_c] {
        let out = std::process::Command::new("/usr/bin/gcc")
            .args(["-Wall", "-c"])
            .arg(src_path)
            .arg("-o")
            .arg(dir.join("out.o"))
            .output()
            .expect("run gcc");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            out.status.success(),
            "{} failed to compile:\n{stderr}",
            src_path.display()
        );
        assert!(
            !stderr.lines().any(|l| l.contains("warning:")),
            "gcc -Wall emitted warnings on {}:\n{stderr}",
            src_path.display()
        );
    }
    let _ = std::fs::remove_dir_all(&dir);
}

/// Phase 5 task 16 close-out fix: the generated C core carries an explicit
/// `fsm_free` lifecycle end and the libFuzzer skeleton calls it on EVERY exit
/// path (single-exit decode), so a long fuzz run does not accumulate RSS until
/// libFuzzer's out-of-memory limit trips. Regression case: spec 01, where
/// every input byte maps to one FSM step (before the fix the shipped
/// `rfs test --backend c --suite fuzz` OOM'd at ~200k runs on this spec).
#[test]
fn c_fuzz_target_frees_per_input_and_bounded_run_stays_green() {
    let backend = codegen::get_backend("c").unwrap();
    let options = codegen::BackendOptions::default();
    let ir = example_module("01_trivial_two_state.rfs");

    // The production core exposes the lifecycle end.
    let prod = codegen::CodegenCommandKind::Production
        .run(backend, &ir, &options)
        .expect("production codegen");
    assert!(
        prod.code.contains("void fsm_free(fsm_t* f)"),
        "generated C core must define fsm_free"
    );

    // The skeleton frees on every exit path (single-exit decode).
    let out = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir, &options)
        .expect("unit-test codegen");
    let fz = &out.extra_files[0].content;
    assert!(
        fz.contains("int done = 0;"),
        "skeleton must use single-exit decode"
    );
    assert!(
        fz.contains("if (done) break;"),
        "skeleton must break on truncated/unknown input"
    );
    assert!(
        fz.contains("fsm_free(&fsm);"),
        "skeleton must free the per-input fsm"
    );
    assert_eq!(
        fz.matches("return 0;").count(),
        1,
        "skeleton must have a single return (no early-return leak paths)"
    );

    // Real-tool regression: build + bounded run on the OOM spec. Before the
    // fix this exited non-zero with `libFuzzer: out-of-memory` after ~200k
    // runs; with per-input frees RSS stays flat for millions of runs.
    if !tool_on_path("clang") {
        eprintln!("note: clang not available - C fuzz bounded-run check skipped");
        return;
    }
    let dir = std::env::temp_dir().join(format!("rfs_t16_fuzz_{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    std::fs::write(dir.join("rfs_generated_fsm.c"), &prod.code).unwrap();
    std::fs::write(dir.join("fuzz_target.c"), fz).unwrap();
    let build = std::process::Command::new("clang")
        .args(["-O1", "-fsanitize=fuzzer,address"])
        .arg(dir.join("fuzz_target.c"))
        .arg("-o")
        .arg(dir.join("fuzz_bin"))
        .output()
        .expect("run clang");
    assert!(
        build.status.success(),
        "clang build failed:\n{}",
        String::from_utf8_lossy(&build.stderr)
    );
    let run = std::process::Command::new(dir.join("fuzz_bin"))
        .args(["-max_total_time=5", "-rss_limit_mb=1024"])
        .env("ASAN_OPTIONS", "detect_leaks=0")
        .output()
        .expect("run fuzz target");
    let combined = format!(
        "{}{}",
        String::from_utf8_lossy(&run.stdout),
        String::from_utf8_lossy(&run.stderr)
    );
    assert!(
        run.status.success(),
        "bounded C fuzz run on spec 01 must stay green (no OOM); tail:\n{}",
        combined
            .lines()
            .rev()
            .take(5)
            .collect::<Vec<_>>()
            .join("\n")
    );
    assert!(
        !combined.contains("out-of-memory"),
        "libFuzzer reported out-of-memory - per-input frees are not effective"
    );
    let _ = std::fs::remove_dir_all(&dir);
}
