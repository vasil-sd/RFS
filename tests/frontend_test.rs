//! Frontend CLI logic tests: format conversion (`rfs parse`), output path
//! planning (target-dir structure, extension handling, collision guard), and
//! validation dispatch (`rfs validate`) including `*.rfs-validation.err`
//! files.

use std::path::{Path, PathBuf};

use rfs::{frontend, printer, Format, Ir};

const VALID_RFS: &str =
    "module m { enum State { A, B } initial A event Go transition A on Go { next B; } }";

/// Parses but fails validation (unknown next state).
const BAD_STATE_RFS: &str =
    "module m { enum State { A } initial A event Go transition A on Go { next Nope; } }";

/// Syntactically invalid RFS.
const UNPARSEABLE_RFS: &str = "module m { @ }";

fn tmp_root(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!("rfs_frontend_{}_{}", std::process::id(), tag));
    let _ = std::fs::remove_dir_all(&dir);
    std::fs::create_dir_all(&dir).unwrap();
    dir
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn write_file(root: &Path, rel: &str, content: &str) -> PathBuf {
    let p = root.join(rel);
    if let Some(parent) = p.parent() {
        std::fs::create_dir_all(parent).unwrap();
    }
    std::fs::write(&p, content).unwrap();
    p
}

fn ir_json(source: &str) -> String {
    rfs::parse_source(source)
        .expect("parse")
        .to_json()
        .expect("serialize")
        .to_string_pretty()
}

/// Parse an IR JSON document text into an `Ir` (panics with the error on failure).
fn json_ir(text: &str) -> Ir {
    let value = rfs::json_rw::parse(text).unwrap_or_else(|e| panic!("invalid IR JSON: {e}"));
    Ir::from_json(&value).unwrap_or_else(|e| panic!("invalid IR JSON: {e}"))
}

// ---------------------------------------------------------------------------
// Format parsing
// ---------------------------------------------------------------------------

#[test]
fn format_from_str_and_extension() {
    assert_eq!("rfs".parse::<Format>().unwrap(), Format::Rfs);
    assert_eq!("json".parse::<Format>().unwrap(), Format::Json);
    assert_eq!("RFS".parse::<Format>().unwrap(), Format::Rfs);
    assert_eq!("JSON".parse::<Format>().unwrap(), Format::Json);
    assert!("xml".parse::<Format>().is_err());
    assert!(Format::from_extension("rfs") == Some(Format::Rfs));
    assert!(Format::from_extension("json") == Some(Format::Json));
    assert!(Format::from_extension("txt").is_none());
    assert!(Format::from_extension("").is_none());
}

// ---------------------------------------------------------------------------
// p.4: strict extension handling
// ---------------------------------------------------------------------------

#[test]
fn file_input_rejects_unsupported_extension() {
    let root = tmp_root("ext_file");
    let f = write_file(&root, "a.txt", VALID_RFS);
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &f, None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("a.txt"));
    assert!(
        errors[0].contains(".rfs"),
        "unexpected error: {}",
        errors[0]
    );
    cleanup(&root);
}

#[test]
fn file_input_rejects_mismatched_format_extension() {
    let root = tmp_root("ext_mismatch");
    let f = write_file(&root, "a.json", VALID_RFS); // .json content but --input rfs
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &f, None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].contains("--input json"),
        "unexpected error: {}",
        errors[0]
    );
    cleanup(&root);
}

#[test]
fn directory_scan_only_collects_matching_extension() {
    let root = tmp_root("ext_dir");
    write_file(&root, "good.rfs", VALID_RFS);
    write_file(&root, "other.json", &ir_json(VALID_RFS)); // must be ignored for --input rfs
    write_file(&root, "notes.txt", "hello");
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &root, None);
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    assert_eq!(written.len(), 1);
    assert!(written[0].ends_with("good.json"));
    // The .json input must not have been converted (no good.rfs -> good.json
    // from other.json, i.e. no extra output files).
    assert!(!root.join("other.rfs").exists());
    cleanup(&root);
}

#[test]
fn output_names_drop_original_extension_without_double_extensions() {
    let root = tmp_root("ext_doubled");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &f, None);
    assert!(errors.is_empty());
    assert_eq!(written, vec![root.join("a.json")]);
    assert!(
        !root.join("a.rfs.json").exists(),
        "double extension created"
    );

    // And the reverse direction.
    let j = write_file(&root, "b.json", &ir_json(VALID_RFS));
    let (written, errors) = frontend::run_conversion(Format::Json, Format::Rfs, &j, None);
    assert!(errors.is_empty());
    assert_eq!(written, vec![root.join("b.rfs")]);
    assert!(
        !root.join("b.json.rfs").exists(),
        "double extension created"
    );
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// p.3: output placement and directory structure
// ---------------------------------------------------------------------------

#[test]
fn file_input_writes_next_to_source_without_target_dir() {
    let root = tmp_root("place_file");
    let f = write_file(&root, "sub/a.rfs", VALID_RFS);
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &f, None);
    assert!(errors.is_empty());
    assert_eq!(written, vec![root.join("sub/a.json")]);
    cleanup(&root);
}

#[test]
fn file_input_with_target_dir_writes_into_target_dir() {
    let root = tmp_root("place_file_td");
    let f = write_file(&root, "in/a.rfs", VALID_RFS);
    let td = root.join("out");
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &f, Some(&td));
    assert!(errors.is_empty());
    assert_eq!(written, vec![root.join("out/a.json")]);
    cleanup(&root);
}

#[test]
fn directory_input_preserves_structure_in_target_dir() {
    let root = tmp_root("place_dir_td");
    write_file(&root, "in/top.rfs", VALID_RFS);
    write_file(&root, "in/sub/deep.rfs", VALID_RFS);
    let td = root.join("out");
    let (written, errors) =
        frontend::run_conversion(Format::Rfs, Format::Json, &root.join("in"), Some(&td));
    assert!(errors.is_empty());
    let mut expected = vec![root.join("out/sub/deep.json"), root.join("out/top.json")];
    expected.sort();
    let mut got = written.clone();
    got.sort();
    assert_eq!(got, expected);
    cleanup(&root);
}

#[test]
fn directory_input_without_target_dir_writes_next_to_each_source() {
    let root = tmp_root("place_dir");
    write_file(&root, "d/top.rfs", VALID_RFS);
    write_file(&root, "d/sub/deep.rfs", VALID_RFS);
    let (written, errors) =
        frontend::run_conversion(Format::Rfs, Format::Json, &root.join("d"), None);
    assert!(errors.is_empty());
    let mut expected = vec![root.join("d/top.json"), root.join("d/sub/deep.json")];
    expected.sort();
    let mut got = written.clone();
    got.sort();
    assert_eq!(got, expected);
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// p.5: input/output collision guard
// ---------------------------------------------------------------------------

#[test]
fn normalize_abs_resolves_relative_components() {
    let a = frontend::normalize_abs(Path::new("x/../y.rfs"));
    let b = frontend::normalize_abs(Path::new("y.rfs"));
    assert_eq!(a, b);
    let c = frontend::normalize_abs(Path::new("./z/./w.json"));
    let d = frontend::normalize_abs(Path::new("z/w.json"));
    assert_eq!(c, d);
}

#[test]
fn in_place_same_format_conversion_is_rejected() {
    // rfs -> rfs without --target-dir would write the output on top of the
    // input; this must be refused.
    let root = tmp_root("collision_inplace");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Rfs, &f, None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].contains("refusing to overwrite"),
        "unexpected: {}",
        errors[0]
    );
    // Input untouched.
    assert_eq!(std::fs::read_to_string(&f).unwrap(), VALID_RFS);
    cleanup(&root);
}

#[test]
fn target_dir_equal_to_source_dir_is_rejected() {
    let root = tmp_root("collision_td");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let (written, errors) =
        frontend::run_conversion(Format::Rfs, Format::Rfs, &f, Some(root.as_path()));
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("refusing to overwrite"));
    cleanup(&root);
}

#[test]
fn directory_conversion_into_its_own_tree_is_rejected() {
    let root = tmp_root("collision_dir");
    write_file(&root, "d/a.rfs", VALID_RFS);
    // Converting d/ (rfs->rfs) into d/ itself collides for every file.
    let (written, errors) = frontend::run_conversion(
        Format::Rfs,
        Format::Rfs,
        &root.join("d"),
        Some(&root.join("d")),
    );
    assert!(written.is_empty());
    assert!(!errors.is_empty());
    assert!(errors.iter().all(|e| e.contains("refusing to overwrite")));
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// Conversion content correctness (all four format combinations)
// ---------------------------------------------------------------------------

fn check_roundtrip_source(source: &str, json: &str) {
    let ir = rfs::parse_source(source).expect("parse source");
    // rfs -> json
    let parsed_json = json_ir(json);
    assert_eq!(ir, parsed_json);
    // rfs -> json -> rfs and json -> rfs -> json stability.
    printer::roundtrip_rfs(source).expect("rfs round-trip");
    printer::roundtrip_json(json).expect("json round-trip");
}

#[test]
fn converts_rfs_to_json() {
    let root = tmp_root("conv_r2j");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &f, None);
    assert!(errors.is_empty());
    let out = written[0].clone();
    let json = std::fs::read_to_string(&out).unwrap();
    check_roundtrip_source(VALID_RFS, &json);
    // Pretty-printed with trailing newline.
    assert!(json.starts_with('{'));
    assert!(json.ends_with('\n'));
    cleanup(&root);
}

#[test]
fn converts_json_to_rfs() {
    let root = tmp_root("conv_j2r");
    let json = ir_json(VALID_RFS);
    let f = write_file(&root, "a.json", &json);
    let (written, errors) = frontend::run_conversion(Format::Json, Format::Rfs, &f, None);
    assert!(errors.is_empty());
    let out = written[0].clone();
    let rfs_text = std::fs::read_to_string(&out).unwrap();
    let ir1 = json_ir(&json);
    let ir2 = rfs::parse_source(&rfs_text).expect("printed rfs parses");
    assert_eq!(ir1, ir2);
    cleanup(&root);
}

#[test]
fn converts_rfs_to_rfs_with_target_dir() {
    let root = tmp_root("conv_r2r");
    let f = write_file(&root, "in/a.rfs", VALID_RFS);
    let td = root.join("out");
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Rfs, &f, Some(&td));
    assert!(errors.is_empty());
    let rfs_text = std::fs::read_to_string(&written[0]).unwrap();
    let ir1 = rfs::parse_source(VALID_RFS).unwrap();
    let ir2 = rfs::parse_source(&rfs_text).unwrap();
    assert_eq!(ir1, ir2);
    cleanup(&root);
}

#[test]
fn converts_json_to_json_with_target_dir() {
    // json -> json without --target-dir would collide with the input file,
    // so a target dir is used here (the collision case itself is covered by
    // in_place_same_format_conversion_is_rejected).
    let root = tmp_root("conv_j2j");
    let json = ir_json(VALID_RFS);
    let f = write_file(&root, "in/a.json", &json);
    let td = root.join("out");
    let (written, errors) = frontend::run_conversion(Format::Json, Format::Json, &f, Some(&td));
    assert!(errors.is_empty(), "unexpected errors: {:?}", errors);
    let out_text = std::fs::read_to_string(&written[0]).unwrap();
    let ir1 = json_ir(&json);
    let ir2 = json_ir(&out_text);
    assert_eq!(ir1, ir2);
    cleanup(&root);
}

#[test]
fn json_to_json_in_place_is_rejected_by_collision_guard() {
    let root = tmp_root("conv_j2j_collide");
    let f = write_file(&root, "a.json", &ir_json(VALID_RFS));
    let (written, errors) = frontend::run_conversion(Format::Json, Format::Json, &f, None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(
        errors[0].contains("refusing to overwrite"),
        "unexpected: {}",
        errors[0]
    );
    cleanup(&root);
}

#[test]
fn conversion_reports_errors_and_continues_other_files() {
    let root = tmp_root("conv_mixed");
    write_file(&root, "good.rfs", VALID_RFS);
    write_file(&root, "bad.rfs", UNPARSEABLE_RFS);
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &root, None);
    assert_eq!(written.len(), 1);
    assert!(written[0].ends_with("good.json"));
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("bad.rfs"));
    cleanup(&root);
}

