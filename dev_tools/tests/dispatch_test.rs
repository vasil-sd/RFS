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
