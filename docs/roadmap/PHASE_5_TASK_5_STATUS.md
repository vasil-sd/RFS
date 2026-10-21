# Phase 5, Task 5 - Implementation status

> Written once when all exit criteria of `docs/roadmap/PHASE_5_TASK_5.md` are
> reached. Not rewritten afterwards; later fixes go into TASKS.md entries and
> follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | The task extends `src/backends/rust_test.rs` (renderer), plus the planner-soundness fix in `src/codegen_test.rs`, the `ExtraFile` plumbing in `src/codegen.rs`/`src/main.rs`, and the signal-renderer extraction in `src/backends/rust_fsm.rs`. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/rust_test.rs` | changed | Renders the four extended test classes into `mod tests` (additive - the three module-level tests are untouched): (1) per-transition `t<id>_<state>_on_<event>` / `..._not_taken` - installs the case's state + COMPLETE variable configuration, drives the concrete input with `ContinueAssumptionHandler`, asserts on the RECORDED trace step (`last.to_state`/`last.outputs`) so the assertion stays correct even when a property violation halts the commit; not-taken asserts `UnexpectedEvent` + no recorded step, or a landing observably different from the transition's own signature (full-value comparison - codegen errors if a signature arg was not statically evaluable); (2) `invariant_<label>` - re-checks the per-sample atom at every sample of the positive drive trace (greedy `StopAssumptionHandler` drive, same policy as the module-level positive test), failing with the step index; bounded invariants check samples `< N`; the atom is rendered against `&Sample` by the shared `rust_fsm::render_signal_expr`; (3) `monitor_<label>_fires_on_violation` - drives the planner's violating trace (initial-config case = plain `Fsm::new()`) and asserts the sticky `property_violation` record holds the target label; (4) `random_trace_seed<seed>_steps<K>` - drives the precomputed LCG index sequence over `ALL_INPUTS` with `StopAssumptionHandler`, prints one deterministic coverage line (`ok`/`unexpected`/`assumption`/`property` counts + distinct visited states). NEW: `emit_fuzz_target(module, variants) -> Result<String, Vec<String>>` emits the SEPARATE `fuzz_target.rs` skeleton - `#![no_main]` + `libfuzzer_sys::fuzz_target!`, first byte = variant selector (`byte % N`), fields decoded per `FuzzDecodePlan` (bool = 1 non-zero byte, bit<W> = ceil(W/8) little-endian masked bytes, enum = constant representative), loop steps the FSM until the input is exhausted; documented cargo-fuzz wiring header + `ADJUST` import note (task 12 wires a real example). `generate_unit_tests` appends it as `CodegenOutput.extra_files`; `emit_test_module` now returns `Result<String, Vec<String>>` (unresolvable output variants / un-evaluable signature args are codegen errors - no silent degradation). |
| `src/codegen.rs` | changed | New pub type `ExtraFile { name, content }`; `CodegenOutput` gains `extra_files: Vec<ExtraFile>` (all seven backend construction sites updated; only the rust `test` command populates it). |
| `src/backends/rust_fsm.rs` | changed | Extracted the per-sample signal rendering out of `Gen` into module-level functions shared with the test renderer: `render_signal_expr(module, variants, sig) -> Result<String, String>` (pub(crate), used by `rust_test` for invariant atoms) + private `signal_expr_ctx` / `input_is_sample_expr` / `output_has_sample_expr`. `Gen::gen_signal` now delegates - monitor rendering is BYTE-IDENTICAL (verified: 45-example production sweep vs the pre-change build, incl. the signal-heavy examples 41/43; no_std body-parity suite still green). |
| `src/codegen_test.rs` | changed | PLANNER SOUNDNESS FIX (found by the task 5 renderer): `plan_monitor_violations` now skips candidate transitions that a higher-firing sibling would preempt - new helper `earlier_sibling_fires(module, t_idx, input_case, env)` mirrors the generated firing order (priority desc, source order) and requires every earlier matching sibling to be disabled by the same pre-step assignment. Without it, example 44's planned drive (`Tick` from `{holder:0, req:255}`) took transition #3 in the planner but the priority-2 grant transition at runtime - the post-step config did NOT violate `holder_no_request` and the generated monitor test failed. Also: `complete_configuration(module, assignment)` (new) makes every planned case carry a COMPLETE variable configuration (module declaration order, representatives for uncovered vars - exactly what `eval_env` assumed statically); applied to transition taken/not-taken cases and the one-step monitor pre-config (`post_vars` was already complete via `apply_assignments`). `build_fuzz_decode` is now pub(crate) (consumed by the fuzz-skeleton renderer). NEW permanent corpus test `planned_monitor_violations_are_executable_and_record_target_first` pins BOTH guarantees for all 45 examples: (1) the drive actually takes the recorded transition (firing-order re-derivation + `apply_assignments == post_vars`), (2) among ALL monitored properties the target label is what the sticky record holds first (reference-interpreter simulation over [initial, post-step], declaration order). |
| `src/main.rs` | changed | `cmd_codegen`: with `--out`, each `extra_files` entry is written next to it (same parent dir) and announced with `wrote <path>`; WITHOUT `--out`, a command with extras is REFUSED (exit 1 + diagnostic naming the files - no graceful degradation, stdout would lose them). Usage text notes the rust `test` requirement. |
| `src/backends/rust.rs` | changed | `TEST_INFO` help text: new summary (five test classes), `--out FILE` documented as REQUIRED with the fuzz-skeleton note. |
| `src/backends/{c_fsm,c_test,c_mc,rust_mc}.rs` | changed | Mechanical: `extra_files: Vec::new()` at the `CodegenOutput` construction sites. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | changed | 3.1: `ExtraFile` + `CodegenOutput.extra_files`; 4.2: extra-file writing + stdout-mode refusal; 5.2: rendering section (the four classes, stable names, trace-step assertion rationale, invariant re-check semantics, monitor-record guarantee, random-trace line format, fuzz skeleton as separate file + task-12 pointer). |
| `src/backends/RUST.md` | changed | File-layout row for `rust_test.rs`; section 3 rewritten: all five rendered test classes with naming conventions + the separate-fuzz-file design. |
| `BUILD.md` | changed | Section 3 counts re-pinned: root 326 -> **333** (lib 178 -> 179, codegen 33 -> 39 incl. the task 5 suite), workspace 334 -> **341**, generated-code 537 -> **1453** (179 modules); alias re-verification line + known-good baseline updated (2026-10-21, after Phase 5 task 5). |
| `KB.md` | changed | Section 1 Tests row: 333 non-doc tests with the current per-suite breakdown; aggregate 1453. 4.5: `--out` note for the test command (extra file next to it, stdout refused). 13.3: the four new generated-test classes documented. 13.5: 1453 count. |

