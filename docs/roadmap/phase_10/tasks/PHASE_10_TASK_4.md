# Phase 10, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the validation + monitor-compilation
cluster - `src/validator.rs`, `src/codegen_fsm.rs`, `src/monitor.rs`
(~105 fns / ~13 types) - into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/validator.rs` (1,239 lines, 52 fns / 3 types): semantic validation and
    expression/formula typing; Phase 2 priority/duplicate-transition errors and
    the non-fatal warning channel (`module_warnings`: unreachable states, dead
    events, unused vars, impossible guards). Used by codegen, CLI validate, and
    snippet classification.
  - `src/codegen_fsm.rs` (231 lines, 10 fns / 3 types): language-agnostic
    production-FSM helpers - variant flattening (`build_variants`),
    representative values; legacy Phase 1 formula-subset helpers
    (`contains_liveness`, `safety_formula_inner`) kept for tests.
  - `src/monitor.rs` (1,034 lines, 43 fns / 7 types): the Phase 3 shared monitor
    compiler - formula -> per-label `MonitorPlan` (per-sample boolean signals,
    delay registers for `next`, obligation monitors with trigger/deadline
    countdown for `be[N]`, countdown monitors for `always[N]`), restrictions
    R1-R4; language-agnostic, both backends render the same plan; includes a
    reference interpreter used only by tests.
- Re-verify counts first (~105/~13); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all three are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2., 3. | Chunks A-B | validator/monitor rows depend on parser + IR identifiers that already exist in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | classify these rows |
| 18. | File mapping table | maps these files |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~105 function rows + ~13 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~105/~13; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions. Note the test-only reference
interpreter functions as such in their descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~105 rows for the validation/monitor cluster |

### Enumerate types
One row per type in `## All types` (~13): validator result/warning types,
codegen_fsm helpers, MonitorPlan family.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~13 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/validator.rs`, `src/codegen_fsm.rs`,
  `src/monitor.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
