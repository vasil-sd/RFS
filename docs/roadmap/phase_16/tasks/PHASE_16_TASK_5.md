# Phase 16, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope. Shared context (baselines, nix discipline) lives in
> `PHASE_16_PLAN.md` ("Context").

## Goal
Close out Phase 16: run the complete exit-criteria sweep (canonical + nix), write
`PHASE_16_STATUS.md` per FORMATTING.md section 4, append the `TASKS.md` entry, and
update `CURRENT_STATE.md`. The phase exits only with all criteria met and the record
kept.

## Context
- **Baselines to preserve (Phase 15 close-out):** `cargo test` 557/0,
  `--workspace` 685/0, `--features smt` 559/0, generated-code 1683/1683, clippy 55
  warning lines / 0 errors (34 unique), fmt clean; nix doCheck build 42/42; nix
  develop full smt suite 559/0. Tasks 2-4 ADD tests - record the new totals in the
  status file, never silently change the old ones.
- **Nix verification discipline (Phase 15 close-out finding):** after any HOST
  canonical sweep, `target/` holds host-rustc rlibs that make nix-develop runs fail
  E0514 in `nostd_minimal.rs` - wipe `target/` before nix develop suite runs. Nix
  binary: `/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin/nix` (not on
  PATH), `--impure` required.
- **Known sandbox artifact:** the external mmd half of diagram_render_parity_test
  needs a short TMPDIR in this sandbox (Chrome socket path artifact - pre-existing,
  unrelated to this phase).
- **Status-file content obligations (FORMATTING.md section 4):** files
  added/changed/removed, functionality deltas (the three new modes + the refactor +
  the shared wire formats), tests run with EXACT counts, findings - including the
  pinned nondet liveness fairness semantics (task 2), the pick_index-already-exists
  finding (task 3), any draw-sequence or encoding corrections, and the
  both-z3-versions parity result (task 4).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-4. | split / nondet / fuzz / smt | all implementation work must be complete and committed before the close-out sweep |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/phase_16/PHASE_16_STATUS.md` | new | the phase status record per FORMATTING.md section 4 |
| `TASKS.md` | change | append the phase-16 entry per its first-paragraph template |
| `docs/roadmap/CURRENT_STATE.md` | change | phase-16 row |

## Planned actions

### 1. Run the complete exit-criteria sweep
Host: `cargo test` (557 + new tests), `cargo test --workspace` (685 + new),
`cargo test --features smt` (559 + new), the generated-code suite (1683/1683 - must
be UNCHANGED, proving template interpolation moved no bytes), fmt check, clippy
(baseline: 55 warning lines / 34 unique), `rfs-dev inventory audit` EXACT,
`rfs-dev sync-check` clean. Nix: wipe `target/`, then doCheck build (42/42) and the
dev-shell full smt suite (green). Rationale: PROTOCOL.md section 3 steps r-u - the
phase exits only with all criteria met; the unchanged 1683 count is the byte-identity
proof for tasks 3-4's template changes.

#### Affected files

(none - verification only)

### 2. Write PHASE_16_STATUS.md, append TASKS.md, update CURRENT_STATE.md
Write the status file per FORMATTING.md section 4 with the exact test counts and all
findings (Context bullet 3); append the TASKS.md entry per its template; add the
phase-16 row to CURRENT_STATE.md. Rationale: the record must let a future session
reconstruct what changed, why, and what was verified without re-running anything.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_16/PHASE_16_STATUS.md` | new | the complete phase record |
| `TASKS.md` | change | phase-16 entry appended |
| `docs/roadmap/CURRENT_STATE.md` | change | phase-16 row |

## Task deliverables
- A green complete sweep: canonical counts (old baselines preserved + new tests
  recorded), 1683/1683 generated-code byte-identity, fmt/clippy at baseline,
  inventory EXACT, sync-check clean, nix doCheck 42/42, dev-shell smt suite green.
- `docs/roadmap/phase_16/PHASE_16_STATUS.md` written per FORMATTING.md section 4.
- The `TASKS.md` entry and the `CURRENT_STATE.md` phase-16 row.

## Task exit criteria
- [ ] Full canonical sweep green: cargo test / --workspace / --features smt at recorded new totals (old baselines preserved), generated-code 1683/1683 UNCHANGED, fmt clean, clippy at baseline (55 warning lines / 34 unique).
- [ ] `rfs-dev inventory audit` EXACT and `rfs-dev sync-check` clean.
- [ ] Nix doCheck build 42/42 and the dev-shell smt suite green (fresh target after host runs).
- [ ] PHASE_16_STATUS.md written per FORMATTING.md section 4 with exact counts + all findings (pinned nondet liveness semantics, pick_index finding, both-z3-versions parity result).
- [ ] TASKS.md entry appended; CURRENT_STATE.md phase-16 row updated.

## Non-goals
- No source changes in this task - if the sweep finds a defect, it is fixed as a plan correction in the responsible task's file (recorded there), not silently here.
- No new documentation beyond the status/bookkeeping files.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- If any canonical count moved unexpectedly, STOP and diagnose before writing the
  status file - a moved baseline is either a missed test registration or an
  accidental behavior change, both of which belong in a task correction, not in the
  status narrative.
- The nix half of ref_mc_smt_test (task 4) must be re-run here as part of the
  dev-shell suite so the status file records a fresh both-versions result, not just
  task 4's.
