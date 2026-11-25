# Phase 18, Task 3I - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3I.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `examples/valid/51_lifted_syntax.rfs` | one small spec (3 states, 2 vars) exercising ALL THREE lifts together: a named field in a formula input_is pattern (V17/3F), a bare boolean top-level property AND a top-level next (S4/3G), and a constant-expression var init `2 + 3` (E4/S1/3H); PASS complete under exact mode, PASS WITHIN BOUND under smt-bounded |
| `examples/invalid/invalid_transition_pattern_literal_width.rfs` | a transition pattern with an out-of-range literal (`on Recv(Ack(300))` on a `bit<8>` field) - proves V8a field typing survived the V17 lift where it actually lives (transition patterns) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/lang/validator.rs` | changed (inline tests only) | three new unit tests: named-field formula input_is validates (the lift); transition-pattern V8a still fires (the narrowness); formula patterns have NO structural checks by design - unknown group / arity mismatch / out-of-range literal all validate and are soundly vacuous (pins the absence so nobody "restores" a check that never existed) |
| `src/codegen/test_plan.rs` | changed (test pins only) | corpus-size pin 50 -> 51 in two planner-sweep tests (the new valid example joined the corpus) |

## Added/changed/extended documentation files

None (task 3J owns the doc set; the new example carries its own header comment per the corpus convention).

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_smt_test.rs` | changed | `plan_spec` helper + three planner-level pins: bare/next top forms plan as Always kind, no bound, DEFAULT class (S4); the wrapped obligation form plans FULL (gate intact); `init 5 + 1` folds to the constant 6 and a var-referencing init is rejected with the exact-mode-identical message (E4/S1) |
| `tests/smt_bounded_test.rs` | changed | new leg `smt_bounded_task3f_3h_lifted_restrictions` + `T3I_LIFTS_SRC`: exact mode in-process PASS complete FIRST (authoring discipline), then smt-bounded in-process + both generated engines byte-identical, pinned "PASS WITHIN BOUND 6" |
| `tests/backend_parity_test.rs` | changed | re-pinned exact-mode expectation for `51_lifted_syntax.rfs` (pass-complete, 7 states / 7 transitions / max_depth 6 - from actual output); pass/fail split pin (14,9) -> (15,9) with the comment updated |
| `tests/codegen_test.rs` | changed | no_std sweep corpus pin 50 -> 51 |
| `tests/info_format_test.rs` | changed | invalid-corpus pins 13 -> 14 (three sites: file walk, directory-mode fail count, JSON array length) |
| `tests/psl_translation_test.rs` | changed | the independent reference walker's `confirm` gained the two task-3G lifted top-form arms - top-level `next` (RangeCheck over the shifted range, inner checked from i+1 on) and bare boolean top forms (checked at every sample) - mirroring the production monitor compiler exactly; the panic arm for genuinely non-monitorable shapes is kept |

## Changed functionality

None - tests + corpus examples only. No engine, planner, validator-logic, or codegen change.

## Removed functionality

None.

## New functionality

- Suite adoption of the 3F-3H lifts: every lifted form is now pinned in both directions (accepts what was rejected; still rejects what must stay rejected) at unit level (validator), planner level (build_smt_spec classification + folding), and end-to-end level (exact vs smt-bounded parity across in-process + both generated engines).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `validator::tests::{formula_input_is_named_fields_validate, transition_pattern_field_typing_still_fires, formula_pattern_structural_checks_absent_by_design}` | added | V17 lift + narrowness + vacuity pins |
| `ref_mc_smt_test::{plan_spec, task3i_bare_and_next_top_forms_plan_default_class, task3i_wrapped_obligation_stays_full_class, task3i_constant_init_folds_and_var_ref_rejected}` | added | S4 classification + E4/S1 folding/rejection pins |
| `smt_bounded_test::smt_bounded_task3f_3h_lifted_restrictions` (+ `T3I_LIFTS_SRC`) | added | the cross-mode end-to-end regression net for all three lifts |
| corpus-size / split / pin updates in test_plan, backend_parity, codegen, info_format, psl_translation | changed | suite adoption of the two new corpus examples + the lifted top forms in the PSL reference walker |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 615 passed / 0 failed (baseline 612 + 3 new validator tests) |
| `cargo test --features smt` | OK - 646 passed / 0 failed (baseline 639 + 3 validator + 3 planner + 1 e2e leg) |
| `cargo test-generated-code` | OK - 1727 passed / 0 failed (baseline 1720 + 7: the new module m045__51_lifted_syntax; all pre-existing modules byte-identical - the plan's generated-code caveat, not a regression) |
| `cargo clippy --all-targets` | OK - 71 warning lines = exact baseline, zero new |
| `cargo clippy --features smt --all-targets` | OK - no errors; the `unused_enumerate_index` warnings at ref_mc_smt.rs:936/949 are PRE-EXISTING on HEAD (verified via stash) and only visible under the smt feature |
| `cargo fmt --check` | OK - clean |
| `rfs validate --input rfs examples/valid` | exit 0 (51 valid) |
| `rfs validate --input rfs examples/invalid` | exit 1 (14 invalid, all rejected) |

## Found problems/issues/mismatches

- The task file's premise for the invalid example was wrong in a way task 3F had already recorded: field typing INSIDE formula patterns never existed (`check_pattern_group`/`check_output_pattern_group` are no-ops; re-verified empirically - unknown group, arity mismatch, and out-of-range literal all validate). The invalid example therefore pins V8a on a TRANSITION pattern (where field typing actually lives), and the formula-pattern vacuity is pinned by a dedicated validator unit test instead.
- Five pre-existing corpus-size pins had to move with the new examples (test_plan x2, info_format x3, codegen x1) plus the backend_parity pass/fail split and the PSL walker's top-form coverage - all mechanical suite adoption, no behavior change.

## Other important findings/information

- Training set UNCHANGED: snippets are extracted from `training/*.jsonl` entries, and none was fenced in this task (that is task 9's scope) - so no snippet/aggregate regeneration was needed; the generated-code growth came from the generator walking `examples/valid` directly.
- The PSL walker extension mirrors the production monitor compiler arm-for-arm (top-level next = RangeCheck over `range.shifted()`; bare boolean = RangeCheck over the full range) - the panic arm for genuinely non-monitorable top shapes (eventually/until) is preserved.
