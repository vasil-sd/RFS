use rfs::{backends::rust_test, codegen, collect_rfs_files};
use std::process::{Command, ExitCode};

fn main() -> ExitCode {
    let mut files = Vec::new();
    if let Ok(v) = collect_rfs_files("examples/valid") {
        files.extend(v);
    }
    if let Ok(entries) = std::fs::read_dir("training") {
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
    let options = codegen::BackendOptions {
        assumptions: codegen::AssumptionSelection::All,
        steps: 4,
        with_property_monitors: false,
        ..codegen::BackendOptions::default()
    };
    let crate_ = match rust_test::generate_aggregate_test_crate(&files, &options) {
        Ok(c) => c,
        Err(errors) => {
            for e in errors {
                eprintln!("codegen error: {}", e);
            }
            return ExitCode::FAILURE;
        }
    };
    for w in &crate_.warnings {
        eprintln!("warning: {}", w);
    }
    let root = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    if let Err(e) = rust_test::write_generated_crate(&root, &crate_) {
        eprintln!("write failed: {}", e);
        return ExitCode::FAILURE;
    }
    let manifest = root.join("target/rfs_codegen_tests/Cargo.toml");
    let status = Command::new("cargo")
        .arg("test")
        .arg("--manifest-path")
        .arg(&manifest)
        .status();
    match status {
        Ok(s) if s.success() => ExitCode::SUCCESS,
        Ok(s) => {
            eprintln!("generated tests exited with {:?}", s.code());
            ExitCode::FAILURE
        }
        Err(e) => {
            eprintln!("failed to run cargo test: {}", e);
            ExitCode::FAILURE
        }
    }
}
