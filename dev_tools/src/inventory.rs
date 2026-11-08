//! inventory - structure-inventory row operations + the string-masked
//! completeness audit (Phase 12 task 5.5).
//!
//! The two structure inventories (`docs/llm/SRC_STRUCTURE.md`,
//! `docs/llm/TEST_STRUCTURE.md`) are living, complete-by-construction tables
//! of every function and data type. This module keeps them mechanically valid
//! (row add/update/remove) and re-runs the Phase 10/11 string-masked
//! completeness audit: extract every fn/type identifier from the source with
//! syn (string literals are never identifiers, so masking is structural),
//! diff against the inventory rows per file, exit 1 on any diff.
//!
//! Extraction conventions (ported from the Phase 10/11 close-out method, see
//! PHASE_11_TASK_11_STATUS.md findings):
//! - top-level fn / struct / enum / trait / type-alias -> `<file>::<name>`;
//! - inherent-impl methods -> `<file>::<Type>::<method>`;
//! - trait-impl methods -> `<file>::<Type>(<Trait>)::<method>` (the MARKED
//!   form), demarked to `<file>::<Type>::<method>` unless the document itself
//!   carries that exact marked row (marked-set derived from the doc);
//! - trait-definition methods -> `<file>::<Trait>::<method>`;
//! - items nested inside a fn body carry the parent-fn prefix, EXCEPT where
//!   the document writes them unprefixed (per-file irregularity - the matcher
//!   uses `::`-suffix matching so both conventions agree);
//! - associated-type bindings inside impls are phantoms, not inventory types;
//! - a `fuzz_target!(...)` libfuzzer macro invocation counts as a fn named
//!   `fuzz_target` (the Phase 10/11 "macro entries");
//! - host files split: non-`#[cfg(test)]` items go to SRC, `#[cfg(test)]`
//!   items to TEST; dedicated test/fuzz files are whole-file in TEST.
//!
//! Audit scope is derived from the inventories themselves: a file present in
//! both is a host file (SRC = non-cfg, TEST = cfg-only); only in TEST is a
//! whole-file test; only in SRC is production source. A completeness net then
//! flags any in-region source file with items that appears in NEITHER
//! inventory - excluding crate-local fuzz harnesses (`crates/**/fuzz/**`),
//! which the inventories deliberately do not cover.

use std::collections::{BTreeMap, BTreeSet};
use std::path::{Path, PathBuf};

/// One extracted item: kind (`fn`/`type`), identifier (file-relative path
/// after the `<file>::` prefix), and whether it sits in a `#[cfg(test)]` mod.
#[derive(Debug, Clone)]
pub struct Item {
    pub kind: &'static str,
    pub id: String,
    pub in_cfg: bool,
}

fn cfg_test(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        if let syn::Meta::List(ml) = &a.meta {
            ml.path.is_ident("cfg") && ml.tokens.to_string().contains("test")
        } else {
            false
        }
    })
}

fn self_type_name(ty: &syn::Type) -> Option<String> {
    match ty {
        syn::Type::Path(p) => p.path.segments.last().map(|s| s.ident.to_string()),
        _ => None,
    }
}

fn trait_name(path: &syn::Path) -> Option<String> {
    path.segments.last().map(|s| s.ident.to_string())
}

/// Recurse into a fn body collecting nested items (fns, types, and impl
/// blocks on those nested types). Closures are not items.
fn collect_body(block: &syn::Block, prefix: &str, in_cfg: bool, out: &mut Vec<Item>) {
    for stmt in &block.stmts {
        if let syn::Stmt::Item(it) = stmt {
            match it {
                syn::Item::Fn(f) => {
                    let id = format!("{}::{}", prefix, f.sig.ident);
                    collect_body(&f.block, &id, in_cfg, out);
                    out.push(Item {
                        kind: "fn",
                        id,
                        in_cfg,
                    });
                }
                syn::Item::Struct(s) => {
                    out.push(Item {
                        kind: "type",
                        id: format!("{}::{}", prefix, s.ident),
                        in_cfg,
                    });
                }
                syn::Item::Enum(e) => {
                    out.push(Item {
                        kind: "type",
                        id: format!("{}::{}", prefix, e.ident),
                        in_cfg,
                    });
                }
                syn::Item::Impl(i) => {
                    if let Some(tn) = self_type_name(&i.self_ty) {
                        let base = format!("{}::{}", prefix, tn);
                        for ii in &i.items {
                            if let syn::ImplItem::Fn(m) = ii {
                                let id = match &i.trait_ {
                                    Some((_, tp, _)) => match trait_name(tp) {
                                        Some(tr) => format!("{}({})::{}", base, tr, m.sig.ident),
                                        None => format!("{}::{}", base, m.sig.ident),
                                    },
                                    None => format!("{}::{}", base, m.sig.ident),
                                };
                                out.push(Item {
                                    kind: "fn",
                                    id,
                                    in_cfg,
                                });
                            }
                        }
                    }
                }
                _ => {}
            }
        }
    }
}

