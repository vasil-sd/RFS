//! Documentation sync gate (Phase 8 task 1.1).
//!
//! Enforces bidirectional traceability between the documentation sets
//! (`docs/dev/` = Class A for developers, `docs/llm/` = Class B for LLM
//! assistants) and the source code. This suite is part of plain `cargo test`;
//! after any source change its output alone tells you which doc fragments are
//! stale or missing.
//!
//! Checks (all always active over whatever exists under `docs/dev/` +
//! `docs/llm/`):
//!
//! 1. **doc→code**: every inline code span in a doc that looks like a
//!    repo-relative source path must resolve to an existing file, and (for
//!    `.rs` with an `::item` anchor) to an item declared in that file.
//! 2. **code→doc**: every `.rs` file listed in the coverage manifest
//!    (`tests/dev_docs_manifest.txt`) carries exactly one module-doc pointer
//!    line naming ≥1 fragment ID defined in the doc set.
//! 3. **coverage**: if `docs/llm/TASK_MAP.md` exists, every manifest-listed
//!    source file is referenced from it.
//! 4. **no orphans**: every fragment defined in the doc set is named by at
//!    least one source file's pointer line (every fragment has an owner).
//! 5. **Class B budgets**: every `.md` under `docs/llm/` declares a line
//!    budget in its header and stays within it.
//!
//! The pointer-check scope is driven by the manifest, which grew chunk by
//! chunk (tasks 1.2-1.9) and was completed in task 1.10; the completeness
//! check (`manifest_matches_filesystem_scan`) pins manifest == filesystem
//! scan of all 71 hand-written `.rs` files (build-generated artifacts are
//! excluded - see that test).
//!
//! Conventions pinned by this suite (the doc-set READMEs from task 1.2 onward
//! restate them):
//!
//! * **Fragment IDs**: `dev:<name>` (Class A) or `llm:<name>` (Class B), where
//!   `<name>` is kebab-case (`[a-z0-9]+(-[a-z0-9]+)*`). A fragment is defined
//!   by an anchor line `<a id="dev:name"></a>` immediately followed (blank
//!   lines allowed) by a Markdown heading. IDs are unique across the doc set.
//! * **doc→code references**: inline code spans of the form `path/to/file.rs`
//!   or `path/to/file.rs::item` — repo-rooted, at least one path separator,
//!   `.rs`/`.c`/`.h` only; `::item` is supported for `.rs` only. Anchors use
//!   stable identifiers (function/type names), NEVER line numbers. A backticked
//!   string that matches this shape MUST be a real reference — to mention a
//!   filename without referencing it, do not wrap it in backticks.
//! * **code→doc pointers**: exactly one `//! Docs: dev:name1, llm:name2` line
//!   per manifest-listed file (module doc comment; key items may carry
//!   one-line `/// Docs:` pointers as well — a convention, not gate-checked).
//! * **Class B budget header**: `<!-- llm-budget: N -->` within the first 10
//!   lines of each `docs/llm/*.md`; the file's total line count must be ≤ N.
//!
//! The negative-control tests at the bottom (pinned fixtures under
//! `tests/fixtures/dev_docs/`) prove the gate catches each failure class:
//! broken doc→code reference, missing code→doc pointer, dangling pointer,
//! orphaned fragment, budget overflow, malformed manifest entries.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

// ---------------------------------------------------------------------------
// Conventions / parsing helpers
// ---------------------------------------------------------------------------

fn repo_root() -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .map(|p| p.to_string_lossy().into_owned())
        .unwrap_or_else(|_| path.display().to_string())
}

/// Fragment ID grammar: `(dev|llm):` + kebab-case name.
fn is_fragment_id(s: &str) -> bool {
    let Some((class, name)) = s.split_once(':') else {
        return false;
    };
    if !matches!(class, "dev" | "llm") || name.is_empty() {
        return false;
    }
    name.split('-').all(|seg| {
        !seg.is_empty()
            && seg
                .chars()
                .all(|c| c.is_ascii_lowercase() || c.is_ascii_digit())
    })
}

