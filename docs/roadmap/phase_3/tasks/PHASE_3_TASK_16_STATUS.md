# Phase 3, Task 16 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_16.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `tests/monitor_integration_test.rs` | new end-to-end monitor integration suite (4 tests): MC bug-catching + counterexample parity, correct-spec no-false-positive baseline, Rust/C production step-API violation surfaces |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | non-goal honored: no changes to MC runtimes or backends; all four actions were satisfied without any code change |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_3_TASK_16.md` | changed | exit criteria checked (all five); "Task corrections" section appended (Action 1 output-format calibration, Action 2 bound selection) |
| `docs/roadmap/PHASE_3_TASK_16_STATUS.md` | new | this file |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| (none in the repo) | - | generated artifacts written to `target/rfs_monitor_integration_*` (self-cleaning with `cargo clean`) and /tmp scratch (discarded); examples, training sets, build files untouched |

## Changed functionality

- None.

## Removed functionality

- None.

## New functionality

- End-to-end integration coverage of the online property monitors in BOTH backends (`tests/monitor_integration_test.rs`):
  - buggy variant of example 41 (Ack transition dropped) -> MC in both backends exits 1 with a property-specific counterexample: SAME label (`fin_acked_bounded`), SAME trigger step (1), SAME violation/deadline step (5); the walk is fully deterministic (one allowed input per state), so the pinned values are seed-independent
  - unmodified example 41 -> MC in both backends exits 0 at `--bound 4 --runs 100` with zero property violations and identical summary field sets (`runs bound completed unexpected assumption_stops property_violations`)
  - Rust production with `--with-property-monitors` (debug build) -> the step API returns `Err(StepError::PropertyViolation { label: "fin_acked_bounded", step: 5 })` on the shared violating sequence [Send_Fin, Tick x4]
  - C production compiled with `-DRFS_PROPERTY_MONITORS` -> `step_result_t.status == RFS_PROPERTY_VIOLATION` with `prop_label=fin_acked_bounded`, `prop_step=5`, `prop_trigger_step=1` - equal to the Rust values (per-language gating, identical semantics)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `mc_catches_injected_bug_both_backends` | new | Action 1: buggy spec -> both MCs exit 1; label/trigger/violation steps equal across backends AND pinned to (fin_acked_bounded, 1, 5); counterexample trace present |
| `mc_correct_spec_passes_within_bound_both_backends` | new | Action 2: correct spec at bound 4 / runs 100 -> both exit 0, `property_violations=0`, identical summary field names in the same order |
| `rust_production_debug_monitors_surface_property_violation` | new | Action 3: debug-compiled production driver (no `-O`, so `debug_assertions` is on); steps 1-4 Ok, step 5 `PropertyViolation { label: "fin_acked_bounded", step: 5 }` |
| `c_production_monitors_surface_property_violation_parity` | new | Action 4: gcc `-Wall -DRFS_PROPERTY_MONITORS` driver; steps 1-4 RFS_OK, step 5 RFS_PROPERTY_VIOLATION with prop_label/prop_step/prop_trigger_step equal to the Rust values |
| helpers `scratch`, `compile_rust`, `compile_c`, `example41_source`, `buggy_spec_source`, `generate`, `parse_mc_violation`, `parse_mc_summary`, `summary_keys` | new | shared harness: in-process codegen (same path as the CLI), rustc/gcc compilation, fixed-seed runs, output parsing |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --no-fail-fast` lib unit tests (86) | OK |
| `tests/codegen_test.rs` (23) | OK |
| `tests/examples_test.rs` (3) | OK |
| `tests/frontend_test.rs` (41) | OK |
| `tests/integration_example_test.rs` (2) | OK |
| `tests/monitor_integration_test.rs` (4; new in this task) | OK |
| `tests/library_comparison.rs` (4) | OK |
| `tests/training_test.rs` (4) | OK |
| `cargo clippy --all-targets` | no NEW warnings from this change (0 findings reference `monitor_integration_test.rs`; pre-existing e0bc3cb dirt remains, owed before task 22) |
| `rustfmt --edition 2021 --check tests/monitor_integration_test.rs` | clean (file formatted with rustfmt; remaining repo-wide diffs are the known pre-existing `src/backends/c_fsm.rs` sites, untouched here) |

## Found problems/issues/mismatches

- No backend divergence: every cross-backend value compared is equal (label,
  trigger step, violation step, summary field set - even the `completed=51`
  count at bound 4 / runs 100). No plan correction and no follow-up task needed.
- The task file's Action 1 shorthand `property-violated:<label>` does not match
  the actual MC output: both runtimes print `run N property violation:
  <label>` plus `trigger step:` / `violation step:` lines (the Context section's
  status string was aspirational). Assertions were calibrated to the real wire
  format (recorded in Task corrections).
- gcc -Wall emits pre-existing warnings on GENERATED C code (`-Wmaybe-
  uninitialized` for `output_t outs[MAX_OUTPUTS]`, unused parameters in monitor
  signal helpers); compilation succeeds. Generated-code warning cleanliness is
  out of scope for this task (candidate for the task 21/22 close-out, same
  bucket as the pre-existing e0bc3cb clippy/fmt dirt).

## Other important findings/information

- The buggy MC walk is fully deterministic: with the Ack transition dropped,
  exactly one input is allowed per state (Send(Fin) from Idle; Tick in SentFin -
  Recv(Ack)/Send(Fin) become unexpected and are excluded from the MC's allowed
  set), so the pinned step numbers do not depend on the seed. The fixed seed
  (42) is kept for the good runs per the task file.
- Action 2 bound rationale: bound 4 is the largest horizon where NO property can
  be confirmed for ANY seed/run count - `fin_acked_bounded` needs trigger + 4
  samples (sample 5) to expire, `retries_bounded_window` needs 6 ticks (and only
  checks samples 0..9), and the safety invariants are unreachable in a violating
  form. Zero violations is therefore provable, not seed luck. At larger bounds
  (e.g. 8) a random environment that never sends the Ack legitimately violates
  `fin_acked_bounded` - correct monitor behavior, not a false positive.
- The C MC trace lacks a trailing newline after its last step line (Rust has
  one); traces are textually different by design (C prints numeric kinds/states)
  - tests compare label/steps/summary fields only, never trace text.
