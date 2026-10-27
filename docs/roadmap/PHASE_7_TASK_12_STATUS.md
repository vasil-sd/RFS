# Phase 7, Task 12 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_12.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/info_format.rs` | changed - `PUML_SUBSET_MD` (frame lines with IFF-initial `[*] -->` line, transition label grammar `Event \n guard` with the literal two-char `\n`, note-on-link lines for emitted outputs, dashed bracketed on_error edges, verbatim names without escaping) + `diagram_puml_render` + entry registration (outputs: md) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 diagram-puml row; baseline re-pin (root 522 / workspace 546 / smt 524) |

## Other file changes

None. NOTE: the TASKS.md entry for task 12 was never recorded (protocol gap -
see the protocol-violation report).

## Changed functionality

None (new command surface only; existing behaviour untouched).

## Removed functionality

None.

## New functionality

- `rfs info format diagram-puml --output md`: the generated PlantUML subset
  documented in markdown, derived from `diagrams/plantuml.rs` `generate` (the
  single source of truth).

## Tests add/change/remove

- Added (2): `diagram_puml_md_pins_sections`,
  `diagram_puml_generated_output_matches_documented_subset` (coverage sync over
  all 50 valid examples x every module: every generated line is one of the
  documented shapes filled with IR data - no undocumented tokens; corpus pins
  for guard/struct-event/on_error/note-on-link coverage) + the
  `check_puml_documented_subset` helper.
- Changed: none.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 522 pass / 0 fail |
| `cargo test --workspace` | OK - 546 pass / 0 fail |
| `cargo test --features smt` | OK - 524 pass / 0 fail (z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 34 tests (+2) |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

None recorded.

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `24c84f9` message + diff, BUILD.md baseline re-pins.
- Committed as `24c84f9` (2026-10-27), 3 files changed (+303/-10).
