# Phase 3 - Implementation status

> Written once when all exit criteria of `PHASE_3.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/monitor.rs` | Shared PSL-to-monitor compiler: `compile_formula` lowers a validated property formula to a `MonitorPlan` (DFA-style `MonNode` tree) under restrictions R1-R4, plus a `ReferenceMonitor` for tests; single source of truth so Rust and C generate identical monitor behavior (20 unit tests) |
| `tests/backend_parity_test.rs` | Cross-backend differential suite (3 tests): same-seed MC runs over all property-bearing examples must be field-identical between Rust and C (results, violation labels, counterexample fields); both backends' generated test executables must agree on every example |
| `tests/monitor_integration_test.rs` | Monitor integration suite (4 tests): drives an injected bug through the MC in BOTH backends (online detection + property-specific counterexamples) and exercises debug-gated production monitors (Rust `#[cfg(debug_assertions)]`, C `-DRFS_PROPERTY_MONITORS`) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/parser.rs` | changed | `always[N]` operator; task-23 grammar fix: formula-level `&&`/`\|\|`/`!` with mixed atom kinds (expression fallthrough restricted to the non-boolean level) - 17 unit tests |
| `src/ir.rs` | changed | additive `bounded_always` IR variant for `always[N]`; old IR still deserializes, no `IR_VERSION` bump |
| `src/printer.rs` | changed | round-trip printing of `always[N]` and formula-level boolean operators - 9 unit tests |
| `src/validator.rs` | changed | `always[N]` validation (positive bound; zero bound rejected), typing for the new formula shapes - 19 unit tests |
| `src/codegen.rs` | changed | `BackendOptions.with_property_monitors` + `--with-property-monitors` CLI flag (accepted on the `production` command of ANY backend only; `test`/`mc` always include monitors) |
| `src/codegen_fsm.rs` | changed | shared production-FSM analysis: MonitorPlan compilation wired into codegen; legacy post-hoc check planning removed |
| `src/main.rs` | changed | CLI parsing for `--with-property-monitors` |
| `src/lib.rs` | changed | module wiring (`monitor` module) |
| `src/backends/rust_fsm.rs` | changed | online monitor rendering: `Mon_*` structs, per-sample stepping, `StepError::PropertyViolation { label, step, trigger_step }`, debug-gated emission (zero cost in release); post-hoc `property_check`/`assume_check` removed |
| `src/backends/rust_test.rs` | changed | test codegen always includes monitors (D-P3-6) |
| `src/backends/rust_mc.rs` | changed | online property detection during MC search + property-specific counterexample fields; assumption pruning now covers liveness assumptions |
| `src/backends/c_fsm.rs` | changed | C mirror of rust_fsm: `mon_*` structs, `rfs_sample_t`, `#ifdef RFS_PROPERTY_MONITORS` gating, `RFS_PROPERTY_VIOLATION` status + `prop_label`/`prop_step`/`prop_trigger_step`; post-hoc checks removed |
| `src/backends/c_test.rs` | changed | C test codegen always includes monitors (parity with Rust) |
| `src/backends/c_mc.rs` | changed | C MC online detection + same counterexample fields as Rust (field-identical, pinned by the parity suite) |
| `src/bin/generate_code_tests.rs` | changed | aggregate generation over the updated example set (171 modules) |
| `src/bin/test_generated_code.rs` | changed | runs the regenerated aggregate crate (513 tests) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | changed | section 5.10 `always[N]` + monitor semantics; section 6 formula EBNF rewritten for the task-23 grammar; section 7 additive IR note; section 12 subset marked implemented; NEW section 21 with D-P3-1..D-P3-6 decision tables + R1-R4 |
| `KB.md` | changed | section 1 state table (Phase 3 row: 42/12 examples, 170 tests, 513 generated, 224 SFT); section 2.6 `always[N]` + monitors + task-23 grammar fix; section 3.9 example; section 4.2 `src/monitor.rs` row; section 7 limitations updated; section 13 Phase 3 notes |
| `BUILD.md` | changed | section 3 counts (170 main with breakdown, 513 generated, 42/12 examples) + monitor/parity suite descriptions; section 4 `--with-property-monitors` rows; section 5 post-Phase-3 baseline |
| `CODEGEN.md` | changed | `BackendOptions.with_property_monitors` + parse rule; NEW section 5.4 shared monitor compiler (R1-R4, identical warnings); Phase 3 testing conventions; invariant 6 |
| `src/backends/RUST.md` | changed | section 2.1 anatomy (Sample, debug-gated StepError::PropertyViolation, post-hoc functions gone); NEW section 2.3 online monitors; section 4 MC online detection + counterexample format; R1-R4 wording |
| `src/backends/C.md` | changed | section 4 full monitor-parity description (`mon_*` structs, `rfs_sample_t`, `#ifdef` gating, `RFS_PROPERTY_VIOLATION` fields); section 5 online detection + same counterexample fields |
| `START.md` | changed | section 1 snapshot: Phase 3 IMPLEMENTED -> COMPLETE (close-out), next work Phase 4; counts 170/513/42/12/224 |
| `docs/roadmap/PHASE_3_PLAN.md` | changed | "Plan corrections" table appended during execution (4a, 4b, 6, 10a, 11a, 13a + new tasks 23, 24); initial planned-tasks table untouched |
| `docs/roadmap/PHASE_3_TASK_{13,15-24}.md` (+ `_STATUS.md`) | new | per-task implementation-detail files + completion reports per the two-stage protocol (tasks 1-12 predate the protocol; task 14 was executed without a dedicated file - see findings) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/valid/41_property_monitors.rfs` | new | property-bearing example exercising monitors across all commands/backends (parity + integration suites anchor on it) |
| `examples/valid/42_bounded_liveness_protocol.rfs` | new | bounded-liveness protocol example (`always[N]` + assumptions); its formula shape motivated the task-23 grammar fix |
| `examples/invalid/invalid_always_zero_bound.rfs` | new | must-fail example: `always[0] (...)` rejected by validation (task 14) |
| `examples/valid/13_properties_safety.rfs` | changed | task 24: Stay self-loop so the shared TestPlan positive walk no longer violates the example's own `no_bad` property - generated test executables now pass in both backends |
| `examples/valid/tla_adapted/33_german_transfer_tla.rfs` | changed | off-by-one bound fix: `give_up_bounded` deadline extended by one step to match exact D-P3-3 semantics (GiveUp fires at trigger+4) |
| `training/{properties_temporal_logic,model_checking_properties,examples_explain,model_checking_space}/rfs_training.jsonl` | changed | task 19: +5 SFT examples (`always[N]`, monitor compilation, R1-R4, online MC monitoring, counterexamples); first valid snippets ever in `properties_temporal_logic` (0 -> 3) |
| `training/<cat>/snippets/**` | changed/new | regenerated (task 20): 125 valid / 125 invalid snippets; +5 new valid snippet files |
| `all_rfs_training.jsonl` | changed | aggregated SFT set 219 -> 224 examples |

## Changed functionality
- Property checking in generated production code (both backends): post-hoc end-of-trace checks replaced by ONLINE monitors stepped on every sample from a shared `MonitorPlan` - a violation is reported at the confirming sample with label + trigger step, not after the trace ends.
- Model checker (both backends): property detection is now online during search using the same monitor machinery as production; counterexamples carry property-specific information (`label`, `step`, `trigger_step`).
- Assumption handling in MC: assumption-based input pruning now also covers liveness assumptions (previously safety only).
- Generated test executables (both backends): always include property monitors (D-P3-6) - a spec whose representative walk violates its own property now fails its own generated tests, by design.

## Removed functionality
- Post-hoc `property_check` / `assume_check` functions in generated production code (Rust and C): superseded by online monitors; zero occurrences remain in `src/`.
- Legacy post-hoc check planning in the shared production-FSM analysis (`src/codegen_fsm.rs`).

## New functionality
- `always[N]` property operator (bounded, positive N) across parser -> IR (additive `bounded_always`) -> printer -> validator -> both backends.
- Shared PSL-to-monitor compiler (`src/monitor.rs`): `compile_formula` builds a `MonitorPlan` under restrictions R1-R4; out-of-subset formulas produce identical skip warnings in both backends.
- Per-backend monitor rendering: Rust `Mon_*` structs + `StepError::PropertyViolation { label, step, trigger_step }` (debug-gated, zero cost in release); C `mon_*` structs + `RFS_PROPERTY_VIOLATION` status with `prop_label`/`prop_step`/`prop_trigger_step` (`-DRFS_PROPERTY_MONITORS` gating).
- `--with-property-monitors` CLI flag: production builds of ANY backend opt in to monitors; `test` and `mc` commands always include them.
- Cross-backend differential parity suite (`tests/backend_parity_test.rs`): same-seed MC results, violation labels, and counterexample fields must be field-identical between Rust and C; generated test executables must agree on every example.
- Monitor integration suite (`tests/monitor_integration_test.rs`): injected-bug drives through the MC in both backends + debug-gated production monitor exercise.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/backend_parity_test.rs` | new | 3 tests: cross-backend differential parity (same-seed MC field-identity over all property-bearing examples; test-executable agreement in both backends) |
| `tests/monitor_integration_test.rs` | new | 4 tests: injected bug through the MC in BOTH backends (online detection + counterexample fields); debug-gated production monitors (Rust cfg / C define) |
| `tests/codegen_test.rs` | changed | 23 tests: added `rust_production_flag_toggles_property_monitors`, `c_production_flag_toggles_property_monitors`, `mc_and_test_always_include_property_monitors` + flag CLI parsing |
| `tests/integration_example_test.rs` | changed | 2 tests: `BackendOptions` construction updated for the new `with_property_monitors` field (behavior unchanged) |
| lib unit tests (`src/*.rs`) | changed | 86 total (main suite grew 125 -> 170 across all binaries): parser 17 (incl. task-23 formula-level boolean operators), printer 9, validator 19, monitor.rs 20 (R1-R4 plan structure + pinned D-P3-3 semantics), codegen_fsm/test/mc 3 each |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (exit 0) |
| `cargo test` (170 total) | OK |
| - lib unit tests (86) | OK |
| - `frontend_test` (41) | OK |
| - `codegen_test` (23) | OK |
| - `examples_test` (3; all 42 valid parse+validate+round-trip, all 12 invalid rejected) | OK |
| - `training_test` (4) | OK |
| - `library_comparison` (4) | OK |
| - `integration_example_test` (2) | OK |
| - `monitor_integration_test` (4) | OK |
| - `backend_parity_test` (3) | OK |
| `cargo clippy --all-targets` | OK (no errors; 12 pre-existing warnings: 11 lib + 1 codegen_test - unchanged from baseline) |
| `cargo fmt --check` | OK (clean) |
| `cargo generate-code-tests` (171 modules) | OK |
| `cargo test-generated-code` (513) | OK |
| `cargo run -- validate --input rfs examples/valid` | OK (exit 0; known priority warnings only) |
| `gcc -Wall` spot check, example 41 C output: mc full link; production `-c` with and without `-DRFS_PROPERTY_MONITORS` | OK (monitor code zero warnings; only pre-existing `-Wmaybe-uninitialized` on `outs`, identical across variants) |

## Found problems/issues/mismatches
- Example 42 parser constraint: formula-level `&&` with mixed atom kinds (`state == Sending && output_has(Sent)`) - the expression fallthrough absorbed `output_has` as a variable reference, leaving `(Sent)` trailing. Fixed by the task-23 grammar fix (fallthrough restricted to the non-boolean expression level; formula-level `&&`/`\|\|` build `Formula::And/Or`).
- Example 33 off-by-one bound: `tla_adapted/33_german_transfer_tla.rfs` `give_up_bounded` deadline was one step too short under exact D-P3-3 semantics (GiveUp fires at trigger+4); bound corrected.
- Example 13 self-violation (found in task 17, resolved in task 24): the shared TestPlan positive walk took ToBad -> Bad at step 1, violating the example's own `no_bad` property - its generated `test` executable failed identically in BOTH backends (parity held; a spec-level mismatch, not a backend divergence). Resolved with a minimal Stay self-loop so the walk stays within the property.

## Other important findings/information
- Task 14 (invalid zero-bound example) was executed inside batch commit `e0bc3cb` without a dedicated task file (trivial single-file task); its deliverable `examples/invalid/invalid_always_zero_bound.rfs` is in place and covered by the examples suite. Hence the task-file set is `PHASE_3_TASK_{13,15-24}.md`.
- Parser constraint workaround documented in KB: the task-23 grammar fix SUPERSEDES the earlier example-42 workaround (plan correction 13a); pure-expression conjunctions in properties now parse as `Formula::And(Expr, Expr)` instead of `Expr(Binary(LogicAnd))` - additive for JSON (old IR still deserializes), no `IR_VERSION` bump.
- Generated C production code carries a pre-existing `-Wmaybe-uninitialized` warning on the `outs` array in `fsm_step` (3 sites for example 41) - present identically with and without monitors, i.e. not introduced by Phase 3; monitor code itself compiles warning-free under `gcc -Wall`.
- Commit grouping: Phase 3 implementation was committed incrementally per task (protocol step t): commits `e0bc3cb`..`752702a` cover tasks 13/14/15-24 and 18-21; the close-out commit covers only the bookkeeping files (this status file, task-22 status, INTRO.md table, START.md snapshot, TASKS.md entry).
- Next phase entry point: `docs/roadmap/PHASE_4.md` (Scalable model checking); planning-stage trigger: "let's make a plan for Phase 4".
