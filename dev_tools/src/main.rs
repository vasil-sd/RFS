//! `rfs-dev` CLI - Phase 12 maintenance tooling dispatch.
//!
//! Task 1 fixes the FULL subcommand surface as pinned stubs; task 5 fills the
//! bodies. Exit-code convention mirrors the rfs CLI: 0 success, 1 operational
//! failure, 2 usage error. An unimplemented subcommand is a usage error against
//! the current build: pinned message, exit 2. No `unimplemented!()`/`todo!()`
//! macros anywhere - they are panic paths; stubs return typed errors.

/// The pinned subcommand surface (Phase 12 task 1). Task 4's proposal table
/// must match these names exactly; a rename is a plan correction, never a
/// silent edit.
const SUBCOMMANDS: &[&str] = &[
    "sync-check",
    "sync-fix",
    "context",
    "edit-rename",
    "edit-move",
    "edit-replace-item",
    "doc-section",
    "inventory",
    "calls",
];

/// Subverbs of the two grouped subcommands (pinned with the surface).
const INVENTORY_SUBVERBS: &[&str] = &["add", "update", "remove", "audit"];
const CALLS_SUBVERBS: &[&str] = &["extract", "classify"];

fn usage() -> String {
    "usage: rfs-dev <sync-check|sync-fix|context|edit-rename|edit-move|".to_string()
        + "edit-replace-item|doc-section|inventory <add|update|remove|audit>|"
        + "calls <extract|classify>>"
}

fn main() {
    let args: Vec<String> = std::env::args().skip(1).collect();
    std::process::exit(dispatch(&args));
}

/// Dispatch logic, split from `main` so the unit tests can drive it directly.
/// Writes its diagnostics to stderr and returns the exit code.
fn dispatch(args: &[String]) -> i32 {
    let Some(sub) = args.first() else {
        eprintln!("{}`", usage());
        return 2;
    };
    if !SUBCOMMANDS.contains(&sub.as_str()) {
        eprintln!("rfs-dev: unknown subcommand `{sub}`");
        eprintln!("{}`", usage());
        return 2;
    }
    // Grouped subcommands require a valid subverb; the stub message then names
    // the full `group subverb` pair.
    let full = match sub.as_str() {
        "inventory" => match args.get(1) {
            Some(v) if INVENTORY_SUBVERBS.contains(&v.as_str()) => format!("inventory {v}"),
            _ => {
                eprintln!("rfs-dev: `inventory` requires one of: add | update | remove | audit");
                eprintln!("{}`", usage());
                return 2;
            }
        },
        "calls" => match args.get(1) {
            Some(v) if CALLS_SUBVERBS.contains(&v.as_str()) => format!("calls {v}"),
            _ => {
                eprintln!("rfs-dev: `calls` requires one of: extract | classify");
                eprintln!("{}`", usage());
                return 2;
            }
        },
        other => other.to_string(),
    };
    match full.as_str() {
        "sync-check" | "sync-fix" => run_sync(&full, &args[1..]),
        "context" => run_context(&args[1..]),
        "edit-rename" => run_edit_rename(&args[1..]),
        "edit-move" => run_edit_move(&args[1..]),
        "edit-replace-item" => run_edit_replace_item(&args[1..]),
        "doc-section" => run_doc_section(&args[1..]),
        "inventory add" | "inventory update" | "inventory remove" | "inventory audit" => {
            run_inventory(&full, &args[2..])
        }
        "calls extract" => run_calls_extract(&args[2..]),
        "calls classify" => run_calls_classify(&args[2..]),
        // Remaining stubs - pinned message, asserted byte-for-byte by
        // tests/dispatch_test.rs. Task 5 fills them subtask by subtask.
        _ => {
            eprintln!("rfs-dev {full}: not implemented yet (Phase 12 task 5)");
            2
        }
    }
}

/// Resolve `--root DIR` or walk up from the cwd to the repo root - the
/// directory carrying `tests/dev_docs_manifest.txt` (the RFS repo marker).
fn resolve_root(args: &[String]) -> Result<std::path::PathBuf, String> {
    let mut i = 0;
    while i < args.len() {
        if args[i] == "--root" {
            i += 1;
            if i >= args.len() {
                return Err("--root requires a directory argument".into());
            }
            let p = std::path::PathBuf::from(&args[i]);
            if !p.join("tests/dev_docs_manifest.txt").is_file() {
                return Err(format!(
                    "{p:?} is not an RFS repo root (no tests/dev_docs_manifest.txt)"
                ));
            }
            return Ok(p);
        }
        i += 1;
    }
    let mut dir = std::env::current_dir().map_err(|e| e.to_string())?;
    loop {
        if dir.join("tests/dev_docs_manifest.txt").is_file() {
            return Ok(dir);
        }
        if !dir.pop() {
            return Err(
                "no --root given and no tests/dev_docs_manifest.txt found walking up from the cwd"
                    .into(),
            );
        }
    }
}

