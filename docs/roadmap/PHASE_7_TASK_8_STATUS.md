# Phase 7, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/info_format.rs` | changed - mc-report field model (status/metrics/fail fields/statistics) + entry registration (outputs: md, schema); ce-trace root + sub-definitions extracted into the shared `counterexample_model()` builder, reused here via `$ref` |
| `src/json_schema.rs` | changed - `emit_type` made pub (cross-check of the reused counterexample definition) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 mc-report row; baseline re-pin (root 510 / workspace 534 / smt 512) |

## Other file changes

None. NOTE: the TASKS.md entry for task 8 was never recorded (protocol gap -
see the protocol-violation report); `docs/roadmap/PHASE_7_PLAN.md` carries the
corrections instead.

## Changed functionality

- The ce-trace entry's root + sub-definitions now live in a shared
  `counterexample_model()` builder (no output change for `ce-trace`; the
  mc-report document reuses it).

## Removed functionality

None.

## New functionality

- `rfs info format mc-report --output md|schema`: the full exact-mode MC report
  document (status/reason enums derived from the `wire()` methods, fail fields
  optional IFF-fail, statistics, counterexample via `$ref` to the shared
  ce-trace model) described in markdown + JSON Schema draft-07.

## Tests add/change/remove

- Added (4): `mc_report_schema_document_pin`, `mc_report_md_pins_sections`,
  `mc_report_walk_benchmarks_both_backends` (6 benchmark specs pass + FAIL
  variants + deadlock + unknown-state-limit on both backends),
  `mc_report_walk_ref_mc_in_process`.
- Changed: none.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 510 pass / 0 fail |
| `cargo test --workspace` | OK - 534 pass / 0 fail |
| `cargo test --features smt` | OK - 512 pass / 0 fail (z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 22 tests (+4) |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- The status/reason enum values have no pub string constants (only
  `ExactStatus::wire()` / `FailReason::wire()` methods) - the model derives
  them from the methods at build time so a wire-name change follows
  automatically (plan correction 8.1).
- Fail fields (`reason`/`label`/`step`/`counterexample`) are modeled OPTIONAL
  (draft-07 without if/then conditionals); their IFF-fail presence is pinned by
  the walk test over real runs instead (plan correction 8.1).

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `aa49438` message + diff, `PHASE_7_PLAN.md` corrections table, BUILD.md
  baseline re-pins.
- Committed as `aa49438` (2026-10-27), 5 files changed (+689/-115).
