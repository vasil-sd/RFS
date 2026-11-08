# Phase 12, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Close out Phase 12: run the full verification with dev_tools in the workspace,
synchronize every document that inventories the tree or the commands, write
`PHASE_12_STATUS.md`, and update `CURRENT_STATE.md` + `TASKS.md`. Rationale:
PROTOCOL.md section 3 steps r-s - the phase exits only with docs aligned to the
new state and the status report written; the known-good baselines in BUILD.md/
START.md must reflect dev_tools' tests or every future session inherits stale
numbers.

## Context
- Baseline shift: dev_tools is a workspace member (task 1), so `cargo test
  --workspace` count grows by exactly dev_tools' test count; the root-package
  `cargo test` count (535) is UNCHANGED - verify that explicitly, it is the
  proof nothing in the rfs package was touched. `--features smt` count (537)
  likewise unchanged. The NEW workspace baseline is recorded in BUILD.md
  section 3's workspace row and section 5's known-good paragraph, and in
  START.md section 1's snapshot line - old numbers are replaced, not appended
  (those files carry the CURRENT baseline, history lives in TASKS.md).
- Documents that inventory the tree/commands and MUST gain dev_tools:
  BUILD.md (section 3 test-layout row for `-p rfs-dev`; section 4 command table
  row; section 5 checklist note "touched dev_tools -> cargo test -p rfs-dev"),
  START.md (section 3 source-tree map: a `dev_tools/` line; section 1 snapshot:
  the Phase 12 completion line + new baselines), KB.md (section 4.2 file map:
  dev_tools row - locate via `grep -n "^## \|^### " KB.md` then sed-range, per
  START.md section 0 rule 2; NEVER read KB.md fully).
- `PHASE_12_STATUS.md` follows FORMATTING.md section 4 verbatim (all mandatory
  sections; "(none)" rows where empty). Its "Found problems/issues/mismatches"
  and "Other important findings/information" sections collect: analysis
  deviations from plan-Context expectations (task 3 findings), any plan
  corrections made during execution, the DOCS_STRUCTURE.md decision outcome,
  and the demonstration results summary.
- Phase exit criteria check (PHASE_12.md) is performed VERBATIM at this task:
  dep constraint, analysis section summing to 12,596, proposed-tools table
  matching implementation, per-tool demonstrations, full verification green,
  no rfs toolchain change. Any unmet criterion stops the close-out - a plan
  correction + new task is added instead (PROTOCOL.md section 3 step q).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | Implement the proposed tools + tests | all tool work must be done and verified before close-out records the final state |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | (none) | this is the phase's last task - its completion triggers the phase exit criteria check |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | section 3 row + section 4 row + section 5 checklist note + new workspace baseline numbers |
| `START.md` | change | section 1 snapshot line (Phase 12 complete + new baselines) + section 3 source-tree map `dev_tools/` line |
| `KB.md` | change | section 4.2 file-map row for dev_tools (grep+sed-range edit only) |
| `docs/roadmap/CURRENT_STATE.md` | change | phase 12 row -> COMPLETE with close-out date + files present |
| `TASKS.md` | change | one entry per its first-paragraph template (read ONLY that paragraph) |
| `docs/roadmap/phase_12/PHASE_12_STATUS.md` | new | the phase completion report per FORMATTING.md section 4 |

## Planned actions

### Action 1: full verification + baseline recording
Run the complete checklist with toolchain activation (BUILD.md section 2):
`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`,
then `cargo test --workspace` and `cargo test -p rfs-dev`. Record exact counts:
root package (must equal 535 - unchanged), workspace (old 559 + dev_tools'
tests), dev_tools alone. Verify the rfs package is untouched: its per-suite
rows in BUILD.md section 3 must all still hold. Rationale: the numbers written
into the docs in Action 2 are these measured values - nothing is estimated.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | measurement only - no file changes in this action |

