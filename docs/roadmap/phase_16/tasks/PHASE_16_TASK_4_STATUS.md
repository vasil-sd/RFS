# Phase 16, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_16_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/mc/ref_mc_smt.rs` | the in-process SMT-bounded executor (907 lines, module feature-gated on `smt`, D-P16-3): the THIRD consumer of the shared `SmtSpec` encoding model. `run` consumes the model chain exactly as the backends do (`McPlan::compute` -> `build_smt_spec`, rejection errors surfaced verbatim); `run_unrolled` mirrors SMT_ENGINE_RUST step for step - fixed step-variable names/order (`s_{i}`/`v_{i}_{j}`/`ev_{i}`/`fld_{i}_{g}`), fixed base-constraint order (initial config, then per-step enabled/fires/any_fire/alive + next-state and per-var ITE chains), safety queries in declaration order then liveness (never + cycle), identical model reading (first violating i; first equal-config a<b pair); `se_to_z3`/`sf_to_z3` mirror the emitter's rendering rules with the executor's OWN copies of `cmp_widths_full`/`shift_amt_width`/`widen`/`global_field_index` (independence contract - model sharing, not emission sharing); `print_trace` renders the pinned witness-trace format; `SmtOutcome` carries exit code 0/1/2 + full stdout; `z3_version` re-exports the linked solver version for the parity suite. Four unit tests pin fail-at-initial / liveness-cycle (bound 1 vacuous) / pass-within-bound / rejection errors. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/mc_concepts.rs` | changed | D-P16-5 single-sourced SMT wire format: `SmtStatus` enum (wire names pass-within-bound/fail/unknown + exit codes 0/1/2), the four summary-line format constants, `fill_slots` (fills `{}` slots in order - the C templates derive their printf formats this way; `format!` needs a literal on stable), and `smt_summary_line(status, bound, reason, label, step)` - one wire format, three consumers. A unit test byte-pins all four rendered forms + the exit-code mapping. |
| `src/codegen/backends/rust/mc.rs` | changed | SMT_ENGINE_RUST's five summary println sites now interpolate the four `__SMT_SUMMARY_*__` placeholders, filled from the shared constants at codegen time (the LCG/fuzz-interpolation precedent). Emitted bytes UNCHANGED - proven by 1683/1683. |
| `src/codegen/backends/c/mc.rs` | changed | SMT_ENGINE_C's five printf sites use formats derived at codegen time via `fill_slots` with the slot printf types (%u bound/step, %s label). Emitted bytes UNCHANGED - proven by 1683/1683. |
| `src/mc/ref_mc.rs` | changed | `run_smt(module, bound)` entry point (D-P16-1 delegation, feature-gated) + the cfg-gated `use super::ref_mc_smt`. |
| `src/mc/mod.rs` | changed | `#[cfg(feature = "smt")] pub mod ref_mc_smt;` - codegen of smt artifacts stays ungated, in-process execution is gated (the z3 crate is optional). |
| `src/cli/commands/mc_ref.rs` | changed | D-P16-4 unified CLI: `McMode::SmtBounded` + case-sensitive parse of `smt-bounded`; dispatch rejects EVERY flag other than `--bound` under the mode (the generated SMT binary parses only `--bound` - exact-only and fuzz-only flags alike, plus `--assumes`); `--bound N` defaults to `SMT_DEFAULT_BOUND` (10); exit codes 0/1/2 mirror the generated SMT binaries; a non-smt build exits with the clear diagnostic "requires the `smt` cargo feature" (exit 1, the command convention - the one deliberate surface difference). Module doc + usage text document the full per-mode surface. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed | +26 rows: the four mc_concepts items (`smt_summary_line`, `fill_slots`, `SmtStatus::wire`, `SmtStatus::exit_code`), `run_smt`, and all 17 executor fns + 4 types (`SmtOutcome`, `StepVars`, `SeAst`, `SmtStatus`); llm-budget 1760 -> 1786. |
| `docs/llm/TEST_STRUCTURE.md` | changed | +22 rows: the mc_concepts byte-pin, the five executor unit-test rows, and the whole `tests/ref_mc_smt_test.rs` suite (16 helper/test rows); llm-budget 1357 -> 1379. |
| `docs/llm/TASK_MAP.md` | changed | the SMT-bounded-mode row now names the in-process executor (`ref_mc_smt.rs::run`, entry `run_smt` delegates, D-P16-3); the file list gains the ref_mc_smt.rs entry. |
| `docs/dev/mc_engine.md` | changed | dev:mc-engine-smt extended with the "In-process executor" section (third consumer, model-chain consumption, own conversion per the independence contract, pinned conventions) and the executor's parity pins in the "Parity pins" paragraph; Code list extended with all new items. |
| `BUILD.md` | changed | mc-ref CLI line: `--mode exact\|fuzz\|smt-bounded` + the smt-bounded flag surface (`--bound N` only), the `--features smt` requirement, the rejection rule, and the 0/1/2 exit codes. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | +`src/mc/ref_mc_smt.rs` (the dev-docs coverage manifest). |
| `TASKS.md` | appended | the task-4 entry. |

## Changed functionality
- The generated SMT runtimes' summary lines are now rendered from the shared
  `mc_concepts` constants instead of inline literals in each template - emitted
  artifact bytes are UNCHANGED (1683/1683 proves it).
- `rfs mc-ref` CLI surface: `--mode smt-bounded` added; under it every flag other
  than `--bound` is a usage error (the generated SMT binary parses only
  `--bound`).

## Removed functionality
- None.

