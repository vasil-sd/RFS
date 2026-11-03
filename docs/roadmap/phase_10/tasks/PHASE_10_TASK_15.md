# Phase 10, Task 15 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Fill the Component and Functionality cells of EVERY row in the `## All
functions` table (~1,250 rows) using the task-14 vocabulary. Multi-valued
cells allowed (comma-separated). After this task no function row has an empty
Component or Functionality cell.

## Context
- Vocabulary: the filled `## Components` / `## Functionality` tables (task 14).
- Work chunk-by-chunk in task 2-13 order so the source context for each file
  cluster is fresh; re-read a file only when its row's assignment is genuinely
  ambiguous.
- Assignment rules: a function belongs to the component(s) of the file(s) it
  implements, plus any cross-cutting component it clearly serves (e.g. a
  validator helper used by codegen); functionality follows what the function
  DOES (parsing vs validation vs serialization vs ...), not where it sits.
  When in doubt between two functionalities, pick both - under-assignment is
  worse than over-assignment for the refactoring consumer.
- Keep rows one-per-line; only the two cells change, nothing else in a row is
  touched (Depends/description/Documented-in stay byte-identical).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2.-13. | All enumeration chunks | the rows to classify exist |
| 14. | Categories proposal | provides the vocabulary |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 17., 18. | Naming scheme + file mapping | use the per-file component/functionality picture this task produces |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | Component + Functionality cells set on all ~1,250 function rows |

## Planned actions

### Classify chunk by chunk
For each cluster in task 2-13 order: review its rows, set both cells per the
assignment rules. Save after each cluster.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~1,250 rows gain their classification - the input to the naming-scheme and mapping tasks |

### Completeness sweep
Count rows with an empty Component or Functionality cell; must be zero. Spot-
check 10 random rows against their source for sanity.

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
- Fully classified `## All functions` table.

## Task exit criteria
- [ ] Zero function rows with an empty Component or Functionality cell.
- [ ] No other column of any row was modified (diff shows only the two cells).
- [ ] Rows remain one-per-line; docs_md_test green.

## Non-goals
- Types table (task 16), naming scheme (task 17), source changes.
