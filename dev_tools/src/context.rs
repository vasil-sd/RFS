//! context - one-command impact context for a fn/type (Phase 12 task 5.2).
//!
//! Operationalizes the PROTOCOL.md section-4 impact-calculation rule as a
//! single command: given a bare item name (+ optional `--file`), pull from
//! `docs/llm/SRC_STRUCTURE.md` / `docs/llm/TEST_STRUCTURE.md` the inventory
//! row itself, its Depends closure (depth-bounded, default 1), the
//! Documented-in fragments' content (extracted by fragment anchor) and the
//! related DECISIONS.md entries - rendered as a FIXED markdown template.
//! The template is part of the output contract (pinned byte-for-byte by
//! tests/context_test.rs):
//!
//! ```text
//! ## Context for <name>
//! ### Inventory row
//! ### Depends closure
//! ### Documented in
//! ### Related decisions
//! ```
//!
//! Replaces the lookup-sources grep+sed+read sequence (3,376 calls, 26.8% of
//! all recorded tool calls - the frozen analysis' second-largest category).
//! Read-only: never writes anything.

use std::path::{Path, PathBuf};

/// One inventory row from either structure document.
pub struct Row {
    /// Repo-relative path of the inventory document the row came from.
    pub doc: String,
    /// First header cell of the row's table: `Function` or `Type`.
    pub table: String,
    /// Full identifier, `<file>::<name>` (or nested/method forms).
    pub identifier: String,
    /// Parsed Depends column (empty if none).
    pub depends: Vec<String>,
    /// All six cells in column order.
    pub cells: Vec<String>,
    /// The three original lines, verbatim - the Inventory row section of the
    /// template re-emits them byte-for-byte.
    pub raw_header: String,
    pub raw_sep: String,
    pub raw_row: String,
}

/// Split a pipe-table line into cells. `\|` is an escaped literal pipe (the
/// inventories carry operator lists like `\|\|` in description cells); the
/// leading and trailing empty cells produced by the outer pipes are dropped.
pub fn split_row(line: &str) -> Vec<String> {
    let trimmed = line.trim();
    let Some(body) = trimmed.strip_prefix('|').and_then(|b| b.strip_suffix('|')) else {
        return vec![];
    };
    let mut cells = vec![];
    let mut cur = String::new();
    let mut chars = body.chars().peekable();
    while let Some(c) = chars.next() {
        match c {
            '\\' if chars.peek() == Some(&'|') => {
                cur.push('|');
                chars.next();
            }
            '|' => {
                cells.push(cur.trim().to_string());
                cur.clear();
            }
            _ => cur.push(c),
        }
    }
    cells.push(cur.trim().to_string());
    cells
}

/// Parse one inventory document's content into its rows. Tables are detected
/// by their exact header first cell (`Function` / `Type`); rows run until the
/// first non-`|` line. Other tables in the same document (Components,
/// Functionality, ...) are ignored.
pub fn parse_inventory_doc(content: &str, doc: &str) -> Vec<Row> {
    let lines: Vec<&str> = content.lines().collect();
    let mut rows = vec![];
    let mut i = 0;
    while i < lines.len() {
        let header = lines[i];
        let table = if header.starts_with("| Function |") {
            Some("Function")
        } else if header.starts_with("| Type |") {
            Some("Type")
        } else {
            None
        };
        let Some(table) = table else {
            i += 1;
            continue;
        };
        // The separator line must follow the header directly.
        if i + 1 >= lines.len() || !lines[i + 1].starts_with("|---") {
            i += 1;
            continue;
        }
        let raw_header = header.to_string();
        let raw_sep = lines[i + 1].to_string();
        i += 2;
        while i < lines.len() && lines[i].starts_with('|') {
            let cells = split_row(lines[i]);
            if cells.len() == 6 && !cells[0].is_empty() {
                rows.push(Row {
                    doc: doc.to_string(),
                    table: table.to_string(),
                    identifier: cells[0].clone(),
                    depends: cells[1]
                        .split(',')
                        .map(str::trim)
                        .filter(|s| !s.is_empty())
                        .map(str::to_string)
                        .collect(),
                    cells,
                    raw_header: raw_header.clone(),
                    raw_sep: raw_sep.clone(),
                    raw_row: lines[i].to_string(),
                });
            }
            i += 1;
        }
    }
    rows
}

