# Phase 3, Task 13 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_13.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (verification-only task).

## Changed/removed source files

None - all checks passed without touching code or examples.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_3_TASK_13.md` | changed | exit criteria checked (all four) |
| `docs/roadmap/PHASE_3_TASK_13_STATUS.md` | new | this file |

## Other file changes

None.

## Changed functionality

None.

## Removed functionality

None.

## New functionality

None (verification task).

## Tests add/change/remove

None - existing suites only, re-run as verification.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` lib unit tests (86) | OK |
| `tests/codegen_test.rs` (19) | OK |
| `tests/examples_test.rs` (3) | OK |
| `tests/frontend_test.rs` (41) | OK |
| `tests/integration_example_test.rs` (2) | OK |
| `tests/library_comparison.rs` (4) | OK |
| `tests/training_test.rs` (4) | OK |
| `cargo run -q -- validate --input rfs examples/valid` (43 files) | OK (exit 0; pre-existing validator warnings only, see findings) |
| codegen matrix: rust x {production,test,mc} x {ex41,ex42} + c x {production-nomon,production-mon,test,mc} x {ex41,ex42} = 14 runs | OK (zero "outside monitor subset" warnings in every run) |

## Verification details (Action 1 + Action 2)

- Example 42 UNMODIFIED (`git diff` empty on the file) parses; `retry_responds`
  AST: `always(implies(and(expr(eq(state, Sending)), output_has(Sent)),
  bounded_eventually[8](or(output_has(Sent), input_is(Ack)))))` - original
  antecedent order preserved (no reorder applied).
- Monitor presence per run (label = every prop/assume of the spec):
  - Rust (all 6 runs): `struct Mon_<label>` present for ALL labels
    (ex41: closed_implies_acked, ack_sets_flag_next, fin_acked_bounded,
    retries_bounded_window, env_tick; ex42: start_responds, retry_responds,
    env_tick). Production structs carry `#[cfg(debug_assertions)]` (D-P3-6).
  - C production WITHOUT `--with-property-monitors`: assumption monitor
    (`mon_env_tick_*`) present; property monitors ABSENT - by design (D-P3-6:
    C production emission is flag-gated, unlike Rust's compile-time gating).
  - C production WITH the flag, C test, C mc: `mon_<label>_{t,new,step}`
    present for ALL labels in both examples.

## Found problems/issues/mismatches

- None blocking. Observations:
  - The task file's Action 1 shorthand `And(OutputHas, Expr(Eq))` lists the
    operands in reverse order; the actual (and source-order-preserving) AST is
    `And(Expr(Eq), OutputHas)` as verified above. Intent of the criterion
    (original antecedent order preserved) is satisfied.
  - `validate` emits pre-existing warnings on several valid examples, incl.
    example 42 itself: state `Sending` transitions #1/#2 share the Tick event
    pattern with different guards and no priorities (first match wins in
    source order). These are Phase 2 validator style warnings, not errors -
    exit code is 0. Whether to add explicit `priority N;` to example 42's two
    Tick transitions is a cosmetic decision for task 21 (docs/examples sync)
    or later; NOT changed here per this task's non-goals.

## Other important findings/information

- C vs Rust production monitor gating is asymmetric BY DESIGN (D-P3-6): Rust
  always emits monitors (compile-gated via `#[cfg(debug_assertions)]`), C
  emits them only with `--with-property-monitors` (`#ifdef RFS_PROPERTY_MONITORS`
  + header define). Task 15's option-sensitivity tests pin this per backend.
- All generated artifacts for this verification were written to /tmp (not the
  repo); the tree contains only the two doc changes listed above.
