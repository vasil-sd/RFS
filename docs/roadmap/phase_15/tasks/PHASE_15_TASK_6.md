# Phase 15, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Close out Phase 15: prove the canonical toolchain baselines are byte-unchanged
(no RFS source was touched), re-confirm the nix-side exit criteria end to end,
write `PHASE_15_STATUS.md`, append the `TASKS.md` entry, and update
`CURRENT_STATE.md` - per PROTOCOL.md §3 steps r-u.

## Context
- **Canonical baselines (Phase 14 close-out, the numbers this phase must
  preserve):** `cargo test` **548**/0, `cargo test --workspace` **676**/0,
  `cargo test --features smt` **550**/0, `cargo test-generated-code`
  **1683**/1683, fmt clean, clippy at the Phase 14 baseline (55 warning lines /
  34 unique with clippy 1.99.0). BUILD.md §5's "known-good baseline" text still
  shows the older 538/666/540 (re-measured 2026-11-12, pre-Phase-14) - do not
  "correct" it in this phase; note the drift in the status file if it matters.
- **Why the canonical sweep is still mandatory:** the phase changed no `.rs`
  file, so the baselines hold by construction - but the sweep is the PROOF and
  catches accidental edits. Run it under the canonical toolchain (START.md §4
  activation block), NOT under nix.
- **Nix-side re-confirmation:** tasks 2-4 each verified their own slice; this
  task re-runs the PHASE_15.md exit-criteria list verbatim as one sweep:
  package build + closure proof, wrapper override matrix, bare-install `rfs
  test` operation, shell full-suite matrix (task 4's eight commands),
  TOOLCHAIN.md §10 + BUILD.md §8 retrievable via rfs-dev doc-section.
- **Bookkeeping templates:** PHASE_15_STATUS.md follows FORMATTING.md §4
  exactly (Added/Changed source files - expected EMPTY tables or "none",
  documentation files, other file changes for the nix files, functionality
  deltas, tests run status with exact counts, findings). The TASKS.md entry
  follows the template in TASKS.md's FIRST paragraph only (never read the whole
  history). CURRENT_STATE.md's phase-15 row moves from PLANNED to COMPLETE
  with the close-out summary.
- **Commit discipline:** one commit per task already happened in tasks 1-5;
  this task commits the status file + bookkeeping together (PROTOCOL.md §3
  step u), subject like "Phase 15 close-out: full Nix development support".

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-5. | all previous tasks | the close-out verifies their combined result against the phase exit criteria |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/phase_15/PHASE_15_STATUS.md` | new | phase completion report per FORMATTING.md §4 |
| `docs/roadmap/CURRENT_STATE.md` | change | phase-15 row: PLANNED -> COMPLETE with close-out summary |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TASKS.md` | change (append) | one entry per its first-paragraph template |

## Planned actions

### 1. Canonical-toolchain verification sweep
Under the START.md §4 activation block: `cargo build && cargo test && cargo
test --workspace && cargo test --features smt && cargo test-generated-code &&
cargo clippy --all-targets && cargo fmt --check`. All counts must match the
Phase 14 baselines exactly (548/676/550/1683, clippy at baseline, fmt clean).
Any deviation is a phase defect - stop and raise a plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### 2. Nix-side exit-criteria sweep
Re-run, as one pass: (a) `nix build --impure --expr 'import ./nix/package.nix { }'`
(doCheck=true) green; (b) closure proof via `nix path-info -r` (all tool
binaries present, no chromium by default); (c) wrapper matrix - bare-install
`rfs test --suite unit` both backends + `--suite fuzz --backend c`, user
override wins via `--print-script`, optional-args diagram vars set/unset;
(d) `nix develop -f shell.nix` full-suite matrix (task 4's eight commands with
documented counts); (e) `rfs-dev doc-section TOOLCHAIN.md '10. Nix package
manager on this Ubuntu sandbox'` and `rfs-dev doc-section BUILD.md '8.
Building, testing and running RFS with Nix'` both return their sections;
(f) `rfs-dev sync-check` clean + `rfs-dev inventory audit` EXACT (proof no
source/doc-pointer drift).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### 3. Write PHASE_15_STATUS.md, update TASKS.md + CURRENT_STATE.md, commit
Write the status file per FORMATTING.md §4: empty/"none" source-file tables
(packaging-only phase), documentation files table (TOOLCHAIN.md §10, BUILD.md
§8), other file changes (nix/package.nix, shell.nix, nix/z3-unified.nix or the
documented fallback), changed/new functionality (parameterized package,
propagated tool closure, wrapper contract, full-parity shell), tests run status
with EXACT counts from actions 1-2, findings (libFuzzer probe result, the
clang-answering-for-gcc resolution, any D1-D3 decisions as implemented, the
BUILD.md §5 stale-baseline drift note). Append the TASKS.md entry. Update the
CURRENT_STATE.md row to COMPLETE with a one-line close-out summary. Commit all
three together.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_15/PHASE_15_STATUS.md` | new | the phase record - FORMATTING.md §4 template, written once, never rewritten |
| `TASKS.md` | change (append at EOF) | the session's task entry per its first-paragraph template |
| `docs/roadmap/CURRENT_STATE.md` | change (phase-15 row) | PLANNED -> COMPLETE so the phase-state table stays current |

## Task deliverables
- Green canonical sweep + green nix sweep (counts recorded).
- `PHASE_15_STATUS.md`, the `TASKS.md` entry, the updated `CURRENT_STATE.md`
  row - committed.

## Task exit criteria
- [ ] Canonical baselines exactly preserved: 548/676/550/1683, fmt clean,
      clippy at baseline.
- [ ] Every PHASE_15.md exit-criteria item re-confirmed in one sweep (package
      build + closure, wrapper matrix, bare-install operation, shell full-suite
      matrix, both new doc sections retrievable, sync-check clean, inventory
      EXACT).
- [ ] `PHASE_15_STATUS.md` written per FORMATTING.md §4 with exact test counts.
- [ ] `TASKS.md` entry appended; `CURRENT_STATE.md` phase-15 row COMPLETE.
- [ ] Close-out commit made per FORMATTING.md §7.

## Non-goals
- No fixes to anything found - if a criterion fails, the task corrects the PLAN
  ("Plan corrections" table) and adds/re-runs tasks; it does not silently patch
  scope. (The one expected exception: trivial doc typos in this phase's own new
  sections, recorded as task corrections here.)
- No changes to BUILD.md §5's stale baseline text (pre-existing drift, noted in
  the status file for a future docs-refresh task).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- This is the only task that writes PHASE_15_STATUS.md - tasks 1-5 write their
  own `PHASE_15_TASK_M_STATUS.md` files at their close (PROTOCOL.md §3 step m).
