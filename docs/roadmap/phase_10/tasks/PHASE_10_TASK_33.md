# Phase 10, Task 33 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Fill the `## File mapping` table of TEST_STRUCTURE.md (one row per test source
file: components, functionalities, description + rationale incl. proposed new
path) and draw the proposed test-tree layout as a mermaid diagram in `##
Proposed layout`. Completes TEST_STRUCTURE.md - and with it the whole phase.

## Context
- Input: task 31's categories + task 32's classified rows. The naming scheme for
  tests follows the same conventions as SRC_STRUCTURE.md's task-17 section
  (snake_case, responsibility-based names) applied to the test tree.
- File mapping table columns exactly: `| Source file name | Components |
  Functionalities | Description + rationale |`. One row per test source file:
  the 24 `tests/*.rs` suites + 4 `tests/common/*.rs` + 3 fixtures + 2 crate-test
  files + 5 fuzz-target files (~38 rows). Inline `#[cfg(test)]` modules are NOT
  separate rows - they are covered by their host file's row (noted in the
description).
- Layout constraints: integration suites stay where cargo finds them
  (`tests/*.rs` = one binary per file); `tests/common/` stays a shared module;
  fixtures stay under `tests/fixtures/`; the detached root `fuzz/` workspace
  keeps its own layout (cargo-fuzz requirement) - the proposal may only rename
  files INSIDE it, never move the workspace; crate tests stay in their crates'
  `tests/` directories.
- The mermaid diagram shows the PROPOSED test tree: graph TD, subgraphs per
  group (suite families), file leaves, short labels. Must render under
  mermaid-rs-renderer (docs_md_test) - verify by running the gate.
- After this task BOTH inventory documents are complete and the phase exit
  criteria can be checked.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31. | Test categories proposal | per-file category assignments for the table cells |
| 32. | Test classification fill | the fully classified picture the mapping renders |
| 17. | SRC naming scheme | the conventions the test-tree proposal mirrors |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | `## File mapping` table filled (~38 rows) + `## Proposed layout` mermaid - document complete |

## Planned actions

### Fill the file mapping table
One row per test source file with components, functionalities, description +
rationale (incl. proposed new path or "kept" + why). Walk the full test file
list so nothing is missed; inline-test hosts noted in their row.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## File mapping`) | ~38 rows - the executable bridge for the test side of the refactoring phase |

### Draw the proposed test-tree mermaid
Replace the task-19 placeholder in `## Proposed layout` with a graph TD diagram:
subgraphs per suite family, file leaves, short labels.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## Proposed layout`) | the visual form of the proposed test tree - must survive the docs_md_test mermaid pipeline |

### Gate check + completeness audit
docs_md_test green (proves the mermaid renders); mapping-table row count ==
test file count (re-count with find over tests/, crates/*/tests/, fuzz/
fuzz_targets/); every proposed path consistent with the layout constraints.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete TEST_STRUCTURE.md: filled file-mapping table + proposed-layout mermaid.
- Both inventory documents finished - phase exit criteria checkable.

## Task exit criteria
- [ ] Mapping table has exactly one row per test source file (count verified),
      all four cells filled on every row; inline-test hosts noted.
- [ ] Mermaid diagram present in `## Proposed layout`, renders under docs_md_test.
- [ ] Layout constraints respected (cargo suite discovery, tests/common,
      fixtures, detached fuzz workspace, crate test dirs).
- [ ] All six sections of TEST_STRUCTURE.md contain content (no placeholders
      remain).
- [ ] docs_md_test green.

## Non-goals
- No actual moves/renames (later refactoring phase), no source changes. Phase
  close-out (status file, CURRENT_STATE row, TASKS.md entry, full verification)
  happens per PROTOCOL.md section 3 steps r-t after the user confirms all tasks
  are done.
