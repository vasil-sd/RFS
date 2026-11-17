# Phase 16, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_16_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | no new source files - the engine, config, CLI and diff suite already exist from tasks 0-1 |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/mc/ref_mc_exact.rs` | changed | new `pub(crate) fn match_effects_all` - the ALL-matches enumeration of the common core's matching semantics (every matching transition in firing order as an effect, on_error only when nothing matched, empty = unexpected input; deterministic first-match is its one-element head). The `run()` BFS and `collect_min_labels` now select effects by strategy: deterministic keeps the single-effect path byte-for-byte, `--nondet` enqueues one successor per effect (each carrying its tid in parent/adj); the existing `any_successor` deadlock check generalizes (no effect for ANY input = deadlock). `liv_analyze` gains a `complete: &[bool]` parameter (per-config fully-expanded flag, already tracked by the BFS) and a `nondet: bool` parameter: in nondet mode a bad cycle must be STRONGLY FAIR (D-P4-11) - a config is eligible only if `adj[c].len() == 1 && complete[c]`; the witness search tries EVERY strongly-fair-eligible on-cycle pair at the falsifying q (the first may sit in a mixed SCC whose cycle passes through ineligible configs) and the closing DFS descends only into eligible pairs. Deterministic mode: every config eligible, behavior unchanged. Four new unit tests pin the enumeration (multi-effect firing order, on_error fallback, empty = unexpected), the nondet exploration breadth, and the fairness guard flipping a deterministic liveness FAIL to a nondet PASS. |
| `src/mc/ref_mc.rs` | changed | `RefConfig.nondet: bool` field (Default = false). Widened to `pub(crate)` for the engine's enumeration: `bool_of`, `pattern_bindings`, `input_group`, `input_matches_variant`, `var_map`, `state_index`, `output_variant_idx`. `step_trace` re-signatured from `&[InputVal]` to `&[JsonStep] + nondet: bool`: deterministic replay is unchanged (first-match per step); nondet replay selects the enabled effect whose (to_state, vars, outputs) matches the CE's claim for that step and errors if the claim is not an enabled effect at all - verification stays a genuine ground-truth check for nondet CEs. `verify_counterexample` passes the raw steps through (both the safety and liveness branches). |
| `src/cli/commands/mc_ref.rs` | changed | new `--nondet` flag parsed in `run_cli`, `McRefOptions.nondet` field (Default = false), passed into `RefConfig`. Module doc + usage text rewritten: the "Not applicable: --nondet" note is REMOVED and replaced with the real description (fires every enabled transition as its own successor; liveness then requires strongly-fair bad cycles). |
| `tests/ref_mc_diff_test.rs` | changed | new `compare_one_nondet` helper + two diff tests (`ref_mc_vs_generated_all_eligible_examples_nondet`, `ref_mc_vs_generated_random_specs_nondet`): ref-mc `nondet: true` vs BOTH generated binaries with `--nondet --counterexample-format json` over eligible examples + 20 pinned-seed random specs; asserts status/exit agreement, EXACT state counts, reason/label/step on failure (transitions NOT asserted - documented convention), CE replay through the nondet-aware `verify_counterexample`, and handles the sample-0 assumption-violation asymmetry. The existing `ref_cfg()` literal gains `nondet: false`. |
| `tests/toolchain_fuzz_test.rs` | changed | the one other `RefConfig` struct literal in the tree updated with `nondet: false` (exhaustive-literal compile fix). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed | +1 row: `src/mc/ref_mc_exact.rs::match_effects_all` (depends on the common-core matching items it calls). |
| `docs/llm/TEST_STRUCTURE.md` | changed | +8 rows: 3 diff-suite rows (`compare_one_nondet`, the two nondet tests) + 5 engine-test rows (`nondet_cfg` helper, 4 unit tests); llm-budget 1327 -> 1335. |
| `docs/dev/mc_engine.md` | changed | dev:mc-engine-exact documents `match_effects_all` as the all-matches enumeration sharing one matching core with `match_effect`; the run() BFS paragraph now describes the strategy switch; dev:mc-engine-liveness documents the D-P4-11 strong-fairness eligibility guard (exactly-one-explored-successor AND fully-expanded), the every-eligible-candidate witness search, and the eligible-only DFS descent. |
| `BUILD.md` | changed | mc-ref row: `--nondet` moved from the N/A list to the flag list with its description; the N/A list keeps only the fuzz concepts + `--threads`/`--fast`. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | appended | the task-2 entry. |

## Changed functionality
- `rfs mc-ref` CE verification (`step_trace` / `verify_counterexample`): the replay
  now takes the raw JSON steps and a nondet flag instead of pre-parsed inputs -
  deterministic behavior is identical (first-match per step), but the function's
  input type changed from `&[InputVal]` to `&[JsonStep]`.