/// doc→code reference: repo-relative source path (≥1 `/`, `.rs`/`.c`/`.h`),
/// optionally followed by `::item`. Returns `(path, item)`.
fn parse_code_ref(span: &str) -> Option<(String, Option<String>)> {
    let span = span.trim();
    let (path_part, item_part) = match span.split_once("::") {
        Some((p, i)) => (p, Some(i)),
        None => (span, None),
    };
    if path_part.is_empty() || !path_part.contains('/') {
        return None;
    }
    if !path_part
        .chars()
        .all(|c| c.is_ascii_alphanumeric() || matches!(c, '/' | '.' | '_' | '-'))
    {
        return None;
    }
    if !(path_part.ends_with(".rs") || path_part.ends_with(".c") || path_part.ends_with(".h")) {
        return None;
    }
    if path_part
        .split('/')
        .any(|seg| seg.is_empty() || seg == "..")
    {
        return None;
    }
    let item = item_part.and_then(|i| {
        let mut chars = i.chars();
        let ok = match chars.next()? {
            c if c == '_' || c.is_ascii_alphabetic() => {
                chars.all(|c| c == '_' || c.is_ascii_alphanumeric())
            }
            _ => false,
        };
        ok.then(|| i.to_string())
    });
    Some((path_part.to_string(), item))
}

/// Inline code spans of a Markdown document that parse as doc→code references:
/// `(line, span_text, path, item)`. Fenced/indented code blocks are NOT
/// scanned (their text is not inline content in comrak's AST), so examples of
/// the reference syntax inside ``` fences in the docs themselves are safe.
fn scan_doc_refs(content: &str) -> Vec<(usize, String, String, Option<String>)> {
    let arena = comrak::Arena::new();
    let options = comrak::Options::default();
    let root = comrak::parse_document(&arena, content, &options);
    let mut refs = vec![];

    fn walk(node: comrak::Node<'_>, refs: &mut Vec<(usize, String, String, Option<String>)>) {
        let line;
        let literal: Option<String>;
        {
            let ast = node.data.borrow();
            line = ast.sourcepos.start.line;
            literal = match &ast.value {
                comrak::nodes::NodeValue::Code(c) => Some(c.literal.clone()),
                _ => None,
            };
        }
        if let Some(lit) = literal {
            if let Some((path, item)) = parse_code_ref(&lit) {
                refs.push((line, lit, path, item));
            }
        }
        let mut child = node.first_child();
        while let Some(c) = child {
            walk(c, refs);
            child = c.next_sibling();
        }
    }

    walk(root, &mut refs);
    refs
}

struct FragDef {
    id: String,
    file: PathBuf,
    line: usize,
}

/// Lines of a doc with fence state: `(line_no, line, inside_fence)`. Fence
/// lines themselves are reported as outside. (Well-formed fences are enforced
/// by `tests/docs_md_test.rs`; this tracker only needs to skip fenced blocks.)
fn doc_lines_with_fence(content: &str) -> Vec<(usize, &str, bool)> {
    let mut out = vec![];
    let mut in_fence = false;
    for (i, line) in content.lines().enumerate() {
        let t = line.trim_start();
        let is_fence = t.starts_with("```") || t.starts_with("~~~");
        if is_fence {
            in_fence = !in_fence;
            out.push((i + 1, line, false));
        } else {
            out.push((i + 1, line, in_fence));
        }
    }
    out
}

