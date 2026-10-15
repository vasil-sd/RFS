use std::path::PathBuf;

use rfs::{collect_rfs_files, parse_source, printer, training, Ir};

fn project_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

#[test]
fn training_categories_exist_and_have_valid_jsonl() {
    let root = project_root();
    let categories = training::category_dirs(&root).expect("training directory should be readable");
    assert!(
        !categories.is_empty(),
        "expected at least one training category under training/"
    );

    let mut total_examples = 0;

    for category in &categories {
        let jsonl = training::category_jsonl_path(category);
        assert!(
            jsonl.exists(),
            "missing {} for category {}",
            jsonl.display(),
            category.file_name().unwrap_or_default().to_string_lossy()
        );

        let lines = training::read_jsonl_lines(&jsonl).expect("training JSONL should be readable");
        assert!(
            !lines.is_empty(),
            "category {} has an empty training JSONL",
            category.file_name().unwrap_or_default().to_string_lossy()
        );

        for line in &lines {
            training::validate_training_line(line)
                .unwrap_or_else(|e| panic!("invalid training line in {}: {e}", jsonl.display()));
        }

        total_examples += lines.len();
    }

    assert!(total_examples > 0);
}

#[test]
fn extracted_training_snippets_parse_as_expected() {
    let root = project_root();
    let categories = training::category_dirs(&root).expect("training directory should be readable");

    let mut valid_count = 0;
    let mut invalid_count = 0;

    for category in &categories {
        let name = category
            .file_name()
            .map(|s| s.to_string_lossy().into_owned())
            .unwrap_or_default();

        let valid_dir = category.join("snippets").join("valid");
        if valid_dir.exists() {
            let files =
                collect_rfs_files(&valid_dir).expect("valid snippets should be collectible");
            for file in &files {
                let source = std::fs::read_to_string(file)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
                let ir = parse_source(&source).unwrap_or_else(|e| {
                    panic!(
                        "valid training snippet {} failed: {}",
                        file.display(),
                        e.rich_display()
                    )
                });
                rfs::validator::validate_ir(&ir).unwrap_or_else(|errors| {
                    panic!(
                        "valid training snippet {} failed validation: {:?}",
                        file.display(),
                        errors
                    )
                });
                let json = serde_json::to_string(&ir).expect("valid snippet IR should serialize");
                let back: Ir =
                    serde_json::from_str(&json).expect("valid snippet IR should deserialize");
                assert_eq!(ir, back);

                // Round-trips through the RFS printer for valid snippets.
                printer::roundtrip_rfs(&source).unwrap_or_else(|e| {
                    panic!(
                        "rfs->json->rfs round-trip failed for {}: {}",
                        file.display(),
                        e
                    )
                });
                printer::roundtrip_json(&json).unwrap_or_else(|e| {
                    panic!(
                        "json->rfs->json round-trip failed for {}: {}",
                        file.display(),
                        e
                    )
                });

                valid_count += 1;
            }
        }

        let invalid_dir = category.join("snippets").join("invalid");
        if invalid_dir.exists() {
            let files =
                collect_rfs_files(&invalid_dir).expect("invalid snippets should be collectible");
            for file in &files {
                let source = std::fs::read_to_string(file)
                    .unwrap_or_else(|e| panic!("failed to read {}: {e}", file.display()));
                let translation_ready = match parse_source(&source) {
                    Err(_) => false,
                    Ok(ir) => rfs::validator::validate_ir(&ir).is_ok(),
                };
                assert!(
                    !translation_ready,
                    "invalid training snippet {} is translation-ready: {}",
                    name,
                    file.display()
                );
                invalid_count += 1;
            }
        }
    }

    assert!(
        valid_count + invalid_count > 0,
        "expected extracted RFS snippets under training/*/snippets"
    );
}

#[test]
fn aggregate_training_sets_matches_category_files() {
    let root = project_root();
    let categories = training::category_dirs(&root).expect("training directory should be readable");

    let expected = categories
        .iter()
        .filter_map(|category| {
            let path = training::category_jsonl_path(category);
            path.exists()
                .then(|| training::read_jsonl_lines(&path).expect("category JSONL readable"))
        })
        .map(|lines| lines.len())
        .sum::<usize>();

    let out = std::env::temp_dir().join(format!(
        "rfs_training_aggregate_{}.jsonl",
        std::process::id()
    ));

    training::aggregate_training_sets(&root, &out).expect("training aggregation should succeed");

    let actual = training::read_jsonl_lines(&out).expect("aggregate file should be readable");
    assert_eq!(actual.len(), expected);

    for line in &actual {
        training::validate_training_line(line).expect("aggregate line should be valid");
    }

    let _ = std::fs::remove_file(out);
}

#[test]
fn committed_all_rfs_training_file_is_fresh() {
    let root = project_root();
    let all = root.join("all_rfs_training.jsonl");
    assert!(
        all.exists(),
        "run `cargo build-training` to generate all_rfs_training.jsonl"
    );

    let categories = training::category_dirs(&root).expect("training directory should be readable");
    let expected = categories
        .iter()
        .filter_map(|category| {
            let path = training::category_jsonl_path(category);
            path.exists()
                .then(|| training::read_jsonl_lines(&path).expect("category JSONL readable"))
        })
        .map(|lines| lines.len())
        .sum::<usize>();

    let actual =
        training::read_jsonl_lines(&all).expect("all_rfs_training.jsonl should be readable");
    assert_eq!(
        actual.len(),
        expected,
        "all_rfs_training.jsonl is stale; run `cargo build-training`"
    );
}
