# Phase 10: Source structure analysis and refactoring preparation

## Goal
Produce a complete, dependency-annotated inventory of every function and every
data type in the project's source code (non-test files -> `SRC_STRUCTURE.md`,
test files -> `TEST_STRUCTURE.md`), classify them into proposed component and
functionality categories, and propose a new file-naming scheme and directory
tree layout for both sets. This is the preparation phase for a later
refactoring phase that will actually split/join/rename/move source files.
Functionality is NOT altered in this phase - it is analysis and proposal only.
Beyond preparing the refactor, the dependency-annotated tables are designed to
support fine-grained impact analysis of future source-code changes and to
define the scope of refactoring/editing tasks precisely.

## Scope
- `SRC_STRUCTURE.md` at the repo root with six sections:
  - `## All functions` - one row per function in every non-test source file
    (columns: Function, Depends, Component, Functionality, Short description,
    Documented in); Component/Functionality left blank during enumeration and
    filled after the categories are proposed.
  - `## All types` - one row per data type/structure/enum/trait/type-alias in
    non-test source files, with the identifier column named `Type`: `| Type |
    Depends | Component | Functionality | Short description | Documented in |`.
  - `## Components` - proposed structural component categories (table:
    name, depends on, description + rationale).
  - `## Functionality` - proposed cross-cutting functionality categories
    (same table shape).
  - File mapping table: | source file name | components | functionalities |
    description + rationale |.
  - Proposed new naming scheme + directory tree layout as a mermaid diagram.
- `TEST_STRUCTURE.md` at the repo root: the same six-section structure redone
  for test source files (integration suites, shared test helpers, inline
  `#[cfg(test)]` modules, workspace-crate tests, toolchain fuzz targets).
- Enumeration is chunked by file cluster (the full inventory is ~1,300
  non-test functions / ~240 types and ~950 test functions - too large for a
  single pass); each chunk is one planned task.
- `docs/roadmap/CURRENT_STATE.md` row 10 kept current (PLANNED -> IN PROGRESS
  -> COMPLETE).

## Inventory conventions (binding for all enumeration tasks)
- Identifier formats: functions `<file>::<function>`, types `<file>::<Type>`,
  file paths relative to the repo root (e.g. `src/parser.rs::parse_source`,
  `src/ir.rs::Ir`). The All functions table's identifier column is named
  `Function`; the All types table's is named `Type`.
- Depends column (BOTH tables, BOTH directions): comma-separated list of
  project identifiers - `<file>::<function>` AND `<file>::<Type>` - that the
  row depends on. A function row lists the functions it calls and the types it
  constructs/contains/matches; a type row lists the types it contains and the
  implementation functions its trait impls/methods delegate to. Cross-file and
  same-file alike; empty if none. Macro-generated call sites count once per
  distinct callee. Rationale: the tables must support fine-grained impact
  analysis of future source changes and precise scoping of refactoring/editing
  tasks - a missing edge direction makes that calculation incomplete.
- Documented in column: the `dev:` / `llm:` fragment names from the file's
  single `//! Docs:` pointer line (the 71-file manifest); blank for files
  without one (all test files except `tests/dev_docs_test.rs`), with the
  `docs/llm/TASK_MAP.md` suite entry noted where it exists.
- A function or type may belong to SEVERAL components and functionalities;
  multi-valued cells are comma-separated.
- Private (`fn`, no `pub`) items are included - the inventory is complete by
  construction, not public-API-only.

## Enumeration chunking strategy
Non-test source is split into 12 file-cluster chunks (tasks 2-13 of the plan),
sized so one chunk fits a single working session (~40-200 functions). Test
source is split into 11 chunks (tasks 20-30), including a dedicated chunk for
the 311 inline `#[cfg(test)]` functions inside `src/` files (split in two)
and one for workspace-crate tests + all five toolchain fuzz targets (their
  sole home - user decision at plan review). Boundary rule:
SRC_STRUCTURE.md covers `src/**`, `crates/*/src/**`, example-crate `src/` +
`build.rs`; TEST_STRUCTURE.md covers `tests/**`, inline `#[cfg(test)]`
modules, `crates/*/tests/**`, and `fuzz/fuzz_targets/**`.

## Deliverables
- `SRC_STRUCTURE.md` (repo root): all six sections fully filled - ~1,300
  function rows, ~240 type rows, component + functionality category tables,
  per-file mapping table, proposed naming scheme, proposed layout mermaid.
- `TEST_STRUCTURE.md` (repo root): same six-section structure for test source
  (~950 function rows), including its own proposed test-tree layout mermaid.
- Updated `docs/roadmap/CURRENT_STATE.md` row 10.
- `PHASE_10_STATUS.md` at close-out.

## Exit criteria
- [ ] `SRC_STRUCTURE.md` exists at the repo root; `## All functions` has
      exactly one row per non-test function (~1,250 rows - count re-verified against a fresh grep at close-out),
      `## All types` one row per non-test type (~237 rows); every row
      carries Depends, Short description, and Documented in (or a documented
      blank reason).
- [ ] `## Components` and `## Functionality` tables exist with name /
      depends-on / description+rationale columns; every function and type row
      has its Component and Functionality cells filled (multi-valued allowed).
- [ ] The file-mapping table covers EVERY non-test source file (one row per
      file: components, functionalities, description + rationale) and the
      proposed layout is drawn as a mermaid diagram that renders under the
      docs_md_test gate.
- [ ] `TEST_STRUCTURE.md` satisfies the same four criteria for test source
      files (its own category tables may reuse SRC categories plus test-
      specific ones; its own file-mapping table + mermaid layout).
- [ ] No source file was moved, renamed, split, joined, or functionally
      changed: `git diff --stat` over the phase shows only the two new .md
      files, roadmap bookkeeping, and TASKS.md.
- [ ] Full verification green: `cargo test` (535 baseline incl. docs_md_test
      5/5 parsing the two new files + dev_docs_test 9/9),
      `cargo clippy --all-targets`, `cargo fmt --check`.
- [ ] `CURRENT_STATE.md` row 10 -> COMPLETE with close-out date;
      `PHASE_10_STATUS.md` written; TASKS.md entry appended.

## Non-goals
- NO actual refactoring: no file moves/renames/splits/joins, no module-path
  changes, no Cargo.toml/workspace edits - that is a later phase consuming
  these two documents.
- No functionality, API, CLI, codegen-output, or test-behavior changes of any
  kind (docs-only phase).
- No restructuring of `docs/dev/` or `docs/llm/` content - the `//! Docs:`
  pointer lines and fragment anchors stay valid until the refactoring phase,
  which must re-sync them as part of its work.
- No changes to examples, training sets, or generated-code output.
- No visual comparison of the proposed layout against any external standard -
  the proposal is justified by the project's own component/functionality
  categories only.
