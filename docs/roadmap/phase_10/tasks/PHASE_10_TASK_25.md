# Phase 10, Task 25 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the model-checking test cluster -
`tests/benchmark_mc_test.rs` (24 fns / 5 types) + `tests/liveness_mc_test.rs`
(17 fns / 10 types) + `tests/smt_bounded_test.rs` (8 fns / 3 types) - into
`TEST_STRUCTURE.md`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/benchmark_mc_test.rs` (1,052 lines): the six property-bearing
    benchmark specs (43/44/45/47/48/49) through both backends in exact mode -
    pass-complete statuses, pinned deterministic metrics, inline FAIL variants
    with counterexample parity + ce_steps pins, `--threads 1/2/4` byte-identity,
    JSON report (`target/mc_benchmarks.json`) + timing tables.
  - `tests/liveness_mc_test.rs` (494 lines): liveness `eventually` / WEAK
    `until` + lasso counterexamples across the examples.
  - `tests/smt_bounded_test.rs` (315 lines): the feature-gated SMT bounded mode
    (`--mode smt-bounded`, needs the `smt` cargo feature + z3 env per
    TOOLCHAIN.md section 5.2) - availability-gated.
- Re-verify counts first (~49/~23); mismatch -> plan correction.
- Documented-in: blank for all three.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format |
| 20. | Shared helpers chunk | mc_json/fail_variants identifiers exist for the Depends columns |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31., 32. | Test categories + classification | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~49 function rows + ~23 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~49/~23; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn/type with bidirectional project function/type Depends
and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~49 fn rows + ~23 type rows for the MC test cluster |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for the three MC test suites.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites, no classification, no source changes.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| Type rows ~23 -> 6 | The Goal's per-file type figures ("5 types" benchmark_mc, "10 types" liveness_mc, "3 types" smt_bounded) and the Context's "~23" total were grep counts inflated by RFS spec enums inside inline raw strings - the phantom class of corrections 7a/8a/20a/21a/23a/24a. liveness_mc_test.rs: nine of the ten type hits are `enum State` declarations inside its nine inline spec constants (PASS_SPEC, FAIL_EV_SPEC, FAIL_UNTIL_SPEC, FAIL_SKEL_SAFETY_SPEC, FAIL_INPUT_IS_SPEC, NONDET_DISC_SPEC, TRUNC_FAIL_SPEC, NEST_DIAG_SPEC, SAFETY_PRECEDENCE_SPEC); only LivOut is real. smt_bounded_test.rs: all three hits are `enum State` inside SMT_FAIL_SRC / SMT_LIV_SRC / SMT_PASS_SRC - zero real types. benchmark_mc_test.rs is clean at 5 (Pin, BenchRun, FailPin, CodegenTimes, SpecRuns). The fn counts were all clean (24 + 17 + 8 = 49). Authoritative total: 49 fns / 6 types. Recorded as plan correction 25a. |
