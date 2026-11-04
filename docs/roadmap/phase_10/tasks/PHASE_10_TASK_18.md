# Phase 10, Task 18 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Fill the `## File mapping` table (one row per non-test source file: components,
functionalities, description + rationale incl. its proposed new path) and draw
the proposed layout as a mermaid diagram in `## Proposed layout`. Completes
SRC_STRUCTURE.md - the bridge document the refactoring phase executes against.

## Context
- Input: task 17's naming scheme (target tree + conventions) and the classified
  inventory (tasks 15-16).
- File mapping table columns exactly: `| Source file name | Components |
  Functionalities | Description + rationale |`. One row per current non-test source
  file (63 non-test source files - 49 in src/ incl. bin/ + 14 in crates/;
  CORRECTION 18a supersedes the planned ~42 + ~17 estimate). Components/Functionalities
  cells are the file's assigned categories (multi-valued, comma-separated);
  description + rationale states what the file is and where it lands under the
  proposed scheme (new path, or "kept" with why).
- The mermaid diagram shows the PROPOSED directory tree: directories as nodes,
  files as leaves, grouped by component; use a graph TD layout with subgraphs
  per top-level group. It must render under mermaid-rs-renderer's full pipeline
  (docs_md_test) - verify by running the gate, and keep node labels short
  (file names only; categories live in the table, not the diagram).
- After this task SRC_STRUCTURE.md is complete: all six sections filled.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | per-file category assignments for the table cells |
| 17. | Naming scheme | the proposed paths the table and diagram render |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | `## File mapping` table filled (~59 rows) + `## Proposed layout` mermaid diagram - document complete |

## Planned actions

### Fill the file mapping table
One row per non-test source file with its components, functionalities,
description + rationale (incl. proposed new path or "kept"). Walk the full file
list from the inventory so nothing is missed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## File mapping`) | ~59 rows - the executable bridge for the refactoring phase |

### Draw the proposed layout mermaid
Replace the task-1 placeholder in `## Proposed layout` with a graph TD diagram:
subgraphs per top-level group, directory nodes, file leaves. Short labels only.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## Proposed layout`) | the visual form of the proposed tree - must survive the docs_md_test mermaid pipeline |

### Gate check + completeness audit
docs_md_test green (proves the mermaid renders); row count in the mapping table
== number of non-test source files (re-count with find); every row's proposed
path consistent with task 17's target tree.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete SRC_STRUCTURE.md: filled file-mapping table + proposed-layout mermaid.

## Task exit criteria
- [ ] Mapping table has exactly one row per non-test source file (count
      verified against `find`), all four cells filled on every row.
- [ ] Mermaid diagram present in `## Proposed layout`, renders under docs_md_test.
- [ ] Every proposed path in the table matches task 17's target tree.
- [ ] All six sections of SRC_STRUCTURE.md now contain content (no placeholders
      remain).
- [ ] docs_md_test green.

## Non-goals
- No actual moves/renames, no TEST_STRUCTURE.md work (tasks 19+), no source changes.
