//! sync-check / sync-fix - docs<->source reference re-syncing (Phase 12
//! task 5.1).
//!
//! Productizes the Phase 11 task-10 operation: after any refactor, every
//! backticked source-path reference in the LIVING doc set and every `//!
//! Docs:` pointer line in manifest-listed `.rs` files must still resolve.
//! The check walks three directions and reports a compact fix-list (one
//! problem per line, exit 1 if any, 0 if clean):
//!
//! - code->doc: each manifest entry exists, carries exactly one `//! Docs:`
//!   line, and every fragment it names is defined under docs/dev/ + docs/llm/;
//! - doc->code: every inline-code reference (repo-relative `.rs`/`.c`/`.h`
//!   path, optional `::item`) in docs/dev/** + docs/llm/** resolves to an
//!   existing file, and `.rs::item` references name an item declared in that
//!   file. The scope is EXACTLY the gate's (tests/dev_docs_test.rs scans only
//!   these two directories): docs/roadmap/** is append-only history that
//!   deliberately names pre-refactor paths; docs/user/** and
//!   docs/SEMANTICS.md are prose. The two structure inventories are excluded
//!   from reference resolution - their backticked paths are identifier-format
//!   examples and rename-rule prose, audited by `inventory audit` instead;
//! - orphans: fragments defined in the living doc set but owned by no
//!   manifest-listed file. Anchor collection skips fenced code blocks (the
//!   README's anchor-format example lives in a fence) and requires each
//!   anchor to be immediately followed by a heading, as the gate does.
//!
//! sync-fix applies PATH MAPPINGS (`--map OLD=NEW`, repeatable) to stale
//! doc->code references - exactly the Phase 11 task-10 scripted mapping -
//! plan-first, with `--dry-run`. Without mappings it prints the fix-list and
//! changes nothing. Reference grammar and item-declaration semantics mirror
//! `tests/dev_docs_test.rs` (the committed sync gate) so the tool and the
//! gate cannot drift apart in what counts as a reference.

use std::path::{Path, PathBuf};

/// doc->code reference: repo-relative source path (>=1 `/`, `.rs`/`.c`/`.h`),
/// optionally followed by `::item`. Returns `(path, item)`.
pub fn parse_code_ref(span: &str) -> Option<(String, Option<String>)> {
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

/// Fragment id: `(dev|llm):` + lowercase-dash segments.
pub fn is_fragment_id(s: &str) -> bool {
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

/// Parse a `//! Docs:` pointer line into its fragment list.
pub fn parse_pointer_line(line: &str) -> Option<Vec<String>> {
    let rest = line.strip_prefix("//! Docs:")?;
    let frags: Vec<String> = rest
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(str::to_string)
        .collect();
    if frags.is_empty() {
        None
    } else {
        Some(frags)
    }
}

fn is_ident_char(c: char) -> bool {
    c == '_' || c.is_ascii_alphanumeric()
}

/// True if `s` begins with `item` as a WHOLE identifier: every char of `item`
/// must match, and the following char (if any) must not be an ident char -
/// so `parse_specifier` does not satisfy a lookup for `parse_spec`.
fn starts_with_ident(s: &str, item: &str) -> bool {
    let mut chars = s.chars();
    for expected in item.chars() {
        if chars.next() != Some(expected) {
            return false;
        }
    }
    match chars.next() {
        None => true,
        Some(c) => !is_ident_char(c),
    }
}

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
        return s;
    }
    if r.is_empty() || r.starts_with(|c: char| c.is_whitespace()) {
        return r.trim_start();
    }
    s
}

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
        // Mirrors dev_docs_test exactly: a generic tail (`Spec<T>`) does NOT
        // count - local types are caught by their declaration line instead, and
        // this keeps tool and gate from drifting apart.
        return after.is_empty()
            || after.starts_with(|c: char| c.is_whitespace() || matches!(c, '{' | ':' | ','));
    }
    // `impl Trait for Type` - the impl'd type follows `for` (token-based, as
    // in dev_docs_test; `first` already consumed the first token).
    tokens.next() == Some("for") && tokens.next() == Some(item)
}

