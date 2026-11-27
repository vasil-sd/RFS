# Phase 18, Task 3M - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3M.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| - | none - all changes are to existing files |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/mc/ref_mc.rs` | changed | two new `pub(crate)` helpers: `initial_configuration` (the q0 + init-vars computation with the task-3L rule-4 masking, extracted from `ref_mc_exact.rs`) and `sample0_violated_assumptions` (compiles each selected assumption with the monitor compiler - compile failure = skip, matching exact mode - steps sample 0 with no input, returns violated labels in declaration order) |
| `src/mc/ref_mc_exact.rs` | changed | `run` calls both helpers; the sample-0 error path stays byte-identical (same message, first label, same position relative to the property check). The assumption monitors carried into the BFS are STILL stepped in place at sample 0 after the helper check - the helper steps its own copies, and a countdown-bearing assumption (e.g. `bounded_eventually[N]`) observes a different obligation from sample 1 on depending on whether sample 0 was stepped |
| `src/mc/smt_plan.rs` | changed | `SmtSpec` gains `assumption_sample0_violated: Vec<bool>` (parallel to `assumptions`); `build_smt_spec` computes it via the shared helper with selection = all assumption names |
| `src/mc/smt_emit.rs` | changed | both emitters render the constant array (`ASSUME_SAMPLE0_VIOLATED`, Rust + C mirrors); both engine templates check selected ∩ violated after `selected_assumes` is computed and BEFORE any z3 work - `error: assumption '<label>' violated at initial configuration (sample 0)` to stderr + exit 1, first label in declaration order; the `test_spec()` fixture gains the new field |
| `src/mc/ref_mc_smt.rs` | changed | `run` checks selected ∩ `spec.assumption_sample0_violated` after `parse_assume_selection`, before `run_unrolled` - returns the identical `Err` message, surfaced by `mc_ref.rs` as exit 1 + the `error: {e}` line |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/user/MODEL_CHECKING.md` | changed | section 10.5 deleted (the divergence is closed); the intro's closed list gains the task-3M bullet; the "What REMAINS" sentence now names D5/D6 + the two declared omissions only; the summary table row reads "identical pre-solve rejection, byte-identical line (task 3M) \| none - unified (task 3M)" |
| `docs/user/FAQ.md` | changed | section 2's smt-bounded assumption bullet: the "One difference" sentence replaced with the unified-check statement |
| `docs/SEMANTICS.md` | changed | §9.9 Assumptions paragraph gains the pre-solve check sentence (incl. the out-of-subset skip rule); the closing residual pointer drops the sample-0 divergence |
| `docs/dev/mc_engine.md` | changed | the SMT "Engine + verdicts" paragraph gains one sentence: a selected assumption already violated at the initial configuration is rejected before any solver work with exact mode's byte-identical error line (task 3M, shared helper) |
| `docs/llm/SRC_STRUCTURE.md` | changed | inventory rows for `initial_configuration` + `sample0_violated_assumptions`; descriptions updated on `ref_mc_exact.rs::run`, `ref_mc_smt.rs::run`, `smt_plan.rs::build_smt_spec` (via `rfs-dev inventory add/update`) |
| `docs/llm/TEST_STRUCTURE.md` | changed | inventory rows for the three new test fns; llm-budget 1494 -> 1497 |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/mc_walkthrough/captures/fail-rust.json` | changed | `peak_rss_kb` churn (2444 -> 2556) from the example suite re-running the walkthrough - a volatile runtime measurement, no semantic change; consistent with how tasks 3I/3L/3J committed capture updates |

## Changed functionality
- Exact mode (`ref_mc_exact.rs::run`):
  - the q0/init-vars computation and the sample-0 assumption check now run through the shared helpers in `src/mc/ref_mc.rs` instead of inline code - behavior byte-identical (same error message, first violated label in declaration order, same position relative to the property check)
- smt-bounded mode (all three engines):
  - a SELECTED assumption already violated at the initial configuration is now rejected BEFORE any solver work with the byte-identical `error: assumption '<label>' violated at initial configuration (sample 0)` line on stderr + exit 1 and EMPTY stdout - previously this shape made the base constraint UNSAT at sample 0 and the run reported a VACUOUS `PASS WITHIN BOUND`

## Removed functionality
- The vacuous `PASS WITHIN BOUND` verdict for a selected assumption already violated at sample 0 (smt-bounded, all three engines) - replaced by the unified rejection; no other verdict path touched

