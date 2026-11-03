# Phase 10, Task 29 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate the inline `#[cfg(test)]` functions of the frontend/IR cluster -
`src/{parser,validator,json_rw,printer,monitor,codegen_test,ext_tool,cli_log,
lib,frontend,spec_hash,codegen_fsm}.rs` (~180 test fns) - into
`TEST_STRUCTURE.md` with identifiers `src/<file>.rs::tests::<fn>`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19) - the inline-test identifier
  form is pinned there. Component/Functionality cells stay BLANK.
- Per-file counts measured at planning time (re-verify; mismatch -> plan
  correction): parser 32, validator 26, json_rw 26, monitor 28, codegen_test 31,
  printer 14, ext_tool 10, cli_log 2, lib 2, frontend 3, spec_hash 2,
  codegen_fsm 4 = ~180. (`smt_emit.rs`'s 10 inline tests are NOT in this
  cluster - they go to task 30 with the MC/CLI files.)
- These are unit tests living INSIDE source files: they test the very functions
  enumerated in SRC_STRUCTURE.md - their Depends columns will point at those
  source identifiers (cross-document references are expected and fine; note the
  target document in the row description where helpful).
- Extraction method: awk over `#[cfg(test)]` regions per file (the planning-time
  command is in PHASE_10_PLAN.md Context); read each test body for what it pins.
- Documented-in: blank (inline tests have no manifest line of their own; the
  host file's fragments are already recorded on the host's source rows).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format incl. inline-test identifier form |
| 2.-13. | SRC enumeration chunks | the source-function identifiers these tests depend on already exist in SRC_STRUCTURE.md |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 30. | Inline cluster 2 + crate tests + fuzz targets | completes the inline enumeration |
| 31., 32. | Test categories + classification | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~180 inline-test function rows appended (frontend/IR cluster) |

## Planned actions

### Re-verify per-file counts
Run the awk extraction over the twelve files; compare with Context (~180);
mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate inline test functions
For each file: extract the `#[cfg(test)]` fn list, read each body, append one
row per test with identifier `src/<file>.rs::tests::<fn>`, Depends on the source
functions under test, the types they construct, and any helpers, one-line description of what behavior is
pinned.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`) | ~180 inline-test rows - first half of the inline enumeration |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- ~180 inline-test rows for the frontend/IR cluster files.

## Task exit criteria
- [ ] Row count equals the re-verified awk extraction total (~180).
- [ ] Every row uses the `src/<file>.rs::tests::<fn>` identifier form.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No backends/MC/CLI inline cluster (task 30), no types (inline test modules
carry no project types of their own - if any appear, record a plan correction),
no classification, no source changes.
