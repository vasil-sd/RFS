# Phase 14, Task 11 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Run the phase's full verification against PHASE_14.md's exit criteria, flip the
phase state in CURRENT_STATE.md, write the TASKS.md entry, and commit per
FORMATTING.md section 7 - the PROTOCOL.md section 3 close-out.

## Context

- PHASE_14.md's exit criteria are the checklist this task verifies verbatim:
  help texts reference only `rfs`; no living doc carries the `cargo run --`
  pattern; the 14 training entries rewritten + regeneration green; the flag
  family + spec-stem default + `--out -` behave per the 23-row contract table;
  C production emits the pair with no FSM duplication and no source-includes;
  Rust mc/test/fuzz-target reference the fsm file; `rfs test` green for both
  backends (unit AND fuzz); c_sync_handler builds via explicit objects with
  unchanged scenario output and fuzz_c builds under clang; full verification
  green (root tests at/above the 535 baseline with any delta explained by new
  tests, generated-code tests passing, fmt clean, clippy at the pre-phase
  baseline of 46 warning lines / 35 unique); docs synchronized (CODEGEN.md +
  backend docs describe the artifact model, user docs show the new CLI, dev_docs
  gate green).
- Baselines to compare against (from START.md's known-good snapshot + Phase 13
  close-out): 535/535 root main tests (663 with --workspace incl. rfs-dev's
  104; 537 with --features smt), 1683/1683 generated-code tests (192 modules),
  clippy 46 warning lines / 35 unique, fmt clean.
- This phase ADDS tests (flag-family coverage, help regression, reference
  assertions) - the root count will exceed 535; the delta must be itemized in
  the status file. The generated-code suite must stay at exactly 1683 (the
  fused `generate_core` contract is preserved by task 5).
- Commit message format: FORMATTING.md section 7 - one-line imperative subject,
  blank line, then bare-label sections `Changes:` / `Issues and findings:` /
  `Verification results:` with factual bullets (files, commands, counts).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-10. | All preceding tasks | Nothing is verified until the work exists. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | (none) | Final task of the phase. |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/CURRENT_STATE.md` | change | Phase 14 row: PLANNING -> COMPLETE with close-out summary. |
| `TASKS.md` | change | Append the phase close-out entry per its first-paragraph template (never rewrite history). |

## Planned actions

### Action 11.1 - Full verification sweep

Run, in order: `cargo build --workspace`; `cargo test` (root - record the exact
count, itemize the delta vs 535); `cargo test --workspace` (record vs 663);
`cargo test --features smt` (record vs 537); `cargo test-generated-code`
(record - must be exactly 1683/1683); `cargo clippy --all-targets` (compare
warning lines/uniques against the 46/35 baseline - new warnings are a defect,
relocated pre-existing ones are not); `cargo fmt --check`; the example-project
suite; the docs gate (`cargo test --test dev_docs_test`); the training trio's
test leg (`cargo test-training`). Spot-check the observable CLI contract:
`rfs --help` and a sample of per-command helps (no `cargo run`), one no-flag
spec-stem generation per backend, one take-or-emit reuse, one partial-pair
error, one `--out -` stdout run.

Rationale: the exit criteria are checked verbatim at close-out (PROTOCOL.md
section 3 step r) - every line needs a recorded result, not an assumption.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (verification only - defect fixes route back to the owning task's corrections) | - | Full sweep. |

### Action 11.2 - Phase bookkeeping + commit

Update `docs/roadmap/CURRENT_STATE.md`'s phase 14 row to COMPLETE with a close-
out summary (verification results, one line). Append the TASKS.md entry per its
first-paragraph template. Commit the phase close-out with a FORMATTING.md
section 7 message: subject summarizing the phase; Changes bullets naming the
CLI surface, the artifact model, the doc/training sync; Issues and findings
bullets from the task status files' findings sections; Verification results
bullets with exact counts.

Rationale: the protocol's close-out - the phase exits only with docs
synchronized, state recorded, and work committed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/CURRENT_STATE.md` | change (phase 14 row) | State -> COMPLETE. |
| `TASKS.md` | change (append entry) | Close-out record. |

## Task deliverables
- Recorded full-verification results against every exit criterion.
- CURRENT_STATE.md phase 14 row flipped to COMPLETE.
- TASKS.md close-out entry + the close-out commit.

## Task exit criteria
- [ ] Every PHASE_14.md exit-criterion line has a recorded pass (counts
      itemized; generated-code suite exactly 1683; clippy at baseline).
- [ ] CURRENT_STATE.md shows phase 14 COMPLETE with the close-out summary.
- [ ] TASKS.md entry appended; commit made in the section-7 format.

## Non-goals
- No new functionality - this task verifies and records only.
- No rewrites of earlier tasks' status files (they are written once).
- No follow-up phase planning (that is a separate user trigger).

## Other information

- If any verification leg fails, do NOT fix it inside this task: open the
  owning task's "Task corrections" table (or a plan correction + new task if
  the gap is plan-level) and re-run that task first - close-out records green,
it does not manufacture it.
- The smt-feature leg needs the z3 environment from TOOLCHAIN.md section 5.2
  (PATH + Z3_LIBRARY_PATH_OVERRIDE) - set it before running, or record the leg
  as SKIPPED with the reason if z3 is unavailable in the environment.