/// Collect fragment definitions (`<a id="..."></a>` anchor lines outside code
/// fences). Returns `(defs, problems)`.
fn collect_fragment_defs(root: &Path, path: &Path, content: &str) -> (Vec<FragDef>, Vec<String>) {
    let mut defs = vec![];
    let mut problems = vec![];
    let lines = doc_lines_with_fence(content);
    for (idx, (line_no, line, in_fence)) in lines.iter().enumerate() {
        if *in_fence {
            continue;
        }
        let t = line.trim();
        let Some(inner) = t
            .strip_prefix("<a id=\"")
            .and_then(|s| s.strip_suffix("\"></a>"))
        else {
            continue;
        };
        if !is_fragment_id(inner) {
            problems.push(format!(
                "{}:{}: anchor ID `{inner}` is not a fragment ID (expected `dev:<kebab>` or `llm:<kebab>`)",
                rel(root, path),
                line_no
            ));
            continue;
        }
        // The anchor must be immediately followed by a heading (blank lines allowed).
        let mut j = idx + 1;
        while j < lines.len() && lines[j].1.trim().is_empty() {
            j += 1;
        }
        if !(j < lines.len() && !lines[j].2 && lines[j].1.starts_with('#')) {
            problems.push(format!(
                "{}:{}: fragment anchor `{inner}` must be immediately followed by a heading",
                rel(root, path),
                line_no
            ));
        }
        defs.push(FragDef {
            id: inner.to_string(),
            file: path.to_path_buf(),
            line: *line_no,
        });
    }
    (defs, problems)
}

struct PointerParse {
    ids: Vec<String>,
    docs_line_count: usize,
    problems: Vec<String>,
}

/// Parse the `//! Docs:` pointer lines of a Rust source file. In valid Rust all
/// `//!` lines are module-level (inner doc comments precede every item), so
/// scanning the whole file is equivalent to scanning the header block.
fn parse_code_pointers(content: &str) -> PointerParse {
    let mut pp = PointerParse {
        ids: vec![],
        docs_line_count: 0,
        problems: vec![],
    };
    for line in content.lines() {
        let Some(t) = line.trim_start().strip_prefix("//!") else {
            continue;
        };
        let t = t.trim_start();
        let Some(rest) = t.strip_prefix("Docs:") else {
            continue;
        };
        pp.docs_line_count += 1;
        let rest = rest.trim();
        if rest.is_empty() {
            pp.problems
                .push("`//! Docs:` line names no fragments".to_string());
            continue;
        }
        for part in rest.split(',') {
            let id = part.trim();
            if id.is_empty() {
                pp.problems.push(format!(
                    "malformed `//! Docs:` list (empty entry): `{line}`"
                ));
            } else if !is_fragment_id(id) {
                pp.problems.push(format!(
                    "`//! Docs:` entry `{id}` is not a fragment ID (expected `dev:<kebab>` or `llm:<kebab>`)"
                ));
            } else {
                pp.ids.push(id.to_string());
            }
        }
    }
    pp
}

fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

/// True if `s` starts with the identifier `item` (whole-token match).
fn starts_with_ident(s: &str, item: &str) -> bool {
    let Some(rest) = s.strip_prefix(item) else {
        return false;
    };
    rest.is_empty() || !rest.starts_with(is_ident_char)
}

/// Strip a leading `pub` / `pub(...)` visibility qualifier.
fn strip_visibility(s: &str) -> &str {
    let Some(r) = s.strip_prefix("pub") else {
        return s;
    };
    if r.starts_with('(') {
        let mut depth = 0usize;
        for (i, b) in r.bytes().enumerate() {
            match b {
                b'(' => depth += 1,
                b')' => {
                    depth -= 1;
                    if depth == 0 {
                        return s["pub".len() + i + 1..].trim_start();
                    }
                }
                _ => {}
            }
        }
        return s; // unbalanced — not a visibility qualifier
    }
    if r.starts_with(|c: char| c.is_whitespace()) {
        return r.trim_start();
    }
    s // e.g. "pubfoo" — not a visibility qualifier
}

/// True if `s` (visibility already stripped) starts with an item declaration
/// keyword followed by the identifier `item`.
fn keyword_declares(s: &str, item: &str) -> bool {
    for kw in [
        "fn", "struct", "enum", "trait", "type", "const", "static", "mod",
    ] {
        let Some(r) = s.strip_prefix(kw) else {
            continue;
        };
        if !r.starts_with(|c: char| c.is_whitespace()) {
            continue; // not a whole keyword (e.g. "function")
        }
        let r = r.trim_start();
        let r = if kw == "static" {
            r.strip_prefix("mut ").map(str::trim_start).unwrap_or(r)
        } else {
            r
        };
        if starts_with_ident(r, item) {
            return true;
        }
    }
    false
}

