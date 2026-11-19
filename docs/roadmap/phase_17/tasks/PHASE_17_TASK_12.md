# Phase 17, Task 12 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Close out Phase 17: run the full verification matrix, write the phase status
report, and synchronize every record that names the project state (TASKS.md,
CURRENT_STATE.md, START.md's doc map, BUILD.md's test rows).

## Context
- The Phase 16 close-out baseline (the FLOOR): `cargo test` 573, `--workspace`
  701, `--features smt` 583, generated-code exactly 1683. This phase ADDS tests
  (three new workspace members' inline tests + ~12 new example_projects_test
  legs) - the root and workspace counts will be HIGHER; generated-code must
  stay EXACTLY 1683 (no codegen behaviour changed).
- Doc gates: `cargo test --test docs_md_test` (every repo .md - now including
  USAGE.md + 15 example READMEs) and `cargo test --test dev_docs_test` (sync
  gate + Class B budgets - the extended 1830/1430 headers must hold with the
  appended rows inside them).
- Inventory completeness: `rfs-dev inventory audit` (the string-masked
  completeness check, PROTOCOL.md section 5) must be clean - every fn/type
  added by tasks 7-9 has its row.
- PHASE_17_STATUS.md follows FORMATTING.md section 4 exactly (Added source
  files / Changed-removed source files / Added-changed-extended documentation
  files / Other file changes / Changed functionality / Removed functionality /
  New functionality / Tests add-change-remove / Tests run status / Found
  problems / Other important findings). Functionality deltas: NONE (docs +
  examples + training data only - the usage() edit is printed text).
- TASKS.md: append ONE entry per its FIRST-PARAGRAPH template only (never read
  the whole history).
- CURRENT_STATE.md: add the phase-17 row (state COMPLETE, files present) and
  refresh the stale header date ("Phase state (2026-11-07)" -> today).
- START.md section 2 doc map: the `docs/user/` row currently reads "7 files +
  MIGRATION.md" - it gains USAGE.md (8 files + MIGRATION.md); update the row's
  content description to mention the CLI usage reference.
- BUILD.md section 3: the example-projects test row (currently "3 example
  projects") must reflect the new legs' count; the training rows (138/128
  snippets, 267 aggregate) must reflect task 11's new totals if they shifted.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-11. | all previous tasks | the close-out verifies and records their combined result |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/phase_17/PHASE_17_STATUS.md` | new | the phase completion report (FORMATTING.md section 4) |
| `TASKS.md` | change | append the close-out entry (first-paragraph template) |
| `docs/roadmap/CURRENT_STATE.md` | change | phase-17 row + header date refresh |
| `START.md` | change | doc-map row for docs/user/ gains USAGE.md |
| `BUILD.md` | change | example-projects test row + training totals if shifted |

## Planned actions

### Run the full verification matrix
`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`
(root), `cargo test --workspace`, `cargo test --features smt` (z3 env per
TOOLCHAIN.md section 5.2), `cargo test-generated-code` (must be exactly 1683),
`cargo test --test docs_md_test`, `cargo test --test dev_docs_test`, `rfs-dev
inventory audit`. Record every count. Any failure is fixed via task corrections
in the offending task's file - never by weakening an assertion.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (verification only) | - | the phase exits only on a fully green matrix |

### Write PHASE_17_STATUS.md
Per FORMATTING.md section 4: every file added/changed across tasks 1-11 (the
three new example dirs x2 categories, three member crates, USAGE.md, all README
changes, the training files, the inventory rows, the usage() edit); Changed /
Removed functionality = none; New functionality = the documentation + examples
+ training surface (hierarchical bullets); Tests add/change/remove table (the
new legs + members' tests); Tests run status with exact counts; findings from
the whole phase.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_17/PHASE_17_STATUS.md` | new | the permanent record of what the phase did |

### Synchronize the state records
TASKS.md entry (first-paragraph template). CURRENT_STATE.md phase-17 row +
header date. START.md doc-map row (docs/user/ gains USAGE.md). BUILD.md
example-projects row + training totals if shifted.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TASKS.md` | change (append) | the append-only history gets its close-out entry |
| `docs/roadmap/CURRENT_STATE.md` | change | the phase-state table must show 17 COMPLETE |
| `START.md` | change (section 2 row) | the doc map is read every session - it must name USAGE.md |
| `BUILD.md` | change (section 3 rows) | expected test counts must match reality for the next session |

## Task deliverables
- A fully green verification matrix with recorded counts.
- PHASE_17_STATUS.md + synchronized TASKS.md / CURRENT_STATE.md / START.md /
  BUILD.md.

## Task exit criteria
- [ ] Every PHASE_17.md exit-criterion item verified true (walk the list).
- [ ] `cargo test-generated-code` exactly 1683/1683.
- [ ] `rfs-dev inventory audit` clean; both doc gates green.
- [ ] PHASE_17_STATUS.md written per FORMATTING.md section 4 with exact test
      counts.
- [ ] TASKS.md entry appended; CURRENT_STATE.md shows phase 17 COMPLETE with
      today's date in the header; START.md names USAGE.md; BUILD.md counts
      match the measured totals.
- [ ] Working tree committed per FORMATTING.md section 7 commit format.

## Non-goals
- No new functionality of any kind - this task only verifies and records.
- No rewrites of earlier tasks' status files (they are frozen once written).