#[test]
fn conversion_rejects_nonexistent_path() {
    let root = tmp_root("conv_missing");
    let (written, errors) =
        frontend::run_conversion(Format::Rfs, Format::Json, &root.join("nope.rfs"), None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    cleanup(&root);
}

#[test]
fn conversion_rejects_directory_without_matching_files() {
    let root = tmp_root("conv_empty");
    write_file(&root, "only.txt", "x");
    let (written, errors) = frontend::run_conversion(Format::Rfs, Format::Json, &root, None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("no .rfs files"));
    cleanup(&root);
}

#[test]
fn conversion_rejects_invalid_json_input() {
    let root = tmp_root("conv_badjson");
    let f = write_file(&root, "a.json", "{\"not\": \"an ir\"}");
    let (written, errors) = frontend::run_conversion(Format::Json, Format::Rfs, &f, None);
    assert!(written.is_empty());
    assert_eq!(errors.len(), 1);
    assert!(errors[0].contains("invalid RFS IR JSON"));
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// validate: file mode (p.1/p.2)
// ---------------------------------------------------------------------------

#[test]
fn validate_valid_rfs_file() {
    let root = tmp_root("val_rfs_ok");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let res = frontend::run_validate(None, &f).expect("no misuse error");
    assert!(!res.is_directory);
    assert_eq!(res.ok_files, vec![f.clone()]);
    assert!(res.failed.is_empty());
    cleanup(&root);
}

#[test]
fn validate_valid_json_file() {
    let root = tmp_root("val_json_ok");
    let f = write_file(&root, "a.json", &ir_json(VALID_RFS));
    let res = frontend::run_validate(None, &f).expect("no misuse error");
    assert_eq!(res.ok_files, vec![f.clone()]);
    assert!(res.failed.is_empty());
    cleanup(&root);
}

#[test]
fn validate_invalid_rfs_file_reports_errors() {
    let root = tmp_root("val_rfs_bad");
    let f = write_file(&root, "a.rfs", BAD_STATE_RFS);
    let res = frontend::run_validate(None, &f).expect("no misuse error");
    assert!(res.ok_files.is_empty());
    assert_eq!(res.failed.len(), 1);
    assert_eq!(res.failed[0].path, f);
    assert!(!res.failed[0].errors.is_empty());
    let joined: String = res.failed[0].errors.join("\n");
    assert!(
        joined.contains("Nope"),
        "expected unknown-state error, got: {joined}"
    );
    cleanup(&root);
}

#[test]
fn validate_unparseable_file_reports_error() {
    let root = tmp_root("val_parse_err");
    let f = write_file(&root, "a.rfs", UNPARSEABLE_RFS);
    let res = frontend::run_validate(None, &f).expect("no misuse error");
    assert_eq!(res.failed.len(), 1);
    assert!(!res.failed[0].errors.is_empty());
    cleanup(&root);
}

#[test]
fn validate_file_with_conflicting_input_format_is_misuse() {
    let root = tmp_root("val_conflict");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let err = frontend::run_validate(Some(Format::Json), &f).expect_err("must be misuse");
    assert!(err[0].contains("conflicts"));
    cleanup(&root);
}

#[test]
fn validate_file_with_unknown_extension_is_misuse() {
    let root = tmp_root("val_badext");
    let f = write_file(&root, "a.txt", VALID_RFS);
    let err = frontend::run_validate(None, &f).expect_err("must be misuse");
    assert!(err[0].contains(".rfs or .json"));
    cleanup(&root);
}

#[test]
fn validate_nonexistent_path_is_misuse() {
    let root = tmp_root("val_missing");
    let err = frontend::run_validate(None, &root.join("nope.rfs")).expect_err("must be misuse");
    assert_eq!(err.len(), 1);
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// validate: directory mode and *.rfs-validation.err files (p.2)
// ---------------------------------------------------------------------------

#[test]
fn validate_directory_requires_input_format() {
    let root = tmp_root("val_dir_needs_fmt");
    write_file(&root, "a.rfs", VALID_RFS);
    let err = frontend::run_validate(None, &root).expect_err("must be misuse");
    assert!(err[0].contains("--input"));
    cleanup(&root);
}

#[test]
fn validate_directory_writes_err_files_next_to_failures() {
    let root = tmp_root("val_dir_err");
    write_file(&root, "good.rfs", VALID_RFS);
    let bad = write_file(&root, "bad.rfs", BAD_STATE_RFS);
    let res = frontend::run_validate(Some(Format::Rfs), &root).expect("no misuse error");
    assert!(res.is_directory);
    assert_eq!(res.ok_files.len(), 1);
    assert_eq!(res.failed.len(), 1);
    assert_eq!(res.failed[0].path, bad);

    // Error file next to the failed source: <name>.rfs-validation.err
    let err_path = root.join("bad.rfs.rfs-validation.err");
    assert!(err_path.exists(), "missing {}", err_path.display());
    let content = std::fs::read_to_string(&err_path).unwrap();
    assert!(content.contains("bad.rfs"));
    assert!(content.contains("Nope"), "unexpected content: {content}");

    // No error file for the passing source.
    assert!(!root.join("good.rfs.rfs-validation.err").exists());
    cleanup(&root);
}

#[test]
fn validate_json_directory_writes_err_files_with_json_name() {
    let root = tmp_root("val_dir_json");
    write_file(&root, "ok.json", &ir_json(VALID_RFS));
    // Tampered IR: valid JSON but invalid semantics (unknown next state).
    let tampered = ir_json(BAD_STATE_RFS);
    assert!(tampered.contains("\"Nope\""));
    let bad = write_file(&root, "bad.json", &tampered);
    let res = frontend::run_validate(Some(Format::Json), &root).expect("no misuse error");
    assert_eq!(res.ok_files.len(), 1);
    assert_eq!(res.failed.len(), 1);
    assert_eq!(res.failed[0].path, bad);
    let err_path = root.join("bad.json.rfs-validation.err");
    assert!(err_path.exists(), "missing {}", err_path.display());
    cleanup(&root);
}

#[test]
fn validate_directory_all_valid_has_no_err_files() {
    let root = tmp_root("val_dir_all_ok");
    write_file(&root, "a.rfs", VALID_RFS);
    write_file(&root, "sub/b.rfs", VALID_RFS);
    let res = frontend::run_validate(Some(Format::Rfs), &root).expect("no misuse error");
    assert_eq!(res.ok_files.len(), 2);
    assert!(res.failed.is_empty());
    assert!(!root.join("a.rfs.rfs-validation.err").exists());
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// Positive/negative validator behavior through the frontend (task 2.4)
// ---------------------------------------------------------------------------

#[test]
fn positive_specs_pass_validation() {
    let specs = [
        VALID_RFS,
        "module t { enum State { A } initial A event Tick var n: bit<8> init 0 transition A on Tick { next A; set n = n + 1; } prop p : always (state == A) ; assume e : always (bounded_eventually[5] (input_is(Tick))) ; }",
        "module t2 { enum State { A, B } initial A event E(V(x: bit<4>)) output O(W(y: bool)) var v: bit<4> init 0 transition A on E(V(x)) where x > 0 { next B; emit O(W(true)); set v = x; } }",
    ];
    for (i, s) in specs.iter().enumerate() {
        let root = tmp_root(&format!("val_pos_{i}"));
        let f = write_file(&root, "a.rfs", s);
        let res = frontend::run_validate(None, &f).expect("no misuse error");
        assert!(
            res.failed.is_empty(),
            "spec {i} should validate: {:?}",
            res.failed
        );
        cleanup(&root);
    }
}

#[test]
fn negative_specs_fail_validation_with_messages() {
    let cases: [(&str, &str); 4] = [
        ("unknown next state", "module m { enum State { A } initial A event E transition A on E { next Nope; } }"),
        ("unknown event group", "module m { enum State { A } initial A event E transition A on Missing { next A; } }"),
        ("unknown var in guard", "module m { enum State { A } initial A event E transition A on E where ghost == 1 { next A; } }"),
        (
            "set unknown variable",
            "module m { enum State { A } initial A event E transition A on E { next A; set ghost = 1; } }",
        ),
    ];
    for (i, (label, src)) in cases.iter().enumerate() {
        let root = tmp_root(&format!("val_neg_{i}"));
        let f = write_file(&root, "a.rfs", src);
        let res = frontend::run_validate(None, &f).expect("no misuse error");
        assert!(
            res.failed.len() == 1 && !res.failed[0].errors.is_empty(),
            "case {label} should fail validation"
        );
        cleanup(&root);
    }
}

// ---------------------------------------------------------------------------
// End-to-end: the real binary, exit codes and streams (p.6)
// ---------------------------------------------------------------------------

fn rfs_bin() -> &'static str {
    env!("CARGO_BIN_EXE_rfs")
}

#[test]
fn e2e_parse_rfs_to_json_exits_zero_and_writes_output() {
    let root = tmp_root("e2e_r2j");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["parse", "--input", "rfs", "--output", "json"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let out_file = root.join("a.json");
    assert!(out_file.exists());
    let ir1 = rfs::parse_source(VALID_RFS).unwrap();
    let ir2 = json_ir(&std::fs::read_to_string(&out_file).unwrap());
    assert_eq!(ir1, ir2);
    cleanup(&root);
}

#[test]
fn e2e_parse_in_place_same_format_exits_one() {
    let root = tmp_root("e2e_collision");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["parse", "--input", "rfs", "--output", "rfs"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("refusing to overwrite"), "stderr: {stderr}");
    // Input untouched.
    assert_eq!(std::fs::read_to_string(&f).unwrap(), VALID_RFS);
    cleanup(&root);
}

#[test]
fn e2e_parse_missing_flags_exits_one() {
    let out = std::process::Command::new(rfs_bin())
        .args(["parse", "examples/valid"])
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("--input"), "stderr: {stderr}");
}

#[test]
fn e2e_validate_valid_file_exits_zero() {
    let root = tmp_root("e2e_val_ok");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["validate"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    cleanup(&root);
}

#[test]
fn e2e_validate_invalid_file_exits_one_with_stderr() {
    let root = tmp_root("e2e_val_bad");
    let f = write_file(&root, "a.rfs", BAD_STATE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["validate"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("Nope"), "stderr: {stderr}");
    cleanup(&root);
}

#[test]
fn e2e_validate_directory_writes_err_file_and_exits_one() {
    let root = tmp_root("e2e_val_dir");
    write_file(&root, "good.rfs", VALID_RFS);
    write_file(&root, "bad.rfs", BAD_STATE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["validate", "--input", "rfs"])
        .arg(&root)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    assert!(root.join("bad.rfs.rfs-validation.err").exists());
    assert!(!root.join("good.rfs.rfs-validation.err").exists());
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// Phase 5 task 2: IR JSON golden pins (hand-rolled json_rw, no external crates)
// ---------------------------------------------------------------------------

/// Representative spec covering enums, bit types, unit + struct event/output
/// variants, vars, guards, emits with args, assignments, and an liveness
/// property. Its exact JSON text is PINNED here: declaration-order keys,
/// kind-first tagging, nulls for absent Option fields, 2-space pretty layout.
const GOLDEN_SPEC_RFS: &str = "module m {\n  enum State { Idle, Busy } initial Idle\n  event Req(Value(x: bit<8>))\n  event Ping\n  output Done(Ok(code: bit<4>))\n  var n: bit<8> init 0\n  transition Idle on Req(Value(x)) where x > 0 { next Busy; emit Done(Ok(1)); set n = x + 1; }\n  transition Busy on Ping { next Idle; }\n  prop p1 : always (state == Idle -> eventually (state == Busy)) ;\n}";

#[test]
fn ir_json_golden_compact_pins_key_order() {
    let ir = rfs::parse_source(GOLDEN_SPEC_RFS).expect("parse golden spec");
    assert_eq!(
        ir.to_json().expect("to_json").to_string(),
        r#"{"ir_version":0,"modules":[{"name":"m","enums":[{"name":"State","variants":["Idle","Busy"]}],"initial_state":"Idle","events":[{"name":"Req","variants":[{"kind":"struct","name":"Value","fields":[{"name":"x","ty":{"kind":"bit","width":8}}]}]},{"name":"Ping","variants":[{"kind":"unit","name":"Ping"}]}],"outputs":[{"name":"Done","variants":[{"kind":"struct","name":"Ok","fields":[{"name":"code","ty":{"kind":"bit","width":4}}]}]}],"vars":[{"name":"n","ty":{"kind":"bit","width":8},"init":{"kind":"int","value":0}}],"transitions":[{"id":0,"state":"Idle","pattern":{"group":"Req","variant":{"kind":"struct","name":"Value","fields":[{"name":"x","pattern":{"kind":"bind","name":"x"}}]}},"guard":{"kind":"binary","op":"gt","left":{"kind":"path","path":["x"]},"right":{"kind":"int","value":0}},"next_state":"Busy","emits":[{"group":"Done","variant":"Ok","args":[{"kind":"int","value":1}]}],"assignments":[{"var":"n","value":{"kind":"binary","op":"plus","left":{"kind":"path","path":["x"]},"right":{"kind":"int","value":1}}}],"priority":null},{"id":1,"state":"Busy","pattern":{"group":"Ping","variant":null},"guard":null,"next_state":"Idle","emits":[],"assignments":[],"priority":null}],"properties":[{"name":"p1","formula":{"kind":"always","inner":{"kind":"implies","left":{"kind":"expr","expr":{"kind":"binary","op":"eq","left":{"kind":"path","path":["state"]},"right":{"kind":"path","path":["Idle"]}}},"right":{"kind":"eventually","inner":{"kind":"expr","expr":{"kind":"binary","op":"eq","left":{"kind":"path","path":["state"]},"right":{"kind":"path","path":["Busy"]}}}}}}}],"assumptions":[]}]}"#
    );
}

#[test]
fn ir_json_golden_pretty_pins_layout() {
    let ir = rfs::parse_source(GOLDEN_SPEC_RFS).expect("parse golden spec");
    let pretty = ir.to_json().expect("to_json").to_string_pretty();
    assert_eq!(
        pretty,
        r#"{
  "ir_version": 0,
  "modules": [
    {
      "name": "m",
      "enums": [
        {
          "name": "State",
          "variants": [
            "Idle",
            "Busy"
          ]
        }
      ],
      "initial_state": "Idle",
      "events": [
        {
          "name": "Req",
          "variants": [
            {
              "kind": "struct",
              "name": "Value",
              "fields": [
                {
                  "name": "x",
                  "ty": {
                    "kind": "bit",
                    "width": 8
                  }
                }
              ]
            }
          ]
        },
        {
          "name": "Ping",
          "variants": [
            {
              "kind": "unit",
              "name": "Ping"
            }
          ]
        }
      ],
      "outputs": [
        {
          "name": "Done",
          "variants": [
            {
              "kind": "struct",
              "name": "Ok",
              "fields": [
                {
                  "name": "code",
                  "ty": {
                    "kind": "bit",
                    "width": 4
                  }
                }
              ]
            }
          ]
        }
      ],
      "vars": [
        {
          "name": "n",
          "ty": {
            "kind": "bit",
            "width": 8
          },
          "init": {
            "kind": "int",
            "value": 0
          }
        }
      ],
      "transitions": [
        {
          "id": 0,
          "state": "Idle",
          "pattern": {
            "group": "Req",
            "variant": {
              "kind": "struct",
              "name": "Value",
              "fields": [
                {
                  "name": "x",
                  "pattern": {
                    "kind": "bind",
                    "name": "x"
                  }
                }
              ]
            }
          },
          "guard": {
            "kind": "binary",
            "op": "gt",
            "left": {
              "kind": "path",
              "path": [
                "x"
              ]
            },
            "right": {
              "kind": "int",
              "value": 0
            }
          },
          "next_state": "Busy",
          "emits": [
            {
              "group": "Done",
              "variant": "Ok",
              "args": [
                {
                  "kind": "int",
                  "value": 1
                }
              ]
            }
          ],
          "assignments": [
            {
              "var": "n",
              "value": {
                "kind": "binary",
                "op": "plus",
                "left": {
                  "kind": "path",
                  "path": [
                    "x"
                  ]
                },
                "right": {
                  "kind": "int",
                  "value": 1
                }
              }
            }
          ],
          "priority": null
        },
        {
          "id": 1,
          "state": "Busy",
          "pattern": {
            "group": "Ping",
            "variant": null
          },
          "guard": null,
          "next_state": "Idle",
          "emits": [],
          "assignments": [],
          "priority": null
        }
      ],
      "properties": [
        {
          "name": "p1",
          "formula": {
            "kind": "always",
            "inner": {
              "kind": "implies",
              "left": {
                "kind": "expr",
                "expr": {
                  "kind": "binary",
                  "op": "eq",
                  "left": {
                    "kind": "path",
                    "path": [
                      "state"
                    ]
                  },
                  "right": {
                    "kind": "path",
                    "path": [
                      "Idle"
                    ]
                  }
                }
              },
              "right": {
                "kind": "eventually",
                "inner": {
                  "kind": "expr",
                  "expr": {
                    "kind": "binary",
                    "op": "eq",
                    "left": {
                      "kind": "path",
                      "path": [
                        "state"
                      ]
                    },
                    "right": {
                      "kind": "path",
                      "path": [
                        "Busy"
                      ]
                    }
                  }
                }
              }
            }
          }
        }
      ],
      "assumptions": []
    }
  ]
}"#
    );
}

#[test]
fn ir_json_roundtrip_property_all_valid_examples() {
    // Phase 5 task 2 exit criterion: for every valid example,
    // parse rfs -> to_json -> from_json equals the original IR.
    let dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("examples")
        .join("valid");
    let files = rfs::collect_rfs_files(&dir).expect("valid examples directory");
    assert!(
        files.len() >= 20,
        "expected at least 20 valid examples, found {}",
        files.len()
    );
    for file in &files {
        let source = std::fs::read_to_string(file)
            .unwrap_or_else(|e| panic!("failed to read {}: {}", file.display(), e));
        let ir = rfs::parse_source(&source)
            .unwrap_or_else(|e| panic!("failed to parse {}: {}", file.display(), e.rich_display()));
        let value = ir
            .to_json()
            .unwrap_or_else(|e| panic!("to_json failed for {}: {}", file.display(), e));
        let back = Ir::from_json(&value)
            .unwrap_or_else(|e| panic!("from_json failed for {}: {}", file.display(), e));
        assert_eq!(
            ir,
            back,
            "IR round-trip via JSON changed the IR for {}",
            file.display()
        );
    }
}

// ---------------------------------------------------------------------------
// `rfs info` (Phase 5 task 3): static spec introspection
// ---------------------------------------------------------------------------

fn example_root_dir() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("examples")
}

#[test]
fn info_text_on_all_valid_examples() {
    let dir = example_root_dir().join("valid");
    let files = rfs::collect_rfs_files(&dir).expect("valid examples directory");
    assert!(files.len() >= 20, "expected the valid examples set");
    for f in &files {
        let out = rfs::info::run_info(f, &rfs::info::InfoOptions { json: false })
            .unwrap_or_else(|e| panic!("info failed on {}: {}", f.display(), e));
        assert!(
            !out.trim().is_empty(),
            "empty info output for {}",
            f.display()
        );
        assert!(
            out.starts_with("module "),
            "info text for {} must start with the module header: {out}",
            f.display()
        );
        // Every section header is present (stable layout).
        for sec in [
            "enums:",
            "events:",
            "outputs:",
            "variables:",
            "transitions:",
            "properties:",
            "assumptions:",
        ] {
            assert!(
                out.contains(sec),
                "missing '{sec}' in info for {}: {out}",
                f.display()
            );
        }
    }
}

#[test]
fn info_json_parses_and_matches_pinned_example() {
    // Pinned example: exercises BOTH monitor statuses (safety props are
    // monitored; top-level eventually/until liveness is skipped R1-R4).
    let f = example_root_dir().join("valid").join("44_token_ring.rfs");
    let opts = rfs::info::InfoOptions { json: true };
    let text = rfs::info::run_info(&f, &opts).expect("info --json");
    let doc = rfs::json_rw::parse(&text).unwrap_or_else(|e| panic!("info JSON must parse: {e}"));

    // Data consistency against the IR parsed directly from the same source.
    let ir =
        rfs::parse_source(&std::fs::read_to_string(&f).expect("read spec")).expect("parse spec");
    assert_eq!(ir.modules.len(), 1);
    let m = &ir.modules[0];

    let modules = doc
        .get("modules")
        .and_then(rfs::json_rw::Json::as_arr)
        .expect("modules array");
    assert_eq!(modules.len(), 1);
    let jm = &modules[0];
    assert_eq!(
        jm.get("name").and_then(rfs::json_rw::Json::as_str),
        Some(m.name.as_str())
    );
    assert_eq!(
        jm.get("initial_state").and_then(rfs::json_rw::Json::as_str),
        m.initial_state.as_deref()
    );
    assert_eq!(
        jm.get("transitions").and_then(rfs::json_rw::Json::as_int),
        Some(m.transitions.len() as i64)
    );

    // Enums and events/outputs match the IR (names + variant counts).
    let j_enums = jm
        .get("enums")
        .and_then(rfs::json_rw::Json::as_arr)
        .expect("enums");
    assert_eq!(j_enums.len(), m.enums.len());
    for (je, e) in j_enums.iter().zip(&m.enums) {
        assert_eq!(
            je.get("name").and_then(rfs::json_rw::Json::as_str),
            Some(e.name.as_str())
        );
        let variants = je
            .get("variants")
            .and_then(rfs::json_rw::Json::as_arr)
            .expect("variants");
        assert_eq!(variants.len(), e.variants.len());
    }
    for (key, groups) in [("events", &m.events), ("outputs", &m.outputs)] {
        let jg = jm.get(key).and_then(rfs::json_rw::Json::as_arr).expect(key);
        assert_eq!(jg.len(), groups.len(), "{key} count");
        for (jx, g) in jg.iter().zip(groups) {
            assert_eq!(
                jx.get("name").and_then(rfs::json_rw::Json::as_str),
                Some(g.name.as_str())
            );
            let variants = jx
                .get("variants")
                .and_then(rfs::json_rw::Json::as_arr)
                .expect("variants");
            assert_eq!(variants.len(), g.variants.len(), "{key} variant count");
        }
    }

    // Properties: every entry carries a monitor_status that is either
    // "monitored" or "skipped: <reason>"; this spec has BOTH kinds.
    let props = jm
        .get("properties")
        .and_then(rfs::json_rw::Json::as_arr)
        .expect("properties");
    assert!(!props.is_empty());
    let mut has_monitored = false;
    let mut has_skipped = false;
    for jp in props {
        let status = jp
            .get("monitor_status")
            .and_then(rfs::json_rw::Json::as_str)
            .expect("monitor_status");
        if status == "monitored" {
            has_monitored = true;
        } else {
            assert!(
                status.starts_with("skipped: "),
                "bad monitor_status: {status}"
            );
            has_skipped = true;
        }
    }
    assert!(
        has_monitored && has_skipped,
        "pinned spec must exercise both statuses"
    );

    // JSON is pretty-printed (2-space indent) per the shared writer.
    assert!(text.starts_with("{\n  \"modules\""));
}

#[test]
fn e2e_info_valid_exits_zero_and_prints_module() {
    let root = tmp_root("e2e_info_ok");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["info"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("module m"), "stdout: {stdout}");
    assert!(stdout.contains("transitions: 1"), "stdout: {stdout}");
    cleanup(&root);
}

#[test]
fn e2e_info_invalid_spec_exits_one_with_diagnostics() {
    let root = tmp_root("e2e_info_bad");
    let f = write_file(&root, "bad.rfs", BAD_STATE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["info"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("error:"), "stderr: {stderr}");
    assert!(
        stderr.contains("Nope"),
        "diagnostic must name the bad state: {stderr}"
    );
    cleanup(&root);
}

#[test]
fn e2e_info_unparseable_spec_exits_one() {
    let root = tmp_root("e2e_info_unparsable");
    let f = write_file(&root, "bad.rfs", UNPARSEABLE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["info"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("error:"));
    cleanup(&root);
}

#[test]
fn e2e_info_log_level_filters_stderr_progress() {
    let root = tmp_root("e2e_info_log");
    let f = write_file(&root, "a.rfs", VALID_RFS);

    // Default level (info): the debug progress line is NOT printed.
    let out = std::process::Command::new(rfs_bin())
        .args(["info"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(out.status.success());
    assert!(
        !String::from_utf8_lossy(&out.stderr).contains("debug:"),
        "default level must hide debug"
    );

    // --log-level debug: it IS printed on stderr.
    let out = std::process::Command::new(rfs_bin())
        .args(["info", "--log-level", "debug"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).contains("debug:"),
        "debug level must print progress"
    );

    // --log-level off: nothing on stderr at all.
    let out = std::process::Command::new(rfs_bin())
        .args(["info", "--log-level", "off"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(out.status.success());
    assert!(
        String::from_utf8_lossy(&out.stderr).is_empty(),
        "off level must print nothing"
    );

    // Bad level value -> exit 1 with a diagnostic.
    let out = std::process::Command::new(rfs_bin())
        .args(["info", "--log-level", "chatty"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("--log-level"));
    cleanup(&root);
}

#[test]
fn e2e_info_usage_lists_info_and_rejects_unknown_option() {
    // No args: usage printed, exit 1, and `info` is listed.
    let out = std::process::Command::new(rfs_bin())
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(
        stdout.contains("rfs info"),
        "usage must list info: {stdout}"
    );

    // Unknown option -> exit 1.
    let root = tmp_root("e2e_info_opt");
    let f = write_file(&root, "a.rfs", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["info", "--bogus"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown option: --bogus"));
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// rfs diagram (Phase 5 task 8)
// ---------------------------------------------------------------------------

/// Small hand-written spec exercising every feature the generators must map:
/// 3 states, initial state, unit + struct-variant events, a guard, emitted
/// outputs, and an on_error fallback with its own emit.
const PIN_DIAGRAM_RFS: &str = "module pin_fsm {
  enum State { Idle, Active, Error }
  initial Idle

  event Tick
  event Cmd(Go(level: bit<4>))

  output Ack(Ack(code: bit<8>))

  var counter: bit<8> init 0

  transition Idle on Tick {
    next Active;
    set counter = counter + 1;
  }
  transition Active on Cmd(Go(lvl)) where lvl > 2 {
    emit Ack(Ack(counter));
    next Active;
  }
  transition Active on Tick {
    next Idle;
  }

  on_error {
    next Error;
    emit Ack(Ack(99));
  }
}\n";

fn diagram_opts(format: &str) -> rfs::diagram::DiagramOptions {
    rfs::diagram::DiagramOptions {
        format: format.to_string(),
        ..Default::default()
    }
}

fn diagram_stdout(spec: &std::path::Path, format: &str) -> String {
    let result = rfs::diagram::run_diagram(spec, &diagram_opts(format))
        .unwrap_or_else(|e| panic!("{format}: {e}"));
    result.stdout.expect("text mode emits stdout")
}

#[test]
fn diagram_pinned_dot_output() {
    let root = tmp_root("diag_pin_dot");
    let f = write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    assert_eq!(
        diagram_stdout(&f, "dot"),
        "digraph pin_fsm {
  rankdir=LR;
  node [shape=box];
  \"Idle\" [style=filled, fillcolor=\"#e0e0e0\"];
  \"Active\";
  \"Error\";
  \"Idle\" -> \"Active\" [label=\"Tick -> Active\"];
  \"Active\" -> \"Active\" [label=\"Cmd(Go(lvl)) [(lvl > 2)] -> Active\"];
  \"Active\" -> \"Idle\" [label=\"Tick -> Idle\"];
  \"Idle\" -> \"Error\" [label=\"on_error -> Error\", style=dashed];
  \"Active\" -> \"Error\" [label=\"on_error -> Error\", style=dashed];
  \"Error\" -> \"Error\" [label=\"on_error -> Error\", style=dashed];
}\n"
    );
    cleanup(&root);
}

#[test]
fn diagram_pinned_puml_output() {
    let root = tmp_root("diag_pin_puml");
    let f = write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    assert_eq!(
        diagram_stdout(&f, "puml"),
        "@startuml
title pin_fsm
[*] --> Idle
Idle --> Active : Tick
Active --> Active : Cmd(Go(lvl)) \\n (lvl > 2)
note right on link : emits Ack(Ack(counter))
Active --> Idle : Tick
Idle -[dashed]-> Error : on_error
note right on link : emits Ack(Ack(99))
Active -[dashed]-> Error : on_error
note right on link : emits Ack(Ack(99))
Error -[dashed]-> Error : on_error
note right on link : emits Ack(Ack(99))
@enduml\n"
    );
    cleanup(&root);
}

#[test]
fn diagram_pinned_mmd_output() {
    let root = tmp_root("diag_pin_mmd");
    let f = write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    assert_eq!(
        diagram_stdout(&f, "mmd"),
        "stateDiagram-v2
  [*] --> Idle
  Idle --> Active: Tick
  Active --> Active: Cmd(Go(lvl)) \\n (lvl > 2) \\n emits Ack(Ack(counter))
  Active --> Idle: Tick
  Idle --> Error: [on_error] \\n emits Ack(Ack(99))
  Active --> Error: [on_error] \\n emits Ack(Ack(99))
  Error --> Error: [on_error] \\n emits Ack(Ack(99))\n"
    );
    cleanup(&root);
}

#[test]
fn diagram_pinned_table_output() {
    let root = tmp_root("diag_pin_table");
    let f = write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    assert_eq!(
        diagram_stdout(&f, "table"),
        "Module: pin_fsm
States: Idle, Active, Error
Initial: Idle

Transition matrix (rows = states, columns = events):

state  | Tick      | Cmd(Go(lvl))
-------+----------+------------------------------------------------
Idle   | -> Active | -
Active | -> Idle | [(lvl > 2)] -> Active; emits Ack(Ack(counter))
Error  | - | -

on_error (from any state): -> Error; emits Ack(Ack(99))

Variables:
  counter: bit<8> = 0
Events:
  Tick
  Cmd: Go(level: bit<4>)
Outputs:
  Ack: Ack(code: bit<8>)\n"
    );
    cleanup(&root);
}

#[test]
fn diagram_pinned_md_output() {
    let root = tmp_root("diag_pin_md");
    let f = write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    assert_eq!(
        diagram_stdout(&f, "md"),
        "# pin_fsm - transition table

**States:** Idle, Active, Error

**Initial state:** Idle

## Transition matrix

| state | Tick | Cmd(Go(lvl)) |
| --- | --- | --- |
| Idle | -> Active | - |
| Active | -> Idle | [(lvl > 2)] -> Active; emits Ack(Ack(counter)) |
| Error | - | - |

## on_error

From any state: `-> Error`; emits Ack(Ack(99))

## Variables

- `counter: bit<8>` = `0`

## Events

- `Tick`
- `Cmd: Go(level: bit<4>)`

## Outputs

- `Ack: Ack(code: bit<8>)`\n"
    );
    cleanup(&root);
}

#[test]
fn diagram_all_formats_sweep_valid_examples() {
    let files = rfs::collect_rfs_files("examples/valid").expect("collect examples");
    assert!(
        files.len() >= 45,
        "expected the full example set, got {}",
        files.len()
    );
    for f in &files {
        for format in ["dot", "puml", "mmd", "table", "md"] {
            let result = rfs::diagram::run_diagram(f, &diagram_opts(format))
                .unwrap_or_else(|e| panic!("{format} on {}: {e}", f.display()));
            let text = result.stdout.expect("text mode emits stdout");
            assert!(
                !text.is_empty(),
                "{format} on {} emitted nothing",
                f.display()
            );
        }
    }
}

/// True when a bare tool name resolves on PATH (for gating real-tool tests).
fn tool_on_path(name: &str) -> bool {
    std::env::var_os("PATH")
        .map(|paths| std::env::split_paths(&paths).any(|dir| dir.join(name).is_file()))
        .unwrap_or(false)
}

fn write_stub(path: &std::path::Path, body: &str) {
    std::fs::write(path, body).unwrap();
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        let mut p = std::fs::metadata(path).unwrap().permissions();
        p.set_mode(0o755);
        std::fs::set_permissions(path, p).unwrap();
    }
}

#[test]
fn e2e_diagram_valid_exits_zero_and_prints() {
    let root = tmp_root("e2e_diag_ok");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.starts_with("digraph pin_fsm"), "stdout: {stdout}");
    cleanup(&root);
}

#[test]
fn e2e_diagram_invalid_spec_exits_one() {
    let root = tmp_root("e2e_diag_bad");
    let f = write_file(&root, "bad.rfs", BAD_STATE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("error:"));
    cleanup(&root);
}

#[test]
fn e2e_diagram_render_dot_real_tool() {
    let _g = diag_lock();
    clear_tool_env();
    if !tool_on_path("dot") {
        eprintln!("skipping: dot not on PATH");
        return;
    }
    let root = tmp_root("e2e_diag_dot");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--render", "svg", "--out"])
        .arg(&svg)
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let src = root.join("out.dot");
    assert!(src.is_file(), "source written next to the SVG");
    let text = std::fs::read_to_string(&svg).expect("svg exists");
    assert!(text.contains("pin_fsm"), "svg content: state/label missing");
    assert!(text.contains("Cmd(Go(lvl))"), "event label must render");
    cleanup(&root);
}

#[test]
fn e2e_diagram_render_puml_real_tool() {
    let _g = diag_lock();
    clear_tool_env();
    if !tool_on_path("plantuml") {
        eprintln!("skipping: plantuml not on PATH");
        return;
    }
    let root = tmp_root("e2e_diag_puml");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--format", "puml", "--render", "svg", "--out"])
        .arg(&svg)
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let src = root.join("out.puml");
    assert!(src.is_file(), "puml source written next to the SVG");
    let text = std::fs::read_to_string(&svg).expect("svg exists");
    // The real plantuml accepted our syntax (exit 0) AND rendered the content.
    assert!(text.contains("Cmd(Go(lvl))"), "event label must render");
    assert!(text.contains("lvl > 2"), "guard must render");
    assert!(
        text.contains("emits Ack(Ack(counter))"),
        "note-on-link must render"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_render_mmd_real_tool() {
    let _g = diag_lock();
    clear_tool_env();
    if !tool_on_path("mmdc") {
        eprintln!("skipping: mmdc not on PATH");
        return;
    }
    let root = tmp_root("e2e_diag_mmd");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--format", "mmd", "--render", "svg", "--out"])
        .arg(&svg)
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let src = root.join("out.mmd");
    assert!(src.is_file(), "mmd source written next to the SVG");
    let text = std::fs::read_to_string(&svg).expect("svg exists");
    // The real mmdc accepted our syntax (exit 0) AND rendered the content
    // (mermaid's grammar is loose, so exit code alone would not prove it).
    assert!(text.contains("Cmd(Go(lvl))"), "event label must render");
    assert!(
        text.contains("emits Ack(Ack(counter))"),
        "emitted data must render"
    );
    cleanup(&root);
}

/// Negative control: the harness MUST detect a plantuml syntax error (exit 200)
/// - and NOT be fooled by the error-SVG file plantuml still writes.
#[test]
fn diagram_puml_negative_control_exit_200() {
    if !tool_on_path("plantuml") {
        eprintln!("skipping: plantuml not on PATH");
        return;
    }
    let root = tmp_root("diag_puml_neg");
    let bad = root.join("bad.puml");
    std::fs::write(&bad, "@startuml\nthis is not valid puml @@@\n@enduml\n").unwrap();
    let svg = root.join("bad.svg");
    let tool = rfs::ext_tool::ExtTool::PLANTUML.resolve(None, None);
    let err = rfs::diagrams::plantuml::render_svg(&tool, &bad, &svg)
        .expect_err("syntax error must be detected");
    let joined = err.join("; ");
    assert!(joined.contains("200"), "exit code in diagnostic: {joined}");
    assert!(
        joined.contains("Error line"),
        "tool stderr captured: {joined}"
    );
    cleanup(&root);
}

/// Negative control: the harness MUST detect a mermaid parse error (exit 1).
#[test]
fn diagram_mmd_negative_control_parse_error() {
    if !tool_on_path("mmdc") {
        eprintln!("skipping: mmdc not on PATH");
        return;
    }
    let root = tmp_root("diag_mmd_neg");
    let bad = root.join("bad.mmd");
    std::fs::write(
        &bad,
        "stateDiagram-v2\n  [*] --> A\n  state B {\n    B1 --> B2\n",
    )
    .unwrap();
    let svg = root.join("bad.svg");
    let tool = rfs::ext_tool::ExtTool::MERMAID.resolve(None, None);
    let err = rfs::diagrams::mermaid::render_svg(&tool, &bad, &svg)
        .expect_err("parse error must be detected");
    let joined = err.join("; ");
    assert!(
        joined.contains("Parse error"),
        "tool stderr captured: {joined}"
    );
    cleanup(&root);
}

/// Env-mutating tests share process-global state - serialize them.
static DIAG_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

/// Clear every diagram tool env var so ambient settings can't leak in.
fn clear_tool_env() {
    set_env(&[
        ("RFS_DOT", None),
        ("RFS_DOT_ARGS", None),
        ("RFS_PLANTUML", None),
        ("RFS_PLANTUML_ARGS", None),
        ("RFS_MERMAID", None),
        ("RFS_MERMAID_ARGS", None),
    ]);
}

/// Take the diagram env lock, recovering from poisoning so one panicking
/// test cannot poison every later env-sensitive test.
fn diag_lock() -> std::sync::MutexGuard<'static, ()> {
    DIAG_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

fn set_env(pairs: &[(&str, Option<&str>)]) {
    for (k, v) in pairs {
        match v {
            Some(val) => std::env::set_var(k, val),
            None => std::env::remove_var(k),
        }
    }
}

#[test]
fn e2e_diagram_stub_plantuml_receives_env_and_cli_args() {
    let _g = diag_lock();
    clear_tool_env();
    let root = tmp_root("diag_stub_puml");
    let argv_file = root.join("argv.txt");
    let stub = root.join("stub_plantuml");
    write_stub(
        &stub,
        &format!("#!/bin/sh\necho \"$@\" > {}\nexit 0\n", argv_file.display()),
    );
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    set_env(&[
        ("RFS_PLANTUML", Some(stub.to_str().unwrap())),
        ("RFS_PLANTUML_ARGS", Some("-env1 -env2")),
    ]);
    let out = std::process::Command::new(rfs_bin())
        .args([
            "diagram",
            "--format",
            "puml",
            "--render",
            "svg",
            "--out",
            svg.to_str().unwrap(),
            "--plantuml-args",
            "-cli1",
        ])
        .arg(&f)
        .output()
        .expect("run rfs");
    set_env(&[("RFS_PLANTUML", None), ("RFS_PLANTUML_ARGS", None)]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let argv = std::fs::read_to_string(&argv_file)
        .expect("stub ran")
        .trim()
        .to_string();
    // env args first, CLI args appended, then the real arguments.
    let src_path = root.join("out.puml");
    assert_eq!(
        argv,
        format!("-env1 -env2 -cli1 -tsvg {}", src_path.display()),
        "argv: {argv}"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_stub_dot_receives_env_and_cli_args() {
    let _g = diag_lock();
    clear_tool_env();
    let root = tmp_root("diag_stub_dot");
    let argv_file = root.join("argv.txt");
    let stub = root.join("stub_dot");
    write_stub(
        &stub,
        &format!("#!/bin/sh\necho \"$@\" > {}\nexit 0\n", argv_file.display()),
    );
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    set_env(&[
        ("RFS_DOT", Some(stub.to_str().unwrap())),
        ("RFS_DOT_ARGS", Some("-Gdpi=150")),
    ]);
    let out = std::process::Command::new(rfs_bin())
        .args([
            "diagram",
            "--render",
            "svg",
            "--out",
            svg.to_str().unwrap(),
            "--dot-args",
            "-x",
        ])
        .arg(&f)
        .output()
        .expect("run rfs");
    set_env(&[("RFS_DOT", None), ("RFS_DOT_ARGS", None)]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let argv = std::fs::read_to_string(&argv_file)
        .expect("stub ran")
        .trim()
        .to_string();
    let src_path = root.join("out.dot");
    assert_eq!(
        argv,
        format!(
            "-Gdpi=150 -x -Tsvg {} -o {}",
            src_path.display(),
            svg.display()
        ),
        "argv: {argv}"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_flag_path_beats_env_for_mermaid() {
    let _g = diag_lock();
    clear_tool_env();
    let root = tmp_root("diag_stub_mmd");
    let marker_a = root.join("marker_a.txt");
    let marker_b = root.join("marker_b.txt");
    let stub_a = root.join("stub_a");
    let stub_b = root.join("stub_b");
    write_stub(
        &stub_a,
        &format!("#!/bin/sh\ntouch {}\nexit 0\n", marker_a.display()),
    );
    write_stub(
        &stub_b,
        &format!("#!/bin/sh\ntouch {}\nexit 0\n", marker_b.display()),
    );
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    set_env(&[("RFS_MERMAID", Some(stub_a.to_str().unwrap()))]);
    let out = std::process::Command::new(rfs_bin())
        .args([
            "diagram",
            "--format",
            "mmd",
            "--render",
            "svg",
            "--out",
            svg.to_str().unwrap(),
            "--mermaid",
            stub_b.to_str().unwrap(),
        ])
        .arg(&f)
        .output()
        .expect("run rfs");
    set_env(&[("RFS_MERMAID", None)]);
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    assert!(
        !marker_a.exists(),
        "env-var stub must NOT be used when a flag is given"
    );
    assert!(marker_b.exists(), "flag path wins (FLAG > ENV precedence)");
    cleanup(&root);
}

#[test]
fn e2e_diagram_missing_dot_tool_exits_one() {
    let _g = diag_lock();
    clear_tool_env();
    let root = tmp_root("diag_missing_dot");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    set_env(&[("RFS_DOT", Some("/nonexistent/rfs_test_dot"))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--render", "svg", "--out"])
        .arg(&svg)
        .arg(&f)
        .output()
        .expect("run rfs");
    set_env(&[("RFS_DOT", None)]);
    assert!(
        !out.status.success(),
        "strict semantics: missing tool is an error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("failed to run dot"), "stderr: {stderr}");
    cleanup(&root);
}

#[test]
fn e2e_diagram_failing_plantuml_propagates_stderr() {
    let _g = diag_lock();
    clear_tool_env();
    let root = tmp_root("diag_fail_puml");
    let stub = root.join("failing_plantuml");
    write_stub(
        &stub,
        "#!/bin/sh\necho \"Error line 3 in file: bad.puml\" >&2\nexit 200\n",
    );
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    set_env(&[("RFS_PLANTUML", Some(stub.to_str().unwrap()))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--format", "puml", "--render", "svg", "--out"])
        .arg(&svg)
        .arg(&f)
        .output()
        .expect("run rfs");
    set_env(&[("RFS_PLANTUML", None)]);
    assert!(
        !out.status.success(),
        "strict semantics: renderer failure is an error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("plantuml exited"), "stderr: {stderr}");
    assert!(
        stderr.contains("Error line 3 in file: bad.puml"),
        "captured tool stderr must surface: {stderr}"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_failing_mmdc_exits_one() {
    let _g = diag_lock();
    clear_tool_env();
    let root = tmp_root("diag_fail_mmd");
    let stub = root.join("failing_mmdc");
    write_stub(
        &stub,
        "#!/bin/sh\necho \"Error: Parse error on line 1:\" >&2\nexit 1\n",
    );
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let svg = root.join("out.svg");
    set_env(&[("RFS_MERMAID", Some(stub.to_str().unwrap()))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--format", "mmd", "--render", "svg", "--out"])
        .arg(&svg)
        .arg(&f)
        .output()
        .expect("run rfs");
    set_env(&[("RFS_MERMAID", None)]);
    assert!(
        !out.status.success(),
        "strict semantics: renderer failure is an error"
    );
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("mmdc exited"), "stderr: {stderr}");
    cleanup(&root);
}

fn print_script_of(root: &std::path::Path, args: &[&str]) -> String {
    let out = std::process::Command::new(rfs_bin())
        .args(args)
        .current_dir(root)
        .output()
        .expect("run rfs");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8_lossy(&out.stdout).into_owned()
}

#[test]
fn e2e_diagram_print_script_puml_render_pinned() {
    let root = tmp_root("diag_script_puml");
    let _g = diag_lock();
    clear_tool_env();
    write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    let script = print_script_of(
        &root,
        &[
            "diagram",
            "--format",
            "puml",
            "--render",
            "svg",
            "--out",
            "build/diagram.svg",
            "--print-script",
            "spec.rfs",
        ],
    );
    assert_eq!(script, "#!/bin/sh
# rfs 0.1.0 - generated by `rfs diagram --format puml --render svg --out build/diagram.svg spec.rfs` on spec.rfs
# tools: plantuml=plantuml
set -e
# step 1: write generated diagram source
cat > build/diagram.puml <<'__RFS_DIAGRAM_SOURCE__'
@startuml
title pin_fsm
[*] --> Idle
Idle --> Active : Tick
Active --> Active : Cmd(Go(lvl)) \\n (lvl > 2)
note right on link : emits Ack(Ack(counter))
Active --> Idle : Tick
Idle -[dashed]-> Error : on_error
note right on link : emits Ack(Ack(99))
Active -[dashed]-> Error : on_error
note right on link : emits Ack(Ack(99))
Error -[dashed]-> Error : on_error
note right on link : emits Ack(Ack(99))
@enduml
__RFS_DIAGRAM_SOURCE__
# step 2: render svg
plantuml -tsvg build/diagram.puml
");
    cleanup(&root);
}

#[test]
fn e2e_diagram_print_script_mmd_render_pinned() {
    let root = tmp_root("diag_script_mmd");
    let _g = diag_lock();
    clear_tool_env();
    write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    let script = print_script_of(
        &root,
        &[
            "diagram",
            "--format",
            "mmd",
            "--render",
            "svg",
            "--out",
            "build/diagram.svg",
            "--print-script",
            "spec.rfs",
        ],
    );
    // The mermaid script carries the puppeteer-config step between the heredoc
    // and the mmdc invocation (root/container Chrome requirement).
    assert!(
        script.contains(r#"printf '%s' '{"args":["--no-sandbox","--disable-dev-shm-usage"]}' > build/diagram.puppeteer.json"#),
        "puppeteer step: {script}"
    );
    assert!(
        script.contains(
            "mmdc -i build/diagram.mmd -o build/diagram.svg -p build/diagram.puppeteer.json -q"
        ),
        "mmdc invocation: {script}"
    );
    let tail = script
        .split("__RFS_DIAGRAM_SOURCE__\n")
        .nth(1)
        .unwrap_or("");
    assert_eq!(
        tail,
        "# step 2: write mmdc puppeteer config (Chrome needs --no-sandbox when rfs runs as root)\nprintf '%s' '{\"args\":[\"--no-sandbox\",\"--disable-dev-shm-usage\"]}' > build/diagram.puppeteer.json\n# step 3: render svg\nmmdc -i build/diagram.mmd -o build/diagram.svg -p build/diagram.puppeteer.json -q\n"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_print_script_no_tool_pinned() {
    let root = tmp_root("diag_script_notool");
    let _g = diag_lock();
    clear_tool_env();
    write_file(&root, "spec.rfs", PIN_DIAGRAM_RFS);
    let script = print_script_of(&root, &["diagram", "--print-script", "spec.rfs"]);
    assert_eq!(
        script,
        "#!/bin/sh
# rfs 0.1.0 - generated by `rfs diagram --format dot spec.rfs` on spec.rfs
set -e
# step 1: no external tools needed (text format)
echo 'rfs diagram --format dot <spec> emits text; nothing to run'
"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_list_pinned() {
    let _g = diag_lock();
    clear_tool_env();
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--list"])
        .output()
        .expect("run rfs");
    assert!(out.status.success());
    assert_eq!(String::from_utf8_lossy(&out.stdout), "dot    Graphviz DOT state diagram (pure Rust generation; SVG via the dot binary)
puml   PlantUML state diagram (edge label 'event \\n guard', emitted data via note on link; SVG via the plantuml binary)
mmd    Mermaid stateDiagram-v2 (all transition info in multi-line edge labels; SVG via the mmdc binary)
table  Transition table + variable/event summary (plain text)
md     Transition table + variable/event summary (Markdown)\n");
}

#[test]
fn e2e_diagram_rejects_unknown_format() {
    let root = tmp_root("diag_badfmt");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--format", "bogus"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("unknown diagram format 'bogus'"),
        "stderr: {stderr}"
    );
    assert!(
        stderr.contains("dot, puml, mmd, table, md"),
        "registered list shown: {stderr}"
    );
    cleanup(&root);
}

#[test]
fn e2e_diagram_render_requires_out_svg() {
    let root = tmp_root("diag_render_out");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    // No --out at all.
    let out = std::process::Command::new(rfs_bin())
        .args(["diagram", "--format", "puml", "--render", "svg"])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    assert!(String::from_utf8_lossy(&out.stderr).contains("requires --out FILE ending in .svg"));
    // Wrong extension.
    let out = std::process::Command::new(rfs_bin())
        .args([
            "diagram",
            "--format",
            "puml",
            "--render",
            "svg",
            "--out",
            root.join("x.png").to_str().unwrap(),
        ])
        .arg(&f)
        .output()
        .expect("run rfs");
    assert!(!out.status.success());
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("ending in .svg"), "stderr: {stderr}");
    cleanup(&root);
}

#[test]
fn e2e_diagram_table_md_have_no_renderer() {
    let root = tmp_root("diag_norenderer");
    let f = write_file(&root, "a.rfs", PIN_DIAGRAM_RFS);
    for format in ["table", "md"] {
        let out = std::process::Command::new(rfs_bin())
            .args([
                "diagram",
                "--format",
                format,
                "--render",
                "svg",
                "--out",
                root.join("x.svg").to_str().unwrap(),
            ])
            .arg(&f)
            .output()
            .expect("run rfs");
        assert!(!out.status.success(), "{format} must reject --render");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("has no SVG renderer"), "stderr: {stderr}");
    }
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// Phase 5 task 7: `rfs test` command (generate + compile + run, both
// backends; external-tool parameterization; --print-script).
// ---------------------------------------------------------------------------

/// A spec whose generated invariant test FAILS deterministically: x starts
/// false while `prop p : always (x)` - the invariant re-check fails at sample
/// 0 in BOTH backends.
const FAILING_RFS: &str = "module fail_demo { enum State { A } initial A event Go var x: bool init false transition A on Go { next A; } prop p : always (x) ; }";

/// Env-mutating `rfs test` tests share process-global state - serialize them
/// (same pattern as the diagram env tests).
static TEST_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn test_lock() -> std::sync::MutexGuard<'static, ()> {
    TEST_ENV_LOCK.lock().unwrap_or_else(|e| e.into_inner())
}

/// Clear the rfs-test tool env vars so ambient settings can't leak in.
fn clear_test_env() {
    set_env(&[
        ("RFS_RUSTC", None),
        ("RFS_RUSTC_ARGS", None),
        ("RFS_GCC", None),
        ("RFS_GCC_ARGS", None),
    ]);
}

/// `rfs test <passing spec>` exits 0 in BOTH backends, streaming per-test
/// results on stdout.
#[test]
fn e2e_test_passing_spec_both_backends_exit_zero() {
    let _g = test_lock();
    clear_test_env();
    let root = tmp_root("t7_pass");
    let f = write_file(&root, "ok.rfs", VALID_RFS);
    for backend in ["rust", "c"] {
        let out = std::process::Command::new(rfs_bin())
            .args(["test", "--backend", backend])
            .arg(&f)
            .output()
            .expect("spawn rfs test");
        assert!(
            out.status.success(),
            "{backend}: exit {:?}; stderr: {}",
            out.status.code(),
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        if backend == "rust" {
            assert!(stdout.contains("test result: ok"), "{backend}: {stdout}");
            assert!(
                stdout.contains("initial_state_is_correct ... ok"),
                "{backend}: per-test lines missing: {stdout}"
            );
        } else {
            assert!(
                stdout.contains("PASS initial_state_is_correct"),
                "{backend}: per-test lines missing: {stdout}"
            );
            assert!(
                stdout.contains("all generated C tests passed"),
                "{backend}: {stdout}"
            );
        }
    }
    cleanup(&root);
}

/// A spec whose invariant test fails exits 1 in BOTH backends with the
/// failing test visible on stdout.
#[test]
fn e2e_test_failing_variant_exits_one() {
    let _g = test_lock();
    clear_test_env();
    let root = tmp_root("t7_fail");
    let f = write_file(&root, "bad.rfs", FAILING_RFS);
    for backend in ["rust", "c"] {
        let out = std::process::Command::new(rfs_bin())
            .args(["test", "--backend", backend])
            .arg(&f)
            .output()
            .expect("spawn rfs test");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{backend}: stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        if backend == "rust" {
            assert!(
                stdout.contains("test tests::invariant_p ... FAILED"),
                "{backend}: failing test not visible: {stdout}"
            );
        } else {
            assert!(
                stdout.contains("FAIL invariant_p"),
                "{backend}: failing test not visible: {stdout}"
            );
        }
    }
    cleanup(&root);
}

/// `--print-script` is byte-pinned (wrapper) and performs NO execution: the
/// resolved tool points at a non-existent path, so an executing run would
/// fail with exit 1 - script mode still exits 0. The heredoc body must be
/// exactly what `rfs codegen rust test` emits.
#[test]
fn e2e_test_print_script_pinned_and_no_execution() {
    let _g = test_lock();
    clear_test_env();
    let root = tmp_root("t7_script");
    write_file(&root, "ok.rfs", VALID_RFS);
    set_env(&[("RFS_RUSTC", Some("/nonexistent/rustc-stub"))]);

    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--print-script", "ok.rfs"])
        .current_dir(&root)
        .output()
        .expect("spawn rfs test");
    assert!(
        out.status.success(),
        "script mode must not execute; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let script = String::from_utf8_lossy(&out.stdout);

    // The heredoc body is exactly the codegen output (cross-checked).
    let gen = std::process::Command::new(rfs_bin())
        .args(["codegen", "rust", "test", "ok.rfs", "--out", "gen.rs"])
        .current_dir(&root)
        .output()
        .expect("spawn rfs codegen");
    assert!(
        gen.status.success(),
        "{}",
        String::from_utf8_lossy(&gen.stderr)
    );
    let expected_source = std::fs::read_to_string(root.join("gen.rs")).unwrap();

    let start = "cat > spec_test.rs <<'__RFS_TEST_SOURCE__'\n";
    let end = "\n__RFS_TEST_SOURCE__\n";
    let si = script.find(start).expect("heredoc start");
    let body_start = si + start.len();
    // `end` starts with the source's final newline: the body runs up to and
    // including that newline; the suffix starts at the closing delimiter.
    let ei_rel = script[body_start..].find(end).expect("heredoc end");
    let body = &script[body_start..body_start + ei_rel + 1];
    assert_eq!(
        body, expected_source,
        "heredoc body must equal the codegen output"
    );

    // The wrapper is byte-pinned around the body.
    let pinned_prefix = format!(
        "#!/bin/sh\n\
         # rfs {} - generated by `rfs test --backend rust ok.rfs` on ok.rfs\n\
         # tools: rustc=/nonexistent/rustc-stub\n\
         set -e\n\
         # step 1: write generated test artifact (from `rfs codegen <backend> test`)\n\
         {start}",
        env!("CARGO_PKG_VERSION")
    );
    let pinned_suffix = {
        "__RFS_TEST_SOURCE__\n\
         # step 2: compile the generated test artifact\n\
         /nonexistent/rustc-stub --edition 2021 --test spec_test.rs -o spec_test_bin\n\
         # step 3: run the generated tests\n\
         ./spec_test_bin\n\
         # step 4: clean up generated artifacts\n\
         rm -f spec_test.rs spec_test_bin\n"
    };
    assert_eq!(
        script,
        format!("{pinned_prefix}{body}{pinned_suffix}"),
        "script wrapper drifted"
    );
    cleanup(&root);
}

/// Tool precedence (flag > env var > PATH) proven end-to-end with stub tools
/// that record their argv; extra args reach the compiler (env args first,
/// then CLI args). The stub exits 0 without producing a binary, so the run
/// step fails - the command exits 1 and the recorded argv is the proof.
#[test]
fn e2e_test_tool_precedence_stub_tools() {
    let _g = test_lock();
    clear_test_env();
    let root = tmp_root("t7_tools");
    let f = write_file(&root, "ok.rfs", VALID_RFS);

    // Stub that records its argv and exits 0 (no binary produced).
    let case = |tag: &str,
                env_pairs: &[(&str, Option<&str>)],
                cli_args: &[&str],
                path_override: Option<String>|
     -> String {
        let argv_file = root.join(format!("{tag}_argv.txt"));
        let stub = root.join(format!("stub_{tag}"));
        write_stub(
            &stub,
            &format!("#!/bin/sh\necho \"$@\" > {}\nexit 0\n", argv_file.display()),
        );
        // Reset the tool env first: cases run sequentially and must not leak
        // overrides into each other.
        clear_test_env();
        for (k, v) in env_pairs {
            match v {
                Some(val) => std::env::set_var(k, val),
                None => std::env::remove_var(k),
            }
        }
        let mut cmd = std::process::Command::new(rfs_bin());
        cmd.args(["test"]);
        cmd.args(cli_args);
        if let Some(p) = path_override {
            cmd.env("PATH", p);
        }
        let out = cmd.arg(&f).output().expect("spawn rfs test");
        assert_eq!(
            out.status.code(),
            Some(1),
            "{tag}: run step must fail (stub produces no binary); stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        std::fs::read_to_string(&argv_file)
            .unwrap_or_else(|e| panic!("{tag}: stub was never invoked ({e})"))
            .trim_end()
            .to_string()
    };

    // (a) flag beats env var: RFS_RUSTC points elsewhere, --rustc wins.
    let argv = case(
        "flag",
        &[("RFS_RUSTC", Some("/nonexistent/other-rustc"))],
        &["--rustc", root.join("stub_flag").to_str().unwrap()],
        None,
    );
    // No extra args, no env-var tool: bare compile argv (temp paths volatile).
    assert!(argv.contains("/spec_test.rs -o /"), "argv: {argv}");
    assert!(argv.ends_with("spec_test_bin"), "argv: {argv}");

    // (b) env var beats PATH default.
    let argv = case(
        "env",
        &[("RFS_RUSTC", Some(root.join("stub_env").to_str().unwrap()))],
        &[],
        None,
    );
    assert!(argv.starts_with("--edition 2021 --test "), "argv: {argv}");

    // (c) PATH fallback: no env var, no flag - a `rustc` stub on PATH wins.
    write_stub(
        &root.join("rustc"),
        &format!(
            "#!/bin/sh\necho \"$@\" > {}\nexit 0\n",
            root.join("path_argv.txt").display()
        ),
    );
    let argv = case("path", &[], &[], Some(root.display().to_string()));
    assert!(argv.starts_with("--edition 2021 --test "), "argv: {argv}");

    // (d) extra args reach the compiler: env args first, then CLI args.
    let argv = case(
        "args",
        &[
            ("RFS_RUSTC", Some(root.join("stub_args").to_str().unwrap())),
            ("RFS_RUSTC_ARGS", Some("--env-arg")),
        ],
        &["--rustc-args", "--cli-arg"],
        None,
    );
    assert!(
        argv.starts_with("--env-arg --cli-arg --edition 2021 --test "),
        "argv: {argv}"
    );

    // (e) the C pair is wired the same way (RFS_GCC env var).
    let argv = case(
        "gcc",
        &[("RFS_GCC", Some(root.join("stub_gcc").to_str().unwrap()))],
        &["--backend", "c"],
        None,
    );
    assert!(argv.starts_with("-Wall "), "argv: {argv}");
    assert!(argv.contains("/spec_test.c -o /"), "argv: {argv}");
    assert!(argv.ends_with("spec_test_bin"), "argv: {argv}");

    clear_test_env();
    cleanup(&root);
}

/// Usage/parse/validation errors all exit 1 with pinned diagnostics
/// (consistent with the other commands).
#[test]
fn e2e_test_usage_and_parse_errors() {
    let _g = test_lock();
    clear_test_env();
    let root = tmp_root("t7_usage");

    // Missing spec.
    let out = std::process::Command::new(rfs_bin())
        .arg("test")
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("rfs test requires a spec file"), "{stderr}");

    // Unknown option.
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--bogus"])
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));
    assert!(String::from_utf8_lossy(&out.stderr).contains("unknown option: --bogus"));

    // Unknown backend.
    let f = write_file(&root, "ok.rfs", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--backend", "fortran"])
        .arg(&f)
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("unknown backend 'fortran'"), "{stderr}");

    // Unparseable spec.
    let bad = write_file(&root, "bad.rfs", UNPARSEABLE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["test"])
        .arg(&bad)
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));

    // Validation error.
    let bad2 = write_file(&root, "badstate.rfs", BAD_STATE_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["test"])
        .arg(&bad2)
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));

    // Wrong extension.
    let txt = write_file(&root, "ok.txt", VALID_RFS);
    let out = std::process::Command::new(rfs_bin())
        .args(["test"])
        .arg(&txt)
        .output()
        .expect("spawn rfs");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("must have extension .rfs or .json"),
        "{stderr}"
    );

    cleanup(&root);
}

// ---------------------------------------------------------------------------
// Phase 5 task 17: `rfs test --suite fuzz` (libFuzzer pipelines for both
// backends; suite selection; RFS_CLANG*/RFS_CARGO_FUZZ*/RFS_FUZZ_FLAGS;
// --print-script coverage).
// ---------------------------------------------------------------------------

/// Clear the task 17 tool env vars so ambient settings can't leak in.
fn clear_fuzz_env() {
    set_env(&[
        ("RFS_CLANG", None),
        ("RFS_CLANG_ARGS", None),
        ("RFS_CARGO_FUZZ", None),
        ("RFS_CARGO_FUZZ_ARGS", None),
        ("RFS_FUZZ_FLAGS", None),
        ("ASAN_OPTIONS", None),
    ]);
}

fn clear_all_test_env() {
    clear_test_env();
    clear_fuzz_env();
}

/// Suite selection is pinned: default = unit only (the fuzz tools are never
/// consulted - RFS_CLANG points at a nonexistent path and the run still
/// passes); `--suite fuzz` = fuzz only; `--suite unit,fuzz` = both; invalid
/// names and non-positive budgets are usage errors.
#[test]
fn e2e_fuzz_suite_selection_pinned() {
    let _g = test_lock();
    clear_all_test_env();
    let root = tmp_root("t17_sel");
    let f = write_file(&root, "ok.rfs", VALID_RFS);

    // (a) Default suite: unit only. A broken RFS_CLANG must not matter.
    set_env(&[("RFS_CLANG", Some("/nonexistent/clang-stub"))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--backend", "c"])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert!(
        out.status.success(),
        "default suite must not touch clang; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout);
    assert!(stdout.contains("PASS initial_state_is_correct"), "{stdout}");

    // (b) --suite fuzz: no unit tests run at all (real clang this time).
    set_env(&[("RFS_CLANG", None)]);
    if tool_on_path("clang") {
        let out = std::process::Command::new(rfs_bin())
            .args([
                "test",
                "--backend",
                "c",
                "--suite",
                "fuzz",
                "--fuzz-max-total-time",
                "1",
            ])
            .arg(&f)
            .output()
            .expect("spawn rfs test");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(
            !stdout.contains("PASS "),
            "fuzz-only must skip unit tests: {stdout}"
        );
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("fuzz target ran within its budget"),
            "{stderr}"
        );
    } else {
        eprintln!("note: clang not available - fuzz-only selection check skipped");
    }

    // (c) --suite unit,fuzz: both suites run.
    if tool_on_path("clang") {
        let out = std::process::Command::new(rfs_bin())
            .args([
                "test",
                "--backend",
                "c",
                "--suite",
                "unit,fuzz",
                "--fuzz-max-total-time",
                "1",
            ])
            .arg(&f)
            .output()
            .expect("spawn rfs test");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
        let stdout = String::from_utf8_lossy(&out.stdout);
        assert!(stdout.contains("PASS initial_state_is_correct"), "{stdout}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(stderr.contains("all generated c tests passed"), "{stderr}");
        assert!(
            stderr.contains("fuzz target ran within its budget"),
            "{stderr}"
        );
    }

    // (d) Invalid suite name -> usage error, exit 1.
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--backend", "c", "--suite", "bogus"])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("--suite expects unit|fuzz (got 'bogus')"),
        "{stderr}"
    );

    // (e) Non-positive budget -> usage error, exit 1.
    for bad in ["0", "-3"] {
        let out = std::process::Command::new(rfs_bin())
            .args(["test", "--fuzz-max-total-time", bad])
            .arg(&f)
            .output()
            .expect("spawn rfs test");
        assert_eq!(out.status.code(), Some(1), "budget {bad}");
        let stderr = String::from_utf8_lossy(&out.stderr);
        assert!(
            stderr.contains("--fuzz-max-total-time expects a positive integer"),
            "{stderr}"
        );
    }

    clear_all_test_env();
    cleanup(&root);
}

/// Missing tools for a SELECTED fuzz suite -> exit 1 + pinned diagnostics;
/// the default suite is unaffected (covered by the selection test).
#[test]
fn e2e_fuzz_missing_tools_pinned() {
    let _g = test_lock();
    clear_all_test_env();
    let root = tmp_root("t17_missing");
    let f = write_file(&root, "ok.rfs", VALID_RFS);

    // (a) Missing clang (RFS_CLANG at a nonexistent path).
    set_env(&[("RFS_CLANG", Some("/nonexistent/clang-stub"))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--backend", "c", "--suite", "fuzz"])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("clang not found at '/nonexistent/clang-stub' (required for --suite fuzz)"),
        "{stderr}"
    );

    // (b) Missing cargo-fuzz: a stub `cargo` without the fuzz subcommand.
    let stub = root.join("fake_cargo");
    write_stub(
        &stub,
        "#!/bin/sh\necho \"error: no such command: `fuzz`\" >&2\nexit 101\n",
    );
    set_env(&[("RFS_CARGO_FUZZ", Some(stub.to_str().unwrap()))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--backend", "rust", "--suite", "fuzz"])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(
        stderr.contains("cargo-fuzz not found (required for --suite fuzz on the rust backend)"),
        "{stderr}"
    );

    clear_all_test_env();
    cleanup(&root);
}

/// Real-tool e2e (availability-gated): both backends build + run a bounded
/// fuzz target on a passing spec -> exit 0. Environment gaps (no clang, no
/// cargo-fuzz, no nightly) skip with a note; anything else fails.
#[test]
fn e2e_fuzz_real_tools_both_backends_exit_zero() {
    let _g = test_lock();
    clear_all_test_env();
    let root = tmp_root("t17_real");
    let f = write_file(&root, "ok.rfs", VALID_RFS);

    // C: needs clang on PATH.
    if tool_on_path("clang") {
        let out = std::process::Command::new(rfs_bin())
            .args([
                "test",
                "--backend",
                "c",
                "--suite",
                "fuzz",
                "--fuzz-max-total-time",
                "2",
            ])
            .arg(&f)
            .output()
            .expect("spawn rfs test");
        assert!(
            out.status.success(),
            "stderr: {}",
            String::from_utf8_lossy(&out.stderr)
        );
    } else {
        eprintln!("note: clang not available - C real-tool fuzz check skipped");
    }

    // Rust: needs cargo-fuzz + a nightly toolchain (the scaffold pins one).
    let out = std::process::Command::new(rfs_bin())
        .args([
            "test",
            "--backend",
            "rust",
            "--suite",
            "fuzz",
            "--fuzz-max-total-time",
            "2",
        ])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    if !out.status.success() {
        let stderr = String::from_utf8_lossy(&out.stderr);
        if stderr.contains("cargo-fuzz not found") || stderr.contains("nightly") {
            eprintln!("note: cargo-fuzz/nightly unavailable - Rust real-tool fuzz check skipped");
            clear_all_test_env();
            cleanup(&root);
            return;
        }
        panic!("rust fuzz run failed; stderr: {stderr}");
    }

    clear_all_test_env();
    cleanup(&root);
}

/// A fuzzer non-zero exit -> command exit 1 (stub-pinned, C side): a fake
/// clang that "builds" a binary which exits 31. The rust side shares the
/// same run-failure code path (cargo-fuzz run non-success -> Err).
#[test]
fn e2e_fuzz_crash_stub_exits_one() {
    let _g = test_lock();
    clear_all_test_env();
    let root = tmp_root("t17_crash");
    let f = write_file(&root, "ok.rfs", VALID_RFS);

    // POSIX sh stub: find the -o target and make it a failing binary.
    let stub = root.join("fake_clang");
    write_stub(
        &stub,
        "#!/bin/sh\nprev=\nfor a in \"$@\"; do\n  if [ \"$prev\" = \"-o\" ]; then printf \"#!/bin/sh\\nexit 31\\n\" > \"$a\"; chmod +x \"$a\"; fi\n  prev=$a\ndone\nexit 0\n",
    );
    set_env(&[("RFS_CLANG", Some(stub.to_str().unwrap()))]);
    let out = std::process::Command::new(rfs_bin())
        .args(["test", "--backend", "c", "--suite", "fuzz"])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert_eq!(out.status.code(), Some(1));
    let stderr = String::from_utf8_lossy(&out.stderr);
    assert!(stderr.contains("fuzz target exited non-zero"), "{stderr}");

    clear_all_test_env();
    cleanup(&root);
}

/// Stub argv proofs for the full fuzz pipelines: exact command lines, extra
/// args in env-then-CLI order right after the tool (the ext_tool contract).
#[test]
fn e2e_fuzz_stub_argv_proofs() {
    let _g = test_lock();
    clear_all_test_env();
    let root = tmp_root("t17_argv");
    let f = write_file(&root, "ok.rfs", VALID_RFS);

    // (a) C pipeline: the stub records argv and exits 0 without producing a
    // binary -> the run step fails (exit 1); the recorded argv is the proof.
    let c_argv = root.join("c_argv.txt");
    let c_stub = root.join("stub_clang");
    write_stub(
        &c_stub,
        &format!("#!/bin/sh\necho \"$@\" > {}\nexit 0\n", c_argv.display()),
    );
    set_env(&[
        ("RFS_CLANG", Some(c_stub.to_str().unwrap())),
        ("RFS_CLANG_ARGS", Some("--env-arg")),
    ]);
    let out = std::process::Command::new(rfs_bin())
        .args([
            "test",
            "--backend",
            "c",
            "--suite",
            "fuzz",
            "--clang-args",
            "--cli-arg",
        ])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert_eq!(out.status.code(), Some(1), "run step must fail (no binary)");
    let argv = std::fs::read_to_string(&c_argv)
        .unwrap_or_else(|e| panic!("clang stub never invoked: {e}"))
        .trim_end()
        .to_string();
    // env args, CLI args, then the pinned recipe (temp paths are volatile).
    assert!(
        argv.starts_with("--env-arg --cli-arg -O1 -fsanitize=fuzzer,address -o "),
        "argv: {argv}"
    );
    assert!(argv.contains("/fuzz_bin /"), "argv: {argv}");
    assert!(argv.ends_with("fuzz_target.c"), "argv: {argv}");

    // (b) Rust pipeline: the stub records BOTH invocations (build, then run)
    // and exits 0 -> the command succeeds; two argv lines are the proof.
    let r_argv = root.join("r_argv.txt");
    let r_stub = root.join("stub_cargo");
    write_stub(
        &r_stub,
        &format!("#!/bin/sh\necho \"$@\" >> {}\nexit 0\n", r_argv.display()),
    );
    set_env(&[
        ("RFS_CARGO_FUZZ", Some(r_stub.to_str().unwrap())),
        ("RFS_CARGO_FUZZ_ARGS", Some("--env-arg")),
        ("RFS_FUZZ_FLAGS", Some("-env-fuzz-flag")),
    ]);
    let out = std::process::Command::new(rfs_bin())
        .args([
            "test",
            "--backend",
            "rust",
            "--suite",
            "fuzz",
            "--cargo-fuzz-args",
            "--cli-arg",
            "--fuzz-max-total-time",
            "7",
            "--fuzz-flags",
            "-cli-fuzz-flag",
        ])
        .arg(&f)
        .output()
        .expect("spawn rfs test");
    assert!(
        out.status.success(),
        "stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let r_argv_text = std::fs::read_to_string(&r_argv)
        .unwrap_or_else(|e| panic!("cargo-fuzz stub never invoked: {e}"));
    let lines: Vec<&str> = r_argv_text.lines().collect();
    assert_eq!(
        lines.len(),
        2,
        "expected build + run invocations: {lines:?}"
    );
    // Fixed `fuzz` subcommand right after the tool, then env args, CLI args.
    assert_eq!(lines[0], "fuzz --env-arg --cli-arg build");
    // `-max_total_time` first, then RFS_FUZZ_FLAGS (env), then --fuzz-flags.
    assert_eq!(lines[1], "fuzz --env-arg --cli-arg run fuzz_target -- -max_total_time=7 -env-fuzz-flag -cli-fuzz-flag");

    clear_all_test_env();
    cleanup(&root);
}

/// `--print-script` with the fuzz suite: byte-pinned wrapper around the two
/// heredoc bodies (cross-checked against actual codegen output), no execution
/// (RFS_CLANG at a nonexistent path would fail an executing run).
#[test]
fn e2e_fuzz_print_script_pinned_and_no_execution() {
    let _g = test_lock();
    clear_all_test_env();
    let root = tmp_root("t17_script");
    write_file(&root, "ok.rfs", VALID_RFS);
    set_env(&[("RFS_CLANG", Some("/nonexistent/clang-stub"))]);

    let out = std::process::Command::new(rfs_bin())
        .args([
            "test",
            "--backend",
            "c",
            "--suite",
            "fuzz",
            "--print-script",
            "ok.rfs",
        ])
        .current_dir(&root)
        .output()
        .expect("spawn rfs test");
    assert!(
        out.status.success(),
        "script mode must not execute; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let script = String::from_utf8_lossy(&out.stdout).to_string();

    // Cross-check the heredoc bodies against actual codegen output.
    let gen = std::process::Command::new(rfs_bin())
        .args(["codegen", "c", "test", "ok.rfs", "--out", "gen_test.c"])
        .current_dir(&root)
        .output()
        .expect("spawn rfs codegen");
    assert!(
        gen.status.success(),
        "{}",
        String::from_utf8_lossy(&gen.stderr)
    );
    let expected_skeleton = std::fs::read_to_string(root.join("fuzz_target.c")).unwrap();
    let gen = std::process::Command::new(rfs_bin())
        .args([
            "codegen",
            "c",
            "production",
            "ok.rfs",
            "--out",
            "gen_prod.c",
        ])
        .current_dir(&root)
        .output()
        .expect("spawn rfs codegen");
    assert!(
        gen.status.success(),
        "{}",
        String::from_utf8_lossy(&gen.stderr)
    );
    let expected_fsm = std::fs::read_to_string(root.join("gen_prod.c")).unwrap();

    // Extract the two heredoc bodies.
    let extract = |script: &str, start_marker: &str, end_marker: &str| -> String {
        let si = script.find(start_marker).expect("heredoc start");
        let body_start = si + start_marker.len();
        let ei_rel = script[body_start..].find(end_marker).expect("heredoc end");
        // The body runs up to and including the source's final newline; the
        // suffix starts at the closing delimiter.
        script[body_start..body_start + ei_rel + 1].to_string()
    };
    let skel_body = extract(
        &script,
        "cat > fuzz_target.c <<'__RFS_FUZZ_TARGET_C__'\n",
        "\n__RFS_FUZZ_TARGET_C__\n",
    );
    let fsm_body = extract(
        &script,
        "cat > rfs_generated_fsm.c <<'__RFS_GENERATED_FSM_C__'\n",
        "\n__RFS_GENERATED_FSM_C__\n",
    );
    assert_eq!(skel_body, expected_skeleton, "skeleton heredoc drifted");
    assert_eq!(fsm_body, expected_fsm, "production heredoc drifted");

    // The wrapper is byte-pinned around the bodies.
    let pinned_prefix = format!(
        "#!/bin/sh\n\
         # rfs {} - generated by `rfs test --backend c --suite fuzz ok.rfs` on ok.rfs\n\
         # tools: clang=/nonexistent/clang-stub\n\
         # note: requires clang + libFuzzer runtime (RFS_CLANG / RFS_CLANG_ARGS)\n\
         set -e\n\
         # step 1: write the emitted fuzz skeleton (from `rfs codegen c test` extra file)\n\
         cat > fuzz_target.c <<'__RFS_FUZZ_TARGET_C__'\n",
        env!("CARGO_PKG_VERSION")
    );
    let pinned_middle = "\
__RFS_FUZZ_TARGET_C__\n\
# step 2: write the generated production source (the skeleton's hardcoded #include target)\n\
cat > rfs_generated_fsm.c <<'__RFS_GENERATED_FSM_C__'\n";
    let pinned_suffix = "\
__RFS_GENERATED_FSM_C__\n\
# step 3: build the C fuzz target\n\
/nonexistent/clang-stub -O1 -fsanitize=fuzzer,address -o fuzz_bin fuzz_target.c\n\
# step 4: run the C fuzz target bounded (leak detection off - no free API in the generated FSM)\n\
ASAN_OPTIONS=\"detect_leaks=0${ASAN_OPTIONS:+:$ASAN_OPTIONS}\" ./fuzz_bin -max_total_time=30\n\
# step 5: clean up generated artifacts\n\
rm -f fuzz_target.c rfs_generated_fsm.c fuzz_bin\n";
    assert_eq!(
        script,
        format!("{pinned_prefix}{skel_body}{pinned_middle}{fsm_body}{pinned_suffix}"),
        "script wrapper drifted"
    );

    clear_all_test_env();
    cleanup(&root);
}

// ---------------------------------------------------------------------------
// Phase 5 task 13: `rfs validate --json` (structured diagnostics for editor
// integrations) + the IDE grammar sanity test.
// ---------------------------------------------------------------------------

/// Parse-error document: shape pinned, and line/col cross-checked against the
/// parser's own diagnostic (ParseError::line_col).
#[test]
fn e2e_validate_json_parse_error_shape_and_position() {
    let _g = test_lock();
    clear_all_test_env();
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/invalid/invalid_bad_version_header.rfs");

    // Expected position from the parser's own diagnostic (cross-check).
    let src = std::fs::read_to_string(&file).expect("read example");
    let err = rfs::parse_source(&src).expect_err("example must not parse");
    let (line, col) = err.line_col();

    let out = std::process::Command::new(rfs_bin())
        .args(["validate", "--json"])
        .arg(&file)
        .output()
        .expect("spawn rfs validate --json");
    assert_eq!(
        out.status.code(),
        Some(1),
        "invalid file must exit 1; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let doc = rfs::json_rw::parse(&stdout).expect("JSON document must parse");

    assert_eq!(
        doc.get("file").and_then(|v| v.as_str()),
        Some(file.display().to_string().as_str())
    );
    assert_eq!(doc.get("ok"), Some(&rfs::json_rw::Json::Bool(false)));
    let errors = match doc.get("errors") {
        Some(rfs::json_rw::Json::Arr(a)) => a,
        _ => panic!("errors must be an array: {stdout}"),
    };
    assert_eq!(errors.len(), 1);
    let e0 = &errors[0];
    assert_eq!(e0.get("kind").and_then(|v| v.as_str()), Some("parse"));
    assert_eq!(e0.get("line"), Some(&rfs::json_rw::Json::Int(line as i64)));
    assert_eq!(e0.get("col"), Some(&rfs::json_rw::Json::Int(col as i64)));
    assert_eq!(
        e0.get("message").and_then(|v| v.as_str()),
        Some(err.message.as_str())
    );
}

/// Semantic-error document: shape pinned (kind "semantic", line/col 0 - the
/// validator reports messages without spans), message matches the text
/// diagnostic.
#[test]
fn e2e_validate_json_semantic_error_shape_pinned() {
    let _g = test_lock();
    clear_all_test_env();
    let file = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("examples/invalid/invalid_always_zero_bound.rfs");

    let out = std::process::Command::new(rfs_bin())
        .args(["validate", "--json"])
        .arg(&file)
        .output()
        .expect("spawn rfs validate --json");
    assert_eq!(out.status.code(), Some(1));
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let doc = rfs::json_rw::parse(&stdout).expect("JSON document must parse");

    assert_eq!(doc.get("ok"), Some(&rfs::json_rw::Json::Bool(false)));
    let errors = match doc.get("errors") {
        Some(rfs::json_rw::Json::Arr(a)) => a,
        _ => panic!("errors must be an array: {stdout}"),
    };
    assert_eq!(errors.len(), 1);
    let e0 = &errors[0];
    assert_eq!(e0.get("kind").and_then(|v| v.as_str()), Some("semantic"));
    assert_eq!(e0.get("line"), Some(&rfs::json_rw::Json::Int(0)));
    assert_eq!(e0.get("col"), Some(&rfs::json_rw::Json::Int(0)));
    assert_eq!(
        e0.get("message").and_then(|v| v.as_str()),
        Some("module 'always_zero_bound' property 'bad': always[N] requires N >= 1")
    );
}

/// A valid file: ok=true, empty errors, exit 0.
#[test]
fn e2e_validate_json_ok_true_on_valid_file() {
    let _g = test_lock();
    clear_all_test_env();
    let file = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/valid/43_sliding_window.rfs");

    let out = std::process::Command::new(rfs_bin())
        .args(["validate", "--json"])
        .arg(&file)
        .output()
        .expect("spawn rfs validate --json");
    assert!(
        out.status.success(),
        "valid file must exit 0; stderr: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let doc = rfs::json_rw::parse(&stdout).expect("JSON document must parse");
    assert_eq!(doc.get("ok"), Some(&rfs::json_rw::Json::Bool(true)));
    match doc.get("errors") {
        Some(rfs::json_rw::Json::Arr(a)) => assert!(a.is_empty(), "errors must be empty: {stdout}"),
        _ => panic!("errors must be an array: {stdout}"),
    }
}

/// Directory mode: a JSON ARRAY with one document per failing file; exit 1.
#[test]
fn e2e_validate_json_directory_mode_array() {
    let _g = test_lock();
    clear_all_test_env();
    let dir = Path::new(env!("CARGO_MANIFEST_DIR")).join("examples/invalid");

    let out = std::process::Command::new(rfs_bin())
        .args(["validate", "--json", "--input", "rfs"])
        .arg(&dir)
        .output()
        .expect("spawn rfs validate --json (directory)");
    assert_eq!(out.status.code(), Some(1), "failing files must exit 1");
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    let doc = rfs::json_rw::parse(&stdout).expect("JSON array must parse");

    let arr = match &doc {
        rfs::json_rw::Json::Arr(a) => a,
        _ => panic!("directory mode must emit an array: {stdout}"),
    };
    // Every file in examples/invalid fails validation.
    let n_files = std::fs::read_dir(&dir)
        .expect("read dir")
        .filter_map(|e| e.ok())
        .filter(|e| e.path().extension().is_some_and(|x| x == "rfs"))
        .count();
    assert_eq!(arr.len(), n_files, "one document per failing file");

    for d in arr {
        assert_eq!(d.get("ok"), Some(&rfs::json_rw::Json::Bool(false)));
        let errors = match d.get("errors") {
            Some(rfs::json_rw::Json::Arr(a)) if !a.is_empty() => a,
            _ => panic!("failing file document must have non-empty errors: {stdout}"),
        };
        for e in errors {
            let kind = e.get("kind").and_then(|v| v.as_str()).expect("kind");
            assert!(kind == "parse" || kind == "semantic", "kind: {kind}");
            assert!(e.get("line").is_some() && e.get("col").is_some());
        }
    }
}

/// IDE grammar sanity (Phase 5 task 13): the grammar file is valid JSON and
/// covers EVERY parser keyword (drift guard - the list is shared with the
/// parser via `rfs::parser::KEYWORDS`); the snippet set is present + valid.
#[test]
fn ide_grammar_valid_json_and_covers_all_keywords() {
    let root = Path::new(env!("CARGO_MANIFEST_DIR"));

    let raw = std::fs::read_to_string(root.join("ide/textmate/rfs.json"))
        .expect("ide/textmate/rfs.json must exist");
    let _doc: rfs::json_rw::Json = rfs::json_rw::parse(&raw).expect("grammar must be valid JSON");

    for kw in rfs::parser::KEYWORDS {
        assert!(raw.contains(kw), "grammar missing keyword '{kw}'");
    }

    let snip = std::fs::read_to_string(root.join("ide/snippets.rfs"))
        .expect("ide/snippets.rfs must exist");
    let _sdoc: rfs::json_rw::Json =
        rfs::json_rw::parse(&snip).expect("snippets must be valid JSON");
}
