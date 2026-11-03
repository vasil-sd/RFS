# Phase 10, Task 16 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Fill the Component and Functionality cells of EVERY row in the `## All types`
table (~233 rows) using the task-14 vocabulary. After this task no type row has
an empty Component or Functionality cell, and both inventory tables are fully
classified.

## Context
- Vocabulary: the filled `## Components` / `## Functionality` tables (task 14);
  assignment rules identical to task 15 (a type belongs to the component(s) of
  its file plus any cross-cutting component it clearly serves; functionality
  follows what the type CARRIES/ENABLES).
- Types cluster differently from functions: wire-format structs (codegen_mc,
  json_rw value model), plan models (TestPlan, MonitorPlan, mc_plan types), IR
  model types, backend generation internals. Where a type is shared across
  backends, its component assignment must name the shared home, not both
  backends.
- Work in task 2-13 chunk order; only the two cells change per row.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2.-13. | All enumeration chunks | the type rows exist |
| 14. | Categories proposal | provides the vocabulary |
| 15. | Function classification | keeps the two tables' assignments mutually consistent (same vocabulary, same rules) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 17., 18. | Naming scheme + file mapping | use the fully classified picture |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | Component + Functionality cells set on all ~233 type rows |

## Planned actions

### Classify chunk by chunk
For each cluster in task 2-13 order: review its type rows, set both cells.
Save after each cluster.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~237 rows gain their classification - completes the classified inventory |

### Consistency + completeness sweep
Zero type rows with an empty cell; cross-check that shared types (wire model,
plan models) carry the same component assignment as their sibling function
rows' cluster; spot-check 10 random rows.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Fully classified `## All types` table; both inventory tables complete.

## Task exit criteria
- [ ] Zero type rows with an empty Component or Functionality cell.
- [ ] No other column of any row was modified.
- [ ] Shared-type assignments consistent with the function-table picture.
- [ ] Rows remain one-per-line; docs_md_test green.

## Non-goals
- Naming scheme (task 17), file mapping (task 18), source changes.
