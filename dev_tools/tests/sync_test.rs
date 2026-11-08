//! Phase 12 task 5.1: sync-check / sync-fix end-to-end on fixture repos.
//! The fixtures under tests/fixtures/sync/ are committed synthetic trees -
//! the tests never touch the real repository tree (the real-tree
//! demonstration is recorded in the task status file instead).

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture(name: &str) -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests/fixtures/sync")
        .join(name)
}

/// Copy a fixture into a private temp dir so mutating tests cannot damage the
/// committed tree.
fn copy_fixture_to_temp(name: &str, test: &str) -> PathBuf {
    let dst = std::env::temp_dir().join(format!("rfs-dev-sync-{test}-{}", std::process::id()));
    let _ = std::fs::remove_dir_all(&dst);
    fn copy(src: &Path, dst: &Path) {
        for entry in std::fs::read_dir(src).unwrap().flatten() {
            let p = entry.path();
            let q = dst.join(p.file_name().unwrap());
            if p.is_dir() {
                std::fs::create_dir_all(&q).unwrap();
                copy(&p, &q);
            } else {
                std::fs::copy(&p, &q).unwrap();
            }
        }
    }
    std::fs::create_dir_all(&dst).unwrap();
    copy(&fixture(name), &dst);
    dst
}

fn run_sync(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rfs-dev"))
        .args(args)
        .output()
        .expect("rfs-dev binary must exist and run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
    )
}

#[test]
fn clean_fixture_reports_no_problems_and_exits_0() {
    let (code, out) = run_sync(&["sync-check", "--root", fixture("clean").to_str().unwrap()]);
    assert_eq!(code, 0, "stdout was: {out}");
    assert!(out.trim().is_empty(), "stdout was: {out}");
}

#[test]
fn stale_fixture_reports_all_five_planted_problems_and_exits_1() {
    let (code, out) = run_sync(&["sync-check", "--root", fixture("stale").to_str().unwrap()]);
    assert_eq!(code, 1, "stdout was: {out}");
    for expected in [
        "src/gone.rs: manifest lists the file but it does not exist",
        "src/alpha.rs: no `//! Docs:` pointer line",
        "docs/dev/alpha.md: fragment `dev:alpha-overview` is defined but owned by no manifest-listed file (orphan)",
        "docs/dev/alpha.md: doc->code reference `src/gone2.rs` does not resolve: no such file",
        "docs/dev/alpha.md: doc->code reference `src/alpha.rs::missing_fn` does not resolve: item `missing_fn` is not declared in src/alpha.rs",
    ] {
        assert!(out.lines().any(|l| l == expected), "missing `{expected}` in:\n{out}");
    }
}

#[test]
fn sync_fix_dry_run_prints_the_plan_but_changes_nothing() {
    let root = copy_fixture_to_temp("moved", "dryrun");
    let doc = root.join("docs/dev/newmod.md");
    let before = std::fs::read_to_string(&doc).unwrap();
    let (code, out) = run_sync(&[
        "sync-fix",
        "--root",
        root.to_str().unwrap(),
        "--dry-run",
        "--map",
        "src/oldmod.rs=src/newmod.rs",
    ]);
    assert_eq!(
        code, 1,
        "problems must remain after a dry-run; stdout was: {out}"
    );
    assert!(
        out.lines()
            .any(|l| l.contains("`src/oldmod.rs::run` -> `src/newmod.rs::run`")),
        "plan line missing in:\n{out}"
    );
    assert!(out.contains("dry-run"), "stdout was: {out}");
    assert_eq!(
        before,
        std::fs::read_to_string(&doc).unwrap(),
        "dry-run must not write"
    );
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sync_fix_with_mapping_rewrites_the_reference_and_the_tree_becomes_clean() {
    let root = copy_fixture_to_temp("moved", "apply");
    let (code, out) = run_sync(&[
        "sync-fix",
        "--root",
        root.to_str().unwrap(),
        "--map",
        "src/oldmod.rs=src/newmod.rs",
    ]);
    assert_eq!(
        code, 0,
        "tree must be clean after the fix; stdout was: {out}"
    );
    assert!(out.contains("applied"), "stdout was: {out}");
    let doc = std::fs::read_to_string(root.join("docs/dev/newmod.md")).unwrap();
    assert!(
        doc.contains("`src/newmod.rs::run`"),
        "reference not rewritten:\n{doc}"
    );
    assert!(!doc.contains("oldmod"), "stale path still present:\n{doc}");
    // and a follow-up sync-check on the fixed copy is clean:
    let (code2, out2) = run_sync(&["sync-check", "--root", root.to_str().unwrap()]);
    assert_eq!(code2, 0, "stdout was: {out2}");
    let _ = std::fs::remove_dir_all(&root);
}

#[test]
fn sync_fix_without_mappings_changes_nothing_and_reports_remaining_problems() {
    let root = copy_fixture_to_temp("moved", "nomap");
    let doc = root.join("docs/dev/newmod.md");
    let before = std::fs::read_to_string(&doc).unwrap();
    let (code, out) = run_sync(&["sync-fix", "--root", root.to_str().unwrap()]);
    assert_eq!(code, 1, "stdout was: {out}");
    assert!(out.contains("nothing to do"), "stdout was: {out}");
    assert!(out.contains("remaining:"), "stdout was: {out}");
    assert_eq!(
        before,
        std::fs::read_to_string(&doc).unwrap(),
        "no mappings => no writes"
    );
    let _ = std::fs::remove_dir_all(&root);
}
