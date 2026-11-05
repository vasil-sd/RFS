# Phase 11, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Execute the TEST_STRUCTURE.md proposal: rename the three test suites whose
names do not match their responsibility.

## Context
- Renames: tests/library_comparison.rs -> tests/parser_comparison_test.rs;
  tests/examples_test.rs -> tests/example_corpus_test.rs;
  tests/integration_example_test.rs -> tests/endpoint_protocol_test.rs.
  Every other test file is kept in place (the proposal's layout keeps all suites
  flat top-level tests/*.rs - cargo discovers one binary per file, families are
  visible through naming, not directories).
- No Cargo.toml / manifest edits: integration tests are auto-discovered by file
  name. A green cargo test run proves discovery of the three renamed binaries.
- Doc references to the old names are re-synced in task 10 (do NOT edit docs/
  here - keep this task source-only).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | mc/ | the source tree is complete; nothing else moves after this task |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Docs<->source reference re-sync | rewrites doc references to the new suite names |
| 12. | TEST_STRUCTURE.md actualization | identifiers use the new file names |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/library_comparison.rs` | remove (moved) | git mv to tests/parser_comparison_test.rs |
| `tests/examples_test.rs` | remove (moved) | git mv to tests/example_corpus_test.rs |
| `tests/integration_example_test.rs` | remove (moved) | git mv to tests/endpoint_protocol_test.rs |

## Planned actions

### Rename the three suites
git mv all three; no content edits (the files' internal module names, if any,
are file-local and unaffected - verify by build). Rationale: names match the
responsibility categories the test inventory already uses (Parser comparison
suite / Example corpus suite / Integration example suite).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/library_comparison.rs` | remove | git mv to tests/parser_comparison_test.rs |
| `tests/examples_test.rs` | remove | git mv to tests/example_corpus_test.rs |
| `tests/integration_example_test.rs` | remove | git mv to tests/endpoint_protocol_test.rs |

### Gate
cargo build && cargo test green - the three renamed binaries must appear in the
run output under their new names.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- The three suites at their proposed names; all 535 tests still discovered and green.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline); the three new binary names appear in the output.
- [ ] The old file names no longer exist under tests/.
- [ ] No doc/ file was modified by this task.

## Non-goals
- No test logic change; no other test file touched.
