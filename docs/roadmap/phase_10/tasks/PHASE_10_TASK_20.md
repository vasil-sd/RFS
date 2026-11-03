# Phase 10, Task 20 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the shared test helpers -
`tests/common/{mod,prod_driver,fail_variants,mc_json}.rs` (~27 fns / ~8 types)
- into `TEST_STRUCTURE.md`. First data chunk of the test inventory; helpers go
first so later chunks' Depends columns can reference them.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19) + PHASE_10.md. Component/
  Functionality cells stay BLANK.
- Cluster notes:
  - `tests/common/mod.rs` (9 lines, 0 fns): module root for the shared helpers.
  - `tests/common/prod_driver.rs` (897 lines, 21 fns / 1 type): the production-
    FSM driver used by the parity suites - compiles generated production code
    and drives it with scripted inputs; the reason clippy reports dead-code
    warnings here is that each test binary only uses part of the helpers.
  - `tests/common/fail_variants.rs` (341 lines, 5 fns / 2 types): the pinned
    FAIL-variant specs shared across suites (ring_no_grant = trace CE,
    elev_noup = lasso CE, etc.) - single source of truth for counterexample
    pins.
  - `tests/common/mc_json.rs` (26 lines, 1 fn / 3 types): JSON MC-report parsing
    helpers for the parity/benchmark suites.
- Re-verify counts first (~27/~8); mismatch -> plan correction.
- Documented-in: blank (none of these files has a `//! Docs:` line).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 21.-28. | Suite chunks | their Depends columns reference these helper identifiers |
| 32. | Test classification fill | classifies these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~27 function rows + ~8 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the four files vs ~27/~8; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn and per type with bidirectional project function/type
Depends (helpers call into the `rfs` library API AND its types - record both directions)
and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~27 fn rows + ~8 type rows for the shared helpers - the dependency base of every later suite chunk |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for all four `tests/common/*.rs` files.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank (no manifest
      line exists - correct per convention).
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No suite enumeration (tasks 21-28), no classification, no source changes.
