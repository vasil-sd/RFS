# Phase 10, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the codegen-core cluster -
`src/codegen.rs`, `src/codegen_mc.rs`, `src/codegen_test.rs`
(~66 fns / ~40 types) - into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/codegen.rs` (658 lines, 13 fns / 12 types): the Backend plugin API
    (`Backend`/`BackendCommandInfo`), the compile-time static `BACKENDS`
    registry, `parse_codegen_args` CLI parsing, help rendering, shared
    option/output types, and the spec-hash + toolchain-version stamping into
    line 1 of every generated artifact (D-P6-3).
  - `src/codegen_mc.rs` (863 lines, 17 non-test fns + 13 inline tests in task 29 / 11 types): language-agnostic MC
    concepts - `McConfig` wire format, `ViolationSpec`, `McPhase`, `McStatus`;
    exact-mode `ExactStatus`/`FailReason` + the pinned summary line; Phase 5
    task 10 `CounterexampleFormat` + JSON report model (`McJsonReport` /
    `JsonCounterexample` / `JsonStep` / `JsonIo`, key-ordering contract next to
    the `JSON_KEY_*` constants); Phase 6 `statistics.peak_rss_kb` +
    `counterexample.ce_steps`.
  - `src/codegen_test.rs` (2,320 lines, 36 non-test fns + 31 inline tests in task 29 / 17 types): the language-
    agnostic unit-test planner - `TestPlan` with module-level cases,
    per-transition cases (static guard taken/not-taken analysis), invariant
    cases, monitor-violation cases, seeded random-trace case, fuzz decode plan;
    unexpected-input/deadlock detection. One plan rendered by BOTH backends.
- Re-verify counts first (~66/~40); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all three are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2.-4. | Chunks A-D | codegen rows depend on parser/IR/validator/monitor identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Chunk E (MC plan + SMT) | mc_plan/smt rows reference codegen_mc wire types |
| 7., 8. | Backend chunks | backend rows reference the shared plan/wire identifiers |
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~110 function rows + ~40 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~110/~40; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~110 rows for the codegen-core cluster |

### Enumerate types
One row per type in `## All types` (~40): registry/API types, MC wire + report
model, TestPlan family.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~40 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/codegen.rs`, `src/codegen_mc.rs`,
  `src/codegen_test.rs`.

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
| Row counts ~110/~40 -> 66/38 | Two stale figures superseded at count re-verification. (1) The Scope table and the "Re-verify chunk counts" / "Enumerate functions" actions still carried the pre-correction-2a figure "~110 function rows", which INCLUDES the 44 inline `#[cfg(test)]` functions (13 codegen_mc + 31 codegen_test); re-verification confirmed 13+17+36 = 66 non-test fns, matching correction 2a's "T5 66/40" and the boundary rule (inline tests belong to task 29). (2) The type figure ~40 came from the plan's counting grep, which matched two `type Err = String;` associated-type bindings INSIDE the FromStr impls of McMode/AssumptionSelection in codegen.rs - not standalone data types. Authoritative read-through count: 10 real types in codegen.rs, so 66 fns / 38 types (codegen.rs 10, codegen_mc.rs 11, codegen_test.rs 17). Same counter-phantom class as the emitter template fns noted in correction 2a; no plan correction needed. |
