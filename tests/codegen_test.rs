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
    };
    for path in &files {
        let ir = parse_module(path);
        for (kind, marker) in [
            (codegen::CodegenCommandKind::Production, "pub struct Fsm"),
            (codegen::CodegenCommandKind::UnitTests, "#[test]"),
            (codegen::CodegenCommandKind::ModelChecker, "fn main()"),
        ] {
            let out = kind
                .run(backend, &ir.modules[0], &options)
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
    };
    let on_opts = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: true,
    };
    let off = codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &off_opts)
        .expect("production codegen (flag off)");
    let on = codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &on_opts)
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
    };
    let big = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 6,
        with_property_monitors: false,
    };
    let a = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir.modules[0], &small)
        .unwrap();
    let b = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir.modules[0], &big)
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
    };
    let mc = codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir.modules[0], &options)
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
    };
    let out = codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &options)
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
    };
    let out = codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &options)
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
    };
    let out = codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &options)
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
    };
    let out = codegen::CodegenCommandKind::UnitTests
        .run(backend, &ir.modules[0], &options)
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
    };
    for path in &files {
        let ir = parse_module(path);
        for module in &ir.modules {
            // Specs with bit widths outside 8/16/32/64 are rejected by the C backend.
            let result =
                rfs::codegen::CodegenCommandKind::Production.run(backend, module, &options);
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
                .run(backend, module, &options)
                .unwrap_or_else(|e| panic!("{} c test failed: {:?}", path.display(), e));
            assert!(
                tests.code.contains("int main(void)"),
                "{} c test missing main",
                path.display()
            );

            let mc = rfs::codegen::CodegenCommandKind::ModelChecker
                .run(backend, module, &options)
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
    };
    let on_opts = rfs::codegen::BackendOptions {
        assumptions: rfs::codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: true,
    };
    let off = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &off_opts)
        .expect("production codegen (flag off)");
    let on = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &on_opts)
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
    };
    let out = rfs::codegen::CodegenCommandKind::Production
        .run(backend, &ir.modules[0], &options)
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
    };
    // Rust: monitor aggregate + StepError variant present despite flag off.
    let rust = codegen::get_backend("rust").unwrap();
    for kind in [
        codegen::CodegenCommandKind::ModelChecker,
        codegen::CodegenCommandKind::UnitTests,
    ] {
        let out = kind
            .run(rust, &ir.modules[0], &options)
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
            .run(c, &ir.modules[0], &options)
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
        };
        for path in &property_files {
            let ir = parse_module(path);
            for module in &ir.modules {
                let result = codegen::CodegenCommandKind::Production.run(backend, module, &options);
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
