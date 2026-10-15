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

        let json = serde_json::to_string(&ir)
            .unwrap_or_else(|e| panic!("failed to serialize IR from {}: {}", file.display(), e));

        let back: Ir = serde_json::from_str(&json)
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
fn invalid_examples_fail_to_parse() {
    let dir = example_root().join("invalid");
    if !dir.exists() {
        return;
    }

    let files = collect_rfs_files(&dir).expect("invalid examples directory should be readable");

    for file in &files {
        let source = fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", file.display(), e));

        let result = parse_source(&source);
        assert!(
            result.is_err(),
            "expected {} to fail parsing, but it succeeded",
            file.display()
        );
    }
}

#[test]
fn example_count_is_reported() {
    let dir = example_root().join("valid");
    let files = collect_rfs_files(&dir).expect("valid examples directory should exist");
    eprintln!("valid RFS examples: {}", files.len());
}