/// `context <name> [--file F] [--depth N] [--root DIR]` - one-command impact
/// context for a fn/type (task 5.2). Read-only; the rendered template goes to
/// stdout, operational failures (no match / ambiguous) to stderr with exit 1.
fn run_context(rest: &[String]) -> i32 {
    let mut name: Option<String> = None;
    let mut file: Option<String> = None;
    let mut depth: usize = 1;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--root" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev context: --root requires a directory argument");
                    return 2;
                }
                i += 2; // skip the value
            }
            "--file" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev context: --file requires a path argument");
                    return 2;
                }
                file = Some(rest[i + 1].clone());
                i += 2;
            }
            "--depth" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev context: --depth requires a number argument");
                    return 2;
                }
                match rest[i + 1].parse::<usize>() {
                    Ok(d) if d >= 1 => depth = d,
                    _ => {
                        eprintln!(
                            "rfs-dev context: --depth expects a positive integer, got `{}`",
                            rest[i + 1]
                        );
                        return 2;
                    }
                }
                i += 2;
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev context: unknown flag `{other}`");
                return 2;
            }
            other => {
                if name.is_some() {
                    eprintln!(
                        "rfs-dev context: takes exactly one positional argument (the item name)"
                    );
                    return 2;
                }
                name = Some(other.to_string());
                i += 1;
            }
        }
    }
    let Some(name) = name else {
        eprintln!("rfs-dev context: missing the item name");
        eprintln!("usage: rfs-dev context <name> [--file F] [--depth N] [--root DIR]");
        return 2;
    };
    if !name.chars().all(|c| c == '_' || c.is_ascii_alphanumeric()) || name.is_empty() {
        eprintln!("rfs-dev context: `{name}` is not a bare item name (use --file to disambiguate, never a full identifier)");
        return 2;
    }
    let root = match resolve_root(rest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rfs-dev context: {e}");
            return 2;
        }
    };
    match rfs_dev::context::run(&root, &name, file.as_deref(), depth) {
        Ok(out) => {
            // render() ends with exactly one trailing newline - print! keeps
            // stdout byte-identical to the template (the output contract).
            print!("{out}");
            0
        }
        Err(e) => {
            eprintln!("rfs-dev context: {e}");
            1
        }
    }
}
fn parse_edit_args(
    full: &str,
    rest: &[String],
    n_positional: usize,
    positional_names: &str,
) -> Option<(Vec<String>, std::path::PathBuf, bool)> {
    let mut positionals = vec![];
    let mut dry_run = false;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--root" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev {full}: --root requires a directory argument");
                    return None;
                }
                i += 2; // skip the value
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev {full}: unknown flag `{other}`");
                return None;
            }
            _ => {
                positionals.push(rest[i].clone());
                i += 1;
            }
        }
    }
    if positionals.len() != n_positional {
        eprintln!("rfs-dev {full}: expects exactly {n_positional} positional argument(s): {positional_names}");
        return None;
    }
    let root = resolve_root(rest).ok()?;
    Some((positionals, root, dry_run))
}

/// `edit-rename <old> <new> [--root DIR] [--dry-run]` (task 5.3): syn-parsed
/// rename of a fn/type across all Rust files; strings and comments carrying
/// the old name survive untouched.
fn run_edit_rename(rest: &[String]) -> i32 {
    let Some((pos, root, dry_run)) = parse_edit_args("edit-rename", rest, 2, "<old> <new>") else {
        return 2;
    };
    let (old, new) = (&pos[0], &pos[1]);
    for name in [old, new] {
        if !is_bare_item_name(name) {
            eprintln!("rfs-dev edit-rename: `{name}` is not a bare item name (letters, digits, underscore)");
            return 2;
        }
    }
    match rfs_dev::edit::rename(&root, old, new, dry_run) {
        Ok((plan, changed)) => {
            for line in &plan {
                println!("plan: {line}");
            }
            if plan.is_empty() {
                println!("no occurrences - nothing to do");
            } else if dry_run {
                println!("dry-run: {} file(s) would change", changed.len());
            } else {
                println!("applied: {} file(s) changed", changed.len());
            }
            0
        }
        Err(e) => {
            eprintln!("rfs-dev edit-rename: {e}");
            1
        }
    }
}

