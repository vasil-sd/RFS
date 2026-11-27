# Phase 18, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

None (example specs + test legs only; no `src/` change).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | changed | 7 new pinned test legs (one per FSM B9-B15): run.sh exit 0, pinned PASS summary (state count), pinned FAIL reason/label/step + the CE defect step line (copied from the real capture), pinned JSON label, both-backend parity (3x pass summary), both-backends production compile proofs; suite grows 42 -> 49 legs |
| `docs/llm/TEST_STRUCTURE.md` | changed | +7 inventory rows (one per new test fn, added via `rfs-dev inventory add --doc TEST`); llm-budget raised 1497 -> 1504 |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/{vehicle_power_modes,atm_transaction,uav_flight_modes,cnc_tool_change,printer_thermal,plc_conveyor_seq,watchdog_reset}/README.md` | new | 7 template-conformant walkthroughs (PHASE_18.md shared template): What it demonstrates / The machine (mermaid stateDiagram-v2) / RFS modes demonstrated (one subsection per mode with rationale) / Files layout / Build-run-test / Expected results with the negative CE walkthrough + fix proof |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/{vehicle_power_modes,atm_transaction,uav_flight_modes,cnc_tool_change,printer_thermal,plc_conveyor_seq,watchdog_reset}/` | new | 7 control-FSM example projects: `pass.rfs` + `fail.rfs` (one defect) + `fixed.rfs` (= pass.rfs) + `run.sh` + `captures/` (pinned text + JSON CEs) each |

## Changed functionality

None - no command behaviour changed; new example projects + test legs only.

## Removed functionality

None.

## New functionality

- 7 real-world control FSMs (automotive / financial / aerospace / machining / safety)
  modelled at their discrete mode layer, each demonstrating the full RFS workflow:
  specify -> analyze (`rfs validate`/`rfs info`) -> model-check positive (PASS complete)
  and negative (a pinned counterexample) -> generate production code in both backends.
- Safety-interlock showcases: high-load-rail gate across the RUN->SLEEP brown-out
  (vehicle_power_modes, with wake liveness under an environment assumption),
  no-cash-without-authorization + lockout-after-3 + always-return-card
  (atm_transaction), arm-guard GPS/battery gating + same-sample geofence reaction +
  low-battery failsafe (uav_flight_modes), exactly-one-tool-in-spindle mutual exclusion
  + M06 ordering + axis soft limits (cnc_tool_change), thermal-runaway hard abort from
  every active mode + motion interlock + pause preservation (printer_thermal),
  downstream-clear station interlocks + load conservation (plc_conveyor_seq), and the
  valid-feed-window enforcement + no-spurious-reset with a pre-feed countdown witness
  (watchdog_reset).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` (7 new fns) | changed | one pinned test leg per FSM - pins the failures so a regression in CE format or codegen is caught; suite grows 42 -> 49 legs |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (49) | OK - 2214.90 s, all 7 new legs green |
| `cargo test` (root, all binaries) | OK - 646 pass / 0 fail |
| `cargo test --test docs_md_test` (5) | OK - the 7 new READMEs are in its file set and parse/render clean |
| `cargo test --test dev_docs_test` (9) | OK - after the TEST_STRUCTURE budget raise |
| `rfs-dev inventory audit` | EXACT - SRC 1039 fns / 217 types, TEST 1156 fns / 35 types, zero per-file diffs |
| `cargo clippy --all-targets` | OK - exit 0; zero warnings in this task's files (baseline unchanged) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- Anonymous event payloads do not parse: `event E(x: bit<8>)` fails with the confusing
  "trailing input after RFS source" - P1 requires a named variant (`event E(V(x: bit<8>))`).
  All seven specs use the named-variant form; future Phase 18 tasks must too.
- `input_is(Group(_))` (bare group pattern + wildcard field) in an assumption or property
  matches NO struct input - the obligation can never be satisfied and exact MC reports a
  FALSE deadlock at depth 6 (reproduced on a minimal probe). The full variant pattern with
  a bound name (`input_is(Wake(W(src)))`) matches correctly. Hit on B9's `env_wake`
  assumption; fixed before pinning.
- `input_is(E) -> P` evaluates P on the POST-transition sample (the configuration E lands
  in), not the source state: `input_is(M06) -> state == Home` failed on the legitimate
  Home->MoveTo transition. Reformulated as "every honored M06 lands in MoveTo" (B12).
  Likewise `next (P)` looks one sample PAST the consumption sample (B11's geofence property
  initially checked the wrong sample and was corrected to the same-sample form).
- B11 first draft: arming on a dead battery reached ArmedIdle with `batt <= 5`, violating
  `low_batt_failsafe` - the model checker caught it before pinning; the arm guard now
  carries a battery-margin conjunct (documented in the README).
- State-budget scaling (the plan's simplify-and-note rule): B13 at bit<4> position vars
  reached 34013 states - rescaled to bit<3> (8765 states); B15 at bit<8> vdd + win_hi=12
  reached 49176 states - rescaled to bit<6> vdd, win_hi=8/win_lo=3 (5140 states). Both
  simplifications are noted in the READMEs.
- `rfs mc-ref fail.rfs --counterexample-format json` exits 1 (the FAIL status), so run.sh
  must wrap it in `set +e; ...; set -e` like the text form - an unguarded `&&` chain stops
  before the fix/capture steps.

## Other important findings/information
- B15's `last_feed_rem` witness exists because RFS properties observe the post-transition
  configuration: an accepted feed re-arms `remaining` in the same step, so "the feed was
  too early" is only observable through the countdown captured BEFORE the re-arm. Without
  the witness, the window-enforcement property cannot distinguish a petted too-early feed
  from a legitimate in-window one (both land with `remaining == win_hi`).
- B14's reachable state space is tiny (11 configurations) because the hand-off transitions
  tightly constrain the station-mode combinations - the full 2x4x4x4x8 product is mostly
  unreachable. The interlock invariants still bind: the pinned CE shows A processing into
  an occupied B exactly as the defect intends.
- All seven `fixed.rfs` files are byte-identical to their `pass.rfs` (the fail->fixed diff
  is exactly the one-line defect reversal), matching the Phase 17/3 discipline.