/// The two structure inventories, in lookup order.
pub const INVENTORY_DOCS: &[&str] = &["docs/llm/SRC_STRUCTURE.md", "docs/llm/TEST_STRUCTURE.md"];

/// Load both inventories from the tree.
pub fn load_inventories(root: &Path) -> Vec<Row> {
    let mut rows = vec![];
    for doc in INVENTORY_DOCS {
        let path = root.join(doc);
        if let Ok(content) = std::fs::read_to_string(&path) {
            rows.extend(parse_inventory_doc(&content, doc));
        }
    }
    rows
}

/// The file part of an identifier (everything before the first `::`).
pub fn file_part(id: &str) -> &str {
    id.split("::").next().unwrap_or(id)
}

/// The last `::` segment of an identifier - the bare item name.
pub fn last_segment(id: &str) -> &str {
    id.rsplit("::").next().unwrap_or(id)
}

/// Rows whose last identifier segment equals `name` (whole-segment match -
/// `parse_specifier` does not satisfy a lookup for `parse_spec`), restricted
/// to `file` when given.
pub fn match_rows<'a>(rows: &'a [Row], name: &str, file: Option<&str>) -> Vec<&'a Row> {
    rows.iter()
        .filter(|r| last_segment(&r.identifier) == name)
        .filter(|r| file.is_none_or(|f| file_part(&r.identifier) == f))
        .collect()
}

/// One closure entry: the identifier and whether it resolved to an inventory
/// row (unresolved entries are listed but not expanded).
pub struct ClosureEntry {
    pub id: String,
    pub resolved: bool,
}

/// Depth-bounded BFS over the Depends column. Returns entries grouped by
/// depth (1 = direct dependencies). Each identifier is reported at its FIRST
/// (minimum) depth only - cycles and diamond dependencies cannot loop or
/// duplicate. Order within a depth follows discovery order (the Depends-list
/// order of the expanding rows), so the output is deterministic.
pub fn depends_closure(rows: &[Row], start: &Row, depth: usize) -> Vec<(usize, Vec<ClosureEntry>)> {
    // Identifiers are unique across both inventories (one row per item), so a
    // plain map suffices; membership in it IS the resolved check.
    let by_id: std::collections::HashMap<&str, &Row> =
        rows.iter().map(|r| (r.identifier.as_str(), r)).collect();
    let mut visited: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
    visited.insert(start.identifier.as_str());
    let mut out = vec![];
    let mut frontier: Vec<&Row> = vec![start];
    for d in 1..=depth {
        let mut entries = vec![];
        let mut next: Vec<&Row> = vec![];
        for row in &frontier {
            for dep in &row.depends {
                if visited.contains(dep.as_str()) {
                    continue;
                }
                visited.insert(dep);
                match by_id.get(dep.as_str()) {
                    Some(r) => {
                        entries.push(ClosureEntry {
                            id: dep.clone(),
                            resolved: true,
                        });
                        next.push(*r);
                    }
                    None => entries.push(ClosureEntry {
                        id: dep.clone(),
                        resolved: false,
                    }),
                }
            }
        }
        if entries.is_empty() {
            break;
        }
        out.push((d, entries));
        frontier = next;
    }
    out
}

