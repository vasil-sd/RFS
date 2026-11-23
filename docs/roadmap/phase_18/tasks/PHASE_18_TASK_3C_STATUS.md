# Phase 18, Task 3C - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3C.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None.

## Changed/removed source files

| Name | Change |
|---|---|
| `src/mc/smt_plan.rs` | dropped both v1 rejections (assumptions, on_error); added `SmtAssumption { name, formula, bound, next_depth, full }` + `SmtSpec.assumptions: Vec<…>` and `SmtOnError { next_state, emits }` + `SmtSpec.on_error: Option<…>`; `convert_assumption` converts each selected assumption via the existing `convert_formula` path (cheap input-guard form = default; top-level `eventually`/`bounded_eventually`/`until` or temporal nesting = FULL); `next_depth` carried for trailing-anchor clipping (same rule as properties - a `next`-bearing body must not read `alive[k]` at the last sample); module-header v1-scope comment updated |
| `src/mc/ref_mc_smt.rs` | `run` gained `full_assumptions: bool` + `assumes_arg: &str` (the DISTINCT gate + selection, mirroring the generated binaries); per-sample assumption base constraints (`alive[i].not() OR formula`, always[N] clip at N, next_depth anchor drop, FULL class asserted only with the flag and omitted with a per-assumption warning + `assumptions_omitted=M` summary suffix); on_error fallback encoded as `ite(any_fire, chain, on_error_next)` with the alive recurrence gaining the fallback disjunct; `input_is`/`output_has` evaluated against the ACTUAL input (class var + payload fields) when an on_error block exists (the unmatched input is still consumed, SEMANTICS 3.6); the fallback's emits mark matching `output_has` atoms true at the steps that took it; witness trace prints `[on_error]` markers; old rejection unit test replaced by an acceptance pin |
| `src/mc/smt_emit.rs` | both emitters (Rust + C) render the assumption tables (`ASSUME_NAMES`/`ASSUME_FULL`/`ASSUME_BOUNDS`/`ASSUME_NEXT_DEPTH` + one builder fn per assumption) and the on_error machinery (next-state constant, emit-args tables, `build_on_error_emit_match`, trace markers); the on_error section is emitted ONLY when the module has one, so assumption-free/on_error-free specs generate byte-identical code to before; the emit-args tables are skipped when the module has no output atoms (their only consumer is a constant-false stub then - emitting them would be unused statics under gcc -Wall); the plain `build_input_atom` builder is emitted only WITHOUT on_error (the unrolling loop calls the `_ev` variant then, so the plain one would be dead code); the dead `HAS_ON_ERROR` constant was never consumed and is no longer emitted |
| `src/codegen/backends/rust/mc.rs` | generated SMT runtime parses `--full-assumptions` + `--assumes all\|none\|labels` (default `all`, unknown labels silently dropped); the assumption assertion loop threads selection + the FULL gate + always[N]/next_depth clipping; the on_error fallback slots (next-state ITE, alive disjunct, `_ev` atom builders, trace param/marker/arg) activate when the spec carries one |
| `src/codegen/backends/c/mc.rs` | same runtime surface as the Rust backend (flag parsing, assumption loop with all three clips, on_error slots) - C output stays gcc -Wall clean |
| `src/cli/commands/mc_ref.rs` | `--assumes` is no longer rejected under `--mode smt-bounded`; new smt-bounded-only flag `--full-assumptions` (usage error under the other modes, like `--full-properties`); usage text + doc comments updated |
| `src/mc/ref_mc.rs` | `run_smt` gained the `full_assumptions` + `assumes_arg` parameters and forwards them to `ref_mc_smt::run` |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/SEMANTICS.md` §9.9 | changed | new **Assumptions (task 3C)** + **on_error blocks (task 3C)** paragraphs (per-sample alive-gated assertion = §7.5 successor pruning; cheap/expensive classes; the DISTINCT `--full-assumptions` gate; `--assumes` selection per §7.4; the fallback's next-state-only semantics and actual-input atom evaluation); the v1-scope rejection list no longer names assumptions/on_error (enum-typed vars, wildcard patterns over multi-variant groups, enum-variant output patterns remain) |
| `docs/user/MODEL_CHECKING.md` | changed | flag table gains `--full-assumptions` + `--assumes`; the `--bound N` row drops the rejection note; new **Assumptions and on_error** paragraph under *What it checks*; the shared-flags section now lists smt-bounded among `--assumes`' modes |
| `docs/user/USAGE.md` | changed | the `rfs mc-ref --mode smt-bounded` row documents the two new flags, the pruning/fallback semantics, and drops the REJECTED note + the assumption-rejection error column |
| `docs/llm/TEST_STRUCTURE.md` | changed | the two stale rejection-pin rows renamed to their acceptance successors; new rows for `parity_task3c`, the four `task3c_*` parity tests, and `smt_bounded_test.rs::{run_bin_full, smt_bounded_task3c_on_error_and_assumptions}` |
| `docs/llm/SRC_STRUCTURE.md` | changed | the `build_smt_spec` row now describes assumptions (D3) + on_error (D4) as carried in the spec, with the remaining v1 rejections listed without them |

## Other file changes

None.

## Changed functionality

- `rfs mc-ref --mode smt-bounded` and the generated SMT binaries ACCEPT specs with assumptions and/or on_error blocks (previously: shared rejection error, exit 1). `--assumes all|none|labels` now works on both smt surfaces (default `all`; `none` reproduces the assumption-free verdict byte-for-byte); new `--full-assumptions` flag gates the expensive assumption class, DISTINCT from `--full-properties`.

## Removed functionality

- The shared v1 rejections `"smt-bounded mode does not support assumptions (spec has N)"` and `"smt-bounded mode does not support on_error blocks"` are gone from both backends and the ref engine.

## New functionality

- **D3 (assumptions):** each selected assumption's formula is asserted at every sample it covers as a base constraint - cheap input-guard forms by default, expensive temporal forms behind `--full-assumptions` (omitted otherwise with a per-assumption warning + `assumptions_omitted=M`). This is exact mode's §7.5 successor pruning expressed as per-step SMT facts: only inputs satisfying every active assumption are allowed.
- **D4 (on_error):** the §3.6 catch-all fallback as a lowest-priority transition from ANY state - next-state only, no variable assignments. Unmatched inputs take it instead of terminating the path; `input_is`/`output_has` are evaluated against the actual input (the unmatched input is still consumed); the fallback's emits mark matching `output_has` atoms true at the steps that took it.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_smt_test.rs::ref_smt_accepts_assumption_and_on_error_specs` | changed (was `ref_smt_rejects_…`) | the old shared-error pin now asserts the NEW contract: assumption-bearing and on_error specs run, and the pre-3C error strings must not resurface |
| `tests/ref_mc_smt_test.rs::parity_task3c` | added | parity helper: ref vs both generated backends at every bound with the full-properties/full-assumptions flags and the `--assumes` selection threaded through; byte-identical stdout asserted |
| `tests/ref_mc_smt_test.rs::task3c_on_error_pass_and_fail` | added | D4: unmatched inputs take the fallback - PASS variant (the fallback keeps a safe state reachable) and FAIL variant (a fallback-driven violation is reported with `[on_error]` trace markers), parity-pinned |
| `tests/ref_mc_smt_test.rs::task3c_assumption_pruning` | added | D3: a cheap assumption prunes the violating input (PASS with it, FAIL without); `--assumes none` reproduces the assumption-free TWIN byte-for-byte on ref AND both backends; the pinned reason/label summary is asserted at every failing bound |
| `tests/ref_mc_smt_test.rs::task3c_assumed_input_still_fails` | added | D3: the assumed input ITSELF drives a violation - `always (next(input_is(Tick)))` forces x past the bound (the `next` shifts the obligation off sample 0, where no input has been consumed and a bare `input_is` would be statically false); deterministic FAIL at bound >= 4 |
| `tests/ref_mc_smt_test.rs::task3c_expensive_assumption_omission_and_flag` | added | D3: a nested-temporal assumption is omitted WITHOUT `--full-assumptions` (pinned per-assumption warning + `assumptions_omitted=1`, relaxed verdict) and asserted WITH it (verdict flips), parity-pinned |
| `tests/smt_bounded_test.rs::smt_bounded_task3c_on_error_and_assumptions` | added | both generated backends run on_error + assumption specs: cheap-assumption pruning (PASS active / FAIL `--assumes none`), expensive-assumption omission (pinned stderr warning) then assertion under `--full-assumptions` (verdict flips); C output must be gcc -Wall clean |
| `src/mc/ref_mc_smt.rs::tests::assumption_and_on_error_specs_accepted` | changed (was `rejection_errors_surface_verbatim`) | the in-process unit pin of the new contract: with the assumption E is pruned and the on_error fallback keeps x at 0 (PASS); `--assumes none` lets E fire (FAIL) |
| every pre-existing smt pin (task 1A/3A families, ref-vs-generated parity) | unchanged | all stay byte-identical - specs with NO assumptions and NO on_error generate exactly the same code and output as before (the new emission is gated behind `has_on_error` / non-empty assumptions) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --features smt --test ref_mc_smt_test --test smt_bounded_test` | OK - 15/15 + 3/3 (baseline before 3C was 11/11 + 2/2; the delta is the new task-3C tests) |
| `cargo test --features smt` (full suite) | OK - all targets green (271 lib unit tests + every integration suite); the single failure `parity_external_mmd_pinned_subset` is the pre-existing environmental puppeteer issue (chrome-headless-shell missing from the cache), unrelated to this task |
| `cargo build --features smt` | OK - zero warnings |
| `cargo clippy --features smt --all-targets` | OK for all task-3C-touched code (the remaining warnings in `ref_mc_smt.rs` at lines 228/245/406 pre-date this task - outside every 3C diff hunk) |
| z3 under test | in-process z3 4.8.12.0 (z3-sys) + generated binaries against the unified resolver prefix `/workspace/host/z3` - both named in every assertion message via `version_note()` |

## Notes for follow-up tasks

- A positive `input_is(...)`/`output_has(...)` assumption asserted at sample 0 is statically false there (no input consumed yet), which makes the base UNSAT and the verdict a VACUOUS PASS - exact mode instead reports "assumption violated at initial configuration". This divergence is a candidate for the MODEL_CHECKING.md residual-differences section (task 3E); specs that need to force an input at every step should write `always (next(input_is(...)))` (cheap class, satisfiable at sample 0).
- D8 enum support (enum-typed vars, wildcard-group patterns, enum-variant output patterns) remains rejected - task 3D.
