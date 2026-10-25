//! Documentation quality suite (Phase 6 task 7 prep; user rule change 2026-10-25).
//!
//! Validates every Markdown document in the repository with **mdr's own parser
//! stack**. `mdr` itself (github.com/clevercloud/mdr, crates.io `mdr`) is a
//! binary-only crate - cargo ignores it as a dependency ("missing a lib
//! target") - so this suite depends on mdr's parser parts directly, at the
//! exact versions mdr 0.6.1 pins:
//!
//! * `comrak` 0.55 - GFM Markdown -> AST (mdr's structural parser);
//! * `mermaid-rs-renderer` 0.3 (default-features = false, SVG only) - the
//!   renderer mdr uses for ```mermaid blocks.
//!
//! Per document this suite checks:
//!
//! 1. the Markdown parses with comrak under mdr's GFM extension set;
//! 2. every fenced code block is explicitly closed (`NodeCodeBlock.closed` -
//!    an unterminated fence would silently swallow the rest of the file);
//! 3. every ```mermaid block survives mermaid-rs-renderer's FULL pipeline
//!    (validator + parse + layout + SVG render) - i.e. exactly what mdr does,
//!    so a diagram that fails here cannot render in mdr either;
//! 4. every relative link/image target resolves to an existing path
//!    (the cross-link invariant for `docs/user/` and the rest of the docs).
//!
//! The negative-control tests at the bottom prove the checker catches each
//! failure class (unterminated fence, non-rendering mermaid, dead link).

use std::path::{Path, PathBuf};

/// The GFM extension set mdr enables (tables, task lists, strikethrough,
/// footnotes, autolinks - see the mdr README "Features" section).
fn mdr_options() -> comrak::Options<'static> {
    let mut options = comrak::Options::default();
    options.extension.strikethrough = true;
    options.extension.table = true;
    options.extension.tasklist = true;
    options.extension.footnotes = true;
    options.extension.autolink = true;
    options
}

/// Minimal percent-decoding for link targets (`%20` -> space, ...).
fn url_decode(s: &str) -> String {
    let b = s.as_bytes();
    let mut out = Vec::with_capacity(b.len());
    let mut i = 0;
    while i < b.len() {
        if b[i] == b'%' && i + 2 < b.len() {
            if let (Some(hi), Some(lo)) = (hex_val(b[i + 1]), hex_val(b[i + 2])) {
                out.push(hi * 16 + lo);
                i += 3;
                continue;
            }
        }
        out.push(b[i]);
        i += 1;
    }
    String::from_utf8_lossy(&out).into_owned()
}

fn hex_val(b: u8) -> Option<u8> {
    match b {
        b'0'..=b'9' => Some(b - b'0'),
        b'a'..=b'f' => Some(b - b'a' + 10),
        b'A'..=b'F' => Some(b - b'A' + 10),
        _ => None,
    }
}

/// Check one link/image URL: skip external and pure-fragment targets; resolve
/// relative targets against the document's directory.
fn check_link(url: &str, base_dir: &Path, line: usize, problems: &mut Vec<String>) {
    let u = url.trim();
    if u.is_empty() || u.starts_with('#') {
        return;
    }
    if u.contains("://") || u.starts_with("mailto:") || u.starts_with("data:") {
        return;
    }
    let path_part = u.split('#').next().unwrap_or("");
    if path_part.is_empty() {
        return;
    }
    let decoded = url_decode(path_part);
    let target = base_dir.join(&decoded);
    if !target.exists() {
        problems.push(format!(
            "line {line}: relative link `{u}` does not resolve (expected at {})",
            target.display()
        ));
    }
}

/// Run all document-quality checks over one Markdown source. Returns the list
/// of problems (empty = the document is clean).
fn check_markdown(content: &str, base_dir: &Path) -> Vec<String> {
    let mut problems = vec![];
    let arena = comrak::Arena::new();
    let options = mdr_options();
    let root = comrak::parse_document(&arena, content, &options);

    // comrak 0.55 tree API: `Node<'a>` is `&AstNode<'a>`; data lives in
    // `node.data` (`RefCell<Ast>`); traversal via `first_child()` /
    // `next_sibling()` (both return `Option<&Node>`, i.e. `Option<&&AstNode>`).
    fn walk(node: comrak::Node<'_>, base_dir: &Path, problems: &mut Vec<String>) {
        let line;
        let mut mermaid_literal: Option<String> = None;
        {
            let ast = node.data.borrow();
            line = ast.sourcepos.start.line;
            match &ast.value {
                comrak::nodes::NodeValue::CodeBlock(cb) => {
                    if cb.fenced && !cb.closed {
                        problems.push(format!(
                            "line {line}: fenced code block (info `{}`) is never closed",
                            cb.info
                        ));
                    }
                    if cb.fenced && cb.info.trim().eq_ignore_ascii_case("mermaid") {
                        mermaid_literal = Some(cb.literal.clone());
                    }
                }
                comrak::nodes::NodeValue::Link(l) | comrak::nodes::NodeValue::Image(l) => {
                    check_link(&l.url, base_dir, line, problems);
                }
                _ => {}
            }
        }
        // Render outside the borrow: this is mdr's full mermaid pipeline
        // (validator + parse + layout + SVG), so any Err here means mdr itself
        // could not render the block.
        if let Some(lit) = mermaid_literal {
            if let Err(e) = mermaid_rs_renderer::render_strict(
                &lit,
                mermaid_rs_renderer::RenderOptions::default(),
            ) {
                problems.push(format!(
                    "line {line}: mermaid block does not render in mdr's renderer: {e}"
                ));
            }
        }
        let mut child = node.first_child();
        while let Some(c) = child {
            walk(c, base_dir, problems);
            child = c.next_sibling();
        }
    }

    walk(root, base_dir, &mut problems);
    problems
}