## New functionality
- Shared sample-0 assumption check (`src/mc/ref_mc.rs::sample0_violated_assumptions` + `initial_configuration`) - one computation, four consumers (exact mode + the three SMT engines), parity by construction
- Pre-solve rejection in all three smt-bounded engines (in-process `ref_mc_smt::run` + both generated SMT binaries) with exact mode's byte-identical error surface

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_smt_test.rs::ref_smt_vs_generated_sample0_violated_assumption` | new | parity pin: the in-process engine returns the exact-mode Err; BOTH generated backends print the byte-identical stderr line with EMPTY stdout and exit 1, at every bound (1/3/5) |
| `tests/ref_mc_smt_test.rs::ref_smt_sample0_negative_controls` | new | three negative controls: the `always (next (input_is(Tick)))` workaround form PASSes WITHIN BOUND on all three engines; `--assumes none` on the violating spec skips the check (PASS, no error); an out-of-subset assumption (`always (always[2] (...))`, FULL class) never triggers the sample-0 error even with `--full-assumptions` |
| `tests/smt_bounded_test.rs::smt_bounded_task3m_sample0_violated_assumption` | new | generated-backends leg: both backends run the violating spec - identical error line + exit 1; C output gcc -Wall clean (compile_c's hard bar) |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test --features smt` (all 34 suites) | OK - 675 passed / 0 failed / 0 ignored |
| `cargo clippy --all-targets --features smt` | OK - exit 0 (62 pre-existing lib warnings, none from task-3M code) |
| `cargo fmt --check` | OK - clean (after reformatting the task-3M source additions) |
| `cargo test --test dev_docs_test` | OK - 9 passed / 0 failed |
| `cargo test --test docs_md_test` | OK - 5 passed / 0 failed |
| `rfs-dev inventory audit` | EXACT - SRC 1039 fns / 217 types, TEST 1149 fns / 35 types, zero per-file diffs |
| `rfs-dev sync-check` | OK - clean |

Live verification before pinning (all three engines, violating spec `assume force_tick : input_is(Tick)`):
- `rfs mc-ref --mode smt-bounded --bound 5`: exit 1, EMPTY stdout, stderr exactly `error: assumption 'force_tick' violated at initial configuration (sample 0)`
- generated rust + c binaries: identical line + exit 1; `--assumes none` -> `PASS WITHIN BOUND 5 (incomplete - no completeness claim)` on both

## Found problems/issues/mismatches
- The initial helper extraction (3M.1) dropped a SIDE EFFECT of the inline sample-0 loop: it stepped the assumption monitors IN PLACE inside the vector carried into the BFS. The helper steps its own throwaway copies, so countdown-bearing assumptions (`bounded_eventually[N]`) started the BFS one step "fresher" - `mc::ref_mc::tests::verify_deadlock_ce_assumption_pruned` caught it (deadlock depth 1 -> 2). Fixed by restoring the in-place step of the assumption monitors after the helper check (the step cannot fail there - the helper just proved no selected assumption is violated at sample 0).
- The `test_spec()` fixture in `src/mc/smt_emit.rs` constructed `SmtSpec` literally and missed the new `assumption_sample0_violated` field - the lib test build failed with E0063. Fixed by adding `Vec::new()` (the fixture has no assumptions).
- `docs/llm/TEST_STRUCTURE.md`'s `llm-budget` header (1494) was exceeded by the two new rows - bumped to 1497; `dev_docs_test::sync_gate_passes_on_current_tree` caught it.

## Other important findings/information
- The generated RUST smt-bounded artifact is a CARGO crate (`use z3::...`), not standalone-rustc-compilable: build it in a scratch project with `z3 = "0.21"` + `[workspace]` (the same recipe `tests/smt_bounded_test.rs::build_rust` uses, `cargo build --release --offline` with `Z3_SYS_Z3_DIR`/`Z3_LIBRARY_PATH_OVERRIDE`/`Z3_NO_PKG_CONFIG=1`/`Z3_SYS_Z3_VERSION=5.3.1`). The C artifact links libz3 directly.
- `rfs codegen <backend> mc --out X` treats X as a BASE name - artifacts land at `X-mc.rs` / `X-mc.c` (+ `-mc.h` for C); per-artifact flags (`--out-mc-code`, ...) override individual names.
- The sample-0 check is selection-gated in every engine: `--assumes none` (or a label list excluding the violator) skips it entirely - the vacuous-pass shape is still reachable by explicit user choice, which is the documented 10.4-kind declared behavior, not a divergence.
