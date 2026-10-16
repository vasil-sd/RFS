use rfs::{backends::rust_test, codegen, collect_rfs_files};
use std::path::PathBuf;
use std::process::ExitCode;

fn main() -> ExitCode {
    let mut files = Vec::new();
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let mut args = std::env::args().skip(1);
    let mut test_steps = 4usize;
    let mut assumptions = codegen::AssumptionSelection::All;
    while let Some(arg) = args.next() {
        if arg == "--test-steps" {
            test_steps = match args.next().and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => return ExitCode::FAILURE,
            };
        } else if arg == "--assumes" {
            assumptions = match args.next().and_then(|v| v.parse().ok()) {
                Some(v) => v,
                None => return ExitCode::FAILURE,
            };
        }
    }
    // Only well-formed sources: the valid examples plus each category's
    // snippets/valid (snippets/invalid are expected to fail parsing).
    match collect_rfs_files(root.join("examples").join("valid")) {
        Ok(mut v) => files.append(&mut v),
        Err(e) => {
            eprintln!("collect examples/valid: {}", e);
            return ExitCode::FAILURE;
        }
    }
    if let Ok(entries) = std::fs::read_dir(root.join("training")) {
        for entry in entries.flatten() {
            let valid = entry.path().join("snippets").join("valid");
            if valid.is_dir() {
                match collect_rfs_files(&valid) {
                    Ok(mut v) => files.append(&mut v),
                    Err(e) => {
                        eprintln!("collect {}: {}", valid.display(), e);
                        return ExitCode::FAILURE;
                    }
                }
            }
        }
    }
    files.sort();
    files.dedup();
    if files.is_empty() {
        eprintln!("no RFS files found");
        return ExitCode::FAILURE;
    }
    let options = codegen::BackendOptions {
        assumptions,
        steps: test_steps,
        with_property_monitors: false,
    };
    match rust_test::generate_aggregate_test_crate(&files, &options) {
        Ok(crate_) => {
            for w in &crate_.warnings {
                eprintln!("warning: {}", w);
            }
            if let Err(e) = rust_test::write_generated_crate(&root, &crate_) {
                eprintln!("write failed: {}", e);
                return ExitCode::FAILURE;
            }
            println!(
                "generated {} RFS modules into target/rfs_codegen_tests",
                crate_.module_count
            );
            ExitCode::SUCCESS
        }
        Err(errors) => {
            for e in errors {
                eprintln!("codegen error: {}", e);
            }
            ExitCode::FAILURE
        }
    }
}