/// `edit-move <from> <to> [--root DIR] [--dry-run]` (task 5.3): move a module
/// file and rewrite every import site to the new path.
fn run_edit_move(rest: &[String]) -> i32 {
    let Some((pos, root, dry_run)) = parse_edit_args("edit-move", rest, 2, "<from> <to>") else {
        return 2;
    };
    match rfs_dev::edit::move_file(&root, &pos[0], &pos[1], dry_run) {
        Ok((plan, changed)) => {
            for line in &plan {
                println!("plan: {line}");
            }
            if dry_run {
                println!("dry-run: {} change(s) would be made", changed.len());
            } else {
                println!("applied: {} change(s) made", changed.len());
            }
            0
        }
        Err(e) => {
            eprintln!("rfs-dev edit-move: {e}");
            1
        }
    }
}

/// `edit-replace-item <file> <item> --new-from FILE [--root DIR] [--dry-run]`
/// (task 5.3): replace one item by name with the content of a file.
fn run_edit_replace_item(rest: &[String]) -> i32 {
    let mut new_from: Option<String> = None;
    let mut positionals = vec![];
    let mut dry_run = false;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--root" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev edit-replace-item: --root requires a directory argument");
                    return 2;
                }
                i += 2; // skip the value
            }
            "--new-from" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev edit-replace-item: --new-from requires a file argument");
                    return 2;
                }
                if new_from.is_some() {
                    eprintln!("rfs-dev edit-replace-item: --new-from given twice");
                    return 2;
                }
                new_from = Some(rest[i + 1].clone());
                i += 2; // skip the value
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev edit-replace-item: unknown flag `{other}`");
                return 2;
            }
            _ => {
                positionals.push(rest[i].clone());
                i += 1;
            }
        }
    }
    if positionals.len() != 2 {
        eprintln!(
            "rfs-dev edit-replace-item: expects exactly 2 positional argument(s): <file> <item>"
        );
        return 2;
    }
    let Some(new_from) = new_from else {
        eprintln!("rfs-dev edit-replace-item: --new-from FILE is required");
        return 2;
    };
    if !is_bare_item_name(&positionals[1]) {
        eprintln!(
            "rfs-dev edit-replace-item: `{}` is not a bare item name (letters, digits, underscore)",
            positionals[1]
        );
        return 2;
    }
    let root = match resolve_root(rest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rfs-dev edit-replace-item: {e}");
            return 2;
        }
    };
    match rfs_dev::edit::replace_item(&root, &positionals[0], &positionals[1], &new_from, dry_run) {
        Ok((plan, changed)) => {
            for line in &plan {
                println!("plan: {line}");
            }
            if dry_run {
                println!("dry-run: {} file(s) would change", changed.len());
            } else {
                println!("applied: {} file(s) changed", changed.len());
            }
            0
        }
        Err(e) => {
            eprintln!("rfs-dev edit-replace-item: {e}");
            1
        }
    }
}