/// True if an `impl` header line (text after the `impl` keyword) names `item`
/// as the impl'd type: the first ident after optional generics, or the ident
/// after `for`.
fn impl_line_names_type(after_impl: &str, item: &str) -> bool {
    let mut rest = after_impl.trim_start();
    if let Some(r) = rest.strip_prefix('<') {
        let mut depth = 1usize;
        let mut end = None;
        for (i, c) in r.char_indices() {
            match c {
                '<' => depth += 1,
                '>' => {
                    depth -= 1;
                    if depth == 0 {
                        end = Some(i);
                        break;
                    }
                }
                _ => {}
            }
        }
        let Some(e) = end else {
            return false;
        };
        rest = r[e + 1..].trim_start();
    }
    let mut tokens = rest.split(|c: char| !is_ident_char(c));
    let first = tokens.next().unwrap_or("");
    if first.is_empty() {
        return false;
    }
    if first == item {
        let after = &rest[first.len()..];
        return after.is_empty()
            || after.starts_with(|c: char| c.is_whitespace() || matches!(c, '{' | ':' | ','));
    }
    // `impl Trait for Type` — the impl'd type follows `for`.
    tokens.next() == Some("for") && tokens.next() == Some(item)
}

/// True if this Rust source line declares an item named `item`
/// (fn/struct/enum/trait/type/const/static/mod, incl. `async`/`const`/
/// `unsafe fn` and impl headers). Line-anchored on purpose: declarations in
/// this codebase start their line (after indentation/visibility), which keeps
/// the scan free of comment/string false positives.
fn line_declares_item(line: &str, item: &str) -> bool {
    let t = strip_visibility(line.trim_start());
    if keyword_declares(t, item) {
        return true;
    }
    // `async fn` / `const fn` / `unsafe fn` (any combination).
    let mut rest = t;
    loop {
        if let Some(r) = rest.strip_prefix("async ") {
            rest = r.trim_start();
            continue;
        }
        if let Some(r) = rest.strip_prefix("const ") {
            rest = r.trim_start();
            continue;
        }
        if let Some(r) = rest.strip_prefix("unsafe ") {
            rest = r.trim_start();
            continue;
        }
        break;
    }
    if keyword_declares(rest, item) {
        return true;
    }
    if let Some(r) = t.strip_prefix("impl") {
        if (r.is_empty() || r.starts_with(|c: char| c.is_whitespace() || c == '<'))
            && impl_line_names_type(r, item)
        {
            return true;
        }
    }
    false
}

fn file_declares_item(path: &Path, item: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.lines().any(|l| line_declares_item(l, item)))
        .unwrap_or(false)
}

// ---------------------------------------------------------------------------
// Check functions (each returns a list of problems; empty = clean)
// ---------------------------------------------------------------------------

/// doc→code: every reference in every doc must resolve to an existing file,
/// and (for `.rs` + `::item`) to an item declared in that file.
fn check_doc_code_refs(root: &Path, docs: &[PathBuf]) -> Vec<String> {
    let mut problems = vec![];
    for path in docs {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue; // unreadable files are caught elsewhere
        };
        for (line, span, ref_path, item) in scan_doc_refs(&content) {
            let target = root.join(&ref_path);
            if !target.is_file() {
                problems.push(format!(
                    "{}:{}: doc→code reference `{span}` does not resolve: no such file",
                    rel(root, path),
                    line
                ));
                continue;
            }
            if let Some(it) = item {
                if !ref_path.ends_with(".rs") {
                    problems.push(format!(
                        "{}:{}: doc→code reference `{span}`: `::item` anchors are only supported for `.rs` references",
                        rel(root, path),
                        line
                    ));
                } else if !file_declares_item(&target, &it) {
                    problems.push(format!(
                        "{}:{}: doc→code reference `{span}` does not resolve: item `{it}` is not declared in {ref_path}",
                        rel(root, path),
                        line
                    ));
                }
            }
        }
    }
    problems
}