/// Extract one fragment's content from a document: the `<a id="frag"></a>`
/// anchor (outside fenced code blocks) must be immediately followed by a
/// heading (blank lines allowed - the same rule as sync-check); the content
/// runs from that heading up to the next anchor line or the next heading of
/// the SAME OR HIGHER level (deeper headings are part of the fragment).
/// Returns the verbatim content, trailing blank lines trimmed.
pub fn extract_fragment(content: &str, frag: &str) -> Option<String> {
    let lines: Vec<&str> = content.lines().collect();
    let mut in_fence = false;
    for (i, line) in lines.iter().enumerate() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence || !line.contains("<a ") {
            continue;
        }
        let Some(id) = anchor_id(line) else {
            continue;
        };
        if id != frag {
            continue;
        }
        // The anchor must be immediately followed by a heading.
        let mut j = i + 1;
        while j < lines.len() && lines[j].trim().is_empty() {
            j += 1;
        }
        let heading = lines.get(j)?;
        if !heading.starts_with('#') {
            return None;
        }
        let level = heading.chars().take_while(|&c| c == '#').count();
        let mut end = lines.len();
        let mut fence = false;
        for (k, line) in lines.iter().enumerate().skip(j + 1) {
            if line.trim_start().starts_with("```") {
                fence = !fence;
                continue;
            }
            if fence {
                continue;
            }
            if line.contains("<a ")
                || (line.starts_with('#')
                    && line.chars().take_while(|&c| c == '#').count() <= level)
            {
                end = k;
                break;
            }
        }
        let mut body: Vec<&str> = lines[j..end].to_vec();
        while body.last().map(|l| l.trim()).unwrap_or("").is_empty() {
            body.pop();
        }
        return Some(body.join("\n"));
    }
    None
}

/// The `id` attribute of the first `<a ...>` tag on a line, if any.
fn anchor_id(line: &str) -> Option<&str> {
    let start = line.find("<a ")?;
    let rest = &line[start + 3..];
    let id_start = rest.find("id=\"")?;
    let after = &rest[id_start + 4..];
    let end = after.find('"')?;
    Some(&after[..end])
}

/// DECISIONS.md entries (their `###` headings) whose body references
/// `identifier` in a backticked span - exact match, fence-aware. Section-19
/// style bullets all sit under one `###` heading and are reported as that
/// heading. Document order, de-duplicated.
pub fn related_decisions(content: &str, identifier: &str) -> Vec<String> {
    let mut current: Option<String> = None;
    let mut found: Vec<String> = vec![];
    let mut in_fence = false;
    for line in content.lines() {
        if line.trim_start().starts_with("```") {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some(h) = line.strip_prefix("### ") {
            current = Some(h.to_string());
            continue;
        }
        let Some(entry) = &current else {
            continue;
        };
        for span in backtick_spans(line) {
            if span == identifier && !found.contains(entry) {
                found.push(entry.clone());
            }
        }
    }
    found
}

/// Single-backtick inline spans of one line (DECISIONS.md uses no fences
/// around code and no multi-line spans; a line with an odd number of
/// backticks yields no span from the dangling one).
fn backtick_spans(line: &str) -> Vec<&str> {
    let mut spans = vec![];
    let bytes = line.as_bytes();
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'`' {
            if let Some(end) = line[i + 1..].find('`') {
                spans.push(&line[i + 1..i + 1 + end]);
                i += end + 2;
                continue;
            }
        }
        i += 1;
    }
    spans
}

/// The fixed output template. See the module doc for the contract.
/// `fragments` pairs each Documented-in fragment id with its defining file
/// and extracted content (None when the fragment is not defined anywhere).
pub fn render(
    name: &str,
    row: &Row,
    closure: &[(usize, Vec<ClosureEntry>)],
    fragments: &[(String, Option<(String, String)>)],
    decisions: &[String],
    depth: usize,
) -> String {
    let mut out = String::new();
    out.push_str(&format!("## Context for {name}\n\n"));

    out.push_str("### Inventory row\n\n");
    out.push_str(&row.raw_header);
    out.push('\n');
    out.push_str(&row.raw_sep);
    out.push('\n');
    out.push_str(&row.raw_row);
    out.push_str("\n\n");

    out.push_str("### Depends closure\n\n");
    out.push_str(&format!("depth limit: {depth}\n\n"));
    if closure.is_empty() {
        out.push_str("(none)\n\n");
    } else {
        for (d, entries) in closure {
            out.push_str(&format!("- depth {d}:\n"));
            for e in entries {
                if e.resolved {
                    out.push_str(&format!("  - {}\n", e.id));
                } else {
                    out.push_str(&format!("  - {} (missing from inventory)\n", e.id));
                }
            }
        }
        out.push('\n');
    }

    out.push_str("### Documented in\n\n");
    if fragments.is_empty() {
        out.push_str("(none)\n\n");
    } else {
        for (i, (frag, found)) in fragments.iter().enumerate() {
            match found {
                Some((file, c)) => out.push_str(&format!("#### {frag} — {file}\n\n{c}\n")),
                None => out.push_str(&format!(
                    "#### {frag}\n\n(fragment not defined under docs/dev/ + docs/llm/)\n"
                )),
            }
            if i + 1 < fragments.len() {
                out.push('\n');
            }
        }
        out.push('\n');
    }

    out.push_str("### Related decisions\n\n");
    if decisions.is_empty() {
        out.push_str("(none)\n");
    } else {
        for d in decisions {
            out.push_str(&format!("- {d} (docs/dev/DECISIONS.md)\n"));
        }
    }
    out
}

