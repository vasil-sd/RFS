# Phase 10, Task 30 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate the remaining inline `#[cfg(test)]` functions (backends/MC/CLI cluster,
~99 test fns per plan correction 2a - rust_test.rs and test_cmd.rs have NO real test module) plus the workspace-crate tests (`crates/rfs-build/tests/
integration.rs`, `crates/rfs-adapters/tests/{adapters,nostd_minimal}.rs`,
26 fns / 14 types) and ALL five `fuzz/fuzz_targets/*.rs` files (their SOLE home
per the user's plan-review decision) into `TEST_STRUCTURE.md`. Completes the
test inventory.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Per-file inline counts (re-verify; mismatch -> plan correction): rust_test 0,
rust.rs 3, c.rs 4, specgen 11, codegen_mc 13, json_schema 6, test_cmd 0,
info_format 4, smt_emit 6, ref_mc 24, mc_plan 16, mc_ref_cmd 7, gen_spec_cmd 5
= ~99 (correction 2a: the awk-era counts for rust_test/test_cmd/smt_emit were template-fn phantoms).
- Crate tests: rfs-build integration (6 fns / 0 types - RFS_BIN precedence/stub
error propagation + builder argv pins + real end-to-end vs the workspace
binary), rfs-adapters adapters.rs (12 fns / 7 types - channel round-trip/
ordering + error survival, async stream ordering/Pending-until-close/error
carry-through, callback completion/error/step-limit, minimal forwarding/
composition), nostd_minimal.rs (8 fns / 7 types - no_std compile checks).
- Fuzz targets: `fuzz/fuzz_targets/{parser,validator,ir_roundtrip,codegen}.rs`
(0-1 fns each - the libFuzzer entry is the `fuzz_target!` macro; enumerate the
explicit helper fns and note the macro entry in the description) +
`common.rs` (3 fns / 1 type - the shared fuzzer-bytes -> seed + bounds wrapper).
  These are the toolchain-fuzzing layer (Phase 6 task 5) - distinct from
  `rfs test --suite fuzz` which fuzzes GENERATED code.
- Re-verify all counts first (~161 fns / ~14 types total); mismatch -> plan
  correction.
- Documented-in: blank for inline tests and crate tests; the five fuzz-target
  files ARE manifest files - copy their `//! Docs:` fragments.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format |
| 2.-13. | SRC enumeration chunks | source-function identifiers for the Depends columns exist in SRC_STRUCTURE.md |
| 29. | Inline cluster 1 | keeps the two inline halves consistent (same identifier form, same rules) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31. | Test categories proposal | the complete test inventory is its input |
| 32., 33. | Classification + mapping/layout | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~161 function rows + ~14 type rows appended - test inventory complete |

## Planned actions

### Re-verify counts
Awk extraction over the thirteen inline files (~131) + fn/type greps over the
two crate-test files (26/~14 types) + fuzz-target greps (4 fns / 1 type);
mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate inline test functions (backends/MC/CLI cluster)
Same method as task 29: one row per test, `src/<file>.rs::tests::<fn>`
identifiers, Depends on the source functions under test and the types they construct.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`) | ~131 inline-test rows - second half of the inline enumeration |

### Enumerate crate tests + fuzz targets
One row per fn/type in the three crate-test files and five fuzz-target files;
fuzz-target macro entries noted in descriptions; Documented-in fragments copied
for the fuzz targets (manifest files).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~30 fn rows + ~14 type rows for crate tests + fuzz targets - completes the inventory |

### Gate check + completeness sweep
docs_md_test green; total test-table row counts vs the plan's test total
(~957 fns) - mismatch means a file was missed: find it, record a plan
correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete test inventory: all inline tests, crate tests, and fuzz targets
  enumerated in TEST_STRUCTURE.md.

## Task exit criteria
- [ ] Row counts equal re-verified per-file counts.
- [ ] Every row: Depends, one-line description, Documented-in per rule (fuzz-
      target fragments copied; rest blank).
- [ ] Total test function rows match the plan's ~957 (or a plan correction
      explains the delta).
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No categories (task 31), no classification (task 32), no mapping/layout
  (task 33), no source changes.
