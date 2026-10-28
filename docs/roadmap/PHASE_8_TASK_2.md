# Phase 8, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/ARCHITECTURE.md` (Class A): subtask 2.1 writes the pipeline
overview + mermaid component diagram; subtask 2.2 adds the functionality
hierarchy tree + relationship map between components. Every technical
statement carries a doc→code reference that resolves under the sync gate.

## Context

- **Skeleton state (from task 1.2)**: `docs/dev/ARCHITECTURE.md` exists with
  three stable fragment anchors, all owned by `src/lib.rs`:
  `dev:architecture-pipeline`, `dev:architecture-components`,
  `dev:architecture-hierarchy` - each currently a TBD placeholder naming this
  task's subtasks. This task replaces the placeholders; the anchor lines and
  headings stay byte-stable (fragment IDs are frozen by task 1).
- **Class A rules**: no line budget, but the file enters `docs_md_test`
  automatically (Markdown quality + mermaid rendering via the mdr stack) -
  every mermaid block must render. doc→code references are inline code spans
  `path/file.rs[::item]` (repo-rooted, stable item names, NEVER line numbers);
  fenced code blocks (incl. mermaid) are EXEMPT from reference resolution, so
  file names inside diagram labels need no backticks and create no obligation.
- **Content requirements (PHASE_8.md)**: pipeline overview (spec → parse →
  validate → IR → codegen/MC/diagram), component diagram (mermaid),
  functionality hierarchy tree, relationship map between components.
- **Traceability bar**: every statement describing implemented behavior links
  to the implementing file/item; design rationale may link `DESIGN.md`
  sections directly for now (`docs/dev/DECISIONS.md` is task 5.x).
- **Pipeline ground truth** (verify against source while writing): entry
  `src/main.rs` dispatches CLI commands; spec input `.rfs` text or IR JSON;
  parsing `src/parser.rs`; validation `src/validator.rs`; IR `src/ir.rs`
  (+ `src/json_rw.rs`, `src/spec_hash.rs`); printing back to RFS
  `src/printer.rs`; codegen core `src/codegen.rs` + shared helpers
  (`codegen_fsm.rs`, `codegen_mc.rs`, `codegen_test.rs`) with the backend
  plugin registry; Rust/C backends under `src/backends/`; MC engine plan
  `src/mc_plan.rs` (+ SMT path `src/smt_plan.rs`/`src/smt_emit.rs`, reference
  checker `src/ref_mc.rs`); monitor compiler `src/monitor.rs`; diagram plugin
  system `src/diagram.rs` + `src/diagrams/`; external-tool layer
  `src/ext_tool.rs`; frontend conversion helpers `src/frontend.rs`.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.2 | ARCHITECTURE skeleton with the three fragment anchors (owner `src/lib.rs`) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 3-13 (all later content tasks) | cross-link into these fragments; the IDs are stable from now |

## Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/ARCHITECTURE.md` | change | replace the three TBD placeholders with real content (2.1: pipeline overview + component diagram; 2.2: functionality hierarchy + relationship map) |

## Planned actions

### Action 2.1: pipeline overview + component diagram

Replace the `dev:architecture-pipeline` and `dev:architecture-components`
TBDs: a stage-by-stage pipeline write-up (`.rfs`/`.json` input → parse →
validate → IR → codegen / model-checking / test / diagram targets) with a
doc→code reference at every stage, plus a mermaid component diagram of the
toolchain. Run the gate.

### Action 2.2: functionality hierarchy + relationship map

Replace the `dev:architecture-hierarchy` TBD: functionality hierarchy tree
(mermaid) + prose relationship map between components (who calls whom, what
is shared, where parity constraints bind). Run the full gate.

## Task exit criteria

- [ ] All three fragments filled; no TBD placeholders remain in ARCHITECTURE.md.
- [ ] Every technical statement carries a resolving doc→code reference
      (gate check 1); fragment anchors/headings unchanged.
- [ ] Mermaid blocks render under `docs_md_test`.
- [ ] Full verification green: dev_docs_test, docs_md_test, cargo build/test/
      clippy/fmt (+ --workspace).

## Non-goals

- No CODE_MAP / DEPENDENCIES / DECISIONS content (tasks 3-5.x); no Class B
  changes; no source edits.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
