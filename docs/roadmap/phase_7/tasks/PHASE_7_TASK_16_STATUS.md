# Phase 7, Task 16 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_16.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (close-out task).

## Changed/removed source files

None (close-out task - verification + docs only).

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_7_STATUS.md` | new | phase status per template 2.3: file lists, functionality deltas, pointer to D-P7-1..D-P7-13 (DESIGN.md section 25), full verification matrix results, pinned plantuml-core rev |
| `docs/roadmap/PHASE_7_TASK_16_STATUS.md` | new | this file |
| `BUILD.md` | changed | section 5 baseline re-measured at close-out (2026-10-28): training aggregate 261 -> 264, clippy warning breakdown (46 lines / 35 unique), date/phase reference updated; section 3 suite rows already current from tasks 4-14 |
| `KB.md` | changed | lint/format line fixed: 36 -> 46 warning lines / 35 unique with the exact file distribution |
| `START.md` | changed | Phase 7 completion summary + known-good baseline re-measured at close-out (2026-10-28) |
| `DESIGN.md` | changed | new section 25: D-P7-1..D-P7-13 recorded as shipped + execution notes (drift from plan) |
| `docs/roadmap/INTRO.md` | changed | phase-state table row 7 -> COMPLETE (close-out 2026-10-28) |
| `docs/user/README.md`, `docs/user/INTEGRATION.md`, `docs/user/MIGRATION.md` | changed | user-facing: in-process SVG default, `info format` family, v1 -> v1.1 migration section |

## Other file changes

None (the training pipeline was re-run as a consistency check; output byte-identical to task 15's commit - aggregate still 264 lines).

## Changed functionality

None at runtime (close-out task - verification + docs sync only).

## Removed functionality

None.

## New functionality

None (all Phase 7 functionality was introduced in tasks 1-15; this task documents and verifies it).

## Tests add/change/remove

None.

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (526) | OK |
| `cargo test --workspace` (550) | OK |
| `cargo test --features smt` (528) | OK |
| `cargo test-generated-code` (1683, 192 modules) | OK |
| `cargo test --test examples_test` (3; 50 valid + 13 invalid) | OK |
| training pipeline: extract + build + `cargo test-training` (5; aggregate 264 lines, snippets 138+128 unchanged) | OK |
| `cargo clippy --all-targets` (no errors; 46 warning lines / 35 unique - dead-code in shared test modules + pre-existing patterns; zero in Phase 7's new source files and the two new test suites) | OK |
| `cargo fmt --check` | OK |
| `cargo test --test diagram_render_parity_test` (13; internal half all 50 examples x 3 formats + external half pinned subset x dot/plantuml/mmdc - all three tools installed and ran) | OK |

## Found problems/issues/mismatches
- The interrupted close-out session left the BUILD.md section 5 baseline stale (task 14 date, aggregate 261) and the KB.md lint line stale (36 warnings vs the actual 46 lines / 35 unique) - both re-measured and fixed in this task.
- First verification run failed with `cargo: command not found` - the toolchain activation env vars (`RUSTUP_HOME`/`CARGO_HOME`/`PATH` per BUILD.md section 2) must be exported in every shell; re-ran with them.

## Other important findings/information
- Full verification matrix green on the final tree (2026-10-28) - all counts match the baseline re-pinned by tasks 13/14/15; training aggregate confirmed at 264 lines.
- External diagram tools (dot/plantuml/mmdc + java) are installed in this environment, so the availability-gated external parity half ran fully (not skipped).
- `docs/TODO.txt` / `docs/TODO1.txt` are untracked user scratch notes - intentionally left alone and not committed.
