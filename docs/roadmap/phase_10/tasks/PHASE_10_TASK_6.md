# Phase 10, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the MC-planning + SMT cluster -
`src/mc_plan.rs`, `src/smt_plan.rs`, `src/smt_emit.rs` (~83 fns / ~31 types) -
into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/mc_plan.rs` (1,130 lines, 34 fns / 18 types): the shared exact-mode MC
    plan - input abstraction, state encoding, traversal structure, liveness
    obligations; consumed by both backends' generated MC engines.
  - `src/smt_plan.rs` (692 lines, 11 fns / 11 types): SMT bounded-mode spec
    planning (feature-gated behind `smt`).
  - `src/smt_emit.rs` (1,849 lines, 38 fns / 2 types): the SMT emitter - z3 C
    API source generation for the generated C binary (links libz3 directly).
- Re-verify counts first (~83/~31); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all three are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 5. | Chunk D (codegen core) | mc_plan rows reference codegen_mc wire-format identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7., 8. | Backend chunks | backend MC rows reference mc_plan/smt identifiers |
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~83 function rows + ~31 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~83/~31; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~83 rows for the MC-plan/SMT cluster |

### Enumerate types
One row per type in `## All types` (~31): plan/encoding/traversal types, SMT
spec + emitter types.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~31 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/mc_plan.rs`, `src/smt_plan.rs`,
  `src/smt_emit.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
