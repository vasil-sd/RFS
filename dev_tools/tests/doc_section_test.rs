//! Phase 12 task 5.4: doc-section end-to-end on a fixture repo. The fixtures
//! under tests/fixtures/docs/ are committed synthetic docs with overlapping
//! heading names at different depths (a `## Overview` and a nested
//! `### Overview` in one file, the same name in a second file), a fenced
//! block carrying fake headings (negative control) and an empty section (the
//! pinned `(none)` marker). Read-only tool - the tests run straight against
//! the committed fixtures (no temp copy needed); the real-tree demonstration
//! is recorded in the task status file.

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/docs")
}

fn run(args: &[&str]) -> (i32, String, String) {
    let root = fixture();
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

#[test]
fn file_mode_single_heading_is_verbatim() {
    let (code, out, err) = run(&["doc-section", "docs/dev/alpha.md", "Details"]);
    assert_eq!(code, 0, "stderr was: {err}");
    // The section includes the deeper `### Overview` heading and its body.
    assert_eq!(
        out, "details body\n\n### Overview\n\nA deeper Overview, nested under Details.\n",
        "{out:?}"
    );
}

#[test]
fn file_mode_nested_path_reaches_the_same_section() {
    let (code, out, err) = run(&["doc-section", "docs/dev/alpha.md", "Alpha", "Deep"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert_eq!(out, "deep line one\ndeep line two\n", "{out:?}");
    // Direct addressing gives the identical bytes.
    let (code2, out2, _) = run(&["doc-section", "docs/dev/alpha.md", "Deep"]);
    assert_eq!(code2, 0);
    assert_eq!(out2, out);
}

#[test]
fn first_occurrence_wins_for_repeated_headings() {
    // `Overview` occurs twice at ## depth in alpha.md - the first one wins.
    // Its section includes the nested ### Deep subsection (deeper headings
    // are part of the parent's content) and stops at the next ## heading.
    let (code, out, err) = run(&["doc-section", "docs/dev/alpha.md", "Overview"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert_eq!(
        out, "First overview body.\n\n### Deep\n\ndeep line one\ndeep line two\n",
        "{out:?}"
    );
    assert!(
        !out.contains("Second overview"),
        "the second occurrence must not leak: {out:?}"
    );
}

#[test]
fn depth_scoping_reaches_the_nested_overview() {
    // Under `Details`, the ONLY Overview is the nested ### one.
    let (code, out, err) = run(&["doc-section", "docs/dev/alpha.md", "Details", "Overview"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert_eq!(out, "A deeper Overview, nested under Details.\n", "{out:?}");
}

#[test]
fn empty_section_prints_the_pinned_marker() {
    let (code, out, err) = run(&["doc-section", "docs/dev/alpha.md", "Empty"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert_eq!(out, "(none)\n", "{out:?}");
}

#[test]
fn fenced_fake_headings_are_ignored() {
    // The fake headings inside the fence are not resolvable...
    let (code, _out, err) = run(&[
        "doc-section",
        "docs/llm/beta.md",
        "Fake heading inside fence",
    ]);
    assert_eq!(code, 1);
    assert!(
        err.contains("no heading `Fake heading inside fence`"),
        "{err}"
    );
    // ...and they do not truncate the Fenced section's real content.
    let (code, out, err) = run(&["doc-section", "docs/llm/beta.md", "Fenced"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains("# Fake heading inside fence"),
        "fence content is verbatim: {out:?}"
    );
    assert!(out.contains("real body after fence"), "{out:?}");
}

#[test]
fn all_mode_searches_every_docs_file_and_prefixes_matches() {
    let (code, out, err) = run(&["doc-section", "all", "Overview"]);
    assert_eq!(code, 0, "stderr was: {err}");
    // Sorted file order: docs/dev/alpha.md before docs/llm/beta.md. Each match
    // is a `<file>: <path>` line followed by the verbatim content; matches are
    // separated by one blank line.
    assert_eq!(
        out,
        "docs/dev/alpha.md: Overview\nFirst overview body.\n\n### Deep\n\ndeep line one\ndeep line two\n\ndocs/llm/beta.md: Overview\nThe beta overview - same heading name as alpha.md, different file.\n",
        "{out:?}"
    );
}

#[test]
fn all_mode_no_match_exits_1() {
    let (code, _out, err) = run(&["doc-section", "all", "Ghost section"]);
    assert_eq!(code, 1);
    assert!(
        err.contains("no section matching `Ghost section` in any docs file"),
        "{err}"
    );
}

#[test]
fn missing_file_exits_1() {
    let (code, _out, err) = run(&["doc-section", "docs/dev/nope.md", "Overview"]);
    assert_eq!(code, 1);
    assert!(err.contains("docs/dev/nope.md: cannot read"), "{err}");
}

#[test]
fn usage_errors_exit_2() {
    // No heading element.
    let (code, _o, err) = run(&["doc-section", "docs/dev/alpha.md"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("at least one heading element"), "{err}");
    // Unknown flag.
    let (code, _o, err) = run(&["doc-section", "docs/dev/alpha.md", "Overview", "--bogus"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("unknown flag `--bogus`"), "{err}");
}