/// `inventory audit [--root DIR]` / `inventory add|update|remove ...` -
/// structure-inventory row operations + the string-masked completeness audit
/// (task 5.5). Read-only `audit` prints per-file diffs and exits 1 on any; the
/// mutating row ops are plan-first with `--dry-run`.
fn run_inventory(full: &str, rest: &[String]) -> i32 {
    let sub = full.split_whitespace().nth(1).unwrap_or("");
    let mut dry_run = false;
    let mut doc: Option<String> = None;
    let mut kind: Option<String> = None;
    let mut id: Option<String> = None;
    let mut set: Option<String> = None;
    let mut depends = String::new();
    let mut component = String::new();
    let mut functionality = String::new();
    let mut description = String::new();
    let mut documented = String::new();
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--root" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev {full}: --root requires a directory argument");
                    return 2;
                }
                i += 2; // skip the value
            }
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            flag @ ("--doc" | "--kind" | "--id" | "--set" | "--depends" | "--component"
            | "--functionality" | "--description" | "--documented") => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev {full}: {flag} requires a value");
                    return 2;
                }
                let v = rest[i + 1].clone();
                match flag {
                    "--doc" => {
                        if !matches!(v.as_str(), "SRC" | "TEST") {
                            eprintln!("rfs-dev {full}: --doc must be SRC or TEST, got `{v}`");
                            return 2;
                        }
                        doc = Some(v);
                    }
                    "--kind" => {
                        if !matches!(v.as_str(), "fn" | "type") {
                            eprintln!("rfs-dev {full}: --kind must be fn or type, got `{v}`");
                            return 2;
                        }
                        kind = Some(v);
                    }
                    "--id" => id = Some(v),
                    "--set" => set = Some(v),
                    "--depends" => depends = v,
                    "--component" => component = v,
                    "--functionality" => functionality = v,
                    "--description" => description = v,
                    _ => documented = v,
                }
                i += 2;
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev {full}: unknown flag `{other}`");
                return 2;
            }
            _ => {
                eprintln!(
                    "rfs-dev {full}: unexpected positional argument `{}`",
                    rest[i]
                );
                return 2;
            }
        }
    }
    let root = match resolve_root(rest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rfs-dev {full}: {e}");
            return 2;
        }
    };
    // audit is read-only and takes no row arguments.
    if sub == "audit" {
        if doc.is_some() || kind.is_some() || id.is_some() || set.is_some() {
            eprintln!("rfs-dev inventory audit: takes only [--root DIR]");
            return 2;
        }
        match rfs_dev::inventory::audit(&root) {
            Ok((diffs, src_fn, src_ty, test_fn, test_ty)) => {
                if diffs.is_empty() {
                    println!(
                        "EXACT: SRC {} fns / {} types, TEST {} fns / {} types - zero per-file diffs",
                        src_fn, src_ty, test_fn, test_ty
                    );
                    return 0;
                }
                for d in &diffs {
                    if !d.missing.is_empty() {
                        println!(
                            "{} [{}] missing (not inventoried): {}",
                            d.file,
                            d.kind,
                            d.missing.join(", ")
                        );
                    }
                    if !d.stale.is_empty() {
                        println!(
                            "{} [{}] stale (no source item): {}",
                            d.file,
                            d.kind,
                            d.stale.join(", ")
                        );
                    }
                }
                return 1;
            }
            Err(e) => {
                eprintln!("rfs-dev inventory audit: {e}");
                return 1;
            }
        }
    }
    // add / update / remove require --doc, --kind, --id.
    let Some(doc) = doc else {
        eprintln!("rfs-dev {full}: --doc SRC|TEST is required");
        return 2;
    };
    let Some(kind) = kind else {
        eprintln!("rfs-dev {full}: --kind fn|type is required");
        return 2;
    };
    let Some(id) = id else {
        eprintln!("rfs-dev {full}: --id <file.rs::name> is required");
        return 2;
    };
    let result = match sub {
        "add" => {
            let row = match rfs_dev::inventory::build_row(
                &id,
                &depends,
                &component,
                &functionality,
                &description,
                &documented,
            ) {
                Ok(r) => r,
                Err(e) => {
                    eprintln!("rfs-dev {full}: {e}");
                    return 2;
                }
            };
            rfs_dev::inventory::add_row(&root, &doc, &kind, &row, dry_run)
        }
        "update" => {
            let Some(spec) = set else {
                eprintln!("rfs-dev inventory update: --set COLUMN=VALUE is required");
                return 2;
            };
            let Some((col, val)) = spec.split_once('=') else {
                eprintln!("rfs-dev inventory update: --set expects COLUMN=VALUE");
                return 2;
            };
            rfs_dev::inventory::update_row(&root, &doc, &kind, &id, col, val, dry_run)
        }
        "remove" => rfs_dev::inventory::remove_row(&root, &doc, &kind, &id, dry_run),
        _ => unreachable!("dispatch validated the subverb"),
    };
    match result {
        Ok(op) => {
            for line in &op.plan {
                println!("plan: {line}");
            }
            if dry_run {
                println!("dry-run: no files written");
            } else {
                println!("applied");
            }
            0
        }
        Err(e) => {
            eprintln!("rfs-dev {full}: {e}");
            1
        }
    }
}