/// Collect the defining file of every fragment under docs/dev/ + docs/llm/
/// (sorted walk, fence-aware anchors). Returns `(fragment_id, file)` pairs.
pub fn collect_fragment_defs(root: &Path) -> Vec<(String, String)> {
    let mut defs = vec![];
    for dir in ["docs/dev", "docs/llm"] {
        let dir_path = root.join(dir);
        let Ok(entries) = std::fs::read_dir(&dir_path) else {
            continue;
        };
        let mut files: Vec<PathBuf> = entries
            .flatten()
            .map(|e| e.path())
            .filter(|p| p.extension().and_then(|e| e.to_str()) == Some("md"))
            .collect();
        files.sort();
        for p in files {
            let Ok(content) = std::fs::read_to_string(&p) else {
                continue;
            };
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned();
            let mut in_fence = false;
            for line in content.lines() {
                if line.trim_start().starts_with("```") {
                    in_fence = !in_fence;
                    continue;
                }
                if in_fence {
                    continue;
                }
                if let Some(id) = anchor_id(line) {
                    if crate::sync::is_fragment_id(id) {
                        defs.push((id.to_string(), rel.clone()));
                    }
                }
            }
        }
    }
    defs
}

/// The full operation. `name` is a bare item name; `file` optionally
/// restricts the match to one repo-relative source path; `depth` bounds the
/// Depends closure (>= 1). Ok(rendered template) / Err(operational message,
/// exit 1).
pub fn run(root: &Path, name: &str, file: Option<&str>, depth: usize) -> Result<String, String> {
    let rows = load_inventories(root);
    let matches = match_rows(&rows, name, file);
    if matches.is_empty() {
        return Err(match file {
            Some(f) => format!("no inventory row for `{name}` in {f}"),
            None => {
                format!("no inventory row for `{name}` in SRC_STRUCTURE.md or TEST_STRUCTURE.md")
            }
        });
    }
    if matches.len() > 1 {
        let mut msg = format!(
            "ambiguous: {} inventory rows match `{name}` - use --file to disambiguate:",
            matches.len()
        );
        for m in &matches {
            msg.push_str(&format!("\n  {} ({})", m.identifier, m.doc));
        }
        return Err(msg);
    }
    let row = matches[0];
    let closure = depends_closure(&rows, row, depth);

    // Documented-in fragments: column order preserved; each fragment's
    // content is extracted from its defining file.
    let defs = collect_fragment_defs(root);
    let mut fragments: Vec<(String, Option<(String, String)>)> = vec![];
    for frag in row.cells[5]
        .split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
    {
        let found = defs.iter().find(|(id, _)| id == frag).and_then(|(_, f)| {
            std::fs::read_to_string(root.join(f))
                .ok()
                .and_then(|c| extract_fragment(&c, frag))
                .map(|c| (f.clone(), c))
        });
        fragments.push((frag.to_string(), found));
    }

    let decisions = std::fs::read_to_string(root.join("docs/dev/DECISIONS.md"))
        .ok()
        .map(|c| related_decisions(&c, &row.identifier))
        .unwrap_or_default();

    Ok(render(name, row, &closure, &fragments, &decisions, depth))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn split_row_handles_escaped_pipes() {
        assert_eq!(split_row("| a | b\\|c | d |"), vec!["a", "b|c", "d"]);
        assert_eq!(split_row("| x |"), vec!["x"]);
        assert_eq!(split_row("not a row"), Vec::<String>::new());
    }

    #[test]
    fn inventory_doc_parsing_finds_both_tables_and_ignores_others() {
        let content = "# doc\n\n## Components\n\n| Name | Depends on | Description and rationale |\n|---|---|---|\n| comp |  | a component table row |\n\n## All functions\n\n| Function | Depends | Component | Functionality | Short description | Documented in |\n|---|---|---|---|---|---|\n| src/a.rs::f | src/b.rs::g, src/c.rs::h\\|i | comp | func | desc with \\| pipe | dev:x |\n| src/a.rs::second |  | comp | func | second row |  |\n\n## All types\n\n| Type | Depends | Component | Functionality | Short description | Documented in |\n|---|---|---|---|---|---|\n| src/a.rs::T |  | comp | func | a type | dev:y |\n";
        let rows = parse_inventory_doc(content, "docs/llm/SRC_STRUCTURE.md");
        assert_eq!(rows.len(), 3);
        assert_eq!(rows[0].identifier, "src/a.rs::f");
        assert_eq!(rows[0].table, "Function");
        assert_eq!(rows[0].depends, vec!["src/b.rs::g", "src/c.rs::h|i"]);
        assert_eq!(rows[1].identifier, "src/a.rs::second");
        assert!(rows[1].depends.is_empty());
        assert_eq!(rows[2].table, "Type");
        assert_eq!(rows[2].identifier, "src/a.rs::T");
        // the Components table row must NOT be parsed
        assert!(!rows.iter().any(|r| r.identifier == "comp"));
    }

    #[test]
    fn identifier_segments() {
        assert_eq!(
            file_part("src/lang/parser.rs::parse_source"),
            "src/lang/parser.rs"
        );
        assert_eq!(
            last_segment("src/lang/parser.rs::parse_source"),
            "parse_source"
        );
        assert_eq!(file_part("src/a.rs::tests::helper"), "src/a.rs");
        assert_eq!(last_segment("src/a.rs::tests::helper"), "helper");
        assert_eq!(last_segment("src/a.rs::Ir::new"), "new");
    }

    fn row(id: &str, depends: &[&str]) -> Row {
        Row {
            doc: "docs/llm/SRC_STRUCTURE.md".into(),
            table: "Function".into(),
            identifier: id.into(),
            depends: depends.iter().map(|s| s.to_string()).collect(),
            cells: vec![
                id.into(),
                depends.join(", "),
                "c".into(),
                "f".into(),
                "d".into(),
                String::new(),
            ],
            raw_header: String::new(),
            raw_sep: String::new(),
            raw_row: String::new(),
        }
    }

    #[test]
    fn matching_is_whole_segment_and_file_restricted() {
        let rows = vec![
            row("src/a.rs::parse_spec", &[]),
            row("src/b.rs::parse_specifier", &[]),
            row("src/a.rs::tests::parse_spec", &[]),
        ];
        assert_eq!(match_rows(&rows, "parse_spec", None).len(), 2);
        assert_eq!(
            match_rows(&rows, "parse_spec", Some("src/a.rs")).len(),
            2,
            "file part ignores the tests:: segment"
        );
        assert_eq!(match_rows(&rows, "parse_spec", Some("src/b.rs")).len(), 0);
        assert_eq!(match_rows(&rows, "nope", None).len(), 0);
    }

    #[test]
    fn closure_depths_missing_and_cycles() {
        let rows = vec![
            row("a::top", &["a::mid", "a::ghost"]),
            row("a::mid", &["b::leaf", "a::top"]), // cycle back to top
            row("b::leaf", &[]),
        ];
        let start = &rows[0];
        let c1 = depends_closure(&rows, start, 1);
        assert_eq!(c1.len(), 1);
        assert_eq!(c1[0].0, 1);
        assert_eq!(
            c1[0].1.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            vec!["a::mid", "a::ghost"]
        );
        assert!(c1[0].1[0].resolved);
        assert!(!c1[0].1[1].resolved, "ghost is missing from the inventory");

        let c2 = depends_closure(&rows, start, 2);
        assert_eq!(c2.len(), 2);
        // leaf appears at depth 2 exactly once; the cycle to top never re-appears.
        assert_eq!(
            c2[1].1.iter().map(|e| e.id.clone()).collect::<Vec<_>>(),
            vec!["b::leaf"]
        );

        let c3 = depends_closure(&rows, start, 3);
        assert_eq!(c3.len(), 2, "no depth-3 entries exist - the walk stops");

        // a row with no dependencies at all yields an empty closure.
        assert!(depends_closure(&rows, &rows[2], 5).is_empty());
    }

    #[test]
    fn fragment_boundaries() {
        let content = "# doc\n\nprose before.\n\n<a id=\"dev:a\"></a>\n## A\n\nA line one.\n\n### Deeper stays inside\n\nInner line.\n\n<a id=\"dev:b\"></a>\n## B\n\nB line.\n\n## C ends b\n\nC line.\n";
        let a = extract_fragment(content, "dev:a").unwrap();
        assert!(a.starts_with("## A"));
        assert!(
            a.contains("### Deeper stays inside"),
            "deeper headings are part of the fragment"
        );
        assert!(
            a.ends_with("Inner line."),
            "the next anchor ends the fragment; got: {a}"
        );
        let b = extract_fragment(content, "dev:b").unwrap();
        assert_eq!(
            b, "## B\n\nB line.",
            "a same-level heading ends the fragment; got: {b}"
        );
        assert!(
            extract_fragment(content, "dev:c").is_none(),
            "no anchor for C"
        );
    }

    #[test]
    fn fragment_anchor_in_fence_is_ignored() {
        let content = "```text\n<a id=\"dev:x\"></a>\n## X\n\nin fence\n```\n\n<a id=\"dev:y\"></a>\n## Y\n\nreal.\n";
        assert!(extract_fragment(content, "dev:x").is_none());
        assert_eq!(extract_fragment(content, "dev:y").unwrap(), "## Y\n\nreal.");
    }

    #[test]
    fn decisions_scan_continuation_fences_and_dedup() {
        let content = "# D\n\n### D1. First\n\nText. Code: `src/a.rs::top`,\n`src/b.rs::leaf`.\n\nMore text naming `src/a.rs::top` again.\n\n### D2. Second\n\nFenced span must not count:\n\n```text\n`src/a.rs::top`\n```\n\nProse only.\n";
        assert_eq!(
            related_decisions(content, "src/a.rs::top"),
            vec!["D1. First"]
        );
        assert_eq!(
            related_decisions(content, "src/b.rs::leaf"),
            vec!["D1. First"]
        );
        assert!(related_decisions(content, "src/c.rs::none").is_empty());
    }

    #[test]
    fn render_pins_the_template_shape() {
        let r = row("src/a.rs::top", &["src/b.rs::leaf"]);
        let closure = vec![(
            1,
            vec![ClosureEntry {
                id: "src/b.rs::leaf".into(),
                resolved: true,
            }],
        )];
        let frags = vec![(
            "dev:a".to_string(),
            Some(("docs/dev/a.md".to_string(), "## A\n\ncontent.".to_string())),
        )];
        let out = render("top", &r, &closure, &frags, &[], 1);
        for heading in [
            "## Context for top",
            "### Inventory row",
            "### Depends closure",
            "depth limit: 1",
            "- depth 1:",
            "  - src/b.rs::leaf",
            "### Documented in",
            "#### dev:a — docs/dev/a.md",
            "### Related decisions",
            "(none)",
        ] {
            assert!(
                out.lines().any(|l| l == heading),
                "missing `{heading}` in:\n{out}"
            );
        }
    }
}
