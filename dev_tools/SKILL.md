# SKILL.md - using `rfs-dev` (the LLM-assistant maintenance skill)

**Read this at session start, after START.md.** It is the operating guide for the
`dev_tools/` crate (binary `rfs-dev`). The single governing rule:

> For any of the operations below, reach for `rfs-dev` FIRST - it is deterministic,
> plan-first, and immune to the string/comment "phantom" failures that make hand
> `grep`/`sed`/`perl` edits unreliable. Fall back to raw shell only for the cases
> listed under *No tool* at the end.

`rfs-dev` is a workspace member, so it builds with the project toolchain and is on
the same activation as everything else (see BUILD.md "Building dev_tools" and
TOOLCHAIN.md). Verify it is present before relying on it:

```sh
export RUSTUP_HOME=/workspace/host/rustup CARGO_HOME=/workspace/host/.cargo PATH="$CARGO_HOME/bin:$PATH"
cd /workspace/host/RFS
cargo build -p rfs-dev          # builds the binary (fast; deps: syn + pulldown-cmark + regex)
./target/debug/rfs-dev --help   # full self-contained CLI reference (exit 0)
```

CLI conventions for every tool: compact structured stdout (tables/JSON, never raw
file dumps); exit `0` success / `1` operational failure / `2` usage error; `--root`
defaults to the repo root (walked up from the cwd to `tests/dev_docs_manifest.txt`);
mutating tools print the exact edit plan first and accept `--dry-run`. Invoke via
`cargo run -q -p rfs-dev -- <tool> ...` (no build step needed if already built) or
the built binary directly. `-h`/`--help` works at top level and after any
subcommand/subverb; the CLI reference below is complete, so you never need to read
the `rfs-dev` sources to learn its usage.

## CLI reference (complete)

### `sync-check`
```
usage: rfs-dev sync-check [--root DIR]
```
Read-only. Cross-checks every manifest-listed source file's `//! Docs:` pointer line
against the fragment definitions under docs/dev/ + docs/llm/ and prints one line per
stale/missing/orphan reference. Exit 0 when clean, 1 if any problem is listed.

### `sync-fix`
```
usage: rfs-dev sync-fix [--map OLD=NEW ...] [--dry-run] [--root DIR]
```
Rewrites stale doc references per the given mappings (repeatable `OLD=NEW` pairs)
across docs/dev/ + docs/llm/. Prints the exact edit plan first; `--dry-run` writes
nothing. After applying, re-runs the check and prints any remaining problems. Exit 0
only when nothing remains stale, 1 otherwise.

### `context`
```
usage: rfs-dev context <name> [--file F] [--depth N] [--root DIR]
```
One-command impact context for a fn/type: its inventory row, its bounded Depends
closure, the doc fragments that describe it, and related design decisions.
- `<name>` - bare item name (letters/digits/underscore), never a full identifier.
- `--file F` - restrict the search to one file (disambiguates same-named items).
- `--depth N` - widen the dependency closure (positive integer, default 1).
Read-only; the rendered template goes to stdout. Exit 1 on no match or ambiguity.

### `edit-rename`
```
usage: rfs-dev edit-rename <old> <new> [--dry-run] [--root DIR]
```
syn-parsed rename of a fn/type across all Rust files. String literals and comments
that merely contain the old name survive untouched (the "phantom" failure class).
Both arguments must be bare item names. Plan-first; `--dry-run` writes nothing.
Exit 1 if the symbol is missing or ambiguous.

### `edit-move`
```
usage: rfs-dev edit-move <from> <to> [--dry-run] [--root DIR]
```
Moves a module file and rewrites every import site to the new path. Plan-first;
`--dry-run` writes nothing. Exit 1 if the source file or its imports are not found.

