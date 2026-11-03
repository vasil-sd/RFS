# Phase 10, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the C backend -
`src/backends/c.rs`, `c_fsm.rs`, `c_test.rs`, `c_mc.rs` (~73 fns / ~5 types) -
into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/backends/c.rs` (177 lines, 6 non-test fns + 4 inline tests in task 30 / 0 types): C backend registration +
    helpers.
  - `src/backends/c_fsm.rs` (2,160 lines, 37 fns / 2 types): production FSM C
    generation; wide bitvector ops use native unsigned wrap (plan correction
    2a of Phase 6 - well-defined for all inputs).
  - `src/backends/c_test.rs` (1,035 lines, 15 fns / 0 types): plain-assert
    per-test harness with the SAME stable test names as Rust + separate
    `fuzz_target.c` libFuzzer skeleton emission.
  - `src/backends/c_mc.rs` (2,836 lines, 15 fns / 3 types): bounded model
    checker mirroring the Rust exact engine - same summary line, counterexample
    fields, and CLI wire format (parity-pinned).
- Re-verify counts first (~73/~5); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all four are manifest files).
- Execute AFTER task 7 so cross-backend dependency notes stay consistent.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 4.-7. | Chunks C-G | C-backend rows depend on shared-plan identifiers and mirror the Rust-backend chunk's vocabulary |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~77 function rows + ~5 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the four files vs ~77/~5; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~77 rows for the C backend |

### Enumerate types
One row per type in `## All types` (~5): registration/helper + MC internals.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~5 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for all four `src/backends/c*.rs` files.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
