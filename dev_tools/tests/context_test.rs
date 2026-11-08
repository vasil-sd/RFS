//! Phase 12 task 5.2: context end-to-end on a fixture repo.
//! The fixtures under tests/fixtures/context/ are committed synthetic trees
//! (a 2-deep Depends chain, a planted missing dependency, an ambiguous bare
//! name across the two inventories, fragment boundary cases, a fenced
//! DECISIONS.md negative control) - the tests never touch the real repository
//! tree (the real-tree demonstration is recorded in the task status file).

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/context")
}

fn run_context(args: &[&str]) -> (i32, String, String) {
    let out = Command::new(env!("CARGO_BIN_EXE_rfs-dev"))
        .args(args)
        .output()
        .expect("rfs-dev binary must exist and run");
    (
        out.status.code().unwrap_or(-1),
        String::from_utf8_lossy(&out.stdout).into_owned(),
        String::from_utf8_lossy(&out.stderr).into_owned(),
    )
}

fn ctx(args: &[&str]) -> (i32, String, String) {
    let root = fixture();
    let mut full = vec!["context"];
    full.extend_from_slice(args);
    full.push("--root");
    full.push(root.to_str().unwrap());
    run_context(&full)
}

/// The output contract: the FULL rendered template for `top` (disambiguated
/// with --file), byte-for-byte. Any template drift breaks this test.
#[test]
fn full_template_is_byte_pinned() {
    let (code, out, err) = ctx(&["top", "--file", "src/a.rs"]);
    assert_eq!(code, 0, "stderr was: {err}");
    let expected = "## Context for top

### Inventory row

| Function | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|
| src/a.rs::top | src/a.rs::mid, src/b.rs::ghost | comp | func | top-level entry | dev:alpha-overview, llm:alpha-card |

### Depends closure

depth limit: 1

- depth 1:
  - src/a.rs::mid
  - src/b.rs::ghost (missing from inventory)

### Documented in

#### dev:alpha-overview — docs/dev/alpha.md

## Overview

Overview paragraph one.
Overview paragraph two.

A deeper heading does not end the fragment:

### Subsection inside overview

Inner content line.

#### llm:alpha-card — docs/llm/alpha.md

## Card

Card content.

### Related decisions

- D1. Choose the top entry point (docs/dev/DECISIONS.md)
";
    assert_eq!(out, expected, "template drift");
}

#[test]
fn depth_2_reaches_the_leaf_and_default_depth_does_not() {
    let (_, out1, _) = ctx(&["top", "--file", "src/a.rs"]);
    assert!(
        !out1.contains("src/b.rs::leaf"),
        "depth 1 must not reach the leaf:\n{out1}"
    );

    let (code, out2, err) = ctx(&["top", "--file", "src/a.rs", "--depth", "2"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out2.contains("depth limit: 2"), "stdout was: {out2}");
    let depth2 = out2
        .lines()
        .rev()
        .take_while(|l| *l != "- depth 2:")
        .collect::<Vec<_>>();
    // closure entries are rendered as list items: `  - <id>`
    assert!(
        depth2.iter().any(|l| l.trim() == "- src/b.rs::leaf"),
        "stdout was: {out2}"
    );
    // the missing dependency stays marked at its own depth, not re-listed later
    assert_eq!(out2.matches("(missing from inventory)").count(), 1);
}

#[test]
fn ambiguous_bare_name_lists_all_rows_and_exits_1() {
    let (code, out, err) = ctx(&["top"]);
    assert_eq!(code, 1, "stdout was: {out}");
    assert!(
        err.contains("ambiguous: 2 inventory rows match `top`"),
        "stderr was: {err}"
    );
    assert!(
        err.contains("src/a.rs::top (docs/llm/SRC_STRUCTURE.md)"),
        "stderr was: {err}"
    );
    assert!(
        err.contains("tests/t.rs::top (docs/llm/TEST_STRUCTURE.md)"),
        "stderr was: {err}"
    );
    assert!(err.contains("--file"), "stderr was: {err}");
    assert!(out.is_empty(), "errors go to stderr; stdout was: {out}");
}

#[test]
fn unknown_name_exits_1_with_a_pointed_message() {
    let (code, _, err) = ctx(&["nope"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("no inventory row for `nope` in SRC_STRUCTURE.md or TEST_STRUCTURE.md"),
        "stderr was: {err}"
    );
}

#[test]
fn file_restriction_that_matches_nothing_is_a_pointed_error() {
    let (code, _, err) = ctx(&["top", "--file", "src/b.rs"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("no inventory row for `top` in src/b.rs"),
        "stderr was: {err}"
    );
}

#[test]
fn type_lookup_renders_all_three_empty_sections() {
    let (code, out, err) = ctx(&["TopCfg"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains("| src/a.rs::TopCfg |  | comp | func | config type |  |"),
        "stdout was: {out}"
    );
    // no dependencies, no Documented-in column, no decisions - all three
    // sections render their pinned empty form.
    let none_count = out.lines().filter(|l| *l == "(none)").count();
    assert_eq!(none_count, 3, "stdout was: {out}");
}

#[test]
fn leaf_pins_the_single_fragment_without_a_trailing_separator() {
    let (code, out, err) = ctx(&["leaf"]);
    assert_eq!(code, 0, "stderr was: {err}");
    // exactly one fragment block; the content ends at the same-level heading
    assert!(
        out.contains(
            "#### dev:alpha-detail — docs/dev/alpha.md\n\n## Detail\n\nDetail content line.\n"
        ),
        "stdout was: {out}"
    );
    assert!(
        !out.contains("Unrelated section"),
        "fragment ran past its boundary:\n{out}"
    );
    assert!(out.lines().any(|l| l == "(none)"), "closure must be (none)");
}

#[test]
fn usage_errors_exit_2() {
    // no name
    let (code, _, err) = ctx(&[]);
    assert_eq!(code, 2, "stderr was: {err}");
    assert!(err.contains("missing the item name"), "stderr was: {err}");
    // two positionals
    let (code, _, err) = ctx(&["a", "b"]);
    assert_eq!(code, 2, "stderr was: {err}");
    // a full identifier is not a bare name
    let (code, _, err) = ctx(&["src/a.rs::top"]);
    assert_eq!(code, 2, "stderr was: {err}");
    assert!(err.contains("not a bare item name"), "stderr was: {err}");
    // depth must be a positive integer
    for bad in ["0", "x", "-1"] {
        let (code, _, err) = ctx(&["top", "--depth", bad]);
        assert_eq!(code, 2, "stderr was: {err}");
        assert!(
            err.contains("--depth expects a positive integer"),
            "stderr was: {err}"
        );
    }
    // unknown flag
    let (code, _, err) = ctx(&["top", "--bogus"]);
    assert_eq!(code, 2, "stderr was: {err}");
    // --file without a value
    let (code, _, err) = ctx(&["top", "--file"]);
    assert_eq!(code, 2, "stderr was: {err}");
}
