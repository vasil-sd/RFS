# Phase 18, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Close out Phase 18: run the full verification matrix, write the phase status report, and
synchronize every record that names the project state (TASKS.md, CURRENT_STATE.md, START.md's
doc map, BUILD.md's test rows). Rationale: the phase exits only on a fully green matrix +
recorded counts.

## Context
- The Phase 17 close-out baseline (the FLOOR): `cargo test` 584, `--workspace` 720,
  `--features smt` 594, generated-code 1720/194. This phase ADDS tests (50 example-project legs
  + 3 nested-cargo member legs + the three members' inline tests) - root and workspace counts
  will be HIGHER; generated-code moves ONLY if task 9 fenced new valid snippets (recorded, not a
  regression - all pre-existing modules stay byte-identical).
- Doc gates: `cargo test --test docs_md_test` (every repo .md - now including the 50 new example
  READMEs + the EXAMPLES.md "Other examples" section) and `cargo test --test dev_docs_test`
  (sync gate + Class B budgets - the TEST_STRUCTURE rows from tasks 1-7 must fit inside the
  existing budget; extend it first if needed, as Phase 17 task 3 did).
- Inventory completeness: `rfs-dev inventory audit` must be clean - every new test fn has its
  TEST_STRUCTURE row (added in tasks 1-7).
- PHASE_18_STATUS.md follows FORMATTING.md section 4 exactly. Functionality deltas: NONE (examples
  + docs + training data only - no source behaviour change).
- TASKS.md: append ONE entry per its FIRST-PARAGRAPH template only (never read the whole history).
- CURRENT_STATE.md: add the phase-18 row (state COMPLETE, files present) and refresh the header
  date to today.
- START.md section 2 doc map: no new top-level doc is added (the examples live under
  crates/examples/), so the doc map likely needs no change - re-check and only touch it if a doc
  file was added. BUILD.md section 3: the example-projects test row must reflect the new legs'
  count; the training rows must reflect task 9's new totals if they shifted; the root/workspace/
  smt/generated-code headline totals must match the measured values.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-9. | all previous tasks | the close-out verifies and records their combined result |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/phase_18/PHASE_18_STATUS.md` | new | the phase completion report (FORMATTING.md section 4) |
| `TASKS.md` | change | append the close-out entry (first-paragraph template) |
| `docs/roadmap/CURRENT_STATE.md` | change | phase-18 row + header date refresh |
| `BUILD.md` | change | example-projects test row + training totals + headline totals if shifted |
| `START.md` | change (only if a doc file was added) | re-check the doc map; likely no change |

## Planned actions

### 10.1 - Run the full verification matrix
`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` (root),
`cargo test --workspace`, `cargo test --features smt` (z3 env per TOOLCHAIN.md section 5.2),
`cargo test-generated-code`, `cargo test --test docs_md_test`, `cargo test --test dev_docs_test`,
`rfs-dev inventory audit`. Record every count. Any failure is fixed via task corrections in the
offending task's file - never by weakening an assertion.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (verification only) | - | the phase exits only on a fully green matrix |

### 10.2 - Write PHASE_18_STATUS.md
Per FORMATTING.md section 4: every file added/changed across tasks 1-9 (the 50 example dirs, the
three member crates, the EXAMPLES.md section, the training files, the TEST_STRUCTURE rows);
Changed / Removed functionality = none; New functionality = the real-world-FSM catalogue + docs +
training surface (hierarchical bullets); Tests add/change/remove table (the 50 + 3 legs + member
inline tests); Tests run status with exact counts; findings from the whole phase.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_18/PHASE_18_STATUS.md` | new | the permanent record of what the phase did |

### 10.3 - Synchronize the state records
TASKS.md entry (first-paragraph template). CURRENT_STATE.md phase-18 row + header date. BUILD.md
example-projects row + training totals + headline totals if shifted. START.md doc map only if a
doc file was added.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TASKS.md` | change (append) | the append-only history gets its close-out entry |
| `docs/roadmap/CURRENT_STATE.md` | change | the phase-state table must show 18 COMPLETE |
| `BUILD.md` | change (section 3 rows) | expected test counts must match reality for the next session |

### 10.4 - Commit
Commit the phase close-out per FORMATTING.md section 7 commit format.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (git) | - | the working tree is committed with a section-7-format message |

## Task deliverables
- A fully green verification matrix with recorded counts.
- PHASE_18_STATUS.md + synchronized TASKS.md / CURRENT_STATE.md / BUILD.md (+ START.md if needed).

## Task exit criteria
- [ ] Every PHASE_18.md exit-criterion item verified true (walk the list).
- [ ] `cargo test-generated-code` matches the recorded value (moves only if task 9 fenced new valid snippets - all pre-existing modules byte-identical).
- [ ] `rfs-dev inventory audit` clean; both doc gates green.
- [ ] PHASE_18_STATUS.md written per FORMATTING.md section 4 with exact test counts.
- [ ] TASKS.md entry appended; CURRENT_STATE.md shows phase 18 COMPLETE with today's date in the header; BUILD.md counts match the measured totals.
- [ ] Working tree committed per FORMATTING.md section 7 commit format.

## Non-goals
- No new functionality of any kind - this task only verifies and records.
- No rewrites of earlier tasks' status files (they are frozen once written).
