# Phase 7, Task 11 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_11.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/info_format.rs` | changed - `DOT_SUBSET_MD` (fixed 3-line header, node line shapes with filled/fillcolor IFF initial state, edge label grammar `Event [guard] -> next`, dashed on_error edges, quote escaping) + `diagram_dot_render` + entry registration (outputs: md) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 diagram-dot row; baseline re-pin (root 520 / workspace 544 / smt 522) |

## Other file changes

None. NOTE: the TASKS.md entry for task 11 was never recorded (protocol gap -
see the protocol-violation report).

## Changed functionality

None (new command surface only; existing behaviour untouched).

## Removed functionality

None.

## New functionality

- `rfs info format diagram-dot --output md`: the generated DOT subset
  documented in markdown, derived from `diagrams/dot.rs` `generate` (the single
  source of truth).

## Tests add/change/remove

- Added (2): `diagram_dot_md_pins_sections`,
  `diagram_dot_generated_output_matches_documented_subset` (coverage sync over
  all 50 valid examples x every module: every generated line is one of the
  documented shapes filled with IR data - no undocumented tokens; corpus pins
  for guard/struct-event/on_error coverage) + the `check_dot_documented_subset`
  helper.
- Changed: none.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 520 pass / 0 fail |
| `cargo test --workspace` | OK - 544 pass / 0 fail |
| `cargo test --features smt` | OK - 522 pass / 0 fail (z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 32 tests (+2) |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

None recorded.

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `536c8e8` message + diff, BUILD.md baseline re-pins.
- Committed as `536c8e8` (2026-10-27), 3 files changed (+292/-10).
