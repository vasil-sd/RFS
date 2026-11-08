//! Phase 12 task 5.5: inventory end-to-end on committed fixture repos.
//! The fixtures under tests/fixtures/inventory{,_stale}/ are synthetic mini
//! repos (a small src/ tree + matching SRC/TEST inventories). The clean one
//! audits to EXACT; the stale one carries a phantom row and a missing row.
//! Row operations run on temp COPIES so the committed fixtures never mutate.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join(format!("tests/fixtures/{name}"))
}

/// Copy a fixture repo to a fresh temp dir (row ops mutate their target).
fn work_area(name: &str, tag: &str) -> PathBuf {
    let src = fixture(name);
    let dst = std::env::temp_dir().join(format!("rfs-inv-it-{}-{}", tag, std::process::id()));
    let _ = std::fs::remove_dir_all(&dst);
    copy_recursive(&src, &dst);
    dst
}

fn copy_recursive(src: &Path, dst: &Path) {
    std::fs::create_dir_all(dst).unwrap();
    for entry in std::fs::read_dir(src).unwrap().flatten() {
        let p = entry.path();
        let q = dst.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_recursive(&p, &q);
        } else {
            std::fs::copy(&p, &q).unwrap();
        }
    }
}

fn run(root: &Path, args: &[&str]) -> (i32, String, String) {
    let mut full = vec![];
    full.extend_from_slice(args);
    full.push("--root");
    full.push(root.to_str().unwrap());
    let out = Command::new(env!("CARGO_BIN_EXE_rfs-dev"))
        .args(&full)
        .output()
        .expect("rfs-dev binary must exist and run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn read(root: &Path, rel: &str) -> String {
    std::fs::read_to_string(root.join(rel)).unwrap()
}

#[test]
fn audit_is_exact_on_the_clean_fixture() {
    let root = work_area("inventory", "clean");
    let (code, out, err) = run(&root, &["inventory", "audit"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out.starts_with("EXACT:"), "{out}");
    assert!(out.contains("zero per-file diffs"), "{out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn audit_reports_the_planted_stale_and_missing_rows() {
    let root = work_area("inventory_stale", "stale");
    let (code, out, err) = run(&root, &["inventory", "audit"]);
    assert_eq!(code, 1, "stderr was: {err}");
    // the phantom row has no source item
    assert!(
        out.contains("src/b.rs [fn] stale (no source item): ghost"),
        "{out}"
    );
    // the real fn has no row
    assert!(
        out.contains("src/b.rs [fn] missing (not inventoried): two"),
        "{out}"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn add_update_remove_round_trip_keeps_the_table_valid() {
    let root = work_area("inventory", "roundtrip");
    // add a new fn to src/b.rs - plan-first, then applied.
    let (code, out, err) = run(
        &root,
        &[
            "inventory",
            "add",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::three",
            "--description",
            "two plus one",
        ],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains("plan: add Function row: src/b.rs::three"),
        "{out}"
    );
    assert!(out.contains("applied"), "{out}");
    let doc = read(&root, "docs/llm/SRC_STRUCTURE.md");
    assert!(doc.contains("| src/b.rs::three |"), "{doc}");
    // the new row sits after the other src/b.rs row (file grouping kept)
    let lines: Vec<&str> = doc.lines().collect();
    let idx_two = lines
        .iter()
        .position(|l| l.contains("src/b.rs::two "))
        .unwrap();
    let idx_three = lines
        .iter()
        .position(|l| l.contains("src/b.rs::three "))
        .unwrap();
    assert!(idx_two < idx_three, "{lines:?}");

    // update its description cell.
    let (code, _out, err) = run(
        &root,
        &[
            "inventory",
            "update",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::three",
            "--set",
            "description=three now documented",
        ],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(read(&root, "docs/llm/SRC_STRUCTURE.md").contains("three now documented"));

    // remove it again - the table returns to its original shape.
    let (code, _out, err) = run(
        &root,
        &[
            "inventory",
            "remove",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::three",
        ],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(!read(&root, "docs/llm/SRC_STRUCTURE.md").contains("src/b.rs::three"));
    // and the audit is EXACT again (the source never changed).
    let (code, out, err) = run(&root, &["inventory", "audit"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out.starts_with("EXACT:"), "{out}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn row_ops_dry_run_writes_nothing_and_errors_are_exit_1() {
    let root = work_area("inventory", "dryrun");
    let before = read(&root, "docs/llm/SRC_STRUCTURE.md");
    // dry-run add reports the plan but writes nothing.
    let (code, out, err) = run(
        &root,
        &[
            "inventory",
            "add",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::four",
            "--dry-run",
        ],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out.contains("dry-run: no files written"), "{out}");
    assert_eq!(
        before,
        read(&root, "docs/llm/SRC_STRUCTURE.md"),
        "dry-run must not write"
    );
    // updating / removing a missing row is an operational failure (exit 1).
    let (code, _out, err) = run(
        &root,
        &[
            "inventory",
            "update",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::ghost",
            "--set",
            "description=x",
        ],
    );
    assert_eq!(code, 1, "stderr was: {err}");
    assert!(err.contains("no row with identifier"), "{err}");
    let (code, _out, err) = run(
        &root,
        &[
            "inventory",
            "remove",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::ghost",
        ],
    );
    assert_eq!(code, 1, "stderr was: {err}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn row_ops_usage_errors_exit_2() {
    let root = work_area("inventory", "usage");
    // missing --doc
    let (code, _out, err) = run(
        &root,
        &["inventory", "add", "--kind", "fn", "--id", "src/b.rs::x"],
    );
    assert_eq!(code, 2, "stderr was: {err}");
    // bad --doc value
    let (code, _out, err) = run(
        &root,
        &[
            "inventory",
            "add",
            "--doc",
            "BOGUS",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::x",
        ],
    );
    assert_eq!(code, 2, "stderr was: {err}");
    // --set without =
    let (code, _out, err) = run(
        &root,
        &[
            "inventory",
            "update",
            "--doc",
            "SRC",
            "--kind",
            "fn",
            "--id",
            "src/b.rs::two",
            "--set",
            "description",
        ],
    );
    assert_eq!(code, 2, "stderr was: {err}");
    let _ = std::fs::remove_dir_all(&root);
}