/// `calls extract <commands.md> --out FILE` (task 5.6): the Rust port of
/// extract_calls.py - one JSON row per recorded call + the reconciliation
/// summary. Read-only over the input; writes only the `--out` inventory.
fn run_calls_extract(rest: &[String]) -> i32 {
    let mut positionals = vec![];
    let mut out: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--out" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev calls extract: --out requires a file argument");
                    return 2;
                }
                if out.is_some() {
                    eprintln!("rfs-dev calls extract: --out given twice");
                    return 2;
                }
                out = Some(rest[i + 1].clone());
                i += 2; // skip the value
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev calls extract: unknown flag `{other}`");
                return 2;
            }
            _ => {
                positionals.push(rest[i].clone());
                i += 1;
            }
        }
    }
    if positionals.len() != 1 {
        eprintln!("rfs-dev calls extract: expects exactly 1 positional argument: <commands.md>");
        return 2;
    }
    let Some(out) = out else {
        eprintln!("rfs-dev calls extract: --out FILE is required");
        return 2;
    };
    match rfs_dev::calls::run_extract(
        std::path::Path::new(&positionals[0]),
        std::path::Path::new(&out),
    ) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(e) => {
            eprintln!("rfs-dev calls extract: {e}");
            1
        }
    }
}

/// `calls classify <calls.jsonl> --kind K` (task 5.6): the Rust port of
/// classify_calls.py - one TSV per call kind under the analysis dir. Read-only
/// over the inventory; writes only the `<kind>_categories.tsv` output.
fn run_calls_classify(rest: &[String]) -> i32 {
    let mut positionals = vec![];
    let mut kind: Option<String> = None;
    let mut out: Option<String> = None;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--kind" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev calls classify: --kind requires a value");
                    return 2;
                }
                let v = rest[i + 1].clone();
                if !matches!(v.as_str(), "terminal" | "edit_file" | "python" | "misc") {
                    eprintln!("rfs-dev calls classify: --kind must be terminal | edit_file | python | misc, got `{v}`");
                    return 2;
                }
                if kind.is_some() {
                    eprintln!("rfs-dev calls classify: --kind given twice");
                    return 2;
                }
                kind = Some(v);
                i += 2; // skip the value
            }
            "--out" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev calls classify: --out requires a file argument");
                    return 2;
                }
                if out.is_some() {
                    eprintln!("rfs-dev calls classify: --out given twice");
                    return 2;
                }
                out = Some(rest[i + 1].clone());
                i += 2; // skip the value
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev calls classify: unknown flag `{other}`");
                return 2;
            }
            _ => {
                positionals.push(rest[i].clone());
                i += 1;
            }
        }
    }
    if positionals.len() != 1 {
        eprintln!("rfs-dev calls classify: expects exactly 1 positional argument: <calls.jsonl>");
        return 2;
    }
    let Some(kind) = kind else {
        eprintln!("rfs-dev calls classify: --kind K is required");
        return 2;
    };
    // Default output mirrors the python script: <kind>_categories.tsv next to
    // the analysis scripts; --out overrides for the port-equality test.
    let out_path = match out {
        Some(p) => p,
        None => format!("{kind}_categories.tsv"),
    };
    match rfs_dev::calls::run_classify(
        std::path::Path::new(&positionals[0]),
        &kind,
        std::path::Path::new(&out_path),
    ) {
        Ok(text) => {
            print!("{text}");
            0
        }
        Err(e) => {
            eprintln!("rfs-dev calls classify: {e}");
            1
        }
    }
}

fn is_bare_item_name(s: &str) -> bool {
    !s.is_empty() && s.chars().all(|c| c == '_' || c.is_ascii_alphanumeric())
}

/// `doc-section <file|all> <heading...> [--root DIR]` (task 5.4): print the
/// content of the section addressed by the heading path. Each positional
/// after the file is ONE heading element (quote multi-word headings); the
/// path walks from a top-level match into strictly deeper children.
fn run_doc_section(rest: &[String]) -> i32 {
    let mut positionals = vec![];
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--root" => {
                if i + 1 >= rest.len() {
                    eprintln!("rfs-dev doc-section: --root requires a directory argument");
                    return 2;
                }
                i += 2; // skip the value
            }
            other if other.starts_with("--") => {
                eprintln!("rfs-dev doc-section: unknown flag `{other}`");
                return 2;
            }
            _ => {
                positionals.push(rest[i].clone());
                i += 1;
            }
        }
    }
    if positionals.len() < 2 {
        eprintln!(
            "rfs-dev doc-section: expects a file (or `all`) and at least one heading element"
        );
        eprintln!("usage: rfs-dev doc-section <file|all> <heading...> [--root DIR]");
        return 2;
    }
    let root = match resolve_root(rest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rfs-dev doc-section: {e}");
            return 2;
        }
    };
    let target = &positionals[0];
    let path: Vec<String> = positionals[1..].to_vec();
    if target == "all" {
        let matches = rfs_dev::doc_section::search_all(&root, &path);
        if matches.is_empty() {
            eprintln!(
                "rfs-dev doc-section: no section matching `{}` in any docs file",
                path.join(" > ")
            );
            return 1;
        }
        for (i, (file, out)) in matches.iter().enumerate() {
            if i > 0 {
                println!();
            }
            println!("{file}: {}", path.join(" > "));
            // out ends with exactly one newline - print! keeps stdout exact.
            print!("{out}");
        }
        return 0;
    }
    let file_path = root.join(target);
    let content = match std::fs::read_to_string(&file_path) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("rfs-dev doc-section: {target}: cannot read: {e}");
            return 1;
        }
    };
    match rfs_dev::doc_section::doc_section(&content, &path) {
        Ok(Some(out)) => {
            print!("{out}");
            0
        }
        Ok(None) => {
            eprintln!(
                "rfs-dev doc-section: no section matching `{}` in {target}",
                path.join(" > ")
            );
            1
        }
        Err(e) => {
            eprintln!("rfs-dev doc-section: {target}: {e}");
            1
        }
    }
}

