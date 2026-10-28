# Phase 8, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/DEPENDENCIES.md` (Class A): the file-level dependency graph
(subtask 4.1) and function-level dependency/call graphs for the core modules
(subtasks 4.2-4.5: parser+validator, codegen core+Rust backend, C backend+MC
engine, monitor compiler). Every graph is mermaid (validated by the docs
gate); every prose statement carries a doc→code reference.

## Context

- **Skeleton state (from task 1.2)**: `docs/dev/DEPENDENCIES.md` exists with
  one anchored fragment - `dev:deps-file-level` (owner `src/lib.rs`) - and
  four plain-heading sections for the function-level graphs. The planned
  function-level fragments (from the task 1 Context table) and their owners:
  `dev:deps-fn-parser` (`src/parser.rs`), `dev:deps-fn-validator`
  (`src/validator.rs`), `dev:deps-fn-codegen-rust` (`src/codegen.rs`),
  `dev:deps-fn-backend-c` (`src/backends/c.rs`), `dev:deps-fn-mc-engine`
  (`src/mc_plan.rs`), `dev:deps-fn-monitor` (`src/monitor.rs`).
- **ORPHAN RULE (critical)**: adding a fragment anchor makes the gate demand
  an owner - each subtask that adds an anchor MUST also extend the owner
  file's `//! Docs:` line with the new fragment ID (single line; the parser
  only reads IDs off the `//! Docs:` line itself).
- **File-level ground truth (derived 2026-10-28 from `use crate::` /
  `use crate::{...}` / `use super::` statements + verified inline
  `crate::x::` paths, doc-comment mentions excluded)**: see the edge list in
  Action 4.1 below. Workspace edges: rfs-build and rfs-adapters are
  standalone (no rfs-crate dep); examples depend on rfs-adapters (+ rfs-build
  as build-dep); the fuzz crate depends on the root crate.
- **Mermaid constraints**: `docs_md_test` renders every mermaid block; the
  file-level graph is split into per-area subgraphs (plan line 4.1) to stay
  within layout limits. Fenced blocks are exempt from doc→code reference
  resolution, so file names inside diagram labels need no backticks.
- **Count correction (from task 1.10)**: the skeleton's TBD says "all 72
  hand-written files" - the true count is 71 (arq.rs is build-generated).
  Fixed in 4.1.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.2 / 1.4 / 1.6 / 1.8 | DEPENDENCIES skeleton + the owner files of the fn-* fragments in the manifest (per plan line dependencies) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 6.x-13.x | ALGORITHMS deep dives cross-link into the function-level fragments; IDs stable from now |

## Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/DEPENDENCIES.md` | change | replace the five TBD sections (4.1: file-level graph; 4.2-4.5: function-level graphs + their anchors) |
| `src/parser.rs`, `src/validator.rs`, `src/codegen.rs`, `src/backends/c.rs`, `src/mc_plan.rs`, `src/monitor.rs` | change | `//! Docs:` line extended with the new fn-* fragment each subtask anchors (doc comments only) |

## Planned actions

### Action 4.1: file-level dependency graph

Replace the `dev:deps-file-level` TBD: one mermaid flowchart of all 71
hand-written files, per-area subgraphs (root, parser, validator+IR, codegen
core, backends, MC, monitor, CLI, diagrams, bins+training, workspace crates,
fuzz targets), edges = verified module dependencies (A --> B reads "A uses
B"). Fix the stale 72 count in the TBD text. Run the gate.

### Action 4.2: function-level graphs - parser + validator

Add the `dev:deps-fn-parser` + `dev:deps-fn-validator` anchors and function
call graphs for `src/parser.rs` and `src/validator.rs`; extend both owners'
`//! Docs:` lines. Run the gate.

### Action 4.3: function-level graphs - codegen core + Rust backend

Add the `dev:deps-fn-codegen-rust` anchor and function graphs for
`src/codegen.rs` + the Rust backend files; extend `src/codegen.rs`'s
`//! Docs:` line. Run the gate.

### Action 4.4: function-level graphs - C backend + MC engine

Add the `dev:deps-fn-backend-c` + `dev:deps-fn-mc-engine` anchors and
function graphs for the C backend + `mc_plan`/`ref_mc` (+ SMT path);
extend `src/backends/c.rs`'s and `src/mc_plan.rs`'s `//! Docs:` lines.
Run the gate.

### Action 4.5: function-level graph - monitor compiler

Add the `dev:deps-fn-monitor` anchor and the monitor-compiler function graph
(+ SMT path where relevant); extend `src/monitor.rs`'s `//! Docs:` line.
Run the full gate.

## Task exit criteria

- [ ] All five sections filled; no TBD placeholders remain; stale 72 count
      fixed.
- [ ] All six fragments anchored AND owned (gate check 4); owner `//! Docs:`
      lines extended in the same subtask as their anchor.
- [ ] Every mermaid block renders under `docs_md_test`; every prose
      statement carries a resolving doc→code reference.
- [ ] Full verification green: dev_docs_test, docs_md_test, cargo build/test/
      clippy/fmt (+ --workspace).

## Non-goals

- No DECISIONS / ALGORITHMS content (tasks 5-6.x); no Class B changes; no
  non-doc-comment source edits.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
