# Phase 10, Task 19 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Create `TEST_STRUCTURE.md` at the repo root as an empty six-section skeleton
with a conventions preamble adapted for test source, mirroring task 1. Foundation
of the test-inventory half of the phase (user step p7).

## Context
- Same six sections and table headers as SRC_STRUCTURE.md (PHASE_10.md fixes
  them, including the identifier column names - `Function` in the functions
  table, `Type` in the types table): `## All functions`, `## All types`,
  `## Components`, `## Functionality`, `## File mapping`, `## Proposed layout`.
- Test-specific convention deltas to pin in the preamble:
  - Inline `#[cfg(test)]` functions are enumerated with identifiers of the form
    `src/<file>.rs::tests::<fn>` (their host file is the identifier's file part;
    they live in TEST_STRUCTURE.md, not SRC_STRUCTURE.md).
  - Documented-in is blank for all test files EXCEPT `tests/dev_docs_test.rs`
    (the only one with a `//! Docs:` line); where `docs/llm/TASK_MAP.md` has a
    suite entry, note it in the row's description instead.
    **CORRECTION 19a** (recorded at execution): the "only one with a `//! Docs:`
    line" claim is wrong. `tests/dev_docs_test.rs` has NO pointer line (its
    `Docs:` mentions are prose describing the convention) and is not in the doc
    manifest, so its Documented-in is blank. The test files that DO carry real
    `//! Docs:` pointers to fragments defined in `docs/` are the five fuzz targets
    (`fuzz/fuzz_targets/{common,parser,codegen,validator,ir_roundtrip}.rs`) and
    the three workspace-crate tests (`crates/rfs-adapters/tests/{adapters,
    nostd_minimal}.rs`, `crates/rfs-build/tests/integration.rs`) - all eight are
    in `tests/dev_docs_manifest.txt` and their fragments are copied like any
    other file's. The three fixtures' `//! Docs:` lines point at fixture-only
    fragments (`dev:fixture-*` / `llm:fixture-*`, not defined in the doc set) as
    deliberate negative controls. TEST_STRUCTURE.md's preamble pins this accurate
    rule.
  - The three `tests/fixtures/dev_docs/*.rs` files are negative-control
    FIXTURES for dev_docs_test - enumerated like any other file but flagged as
    fixtures in their descriptions (their "functions" are deliberate test
    subjects, not project code).
  - Generated build output (`crates/examples/fuzz_rust/fuzz/target/**`) is NEVER
    enumerated.
- Scope boundary (binding, from PHASE_10.md): `tests/**`, inline
  `#[cfg(test)]` modules in `src/`, `crates/*/tests/**`, and
  `fuzz/fuzz_targets/**` (the fuzz targets' SOLE home - user decision at plan
  review).
- From creation, docs_md_test parses this file - keep it gate-clean.
- Component/Functionality cells stay BLANK until task 32.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 20.-30. | All test enumeration chunks | need the skeleton + conventions |
| 31. | Test categories proposal | fills the two empty category tables |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | new | six-section skeleton + test-adapted Conventions preamble, no data rows yet |

## Planned actions

### Create the skeleton file
Write TEST_STRUCTURE.md: title (`# RFS Test Structure - function and type inventory`),
short intro (phase + later refactoring consumer), the conventions preamble with
the test-specific deltas from Context, then the six sections with exact headers
and no data rows; `## Proposed layout` gets a "filled by task 33" placeholder.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | new | the skeleton - single canonical test-inventory location at repo root |

### Gate check
`cargo test --test docs_md_test` green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `TEST_STRUCTURE.md` at repo root: conventions preamble + six empty sections.

## Task exit criteria
- [ ] File exists with all six section headings in the fixed order and exact
      table headers.
- [ ] Preamble pins the test-specific deltas: inline-test identifier form,
      Documented-in rule, fixture flagging, generated-output exclusion, scope
      boundary incl. fuzz targets' sole home.
- [ ] No data rows anywhere.
- [ ] docs_md_test green (5/5).

## Non-goals
- No test rows (tasks 20-30), no categories (task 31), no mapping/layout
  (task 33), no source changes.
