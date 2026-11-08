//! edit-rename / edit-move / edit-replace-item - syn-parsed source edits
//! (Phase 12 task 5.3).
//!
//! Productizes the pattern-fragile rewrite calls (bulk-rewrite 229 python +
//! shell-edit 32 terminal = 261, plus the import-site rewrites that follow
//! module moves): every edit is a SYNTAX operation over syn-parsed files, so
//! string literals and comments containing the old pattern are UNTOUCHED by
//! construction (the phantom negative control - Phase 10's template-phantom
//! failure class). Edits are surgical: only the byte ranges of the rewritten
//! tokens change; all other bytes (formatting, whitespace, strings, comments)
//! survive identically.
//!
//! The three operations share one rewrite core (span collection + back-to-
//! front text replacement):
//!
//! - `edit-rename <old> <new>`: renames a fn/type/const/static/mod/trait/
//!   enum/union/type-alias (top-level OR associated) across every `.rs` file
//!   under `--root`. Declaration sites are counted first: zero -> "not
//!   found", more than one -> "ambiguous" (both exit 1). Rewritten tokens:
//!   the declaration ident, every path whose LAST segment is the name
//!   (qualified calls, type positions, `Self::x`, `super::x`), and use-tree
//!   names (`use a::b::old;`, `use a::{old};`, `use a::old as alias;` - the
//!   alias itself is never touched).
//! - `edit-move <from> <to>`: moves one module file and rewrites every import
//!   site. The old module path (segments after the nearest `src/`) is looked
//!   up as a contiguous segment sequence inside every path and use-tree
//!   chain; the matched range is replaced by the new module path. Mod
//!   declarations are moved with the file: removed from the old parent's
//!   declaration file, added to the new parent's (or a fresh `mod.rs` is
//!   created). Same-parent moves rewrite the stem in place.
//! - `edit-replace-item <file> <item> --new-from FILE`: replaces one item by
//!   name (top-level item ident, or an impl block named by its self type)
//!   with the content of FILE, spanning leading attributes through the
//!   closing brace/semicolon. Zero matches -> "not found", more than one ->
//!   "ambiguous" (exit 1).
//!
//! All three are plan-first: the exact edit plan prints to stdout before
//! anything is written, and `--dry-run` stops after the plan. Exit codes:
//! 0 success, 1 operational failure (not found / ambiguous / parse error /
//! bad move), 2 usage error (main.rs).

use std::ops::Range;
use std::path::{Path, PathBuf};

/// Every `.rs` file under `root`, skipping `target/` and `.git/` (build
/// artifacts and VCS internals never carry project symbols), sorted for
/// deterministic plans.
pub fn scan_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = vec![];
    fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let p = entry.path();
            if p.is_dir() {
                let skip = matches!(
                    p.file_name().and_then(|n| n.to_str()),
                    Some("target") | Some(".git")
                );
                if !skip {
                    walk(&p, out);
                }
            } else if p.extension().and_then(|e| e.to_str()) == Some("rs") {
                out.push(p);
            }
        }
    }
    walk(root, &mut out);
    out.sort();
    out
}

/// Replace the byte ranges in `text` with `replacement`, back-to-front so
/// earlier offsets stay valid. Duplicate and overlapping ranges are collapsed
/// (the same token can be reached by two visitor paths). Panics on a range
/// outside the text - a span/length mismatch is a tool bug, not user input.
pub fn replace_spans(text: &str, spans: &[Range<usize>], replacement: &str) -> String {
    let mut ranges: Vec<Range<usize>> = spans.to_vec();
    ranges.sort_by_key(|r| (r.start, r.end));
    ranges.dedup();
    assert!(
        ranges.iter().all(|r| r.end <= text.len()),
        "span {ranges:?} out of bounds for a {}-byte file",
        text.len()
    );
    let mut out = text.to_string();
    for r in ranges.iter().rev() {
        out.replace_range(r.clone(), replacement);
    }
    out
}

/// 1-based line number of a byte offset.
pub fn line_of(text: &str, byte: usize) -> usize {
    text[..byte.min(text.len())].matches('\n').count() + 1
}

fn rel(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

/// A declaration site of an item name (feeds the found/ambiguous check).
pub struct DeclSite {
    /// Repo-relative file.
    pub file: String,
    /// `fn` / `struct` / ... / `method` / `associated const` / ...
    pub kind: &'static str,
    /// 1-based line of the ident.
    pub line: usize,
}

/// Collects declaration sites and rewrite spans for one name in one file.
struct Finder<'a> {
    old: &'a str,
    decls: &'a mut Vec<(usize, &'static str)>,
    spans: &'a mut Vec<Range<usize>>,
}

