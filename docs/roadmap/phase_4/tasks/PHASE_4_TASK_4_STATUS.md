# Phase 4, Task 4 - Status

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-18

## Files added / changed / removed
- `tests/backend_parity_test.rs` (changed): MC invocations switched from same-seed fuzz to seedless exact mode; new `McExact` parser (status + summary fields + counterexample fields); Action 1 test now asserts full field-identity across backends on ALL property-bearing examples; Action 2 replaced with a re-pinned value table covering all 15 examples; forced-violation (`--violate`) subtest removed (fuzz-only mechanism).
- `tests/monitor_integration_test.rs` (changed): injected-bug run switched to plain exact mode with the exact-format violation parser; correct-spec test renamed to `mc_correct_spec_reports_no_property_violation_both_backends` and re-pinned to the exact-mode result (FAIL(deadlock) at depth 3, no property violation). Production debug-monitor tests untouched.
- No source files changed (formats were pinned by tasks 2/3; only driving mode + expected values moved).

## Old -> new value changes (D-P4-14: expected to differ from fuzz-era pins)

### backend_parity_test.rs
Fuzz era (`--bound 8 --runs 20 --seed 777`): natural violations on exactly TWO examples - `13_properties_safety` (`no_bad`, run 0) and `41_property_monitors` (`fin_acked_bounded`, run 19); forced violation on 41 pinned at trigger=1/violation=5; all other examples completed without violations.

Exact mode (re-pinned from actual output): **6 pass-complete / 9 fail**.

| Example | Old (fuzz) | New (exact) |
|---|---|---|
| 04_tcp_handshake_like | no violation | FAIL deadlock step=2 (states=4 transitions=3 max_depth=2) |
| 13_properties_safety | `no_bad` run 0 | FAIL property-violation `no_bad` step=1, no trigger (states=1 transitions=2 max_depth=0) - same label, now at the deterministic first-violation position |
| 14_assumptions_tick | no violation | PASS (complete) states=258 transitions=258 max_depth=257 |
| 15_bounded_eventually | no violation | FAIL deadlock step=2 (states=3 transitions=2 max_depth=2) |
| 16_next_property | no violation | FAIL deadlock step=1 (states=2 transitions=1 max_depth=1) |
| 24_event_no_payload_properties | no violation | PASS (complete) states=1 transitions=1 max_depth=0 |
| 37_assumption_demo | no violation | PASS (complete) states=2 transitions=3 max_depth=1 |
| 41_property_monitors | `fin_acked_bounded` run 19; forced trigger=1/violation=5 | FAIL deadlock step=3 (states=7 transitions=6 max_depth=3) - exact BFS reaches the terminal-state deadlock earlier in traversal order than any property violation |
| 42_bounded_liveness_protocol | no violation | FAIL deadlock step=2 (states=6 transitions=5 max_depth=3) |
| 31_traffic_light_tla | no violation | PASS (complete) states=3 transitions=3 max_depth=2 |
| 32_tcp_endpoint_tla | no violation | FAIL deadlock step=3 (states=11 transitions=14 max_depth=3) |
| 33_german_transfer_tla | no violation | FAIL deadlock step=2 (states=4 transitions=3 max_depth=2) |
| 34_single_lane_bridge_tla | no violation | PASS (complete) states=3 transitions=4 max_depth=1 |
| 35_bounded_buffer_tla | no violation | PASS (complete) states=3 transitions=4 max_depth=2 |
| 36_echo_node_tla | no violation | FAIL deadlock step=2 (states=5 transitions=4 max_depth=2) |

The 8 new deadlock findings are genuine: those specs have terminal states with no
outgoing transition and no `on_error` fallback (D-P4-5). The fuzz walk simply
never reported them. No example hit the default `--max-states` limit, so no
per-example limit raise was needed (task file "Other information").

### monitor_integration_test.rs
- Injected-bug test: driving changed from `--mode fuzz --bound 10 --runs 5 --seed 42`
  to plain exact mode. Counterexample values UNCHANGED (label=`fin_acked_bounded`,
  trigger=1, violation=5 - the walk was already deterministic); new summary fields
  pinned: states=5 transitions=5 max_depth=4 reason=property-violation step=5.
- Correct-spec test: old pin = exit 0 within fuzz bound 4 (zero violations provable
  inside the horizon). New pin = FAIL(deadlock) at depth 3 with states=7
  transitions=6 max_depth=3 and NO property violation anywhere in the report -
  the no-false-positive claim survives, the "passes" framing does not (exact mode
  explores the full graph and finds the terminal state).

## Tests
- `cargo test`: **184/184** (was 184; same count - two tests renamed/restructured, none added/removed net).
- `mc_exact_runs_parity_all_property_bearing_examples`: field-identity Rust vs C on all 15 property-bearing examples (dynamic enumeration incl. `tla_adapted/`).
- `mc_exact_results_match_re_pinned_values`: every example's full result pinned; pass/fail split pinned at (6, 9).
- `mc_catches_injected_bug_both_backends` + `mc_correct_spec_reports_no_property_violation_both_backends`: green in seedless exact mode.
- Production debug-monitor tests (`rust_production_debug_monitors_surface_property_violation`, `c_production_monitors_surface_property_violation_parity`): untouched, green.
- No same-seed fuzz assertions remain in either suite (grep-verified); fuzz stays covered by the codegen flag-sensitivity tests + task 2's spot check.
- clippy: baseline 12 warnings (11 lib + 1 codegen_test), zero from this task; `cargo fmt --check` clean.

## Findings / notes for later tasks
- Exact mode surfaces deadlocks on 8 of 15 property-bearing examples - all specs with terminal states. Task 7 (liveness) and any future spec work should expect FAIL(deadlock) as a legitimate exact-mode outcome, not a regression.
- The re-pinned table is the reference for task 9's benchmark suite: largest current run is `14_assumptions_tick` at 258 states (instant); no example comes close to the default 1M state limit.