pub fn line_declares_item(line: &str, item: &str) -> bool {
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

pub fn file_declares_item(path: &Path, item: &str) -> bool {
    std::fs::read_to_string(path)
        .map(|c| c.lines().any(|l| line_declares_item(l, item)))
        .unwrap_or(false)
}

/// The two structure inventories: backticked paths there are identifier-format
/// examples and rename-rule prose (some deliberately name OLD paths), so they
/// are audited by `inventory audit`, not reference-resolved here.
pub const INVENTORY_DOCS: &[&str] = &["docs/llm/SRC_STRUCTURE.md", "docs/llm/TEST_STRUCTURE.md"];

fn read_manifest(root: &Path) -> Vec<String> {
    let path = root.join("tests/dev_docs_manifest.txt");
    std::fs::read_to_string(path)
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .filter(|l| !l.is_empty() && !l.starts_with('#'))
        .map(str::to_string)
        .collect()
}

/// Collect defined fragments: `(fragment_id, defining_file)` pairs from
/// `<a id="..."></a>` anchors under docs/dev/ + docs/llm/.
fn collect_fragment_defs(root: &Path) -> (Vec<(String, String)>, Vec<String>) {
    let mut defs = vec![];
    let mut problems = vec![];
    for dir in ["docs/dev", "docs/llm"] {
        let dir_path = root.join(dir);
        let Ok(entries) = std::fs::read_dir(&dir_path) else {
            continue;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.extension().and_then(|e| e.to_str()) != Some("md") {
                continue;
            }
            let Ok(content) = std::fs::read_to_string(&p) else {
                continue;
            };
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned();
            // Fence tracking mirrors the gate: an anchor inside a fenced code
            // block is an example of the format, not a definition (the README
            // shows `<a id="dev:name"></a>` in a fence).
            let lines: Vec<&str> = content.lines().collect();
            let mut in_fence = false;
            for (i, line) in lines.iter().enumerate() {
                if line.trim_start().starts_with("```") {
                    in_fence = !in_fence;
                    continue;
                }
                if in_fence {
                    continue;
                }
                // anchor form: <a id="dev:name"></a> (possibly with other attrs)
                if let Some(start) = line.find("<a ") {
                    let rest = &line[start + 3..];
                    if let Some(id_start) = rest.find("id=\"") {
                        let after = &rest[id_start + 4..];
                        if let Some(end) = after.find('\"') {
                            let id = &after[..end];
                            if is_fragment_id(id) {
                                // The anchor must be immediately followed by a
                                // heading (blank lines allowed) - gate rule.
                                let mut j = i + 1;
                                while j < lines.len() && lines[j].trim().is_empty() {
                                    j += 1;
                                }
                                if !(j < lines.len() && lines[j].starts_with('#')) {
                                    problems.push(format!(
                                        "{rel}:{}: fragment anchor `{id}` must be immediately followed by a heading",
                                        i + 1
                                    ));
                                }
                                defs.push((id.to_string(), rel.clone()));
                            }
                        }
                    }
                }
            }
        }
    }
    (defs, problems)
}

/// Inline code spans of a markdown document (fenced/indented blocks are NOT
/// spans in pulldown-cmark's event stream, so reference-syntax examples inside
/// fences in the docs themselves are safe).
pub fn inline_code_spans(content: &str) -> Vec<String> {
    let mut spans = vec![];
    for event in pulldown_cmark::Parser::new(content) {
        if let pulldown_cmark::Event::Code(text) = event {
            spans.push(text.to_string());
        }
    }
    spans
}

/// The full check. Returns the fix-list: one problem per line, empty = clean.
pub fn check(root: &Path) -> Vec<String> {
    let mut problems = vec![];
    let manifest = read_manifest(root);
    let (defs, def_problems) = collect_fragment_defs(root);
    problems.extend(def_problems);
    let defined: std::collections::BTreeSet<&str> =
        defs.iter().map(|(id, _)| id.as_str()).collect();

    // code->doc: every manifest entry carries exactly one valid pointer line.
    let mut owned = std::collections::BTreeSet::new();
    for entry in &manifest {
        let path = root.join(entry);
        let Ok(content) = std::fs::read_to_string(&path) else {
            problems.push(format!(
                "{entry}: manifest lists the file but it does not exist"
            ));
            continue;
        };
        let pointer_lines: Vec<&str> = content
            .lines()
            .filter(|l| l.starts_with("//! Docs:"))
            .collect();
        match pointer_lines.len() {
            0 => problems.push(format!("{entry}: no `//! Docs:` pointer line")),
            n if n > 1 => problems.push(format!(
                "{entry}: {n} `//! Docs:` pointer lines (exactly one required)"
            )),
            _ => {
                let Some(frags) = parse_pointer_line(pointer_lines[0]) else {
                    problems.push(format!("{entry}: `//! Docs:` line names no fragments"));
                    continue;
                };
                for f in frags {
                    if !is_fragment_id(&f) {
                        problems.push(format!("{entry}: pointer fragment `{f}` is not a valid id (want dev:name or llm:name)"));
                    } else if !defined.contains(f.as_str()) {
                        problems.push(format!("{entry}: pointer fragment `{f}` is not defined under docs/dev/ + docs/llm/"));
                    } else {
                        owned.insert(f);
                    }
                }
            }
        }
    }

    // orphans: defined but owned by no manifest-listed file.
    for (id, file) in &defs {
        if !owned.contains(id) {
            problems.push(format!(
                "{file}: fragment `{id}` is defined but owned by no manifest-listed file (orphan)"
            ));
        }
    }

    // doc->code: every reference in the LIVING doc set resolves - exactly
    // docs/dev/** + docs/llm/** (the gate's scope; roadmap history, user docs
    // and SEMANTICS.md are deliberately out of scope).
    let mut md_files = vec![];
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(p);
            }
        }
    }
    for dir in ["docs/dev", "docs/llm"] {
        walk(&root.join(dir), &mut md_files);
    }
    md_files.sort();
    for path in &md_files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        if INVENTORY_DOCS.contains(&rel.as_str()) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        for span in inline_code_spans(&content) {
            let Some((ref_path, item)) = parse_code_ref(&span) else {
                continue;
            };
            let target = root.join(&ref_path);
            if !target.is_file() {
                problems.push(format!(
                    "{rel}: doc->code reference `{span}` does not resolve: no such file"
                ));
                continue;
            }
            if let Some(it) = item {
                if !ref_path.ends_with(".rs") {
                    problems.push(format!("{rel}: doc->code reference `{span}`: `::item` anchors are only supported for `.rs` references"));
                } else if !file_declares_item(&target, &it) {
                    problems.push(format!("{rel}: doc->code reference `{span}` does not resolve: item `{it}` is not declared in {ref_path}"));
                }
            }
        }
    }

    problems.sort();
    problems
}