impl<'ast> syn::visit::Visit<'ast> for Finder<'_> {
    // --- declaration sites (top-level items) -----------------------------
    fn visit_item_fn(&mut self, i: &syn::ItemFn) {
        if i.sig.ident == *self.old {
            let r = i.sig.ident.span().byte_range();
            self.decls.push((r.start, "fn"));
            self.spans.push(r);
        }
        syn::visit::visit_item_fn(self, i);
    }
    fn visit_item_struct(&mut self, i: &syn::ItemStruct) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "struct"));
            self.spans.push(r);
        }
        syn::visit::visit_item_struct(self, i);
    }
    fn visit_item_enum(&mut self, i: &syn::ItemEnum) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "enum"));
            self.spans.push(r);
        }
        syn::visit::visit_item_enum(self, i);
    }
    fn visit_item_trait(&mut self, i: &syn::ItemTrait) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "trait"));
            self.spans.push(r);
        }
        syn::visit::visit_item_trait(self, i);
    }
    fn visit_item_type(&mut self, i: &syn::ItemType) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "type alias"));
            self.spans.push(r);
        }
        syn::visit::visit_item_type(self, i);
    }
    fn visit_item_const(&mut self, i: &syn::ItemConst) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "const"));
            self.spans.push(r);
        }
        syn::visit::visit_item_const(self, i);
    }
    fn visit_item_static(&mut self, i: &syn::ItemStatic) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "static"));
            self.spans.push(r);
        }
        syn::visit::visit_item_static(self, i);
    }
    fn visit_item_mod(&mut self, i: &syn::ItemMod) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "mod"));
            self.spans.push(r);
        }
        syn::visit::visit_item_mod(self, i);
    }
    fn visit_item_union(&mut self, i: &syn::ItemUnion) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "union"));
            self.spans.push(r);
        }
        syn::visit::visit_item_union(self, i);
    }

    // --- declaration sites (associated items) ----------------------------
    fn visit_impl_item_fn(&mut self, i: &syn::ImplItemFn) {
        if i.sig.ident == *self.old {
            let r = i.sig.ident.span().byte_range();
            self.decls.push((r.start, "method"));
            self.spans.push(r);
        }
        syn::visit::visit_impl_item_fn(self, i);
    }
    fn visit_impl_item_const(&mut self, i: &syn::ImplItemConst) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "associated const"));
            self.spans.push(r);
        }
        syn::visit::visit_impl_item_const(self, i);
    }
    fn visit_impl_item_type(&mut self, i: &syn::ImplItemType) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "associated type"));
            self.spans.push(r);
        }
        syn::visit::visit_impl_item_type(self, i);
    }
    fn visit_trait_item_fn(&mut self, i: &syn::TraitItemFn) {
        if i.sig.ident == *self.old {
            let r = i.sig.ident.span().byte_range();
            self.decls.push((r.start, "trait method"));
            self.spans.push(r);
        }
        syn::visit::visit_trait_item_fn(self, i);
    }
    fn visit_trait_item_const(&mut self, i: &syn::TraitItemConst) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "trait const"));
            self.spans.push(r);
        }
        syn::visit::visit_trait_item_const(self, i);
    }
    fn visit_trait_item_type(&mut self, i: &syn::TraitItemType) {
        if i.ident == *self.old {
            let r = i.ident.span().byte_range();
            self.decls.push((r.start, "trait associated type"));
            self.spans.push(r);
        }
        syn::visit::visit_trait_item_type(self, i);
    }

    // --- reference sites ---------------------------------------------------
    /// Every path whose LAST segment is the name: qualified calls
    /// (`crate::a::old()`), type positions, `Self::old`, `super::old`, bare
    /// calls after a use-import. String literals and comments are not paths -
    /// they can never match (the phantom guarantee).
    fn visit_path(&mut self, p: &syn::Path) {
        if let Some(seg) = p.segments.last() {
            if seg.ident == *self.old {
                self.spans.push(seg.ident.span().byte_range());
            }
        }
        syn::visit::visit_path(self, p);
    }
    /// Use-tree segments. `use a::b::old;` is a chain of UsePath nodes whose
    /// final name sits in a UseName/UseRename leaf - visit_path never sees
    /// it, so each use-tree node kind needs its own check. The RENAME alias
    /// (`old as alias`) rewrites the original name only.
    fn visit_use_path(&mut self, p: &syn::UsePath) {
        if p.ident == *self.old {
            self.spans.push(p.ident.span().byte_range());
        }
        syn::visit::visit_use_path(self, p);
    }
    fn visit_use_name(&mut self, n: &syn::UseName) {
        if n.ident == *self.old {
            self.spans.push(n.ident.span().byte_range());
        }
        syn::visit::visit_use_name(self, n);
    }
    fn visit_use_rename(&mut self, r: &syn::UseRename) {
        if r.ident == *self.old {
            self.spans.push(r.ident.span().byte_range());
        }
        syn::visit::visit_use_rename(self, r);
    }
    /// Macro invocations hold their arguments as a raw token stream the
    /// default traversal never descends into - walk it explicitly so
    /// references inside `assert_eq!(old(...))` and friends are rewritten
    /// too. String literals in macro args are Lit tokens (never Ident), so
    /// the phantom guarantee holds inside macros as well.
    fn visit_macro(&mut self, m: &syn::Macro) {
        // proc-macro2 1.0.107 iterates TokenStream by value only.
        collect_macro_ident_spans(m.tokens.clone(), self.old, self.spans);
        syn::visit::visit_macro(self, m);
    }
}

/// Recursively collect the byte ranges of every Ident equal to `old` inside
/// a macro's token stream.
fn collect_macro_ident_spans(
    ts: proc_macro2::TokenStream,
    old: &str,
    spans: &mut Vec<Range<usize>>,
) {
    for t in ts {
        match t {
            proc_macro2::TokenTree::Ident(i) => {
                if i == old {
                    spans.push(i.span().byte_range());
                }
            }
            proc_macro2::TokenTree::Group(g) => collect_macro_ident_spans(g.stream(), old, spans),
            _ => {}
        }
    }
}

