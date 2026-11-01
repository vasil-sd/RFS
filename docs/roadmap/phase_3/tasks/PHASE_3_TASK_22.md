# Phase 3, Task 22 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 22: full verification per the BUILD.md checklist, write
`docs/roadmap/PHASE_3_STATUS.md` (template 2.3), update the INTRO.md phase-state table,
append the TASKS.md entry, and commit - closing Phase 3.

## Context

- Verification commands (plan "Other information", task 22):
  ```sh
  export RUSTUP_HOME=/workspace/host/rustup && export CARGO_HOME=/workspace/host/.cargo \
    && export PATH="$CARGO_HOME/bin:$PATH" && cd /workspace/host/RFS
  cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check
  cargo generate-code-tests && cargo test-generated-code
  cargo run -- validate --input rfs examples/valid   # exit 0 expected
  gcc -Wall <generated C output>                      # spot check, NEW monitor output compiles clean
  ```
  (`tests/backend_parity_test.rs` runs inside `cargo test`.)
- Counts to record: main-suite total (from `cargo test`), generated aggregate
  (task 18 final numbers), training counts (task 20), example counts (valid incl.
  41/42 + tla_adapted, invalid incl. invalid_always_zero_bound).
- Phase-state table: `docs/roadmap/INTRO.md` section 1 - row 3 becomes COMPLETE with
  the full file list; keep the transition note accurate (it documents that executed
  tasks 1-12 predate the task files - that stays true and is historical fact).
- Commit: ONE commit covering the whole Phase 3 working tree (source, tests, examples,
  training, docs, roadmap files). Message style per `git log` (e.g. "Phase 3: PSL-like
  properties and monitors (all backends, parity-tested)").

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13.-21. | All remaining tasks green | nothing may be outstanding before close-out |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/PHASE_3_STATUS.md` | new | phase completion report per template 2.3 |
| `docs/roadmap/INTRO.md` | change | section 1 phase-state table row 3 -> COMPLETE + file list |
| `TASKS.md` | change | append the Phase 3 close-out entry (first-paragraph template) |

## Planned actions

### Action 1: Run the full verification checklist

Run every command in Context. Fix nothing silently: any failure means an earlier task
is not actually done - record a plan correction, re-open that task's file, and loop
back per protocol step q. Also run the `gcc -Wall` spot check on freshly generated C
monitor output (production + mc for example 41) to prove the NEW monitor code compiles
clean.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - verification only) | - | results recorded in PHASE_3_STATUS.md "Tests run status" |

### Action 2: Write PHASE_3_STATUS.md per template 2.3

Fill ALL sections of template 2.3: added source files (`src/monitor.rs`),
changed/removed source files (parser, ir, printer, validator, codegen, main,
backends rust/c x fsm/test/mc, bins, tests), documentation files (all task-21 files +
C.md/RUST.md), other file changes (examples 41/42 + invalid_always_zero_bound + example
33 bound fix, training jsonl/snippets/aggregate), changed functionality (post-hoc
checks replaced by online monitors in both backends; MC online detection + enhanced
counterexamples; assumption pruning now covers liveness assumptions), removed
functionality (post-hoc `property_check`/`assume_check` in generated code, both
backends), new functionality (always[N], monitor compiler + R1-R4, per-backend monitor
rendering, StepError::PropertyViolation / RFS_PROPERTY_VIOLATION,
--with-property-monitors, parity test suite), tests add/change/remove (all new/changed
tests with counts), tests run status (exact counts per suite incl. generated aggregate
+ training), found problems (example 42 parser constraint + example 33 off-by-one bound,
both fixed), other findings (parser constraint workaround documented in KB).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/PHASE_3_STATUS.md` | new | the phase completion report (written once, never rewritten) |

### Action 3: Update INTRO.md phase-state table

Row 3 -> `COMPLETE`, files present: `PHASE_3.md`, `PHASE_3_PLAN.md`,
`PHASE_3_TASK_{13..22}.md` (+ their STATUS files), `PHASE_3_STATUS.md`. Keep the table
note about pre-task-file execution (historical accuracy).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/INTRO.md` | change (section 1 state table row 3) | protocol: keep the phase-state table current |

### Action 4: TASKS.md entry + commit

Append ONE close-out entry to TASKS.md per its first-paragraph template (interpretation
+ compact report with exact counts), then `git add -A` and commit the whole Phase 3
working tree. Verify `git status` is clean afterwards.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TASKS.md` | change (append entry) | append-only history per START.md rule 5 |
| (whole working tree) | commit | protocol step t: update TASKS.md and commit |

## Task deliverables
- Green full verification; `PHASE_3_STATUS.md`; updated state table; one commit;
clean working tree.

## Task exit criteria
- [ ] All checklist commands exit 0 (build, test, clippy, fmt, generated aggregate, validate examples/valid, gcc -Wall spot check).
- [ ] `PHASE_3_STATUS.md` exists and follows template 2.3 with exact counts.
- [ ] INTRO.md state table row 3 = COMPLETE with full file list.
- [ ] TASKS.md entry appended; single commit made; `git status` clean.

## Non-goals
- No new functionality in this task - verification, reporting, bookkeeping only.
- If Action 1 fails: this task is NOT complete; loop back per protocol step q (correct
  plan, re-execute the responsible task) before returning here.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| START.md snapshot sync (protocol step s) | Beyond the original affected-files table (`PHASE_3_STATUS.md`, `INTRO.md`, `TASKS.md`), protocol step s ("update all related documentation files; re-check that docs are aligned with the current state") required a minimal update of `START.md` section 1: its snapshot still said "only the phase close-out report + roadmap table update remain (task 22)" and "finish the Phase 3 close-out, task 22, first" - both false after close-out and misleading for the next session's orientation. Changed: IMPLEMENTED -> COMPLETE (close-out 2026-10-17), removed the task-22 parenthetical. Snapshot date label kept at 2026-10-17 (machine-local UTC, per project convention). No other START.md content touched. |

## Other information

- Commit grouping: everything Phase 3-related in one commit (the protocol-change commit
  8707574 is already separate and stays as-is).
- After the commit, the next phase entry point is `docs/roadmap/PHASE_4.md` + planning
  stage trigger "let's make a plan for Phase 4".
