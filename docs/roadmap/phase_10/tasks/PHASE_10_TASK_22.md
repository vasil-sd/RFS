# Phase 10, Task 22 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the documentation-quality gate suites -
`tests/info_format_test.rs` (59 fns) + `tests/docs_md_test.rs` (13 fns) - into
`TEST_STRUCTURE.md`.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/info_format_test.rs` (2,697 lines, 59 fns / 0 types): the Phase 7
    38-test `rfs info format` suite - registry `--list` pin, spec-rfs
    md/plantuml/svg pins with KEYWORDS coverage, DESIGN.md section 6 rule-name
    parity, draft-07 schema document pins (spec-json/ce-trace/mc-report/
    validate-json/info-json), the D-P7-9 sync walks of REAL tool output against
    the embedded models, diagram-dot/puml/mmd generated-subset coverage syncs,
    artifact-header pin + hash-variation sanity.
  - `tests/docs_md_test.rs` (311 lines, 13 fns / 0 types): the mdr-stack
    doc-quality gate - every repo `.md` parsed with comrak under mdr's GFM set,
    fenced-block closing, every ```mermaid block rendered through
    mermaid-rs-renderer's full pipeline, relative links resolve; negative
    controls per failure class. NOTE: this suite parses TEST_STRUCTURE.md and
    SRC_STRUCTURE.md themselves - the inventory files must stay gate-clean.
- Re-verify counts first (~72/~0); mismatch -> plan correction.
- Documented-in: blank for both (neither is a manifest file).

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
| `TEST_STRUCTURE.md` | change | ~72 function rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the two files vs ~72/~0; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn with bidirectional project function/type Depends and
one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`) | ~72 rows for the doc-quality gate suites |

### Gate check
docs_md_test green (this chunk's own subject suite - extra care that the new
rows keep it passing).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function rows for `tests/info_format_test.rs` and
  `tests/docs_md_test.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in blank per rule.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No other suites, no classification, no source changes.