/// Rename `old` to `new` across every `.rs` file under `root`. Returns
/// (plan lines, changed files). Errors: not found / ambiguous / parse error.
pub fn rename(
    root: &Path,
    old: &str,
    new: &str,
    dry_run: bool,
) -> Result<(Vec<String>, Vec<String>), String> {
    let files = scan_rs_files(root);
    let mut decls: Vec<DeclSite> = vec![];
    let mut per_file: Vec<(PathBuf, String, Vec<Range<usize>>)> = vec![];
    for path in &files {
        let text = std::fs::read_to_string(path)
            .map_err(|e| format!("{}: cannot read: {e}", rel(root, path)))?;
        let file = syn::parse_str::<syn::File>(&text)
            .map_err(|e| format!("{}: parse error: {e}", rel(root, path)))?;
        let r = rel(root, path);
        let mut decl_bytes: Vec<(usize, &'static str)> = vec![];
        let mut spans: Vec<Range<usize>> = vec![];
        let mut finder = Finder {
            old,
            decls: &mut decl_bytes,
            spans: &mut spans,
        };
        syn::visit::Visit::visit_file(&mut finder, &file);
        for (byte, kind) in decl_bytes {
            decls.push(DeclSite {
                file: r.clone(),
                kind,
                line: line_of(&text, byte),
            });
        }
        if !spans.is_empty() {
            per_file.push((path.clone(), text, spans));
        }
    }
    if decls.is_empty() {
        return Err(format!(
            "`{old}` is not declared (fn/type/const/static/mod/trait/enum/union or associated item) in any Rust file under the root"
        ));
    }
    if decls.len() > 1 {
        let mut msg = format!(
            "ambiguous: `{old}` is declared {} times - rename a unique name, or narrow the tree with --root:",
            decls.len()
        );
        for d in &decls {
            msg.push_str(&format!("\n  {}:{} {}", d.file, d.line, d.kind));
        }
        return Err(msg);
    }
    let site = &decls[0];

    let mut plan = vec![format!(
        "declaration: {}:{} {} `{old}` -> `{new}`",
        site.file, site.line, site.kind
    )];
    let mut changed = vec![];
    for (path, text, spans) in &per_file {
        let r = rel(root, path);
        let new_text = replace_spans(text, spans, new);
        if new_text == *text {
            continue;
        }
        let refs = spans.len() - if r == site.file { 1 } else { 0 };
        plan.push(format!("{r}: rewrite {refs} reference(s) of `{old}`"));
        if !dry_run {
            std::fs::write(path, new_text).unwrap_or_else(|e| panic!("writing {r}: {e}"));
        }
        changed.push(r);
    }
    Ok((plan, changed))
}

/// The module path of a `.rs` file: the segments after the nearest `src/`
/// ancestor, extension stripped (`src/lang/parser.rs` -> `["lang","parser"]`,
/// `crates/x/src/y.rs` -> `["y"]`). `None` for files not under a `src/`
/// directory (integration tests, standalone binaries - no module path to
/// rewrite).
pub fn module_path(root: &Path, file: &Path) -> Option<Vec<String>> {
    let rel = file.strip_prefix(root).ok()?;
    let comps: Vec<&std::ffi::OsStr> = rel.components().map(|c| c.as_os_str()).collect();
    let src_idx = comps
        .iter()
        .rposition(|c| *c == std::ffi::OsStr::new("src"))?;
    let rest = &comps[src_idx + 1..];
    if rest.is_empty() {
        return None; // the src/ directory itself, not a file in it
    }
    let mut segs: Vec<String> = vec![];
    for c in &rest[..rest.len() - 1] {
        segs.push(c.to_string_lossy().into_owned());
    }
    let stem = rest[rest.len() - 1].to_string_lossy();
    let stem = stem.strip_suffix(".rs").unwrap_or(&stem);
    if !stem.is_empty() {
        segs.push(stem.to_string());
    }
    Some(segs)
}

/// One import-site rewrite: a byte range to replace with `new_text`.
struct SiteRewrite {
    range: Range<usize>,
    new_text: String,
}

/// Walk every path and use-tree chain of one file, collecting import-site
/// rewrites for `old` -> `new`. Use trees are chains of UsePath nodes (one
/// per segment) ending in Name/Rename/Glob leaves; the same contiguous-range
/// rule applies. A branch yields at most ONE rewrite (the leftmost match), so
/// a deeper identical segment cannot double-rewrite inside an already
/// replaced range.
fn site_rewrites(file: &syn::File, old: &[String], new: &[String]) -> Vec<SiteRewrite> {
    let new_text = new.join("::");
    struct Walker<'a> {
        old: &'a [String],
        new_text: String,
        out: Vec<SiteRewrite>,
    }
    impl<'ast> syn::visit::Visit<'ast> for Walker<'_> {
        fn visit_path(&mut self, p: &syn::Path) {
            let segs: Vec<String> = p.segments.iter().map(|s| s.ident.to_string()).collect();
            if segs.len() >= self.old.len() {
                for i in 0..=(segs.len() - self.old.len()) {
                    if segs[i..i + self.old.len()] == *self.old {
                        let start = p
                            .segments
                            .iter()
                            .nth(i)
                            .unwrap()
                            .ident
                            .span()
                            .byte_range()
                            .start;
                        let end = p
                            .segments
                            .iter()
                            .nth(i + self.old.len() - 1)
                            .unwrap()
                            .ident
                            .span()
                            .byte_range()
                            .end;
                        self.out.push(SiteRewrite {
                            range: start..end,
                            new_text: self.new_text.clone(),
                        });
                        break; // leftmost only
                    }
                }
            }
            syn::visit::visit_path(self, p);
        }
        fn visit_use_tree(&mut self, t: &syn::UseTree) {
            self.walk_use(t, &mut vec![]);
        }
    }
    impl Walker<'_> {
        /// Depth-first over the use tree carrying the UsePath chain so far.
        fn walk_use(&mut self, t: &syn::UseTree, chain: &mut Vec<(String, Range<usize>)>) {
            match t {
                syn::UseTree::Path(p) => {
                    let ident = p.ident.to_string();
                    let r = p.ident.span().byte_range();
                    chain.push((ident, r));
                    if chain.len() >= self.old.len() {
                        let n = self.old.len();
                        let matches_old = chain[chain.len() - n..]
                            .iter()
                            .zip(self.old.iter())
                            .all(|((s, _), name)| *s == *name);
                        if matches_old {
                            let start = chain[chain.len() - n].1.start;
                            let end = chain[chain.len() - 1].1.end;
                            self.out.push(SiteRewrite {
                                range: start..end,
                                new_text: self.new_text.clone(),
                            });
                            // Do not descend: a match here would overlap any
                            // deeper match on the same branch.
                            return;
                        }
                    }
                    self.walk_use(&p.tree, chain);
                    chain.pop();
                }
                other => {
                    syn::visit::visit_use_tree(self, other);
                }
            }
        }
    }
    let mut w = Walker {
        old,
        new_text,
        out: vec![],
    };
    syn::visit::Visit::visit_file(&mut w, file);
    w.out
}

/// The file that declares the modules living directly inside `dir`:
/// `<dir>/mod.rs` if present, else `<parent>/<dir>.rs`, else (when `dir` is
/// a `src/` root) `lib.rs` or `main.rs`. `None` when `dir` is a brand-new
/// intermediate directory that has no declaration file yet.
fn decl_file_for_dir(root: &Path, dir: &Path) -> Option<PathBuf> {
    let mod_rs = dir.join("mod.rs");
    if mod_rs.is_file() {
        return Some(mod_rs);
    }
    let dirname = dir.file_name()?;
    if let Some(parent) = dir.parent() {
        let alt = parent.join(format!("{}.rs", dirname.to_string_lossy()));
        if alt.is_file() {
            return Some(alt);
        }
    }
    // The src/ root: the crate root file declares its direct children.
    for name in ["lib.rs", "main.rs"] {
        let p = dir.join(name);
        if p.is_file() {
            return Some(p);
        }
    }
    let _ = root;
    None
}

/// Append a declaration line to a file (creating the trailing newline if
/// needed), reading the current content fresh so earlier surgery in the same
/// run is visible.
fn append_declaration(path: &Path, decl_text: &str, root: &Path) {
    let cur = std::fs::read_to_string(path).unwrap_or_default();
    let mut out = cur;
    if !out.ends_with('\n') && !out.is_empty() {
        out.push('\n');
    }
    out.push_str(decl_text);
    out.push('\n');
    std::fs::write(path, out).unwrap_or_else(|e| panic!("writing {}: {e}", rel(root, path)));
}

