# Phase 3, Task 15 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_15.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | tests only, in an existing file |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs` | changed | 4 new tests pinning D-P3-6: `rust_production_flag_toggles_property_monitors` (after `all_rust_commands_generate_for_valid_examples`), `c_production_flag_toggles_property_monitors` (after `all_c_commands_generate_for_valid_examples`), `mc_and_test_always_include_property_monitors` + `no_skip_warnings_for_in_subset_properties_both_backends` (new "Phase 3: property-monitor flag semantics" section at end of file); no existing test modified |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_3_TASK_15.md` | changed | exit criteria checked (all four); "Task corrections" section appended (off-output marker calibration, observed CLI rejection for mc/test, stale 3-example list) |
| `docs/roadmap/PHASE_3_TASK_15_STATUS.md` | new | this file |

## Other file changes

- Scratch test `tests/tmp_task15_scan.rs` was created to empirically scan all
  property-bearing examples (result: 15 files x both backends, zero warnings)
  and DELETED after the scan; it is not part of the change set.

## Changed functionality

None (tests only, per task scope).

## Removed functionality

None.

## New functionality

None in product code. New pinned behavior (4 tests):
- Rust production: flag on adds `StepError::PropertyViolation { label, step }`
  variant + `PropertyViolation` struct + `PropMonitors` aggregate, all under
  `#[cfg(debug_assertions)]`; flag off contains none of them and no
  `#[cfg(debug_assertions)]` at all (zero-cost-when-disabled).
- C production: flag on adds `#define RFS_PROPERTY_MONITORS 1` +
  `#ifdef RFS_PROPERTY_MONITORS`-gated monitor code (`rfs_property_violation_t`,
  per-label `mon_*` functions, `prop_monitors` FSM state); flag off has none of
  them; the `rfs_status_t` wire format (incl. `RFS_PROPERTY_VIOLATION`) is
  stable in both modes.
- mc/test in BOTH backends include monitors unconditionally with the flag off
  (Rust: `PropMonitors` + StepError variant; C: `rfs_property_violation_t` +
  `prop_monitors`), and the CLI REJECTS `--with-property-monitors` for mc/test
  with "only valid for the production command" (D-P3-6 not-accepted path).
- All 15 property-bearing valid examples (top-level 04, 13, 14, 15, 16, 24,
  37, 41, 42 + tla_adapted 31-36; enumerated dynamically) compile production
  with the flag on in BOTH backends with zero "outside monitor subset" warnings.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `rust_production_flag_toggles_property_monitors` | new | Action 1: example 41 production off/on differ; on has StepError variant + PropertyViolation struct + PropMonitors + debug gating; off has none of them |
| `c_production_flag_toggles_property_monitors` | new | Action 2: same for C - on has define + ifdef gate + violation type + prop_monitors; off has none; rfs_status_t ABI stable in both |
| `mc_and_test_always_include_property_monitors` | new | Action 3: mc+test x rust+c with flag off contain monitor code; CLI rejects the flag for mc/test (error message asserted) |
| `no_skip_warnings_for_in_subset_properties_both_backends` | new | Action 4: dynamic enumeration of property-bearing valid examples; production flag-on in both backends must emit no "outside the monitor subset" warnings (C bit-width rejections tolerated, matching existing convention) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` lib unit tests (86) | OK |
| `tests/codegen_test.rs` (23; was 19 before this task) | OK |
| `tests/examples_test.rs` (3) | OK |
| `tests/frontend_test.rs` (41) | OK |
| `tests/integration_example_test.rs` (2) | OK |
| `tests/library_comparison.rs` (4) | OK |
| `tests/training_test.rs` (4) | OK |
| `cargo clippy --all-targets` | no NEW warnings from this change (only pre-existing: 11 lib warnings + codegen_test.rs:25 `&PathBuf`, both recorded since task 23 status as e0bc3cb dirt to fix before task 22) |
| `cargo fmt --check` on `tests/codegen_test.rs` | clean (file formatted with rustfmt; remaining repo-wide diffs are the known pre-existing `src/backends/c_fsm.rs` sites, untouched here) |

## Found problems/issues/mismatches

- None blocking; no D-P3-6 mismatch surfaced, so no plan correction and no new
task were needed.
- Calibration facts (recorded in the task file's Task corrections):
  - Rust off-output still contains the per-label `Mon_*` struct definitions;
    the flag gates only the aggregate/stepping/StepError machinery (all
    debug-gated). The task file's shorthand "off has neither Mon_ structs"
    was inaccurate; assertions were calibrated against generated output first.
  - C off-output keeps `RFS_PROPERTY_VIOLATION` (enum) and
    `prop_label`/`prop_step`/`prop_trigger_step` (step_result_t fields) as
    stable wire format in BOTH modes.
  - The task file's Action 4 guess "(33, 41, 42)" was stale: 15 files carry
    prop/assume declarations (the TLA adaptations are in the
    `examples/valid/tla_adapted/` subdirectory). Dynamic enumeration handled
    it; all 15 x both backends are warning-free.
- Pre-existing clippy/fmt dirt from e0bc3cb (`src/backends/c_fsm.rs` etc.)
  remains; still owed before the task 22 close-out checklist.

## Other important findings/information

- Observed CLI behavior for Action 3 (as required by the task file): passing
  `--with-property-monitors` to `mc` or `test` is REJECTED with "--with-property-
  monitors is only valid for the production command; test and mc always include
  property monitors" - consistent with D-P3-6; the test pins this error text.
- Test placement follows the task file: Actions 1/2 sit directly after their
  respective per-backend generation tests; Actions 3/4 form a new section at
  the end of `tests/codegen_test.rs`.
