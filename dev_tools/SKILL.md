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
./target/debug/rfs-dev --help   # or: cargo run -q -p rfs-dev -- <tool>
```

CLI conventions for every tool: compact structured stdout (tables/JSON, never raw
file dumps); exit `0` success / `1` operational failure / `2` usage error; `--root`
defaults to the repo root; mutating tools print the exact edit plan first and accept
`--dry-run`. Invoke via `cargo run -q -p rfs-dev -- <tool> ...` (no build step needed
if already built) or the built binary directly.

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
rfs-dev inventory add --doc SRC --file src/x.rs --item 'fn foo' ...   # add a row
rfs-dev inventory update --doc TEST --file tests/y.rs --item 'fn bar' ... 
rfs-dev inventory remove --doc SRC --file src/x.rs --item 'fn gone'
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