/// Apply path mappings to stale doc->code references. Returns (plan_lines,
/// changed_files). With `dry_run` the plan is computed but nothing is written.
pub fn fix(root: &Path, maps: &[(String, String)], dry_run: bool) -> (Vec<String>, Vec<String>) {
    let mut plan = vec![];
    let mut changed = vec![];
    if maps.is_empty() {
        return (plan, changed);
    }
    // Same living-doc-set scope as check(): never rewrite roadmap history.
    let mut md_files = vec![];
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                walk(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("md") {
                out.push(p);
            }
        }
    }
    for dir in ["docs/dev", "docs/llm"] {
        walk(&root.join(dir), &mut md_files);
    }
    md_files.sort();
    for path in &md_files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        if INVENTORY_DOCS.contains(&rel.as_str()) {
            continue;
        }
        let Ok(content) = std::fs::read_to_string(path) else {
            continue;
        };
        let mut new_content = content.clone();
        for (old, new) in maps {
            // Rewrite the path part of every backticked reference whose path
            // equals `old`, preserving any `::item` suffix.
            let pat = format!("`{old}");
            if !new_content.contains(&pat) {
                continue;
            }
            for span in inline_code_spans(&content) {
                if let Some((ref_path, item)) = parse_code_ref(&span) {
                    if ref_path == *old {
                        // The replacement keeps the span a well-formed inline
                        // code span - the closing backtick is part of it.
                        let rewritten = match item {
                            Some(it) => format!("`{new}::{it}`"),
                            None => format!("`{new}`"),
                        };
                        plan.push(format!("{rel}: `{span}` -> {rewritten}"));
                        new_content = new_content.replace(&format!("`{span}"), &rewritten);
                    }
                }
            }
        }
        if new_content != content && !dry_run {
            std::fs::write(path, new_content).unwrap_or_else(|e| panic!("writing {rel}: {e}"));
            changed.push(rel);
        } else if new_content != content && dry_run {
            changed.push(format!("{rel} (dry-run)"));
        }
    }
    (plan, changed)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn code_ref_grammar() {
        assert_eq!(
            parse_code_ref("src/lang/parser.rs"),
            Some(("src/lang/parser.rs".into(), None))
        );
        assert_eq!(
            parse_code_ref("src/lang/parser.rs::parse_spec"),
            Some(("src/lang/parser.rs".into(), Some("parse_spec".into())))
        );
        assert_eq!(
            parse_code_ref("crates/examples/sync_handler/src/lib.rs::run"),
            Some((
                "crates/examples/sync_handler/src/lib.rs".into(),
                Some("run".into())
            ))
        );
        // not references:
        assert_eq!(parse_code_ref("parser.rs"), None, "no slash");
        assert_eq!(
            parse_code_ref("src/lang/parser.py"),
            None,
            "wrong extension"
        );
        assert_eq!(parse_code_ref("src/../etc/passwd.rs"), None, "dotdot");
        assert_eq!(parse_code_ref("src/a//b.rs"), None, "empty segment");
        // an invalid item suffix degrades to a file-only reference - exactly
        // as dev_docs_test::parse_code_ref does (mirrored semantics).
        assert_eq!(
            parse_code_ref("src/a.rs::9bad"),
            Some(("src/a.rs".into(), None)),
            "invalid item degrades to file-only"
        );
        assert_eq!(
            parse_code_ref("`src/a.rs`"),
            None,
            "backticks are not part of the span text"
        );
    }

    #[test]
    fn fragment_id_validation() {
        assert!(is_fragment_id("dev:mc-engine-overview"));
        assert!(is_fragment_id("llm:parser"));
        assert!(!is_fragment_id("doc:parser"), "unknown class");
        assert!(!is_fragment_id("dev:"), "empty name");
        assert!(!is_fragment_id("dev:Bad-Name"), "uppercase");
        assert!(!is_fragment_id("dev:a--b"), "empty segment");
    }

    #[test]
    fn pointer_line_parsing() {
        let frags = parse_pointer_line("//! Docs: dev:a, llm:b, dev:c").unwrap();
        assert_eq!(frags, vec!["dev:a", "llm:b", "dev:c"]);
        assert!(parse_pointer_line("//! Docs:").is_none(), "no fragments");
        assert!(parse_pointer_line("//! other").is_none(), "wrong prefix");
    }

    #[test]
    fn item_declaration_detection() {
        assert!(line_declares_item(
            "pub fn parse_spec(input: &str) -> Result<Spec, ParseError> {",
            "parse_spec"
        ));
        assert!(line_declares_item("async fn step(&mut self) {", "step"));
        assert!(line_declares_item("const fn width() -> u32 {", "width"));
        assert!(line_declares_item("unsafe fn raw(ptr: *mut u8) {", "raw"));
        assert!(line_declares_item("pub struct Spec {", "Spec"));
        assert!(line_declares_item("enum State {", "State"));
        assert!(line_declares_item("trait ProtocolFsm {", "ProtocolFsm"));
        assert!(line_declares_item("type Alias = u32;", "Alias"));
        assert!(line_declares_item("const LIMIT: u32 = 4;", "LIMIT"));
        assert!(line_declares_item(
            "static mut COUNTER: u32 = 0;",
            "COUNTER"
        ));
        assert!(line_declares_item("mod parser {", "parser"));
        assert!(line_declares_item("impl Spec {", "Spec"));
        // gate-mirrored semantics: a generic tail does not count (local types
        // resolve via their declaration line); `for`-named types do.
        assert!(
            !line_declares_item("impl<T: Clone> Spec<T> {", "Spec"),
            "generic tail is not an impl-name per the gate"
        );
        assert!(line_declares_item(
            "impl ProtocolFsm for ChannelFsm {",
            "ChannelFsm"
        ));
        assert!(
            !line_declares_item("fn parse_specifier() {", "parse_spec"),
            "identifier must match whole"
        );
        assert!(
            !line_declares_item("let fn = 3;", "fn"),
            "not a declaration"
        );
    }
}
