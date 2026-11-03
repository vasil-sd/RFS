# Phase 10, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the Rust backend -
`src/backends/rust.rs`, `rust_fsm.rs`, `rust_test.rs`, `rust_mc.rs`
(~131 fns / ~26 types) - into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/backends/rust.rs` (215 lines, 7 non-test fns + 3 inline tests in task 30 / 0 types): `RUST_BACKEND`
    registration + shared Rust helpers (ident mangling, type mapping).
  - `src/backends/rust_fsm.rs` (1,896 lines, 46 fns / 15 types): production FSM
    source generation (core reused by test/mc); `--no-std` and
    `--with-protocol-fsm` emission paths live here.
  - `src/backends/rust_test.rs` (977 lines, ~25 fns per test-aware count - NOTE: this file EMITS test code as string templates containing fn definitions that line counters miscount, and it has NO real test module of its own; the read-through establishes the authoritative non-test count / 1 type): unit-test
    generation + aggregate test-crate tooling (`generate_aggregate_test_crate`).
  - `src/backends/rust_mc.rs` (2,034 lines, 53 fns / 10 types): bounded/exact
    model-checker executable generation; the embedded json_rw writer copy for
    `--counterexample-format json` is emitted from here.
- Re-verify counts first (~131/~26); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all four are manifest files).
- Read this chunk BEFORE task 8 so the C-backend rows can mirror the same
  dependency vocabulary (cross-backend parity is a project invariant).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 4.-6. | Chunks C-F | backend rows depend on monitor/codegen/mc-plan identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | Chunk G (C backend) | mirrors this chunk's structure for consistent cross-backend notes |
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~134 function rows + ~26 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the four files vs ~134/~26; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~134 rows for the Rust backend |

### Enumerate types
One row per type in `## All types` (~26): registration/helper types, FSM/test/
MC generation internals.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~26 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for all four `src/backends/rust*.rs` files.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no C backend (task 8), no source changes.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| Row counts ~134/~26 -> 71/3 | The Scope table and the "Re-verify chunk counts" / "Enumerate functions" actions carried the pre-correction-2a figure "~134 function rows" (test-inclusive: 131 + rust.rs's 3 inline tests) and "~26 type rows". Both were wrong for different reasons, established by read-through (authoritative per correction 2a's own rule for emitter files). (1) Functions: the emitters' raw-string code templates contain fn definitions that line counters miscount - rust_fsm.rs 5 phantom fns (ProtocolFsm + Fsm templates), rust_test.rs 9 phantom test-fn templates in format! strings (and NO real test module: its awk-era "25 test fns" started counting at a DOC COMMENT mentioning #[cfg(test)]), rust_mc.rs 46 phantom fns across seven engine raw strings. Authoritative non-test counts: rust.rs 7, rust_fsm.rs 41, rust_test.rs 16, rust_mc.rs 7 = 71 (not ~131). (2) Types: the same templates define types - rust_fsm.rs 13 phantom types (TraceStep/Trace/Sample/AssumptionPolicy/AssumptionHandler/handlers/StepError/Fsm + 3 associated-type bindings), rust_mc.rs 10 phantom types (Node/StepOutcome/LivGraph/LivLeaf/LivProperty/LivWitness/McJson/LevelOutcome/Rng/ViolationSpec). Authoritative type counts: rust_fsm.rs 2 (Gen, ExprCtx), rust_test.rs 1 (GeneratedCrate), rust_mc.rs 0 = 3 (not ~26). Plan correction 7a records the chunk total 71/3 and the revised src/ totals (~935 fns / ~210 types). |