/// code→doc: every manifest-listed file carries exactly one `//! Docs:` line
/// naming ≥1 defined fragment. Returns `(problems, owned_fragments)`.
fn check_code_pointers(
    root: &Path,
    manifest: &[String],
    defined: &BTreeSet<String>,
) -> (Vec<String>, BTreeSet<String>) {
    let mut problems = vec![];
    let mut owned = BTreeSet::new();
    for entry in manifest {
        let path = root.join(entry);
        let Ok(content) = std::fs::read_to_string(&path) else {
            problems.push(format!("manifest: `{entry}` is not readable"));
            continue;
        };
        let pp = parse_code_pointers(&content);
        if pp.docs_line_count == 0 {
            problems.push(format!(
                "{entry}: no `//! Docs:` pointer line (expected exactly one, e.g. `//! Docs: dev:name, llm:name`)"
            ));
        } else if pp.docs_line_count > 1 {
            problems.push(format!(
                "{entry}: {} `//! Docs:` lines found (expected exactly one)",
                pp.docs_line_count
            ));
        }
        for id in &pp.ids {
            if defined.contains(id) {
                owned.insert(id.clone());
            } else {
                problems.push(format!(
                    "{entry}: `//! Docs:` names undefined fragment `{id}`"
                ));
            }
        }
        for p in &pp.problems {
            problems.push(format!("{entry}: {p}"));
        }
    }
    (problems, owned)
}

/// Coverage: if `docs/llm/TASK_MAP.md` exists, every manifest-listed source
/// file must be referenced from it (as a doc→code reference, with or without
/// an `::item` anchor).
fn check_task_map_coverage(root: &Path, manifest: &[String]) -> Vec<String> {
    let task_map = root.join("docs/llm/TASK_MAP.md");
    if !task_map.is_file() {
        return vec![]; // not written yet (task 13.1) — the check activates on creation
    }
    let Ok(content) = std::fs::read_to_string(&task_map) else {
        return vec![];
    };
    let covered: BTreeSet<String> = scan_doc_refs(&content)
        .into_iter()
        .map(|(_, _, p, _)| p)
        .collect();
    manifest
        .iter()
        .filter(|entry| !covered.contains(*entry))
        .map(|entry| {
            format!(
                "docs/llm/TASK_MAP.md: source file `{entry}` is not covered (no doc→code reference to it)"
            )
        })
        .collect()
}

/// Parse a `<!-- llm-budget: N -->` header line.
fn parse_budget_header(line: &str) -> Option<usize> {
    let t = line.trim();
    let inner = t.strip_prefix("<!--")?.strip_suffix("-->")?;
    let rest = inner.trim().strip_prefix("llm-budget:")?;
    let n: usize = rest.trim().parse().ok()?;
    (n > 0).then_some(n)
}

/// Class B budgets: every `docs/llm/*.md` declares exactly one budget header
/// within its first 10 lines and stays within it.
fn check_llm_budgets(root: &Path, llm_docs: &[PathBuf]) -> Vec<String> {
    let mut problems = vec![];
    for path in llm_docs {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let lines: Vec<&str> = content.lines().collect();
        let total = lines.len();
        let headers: Vec<(usize, usize)> = lines
            .iter()
            .enumerate()
            .filter_map(|(i, l)| parse_budget_header(l).map(|b| (i + 1, b)))
            .collect();
        match headers.as_slice() {
            [] => problems.push(format!(
                "{}: no `<!-- llm-budget: N -->` header (required within the first 10 lines)",
                rel(root, path)
            )),
            [(ln, b)] => {
                if *ln > 10 {
                    problems.push(format!(
                        "{}: llm-budget header is on line {ln} (must be within the first 10 lines)",
                        rel(root, path)
                    ));
                }
                if total > *b {
                    problems.push(format!(
                        "{}: line count {total} exceeds declared llm-budget {b}",
                        rel(root, path)
                    ));
                }
            }
            _ => problems.push(format!(
                "{}: multiple llm-budget headers (expected exactly one)",
                rel(root, path)
            )),
        }
    }
    problems
}