- The ref exact engine's liveness analysis (`liv_analyze`) gained two parameters
  (`complete`, `nondet`); deterministic-mode results are unchanged (every config
  eligible, single candidate per q as before).

## Removed functionality
- None. The "--nondet not applicable" CLI note is removed because the flag now
  exists; no behavior was dropped.

## New functionality
- `rfs mc-ref --nondet`: the all-enabled-successor strategy - every enabled
  transition fires as its own successor (mirroring the generated engines' nondet
  mode), with D-P4-11 strong-fairness liveness semantics. First independent
  ground truth for nondet: both generated engines derive from one shared plan,
  so agreement with ref-mc's own enumeration is a new signal.
- Nondet CE verification: a counterexample's claimed step must be ONE of the
  enabled effects (matched on to_state + vars + outputs), not just the first -
  invalid nondet CEs are still rejected.
- Cross-backend nondet differential suite over eligible examples + random specs
  (status/exit/states exact, reason/label/step on failure, transitions not
  asserted).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `src/mc/ref_mc_exact.rs::tests::match_effects_all_returns_every_matching_transition` | new | two same-pattern transitions with different priorities both match one input; the enumeration returns both in firing order, first-match returns only the head. |
| `src/mc/ref_mc_exact.rs::tests::match_effects_all_on_error_fallback_and_empty` | new | on_error is an effect only when nothing matched (sentinel tid); no on_error + no match = empty enumeration. |
| `src/mc/ref_mc_exact.rs::tests::nondet_explores_every_enabled_successor` | new | nondet reaches the lower-priority target state the deterministic first-match never explores (2 vs 3 states). |
| `src/mc/ref_mc_exact.rs::tests::nondet_liveness_requires_strongly_fair_cycles` | new | deterministic FAILs an eventually-property on a two-state cycle; nondet PASSes because the branching config is not strongly fair - the D-P4-11 guard. |
| `tests/ref_mc_diff_test.rs::compare_one_nondet` (+2 tests) | new | the --nondet cross-backend comparison (see Changed source files). |
| `tests/toolchain_fuzz_test.rs` RefConfig literal | changed | +`nondet: false` (compile fix, no behavior change). |

## Tests run status
- cargo test 563/0 (= Phase 15 baseline 557 + 4 unit + 2 diff tests)
- cargo test --workspace 691/0 (= 685 + 6)
- cargo test --features smt 565/0 (= 559 + 6)
- cargo test-generated-code 1683/1683 (UNCHANGED - no template touched)
- cargo fmt clean; cargo clippy --all-targets at baseline (4 needless-borrow
  warnings in the new test code fixed; zero warnings in any changed file)
- rfs-dev inventory audit EXACT (SRC 966 fns / 200 types, TEST 988 fns /
  34 types - zero per-file diffs); rfs-dev sync-check clean; dev_docs_test 9/0

## Found problems/issues/mismatches
- The session initially ran the whole sweep under the WRONG toolchain (the
  sandbox's nightly 1.102 picked up from a bare PATH, instead of the canonical
  project-local stable 1.99.0 per TOOLCHAIN.md section 2). All verification was
  redone under the documented activation block; the recorded totals above are
  the canonical-toolchain numbers.
- Stale `librfs_adapters-*.rlib` artifacts from a previous toolchain made
  rfs-adapters' nostd_minimal tests fail E0514 (the test invokes rustc directly
  against whatever rlib it finds first in target/debug/deps). Removing the stale
  rlibs fixed it - the same cross-toolchain target-contamination finding already
  recorded in Phase 15 task 6.
- The TEST_STRUCTURE.md llm-budget header (1327) was not updated by the
  interrupted session; the dev_docs_test sync gate caught it (line count 1335
  exceeds declared budget). Fixed to 1335 in this session.

## Other important findings/information
- The fairness guard needed NO rewrite of the stay-graph construction: the BFS
  already tracked per-config expansion completion implicitly (the level loop),
  so a `complete: Vec<bool>` flag set after each config's input loop was enough
  to express "exactly one explored successor AND fully expanded". The one real
  change beyond adding the guard: the witness search must try EVERY eligible
  on-cycle pair at q (not just the first), because the first may sit in a mixed
  SCC whose only cycle passes through ineligible configs - the gated DFS cannot
  close it and would silently skip a real violation.
- Liveness examples DO fail under --nondet in the eligible set (42_bounded_
  liveness_protocol: deadlock; 44_token_ring: property-violation), so the
  cross-engine diff suite exercises the fairness guard on real specs, not only
  on the unit-test fixture.
