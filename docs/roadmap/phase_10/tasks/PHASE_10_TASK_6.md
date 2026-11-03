# Phase 10, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the MC-planning + SMT cluster -
`src/mc_plan.rs`, `src/smt_plan.rs`, `src/smt_emit.rs` (~61 fns / ~29 types) -
into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/mc_plan.rs` (1,130 lines, 18 non-test fns + 16 inline tests in task 29 / 16 types): the shared exact-mode MC
    plan - input abstraction, state encoding, traversal structure, liveness
    obligations; consumed by both backends' generated MC engines.
  - `src/smt_plan.rs` (692 lines, 11 fns / 11 types): SMT bounded-mode spec
    planning (feature-gated behind `smt`).
  - `src/smt_emit.rs` (1,849 lines, ~32 non-test fns per test-aware count - NOTE: this emitter's string templates contain C function definitions that line counters miscount; the read-through establishes the authoritative count - + 6 inline tests in task 30 / 2 types): the SMT emitter - z3 C
    API source generation for the generated C binary (links libz3 directly).
- Re-verify counts first (~61/~29); mismatch -> plan correction.
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

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| Row counts ~83/~31 -> 57/29 | Three stale figures superseded at count re-verification. (1) The Scope table and the "Re-verify chunk counts" / "Enumerate functions" actions carried the pre-correction-2a figure "~83 function rows", which INCLUDES the 26 inline `#[cfg(test)]` functions (16 mc_plan + 10 smt_emit); per the boundary rule those belong to tasks 29/30, so only non-test fns are enumerated here. (2) The cluster note's "~32 non-test fns" for smt_emit.rs (inherited from correction 2a's "T6 61/29") was based on the claim that its test module holds 6 real fns, not 10; the read-through shows ALL 10 are real and there are NO template phantoms - the generated `fn` text lives inside format!/push_str string literals and never starts a line. Authoritative smt_emit.rs count: 28 non-test fns / 2 types. (3) The "~31 type rows" figure was likewise pre-correction-2a; the authoritative type count is 29 (mc_plan.rs 16 - its grep's 2 extra matches are template enums inside the test region, smt_plan.rs 11, smt_emit.rs 2). Chunk E total: 57 fns / 29 types. Recorded as plan correction 6a (supersedes correction 2a's "T6 61/29"). |
