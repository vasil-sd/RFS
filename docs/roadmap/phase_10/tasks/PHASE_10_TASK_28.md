# Phase 10, Task 28 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the remaining `tests/` suites -
`tests/dev_docs_test.rs` (37 fns / 2 types) + `tests/diagram_render_parity_test.rs`
(27 fns) + `tests/example_projects_test.rs` (9 fns) +
`tests/integration_example_test.rs` (7 fns) + `tests/training_test.rs` (5 fns)
plus the three `tests/fixtures/dev_docs/*.rs` fixture files (1 fn each,
flagged as fixtures) - into `TEST_STRUCTURE.md`. Finishes the `tests/` tree.

## Context
- Conventions: TEST_STRUCTURE.md preamble (task 19). Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `tests/dev_docs_test.rs` (985 lines): the Phase 8 doc-sync gate - code->doc
    pointers over the 71-file manifest, doc->code reference resolution,
    fragment definition/uniqueness/ownership, TASK_MAP manifest coverage, Class
    B line budgets + negative-control fixtures. The ONLY test file with a
    `//! Docs:` line - copy its fragments into Documented-in.
  - `tests/diagram_render_parity_test.rs` (360 lines): cross-renderer syntax
    parity (Phase 7 task 4) - internal half all 50 valid examples x dot/puml/
    mmd in-process; external half pinned 4-spec subset via real tools,
    availability-gated + serialized.
  - `tests/example_projects_test.rs` (214 lines): c_sync_handler Makefile
    rebuild + C/Rust scenario parity with pinned values, fuzz_rust skeleton
    cargo check, fuzz_c generation without clang.
  - `tests/integration_example_test.rs` (229 lines): end-to-end integration over
    the example specs (stops at first failing file - noted in description).
  - `tests/training_test.rs` (197 lines): training-set structure/snippets/
    aggregation/freshness.
  - `tests/fixtures/dev_docs/{good_code,missing_pointer,dangling_pointer}.rs`
    (1 fn each): negative-control FIXTURES for dev_docs_test - flagged as
    fixtures per the preamble convention; their single function is a deliberate
    test subject.
- Re-verify counts first (~87/~2); mismatch -> plan correction.
- Documented-in: fragments from dev_docs_test.rs's `//! Docs:` line; blank for
  the rest.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | TEST skeleton + conventions | tables + row format (incl. the fixture-flagging rule) |
| 20. | Shared helpers chunk | helper identifiers exist for the Depends columns |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 31., 32. | Test categories + classification | use these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | ~87 function rows + ~2 type rows appended - tests/ tree complete |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the eight files vs ~87/~2; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions + types
Grep signatures, read bodies, one row per fn/type with bidirectional project function/type Depends
and one-line descriptions; fixture rows carry the fixture flag in their
description.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`, `## All types`) | ~87 fn rows + ~2 type rows for the remaining suites + fixtures |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for all remaining `tests/` files incl. fixtures.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in per rule (dev_docs_test
      fragments copied; rest blank).
- [ ] Fixture rows explicitly flagged as fixtures.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No inline/crate/fuzz enumeration (task 29-30), no classification, no source changes.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| Row counts 88/2 -> 85/2 | The Goal's "7 fns" for integration_example_test.rs was a grep count inflated by THREE template phantoms - `fn input_from` + `fn main` inside the DRIVER_SRC raw string and a second `fn main` inside the on_error test's inline driver string (all at column 0 of the string content, so line-based counters see them). Authoritative: integration_example_test.rs 4 fns / 0 types; all other per-file figures clean (dev_docs 37/2 incl. its three real nested walk fns, diagram_render_parity 27/0, example_projects 9/0, training 5/0, fixtures 1 fn each). Chunk total: 85 fns / 2 types. Recorded as plan correction 28a. The Context's Documented-in claim ("the ONLY test file with a //! Docs: line") is the same error correction 19a already settled - dev_docs_test.rs has no pointer line and is not in the manifest, so every row's Documented-in is blank per the pinned preamble rule. |