/// Move module file `from` to `to` (repo-relative) and rewrite every import
/// site. Returns (plan lines, changed files - the moved file listed as
/// `<from> -> <to>`). Errors: missing source, existing target, no module
/// path, submodule directory left behind, unresolvable parent declaration.
pub fn move_file(
    root: &Path,
    from: &str,
    to: &str,
    dry_run: bool,
) -> Result<(Vec<String>, Vec<String>), String> {
    let from_p = root.join(from);
    let to_p = root.join(to);
    if !from_p.is_file() {
        return Err(format!("{from}: no such file"));
    }
    if to_p.exists() {
        return Err(format!("{to}: target already exists"));
    }
    if from_p.with_extension("").is_dir() {
        return Err(format!(
            "{from}: the module has a submodule directory ({}) - move it together with the file",
            from_p.with_extension("").display()
        ));
    }
    let old_mod = module_path(root, &from_p)
        .ok_or_else(|| format!("{from}: not under a src/ directory - no module path to rewrite"))?;
    let new_mod = module_path(root, &to_p)
        .ok_or_else(|| format!("{to}: not under a src/ directory - no module path to rewrite"))?;
    if old_mod == new_mod {
        return Err(format!(
            "{from} and {to} name the same module path ({}) - nothing to move",
            old_mod.join("::")
        ));
    }
    let old_stem = old_mod.last().unwrap();
    let new_stem = new_mod.last().unwrap();

    // --- mod declaration surgery ------------------------------------------
    let old_decl = decl_file_for_dir(root, from_p.parent().unwrap())
        .ok_or_else(|| format!("{from}: no declaration file found for the old parent module"))?;
    let new_decl_dir = to_p.parent().unwrap().to_path_buf();
    let new_decl = decl_file_for_dir(root, &new_decl_dir);

    // Read the old declaration line (visibility + `mod <stem>;`) to reuse it.
    let old_decl_text = std::fs::read_to_string(&old_decl)
        .map_err(|e| format!("{}: cannot read: {e}", rel(root, &old_decl)))?;
    let old_file = syn::parse_str::<syn::File>(&old_decl_text)
        .map_err(|e| format!("{}: parse error: {e}", rel(root, &old_decl)))?;
    let mut decl_line_range: Option<Range<usize>> = None;
    let mut decl_delete_range: Option<Range<usize>> = None;
    let mut decl_text: Option<String> = None;
    for item in &old_file.items {
        if let syn::Item::Mod(m) = item {
            if m.ident == *old_stem && m.content.is_none() {
                // Include the visibility: deleting from `mod` would leave a
                // dangling `pub` behind.
                let start =
                    first_token_start(&m.attrs, &m.vis, m.mod_token.span.byte_range().start);
                let semi_end = m
                    .semi
                    .as_ref()
                    .map(|s| s.span.byte_range().end)
                    .unwrap_or(start);
                decl_line_range = Some(start..semi_end);
                // Deletion swallows the trailing newline so no blank line is
                // left behind; replacement (same-parent) keeps it.
                let mut del_end = semi_end;
                if old_decl_text.get(del_end..del_end + 1) == Some("\n") {
                    del_end += 1;
                }
                decl_delete_range = Some(start..del_end);
                decl_text = Some(old_decl_text[start..semi_end].trim_end().to_string());
                break;
            }
        }
    }
    let (decl_line_range, decl_delete_range, decl_text) =
        match (decl_line_range, decl_delete_range, decl_text) {
            (Some(r), Some(d), Some(t)) => (r, d, t),
            _ => {
                return Err(format!(
            "{}: no `mod {old_stem};` declaration found - the module is not declared in its parent",
            rel(root, &old_decl)
        ))
            }
        };

    let mut plan = vec![format!(
        "move {from} -> {to} (module {} -> {})",
        old_mod.join("::"),
        new_mod.join("::")
    )];
    let mut changed: Vec<String> = vec![];
    let same_parent = new_decl.as_deref() == Some(old_decl.as_path());

    if same_parent {
        // In-place stem rewrite in the shared declaration file.
        let new_decl_text2 =
            decl_text.replace(&format!("mod {old_stem};"), &format!("mod {new_stem};"));
        plan.push(format!(
            "{}: rewrite declaration `{}` -> `{}`",
            rel(root, &old_decl),
            decl_text,
            new_decl_text2
        ));
        if !dry_run {
            let rewritten = replace_spans(&old_decl_text, &[decl_line_range], &new_decl_text2);
            std::fs::write(&old_decl, rewritten)
                .unwrap_or_else(|e| panic!("writing {}: {e}", rel(root, &old_decl)));
        }
        changed.push(rel(root, &old_decl));
    } else {
        plan.push(format!(
            "{}: remove declaration `{}`",
            rel(root, &old_decl),
            decl_text
        ));
        if !dry_run {
            let rewritten = replace_spans(&old_decl_text, &[decl_delete_range], "");
            std::fs::write(&old_decl, rewritten)
                .unwrap_or_else(|e| panic!("writing {}: {e}", rel(root, &old_decl)));
        }
        changed.push(rel(root, &old_decl));

        let new_decl_text =
            decl_text.replace(&format!("mod {old_stem};"), &format!("mod {new_stem};"));
        match &new_decl {
            Some(nd) => {
                plan.push(format!(
                    "{}: add declaration `{}`",
                    rel(root, nd),
                    new_decl_text
                ));
                if !dry_run {
                    append_declaration(nd, &new_decl_text, root);
                }
                changed.push(rel(root, nd));
            }
            None => {
                // Brand-new intermediate directory: create its mod.rs AND
                // declare the new directory module in ITS parent's
                // declaration file (which must already exist - only the final
                // directory level is created).
                let dirname = new_decl_dir
                    .file_name()
                    .unwrap()
                    .to_string_lossy()
                    .into_owned();
                let grandparent_decl = decl_file_for_dir(root, new_decl_dir.parent().unwrap())
                    .ok_or_else(|| format!(
                        "{to}: the intermediate directory {} needs a parent declaration file (mod.rs or <dir>.rs) that does not exist - only one new directory level is created",
                        rel(root, &new_decl_dir)
                    ))?;
                let dir_decl_text =
                    decl_text.replace(&format!("{old_stem};"), &format!("{dirname};"));
                let new_mod_rs = new_decl_dir.join("mod.rs");
                plan.push(format!(
                    "create {} with `{}`",
                    rel(root, &new_mod_rs),
                    new_decl_text
                ));
                plan.push(format!(
                    "{}: add declaration `{}`",
                    rel(root, &grandparent_decl),
                    dir_decl_text
                ));
                if !dry_run {
                    std::fs::create_dir_all(&new_decl_dir)
                        .unwrap_or_else(|e| panic!("creating {}: {e}", rel(root, &new_decl_dir)));
                    std::fs::write(&new_mod_rs, format!("{new_decl_text}\n"))
                        .unwrap_or_else(|e| panic!("writing {}: {e}", rel(root, &new_mod_rs)));
                    append_declaration(&grandparent_decl, &dir_decl_text, root);
                }
                changed.push(rel(root, &new_mod_rs));
                changed.push(rel(root, &grandparent_decl));
            }
        }
    }

    // --- import-site rewrites ---------------------------------------------
    let files = scan_rs_files(root);
    for path in &files {
        if path == &from_p || path == &to_p {
            continue;
        }
        let text = match std::fs::read_to_string(path) {
            Ok(t) => t,
            Err(_) => continue,
        };
        let file = match syn::parse_str::<syn::File>(&text) {
            Ok(f) => f,
            Err(e) => return Err(format!("{}: parse error: {e}", rel(root, path))),
        };
        let rewrites = site_rewrites(&file, &old_mod, &new_mod);
        if rewrites.is_empty() {
            continue;
        }
        // Collapse to non-overlapping ranges (keep the leftmost of each
        // overlapping group).
        let mut ranges: Vec<SiteRewrite> = rewrites;
        ranges.sort_by_key(|r| (r.range.start, r.range.end));
        let mut collapsed: Vec<SiteRewrite> = vec![];
        for r in ranges {
            match collapsed.last_mut() {
                Some(last) if r.range.start < last.range.end => {} // overlap - skip
                _ => collapsed.push(r),
            }
        }
        let mut out = text.clone();
        for r in collapsed.iter().rev() {
            out.replace_range(r.range.clone(), &r.new_text);
        }
        if out == text {
            continue;
        }
        plan.push(format!(
            "{}: rewrite {} import site(s) of module `{}`",
            rel(root, path),
            collapsed.len(),
            old_mod.join("::")
        ));
        if !dry_run {
            std::fs::write(path, out)
                .unwrap_or_else(|e| panic!("writing {}: {e}", rel(root, path)));
        }
        changed.push(rel(root, path));
    }

    // --- the physical move --------------------------------------------------
    if !dry_run {
        if let Some(parent) = to_p.parent() {
            std::fs::create_dir_all(parent)
                .unwrap_or_else(|e| panic!("creating {}: {e}", rel(root, parent)));
        }
        std::fs::rename(&from_p, &to_p).unwrap_or_else(|e| panic!("moving {from} -> {to}: {e}"));
    }
    changed.push(format!("{from} -> {to}"));

    // The declaration file can appear twice (surgery + import-site rewrite);
    // de-duplicate keeping first occurrence.
    let mut seen = std::collections::BTreeSet::new();
    changed.retain(|c| seen.insert(c.clone()));
    Ok((plan, changed))
}

