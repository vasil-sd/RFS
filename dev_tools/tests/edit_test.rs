//! Phase 12 task 5.3: edit-rename / edit-move / edit-replace-item end-to-end
//! on a fixture repo. The fixtures under tests/fixtures/edit/ are committed
//! synthetic trees (a mini crate with cross-module imports, string literals
//! and comments carrying the rename target - the phantom negative control).
//! Mutating tools change their tree, so every test first copies the fixture
//! to a private temp dir and runs the binary against that copy with --root;
//! the committed fixtures are never touched (the real-tree demonstration is
//! recorded in the task status file).

use std::path::{Path, PathBuf};
use std::process::Command;

fn fixture() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/edit")
}

/// Copy the committed fixture tree to a fresh temp dir (the work area).
fn work_area(tag: &str) -> PathBuf {
    let dir = std::env::temp_dir().join(format!(
        "rfs-dev-edit-it-{}-{}-{}",
        std::process::id(),
        tag,
        std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap()
            .as_nanos()
    ));
    copy_recursive(&fixture(), &dir);
    dir
}

fn copy_recursive(from: &Path, to: &Path) {
    std::fs::create_dir_all(to).unwrap();
    for entry in std::fs::read_dir(from).unwrap().flatten() {
        let p = entry.path();
        let dest = to.join(p.file_name().unwrap());
        if p.is_dir() {
            copy_recursive(&p, &dest);
        } else {
            std::fs::copy(&p, &dest).unwrap();
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

// --- edit-rename -----------------------------------------------------------

#[test]
fn rename_rewrites_every_import_site() {
    let root = work_area("rename");
    let (code, out, err) = run(&root, &["edit-rename", "target_fn", "doubled"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains("plan: declaration: src/alpha.rs:2 fn `target_fn` -> `doubled`"),
        "{out}"
    );
    assert!(out.contains("applied: 3 file(s) changed"), "{out}");

    let alpha = read(&root, "src/alpha.rs");
    assert!(alpha.contains("pub fn doubled(x: u32) -> u32 {"), "{alpha}");
    assert!(
        !alpha.contains("target_fn"),
        "declaration file must be clean: {alpha}"
    );

    let lib = read(&root, "src/lib.rs");
    assert!(lib.contains("crate::alpha::doubled(1)"), "{lib}");

    let beta = read(&root, "src/beta.rs");
    assert_eq!(
        beta, "use crate::alpha::doubled;\n\npub fn helper() -> u32 {\n    doubled(10)\n}\n",
        "{beta}"
    );
}

#[test]
fn rename_leaves_string_literals_and_comments_byte_identical() {
    let root = work_area("phantom");
    let before_lib = read(&root, "src/lib.rs");
    let (code, _out, err) = run(&root, &["edit-rename", "target_fn", "doubled"]);
    assert_eq!(code, 0, "stderr was: {err}");
    let after_lib = read(&root, "src/lib.rs");
    // The phantom lines must survive the rename byte-for-byte.
    assert!(
        after_lib.contains("let note = \"target_fn is documented here\";"),
        "{after_lib}"
    );
    assert!(
        after_lib.contains("// target_fn: see the alpha module"),
        "{after_lib}"
    );
    // Everything else in the file changed exactly where planned.
    assert_ne!(before_lib, after_lib);
    assert!(
        !after_lib.contains("crate::alpha::target_fn"),
        "{after_lib}"
    );
}

#[test]
fn rename_dry_run_prints_plan_and_writes_nothing() {
    let root = work_area("dryrun");
    let before: Vec<String> = ["src/lib.rs", "src/alpha.rs", "src/beta.rs"]
        .iter()
        .map(|f| read(&root, f))
        .collect();
    let (code, out, err) = run(&root, &["edit-rename", "target_fn", "doubled", "--dry-run"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out.contains("plan: declaration:"), "{out}");
    assert!(out.contains("dry-run: 3 file(s) would change"), "{out}");
    for (f, content) in ["src/lib.rs", "src/alpha.rs", "src/beta.rs"]
        .iter()
        .zip(&before)
    {
        assert_eq!(&read(&root, f), content, "dry-run must not write {f}");
    }
}

#[test]
fn rename_not_found_exits_1() {
    let root = work_area("notfound");
    let (code, _out, err) = run(&root, &["edit-rename", "ghost_item", "x"]);
    assert_eq!(code, 1);
    assert!(err.contains("`ghost_item` is not declared"), "{err}");
}

#[test]
fn rename_ambiguous_exits_1_and_lists_every_site() {
    let root = work_area("ambiguous");
    // Plant a second declaration of the same name in another file.
    std::fs::write(
        root.join("src/gamma.rs"),
        "pub fn target_fn() -> u32 { 0 }\n",
    )
    .unwrap();
    let (code, _out, err) = run(&root, &["edit-rename", "target_fn", "doubled"]);
    assert_eq!(code, 1);
    assert!(
        err.starts_with("rfs-dev edit-rename: ambiguous: `target_fn` is declared 2 times"),
        "{err}"
    );
    assert!(err.contains("src/alpha.rs:2 fn"), "{err}");
    assert!(err.contains("src/gamma.rs:1 fn"), "{err}");
}

#[test]
fn rename_usage_errors_exit_2() {
    let root = work_area("usage");
    let (code, _o, err) = run(&root, &["edit-rename", "only_one"]);
    assert_eq!(code, 2, "{err}");
    let (code, _o, err) = run(&root, &["edit-rename", "a", "b", "--bogus"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("unknown flag `--bogus`"), "{err}");
    let (code, _o, err) = run(&root, &["edit-rename", "a::b", "c"]);
    assert_eq!(code, 2, "{err}");
    assert!(err.contains("not a bare item name"), "{err}");
}

// --- edit-move ---------------------------------------------------------------

#[test]
fn move_rewrites_import_sites_and_mod_declarations() {
    let root = work_area("move");
    let (code, out, err) = run(&root, &["edit-move", "src/beta.rs", "src/omega/beta.rs"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains("plan: move src/beta.rs -> src/omega/beta.rs (module beta -> omega::beta)"),
        "{out}"
    );
    assert!(
        out.contains("plan: src/lib.rs: remove declaration `pub mod beta;`"),
        "{out}"
    );
    assert!(
        out.contains("plan: create src/omega/mod.rs with `pub mod beta;`"),
        "{out}"
    );
    // The new intermediate directory module is declared in lib.rs.
    assert!(
        out.contains("plan: src/lib.rs: add declaration `pub mod omega;`"),
        "{out}"
    );
    assert!(
        out.contains("plan: src/lib.rs: rewrite 2 import site(s) of module `beta`"),
        "{out}"
    );
    assert!(out.contains("applied:"), "{out}");

    assert!(!root.join("src/beta.rs").exists(), "old file must be gone");
    assert!(
        root.join("src/omega/beta.rs").is_file(),
        "new file must exist"
    );
    assert_eq!(read(&root, "src/omega/mod.rs"), "pub mod beta;\n");

    let lib = read(&root, "src/lib.rs");
    assert!(!lib.lines().any(|l| l.trim() == "pub mod beta;"), "{lib}");
    assert!(lib.contains("pub mod omega;"), "{lib}");
    assert!(
        lib.contains("omega::beta::helper() + crate::omega::beta::helper()"),
        "{lib}"
    );
    // The phantom lines survive a move too.
    assert!(lib.contains("\"target_fn is documented here\""), "{lib}");
}

#[test]
fn move_same_parent_rewrites_stem_in_place() {
    let root = work_area("movesame");
    let (code, out, err) = run(&root, &["edit-move", "src/beta.rs", "src/gamma.rs"]);
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains("plan: src/lib.rs: rewrite declaration `pub mod beta;` -> `pub mod gamma;`"),
        "{out}"
    );
    let lib = read(&root, "src/lib.rs");
    assert!(lib.contains("pub mod gamma;"), "{lib}");
    assert!(!lib.lines().any(|l| l.trim() == "pub mod beta;"), "{lib}");
    assert!(
        lib.contains("gamma::helper() + crate::gamma::helper()"),
        "{lib}"
    );
    assert!(root.join("src/gamma.rs").is_file());
}

#[test]
fn move_dry_run_moves_nothing() {
    let root = work_area("movedry");
    let (code, out, err) = run(
        &root,
        &["edit-move", "src/beta.rs", "src/omega/beta.rs", "--dry-run"],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out.contains("dry-run:"), "{out}");
    assert!(
        root.join("src/beta.rs").is_file(),
        "dry-run must not move the file"
    );
    assert!(
        !root.join("src/omega").exists(),
        "dry-run must not create the target dir"
    );
    let lib = read(&root, "src/lib.rs");
    assert!(
        lib.contains("pub mod beta;"),
        "dry-run must not touch declarations: {lib}"
    );
}

#[test]
fn move_errors_exit_1() {
    let root = work_area("moveerr");
    let (code, _o, err) = run(&root, &["edit-move", "src/nope.rs", "src/omega/nope.rs"]);
    assert_eq!(code, 1);
    assert!(err.contains("src/nope.rs: no such file"), "{err}");
    let (code, _o, err) = run(&root, &["edit-move", "src/beta.rs", "src/alpha.rs"]);
    assert_eq!(code, 1);
    assert!(err.contains("src/alpha.rs: target already exists"), "{err}");
}

// --- edit-replace-item ---------------------------------------------------------

#[test]
fn replace_item_swaps_only_the_named_item() {
    let root = work_area("replace");
    std::fs::write(
        root.join("new_target.txt"),
        "pub fn target_fn(x: u32) -> u32 {\n    x + 41\n}\n",
    )
    .unwrap();
    let (code, out, err) = run(
        &root,
        &[
            "edit-replace-item",
            "src/alpha.rs",
            "target_fn",
            "--new-from",
            "new_target.txt",
        ],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(
        out.contains(
            "plan: src/alpha.rs: replace fn `target_fn` (lines 1-4) with content of new_target.txt"
        ),
        "{out}"
    );
    assert!(out.contains("applied: 1 file(s) changed"), "{out}");

    // The item's leading doc comment is an attribute - it belongs to the old
    // item and goes with it; the replacement content brings its own docs.
    let alpha = read(&root, "src/alpha.rs");
    assert_eq!(
        alpha,
        "pub fn target_fn(x: u32) -> u32 {\n    x + 41\n}\n\npub struct Alpha;\n\nimpl Alpha {\n    pub fn make() -> Self {\n        Alpha\n    }\n}\n",
        "{alpha}"
    );
}

#[test]
fn replace_item_dry_run_writes_nothing() {
    let root = work_area("replacedry");
    std::fs::write(
        root.join("new_target.txt"),
        "pub fn target_fn(x: u32) -> u32 { x }\n",
    )
    .unwrap();
    let before = read(&root, "src/alpha.rs");
    let (code, out, err) = run(
        &root,
        &[
            "edit-replace-item",
            "src/alpha.rs",
            "target_fn",
            "--new-from",
            "new_target.txt",
            "--dry-run",
        ],
    );
    assert_eq!(code, 0, "stderr was: {err}");
    assert!(out.contains("dry-run: 1 file(s) would change"), "{out}");
    assert_eq!(read(&root, "src/alpha.rs"), before);
}

#[test]
fn replace_item_errors() {
    let root = work_area("replaceerr");
    // --new-from is required.
    let (code, _o, err) = run(&root, &["edit-replace-item", "src/alpha.rs", "target_fn"]);
    assert_eq!(code, 2);
    assert!(err.contains("--new-from FILE is required"), "{err}");
    // Missing item.
    std::fs::write(root.join("n.txt"), "fn x() {}\n").unwrap();
    let (code, _o, err) = run(
        &root,
        &[
            "edit-replace-item",
            "src/alpha.rs",
            "ghost",
            "--new-from",
            "n.txt",
        ],
    );
    assert_eq!(code, 1);
    assert!(
        err.contains("`ghost` is not declared in src/alpha.rs"),
        "{err}"
    );
    // Ambiguous: the method `make`... no - use a name that hits both a top-level
    // item and an associated item: plant one.
    std::fs::write(
        root.join("src/delta.rs"),
        "struct D;\nimpl D {\n    pub fn dup() {}\n}\npub fn dup() {}\n",
    )
    .unwrap();
    let (code, _o, err) = run(
        &root,
        &[
            "edit-replace-item",
            "src/delta.rs",
            "dup",
            "--new-from",
            "n.txt",
        ],
    );
    assert_eq!(code, 1);
    assert!(
        err.starts_with(
            "rfs-dev edit-replace-item: ambiguous: `dup` matches 2 items in src/delta.rs:"
        ),
        "{err}"
    );
    assert!(err.contains("method") && err.contains("fn"), "{err}");
}
