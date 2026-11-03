# Phase 10, Task 32 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Fill the Component and Functionality cells of EVERY row in both TEST_STRUCTURE.md
inventory tables (~957 function rows + ~14 type rows) using the task-31
vocabulary. After this task no test row has an empty Component or Functionality
cell.

## Context
- Vocabulary: the filled `## Components` / `## Functionality` tables (task 31);
  assignment rules as in tasks 15-16 (a test belongs to the component of what it
  TESTS plus its own suite component; functionality follows what the test
  VERIFIES - parity pinning, determinism, exit codes, ...).
- Inline-test rows (`src/<file>.rs::tests::<fn>`) get the host file's source
  component(s) + the "inline unit tests" component.
- Fixture rows keep their fixture flag and get the doc-quality-gate component.
- Work chunk-by-chunk in task 20-30 order; only the two cells change per row.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 20.-30. | All test enumeration chunks | the rows to classify exist |
| 31. | Test categories proposal | provides the vocabulary |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 33. | Test mapping + layout | uses the per-file classification picture |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | Component + Functionality cells set on all test rows |

## Planned actions

### Classify chunk by chunk
For each cluster in task 20-30 order: review its rows, set both cells. Save
after each cluster.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~970 rows gain their classification - completes the classified test inventory |

### Completeness sweep
Zero rows with an empty Component or Functionality cell; spot-check 10 random
rows against their source.

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
- Fully classified TEST_STRUCTURE.md inventory tables.

## Task exit criteria
- [ ] Zero test rows with an empty Component or Functionality cell.
- [ ] No other column of any row was modified.
- [ ] Rows remain one-per-line; docs_md_test green.

## Non-goals
- Mapping table + layout (task 33), source changes.