/// No orphans: every defined fragment must be named by ≥1 source file's
/// `//! Docs:` line (every fragment has an owner).
fn check_no_orphans(
    root: &Path,
    defined: &BTreeMap<String, (PathBuf, usize)>,
    owned: &BTreeSet<String>,
) -> Vec<String> {
    defined
        .iter()
        .filter(|(id, _)| !owned.contains(*id))
        .map(|(id, (file, line))| {
            format!(
                "fragment `{id}` (defined at {}:{line}) is not named by any source file's `//! Docs:` line — every fragment needs an owner",
                rel(root, file)
            )
        })
        .collect()
}

/// Validate manifest entries (repo-relative .rs paths). Returns
/// `(valid_entries, problems)`.
fn validate_manifest_entries(root: &Path, raw_lines: &[&str]) -> (Vec<String>, Vec<String>) {
    let mut entries = vec![];
    let mut problems = vec![];
    for (i, line) in raw_lines.iter().enumerate() {
        let t = line.trim();
        if t.is_empty() || t.starts_with('#') {
            continue;
        }
        let line_no = i + 1;
        if !t.ends_with(".rs") {
            problems.push(format!("manifest line {line_no}: `{t}` is not a .rs path"));
            continue;
        }
        if entries.iter().any(|e| e == t) {
            problems.push(format!("manifest line {line_no}: duplicate entry `{t}`"));
            continue;
        }
        if !root.join(t).is_file() {
            problems.push(format!("manifest line {line_no}: `{t}` does not exist"));
            continue;
        }
        entries.push(t.to_string());
    }
    (entries, problems)
}

/// Read + validate the coverage manifest.
fn read_manifest(root: &Path) -> (Vec<String>, Vec<String>) {
    let path = root.join("tests/dev_docs_manifest.txt");
    if !path.is_file() {
        return (
            vec![],
            vec![
                "tests/dev_docs_manifest.txt is missing (the code→doc pointer scope driver)"
                    .to_string(),
            ],
        );
    }
    let content = std::fs::read_to_string(&path).expect("manifest must be UTF-8 readable");
    validate_manifest_entries(root, &content.lines().collect::<Vec<_>>())
}

/// All `.md` files under a directory (recursive; skips hidden dirs + target/).
fn collect_md(dir: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    fn walk(d: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(d) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "target" {
                    walk(&p, out);
                }
            } else if p.extension().is_some_and(|e| e == "md") {
                out.push(p);
            }
        }
    }
    walk(dir, &mut out);
    out.sort();
    out
}

// ---------------------------------------------------------------------------
// The gate over the real tree
// ---------------------------------------------------------------------------

#[test]
fn sync_gate_passes_on_current_tree() {
    let root = repo_root();
    let mut problems = vec![];

    // Doc set: everything under docs/dev/ + docs/llm/.
    let dev_dir = root.join("docs/dev");
    let llm_dir = root.join("docs/llm");
    for dir in [&dev_dir, &llm_dir] {
        if dir.is_dir() && collect_md(dir).is_empty() {
            problems.push(format!(
                "{} exists but contains no .md files",
                rel(&root, dir)
            ));
        }
    }
    let mut docs = collect_md(&dev_dir);
    docs.extend(collect_md(&llm_dir));

    // Manifest (code→doc pointer scope).
    let (manifest, mp) = read_manifest(&root);
    problems.extend(mp);

    // Fragment definitions: scheme, heading adjacency, uniqueness.
    let mut defined: BTreeMap<String, (PathBuf, usize)> = BTreeMap::new();
    for path in &docs {
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let (defs, dp) = collect_fragment_defs(&root, path, &content);
        problems.extend(dp);
        for d in defs {
            match defined.get(&d.id) {
                Some((f0, l0)) => problems.push(format!(
                    "{}:{}: duplicate fragment ID `{}` (first defined at {}:{l0})",
                    rel(&root, path),
                    d.line,
                    d.id,
                    rel(&root, f0)
                )),
                None => {
                    defined.insert(d.id.clone(), (d.file.clone(), d.line));
                }
            }
        }
    }

    // doc→code reference resolution.
    problems.extend(check_doc_code_refs(&root, &docs));

    // code→doc pointers + fragment ownership.
    let defined_set: BTreeSet<String> = defined.keys().cloned().collect();
    let (pp, owned) = check_code_pointers(&root, &manifest, &defined_set);
    problems.extend(pp);

    // TASK_MAP coverage (activates once the file exists).
    problems.extend(check_task_map_coverage(&root, &manifest));

    // Class B line budgets.
    let llm_docs: Vec<PathBuf> = docs
        .iter()
        .filter(|p| p.starts_with(&llm_dir))
        .cloned()
        .collect();
    problems.extend(check_llm_budgets(&root, &llm_docs));

    // No orphaned fragments.
    problems.extend(check_no_orphans(&root, &defined, &owned));

    assert!(
        problems.is_empty(),
        "sync gate found {} problem(s):\n{}",
        problems.len(),
        problems.join("\n")
    );
}