### `edit-replace-item`
```
usage: rfs-dev edit-replace-item <file> <item> --new-from FILE [--dry-run] [--root DIR]
```
Replaces one item (fn/type) in `<file>` by name with the content of FILE. `<item>`
must be a bare item name; `--new-from` is required and given once. Plan-first;
`--dry-run` writes nothing. Exit 1 if the item is missing or ambiguous in `<file>`.

### `doc-section`
```
usage: rfs-dev doc-section <file|all> <heading...> [--root DIR]
```
Prints the content of the section addressed by the heading path. Each positional
after the file is ONE heading element (quote multi-word headings); the path walks
from a top-level match into strictly deeper children. `all` searches every docs file
and prefixes each hit with its file name. Read-only; exit 1 if no section matches.

### `inventory add|update|remove|audit`
```
usage: rfs-dev inventory audit [--root DIR]
       rfs-dev inventory add --doc SRC|TEST --kind fn|type --id <file.rs::name>
                             [--depends D] [--component C] [--functionality F]
                             [--description T] [--documented D] [--dry-run] [--root DIR]
       rfs-dev inventory update --doc SRC|TEST --kind fn|type --id <file.rs::name>
                                --set COLUMN=VALUE [--dry-run] [--root DIR]
       rfs-dev inventory remove --doc SRC|TEST --kind fn|type --id <file.rs::name>
                                [--dry-run] [--root DIR]
```
Maintains the `docs/llm/SRC_STRUCTURE.md` / `TEST_STRUCTURE.md` rows (never
hand-edit the tables).
- `audit` - read-only completeness check: extracts every fn/type identifier from
  source (strings masked) and diffs against the inventory rows per file. Prints
  `EXACT: ...` and exits 0 when clean; one line per file's missing/stale items and
  exit 1 otherwise. Takes no row arguments.
- `add` - inserts a new row, keeping file grouping (after the last row of the same
  file, else at the end of the table). `--id` is the full `<file>.rs::<name>`
  identifier; the optional columns fill the remaining cells, missing ones stay
  empty. Exit 1 if the row already exists or the target table is not found.
- `update` - replaces one cell of an existing row. COLUMN is one of
  depends|component|functionality|description|documented (the identifier column is
  immutable - a rename is a remove+add). Exit 1 if the row is absent or the column
  is unknown.
- `remove` - deletes the row with the given identifier. Exit 1 if absent.
The row ops are plan-first; `--dry-run` prints the plan and writes nothing.

### `calls extract` / `calls classify`
```
usage: rfs-dev calls extract <commands.md> --out FILE
       rfs-dev calls classify <calls.jsonl> --kind terminal|edit_file|python|misc [--out FILE]
```
The Rust ports of the python oracle scripts (must stay byte-identical to them).
`extract` parses the recorded tool-call log and writes one JSON row per call plus
the reconciliation summary to `--out` (required); read-only over the input.
`classify` categorizes the extracted calls and writes one TSV per kind; default
output is `<kind>_categories.tsv` in the current directory, `--out` overrides it.
Neither takes `--root`.

## The six tools - when to use each

### 1. `doc-section` - read a doc section by heading (replaces grep+sed range reads)

The largest single win: instead of `grep -n '^## ' FILE` then `sed -n 'a,bp'`, ask for
the section by its heading path.

```sh
rfs-dev doc-section KB.md 4.2                 # one section of a known file
rfs-dev doc-section DESIGN.md 5 5.10          # a range of sibling headings
rfs-dev doc-section all "production codegen"  # search EVERY docs file, prefix matches with the file name
```
Use it for KB.md / DESIGN.md / docs/dev / docs/llm / any markdown. It walks real
headings (pulldown-cmark), so it is robust to line-number drift that breaks sed ranges.

### 2. `context` - one-command impact context for a fn/type (replaces the grep+sed+read hunt)

Before editing any function or type, get its inventory row, its bounded Depends
closure, the doc fragments that describe it, and related design decisions in ONE call:

```sh
rfs-dev context build_smt_spec            # default depth 1
rfs-dev context parse --depth 2           # widen the dependency closure
```
This operationalizes the PROTOCOL.md impact-calculation rule: you see exactly what
depends on the item and where it is documented before you touch it.

### 3. `edit-rename` / `edit-move` / `edit-replace-item` - syn-parsed source edits

Never rename/move/replace a Rust item with `sed`/`perl` - those rewrite string
literals and comments that merely CONTAIN the pattern (the "phantom" failure class).
These tools parse with `syn` and touch only real use-declarations and path references:

```sh
rfs-dev edit-rename old_name new_name --dry-run     # rename a fn/type across all .rs
rfs-dev edit-move src/a.rs src/b/c.rs --dry-run     # move a file + rewrite every import site
rfs-dev edit-replace-item src/x.rs some_fn --new-from /tmp/new_fn.rs --dry-run
```
Always `--dry-run` first, review the plan, then run for real. Exit 1 if the symbol is
missing or ambiguous - that is the tool telling you the edit is unsafe.

### 4. `sync-check` / `sync-fix` - docs<->source reference re-syncing

After any rename/move/refactor, doc pointers and the `//! Docs:` lines go stale.
Check, then fix:

```sh
rfs-dev sync-check                 # read-only; lists every stale/missing/orphan pointer, exit 1 if any
rfs-dev sync-fix --dry-run         # print the exact edit plan
rfs-dev sync-fix                   # apply it
```
Run `sync-check` whenever `cargo test --test dev_docs_test` fails - the fix-list names
exactly what to repair instead of manual triage.

### 5. `inventory` - maintain the structure inventories + completeness audit

The `docs/llm/SRC_STRUCTURE.md` / `TEST_STRUCTURE.md` Depends maps must stay
mechanically valid. Use the row operations (never hand-edit the tables) and the audit:

```sh
rfs-dev inventory add --doc SRC --kind fn --id src/x.rs::foo --description '...'   # add a row
rfs-dev inventory update --doc TEST --kind fn --id tests/y.rs::bar --set documented='dev:example'
rfs-dev inventory remove --doc SRC --kind fn --id src/x.rs::gone
rfs-dev inventory audit          # string-masked completeness check; exit 1 on any source/inventory diff
```
`audit` is the Phase 10/11 completeness method: it extracts every fn/type identifier
from source (strings masked) and diffs against the inventory rows per file.

### 6. `calls extract` / `calls classify` - the analysis pipeline (maintenance of dev_tools itself)

Only needed when re-running the tool-call analysis on a new `commands.md` (see
`dev_tools/PROTOCOL.md`). It is the Rust port of the python oracle and must stay
byte-identical to it:

```sh
rfs-dev calls extract commands.md --out /tmp/calls.jsonl
rfs-dev calls classify /tmp/calls.jsonl --kind terminal
```

## No tool - use the normal mechanism instead

These operation classes deliberately have NO `rfs-dev` wrapper (wrapping them would
duplicate a better tool or add no determinism). Use the plain command:

- **build / test / run** -> `cargo ...` (cargo IS the build tool; see BUILD.md).
- **version control** -> `git ...` (the system of record).
- **environment probing** -> live shell (`command -v`, `--version`) - it differs per sandbox.
- **reading vendored dependency sources** -> read them directly (they change with dep versions; a cache would go stale).
- **creating brand-new files** (new docs, new source modules) -> this is design work, not a mechanical rewrite; write the file, then let `sync-check`/`inventory` keep references valid.
- **scratch file ops** (`/tmp` mv/rm/cp) -> plain shell.

## Session integration

The session-opening protocol in START.md builds `rfs-dev` if it is missing (it is a
workspace member, so `cargo build --workspace` covers it). From then on, default to
`doc-section` for reading docs, `context` before editing, the `edit-*` tools for source
rewrites, and `sync-check`/`inventory audit` as part of your verification - that is what
"use dev_tools extensively" means in practice.
