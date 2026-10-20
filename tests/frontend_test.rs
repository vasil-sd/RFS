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
