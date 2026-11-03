# Phase 10, Task 26 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the reference-MC soundness cluster -
`tests/ref_mc_sweep_test.rs` (6 fns / 3 types) + `tests/ref_mc_diag_test.rs`
(5 fns / 1 type) + `tests/ref_mc_diff_test.rs` (24 fns) +
`tests/toolchain_fuzz_test.rs` (12 fns / 1 type) - into `TEST_STRUCTURE.md`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/ref_mc_sweep_test.rs` (480 lines): ref-MC byte-parity sweep vs the
    generated Rust MC over all eligible valid examples + pinned specgen seeds,
    fail-fast N-way parallel, per-sample timeout.
  - `tests/ref_mc_diag_test.rs` (271 lines): the same sweep without early exit
    - full disagreement report (diagnostic companion).
  - `tests/ref_mc_diff_test.rs` (630 lines, 4 tests): ref-MC vs generated-MC
    differential - eligible small examples + K specgen random specs, both
    backends exact mode vs ref_mc, status/label+step/deadlock agreement, no-
    false-PASS negative control.
  - `tests/toolchain_fuzz_test.rs` (437 lines): the seeded compile-and-run CI
    layer of the two-layer toolchain fuzzing - 3 pinned seeds x 10 small
    specgen specs -> all six artifacts generated + compiled (rustc / gcc -Wall)
    + MC exact & fuzz vs ref_mc ground truth; deterministic trigger-step-0
    report pin.
- Re-verify counts first (~47/~8); mismatch -> plan correction.
- Documented-in: blank for all four.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format |
| 20. | Shared helpers chunk | helper identifiers exist for the Depends columns |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31., 32. | Test categories + classification | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~47 function rows + ~8 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the four files vs ~47/~8; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn/type with bidirectional project function/type Depends
(heavy `rfs::ref_mc` / `rfs::specgen` library-API edges expected) and one-line
descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~47 fn rows + ~8 type rows for the soundness cluster |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for the four soundness suites.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites, no classification, no source changes.