## New functionality
- `rfs mc-ref --mode smt-bounded --bound N`: k-step SMT unrolling over z3
  IN-PROCESS - no compilation cycle. The third consumer of the shared `SmtSpec`
  encoding model (one encoding, three consumers: C emission, Rust emission,
  in-process execution), with byte-identical summary lines + witness traces to
  both generated SMT binaries and mirrored 0/1/2 exit codes.
- Shared wire format (D-P16-5): `SmtStatus` + `smt_summary_line` + the four
  format constants in `mc_concepts.rs`, interpolated into both templates.
- Clear diagnostic on non-smt builds (exit 1) - codegen of smt artifacts stays
  available without the feature; only in-process execution requires it.
- Cross-backend byte-parity suite over the fail/liveness/pass spec family at
  multiple bounds, naming both solver sides in every assertion.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `src/mc/ref_mc_smt.rs::tests::*` (4 tests + 1 helper) | new | fail-at-initial (every bound >= 1 fails at step 0, pinned summary + witness-trace bytes), liveness-cycle (bound 1 passes - vacuous cycle query; bounds >= 2 fail at step 0 with a cycle length), pass-within-bound (exact two-line output), rejection errors verbatim (assumptions / on_error). |
| `src/codegen/mc_concepts.rs::tests::smt_summary_line_formats` | new | byte-pins all four rendered SMT summary formats + the 0/1/2 exit-code mapping. |
| `tests/ref_mc_smt_test.rs` (4 tests) | new | FULL-stdout byte-parity (human lines + witness trace + summary line) + exit-code mirroring vs BOTH generated SMT binaries: the fail spec (wide bitvectors, shifts, oversized-literal comparison), the liveness spec (bound 1 vacuous pass / bounds >= 2 cycle fail), the pass spec - each at bounds 1/3/5; plus the CLI-level rejection pin. Mirrors smt_bounded_test's z3 resolution + build helpers locally (that suite is not restructured). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (573) | OK - = task 3's 572 + 1 (the ungated mc_concepts byte-pin) |
| `cargo test --workspace` (701 passed, 2 ignored) | OK - = 700 + 1; the 2 ignored are pre-existing |
| `cargo test --features smt` (583) | OK - = 574 + 9 (1 mc_concepts byte-pin + 4 executor unit tests + 4 integration tests) |
| `cargo test-generated-code` (1683/1683) | OK - UNCHANGED: the template interpolation moved no bytes |
| `nix develop -f shell.nix --impure` ref_mc_smt_test (4/4, z3 5.3.2 both sides) | OK - the nix half, after wiping target/ (Phase 15 discipline) |
| `cargo fmt --check` | OK - clean |
| `cargo clippy --all-targets` (61 warning lines / 37 unique) | IDENTICAL warning set to the post-task-3 state - zero new warnings from this task's code (the five task-3 warnings remain, routed to task 5) |
| `rfs-dev inventory audit` | OK - EXACT (SRC 998 fns / 210 types, TEST 1031 fns / 35 types - zero per-file diffs) |
| `rfs-dev sync-check` + `dev_docs_test` | OK - clean (exit 0) / 9/0 |

## Found problems/issues/mismatches
- **Host in-process z3 is the SYSTEM solver 4.8.12, below z3-sys' minimum
  (4.13.3):** z3-sys resolves via pkg-config at rfs build time and finds
  /usr's libz3 (4.8.12), emitting a build warning that enum numbering may differ
  across versions. The executor nevertheless works correctly on it - all unit
  tests pass and the parity suite is byte-green against the generated binaries
  running the project-local 5.3.1. The host half of `ref_mc_smt_test` is
  therefore actually CROSS-solver-version parity (4.8.12 in-process vs 5.3.1
  generated) - strictly stronger than the task's "under host z3 5.3.1" wording,
  and exactly the Phase 15 task-2b lesson (parity must hold under ANY solver
  version). Under `nix develop` both sides run the nix unified z3 5.3.2
  (same-version parity, 4/4 green). No action taken: forcing the in-process
  link onto RFS_Z3_DIR would change the canonical build activation for every
  consumer; if a future z3-rs API proves incompatible with 4.8.x, that is a
  toolchain task, not an mc-ref one.
- **Witness-trace format bug caught by the unit tests:** the first `print_trace`
  rendered `vars={x=0 }` (a stray space before the closing brace) where the
  generated engines print `vars={x=0}` - the fail-at-initial unit test's byte
  assertion caught it; fixed, byte parity achieved.
- **The task file's "five distinct printf/println sites per backend" confirmed:**
  there are five sites but only FOUR distinct formats (the unknown line appears
  twice - safety and liveness branches) - four constants, not five.

## Other important findings/information
- z3-rs 0.21's `Version` type does NOT implement Display - the executor
  re-exports `z3::full_version()` as `ref_mc_smt::z3_version()` so the parity
  suite can name the in-process solver (the z3 crate is an optional dependency,
  unreachable from integration tests).
- Rust format strings cannot call functions inline (`{version_note()}` is a
  compile error) - the suite binds the note to a variable per test.
- The generated SMT binary parses ONLY `--bound` at runtime and the codegen-time
  default (`mc_smt_bound`) is overridable by it - so one cached binary per
  (backend, spec) serves every bound under test; the suite codes each spec once
  with `mc_smt_bound: 3` and passes `--bound N` per run.
- The independence audit (`independence_import_audit`) already encoded this
  task's exceptions from task 1: ref_mc_smt.rs may import `crate::mc::plan` +
  `crate::mc::smt_plan` (and, per the established fuzz precedent,
  `crate::codegen::mc_concepts` for the wire format) but not monitor/fsm/
  printer/backends - it passes on the new file.
- The liveness cycle query is VACUOUS at bound 1 (no a<b pair exists) - both
  generated engines and the executor skip it and report pass-within-bound;
  pinned by the unit test and the parity suite's bound-1 liveness case.
