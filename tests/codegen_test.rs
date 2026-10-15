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
    assert!(codegen::get_backend("c").is_none());
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
fn steps_option_changes_test_generation() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let ir = parse_module(&root.join("examples/valid/02_counter.rfs"));
    let backend = codegen::get_backend("rust").unwrap();
    let small = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 1,
    };
    let big = codegen::BackendOptions {
        assumptions: Default::default(),
        steps: 6,
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
        } => {
            assert_eq!(backend_name, "rust");
            assert_eq!(command, codegen::CodegenCommandKind::UnitTests);
            assert_eq!(input, "spec.rfs");
            assert_eq!(out.as_deref(), Some("/tmp/t.rs"));
            assert_eq!(assumptions, codegen::AssumptionSelection::None);
            assert_eq!(steps, 7);
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
    };
    let mc = codegen::CodegenCommandKind::ModelChecker
        .run(backend, &ir.modules[0], &options)
        .expect("model checker codegen should succeed");
    assert!(mc.code.contains("fn main()"));
}
