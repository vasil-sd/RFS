# Phase 4, Task 14 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Close out Phase 4: full verification per BUILD.md section 5 (+ `gcc -Wall` spot
check of the NEW C MC output), write `docs/roadmap/PHASE_4_STATUS.md` per template
2.3 with exact counts, set INTRO.md state-table row 4 -> COMPLETE, add the TASKS.md
entry, single commit, clean tree.

## Context

- Verification (BUILD.md section 5): `cargo build && cargo test && cargo clippy
  --all-targets && cargo fmt --check`; `cargo generate-code-tests && cargo
  test-generated-code`; `cargo run -- validate --input rfs examples/valid` (exit 0);
  training chain if task 12 changed data (`cargo extract-training-snippets && cargo
  build-training && cargo test-training`); `gcc -Wall` spot check of the NEW C MC
  output (exact engine + liveness, not just Phase 3's).
- Baseline to supersede (2026-10-17, after Phase 3): 170/170 main tests, 513/513
  generated tests, 42 valid / 12 invalid examples. Expected growth: +3 examples
  (43-45), new benchmark suite, liveness parser/validator tests - record the FINAL
  exact numbers from this task's runs.
- `PHASE_4_STATUS.md` per INTRO.md template 2.3: brief explanation, influenced /
  affected tasks, added/changed/removed files (source/docs/other), changed/removed/
  new functionality, tests add/change/remove, tests run status (exact counts),
  found problems/issues/mismatches.
- Collect from the per-task STATUS files: re-pinned values (task 4), sizing data
  (tasks 6/8), SMT gate outcome (task 11), training counts (task 12), plan
  corrections (plan file table).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Docs sync | docs must be final before the status file describes them |
| 1.-12. | All implementation tasks | verification covers everything shipped |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/PHASE_4_STATUS.md` | new | phase status per template 2.3 with exact counts |
| `docs/roadmap/INTRO.md` | change | state table row 4 -> COMPLETE |
| `TASKS.md` | change | one entry per the template in its first paragraph |

## Planned actions

### Action 1: Full verification
Run the complete BUILD.md section 5 checklist + the `gcc -Wall` spot check of the
new C MC output (exact engine + liveness paths). Fix any failure at its source
(never by weakening a test); re-run until green.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (fixes at their source, if any) | change | verification failures resolved properly |

### Action 2: PHASE_4_STATUS.md
Write the status file per template 2.3 with EXACT counts from this task's runs
(main tests, generated tests, examples valid/invalid, benchmark suite results,
training counts) and the collected per-task data (Context).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/PHASE_4_STATUS.md` | new | phase close-out record |

### Action 3: Bookkeeping
INTRO.md state-table row 4 -> COMPLETE (with the one-line Phase 4 summary); TASKS.md
entry per its template.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/INTRO.md` | change | state table row 4 |
| `TASKS.md` | change | close-out entry |

### Action 4: Single commit + clean tree
One commit covering the status file + bookkeeping (implementation tasks already
committed individually per protocol step t); `git status` clean.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (commit) | - | close-out commit, clean tree |

## Task deliverables
- Green full verification; `PHASE_4_STATUS.md` with exact counts; INTRO state table
  updated; TASKS.md entry; single close-out commit; clean tree.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green (exact final counts recorded).
- [ ] `cargo generate-code-tests && cargo test-generated-code` green (exact final counts recorded).
- [ ] `cargo run -- validate --input rfs examples/valid` exit 0 (45 valid / 12 invalid).
- [ ] Training chain green with task 12's final counts (if data changed).
- [ ] `gcc -Wall` spot check of the new C MC output clean.
- [ ] `PHASE_4_STATUS.md` complete per template 2.3 (all sections, exact numbers).
- [ ] INTRO.md state-table row 4 COMPLETE; TASKS.md entry added.
- [ ] Single close-out commit; `git status` clean.

## Non-goals
- No new functionality in this task - bookkeeping + verification only.

## Other information
- If verification surfaces a real bug, fix it at the source as its own small step,
  re-run full verification, and note it in PHASE_4_STATUS.md "Found
  problems/issues/mismatches" - do not fold silent fixes into the close-out commit
  without recording them.
