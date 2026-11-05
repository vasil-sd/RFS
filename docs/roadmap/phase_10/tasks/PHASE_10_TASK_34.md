# Phase 10, Task 34 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate the two example-crate inline test functions that no task assigned
(plan correction 34a) into TEST_STRUCTURE.md's All functions table, and update
the document's inline-host bookkeeping so the fresh-grep close-out audit is
exact.

## Context
- The gap: `crates/examples/{async_handler,sync_handler}/src/lib.rs` each carry
  a `#[cfg(test)] mod tests` with one `#[test]` fn (the Phase 5 task 11 CI
  records `arq_session_completes_over_the_stream` /
  `arq_session_completes_over_channels`). Neither inventory document enumerates
  them: SRC_STRUCTURE's boundary puts the example-crate src/ files there, but
  correction 2a already moved inline test fns to the test side, and tasks 29-30
  never assigned these two hosts (the same gap class as correction 30c's
  rfs-build module).
- Classification per task 32's rules: Component = Inline unit tests + Example
  crates (the host file's SRC component); Functionality = behavior pinning
  (pinned protocol invariants - the home axis for inline rows).
- Depends: each test calls its file's `run` and matches on its `ArqReport`
  fields - both identifiers exist in SRC_STRUCTURE.md.
- Documented-in blank per the pinned preamble rule for inline rows (the hosts'
  //! Docs: fragments live on their SRC_STRUCTURE rows).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 32. | Test classification fill | provides the component/functionality vocabulary and rules |
| 33. | File mapping + layout | the document being completed; its intro note lists the inline hosts |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| (none) | - | phase close-out follows directly |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | 2 function rows appended + preamble host count + Components/Functionality inline-row counts + task-33 intro note updated |

## Planned actions

### Enumerate the two rows
Append one row per test fn to the All functions table (after the fuzz-target
rows, before ## All types), with Depends, classification, one-line description.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (`## All functions`) | 2 rows - completes the inline-test enumeration |

### Update the inline-host bookkeeping
Preamble host-file count, the Inline unit tests component row's counts, the
behavior-pinning axis row's count, and task 33's file-mapping intro note.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` | change (preamble, `## Components`, `## Functionality`, `## File mapping` intro) | the document's own counts stay exact against a fresh grep |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- TEST_STRUCTURE.md with all 297 inline-test rows enumerated and its counts exact.

## Task exit criteria
- [ ] Two new rows present with all six cells filled per the conventions.
- [ ] Preamble + Components + Functionality + file-mapping intro counts updated (26 hosts, 297 inline rows).
- [ ] Fresh string-masked grep over tests/, crates/*/tests/, fuzz/fuzz_targets/ plus the inline test modules of all host files equals the table totals exactly (942 fns / 34 types).
- [ ] docs_md_test green.

## Non-goals
- No source changes, no re-classification of existing rows.