/// The first byte of an item: the leading attribute if any, else the
/// visibility token if present, else the keyword. Replacing from the IDENT
/// would leave `pub fn` / `mod` prefixes behind in the file.
fn first_token_start(attrs: &[syn::Attribute], vis: &syn::Visibility, kw_start: usize) -> usize {
    if let Some(a) = attrs.first() {
        return a.pound_token.span.byte_range().start;
    }
    let vis_start = match vis {
        syn::Visibility::Inherited => None,
        syn::Visibility::Public(p) => Some(p.span.byte_range().start),
        // `pub(crate)` / `pub(self)` / `pub(in path)` all land here.
        syn::Visibility::Restricted(r) => Some(r.pub_token.span.byte_range().start),
    };
    vis_start.unwrap_or(kw_start).min(kw_start)
}

/// The byte range of a whole top-level item: first token through the closing
/// brace/semicolon. `None` for item kinds the tool does not replace.
fn item_range(item: &syn::Item) -> Option<Range<usize>> {
    match item {
        syn::Item::Fn(f) => Some(
            first_token_start(&f.attrs, &f.vis, f.sig.fn_token.span.byte_range().start)
                ..f.block.brace_token.span.close().byte_range().end,
        ),
        syn::Item::Struct(s) => {
            let end = match &s.fields {
                syn::Fields::Unit => s.semi_token.as_ref()?.span.byte_range().end,
                syn::Fields::Named(f) => f.brace_token.span.close().byte_range().end,
                syn::Fields::Unnamed(f) => f.paren_token.span.close().byte_range().end,
            };
            Some(first_token_start(&s.attrs, &s.vis, s.struct_token.span.byte_range().start)..end)
        }
        syn::Item::Enum(e) => Some(
            first_token_start(&e.attrs, &e.vis, e.enum_token.span.byte_range().start)
                ..e.brace_token.span.close().byte_range().end,
        ),
        syn::Item::Trait(t) => Some(
            first_token_start(&t.attrs, &t.vis, t.trait_token.span.byte_range().start)
                ..t.brace_token.span.close().byte_range().end,
        ),
        syn::Item::Impl(i) => {
            let kw = i
                .defaultness
                .as_ref()
                .map(|d| d.span.byte_range().start)
                .or_else(|| i.unsafety.as_ref().map(|u| u.span.byte_range().start))
                .unwrap_or_else(|| i.impl_token.span.byte_range().start);
            Some(
                first_token_start(&i.attrs, &syn::Visibility::Inherited, kw)
                    ..i.brace_token.span.close().byte_range().end,
            )
        }
        syn::Item::Type(ty) => Some(
            first_token_start(&ty.attrs, &ty.vis, ty.type_token.span.byte_range().start)
                ..ty.semi_token.span.byte_range().end,
        ),
        syn::Item::Const(c) => Some(
            first_token_start(&c.attrs, &c.vis, c.const_token.span.byte_range().start)
                ..c.semi_token.span.byte_range().end,
        ),
        syn::Item::Static(s) => Some(
            first_token_start(&s.attrs, &s.vis, s.static_token.span.byte_range().start)
                ..s.semi_token.span.byte_range().end,
        ),
        syn::Item::Mod(m) => {
            let end = match &m.content {
                Some((brace, _)) => brace.span.close().byte_range().end,
                None => m.semi.as_ref()?.span.byte_range().end,
            };
            Some(first_token_start(&m.attrs, &m.vis, m.mod_token.span.byte_range().start)..end)
        }
        syn::Item::Union(u) => Some(
            first_token_start(&u.attrs, &u.vis, u.union_token.span.byte_range().start)
                ..u.fields.brace_token.span.close().byte_range().end,
        ),
        _ => None,
    }
}

/// The byte range of an associated item (method / associated const or type)
/// inside an impl or trait block.
fn assoc_item_range(item: &syn::ImplItem) -> Option<Range<usize>> {
    match item {
        syn::ImplItem::Fn(f) => Some(
            first_token_start(&f.attrs, &f.vis, f.sig.fn_token.span.byte_range().start)
                ..f.block.brace_token.span.close().byte_range().end,
        ),
        syn::ImplItem::Const(c) => Some(
            first_token_start(&c.attrs, &c.vis, c.const_token.span.byte_range().start)
                ..c.semi_token.span.byte_range().end,
        ),
        syn::ImplItem::Type(t) => Some(
            first_token_start(&t.attrs, &t.vis, t.type_token.span.byte_range().start)
                ..t.semi_token.span.byte_range().end,
        ),
        _ => None,
    }
}