/// Completeness (task 1.10): the manifest must equal a filesystem scan of
/// every hand-written `.rs` file under `src/`, `crates/` and
/// `fuzz/fuzz_targets/`. Generated artifacts are excluded from the scan:
/// anything under a `target/` directory, plus
/// `crates/examples/fuzz_rust/fuzz/fuzz_targets/arq.rs` - emitted by that
/// crate's build.rs, git-ignored via `crates/examples/fuzz_rust/.gitignore`,
/// and absent on a fresh checkout.
#[test]
fn manifest_matches_filesystem_scan() {
    let root = repo_root();
    let (manifest, mp) = read_manifest(&root);
    assert!(mp.is_empty(), "manifest problems: {}", mp.join("; "));

    let mut scanned: BTreeSet<String> = BTreeSet::new();
    fn walk(root: &Path, d: &Path, out: &mut BTreeSet<String>) {
        let Ok(entries) = std::fs::read_dir(d) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
                if !name.starts_with('.') && name != "target" {
                    walk(root, &p, out);
                }
            } else if p.extension().is_some_and(|e| e == "rs") {
                let rel = rel(root, &p);
                if rel != "crates/examples/fuzz_rust/fuzz/fuzz_targets/arq.rs" {
                    out.insert(rel);
                }
            }
        }
    }
    for dir in ["src", "crates", "fuzz/fuzz_targets"] {
        walk(&root, &root.join(dir), &mut scanned);
    }

    let listed: BTreeSet<String> = manifest.iter().cloned().collect();
    let missing: Vec<&String> = scanned.difference(&listed).collect();
    let extra: Vec<&String> = listed.difference(&scanned).collect();
    assert!(
        missing.is_empty() && extra.is_empty(),
        "manifest ({} entries) != filesystem scan ({} files)\n\
         missing from manifest:\n{}\n\
         in manifest but not on disk:\n{}",
        manifest.len(),
        scanned.len(),
        missing
            .iter()
            .map(|s| format!("  {s}"))
            .collect::<Vec<_>>()
            .join("\n"),
        extra
            .iter()
            .map(|s| format!("  {s}"))
            .collect::<Vec<_>>()
            .join("\n")
    );
}

// ---------------------------------------------------------------------------
// Negative controls (pinned fixtures under tests/fixtures/dev_docs/)
// ---------------------------------------------------------------------------

fn fixture_dir() -> PathBuf {
    repo_root().join("tests/fixtures/dev_docs")
}

#[test]
fn positive_control_fixtures_pass_all_checks() {
    let root = repo_root();
    let dir = fixture_dir();
    let docs = [dir.join("good_doc.md"), dir.join("good_llm_doc.md")];
    for d in &docs {
        assert!(d.is_file(), "missing fixture {}", d.display());
    }
    let manifest = vec!["tests/fixtures/dev_docs/good_code.rs".to_string()];

    let mut problems = vec![];
    let mut defined: BTreeMap<String, (PathBuf, usize)> = BTreeMap::new();
    for path in &docs {
        let content = std::fs::read_to_string(path).expect("fixture must be readable");
        let (defs, dp) = collect_fragment_defs(&root, path, &content);
        problems.extend(dp);
        for d in defs {
            defined
                .entry(d.id.clone())
                .or_insert((d.file.clone(), d.line));
        }
    }
    problems.extend(check_doc_code_refs(&root, &docs));
    let defined_set: BTreeSet<String> = defined.keys().cloned().collect();
    let (pp, owned) = check_code_pointers(&root, &manifest, &defined_set);
    problems.extend(pp);
    problems.extend(check_llm_budgets(&root, &[docs[1].clone()]));
    problems.extend(check_no_orphans(&root, &defined, &owned));

    assert!(problems.is_empty(), "positive control failed: {problems:?}");
}