## Other file changes

- `TASKS.md`: one new entry (2026-10-21).
- `docs/roadmap/INTRO.md`: phase-state row 5 -> "tasks 1-5 + 8-9 of 16 done".

## Changed functionality

- `rfs codegen rust test` output GROWS: the four new test classes are appended to `mod tests` (module-level tests unchanged), and the command now ALSO emits `fuzz_target.rs`.
- CLI contract change for that one command: `--out FILE` is now REQUIRED (stdout mode exits 1 with a diagnostic); with `--out`, `fuzz_target.rs` is written next to it. All other commands are unaffected (no extras -> identical behavior).
- Planner data change (additive in effect): planned cases carry complete variable configurations; example 44's `holder_no_request` monitor-violation case now plans a different (correctly executable) drive - Request from `{holder:1, req:255}` instead of the preempted Tick.

## Removed functionality

- None.

## New functionality

- Rust `test` artifacts carry per-transition unit tests, invariant re-checks, property-monitor debug tests, and a seeded random-trace coverage test (stable names, parity-ready for task 6).
- Separate dependency-free-by-construction `fuzz_target.rs` libfuzzer_sys skeleton per module (decode strategy from the shared planner).
- Planner guarantees pinned by a corpus test: planned monitor-violation traces are executable as planned AND record the target label first.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `rust_test_emits_all_four_extended_classes_for_property_examples` (tests/codegen_test.rs) | new | 41 + 43: per-transition / invariant / monitor-violation / random-trace classes all present; aggregate crate carries them too and stays libfuzzer-free |
| `rust_test_names_follow_the_pinned_convention` | new | every generated test fn in 41/40 matches the pinned naming scheme (t<id>..._on_..., invariant_..., monitor_..._fires_on_violation, random_trace_seed12345_steps4, module-level names) |
| `rust_test_steps_option_changes_random_trace_class` | new | option sensitivity: steps=2 vs 6 change the random-trace test name and its index sequence |
| `rust_test_emits_fuzz_skeleton_as_extra_file` | new | exactly one extra file named `fuzz_target.rs`; `#![no_main]` + libfuzzer import + `fuzz_target!`; main artifact stays dependency-free; 43's skeleton decodes the bit<4>/bit<32> payload widths (masked little-endian) |
| `cli_test_command_writes_fuzz_target_next_to_out` | new | real-binary e2e: `--out` writes both files (announced on stdout); stdout mode exits 1 with the pinned "emits additional files" diagnostic; production command unaffected in stdout mode |
| `aggregate_crate_carries_new_classes_and_stays_dependency_free` | new | regenerated aggregate (179 modules) contains all four classes and no libfuzzer reference |
| `planned_monitor_violations_are_executable_and_record_target_first` (src/codegen_test.rs) | new | corpus guarantee (all 45 examples): the planned drive takes exactly the recorded transition (firing order + post-vars equality) and the target label is what the sticky record holds first (reference-interpreter simulation); helper `firing_transition` re-derives the runtime winner |
| regenerated aggregate crate (`cargo generate-code-tests`) | changed | 537 -> **1453** generated tests (179 modules), all green |

