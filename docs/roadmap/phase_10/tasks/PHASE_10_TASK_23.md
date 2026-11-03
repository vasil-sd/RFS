# Phase 10, Task 23 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the codegen + examples suites -
`tests/codegen_test.rs` (63 fns / 5 types) + `tests/examples_test.rs` (4 fns)
- into `TEST_STRUCTURE.md`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/codegen_test.rs` (2,152 lines): backend registry behavior,
    per-command generation for both backends, option sensitivity (incl.
    `--with-property-monitors`, `--no-std`, `--with-protocol-fsm`,
    `--counterexample-format json`), CLI argument parsing, the 50-example
    codegen sweep incl. the C-fuzz per-input free regression and the no_std
    compile sweep.
  - `tests/examples_test.rs` (99 lines): the file-driven example suite - all 50
    valid examples parse+validate+round-trip, all 13 invalid are rejected;
    dropping a new `.rfs` into `examples/valid|invalid` works without test-code
    change (that property is worth noting in the description).
- Re-verify counts first (~67/~5); mismatch -> plan correction.
- Documented-in: blank for both.

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
| `TEST_STRUCTURE.md` | change | ~67 function rows + ~5 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the two files vs ~67/~5; mismatch -> plan correction.

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
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~67 fn rows + ~5 type rows for the codegen/examples suites |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `tests/codegen_test.rs` and
  `tests/examples_test.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites, no classification, no source changes.