fn trait_assoc_item_range(item: &syn::TraitItem) -> Option<Range<usize>> {
    match item {
        // Trait items carry no visibility of their own.
        syn::TraitItem::Fn(f) => Some(
            first_token_start(
                &f.attrs,
                &syn::Visibility::Inherited,
                f.sig.fn_token.span.byte_range().start,
            )..match &f.default {
                Some(d) => d.brace_token.span.close().byte_range().end,
                None => f.semi_token.as_ref()?.span.byte_range().end,
            },
        ),
        syn::TraitItem::Const(c) => Some(
            first_token_start(
                &c.attrs,
                &syn::Visibility::Inherited,
                c.const_token.span.byte_range().start,
            )..c.semi_token.span.byte_range().end,
        ),
        syn::TraitItem::Type(t) => Some(
            first_token_start(
                &t.attrs,
                &syn::Visibility::Inherited,
                t.type_token.span.byte_range().start,
            )..t.semi_token.span.byte_range().end,
        ),
        _ => None,
    }
}

/// The name an item is replaceable by: its own ident, or (for impl blocks)
/// the last segment of the self type.
fn item_name(item: &syn::Item) -> Option<(String, &'static str)> {
    match item {
        syn::Item::Fn(f) => Some((f.sig.ident.to_string(), "fn")),
        syn::Item::Struct(s) => Some((s.ident.to_string(), "struct")),
        syn::Item::Enum(e) => Some((e.ident.to_string(), "enum")),
        syn::Item::Trait(t) => Some((t.ident.to_string(), "trait")),
        syn::Item::Impl(i) => match i.self_ty.as_ref() {
            syn::Type::Path(tp) => tp
                .path
                .segments
                .last()
                .map(|s| (s.ident.to_string(), "impl block")),
            _ => None,
        },
        syn::Item::Type(ty) => Some((ty.ident.to_string(), "type alias")),
        syn::Item::Const(c) => Some((c.ident.to_string(), "const")),
        syn::Item::Static(s) => Some((s.ident.to_string(), "static")),
        syn::Item::Mod(m) => Some((m.ident.to_string(), "mod")),
        syn::Item::Union(u) => Some((u.ident.to_string(), "union")),
        _ => None,
    }
}

