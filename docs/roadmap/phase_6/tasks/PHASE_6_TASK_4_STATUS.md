# Phase 6, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into follow-up phase plans.

## Added source files

- None.

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/rust_mc.rs` | change | Plan correction 4b (recorded in PHASE_6_PLAN.md): the exact-mode BFS expanded one extra level after a property violation or state-limit hit because the Phase-B `break 'level` skipped the per-configuration stop check; a post-level guard now stops immediately per SEMANTICS 9.5, matching the C engine's loop guard. Counters only - verdict/label/step/counterexample unchanged. |
| `src/ref_mc.rs` | change | Differential-suite support + two bug fixes: new public API `step_trace` (replay a fixed input sequence through ref_mc's own transition logic) and `verify_counterexample` (validate a generated engine's JSON counterexample against ref_mc's independent evaluators - transition relation, monitor compilers, liveness product automaton), with lib unit tests; `RefResult.deadlock_mons` exposes the persistent monitor-state projection of a deadlocked terminal configuration so an external verifier can re-derive the SEMANTICS 8.4 "no surviving successor" judgment; liveness end-component DFS now uses GLOBAL visited pruning (path-only pruning re-explored diamond subgraphs exponentially and spun forever on small graphs - found via specgen seed 900_006); sample-0 minimal-label collection now records EVERY monitorable property whose monitor fails at sample 0 (`run()` itself still reports the first in declaration order). |

## Added/changed test files

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_diff_test.rs` | new | 4.1: ref-MC vs generated-MC differential - status agreement + CE-replay validity on all eligible examples (all valid except the two intractable, `EXCLUDED`) + 20 specgen random specs (pinned seeds); label+step equality where the minimal violation is unique (`AMBIGUOUS` list currently empty); exact `states` counts; no-false-PASS negative control (`tiny_state_limit_never_pass_complete`); `mc-ref` CLI vs in-process vs compiled-engine agreement |
| `tests/backend_parity_test.rs` | change | 4.2: new `mc_forced_violation_parity_all_assumption_bearing_examples` - for every assumption-bearing example x label (dynamic enumeration) + one combined all-labels run: both backends in fuzz mode, pinned seed 777, `--runs 5 --bound 10`, `--violate <label>:1.0:100%`; byte-identical exit code + stdout |
| `tests/common/prod_driver.rs` | new | shared driver machinery for 4.3/4.4/4.5: seeded LCG input scripts (task-3 lineage), action-3 input-level drivers with a fixed raw-value stdout contract, sample-level drivers over the generated property monitors, production-artifact builder (`production_artifacts`), flattened variant tables (now carrying group + variant identity for pattern matching) |
| `tests/prod_parity_test.rs` | new | 4.3: production-FSM cross-backend random-input parity - EVERY valid example (47 files incl. `tla_adapted/`), property monitors ON, identical seeded scripts (seed 43210, 200 steps) driven through both backends' drivers; asserts byte-identical stdout + exit code and that the suite exercised the unexpected-event path |
| `tests/monitor_ref_test.rs` | new | 4.4: specs 43/44/45 - hand-written reference monitors (independent Rust engine in-file + independent C compiled per spec with `gcc -Wall`) vs the generated monitors over seeded random traces (seed 90717, 200 steps) AND crafted sample scripts covering boundary values, bit-width wraparound, obligation deadlines (confirmed exactly at trigger+bound-1), re-trigger after clear, sticky failure and same-sample declaration-order tie-breaks; also cross-checks the generated Rust vs C sample-level drivers on the crafted inputs |
| `tests/psl_translation_test.rs` | new | 4.5: independent recursive window evaluator (own Formula/Expr descent, own bit-vector semantics per SEMANTICS 2.2, obligation scanning by window search rather than countdown automaton) cross-checked against generated monitor verdicts over random traces (seed 7331, 200 steps) for ALL property-bearing examples (dynamic enumeration; 19 examples, 28 monitored properties compared, >=1 live confirmation asserted); `RFS_PSL_REF_TOOL` availability-gated hook with its interface documented in the module docs |
| `tests/codegen_test.rs` | change | clippy: `parse_module(&PathBuf)` -> `&Path` (last pre-existing test warning) |

## Changed functionality

- The generated Rust exact-mode MC no longer expands one extra BFS level after a
  property violation or state-limit hit (correction 4b): the reported
  `states`/`transitions`/`max_depth` counters on FAIL / state-limit runs now
  match the C engine and SEMANTICS 9.5. Verdicts, labels, steps and
  counterexamples are unchanged.
- One test-infrastructure contract extension worth noting:
  the action-3 driver's ASSUME/PROP stop lines now echo the recorded sample
  (`out=`/`st=`/`vars=`). This reflects a generated-engine behavior, not a
  change to it: `commit_step` pushes the transition into the trace BEFORE the
  assumption/property check, so a Stop-violated step is RECORDED but NOT
  applied (state/vars freeze; live property monitors are not stepped on an
  ASSUME-stopped sample, while the probe's fresh monitors step it). Both
  backends behave identically here (verified by 4.3 byte parity).

## New functionality

- No new CLI surface. New public lib APIs in `src/ref_mc.rs`: `step_trace` and
  `verify_counterexample` (task 4.1 differential-suite support; not wired to
  any command). New shared test APIs in `tests/common/prod_driver.rs`:
  `production_artifacts`, `build_sample_drivers` + the sample-driver source
  generators, `flatten_groups`/`FlatVariant { group, variant_name }`, pub
  bit-type maps.

## Removed functionality

- None.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `ref_mc_diff_test` (4 tests) | new | 4.1 suite as above |
| `backend_parity_test::mc_forced_violation_parity_all_assumption_bearing_examples` | new | 4.2 forced-violation parity |
| `prod_parity_test::production_drivers_byte_identical_all_valid_examples` (1 test, 47 examples) | new | 4.3 suite |
| `monitor_ref_test` (2 tests: random traces + crafted scenarios x6) | new | 4.4 suite |
| `psl_translation_test::psl_translation_matches_generated_monitors_all_property_examples` (1 test, 19 examples x 28 properties) | new | 4.5 suite |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` (debug) | OK |
| `cargo test` (full suite, 443 tests / 26 targets) | OK - 0 failed (~216 s) |
| `cargo clippy --all-targets` | OK - no errors; zero warnings in every file this task added/touched; the 11 remaining warnings are the pre-task baseline in untouched `src/` files (`c_fsm.rs` x8, `codegen.rs`, `test_cmd.rs`, `validator.rs`) |
| `cargo fmt --check` | OK (whole repo) |
| per-suite durations | ref_mc_diff ~33 s, backend_parity ~55 s, prod_parity ~1.4 s, monitor_ref ~1 s, psl_translation ~4.4 s - all well under the ~2 min budget |
| `RFS_PSL_REF_TOOL` hook | verified with a quiet tool (passes) and a lying tool (fails with per-label diff); skips cleanly when unset |

## Other information

- Pinned parameters (task file: "pin the seed + step count"): prod parity
  seed 43210 / 200 steps; monitor-ref random traces seed 90717 / 200 steps;
  PSL translation seed 7331 / 200 steps; forced-violation fuzz seed 777 /
  `--runs 5 --bound 10`; ref-MC differential: eligible = all valid examples
  minus `EXCLUDED` (19_guard_complex, 39_endpoint_protocol - the two
  intractable exact-MC specs from task 3) + 20 specgen random specs on pinned
  seeds.
- Engine mismatches found: ONE - plan correction 4b (recorded in
  PHASE_6_PLAN.md): the Rust exact-mode BFS expanded one extra level after a
  property violation / state-limit hit, inflating the reported counters vs the
  C engine and ref_mc; fixed inline during this task's execution per the
  bug-found protocol (fix + suite re-run green). Every OTHER divergence hit
  during development was a bug in the new test harnesses or in the task-3
  reference infrastructure (crate-name dots in scratch tags, driver template
  syntax, C union-member naming conventions, missing `BufRead` import, vars
  rendering separators, format-string arithmetic captures, non-exhaustive
  match arms, an ASSUME-line parse that dropped the recorded phantom sample;
  ref_mc liveness DFS exponential re-exploration on diamond subgraphs).
- The monitorable-subset boundary is respected by construction: 4.4 compares
  only specs' monitorable properties (eventually/until are skipped by
  production codegen with a warning, SEMANTICS 5/6), and 4.5 reads the exact
  monitored set from each artifact's `PROPERTY_LABELS` const rather than
  re-deriving it.