/// Extract every inventory item from one parsed file.
fn extract_items(file: &syn::File) -> Vec<Item> {
    let mut out = vec![];
    fn collect(items: &[syn::Item], prefix: &str, in_cfg: bool, out: &mut Vec<Item>) {
        for item in items {
            match item {
                syn::Item::Fn(f) => {
                    let id = if prefix.is_empty() {
                        f.sig.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, f.sig.ident)
                    };
                    collect_body(&f.block, &id, in_cfg, out);
                    out.push(Item {
                        kind: "fn",
                        id,
                        in_cfg,
                    });
                }
                syn::Item::Struct(s) => out.push(Item {
                    kind: "type",
                    id: if prefix.is_empty() {
                        s.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, s.ident)
                    },
                    in_cfg,
                }),
                syn::Item::Enum(e) => out.push(Item {
                    kind: "type",
                    id: if prefix.is_empty() {
                        e.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, e.ident)
                    },
                    in_cfg,
                }),
                syn::Item::Union(u) => out.push(Item {
                    kind: "type",
                    id: if prefix.is_empty() {
                        u.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, u.ident)
                    },
                    in_cfg,
                }),
                syn::Item::Type(t) => out.push(Item {
                    kind: "type",
                    id: if prefix.is_empty() {
                        t.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, t.ident)
                    },
                    in_cfg,
                }),
                syn::Item::Trait(t) => {
                    let id = if prefix.is_empty() {
                        t.ident.to_string()
                    } else {
                        format!("{}::{}", prefix, t.ident)
                    };
                    for ti in &t.items {
                        if let syn::TraitItem::Fn(m) = ti {
                            out.push(Item {
                                kind: "fn",
                                id: format!("{}::{}", id, m.sig.ident),
                                in_cfg,
                            });
                        }
                    }
                    out.push(Item {
                        kind: "type",
                        id,
                        in_cfg,
                    });
                }
                syn::Item::Impl(i) => {
                    let tn = self_type_name(&i.self_ty);
                    if let (Some(tn), None) = (&tn, &i.trait_) {
                        for ii in &i.items {
                            if let syn::ImplItem::Fn(m) = ii {
                                out.push(Item {
                                    kind: "fn",
                                    id: format!("{}::{}", tn, m.sig.ident),
                                    in_cfg,
                                });
                            }
                        }
                    } else if let (Some(tn), Some((_, tp, _))) = (&tn, &i.trait_) {
                        if let Some(tr) = trait_name(tp) {
                            for ii in &i.items {
                                if let syn::ImplItem::Fn(m) = ii {
                                    out.push(Item {
                                        kind: "fn",
                                        id: format!("{}({})::{}", tn, tr, m.sig.ident),
                                        in_cfg,
                                    });
                                }
                            }
                        }
                    }
                }
                syn::Item::Mod(m) => {
                    if let Some((_, inner)) = &m.content {
                        let np = if prefix.is_empty() {
                            m.ident.to_string()
                        } else {
                            format!("{}::{}", prefix, m.ident)
                        };
                        collect(inner, &np, in_cfg || cfg_test(&m.attrs), out);
                    }
                }
                syn::Item::Macro(mac) if mac.mac.path.is_ident("fuzz_target") => {
                    // The libfuzzer `fuzz_target!(...)` macro expands to a fn
                    // named `fuzz_target` - the inventory counts it as a function.
                    let id = if prefix.is_empty() {
                        "fuzz_target".to_string()
                    } else {
                        format!("{}::fuzz_target", prefix)
                    };
                    out.push(Item {
                        kind: "fn",
                        id,
                        in_cfg,
                    });
                }
                _ => {}
            }
        }
    }
    collect(&file.items, "", false, &mut out);
    out
}

