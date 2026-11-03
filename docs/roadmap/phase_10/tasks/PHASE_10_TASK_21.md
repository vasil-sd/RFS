# Phase 10, Task 21 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the frontend CLI test suite -
`tests/frontend_test.rs` (137 fns / 1 type) + `tests/library_comparison.rs`
(9 fns) - into `TEST_STRUCTURE.md`. The largest single test file gets its own
chunk.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/frontend_test.rs` (4,148 lines): all four RFS/JSON conversion
    combinations, strict extension handling, output placement, collision guard,
    err-file generation, end-to-end binary exit codes; the Phase 5 task 13
    `validate --json` suite; the Phase 6 task 3 `mc-ref`/`gen-spec` suites
    (flag parsing + exit-code pins, text/JSON agreement with the generated
    engine, seed determinism); the Phase 5 task 3 `rfs info` suite; the task 7
    `rfs test` suite (both backends, byte-pinned --print-script wrappers,
    stub-tool precedence proofs); the task 8 `rfs diagram` suite (pinned outputs
    for all 5 formats, in-process + external rendering, strict error cases);
    the task 17 `rfs test --suite fuzz` suite (stub argv proofs, real-tool
    gated e2e).
  - `tests/library_comparison.rs` (82 lines): nom vs chumsky parser smoke
    comparison (dev-dependency only).
- Re-verify counts first (~146/~1); mismatch -> plan correction.
- Documented-in: blank for both files; note the TASK_MAP suite entries in
  descriptions where they exist.

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
| `TEST_STRUCTURE.md` | change | ~146 function rows + ~1 type row appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the two files vs ~146/~1; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn/type with bidirectional project function/type Depends
(incl. tests/common helpers and rfs library API calls) and one-line
descriptions. Work through frontend_test.rs section by section (conversion,
validate, info, test, diagram, mc-ref/gen-spec, fuzz) to keep descriptions
accurate.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~146 fn rows + ~1 type row for the frontend CLI suite cluster |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `tests/frontend_test.rs` and
  `tests/library_comparison.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites (tasks 22-28), no classification, no source changes.