fn run_sync(full: &str, rest: &[String]) -> i32 {
    let mut dry_run = false;
    let mut maps: Vec<(String, String)> = vec![];
    let mut positional_ok = true;
    let mut i = 0;
    while i < rest.len() {
        match rest[i].as_str() {
            "--root" | "--map" => i += 2,
            "--dry-run" => {
                dry_run = true;
                i += 1;
            }
            other => {
                if other.starts_with("--") {
                    eprintln!("rfs-dev {full}: unknown flag `{other}`");
                    return 2;
                }
                positional_ok = false;
                i += 1;
            }
        }
    }
    if !positional_ok {
        eprintln!("rfs-dev {full}: takes no positional arguments");
        return 2;
    }
    let root = match resolve_root(rest) {
        Ok(r) => r,
        Err(e) => {
            eprintln!("rfs-dev {full}: {e}");
            return 2;
        }
    };
    if full == "sync-check" {
        let problems = rfs_dev::sync::check(&root);
        for p in &problems {
            println!("{p}");
        }
        return if problems.is_empty() { 0 } else { 1 };
    }
    // sync-fix
    let mut i = 0;
    while i < rest.len() {
        if rest[i] == "--map" && i + 1 < rest.len() {
            match rest[i + 1].split_once('=') {
                Some((old, new)) if !old.is_empty() && !new.is_empty() => {
                    maps.push((old.to_string(), new.to_string()));
                }
                _ => {
                    eprintln!("rfs-dev sync-fix: --map expects OLD=NEW");
                    return 2;
                }
            }
        }
        i += 1;
    }
    let (plan, changed) = rfs_dev::sync::fix(&root, &maps, dry_run);
    for line in &plan {
        println!("{line}");
    }
    if plan.is_empty() {
        println!("no stale references match the given mappings - nothing to do");
    } else if dry_run {
        println!("dry-run: {} file(s) would change", changed.len());
    } else {
        println!("applied: {} file(s) changed", changed.len());
    }
    let remaining = rfs_dev::sync::check(&root);
    for p in &remaining {
        println!("remaining: {p}");
    }
    if remaining.is_empty() {
        0
    } else {
        1
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn args(s: &str) -> Vec<String> {
        s.split_whitespace().map(str::to_string).collect()
    }

    #[test]
    fn no_args_is_usage_exit_2() {
        assert_eq!(dispatch(&[]), 2);
    }

    #[test]
    fn unknown_subcommand_is_usage_exit_2() {
        assert_eq!(dispatch(&args("frobnicate")), 2);
    }

    #[test]
    fn grouped_subcommands_require_a_valid_subverb() {
        assert_eq!(dispatch(&args("inventory")), 2);
        assert_eq!(dispatch(&args("inventory bogus")), 2);
        assert_eq!(dispatch(&args("calls")), 2);
        assert_eq!(dispatch(&args("calls bogus")), 2);
    }

    #[test]
    fn calls_subverbs_without_required_args_are_usage_exit_2() {
        // Every subcommand is now implemented (5.1-5.6). The calls subverbs
        // with no arguments are usage errors (missing positional / --out /
        // --kind), not stubs - exit 2 either way, but for a different reason.
        assert_eq!(dispatch(&args("calls extract")), 2);
        assert_eq!(dispatch(&args("calls classify")), 2);
    }
}
