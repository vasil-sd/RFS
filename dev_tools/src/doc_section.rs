//! doc-section - retrieve doc content by heading path (Phase 12 task 5.4).
//!
//! The token-saving replacement for chunked sed/head/tail/dd range reads of
//! known sections (lookup-docs: 2,035 calls, 16.2% of all recorded tool
//! calls - the single largest category; 32.5% of them are range reads against
//! already-known line numbers). Given a file (or `all`) and a heading path,
//! prints the addressed section's content verbatim.
//!
//! Heading semantics mirror the two existing consumers of "what a heading is"
//! in this repo - context.rs's `extract_fragment` and sync-check's
//! anchor-followed-by-heading rule: a fence-aware line walk where a line
//! starting with ``` toggles the fence state and only ATX headings outside a
//! fence count. (pulldown-cmark 0.13.6's event API exposes no source spans -
//! recorded as a plan correction in the task status; the walk is therefore
//! line-based, which is also what keeps doc-section's section boundaries
//! byte-identical to context's fragment extraction.)
//!
//! Path resolution: element 0 matches the FIRST heading in the document whose
//! text equals it exactly (trimmed); element k+1 matches the first STRICTLY
//! DEEPER heading inside element k's section. Section content runs from the
//! line after the matched heading to the next fence-aware heading of the same
//! or a higher level - deeper headings are included - with trailing blank
//! lines stripped. An empty section prints the pinned marker `(none)` (the
//! context tool's empty-section vocabulary).

use std::path::{Path, PathBuf};

/// One ATX heading found outside a fenced code block.
#[derive(Debug, Clone)]
pub struct Heading {
    /// 1-6.
    pub level: usize,
    /// Heading text, trimmed; an optional closing `#` sequence stripped.
    pub text: String,
    /// 0-based index of the heading line.
    pub line: usize,
}

/// True when `line` is a fence delimiter (the ``` form - the only one the
/// repo's docs and gates use).
fn is_fence(line: &str) -> bool {
    line.trim_start().starts_with("```")
}

/// Parse one ATX heading line into (level, text), or None. CommonMark rules:
/// 1-6 `#` at column 0, then end-of-line or whitespace; an optional closing
/// sequence (whitespace + `#`s) at the end is not part of the text.
pub fn parse_atx_heading(line: &str) -> Option<(usize, String)> {
    let hashes = line.chars().take_while(|&c| c == '#').count();
    if !(1..=6).contains(&hashes) {
        return None;
    }
    let rest = &line[hashes..];
    if !rest.is_empty() && !rest.starts_with(|c: char| c.is_whitespace()) {
        return None; // `#nospace` is not a heading
    }
    let mut text = rest.trim();
    // Strip an optional closing sequence: a trailing run of # and whitespace
    // containing at least one # (CommonMark - it may abut the text directly).
    if let Some(pos) = text.rfind(|c: char| !matches!(c, '#' | ' ' | '\t')) {
        if text[pos + 1..].contains('#') {
            text = &text[..=pos];
        }
    } else if text.contains('#') {
        text = ""; // the whole text is a closing sequence (e.g. `## #`)
    }
    Some((hashes, text.to_string()))
}

/// Fence-aware ATX heading scan of a document.
pub fn collect_headings(content: &str) -> Vec<Heading> {
    let lines: Vec<&str> = content.lines().collect();
    let mut out = vec![];
    let mut in_fence = false;
    for (i, line) in lines.iter().enumerate() {
        if is_fence(line) {
            in_fence = !in_fence;
            continue;
        }
        if in_fence {
            continue;
        }
        if let Some((level, text)) = parse_atx_heading(line) {
            out.push(Heading {
                level,
                text,
                line: i,
            });
        }
    }
    out
}