### Action 2: synchronize the tree/command inventories
Edit BUILD.md (section 3: add the `-p rfs-dev` suite row with its exact count;
section 4: add the dev_tools command row; section 5: add the "touched
dev_tools" checklist bullet; replace the known-good baseline paragraph's
workspace number with Action 1's measured value). Edit START.md (section 1:
append the Phase 12 completion line to the snapshot - phases list + new
baselines; section 3: add the `dev_tools/` source-tree line describing the
crate, binary, and the analysis/ directory). Edit KB.md section 4.2 (one file-
map row for dev_tools - locate the section by grep first, edit by sed-range,
never read the whole file). Rationale: these three documents are the
orientation layer every future session reads first - a tree that exists but is
not mapped is a trap for the next planning session.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change (section 3 table) | new `-p rfs-dev` suite row with exact count - the test layout must list every workspace member's suite |
| `BUILD.md` | change (section 4 table) | dev_tools command row (build/test/run invocations) - the command reference stays complete |
| `BUILD.md` | change (section 5 checklist + baseline paragraph) | "touched dev_tools" bullet + measured workspace baseline - post-change verification must name the new crate |
| `START.md` | change (section 1 snapshot) | Phase 12 completion line + updated baselines - the session-start state must be current |
| `START.md` | change (section 3 source tree) | `dev_tools/` map line - the one-level tree inventory gains the new directory |
| `KB.md` | change (section 4.2) | dev_tools file-map row - the source-code map stays complete without reading KB.md fully |

### Action 3: write the phase status report
Write `docs/roadmap/phase_12/PHASE_12_STATUS.md` per FORMATTING.md section 4:
added source files (every dev_tools/src + tests + fixtures file, one row each),
changed files (root Cargo.toml, .gitignore, BUILD.md, START.md, KB.md,
CURRENT_STATE.md, TASKS.md), documentation changes (dev_tools/README.md both
sections, dev_tools/PROTOCOL.md noted as the out-of-scope companion written
after planning), functionality deltas (new: the rfs-dev tool set - hierarchical
bullets per tool; changed/removed: none in the rfs package), tests table (every
dev_tools test file + the suites run with exact counts), found problems
(task-3 analysis deviations, plan corrections, demonstration findings), other
findings (DOCS_STRUCTURE.md decision outcome, port-equality result, anything a
future session should know). Rationale: the status report is the phase's
auditable record - PROTOCOL.md section 3 step r, written once, never rewritten.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_12/PHASE_12_STATUS.md` | new | the FORMATTING.md section-4 completion report - written once at exit criteria, not rewritten afterwards |

### Action 4: update phase state + append the task history
Update `docs/roadmap/CURRENT_STATE.md`: phase 12 row -> COMPLETE (close-out
date, full verification green, files present: PHASE_12.md, PHASE_12_PLAN.md,
tasks/PHASE_12_TASK_{1..6}.md + _STATUS files, PHASE_12_STATUS.md). Append the
TASKS.md entry per its FIRST-paragraph template only (read just that paragraph
- never the whole history): interpretation + report of the close-out. Rationale:
the phase-state table and the task history are the two indexes every future
session consults - both must reflect completion in the same session.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/CURRENT_STATE.md` | change | phase 12 row COMPLETE with the full file list - the state table stays current per INTRO.md |
| `TASKS.md` | change (append only) | close-out entry per the first-paragraph template - append-only history, never rewritten |

## Task deliverables
- Measured verification results (root unchanged at 535; new workspace
  baseline; dev_tools suite count).
- BUILD.md / START.md / KB.md synchronized with the dev_tools tree and commands.
- `PHASE_12_STATUS.md` written per template.
- `CURRENT_STATE.md` phase 12 row COMPLETE; `TASKS.md` entry appended.

## Task exit criteria
- [ ] Full verification green: root `cargo test` = 535 (unchanged), workspace =
      559 + dev_tools' exact test count, clippy without errors, fmt clean - all
      measured and recorded.
- [ ] BUILD.md sections 3/4/5, START.md sections 1/3, and KB.md section 4.2 all
      carry the dev_tools entries with the measured numbers.
- [ ] `PHASE_12_STATUS.md` exists, follows FORMATTING.md section 4 verbatim,
      and its tests-run table matches Action 1's measurements.
- [ ] CURRENT_STATE.md shows phase 12 COMPLETE; TASKS.md carries the close-out
      entry.
- [ ] PHASE_12.md exit criteria checked verbatim - ALL met (any unmet criterion
      stops this task: plan correction + new task per PROTOCOL.md section 3 q).

## Non-goals
- No source changes of any kind in this task (documentation + measurement only;
  a discovered defect goes to a plan-correction task, not to a close-out edit).
- No rewriting of earlier status files or TASKS.md history entries.
- No changes to dev_tools/PROTOCOL.md (the out-of-scope companion stands as
  written after planning; future cycles amend it under their own phases).
