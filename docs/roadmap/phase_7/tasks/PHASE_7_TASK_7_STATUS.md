# Phase 7, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `tests/common/fail_variants.rs` | pinned FAIL variants shared by `benchmark_mc_test` and `info_format_test`: RING_NO_GRANT_SRC, ELEV_NO_UP_SRC, the two one-line-delta mutations + `spec_source`/`mutate_spec` helpers |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/info_format.rs` | changed - ce-trace field model (trace/lasso/deadlock-config kinds, free-keyed value maps) + entry registration (outputs: md, schema); md section describing the default text trace format (`step N` lines as printed by MC) |
| `src/json_schema.rs` | changed - `FreeObj(inner)` (arbitrary keys; `additionalProperties` = inner schema), `Union(alts)` (plain anyOf; ce_value = boolean\|integer\|string), `SchemaDoc.extra_sections` (md-only sections); `render_md` renders a tagged-union ROOT's kind table |
| `tests/common/mod.rs` | changed - `pub mod fail_variants;` |
| `tests/benchmark_mc_test.rs` | changed - consumes the shared FAIL variants from `tests/common/fail_variants.rs` instead of inlining ~220 lines of variant spec text |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 ce-trace row; baseline re-pin (root 506 / workspace 530 / smt 508) |

## Other file changes

None. NOTE: the TASKS.md entry for task 7 was never recorded (protocol gap -
see the protocol-violation report); `docs/roadmap/PHASE_7_PLAN.md` carries the
corrections instead.

## Changed functionality

- `tests/benchmark_mc_test.rs` now consumes its pinned FAIL variants from the
  shared `tests/common/fail_variants.rs` module (no behaviour change; drift
  prevention).

## Removed functionality

None (the inlined variant text moved, not removed).

## New functionality

- `rfs info format ce-trace --output md|schema`: the MC counterexample document
  (trace/lasso/deadlock-config kinds, free-keyed value maps) described in
  markdown + JSON Schema draft-07, plus the md section for the default text
  trace format.

## Tests add/change/remove

- Added (5): `ce_trace_schema_document_pin`, `ce_trace_md_pins_sections`,
  `ce_trace_walk_trace_and_lasso_both_backends` (pinned FAIL variants on both
  backends), `ce_trace_walk_deadlock_config_both_backends` (example 04, both
  backends), `ce_trace_walk_ref_mc_in_process`.
- Changed: `tests/benchmark_mc_test.rs` now imports the shared variants (same
  tests, same pins).

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 506 pass / 0 fail |
| `cargo test --workspace` | OK - 530 pass / 0 fail |
| `cargo test --features smt` | OK - 508 pass / 0 fail (z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 18 tests (+5) |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- Plan name drift: `mc_report_json.rs` does not exist - the `JSON_KEY_*`
  constants and counterexample JSON builder live in `src/codegen_mc.rs` (plan
  correction 7.1).
- The task 6 field model could not express ce-trace's name->value maps -
  `FreeObj`/`Union`/`extra_sections` added to `json_schema.rs` (plan correction
  7.1); the walker enforces value types on free-keyed objects but never their
  keys.
- No benchmark_mc_test FAIL variant produces a deadlock, so the
  `deadlock-config` kind needed the example 04 case backend_parity_test already
  pins (plan correction 7.1/7.2).

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `c5c5308` message + diff, `PHASE_7_PLAN.md` corrections table, BUILD.md
  baseline re-pins.
- Committed as `c5c5308` (2026-10-27), 8 files changed (+1242/-376).
