# Phase 11, Task 14 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Run full verification on the final tree, confirm the phase diff is in-scope,
and write the close-out bookkeeping (status file, CURRENT_STATE row, TASKS.md
entries).

## Context
- Full gate: cargo test (535 baseline - the determinism-pinning suites make
  codegen byte-identity a tested invariant, so green proves "no functionality
  change" for the whole phase), cargo clippy --all-targets (no new warnings),
  cargo fmt --check (clean).
- Scope proof: git diff --stat over the whole phase (from the phase-start
  commit) must show ONLY: source moves/edits per the mapping, the two inventory
  documents (moved to docs/llm/), tests/dev_docs_manifest.txt, docs/**/*.md
  reference re-syncs, PROTOCOL.md + PRINCIPLES.md, roadmap bookkeeping, TASKS.md.
  Any other file is a stop-and-report finding.
- Status file per FORMATTING.md section 4; CURRENT_STATE.md row 11 -> COMPLETE
  with close-out date; TASKS.md entries for every task of the phase (one entry
  per executed task - the Phase 10 close-out gap where tasks 32-33 committed
  without entries must not repeat).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-13. | all preceding tasks | the tree + documents are final |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | - | final task of the phase |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/phase_11/PHASE_11_STATUS.md` | new | phase completion report per FORMATTING.md section 4 |
| `docs/roadmap/CURRENT_STATE.md` | change | row 11 -> COMPLETE with close-out date |
| `TASKS.md` | change | one entry per executed task + this close-out |

## Planned actions

### Full verification + scope proof
Run the three gates; run git diff --stat over the phase and check every changed
path against the allowed set. Rationale: the exit criteria demand both a green
baseline and an in-scope diff.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Close-out bookkeeping
Write PHASE_11_STATUS.md (files added/changed/removed, functionality deltas =
none, tests run status, findings incl. any plan corrections); set CURRENT_STATE.md
row 11 to COMPLETE; append the TASKS.md entries. Rationale: protocol steps r-t.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_11/PHASE_11_STATUS.md` | new | the phase completion report |
| `docs/roadmap/CURRENT_STATE.md` | change | row 11 COMPLETE |
| `TASKS.md` | change | per-task entries + close-out entry |

## Task deliverables
- Green full verification; in-scope phase diff; PHASE_11_STATUS.md; updated
  CURRENT_STATE.md; complete TASKS.md record.

## Task exit criteria
- [ ] cargo test 535/535, clippy --all-targets no new warnings, fmt --check clean.
- [ ] git diff --stat over the phase shows only in-scope files (checked path by path).
- [ ] PHASE_11_STATUS.md written per FORMATTING.md section 4.
- [ ] CURRENT_STATE.md row 11 -> COMPLETE with close-out date.
- [ ] TASKS.md carries an entry for every executed task of the phase + this close-out.

## Non-goals
- No source or doc change beyond bookkeeping; any fix found here is a plan
  correction loop back to the owning task, not an inline edit.
