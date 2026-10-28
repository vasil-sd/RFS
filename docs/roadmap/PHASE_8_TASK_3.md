# Phase 8, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/CODE_MAP.md` (Class A): a file-by-file map of all 71
hand-written source files. Each entry states the file's responsibility, its
key items (stable identifiers as doc→code spans), and the doc links both ways
- doc→code via the item spans in the entry, code→doc via the fragments the
file's existing `//! Docs:` line names (named in the entry). Subtasks 3.1-3.4
fill the four area sections in order.

## Context

- **Skeleton state (from task 1.2)**: `docs/dev/CODE_MAP.md` exists with four
  stable fragment anchors, each owned by its area's primary file(s):
  `dev:code-map-frontend` (the 6 root files), `dev:code-map-codegen`
  (`src/codegen.rs`), `dev:code-map-cli` (`src/main.rs`),
  `dev:code-map-workspace` (`crates/rfs-build/src/lib.rs`). The anchors and
  headings stay byte-stable; this task replaces the four TBD bodies.
- **Entry format**: one bullet per file - responsibility (one line), key items
  as inline code spans `path/file.rs::item` (gate-checked, stable names only),
  and the deep-dive fragment(s) the file's `//! Docs:` line names. No markdown
  links (repo docs use backticked paths, per the README style).
- **Count correction (from task 1.10)**: the intro paragraph still says "72
  hand-written source files: `src/` (49), `crates/` (18)" - the true counts
  are 71 / 17 (`crates/examples/fuzz_rust/fuzz/fuzz_targets/arq.rs` is
  build-generated + git-ignored). Fixed in 3.1 as part of touching this file.
- **Parity**: 3.2 covers the Rust and C backends at equal depth (backend
  parity principle applied to documentation).
- **3.4 note**: plan line says "`crates/` (18)" = the 17 tracked files + the
  generated arq.rs; the section maps the 17 tracked files and mentions arq.rs
  as a build artifact, not as an entry.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.4 / 1.8 / 1.9 / 1.10 | the four area sections' owner files are in the manifest (per plan line dependencies) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 4.x-13.x | DEPENDENCIES graphs and deep dives cross-link into these entries; IDs stable from now |

## Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/CODE_MAP.md` | change | replace the four TBD section bodies with file-by-file entries (3.1: frontend 13; 3.2: codegen+backends 18; 3.3: CLI/diagrams+bins/training 18; 3.4: crates+fuzz 22 tracked) |

## Planned actions

### Action 3.1: frontend area (13 files)

Fill `dev:code-map-frontend`: root (6), parser (2), validator+IR (5). Also fix
the stale 72/18 intro counts to 71/17 per the task 1.10 correction. Run the
gate.

### Action 3.2: codegen core + backends (18 files)

Fill `dev:code-map-codegen`: codegen core (4) + Rust backend (5) + C backend
(4); both backends at equal depth. Run the gate.

### Action 3.3: CLI + diagrams area (18 files)

Fill `dev:code-map-cli`: CLI/diagrams (13) + bins/training (5). Run the gate.

### Action 3.4: workspace crates + fuzz (final)

Fill `dev:code-map-workspace`: the 17 tracked crate files + the 5 fuzz
targets; arq.rs mentioned as build artifact. Run the full gate.

## Task exit criteria

- [ ] All four sections filled; no TBD placeholders remain in CODE_MAP.md;
      intro counts corrected to 71/17.
- [ ] Every key-item span resolves (gate check 1); anchors/headings unchanged.
- [ ] Rust and C backends at equal depth (3.2).
- [ ] Full verification green: dev_docs_test, docs_md_test, cargo build/test/
      clippy/fmt (+ --workspace).

## Non-goals

- No DEPENDENCIES / DECISIONS content (tasks 4-5.x); no Class B changes; no
  source edits.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