/// Replace the item named `item` in `file` (repo-relative) with the content
/// of `new_from` (repo-relative). Returns (plan lines, changed files).
pub fn replace_item(
    root: &Path,
    file: &str,
    item: &str,
    new_from: &str,
    dry_run: bool,
) -> Result<(Vec<String>, Vec<String>), String> {
    let path = root.join(file);
    if !path.is_file() {
        return Err(format!("{file}: no such file"));
    }
    let text = std::fs::read_to_string(&path).map_err(|e| format!("{file}: cannot read: {e}"))?;
    let parsed =
        syn::parse_str::<syn::File>(&text).map_err(|e| format!("{file}: parse error: {e}"))?;
    let new_content = std::fs::read_to_string(root.join(new_from))
        .map_err(|e| format!("{new_from}: cannot read: {e}"))?;

    // Top-level items plus associated items inside impl/trait blocks: a
    // method is replaceable by its name exactly like a top-level fn, and the
    // ambiguity check then catches same-named methods across impls.
    let mut matches: Vec<(String, &'static str, Range<usize>)> = vec![];
    for it in &parsed.items {
        if let Some((name, kind)) = item_name(it) {
            if name == item {
                if let Some(r) = item_range(it) {
                    matches.push((name, kind, r));
                }
            }
        }
        if let syn::Item::Impl(i) = it {
            for ai in &i.items {
                let (name, kind) = match ai {
                    syn::ImplItem::Fn(f) => (f.sig.ident.to_string(), "method"),
                    syn::ImplItem::Const(c) => (c.ident.to_string(), "associated const"),
                    syn::ImplItem::Type(t) => (t.ident.to_string(), "associated type"),
                    _ => continue,
                };
                if name == item {
                    if let Some(r) = assoc_item_range(ai) {
                        matches.push((name, kind, r));
                    }
                }
            }
        }
        if let syn::Item::Trait(t) = it {
            for ai in &t.items {
                let (name, kind) = match ai {
                    syn::TraitItem::Fn(f) => (f.sig.ident.to_string(), "trait method"),
                    syn::TraitItem::Const(c) => (c.ident.to_string(), "trait const"),
                    syn::TraitItem::Type(ty) => (ty.ident.to_string(), "trait associated type"),
                    _ => continue,
                };
                if name == item {
                    if let Some(r) = trait_assoc_item_range(ai) {
                        matches.push((name, kind, r));
                    }
                }
            }
        }
    }
    if matches.is_empty() {
        return Err(format!(
            "`{item}` is not declared in {file} (top-level fn/type/const/static/mod/trait/enum/union or an impl block)"
        ));
    }
    if matches.len() > 1 {
        let mut msg = format!(
            "ambiguous: `{item}` matches {} items in {file}:",
            matches.len()
        );
        for (_, kind, r) in &matches {
            msg.push_str(&format!(
                "\n  {} (lines {}-{})",
                kind,
                line_of(&text, r.start),
                line_of(&text, r.end)
            ));
        }
        return Err(msg);
    }
    let (_, kind, range0) = &matches[0];
    // The item's own line terminator moves with it: without this, the
    // replacement would leave a doubled blank line where the item was.
    let mut range = range0.clone();
    if text.get(range.end..range.end + 1) == Some("\n") {
        range.end += 1;
    }
    // Line numbers report the item's own extent (the newline extension is a
    // splicing detail, not part of the item).
    let plan = vec![format!(
        "{file}: replace {kind} `{item}` (lines {}-{}) with content of {new_from}",
        line_of(&text, range.start),
        line_of(&text, range0.end)
    )];
    if !dry_run {
        let mut out = text.clone();
        out.replace_range(range.clone(), &new_content);
        std::fs::write(&path, out).unwrap_or_else(|e| panic!("writing {file}: {e}"));
    }
    Ok((plan, vec![file.to_string()]))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn root_with(files: &[(&str, &str)]) -> tempfile_free::Root {
        tempfile_free::Root::new(files)
    }

    /// A throwaway directory tree (no external dep: std::env::temp_dir + a
    /// unique suffix).
    mod tempfile_free {
        use std::path::PathBuf;
        pub struct Root {
            pub path: PathBuf,
        }
        impl Root {
            pub fn new(files: &[(&str, &str)]) -> Self {
                let dir = std::env::temp_dir().join(format!(
                    "rfs-dev-edit-test-{}-{}",
                    std::process::id(),
                    rand_suffix()
                ));
                for (rel, content) in files {
                    let p = dir.join(rel);
                    if let Some(parent) = p.parent() {
                        std::fs::create_dir_all(parent).unwrap();
                    }
                    std::fs::write(&p, content).unwrap();
                }
                Self { path: dir }
            }
        }
        impl Drop for Root {
            fn drop(&mut self) {
                let _ = std::fs::remove_dir_all(&self.path);
            }
        }
        fn rand_suffix() -> String {
            use std::hash::{Hash, Hasher};
            let mut h = std::collections::hash_map::DefaultHasher::new();
            std::time::SystemTime::now().hash(&mut h);
            format!("{:x}", h.finish())
        }
    }

    const ALPHA: &str = "pub fn target_fn(x: u32) -> u32 {\n    x * 2\n}\n";
    const LIB: &str =
        "pub mod alpha;\npub fn drive() -> u32 {\n    crate::alpha::target_fn(1)\n}\n";

    #[test]
    fn rename_rewrites_declaration_paths_and_use_trees() {
        let root = root_with(&[("src/lib.rs", LIB), ("src/alpha.rs", ALPHA)]);
        let (plan, changed) = rename(&root.path, "target_fn", "doubled", false).unwrap();
        assert_eq!(changed, vec!["src/alpha.rs", "src/lib.rs"]);
        assert!(
            plan[0].contains("declaration: src/alpha.rs:1 fn `target_fn` -> `doubled`"),
            "{plan:?}"
        );
        let alpha = std::fs::read_to_string(root.path.join("src/alpha.rs")).unwrap();
        assert_eq!(alpha, "pub fn doubled(x: u32) -> u32 {\n    x * 2\n}\n");
        let lib = std::fs::read_to_string(root.path.join("src/lib.rs")).unwrap();
        assert_eq!(
            lib,
            "pub mod alpha;\npub fn drive() -> u32 {\n    crate::alpha::doubled(1)\n}\n"
        );
    }

    #[test]
    fn rename_leaves_strings_and_comments_untouched() {
        let src = "pub fn target_fn() -> u32 { 0 }\nfn use_it() {\n    let s = \"target_fn is documented\";\n    // target_fn: see docs\n    target_fn()\n}\n";
        let root = root_with(&[("src/a.rs", src)]);
        rename(&root.path, "target_fn", "doubled", false).unwrap();
        let out = std::fs::read_to_string(root.path.join("src/a.rs")).unwrap();
        assert!(
            out.contains("\"target_fn is documented\""),
            "string literal must survive: {out}"
        );
        assert!(
            out.contains("// target_fn: see docs"),
            "comment must survive: {out}"
        );
        assert!(
            out.contains("doubled()"),
            "call site must be rewritten: {out}"
        );
        assert_eq!(
            out.matches("doubled").count(),
            2,
            "declaration + call only: {out}"
        );
    }

    #[test]
    fn rename_use_name_and_rename_alias() {
        let src = "use crate::alpha::target_fn;\nuse crate::alpha::target_fn as tf;\nfn go() { target_fn(); tf(); }\n";
        let root = root_with(&[
            ("src/lib.rs", "pub mod alpha;\n"),
            ("src/alpha.rs", ALPHA),
            ("src/beta.rs", src),
        ]);
        rename(&root.path, "target_fn", "doubled", false).unwrap();
        let out = std::fs::read_to_string(root.path.join("src/beta.rs")).unwrap();
        assert_eq!(out, "use crate::alpha::doubled;\nuse crate::alpha::doubled as tf;\nfn go() { doubled(); tf(); }\n");
    }

    #[test]
    fn rename_rewrites_references_inside_macro_invocations() {
        let src = "pub fn target_fn(x: u32) -> u32 { x }\nfn check() {\n    assert_eq!(target_fn(1), 2);\n    assert!(target_fn(0) == 0 && \"target_fn stays\");\n}\n";
        let root = root_with(&[("src/a.rs", src)]);
        rename(&root.path, "target_fn", "doubled", false).unwrap();
        let out = std::fs::read_to_string(root.path.join("src/a.rs")).unwrap();
        assert!(out.contains("assert_eq!(doubled(1), 2);"), "{out}");
        assert!(out.contains("doubled(0) == 0"), "{out}");
        // The string literal inside the macro args survives.
        assert!(out.contains("\"target_fn stays\""), "{out}");
    }

    #[test]
    fn rename_not_found_and_ambiguous() {
        let root = root_with(&[("src/a.rs", "fn other() {}\n")]);
        let err = rename(&root.path, "ghost", "x", false).unwrap_err();
        assert!(err.contains("not declared"), "{err}");

        let root = root_with(&[("src/a.rs", "fn dup() {}\n"), ("src/b.rs", "fn dup() {}\n")]);
        let err = rename(&root.path, "dup", "x", false).unwrap_err();
        assert!(
            err.starts_with("ambiguous: `dup` is declared 2 times"),
            "{err}"
        );
        assert!(
            err.contains("src/a.rs:1 fn") && err.contains("src/b.rs:1 fn"),
            "{err}"
        );
    }

    #[test]
    fn rename_dry_run_writes_nothing() {
        let root = root_with(&[("src/lib.rs", LIB), ("src/alpha.rs", ALPHA)]);
        let (plan, changed) = rename(&root.path, "target_fn", "doubled", true).unwrap();
        assert_eq!(changed.len(), 2);
        assert!(plan
            .iter()
            .all(|l| l.starts_with("declaration:") || l.contains("rewrite")));
        assert_eq!(
            std::fs::read_to_string(root.path.join("src/alpha.rs")).unwrap(),
            ALPHA,
            "dry-run must not write"
        );
    }

    #[test]
    fn module_path_derivation() {
        let root = root_with(&[
            ("src/lib.rs", ""),
            ("src/foo.rs", ""),
            ("src/lang/parser.rs", ""),
            ("crates/x/src/y.rs", ""),
            ("tests/integ.rs", ""),
        ]);
        assert_eq!(
            module_path(&root.path, &root.path.join("src/foo.rs")),
            Some(vec!["foo".into()])
        );
        assert_eq!(
            module_path(&root.path, &root.path.join("src/lang/parser.rs")),
            Some(vec!["lang".into(), "parser".into()])
        );
        assert_eq!(
            module_path(&root.path, &root.path.join("crates/x/src/y.rs")),
            Some(vec!["y".into()])
        );
        assert_eq!(
            module_path(&root.path, &root.path.join("tests/integ.rs")),
            None
        );
    }

    #[test]
    fn move_rewrites_import_sites_and_mod_declarations() {
        let lib = "pub mod alpha;\npub mod beta;\npub fn drive() -> u32 {\n    crate::beta::helper() + beta::helper()\n}\n";
        let beta = "pub fn helper() -> u32 { 1 }\n";
        let root = root_with(&[("src/lib.rs", lib), ("src/beta.rs", beta)]);
        let (plan, changed) =
            move_file(&root.path, "src/beta.rs", "src/omega/beta.rs", false).unwrap();
        assert!(
            plan.iter()
                .any(|l| l.starts_with("move src/beta.rs -> src/omega/beta.rs")),
            "{plan:?}"
        );
        assert!(
            plan.iter()
                .any(|l| l.contains("remove declaration `pub mod beta;`")),
            "{plan:?}"
        );
        assert!(
            plan.iter()
                .any(|l| l.contains("create src/omega/mod.rs with `pub mod beta;`")),
            "{plan:?}"
        );
        // The new intermediate directory module must be declared in lib.rs.
        assert!(
            plan.iter()
                .any(|l| l.contains("src/lib.rs: add declaration `pub mod omega;`")),
            "{plan:?}"
        );
        assert!(
            plan.iter()
                .any(|l| l.contains("src/lib.rs: rewrite 2 import site(s)")),
            "{plan:?}"
        );
        assert!(
            !root.path.join("src/beta.rs").exists(),
            "old file must be gone"
        );
        assert!(
            root.path.join("src/omega/beta.rs").is_file(),
            "new file must exist"
        );
        let omega_mod = std::fs::read_to_string(root.path.join("src/omega/mod.rs")).unwrap();
        assert_eq!(omega_mod, "pub mod beta;\n");
        let lib_out = std::fs::read_to_string(root.path.join("src/lib.rs")).unwrap();
        assert!(
            !lib_out.lines().any(|l| l.trim() == "pub mod beta;"),
            "{lib_out}"
        );
        assert!(lib_out.contains("pub mod omega;"), "{lib_out}");
        assert!(
            lib_out.contains("crate::omega::beta::helper() + omega::beta::helper()"),
            "{lib_out}"
        );
        let _ = changed;
    }

    #[test]
    fn move_same_parent_rewrites_stem_in_place() {
        let lib = "pub mod beta;\npub fn drive() -> u32 { beta::helper() }\n";
        let root = root_with(&[
            ("src/lib.rs", lib),
            ("src/beta.rs", "pub fn helper() -> u32 { 1 }\n"),
        ]);
        move_file(&root.path, "src/beta.rs", "src/gamma.rs", false).unwrap();
        let lib_out = std::fs::read_to_string(root.path.join("src/lib.rs")).unwrap();
        assert!(lib_out.contains("pub mod gamma;"), "{lib_out}");
        assert!(lib_out.contains("gamma::helper()"), "{lib_out}");
    }

    #[test]
    fn move_errors() {
        let root = root_with(&[
            ("src/lib.rs", "pub mod beta;\n"),
            ("src/beta.rs", "pub fn helper() -> u32 { 1 }\n"),
        ]);
        assert!(move_file(&root.path, "src/nope.rs", "src/x/nope.rs", false)
            .unwrap_err()
            .contains("no such file"));
        assert!(move_file(&root.path, "src/beta.rs", "src/beta.rs", false)
            .unwrap_err()
            .contains("already exists"));
        let root2 = root_with(&[("tests/integ.rs", "fn main() {}\n")]);
        assert!(
            move_file(&root2.path, "tests/integ.rs", "tests/integ2.rs", false)
                .unwrap_err()
                .contains("not under a src/ directory")
        );
    }

    #[test]
    fn replace_item_swaps_the_named_item_only() {
        let alpha =
            "pub fn target_fn(x: u32) -> u32 {\n    x * 2\n}\n\npub fn keep() -> u32 { 7 }\n";
        let root = root_with(&[
            ("src/alpha.rs", alpha),
            (
                "new.txt",
                "pub fn target_fn(x: u32) -> u32 {\n    x + 41\n}\n",
            ),
        ]);
        let (plan, changed) =
            replace_item(&root.path, "src/alpha.rs", "target_fn", "new.txt", false).unwrap();
        assert_eq!(changed, vec!["src/alpha.rs".to_string()]);
        assert!(
            plan[0].contains("replace fn `target_fn` (lines 1-3)"),
            "{plan:?}"
        );
        let out = std::fs::read_to_string(root.path.join("src/alpha.rs")).unwrap();
        assert_eq!(
            out,
            "pub fn target_fn(x: u32) -> u32 {\n    x + 41\n}\n\npub fn keep() -> u32 { 7 }\n"
        );
    }

    #[test]
    fn replace_item_impl_block_and_errors() {
        let src = "struct Alpha;\nimpl Alpha {\n    pub fn one(&self) {}\n}\npub fn one() {}\n";
        let root = root_with(&[
            ("src/a.rs", src),
            (
                "new.txt",
                "impl T for String {\n    fn go(&self) -> u32 { 1 }\n}\n",
            ),
        ]);
        // `one` is ambiguous: a method AND a top-level fn.
        let err = replace_item(&root.path, "src/a.rs", "one", "new.txt", false).unwrap_err();
        assert!(err.starts_with("ambiguous: `one` matches 2 items"), "{err}");
        assert!(err.contains("method") && err.contains("fn"), "{err}");

        // An impl block for a LOCAL type is inherently ambiguous with the
        // type's own declaration - both are listed, exit 1 (documented
        // contract: the user sees exactly which two items collide).
        let err = replace_item(&root.path, "src/a.rs", "Alpha", "new.txt", false).unwrap_err();
        assert!(
            err.starts_with("ambiguous: `Alpha` matches 2 items"),
            "{err}"
        );
        assert!(
            err.contains("struct") && err.contains("impl block"),
            "{err}"
        );

        // An impl for a FOREIGN type is addressable without ambiguity.
        let src2 = "trait T { fn go(&self); }\nimpl T for String {\n    fn go(&self) {}\n}\n";
        let root2 = root_with(&[
            ("src/b.rs", src2),
            (
                "new.txt",
                "impl T for String {\n    fn go(&self) -> u32 { 1 }\n}\n",
            ),
        ]);
        let (plan, _) = replace_item(&root2.path, "src/b.rs", "String", "new.txt", false).unwrap();
        assert!(plan[0].contains("replace impl block `String`"), "{plan:?}");
        let out = std::fs::read_to_string(root2.path.join("src/b.rs")).unwrap();
        assert_eq!(
            out,
            "trait T { fn go(&self); }\nimpl T for String {\n    fn go(&self) -> u32 { 1 }\n}\n"
        );

        let err = replace_item(&root.path, "src/a.rs", "ghost", "new.txt", false).unwrap_err();
        assert!(err.contains("not declared"), "{err}");
    }

    #[test]
    fn replace_spans_back_to_front_and_dedup() {
        let text = "abcdef";
        assert_eq!(replace_spans(text, &[1..2, 4..5], "X"), "aXcdXf");
        assert_eq!(
            replace_spans(text, &[1..2, 1..2], "X"),
            "aXcdef",
            "duplicates collapse"
        );
    }

    #[test]
    fn line_of_counts_newlines() {
        assert_eq!(line_of("a\nb\nc", 0), 1);
        assert_eq!(line_of("a\nb\nc", 2), 2);
        assert_eq!(line_of("a\nb\nc", 4), 3);
    }
}
