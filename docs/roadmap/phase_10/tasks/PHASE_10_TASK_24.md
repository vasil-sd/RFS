# Phase 10, Task 24 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the cross-backend parity suites -
`tests/backend_parity_test.rs` (41 fns / 14 types) +
`tests/wide_ops_parity_test.rs` (8 fns) + `tests/prod_parity_test.rs`
(3 fns / 1 type) - into `TEST_STRUCTURE.md`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/backend_parity_test.rs` (1,697 lines): the cross-backend differential
    suite - same-seed MC runs over all property-bearing examples must be
    field-identical; both backends' generated test executables agree on every
    example (identical per-test result lists + byte-identical random-trace
    coverage line); forced-violation parity (Phase 6 extension).
  - `tests/wide_ops_parity_test.rs` (331 lines): spec-50 boundary behavior
    (variable shifts by >= width -> 0, wide multiplication) identical across
    Rust and C - pins Phase 6 plan correction 2a.
  - `tests/prod_parity_test.rs` (130 lines): EVERY valid example x both
    backends (monitors ON), identical seeded LCG scripts (seed 43210, 200
    steps) -> byte-identical stdout + exit code.
- Re-verify counts first (~52/~18); mismatch -> plan correction.
- Documented-in: blank for all three.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format |
| 20. | Shared helpers chunk | prod_driver/fail_variants/mc_json identifiers exist for the Depends columns - these suites are their main consumers |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31., 32. | Test categories + classification | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~52 function rows + ~18 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~52/~18; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn/type with bidirectional project function/type Depends
(heavy use of tests/common helpers expected - record those edges) and one-line
descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~52 fn rows + ~18 type rows for the parity suites |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for the three parity suites.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites, no classification, no source changes.