/// Extract every inventory item from a source file on disk. Parse failure is
/// an error (the audit must not silently skip a file).
pub fn extract_file(path: &Path) -> Result<Vec<Item>, String> {
    let text = std::fs::read_to_string(path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let file = syn::parse_str::<syn::File>(&text)
        .map_err(|e| format!("{}: parse error: {}", path.display(), e))?;
    Ok(extract_items(&file))
}

/// Split a pipe-table line into cells, honoring `\|` escapes (the inventories
/// carry operator lists like `\|\|` in description cells). Leading/trailing
/// empty cells from the outer pipes are dropped.
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

/// Parse an inventory document into per-file, per-kind identifier sets. Tables
/// are detected by their exact header first cell (`Function` / `Type`); a table
/// runs until a heading or prose line. Blank lines WITHIN a table are tolerated
/// (TEST_STRUCTURE.md has them). In-table rows whose identifier is not in the
/// file-path-plus-name form are skipped.
pub fn parse_inventory(path: &Path) -> BTreeMap<(String, &'static str), BTreeSet<String>> {
    let mut rows: BTreeMap<(String, &'static str), BTreeSet<String>> = BTreeMap::new();
    let Ok(content) = std::fs::read_to_string(path) else {
        return rows;
    };
    let mut table: Option<&'static str> = None;
    for line in content.lines() {
        if line.starts_with("| Function |") {
            table = Some("fn");
            continue;
        }
        if line.starts_with("| Type |") {
            table = Some("type");
            continue;
        }
        if !line.starts_with('|') {
            if !line.trim().is_empty() {
                table = None; // heading or prose ends the table
            }
            continue;
        }
        let Some(kind) = table else { continue };
        let cells = split_row(line);
        let Some(ident) = cells.first() else { continue };
        let Some((file, rest)) = ident.split_once("::") else {
            continue;
        };
        if !file.ends_with(".rs") {
            continue;
        }
        rows.entry((file.to_string(), kind))
            .or_default()
            .insert(rest.to_string());
    }
    rows
}

/// The marked-set: every `(file, identifier)` the document carries in the
/// `Type(Trait)::method` form. A trait-impl method keeps its marked form only
/// when it is here; otherwise it is demarked (the `(Trait)` group stripped).
pub fn marked_set(
    invs: &[&BTreeMap<(String, &'static str), BTreeSet<String>>],
) -> BTreeSet<(String, String)> {
    let mut marked = BTreeSet::new();
    for inv in invs {
        for ((file, _kind), ids) in inv.iter() {
            for id in ids {
                if id.contains('(') && id.contains(")::") {
                    marked.insert((file.clone(), id.clone()));
                }
            }
        }
    }
    marked
}

/// Demark a set of identifiers: strip the `(Trait)` group from any marked-form
/// identifier the document does not carry marked.
fn demark(
    file: &str,
    items: &BTreeSet<String>,
    marked: &BTreeSet<(String, String)>,
) -> BTreeSet<String> {
    let mut out = BTreeSet::new();
    for id in items {
        if id.contains('(')
            && id.contains(")::")
            && !marked.contains(&(file.to_string(), id.clone()))
        {
            // Remove the first `(Trait)` group, keeping `Type::method`.
            if let Some(open) = id.find('(') {
                if let Some(close) = id[open..].find(')') {
                    let stripped = format!("{}{}", &id[..open], &id[open + close + 1..]);
                    out.insert(stripped);
                    continue;
                }
            }
        }
        out.insert(id.clone());
    }
    out
}

/// Two identifiers match if equal, or one is a `::`-suffix of the other (the
/// per-file irregular nested-naming convention: some files write nested items
/// with the parent-fn prefix, some without).
fn matches(a: &str, b: &str) -> bool {
    a == b || a.ends_with(&format!("::{b}")) || b.ends_with(&format!("::{a}"))
}

/// One file's audit result.
#[derive(Debug)]
pub struct FileDiff {
    pub file: String,
    pub inventory: &'static str, // "SRC" or "TEST"
    pub kind: &'static str,      // "fn" or "type"
    pub missing: Vec<String>,    // in source, not inventoried
    pub stale: Vec<String>,      // inventoried, not in source
}

/// A file is a crate-local fuzz harness (`crates/**/fuzz/**`) - deliberately
/// outside both inventories' coverage.
fn is_crate_fuzz_harness(file: &str) -> bool {
    let parts: Vec<&str> = file.split('/').collect();
    parts.first() == Some(&"crates") && parts[1..].contains(&"fuzz")
}

/// The full audit over both inventories. Returns (file-diffs, src_fn_count,
/// src_type_count, test_fn_count, test_type_count). Empty diffs = EXACT.
pub fn audit(root: &Path) -> Result<(Vec<FileDiff>, usize, usize, usize, usize), String> {
    let src_inv = parse_inventory(&root.join("docs/llm/SRC_STRUCTURE.md"));
    let test_inv = parse_inventory(&root.join("docs/llm/TEST_STRUCTURE.md"));
    let marked = marked_set(&[&src_inv, &test_inv]);

    // Extract every in-region source file.
    let mut ext_nocfg: BTreeMap<(String, &'static str), BTreeSet<String>> = BTreeMap::new();
    let mut ext_cfg: BTreeMap<(String, &'static str), BTreeSet<String>> = BTreeMap::new();
    let mut files = vec![];
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let name = p
                    .file_name()
                    .unwrap_or_default()
                    .to_string_lossy()
                    .into_owned();
                if name == "target" || name == ".git" {
                    continue;
                }
                walk(&p, out);
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    for top in ["src", "crates", "tests", "fuzz"] {
        walk(&root.join(top), &mut files);
    }
    for path in &files {
        let rel = path
            .strip_prefix(root)
            .unwrap_or(path)
            .to_string_lossy()
            .into_owned();
        let items = extract_file(path)?;
        for it in items {
            let map = if it.in_cfg {
                &mut ext_cfg
            } else {
                &mut ext_nocfg
            };
            map.entry((rel.clone(), it.kind)).or_default().insert(it.id);
        }
    }

    let src_files: BTreeSet<&str> = src_inv.keys().map(|(f, _)| f.as_str()).collect();
    let test_files: BTreeSet<&str> = test_inv.keys().map(|(f, _)| f.as_str()).collect();
    let covered: BTreeSet<&str> = src_files.union(&test_files).copied().collect();

    let mut diffs = vec![];
    for f in &covered {
        if src_files.contains(f) {
            for kind in ["fn", "type"] {
                let ex = demark(
                    f,
                    &ext_nocfg
                        .get(&(f.to_string(), kind))
                        .cloned()
                        .unwrap_or_default(),
                    &marked,
                );
                let d = src_inv
                    .get(&(f.to_string(), kind))
                    .cloned()
                    .unwrap_or_default();
                let missing: Vec<String> = ex
                    .iter()
                    .filter(|e| !d.iter().any(|dd| matches(e, dd)))
                    .cloned()
                    .collect();
                let stale: Vec<String> = d
                    .iter()
                    .filter(|dd| !ex.iter().any(|e| matches(dd, e)))
                    .cloned()
                    .collect();
                if !missing.is_empty() || !stale.is_empty() {
                    diffs.push(FileDiff {
                        file: f.to_string(),
                        inventory: "SRC",
                        kind,
                        missing,
                        stale,
                    });
                }
            }
        }
        if test_files.contains(f) {
            let whole = !src_files.contains(f);
            for kind in ["fn", "type"] {
                let raw = if whole {
                    let mut s = ext_nocfg
                        .get(&(f.to_string(), kind))
                        .cloned()
                        .unwrap_or_default();
                    s.extend(
                        ext_cfg
                            .get(&(f.to_string(), kind))
                            .cloned()
                            .unwrap_or_default(),
                    );
                    s
                } else {
                    ext_cfg
                        .get(&(f.to_string(), kind))
                        .cloned()
                        .unwrap_or_default()
                };
                let ex = demark(f, &raw, &marked);
                let d = test_inv
                    .get(&(f.to_string(), kind))
                    .cloned()
                    .unwrap_or_default();
                let missing: Vec<String> = ex
                    .iter()
                    .filter(|e| !d.iter().any(|dd| matches(e, dd)))
                    .cloned()
                    .collect();
                let stale: Vec<String> = d
                    .iter()
                    .filter(|dd| !ex.iter().any(|e| matches(dd, e)))
                    .cloned()
                    .collect();
                if !missing.is_empty() || !stale.is_empty() {
                    diffs.push(FileDiff {
                        file: f.to_string(),
                        inventory: "TEST",
                        kind,
                        missing,
                        stale,
                    });
                }
            }
        }
    }

    // Completeness net: in-region files with items that appear in NEITHER
    // inventory (crate-local fuzz harnesses excluded).
    let all_files: BTreeSet<String> = ext_nocfg
        .keys()
        .chain(ext_cfg.keys())
        .map(|(f, _)| f.clone())
        .collect();
    for f in &all_files {
        if covered.contains(f.as_str()) || is_crate_fuzz_harness(f) {
            continue;
        }
        let items = demark(
            f,
            &ext_nocfg
                .get(&(f.clone(), "fn"))
                .cloned()
                .unwrap_or_default(),
            &marked,
        )
        .union(&demark(
            f,
            &ext_nocfg
                .get(&(f.clone(), "type"))
                .cloned()
                .unwrap_or_default(),
            &marked,
        ))
        .chain(
            demark(
                f,
                &ext_cfg.get(&(f.clone(), "fn")).cloned().unwrap_or_default(),
                &marked,
            )
            .iter(),
        )
        .chain(
            demark(
                f,
                &ext_cfg
                    .get(&(f.clone(), "type"))
                    .cloned()
                    .unwrap_or_default(),
                &marked,
            )
            .iter(),
        )
        .cloned()
        .collect::<BTreeSet<_>>();
        if !items.is_empty() {
            diffs.push(FileDiff {
                file: f.clone(),
                inventory: "NET",
                kind: "fn",
                missing: items.into_iter().collect(),
                stale: vec![],
            });
        }
    }

    let count =
        |inv: &BTreeMap<(String, &'static str), BTreeSet<String>>, k: &'static str| -> usize {
            inv.iter()
                .filter(|((_, kind), _)| *kind == k)
                .map(|(_, s)| s.len())
                .sum()
        };
    let (src_fn, src_ty, test_fn, test_ty) = (
        count(&src_inv, "fn"),
        count(&src_inv, "type"),
        count(&test_inv, "fn"),
        count(&test_inv, "type"),
    );
    diffs.sort_by(|a, b| {
        a.file
            .cmp(&b.file)
            .then(a.inventory.cmp(b.inventory))
            .then(a.kind.cmp(b.kind))
    });
    Ok((diffs, src_fn, src_ty, test_fn, test_ty))
}

/// Escape a literal pipe for use inside a table cell (`|` -> `\|`). The
/// inventories carry operator lists like `\|\|` in description cells.
pub fn escape_cell(s: &str) -> String {
    s.replace('|', "\\|")
}

/// One inventory row (six columns, column order fixed by the documents).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Row {
    pub identifier: String,
    pub depends: String,
    pub component: String,
    pub functionality: String,
    pub description: String,
    pub documented: String,
}

impl Row {
    /// Render the row as a single physical line with escaped cells.
    pub fn render(&self) -> String {
        format!(
            "| {} | {} | {} | {} | {} | {} |",
            escape_cell(&self.identifier),
            escape_cell(&self.depends),
            escape_cell(&self.component),
            escape_cell(&self.functionality),
            escape_cell(&self.description),
            escape_cell(&self.documented)
        )
    }

    /// Parse a physical row line into a Row. `None` if it does not have exactly
    /// six cells (a header, separator, or malformed line).
    pub fn from_line(line: &str) -> Option<Row> {
        let c = split_row(line);
        if c.len() != 6 {
            return None;
        }
        Some(Row {
            identifier: c[0].clone(),
            depends: c[1].clone(),
            component: c[2].clone(),
            functionality: c[3].clone(),
            description: c[4].clone(),
            documented: c[5].clone(),
        })
    }

    /// The file part of the identifier (`<file>.rs`), if well-formed.
    pub fn file(&self) -> Option<&str> {
        self.identifier
            .split_once("::")
            .map(|(f, _)| f)
            .filter(|f| f.ends_with(".rs"))
    }
}

/// Locate a table in the document lines. Returns (header_line_idx, first_row_idx,
/// last_row_idx_inclusive) for the table whose header first cell is `want`
/// ("Function" or "Type"). Blank lines within the table are tolerated.
fn find_table(lines: &[String], want: &str) -> Option<(usize, usize, usize)> {
    let header = lines.iter().position(|l| {
        let c = split_row(l);
        c.first().map(|s| s.as_str()) == Some(want) && c.len() == 6
    })?;
    // The separator line follows the header.
    let mut i = header + 1;
    if i < lines.len()
        && lines[i].starts_with("|")
        && split_row(&lines[i])
            .iter()
            .all(|c| c.chars().all(|ch| ch == '-' || ch == ' '))
    {
        i += 1;
    }
    let first = i;
    let mut last = i - 1;
    while i < lines.len() {
        if lines[i].starts_with('|') {
            last = i;
            i += 1;
        } else if lines[i].trim().is_empty() {
            // tolerate in-table blank lines: peek ahead for the next row
            let mut j = i + 1;
            while j < lines.len() && lines[j].trim().is_empty() {
                j += 1;
            }
            if j < lines.len() && lines[j].starts_with('|') {
                last = j - 1; // keep the blank as part of the table span
                i = j;
            } else {
                break;
            }
        } else {
            break;
        }
    }
    if last < first {
        return None; // header with no data rows
    }
    Some((header, first, last))
}

/// The result of a row operation: the plan line(s) and whether the document
/// text changed.
pub struct RowOp {
    pub plan: Vec<String>,
    pub new_text: String,
    pub changed: bool,
}

fn doc_path(root: &Path, doc: &str) -> PathBuf {
    match doc {
        "SRC" => root.join("docs/llm/SRC_STRUCTURE.md"),
        _ => root.join("docs/llm/TEST_STRUCTURE.md"),
    }
}

/// `inventory add`: insert a new row, keeping file grouping (after the last row
/// of the same file, else at the end of the table). Error if the identifier is
/// already present.
pub fn add_row(
    root: &Path,
    doc: &str,
    kind: &str,
    row: &Row,
    dry_run: bool,
) -> Result<RowOp, String> {
    let path = doc_path(root, doc);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let want = if kind == "fn" { "Function" } else { "Type" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let Some((_, first, last)) = find_table(&lines, want) else {
        return Err(format!("no `{want}` table found in {}", path.display()));
    };
    // Duplicate check across the whole table.
    for line in &lines[first..=last] {
        if let Some(existing) = Row::from_line(line) {
            if existing.identifier == row.identifier {
                return Err(format!(
                    "{}: row already present: {}",
                    path.display(),
                    row.identifier
                ));
            }
        }
    }
    // Insertion point: after the last row of the same file, else end of table.
    let file = row.file().unwrap_or("");
    let mut insert_at = last + 1;
    for i in (first..=last).rev() {
        if let Some(existing) = Row::from_line(&lines[i]) {
            if existing.file() == Some(file) {
                insert_at = i + 1;
                break;
            }
        }
    }
    let rendered = row.render();
    lines.insert(insert_at, rendered.clone());
    let new_text = lines.join("\n") + "\n";
    let plan = vec![format!("add {} row: {}", want, row.identifier)];
    if !dry_run {
        std::fs::write(&path, &new_text)
            .map_err(|e| format!("writing {}: {}", path.display(), e))?;
    }
    Ok(RowOp {
        plan,
        new_text,
        changed: true,
    })
}

/// `inventory update`: replace one cell of an existing row. Error if the row is
/// absent. `column` is one of depends/component/functionality/description/
/// documented (the identifier column is immutable - renaming is a remove+add).
pub fn update_row(
    root: &Path,
    doc: &str,
    kind: &str,
    identifier: &str,
    column: &str,
    value: &str,
    dry_run: bool,
) -> Result<RowOp, String> {
    let path = doc_path(root, doc);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let want = if kind == "fn" { "Function" } else { "Type" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let Some((_, first, last)) = find_table(&lines, want) else {
        return Err(format!("no `{want}` table found in {}", path.display()));
    };
    let mut found = false;
    for (offset, line) in lines[first..=last].iter().enumerate() {
        if let Some(mut row) = Row::from_line(line) {
            if row.identifier == identifier {
                match column {
                    "depends" => row.depends = value.to_string(),
                    "component" => row.component = value.to_string(),
                    "functionality" => row.functionality = value.to_string(),
                    "description" => row.description = value.to_string(),
                    "documented" => row.documented = value.to_string(),
                    other => return Err(format!("unknown column `{other}` (use depends|component|functionality|description|documented)")),
                }
                lines[first + offset] = row.render();
                found = true;
                break;
            }
        }
    }
    if !found {
        return Err(format!(
            "{}: no row with identifier `{identifier}` in the {want} table",
            path.display()
        ));
    }
    let new_text = lines.join("\n") + "\n";
    let plan = vec![format!(
        "update {} row `{identifier}`: {} = {}",
        want, column, value
    )];
    if !dry_run {
        std::fs::write(&path, &new_text)
            .map_err(|e| format!("writing {}: {}", path.display(), e))?;
    }
    Ok(RowOp {
        plan,
        new_text,
        changed: true,
    })
}

/// `inventory remove`: delete the row with the given identifier. Error if absent.
pub fn remove_row(
    root: &Path,
    doc: &str,
    kind: &str,
    identifier: &str,
    dry_run: bool,
) -> Result<RowOp, String> {
    let path = doc_path(root, doc);
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{}: {}", path.display(), e))?;
    let want = if kind == "fn" { "Function" } else { "Type" };
    let mut lines: Vec<String> = text.lines().map(str::to_string).collect();
    let Some((_, first, last)) = find_table(&lines, want) else {
        return Err(format!("no `{want}` table found in {}", path.display()));
    };
    let mut found_at = None;
    for (offset, line) in lines[first..=last].iter().enumerate() {
        if let Some(row) = Row::from_line(line) {
            if row.identifier == identifier {
                found_at = Some(first + offset);
                break;
            }
        }
    }
    let Some(at) = found_at else {
        return Err(format!(
            "{}: no row with identifier `{identifier}` in the {want} table",
            path.display()
        ));
    };
    lines.remove(at);
    let new_text = lines.join("\n") + "\n";
    let plan = vec![format!("remove {} row: {}", want, identifier)];
    if !dry_run {
        std::fs::write(&path, &new_text)
            .map_err(|e| format!("writing {}: {}", path.display(), e))?;
    }
    Ok(RowOp {
        plan,
        new_text,
        changed: true,
    })
}

/// Parse a full identifier (`<file>.rs::<rest>`) into (file, rest). Used by the
/// CLI to build rows.
pub fn split_identifier(identifier: &str) -> Option<(String, String)> {
    let (f, r) = identifier.split_once("::")?;
    if !f.ends_with(".rs") {
        return None;
    }
    Some((f.to_string(), r.to_string()))
}

/// Build a Row from its parts (CLI helper). The identifier must be a full
/// `<file>.rs::<rest>` form.
pub fn build_row(
    identifier: &str,
    depends: &str,
    component: &str,
    functionality: &str,
    description: &str,
    documented: &str,
) -> Result<Row, String> {
    if split_identifier(identifier).is_none() {
        return Err(format!(
            "`{identifier}` is not a full `<file>.rs::<name>` identifier"
        ));
    }
    Ok(Row {
        identifier: identifier.to_string(),
        depends: depends.to_string(),
        component: component.to_string(),
        functionality: functionality.to_string(),
        description: description.to_string(),
        documented: documented.to_string(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn extract(src: &str) -> Vec<Item> {
        let file = syn::parse_str::<syn::File>(src).unwrap();
        extract_items(&file)
    }

    fn ids(items: &[Item], kind: &str, cfg: bool) -> BTreeSet<String> {
        items
            .iter()
            .filter(|i| i.kind == kind && i.in_cfg == cfg)
            .map(|i| i.id.clone())
            .collect()
    }

    #[test]
    fn extraction_top_level_and_impls() {
        let src = r#"
            pub fn top(x: u32) -> u32 { x }
            struct Point;
            enum Color { R, G }
            type Alias = u32;
            trait Tr { fn m(&self); }
            impl Point { fn make() -> Self { Point } }
            impl Tr for Point { fn m(&self) {} }
        "#;
        let items = extract(src);
        // the trait-impl method is emitted in MARKED form (Point(Tr)::m)
        assert_eq!(
            ids(&items, "fn", false),
            ["top", "Point::make", "Tr::m", "Point(Tr)::m"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        );
        assert_eq!(
            ids(&items, "type", false),
            ["Point", "Color", "Alias", "Tr"]
                .iter()
                .map(|s| s.to_string())
                .collect()
        );
    }

    #[test]
    fn extraction_nested_and_macro_and_cfg() {
        let src = r#"
            pub fn outer() {
                fn inner() {}
                struct Local;
                impl Drop for Local { fn drop(&mut self) {} }
            }
            fuzz_target!(|d: &[u8]| {});
            #[cfg(test)]
            mod tests { fn tcase() {} }
        "#;
        let items = extract(src);
        // nested items carry the parent-fn prefix
        assert!(ids(&items, "fn", false).contains("outer::inner"));
        assert!(ids(&items, "type", false).contains("outer::Local"));
        // trait-impl-in-body is marked, demarkable to outer::Local::drop
        assert!(items.iter().any(|i| i.id == "outer::Local(Drop)::drop"));
        // the libfuzzer macro counts as a fn named fuzz_target
        assert!(ids(&items, "fn", false).contains("fuzz_target"));
        // cfg(test) items are separated
        assert!(ids(&items, "fn", true).contains("tests::tcase"));
        assert!(!ids(&items, "fn", false).contains("tests::tcase"));
    }

    #[test]
    fn associated_type_bindings_are_phantoms() {
        let src = r#"
            trait Proto { type Out; fn run(&self) -> Self::Out; }
            struct Engine;
            impl Proto for Engine { type Out = u32; fn run(&self) -> u32 { 0 } }
        "#;
        let items = extract(src);
        // the associated type `Out` in the impl is NOT an inventory type
        assert!(!ids(&items, "type", false).contains("Engine::Out"));
        // but the trait's own associated type and method are inventoried
        assert!(ids(&items, "type", false).contains("Proto"));
        assert!(ids(&items, "fn", false).contains("Proto::run"));
        assert!(items.iter().any(|i| i.id == "Engine(Proto)::run"));
    }

    #[test]
    fn demark_strips_unmarked_trait_group() {
        let marked: BTreeSet<(String, String)> = BTreeSet::new();
        let items: BTreeSet<String> = ["Opt(Default)::default", "Kept(IrJson)::to_json"]
            .iter()
            .map(|s| s.to_string())
            .collect();
        let out = demark("src/a.rs", &items, &marked);
        // neither is marked in the doc, so both are demarked
        assert!(out.contains("Opt::default"));
        assert!(out.contains("Kept::to_json"));
        assert!(!out.contains("Opt(Default)::default"));
    }

    #[test]
    fn row_render_and_parse_roundtrip_with_escaped_pipes() {
        let row = Row {
            identifier: "src/a.rs::f".into(),
            depends: "src/b.rs::g".into(),
            component: "comp".into(),
            functionality: "a || b operator".into(), // carries a literal pipe
            description: "does x | y".into(),
            documented: "dev:x".into(),
        };
        let line = row.render();
        assert!(
            line.contains("a \\|\\| b operator"),
            "pipe must be escaped: {line}"
        );
        let back = Row::from_line(&line).unwrap();
        assert_eq!(back, row);
    }

    #[test]
    fn split_row_handles_escaped_pipes() {
        assert_eq!(split_row("| a | b\\|c | d |"), vec!["a", "b|c", "d"]);
        assert_eq!(split_row("not a row"), Vec::<String>::new());
    }

    // --- row operations on a temp fixture document -------------------------

    static COUNTER: std::sync::atomic::AtomicUsize = std::sync::atomic::AtomicUsize::new(0);

    fn temp_doc() -> (std::path::PathBuf, std::path::PathBuf) {
        // Returns (root, doc_path). The doc has a Function table with two rows
        // in one file and a Type table with one row. A per-call counter keeps
        // parallel tests from sharing (and racing on) one directory.
        let n = COUNTER.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
        let root = std::env::temp_dir().join(format!("rfs-inv-test-{}-{}", std::process::id(), n));
        let _ = std::fs::remove_dir_all(&root);
        std::fs::create_dir_all(root.join("docs/llm")).unwrap();
        let doc = root.join("docs/llm/SRC_STRUCTURE.md");
        std::fs::write(
            &doc,
            "# inv\n\n## All functions\n\n| Function | Depends | Component | Functionality | Short description | Documented in |\n|---|---|---|---|---|---|\n| src/a.rs::one |  | c | f | d1 | dev:x |\n| src/a.rs::two | src/a.rs::one | c | f | d2 |  |\n| src/b.rs::three |  | c | f | d3 |  |\n\n## All types\n\n| Type | Depends | Component | Functionality | Short description | Documented in |\n|---|---|---|---|---|---|\n| src/a.rs::Alpha |  | c | f | a type |  |\n",
        )
        .unwrap();
        (root, doc)
    }

    #[test]
    fn add_inserts_after_same_file_and_rejects_duplicates() {
        let (root, _doc) = temp_doc();
        // add a new fn to src/a.rs - must land after `two`, before `three`.
        let row = build_row("src/a.rs::four", "", "c", "f", "d4", "").unwrap();
        let op = add_row(&root, "SRC", "fn", &row, false).unwrap();
        assert!(op.changed);
        let text = std::fs::read_to_string(root.join("docs/llm/SRC_STRUCTURE.md")).unwrap();
        let lines: Vec<&str> = text.lines().collect();
        let idx = |name: &str| {
            lines
                .iter()
                .position(|l| l.contains(&format!("::{name} ")))
                .unwrap()
        };
        assert!(
            idx("two") < idx("four") && idx("four") < idx("three"),
            "{lines:?}"
        );
        // duplicate is rejected
        let dup = build_row("src/a.rs::four", "", "c", "f", "d4", "").unwrap();
        assert!(add_row(&root, "SRC", "fn", &dup, false).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn update_replaces_a_cell_and_remove_deletes_the_row() {
        let (root, _doc) = temp_doc();
        let op = update_row(
            &root,
            "SRC",
            "fn",
            "src/a.rs::one",
            "description",
            "NEW DESC",
            false,
        )
        .unwrap();
        assert!(op.changed);
        let text = std::fs::read_to_string(root.join("docs/llm/SRC_STRUCTURE.md")).unwrap();
        assert!(text.contains("| NEW DESC |"), "{text}");
        // update a missing row errors
        assert!(update_row(
            &root,
            "SRC",
            "fn",
            "src/a.rs::ghost",
            "description",
            "x",
            false
        )
        .is_err());
        // remove the row
        let op = remove_row(&root, "SRC", "fn", "src/a.rs::two", false).unwrap();
        assert!(op.changed);
        let text = std::fs::read_to_string(root.join("docs/llm/SRC_STRUCTURE.md")).unwrap();
        assert!(!text.contains("::two "), "{text}");
        // remove a missing row errors
        assert!(remove_row(&root, "SRC", "fn", "src/a.rs::ghost", false).is_err());
        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn dry_run_writes_nothing() {
        let (root, doc) = temp_doc();
        let before = std::fs::read_to_string(&doc).unwrap();
        let row = build_row("src/a.rs::five", "", "c", "f", "d5", "").unwrap();
        let op = add_row(&root, "SRC", "fn", &row, true).unwrap();
        assert!(op.changed, "the plan reports a change");
        let after = std::fs::read_to_string(&doc).unwrap();
        assert_eq!(before, after, "dry-run must not write");
        let _ = std::fs::remove_dir_all(&root);
    }
}
