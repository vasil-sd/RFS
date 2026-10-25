use std::fs;
use std::path::PathBuf;

use rfs::{collect_rfs_files, parse_source, printer, Ir};

fn example_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

#[test]
fn valid_examples_parse_and_roundtrip() {
    let dir = example_root().join("valid");
    let files = collect_rfs_files(&dir).expect("valid examples directory should exist");

    assert!(
        files.len() >= 20,
        "expected at least 20 valid examples, found {}",
        files.len()
    );

    for file in &files {
        let source = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", file.display(), e));

        let ir = parse_source(&source)
            .unwrap_or_else(|e| panic!("failed to parse {}: {}", file.display(), e.rich_display()));

        assert!(
            !ir.modules.is_empty(),
            "{} parsed to zero modules",
            file.display()
        );

        let json = ir
            .to_json()
            .unwrap_or_else(|e| panic!("failed to serialize IR from {}: {}", file.display(), e))
            .to_string();

        let value = rfs::json_rw::parse(&json)
            .unwrap_or_else(|e| panic!("failed to parse IR JSON from {}: {}", file.display(), e));
        let back: Ir = Ir::from_json(&value)
            .unwrap_or_else(|e| panic!("failed to deserialize IR from {}: {}", file.display(), e));

        assert_eq!(ir, back, "IR roundtrip failed for {}", file.display());

        // Round-trip rfs -> json -> rfs: the printed RFS must parse back to
        // an equal IR.
        printer::roundtrip_rfs(&source).unwrap_or_else(|e| {
            panic!(
                "rfs->json->rfs round-trip failed for {}: {}",
                file.display(),
                e
            )
        });

        // Round-trip json -> rfs -> json: the printed RFS must parse back to
        // an IR that serializes equivalently.
        printer::roundtrip_json(&json).unwrap_or_else(|e| {
            panic!(
                "json->rfs->json round-trip failed for {}: {}",
                file.display(),
                e
            )
        });
    }
}

#[test]
fn invalid_examples_rejected() {
    // Phase 2: invalid examples may fail at parse time (syntax) or at
    // validation time (semantic errors such as duplicate priorities).
    let dir = example_root().join("invalid");
    if !dir.exists() {
        return;
    }

    let files = collect_rfs_files(&dir).expect("invalid examples directory should be readable");

    for file in &files {
        let source = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", file.display(), e));

        match parse_source(&source) {
            Err(_) => {} // parse failure: rejected as expected
            Ok(ir) => assert!(
                rfs::validator::validate_ir(&ir).is_err(),
                "expected {} to fail validation, but it succeeded",
                file.display()
            ),
        }
    }
}

#[test]
fn example_count_is_reported() {
    let dir = example_root().join("valid");
    let files = collect_rfs_files(&dir).expect("valid examples directory should exist");
    eprintln!("valid RFS examples: {}", files.len());
}
