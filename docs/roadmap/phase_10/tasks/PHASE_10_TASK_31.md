# Phase 10, Task 31 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Propose the test-side component and functionality category systems by reviewing
the finished test inventory, and fill the `## Components` and `## Functionality`
tables of `TEST_STRUCTURE.md`. Reuses the SRC vocabulary where a suite tests a
source component; adds test-specific categories.

## Context
- The full test inventory exists after task 30 (~957 function rows) with
  complete Depends edges - that edge picture is the input.
- Reuse rule: a suite that tests source component X gets component X (the
  tested side) - e.g. the parity suites' rows reference both backends, the doc
  gates reference the info-format/EBNF/schema components. This keeps the two
  documents' vocabularies mutually understandable.
- Test-specific categories to add (start here, refine against the rows):
  Components - shared test helpers (tests/common), frontend CLI suite, codegen
  suite, example-corpus suite, parity suites, MC benchmark/liveness/SMT suites,
  ref-MC soundness suites, monitor/PSL suites, doc-quality gates (docs_md +
  dev_docs + info_format), diagram parity suite, example-projects suite,
  training-pipeline suite, inline unit tests, workspace-crate tests, toolchain
  fuzz targets. Functionality - parity pinning, differential soundness,
  file-driven corpus coverage, benchmarking, doc-quality gating, code<->doc
  traceability, fuzzing (toolchain), fuzzing (generated code - referenced by
  the frontend_test fuzz-suite rows), exit-code/CLI-contract pinning,
  determinism pinning (seeds, byte-identity).
- Every category needs a depends-on list + 1-2 line description and rationale.
- Every test file must be assignable to >= 1 component and >= 1 functionality
  (verify by walking the file list).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 20.-30. | All test enumeration chunks | the complete test inventory + dependency edges are the input |
| 14. | SRC categories proposal | the vocabulary to reuse for tested-side components |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 32. | Test classification fill | uses this task's vocabulary |
| 33. | Test mapping + layout | justified against these categories |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | `## Components` + `## Functionality` tables filled |

## Planned actions

### Review the test dependency picture
Scan all test rows: which suites consume which helpers, which source components
they exercise (via their Depends edges into `rfs::` API and generated code).
Note conflicts with the reuse rule.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | analysis only |

### Design and write the category tables
Fill both tables: one row per category with depends-on + description/rationale.
Walk the test file list to verify full assignability.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## Components`, `## Functionality`) | the test-side classification vocabulary |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Filled `## Components` and `## Functionality` tables in TEST_STRUCTURE.md.

## Task exit criteria
- [ ] Both tables filled: every row has name, depends-on list, description +
      rationale.
- [ ] Every test file (incl. fixtures, inline-test hosts, crate tests, fuzz
      targets) is assignable to >= 1 component and >= 1 functionality.
- [ ] Reused SRC category names are spelled identically in both documents.
- [ ] docs_md_test green.

## Non-goals
- No row-level classification yet (task 32), no layout proposal (task 33),
  no source changes.