/// Collect every `.md` file in the repo. Skips `target/`, `__pycache__/`
/// and all hidden directories (`.git/`, `.ref_docs/` - external reference
/// material whose links are site-relative, not repo-relative).
fn collect_docs(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    fn walk_dir(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                let skip = name.starts_with('.') || matches!(name, "target" | "__pycache__");
                if !skip {
                    walk_dir(&p, out);
                }
            } else if p.extension().is_some_and(|e| e == "md") {
                out.push(p);
            }
        }
    }
    walk_dir(root, &mut out);
    out.sort();
    out
}

#[test]
fn all_repo_docs_pass_quality_checks() {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let docs = collect_docs(&root);

    // Sanity: the walk must have found the core doc set (a silent empty walk
    // would make this test pass vacuously).
    assert!(
        docs.len() >= 10,
        "expected at least 10 .md files, found {}",
        docs.len()
    );
    let rel: Vec<String> = docs
        .iter()
        .map(|p| {
            p.strip_prefix(&root)
                .unwrap()
                .to_string_lossy()
                .into_owned()
        })
        .collect();
    for required in [
        "START.md",
        "BUILD.md",
        "TOOLCHAIN.md",
        "docs/roadmap/INTRO.md",
    ] {
        assert!(
            rel.iter().any(|r| r == required),
            "required doc missing from the walk: {required} (found {} files)",
            rel.len()
        );
    }

    let mut failures = vec![];
    for path in &docs {
        let content = std::fs::read_to_string(path).expect("doc must be UTF-8 readable");
        let problems = check_markdown(&content, path.parent().unwrap());
        if !problems.is_empty() {
            let relp = path.strip_prefix(&root).unwrap().to_string_lossy();
            failures.push(format!("{relp}:\n  {}", problems.join("\n  ")));
        }
    }

    assert!(
        failures.is_empty(),
        "{} of {} docs failed the mdr-stack quality checks:\n{}",
        failures.len(),
        docs.len(),
        failures.join("\n")
    );
}

#[test]
fn valid_mermaid_diagrams_and_links_pass() {
    // Positive control: the three diagram kinds task 7 will use most (flowchart
    // for structure, state for FSMs, sequence for protocols) must render
    // through mdr's renderer, and a relative link into the repo must resolve.
    let content = r#"# Control doc

See [the entry point](START.md).

```mermaid
flowchart LR
    subgraph Toolchain
        P[parser] --> V[validator]
        V --> I[IR]
    end
    I --> D{valid?}
    D -->|yes| R[rust backend]
    D -->|no| E[diagnostic]
```

```mermaid
stateDiagram-v2
    [*] --> Idle
    Idle --> Open : SendData(w) : w > 0
    Open --> Closed : RST_STREAM
    Closed --> [*]
```

```mermaid
sequenceDiagram
    participant H as Host
    participant D as Device
    H->>D: SETUP
    D-->>H: ACK
```
"#;
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let problems = check_markdown(content, &root);
    assert!(problems.is_empty(), "positive control failed: {problems:?}");
}

#[test]
fn unterminated_fence_is_detected() {
    let content = "# Doc\n\n```sh\necho never closed\n";
    let problems = check_markdown(content, Path::new("."));
    assert!(
        problems.iter().any(|p| p.contains("never closed")),
        "expected an unterminated-fence problem, got: {problems:?}"
    );
}

#[test]
fn non_rendering_mermaid_is_detected() {
    // An unclosed subgraph is a typed parse error in mermaid-rs-renderer's
    // strict pipeline - mdr could not render this block either.
    let content = "# Doc\n\n```mermaid\nflowchart TD\n    subgraph X\n        A --> B\n```\n";
    let problems = check_markdown(content, Path::new("."));
    assert!(
        problems
            .iter()
            .any(|p| p.contains("mermaid block does not render")),
        "expected a mermaid-render problem, got: {problems:?}"
    );
}

#[test]
fn dead_relative_link_is_detected() {
    let content =
        "# Doc\n\n[missing](does_not_exist.md) and [external ok](https://example.com/x)\n";
    let problems = check_markdown(content, Path::new("."));
    assert_eq!(
        problems.len(),
        1,
        "expected exactly one problem, got: {problems:?}"
    );
    assert!(
        problems[0].contains("does_not_exist.md"),
        "unexpected problem: {problems:?}"
    );
}
