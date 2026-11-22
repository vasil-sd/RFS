//! Phase 12 task 1: pins the `rfs-dev` CLI contract END-TO-END - every stub's
//! exact stderr message and exit code, plus the usage-error cases. This is the
//! executable form of the "CLI surface fixed in task 1" decision: task 4's
//! proposal table must match it, and task 5's implementations keep it green
//! (message changes are plan corrections, not silent edits).

use std::process::Command;

fn run(args: &[&str]) -> (i32, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rfs-dev"))
        .args(args)
        .output()
        .expect("rfs-dev binary must exist and run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

/// Same as `run` but captures stdout - the help paths print there (exit 0).
fn run_stdout(args: &[&str]) -> (i32, String) {
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
fn no_args_prints_usage_and_exits_2() {
    let (code, err) = run(&[]);
    assert_eq!(code, 2);
    assert!(err.contains("usage: rfs-dev"), "stderr was: {err}");
}

#[test]
fn unknown_subcommand_prints_usage_and_exits_2() {
    let (code, err) = run(&["frobnicate"]);
    assert_eq!(code, 2);
    assert!(
        err.contains("unknown subcommand `frobnicate`"),
        "stderr was: {err}"
    );
    assert!(err.contains("usage: rfs-dev"), "stderr was: {err}");
}

#[test]
fn grouped_subcommands_without_a_valid_subverb_exit_2() {
    for args in [
        vec!["inventory"],
        vec!["inventory", "bogus"],
        vec!["calls"],
        vec!["calls", "bogus"],
    ] {
        let (code, err) = run(&args);
        assert_eq!(code, 2, "args: {args:?}");
        assert!(err.contains("usage: rfs-dev"), "stderr was: {err}");
    }
}

// Every subcommand is now implemented: sync-check / sync-fix (task 5.1),
// context (5.2), edit-rename / edit-move / edit-replace-item (5.3),
// doc-section (5.4), inventory (5.5) and calls extract / calls classify
// (5.6). Their contracts are pinned by tests/sync_test.rs,
// tests/context_test.rs, tests/edit_test.rs, tests/doc_section_test.rs,
// tests/inventory_test.rs and tests/calls_test.rs respectively - no stubs
// remain to assert here.

#[test]
fn top_level_help_exits_0_and_lists_every_subcommand() {
    for flag in ["--help", "-h"] {
        let (code, out) = run_stdout(&[flag]);
        assert_eq!(code, 0, "flag: {flag}");
        assert!(out.contains("usage: rfs-dev"), "stdout was: {out}");
        for sub in [
            "sync-check",
            "sync-fix",
            "context",
            "edit-rename",
            "edit-move",
            "edit-replace-item",
            "doc-section",
            "inventory",
            "calls",
        ] {
            assert!(out.contains(sub), "flag: {flag}, missing {sub}");
        }
    }
}

#[test]
fn per_subcommand_help_exits_0_with_its_own_usage_line() {
    let cases: [(&[&str], &str); 13] = [
        (
            &["sync-check", "--help"],
            "usage: rfs-dev sync-check [--root DIR]",
        ),
        (
            &["sync-fix", "-h"],
            "usage: rfs-dev sync-fix [--map OLD=NEW ...]",
        ),
        (
            &["context", "--help"],
            "usage: rfs-dev context <name> [--file F] [--depth N]",
        ),
        (
            &["edit-rename", "--help"],
            "usage: rfs-dev edit-rename <old> <new>",
        ),
        (&["edit-move", "-h"], "usage: rfs-dev edit-move <from> <to>"),
        (
            &["edit-replace-item", "--help"],
            "usage: rfs-dev edit-replace-item <file> <item> --new-from FILE",
        ),
        (
            &["doc-section", "--help"],
            "usage: rfs-dev doc-section <file|all> <heading...>",
        ),
        (
            &["inventory", "add", "--help"],
            "usage: rfs-dev inventory add --doc SRC|TEST --kind fn|type --id",
        ),
        (
            &["inventory", "update", "-h"],
            "usage: rfs-dev inventory update --doc SRC|TEST --kind fn|type --id",
        ),
        (
            &["inventory", "remove", "--help"],
            "usage: rfs-dev inventory remove --doc SRC|TEST --kind fn|type --id",
        ),
        (
            &["inventory", "audit", "--help"],
            "usage: rfs-dev inventory audit [--root DIR]",
        ),
        (
            &["calls", "extract", "--help"],
            "usage: rfs-dev calls extract <commands.md> --out FILE",
        ),
        (
            &["calls", "classify", "-h"],
            "usage: rfs-dev calls classify <calls.jsonl> --kind terminal|edit_file|python|misc",
        ),
    ];
    for (args, needle) in cases {
        let (code, out) = run_stdout(args);
        assert_eq!(code, 0, "args: {args:?}");
        assert!(out.contains(needle), "args: {args:?}, stdout was: {out}");
    }
}

#[test]
fn group_help_exits_0_and_lists_the_subverbs() {
    let (code, out) = run_stdout(&["inventory", "--help"]);
    assert_eq!(code, 0);
    for v in ["add", "update", "remove", "audit"] {
        assert!(out.contains(v), "missing {v} in: {out}");
    }
    let (code, out) = run_stdout(&["calls", "-h"]);
    assert_eq!(code, 0);
    for v in ["extract", "classify"] {
        assert!(out.contains(v), "missing {v} in: {out}");
    }
}