## Verification

- `cargo build` clean; `cargo test` **333/333** root (was 326: lib 178->179, codegen 33->39); `cargo test --workspace` **341/341** (was 334).
- `cargo generate-code-tests && cargo test-generated-code`: **1453/1453** pass (was 537) - the four new classes present and passing in the aggregate for every property-bearing example.
- 45-example sweep (generate + rustc --test + run each artifact): all green, 417 tests across the valid examples alone.
- Fuzz skeleton compiles against a generated module (stub `libfuzzer_sys` with the same macro shape).
- Production byte-stability: 45-example production sweep vs the pre-change build IDENTICAL (incl. signal-heavy 41/43); no_std body-parity suite green.
- clippy --all-targets zero-new (same 12 baseline: c_fsm x9, codegen from_str, validator, codegen_test &PathBuf); fmt clean.

## Corrections during implementation

- C5-1 (planner bug, found by the renderer): `plan_monitor_violations` ignored sibling firing order - a higher-priority matching sibling can preempt the planned transition at runtime (example 44: priority-2 grant vs unguarded pass on Tick). Fixed with `earlier_sibling_fires`; pinned by the new corpus test.
- C5-2 (planner data gap): planned cases carried PARTIAL variable configurations while their doc comment promised complete ones; the renderer's strictness surfaced it (panic on 41). Fixed at the source with `complete_configuration` (same representative-fill rule `eval_env` always used statically) - all pre-existing pinned planner tests still pass unchanged.
- C5-3 (design decision): `rfs codegen rust test` without `--out` is now a hard error when extras exist (strict, CI/CD-friendly per the user's no-graceful-degradation rule); documented in CODEGEN.md 4.2 + TEST_INFO help.