/// The exclusive end line (0-based) of the section opened by `h`: the next
/// collected heading with level <= h.level, or `total_lines`.
pub fn section_end(headings: &[Heading], total_lines: usize, h: &Heading) -> usize {
    headings
        .iter()
        .find(|n| n.line > h.line && n.level <= h.level)
        .map(|n| n.line)
        .unwrap_or(total_lines)
}

/// Resolve a heading path against a document's headings. Element 0: first
/// exact-text match in the document. Element k+1: first exact-text match that
/// is strictly deeper than element k AND inside element k's section. Errors
/// name the missing element and (for children) the parent it was sought under.
pub fn resolve_path(
    headings: &[Heading],
    total_lines: usize,
    path: &[String],
) -> Result<Heading, String> {
    let first = path.first().ok_or("empty heading path")?;
    let mut match0 = headings
        .iter()
        .find(|h| h.text == *first)
        .ok_or_else(|| format!("no heading `{first}` in the document"))?;
    for elem in &path[1..] {
        let end = section_end(headings, total_lines, match0);
        let next = headings
            .iter()
            .find(|h| h.line > match0.line && h.line < end && h.level > match0.level && h.text == **elem)
            .ok_or_else(|| format!("no heading `{elem}` under `{}` (a deeper heading with that exact text does not occur before the parent section ends)", match0.text))?;
        match0 = next;
    }
    Ok((*match0).clone())
}

/// The section content: source lines after the heading up to `end`, leading
/// AND trailing blank lines stripped, joined with `\n`. Empty when the section
/// has no body. (The blank line that conventionally follows a heading is not
/// part of the section's content.)
pub fn section_content(content: &str, h: &Heading, end: usize) -> String {
    let lines: Vec<&str> = content.lines().collect();
    let mut body: Vec<&str> = lines.get(h.line + 1..end).unwrap_or(&[]).to_vec();
    while !body.is_empty() && body.first().unwrap().trim().is_empty() {
        body.remove(0);
    }
    while !body.is_empty() && body.last().unwrap().trim().is_empty() {
        body.pop();
    }
    body.join("\n")
}

/// The full single-file operation. `Ok(None)`: the file is readable but has no
/// section matching the path (all-mode keeps searching). `Ok(Some(out))`: the
/// rendered output - the content verbatim plus one trailing newline, or the
/// pinned `(none)` marker for an empty section.
pub fn doc_section(content: &str, path: &[String]) -> Result<Option<String>, String> {
    let headings = collect_headings(content);
    let total = content.lines().count();
    let h = resolve_path(&headings, total, path)?;
    let end = section_end(&headings, total, &h);
    let body = section_content(content, &h, end);
    Ok(Some(if body.is_empty() {
        "(none)\n".to_string()
    } else {
        format!("{body}\n")
    }))
}

/// Every `.md` file under `root/docs/`, sorted (deterministic all-mode order).
pub fn docs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
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
    walk(&root.join("docs"), &mut out);
    out.sort();
    out
}

/// Search every docs file for the heading path. Returns (repo-relative file,
/// rendered output) for each match, in sorted file order. Unreadable files are
/// skipped silently (a search, not a check).
pub fn search_all(root: &Path, path: &[String]) -> Vec<(String, String)> {
    let mut matches = vec![];
    for p in docs_files(root) {
        let Ok(content) = std::fs::read_to_string(&p) else {
            continue;
        };
        if let Ok(Some(out)) = doc_section(&content, path) {
            let rel = p
                .strip_prefix(root)
                .unwrap_or(&p)
                .to_string_lossy()
                .into_owned();
            matches.push((rel, out));
        }
    }
    matches
}

#[cfg(test)]
mod tests {
    use super::*;

    const DOC: &str = "# Alpha\n\nIntro.\n\n## Overview\n\nFirst overview.\n\n### Deep\n\nd1\nd2\n\n## Details\n\ndetails body\n\n# Second\n\n## Overview\n\nSecond overview.\n";

