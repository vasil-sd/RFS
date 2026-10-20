# Phase 5, Task 16 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Close out Phase 5: full verification per BUILD.md section 5 (+ Phase 5
additions: gcc -Wall spot checks of new C output, no_std compile sweep,
workspace build of all example crates), write `docs/roadmap/PHASE_5_STATUS.md`
per template 2.3 with exact counts, set INTRO.md state-table row 5 -> COMPLETE,
add the TASKS.md entry, single commit, clean tree.

## Context

- Verification (BUILD.md section 5 + Phase 5 additions): `cargo build && cargo
  test && cargo clippy --all-targets && cargo fmt --check` at the WORKSPACE
  root; every `.cargo/config.toml` alias still works (task 1's guarantee);
  `cargo generate-code-tests && cargo test-generated-code`; `cargo run --
  validate --input rfs examples/valid` (exit 0); training chain (`extract-
  training-snippets && build-training && test-training` - task 14 changed
  data, MUST run); no `serde`/`serde_json` anywhere (`Cargo.toml` + grep over src/tests)
  and the IR JSON round-trip golden tests green (task 2's migration holds); workspace build of all example crates (`cargo test -p`
  where applicable); `gcc -Wall` spot checks of the NEW C output (extended test
  artifacts + c_sync_handler + libFuzzer generation); no_std compile sweep
  (task 9's test covers it - re-run); SMT feature suite still green
  (`cargo test --features smt`, z3 env per TOOLCHAIN.md section 5.2) - Phase 5
  must not have broken the optional mode; `--print-script` smoke on `rfs test`
  and `rfs diagram`.
- Baseline to supersede (Phase 4 close-out, 2026-10-20): 230/230 main tests
  (232 with smt), 537/537 generated tests (179 modules), 45 valid / 12 invalid
  examples, training aggregate 231 lines. Expected growth: new frontend/info/
  diagram/ext_tool tests, grown aggregate crate (tasks 5/6), grown training
  aggregate (task 13) - record the FINAL exact numbers from this task's runs.
- `PHASE_5_STATUS.md` per INTRO.md template 2.3: added/changed/removed files
  (source/docs/other), changed/removed/new functionality, tests add/change/
  remove, tests run status (exact counts), found problems/issues/mismatches.
- Collect from the per-task STATUS files: workspace/alias record (task 1),
  count re-pins (tasks 5/6), no_std sweep results (task 9), JSON parity cases
  (task 10), example-crate build records (tasks 11/12), training counts
  (task 13), plan corrections (plan file table).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15. | Docs sync | docs must be final before the status file describes them |
| 1.-14. | All implementation tasks | verification covers everything shipped |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/PHASE_5_STATUS.md` | new | phase status per template 2.3 with exact counts |
| `docs/roadmap/INTRO.md` | change | state table row 5 -> COMPLETE |
| `TASKS.md` | change | one entry per the template in its first paragraph |

## Planned actions

### Action 1: Full verification
Run the complete checklist from Context. Fix any failure at its source (never
by weakening a test); re-run until green. If verification surfaces real bugs,
fix them as their own small step and record them in PHASE_5_STATUS.md "Found
problems/issues/mismatches" - do not fold silent fixes into the close-out
commit without recording them.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (fixes at their source, if any) | change | verification failures resolved properly |

### Action 2: PHASE_5_STATUS.md
Write the status file per template 2.3 with EXACT counts from this task's runs
(main tests, generated tests + module count, examples valid/invalid, training
counts, suite results) and the collected per-task data (Context).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/PHASE_5_STATUS.md` | new | phase close-out record |

### Action 3: Bookkeeping
INTRO.md state-table row 5 -> COMPLETE (with the one-line Phase 5 summary +
file list); TASKS.md entry per its template.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/INTRO.md` | change | state table row 5 |
| `TASKS.md` | change | close-out entry |

### Action 4: Single commit + clean tree
One commit covering the status file + bookkeeping (implementation tasks already
committed individually per protocol step t); `git status` clean.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (commit) | - | close-out commit, clean tree |

## Task deliverables
- Green full verification; `PHASE_5_STATUS.md` with exact counts; INTRO state
  table updated; TASKS.md entry; single close-out commit; clean tree.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green at the workspace root (exact final counts recorded); all cargo aliases work.
- [ ] `cargo generate-code-tests && cargo test-generated-code` green (exact final counts + module count recorded).
- [ ] `cargo run -- validate --input rfs examples/valid` exit 0 (45 valid / 12 invalid - or the grown count if examples were added).
- [ ] Training chain green with task 14's final counts (if data changed).
- [ ] No `serde`/`serde_json` in Cargo.toml or any source/test (grep check); IR JSON round-trip golden tests green.
- [ ] Workspace build of all example crates green; gcc -Wall spot checks of new C output clean; no_std sweep green.
- [ ] `cargo test --features smt` still green (Phase 4 optional mode unbroken).
- [ ] `--print-script` smoke: `rfs test --print-script` and `rfs diagram --format svg --print-script` emit scripts without executing.
- [ ] `PHASE_5_STATUS.md` complete per template 2.3 (all sections, exact numbers).
- [ ] INTRO.md state-table row 5 COMPLETE; TASKS.md entry added.
- [ ] Single close-out commit; `git status` clean.

## Non-goals
- No new functionality in this task - bookkeeping + verification only.

## Other information
- If a count disagrees with what task 14 recorded in the docs, the VERIFIED
  number wins - fix the doc in this task and note it under found problems.