#[test]
fn broken_doc_code_ref_is_detected() {
    let root = repo_root();
    let path = fixture_dir().join("broken_ref.md");
    assert!(path.is_file(), "missing fixture {}", path.display());
    let problems = check_doc_code_refs(&root, &[path]);
    assert_eq!(
        problems.len(),
        2,
        "expected exactly 2 problems: {problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("no_such_file_xyz.rs") && p.contains("no such file")),
        "missing-file reference not caught: {problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("no_such_item_xyz") && p.contains("not declared")),
        "missing-item reference not caught: {problems:?}"
    );
}

#[test]
fn missing_code_pointer_is_detected() {
    let root = repo_root();
    let defined: BTreeSet<String> = ["dev:fixture-good".to_string()].into_iter().collect();
    let manifest = vec!["tests/fixtures/dev_docs/missing_pointer.rs".to_string()];
    let (problems, _owned) = check_code_pointers(&root, &manifest, &defined);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("no `//! Docs:` pointer line")),
        "missing pointer not caught: {problems:?}"
    );
}

#[test]
fn dangling_pointer_is_detected() {
    let root = repo_root();
    let defined: BTreeSet<String> = BTreeSet::new();
    let manifest = vec!["tests/fixtures/dev_docs/dangling_pointer.rs".to_string()];
    let (problems, owned) = check_code_pointers(&root, &manifest, &defined);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("undefined fragment `dev:no-such-fragment`")),
        "dangling pointer not caught: {problems:?}"
    );
    assert!(
        owned.is_empty(),
        "no fragment may be owned via a dangling pointer"
    );
}

#[test]
fn orphan_fragment_is_detected() {
    let root = repo_root();
    let path = fixture_dir().join("orphan_frag.md");
    assert!(path.is_file(), "missing fixture {}", path.display());
    let content = std::fs::read_to_string(&path).expect("fixture must be readable");
    let (defs, dp) = collect_fragment_defs(&root, &path, &content);
    assert!(
        dp.is_empty(),
        "the orphan fixture itself must be well-formed: {dp:?}"
    );
    let defined: BTreeMap<String, (PathBuf, usize)> = defs
        .iter()
        .map(|d| (d.id.clone(), (d.file.clone(), d.line)))
        .collect();
    let owned: BTreeSet<String> = BTreeSet::new();
    let problems = check_no_orphans(&root, &defined, &owned);
    assert!(
        problems.iter().any(|p| p.contains("`dev:fixture-orphan`")),
        "orphaned fragment not caught: {problems:?}"
    );
}

#[test]
fn budget_overflow_is_detected() {
    let root = repo_root();
    let path = fixture_dir().join("budget_overflow.md");
    assert!(path.is_file(), "missing fixture {}", path.display());
    let problems = check_llm_budgets(&root, &[path]);
    assert!(
        problems
            .iter()
            .any(|p| p.contains("exceeds declared llm-budget")),
        "budget overflow not caught: {problems:?}"
    );
}

#[test]
fn manifest_entry_problems_are_detected() {
    let root = repo_root();
    let (_, problems) = validate_manifest_entries(
        &root,
        &[
            "src/no_such_file_xyz.rs",
            "README.md",
            "src/lib.rs",
            "src/lib.rs",
        ],
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("no_such_file_xyz") && p.contains("does not exist")),
        "nonexistent entry not caught: {problems:?}"
    );
    assert!(
        problems.iter().any(|p| p.contains("not a .rs path")),
        "non-.rs entry not caught: {problems:?}"
    );
    assert!(
        problems
            .iter()
            .any(|p| p.contains("duplicate entry `src/lib.rs`")),
        "duplicate entry not caught: {problems:?}"
    );
}