    #[test]
    fn atx_parsing_rules() {
        assert_eq!(parse_atx_heading("# T"), Some((1, "T".into())));
        assert_eq!(parse_atx_heading("##  T  "), Some((2, "T".into())));
        assert_eq!(parse_atx_heading("###### six"), Some((6, "six".into())));
        assert_eq!(
            parse_atx_heading("#Heading"),
            None,
            "no space is not a heading"
        );
        assert_eq!(
            parse_atx_heading("####### seven"),
            None,
            "seven hashes is not a heading"
        );
        assert_eq!(parse_atx_heading(""), None);
        // closing sequence stripped
        assert_eq!(parse_atx_heading("## T ##"), Some((2, "T".into())));
    }

    #[test]
    fn headings_collected_fence_aware() {
        let doc = "# Top\n\n```\n# not a heading\n## still not\n```\n\n## Real\n";
        let hs = collect_headings(doc);
        assert_eq!(hs.len(), 2);
        assert_eq!(hs[0].text, "Top");
        assert_eq!(hs[1].text, "Real");
    }

    fn total(doc: &str) -> usize {
        doc.lines().count()
    }

    #[test]
    fn path_resolution_first_match_and_depth_scoping() {
        let hs = collect_headings(DOC);
        // first occurrence wins
        let h = resolve_path(&hs, total(DOC), &["Overview".to_string()]).unwrap();
        assert_eq!(h.line, 4);
        // child must be deeper AND inside the parent section
        let h = resolve_path(&hs, total(DOC), &["Alpha".into(), "Deep".into()]).unwrap();
        assert_eq!(h.text, "Deep");
        // the second Overview is reachable only via its own parent
        let h = resolve_path(&hs, total(DOC), &["Second".into(), "Overview".into()]).unwrap();
        assert_eq!(h.line, 19);
        // a top-level heading is NOT a child of another top-level heading
        let err = resolve_path(&hs, total(DOC), &["Alpha".into(), "Second".into()]).unwrap_err();
        assert!(err.contains("no heading `Second` under `Alpha`"), "{err}");
        // missing elements
        assert!(resolve_path(&hs, total(DOC), &["Ghost".into()]).is_err());
        assert!(resolve_path(&hs, total(DOC), &["Alpha".into(), "Ghost".into()]).is_err());
    }

    #[test]
    fn section_content_boundaries() {
        let hs = collect_headings(DOC);
        // Deep's content stops at the same-level `## Details`.
        let h = resolve_path(&hs, total(DOC), &["Deep".to_string()]).unwrap();
        assert_eq!(
            section_content(DOC, &h, section_end(&hs, total(DOC), &h)),
            "d1\nd2"
        );
        // Alpha's content includes everything deeper until EOF-of-section...
        // here until `# Second`.
        let h = resolve_path(&hs, total(DOC), &["Alpha".to_string()]).unwrap();
        let body = section_content(DOC, &h, section_end(&hs, total(DOC), &h));
        assert!(body.starts_with("Intro."), "{body}");
        assert!(
            body.contains("## Details"),
            "deeper headings are included: {body}"
        );
        assert!(!body.contains("# Second"), "{body}");
        // an empty body (same-level heading immediately after) yields ""
        let doc = "## A\n\n## B\n";
        let hs = collect_headings(doc);
        assert_eq!(
            section_content(doc, &hs[0], section_end(&hs, total(doc), &hs[0])),
            ""
        );
    }

    #[test]
    fn empty_section_renders_the_pinned_marker() {
        // A is immediately followed by a same-level heading B -> no body.
        let doc = "## A\n\n## B\n\nbody\n";
        let out = doc_section(doc, &["A".to_string()]).unwrap().unwrap();
        assert_eq!(out, "(none)\n");
    }

    #[test]
    fn doc_section_output_is_verbatim_plus_one_newline() {
        let out = doc_section(DOC, &["Details".to_string()]).unwrap().unwrap();
        assert_eq!(out, "details body\n");
    }
}
