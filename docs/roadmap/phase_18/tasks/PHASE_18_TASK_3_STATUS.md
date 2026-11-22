# Phase 18, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (example specs + test legs only; no `src/` change).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | changed | 8 new pinned test legs (one per FSM B1-B8): run.sh exit 0, pinned PASS summary (state count), pinned FAIL reason/label/step + the CE defect step line (copied from the real capture), pinned JSON label, both-backend parity (3x pass summary), both-backends production compile proofs; traffic_light_ctrl + vending_machine additionally pin the smt-bounded verdict (PASS WITHIN BOUND 16); suite grows 34 -> 42 legs |
| `docs/llm/TEST_STRUCTURE.md` | changed | +8 inventory rows (one per new test fn, added via `rfs-dev inventory add --doc TEST`); llm-budget raised 1432 -> 1440 |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/{blc_motor_foc,bms_protection,solar_inverter,wind_turbine_ctrl,hvac_thermostat,traffic_light_ctrl,vending_machine,washing_machine}/README.md` | new | 8 template-conformant walkthroughs (PHASE_18.md shared template): What it demonstrates / The machine (mermaid stateDiagram-v2) / RFS modes demonstrated (one subsection per mode with rationale) / Files layout / Build-run-test / Expected results with the negative CE walkthrough + fix proof; traffic_light_ctrl + vending_machine carry the smt-bounded subsection |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/{blc_motor_foc,bms_protection,solar_inverter,wind_turbine_ctrl,hvac_thermostat,traffic_light_ctrl,vending_machine,washing_machine}/` | new | 8 control-FSM example projects: `pass.rfs` + `fail.rfs` (one defect) + `fixed.rfs` (= pass.rfs) + `run.sh` + `captures/` (pinned text + JSON CEs, smt verdicts for B6/B7) each |

## Changed functionality

None - no command behaviour changed; new example projects + test legs only.

## Removed functionality

None.

## New functionality

- 8 real-world control FSMs (power / energy / building / consumer) modelled at
  their discrete mode layer, each demonstrating the full RFS workflow: specify ->
  analyze (`rfs validate`/`rfs info`) -> model-check positive (PASS complete) and
  negative (a pinned counterexample) -> generate production code in both backends.
- Safety-interlock showcases: latched-fault safe-stop (blc_motor_foc), hard voltage
  bounds + thermal latch (bms_protection), confirmed anti-islanding no-backfeed
  (solar_inverter), overspeed invariant + irreversible cut-out (wind_turbine_ctrl),
  actuator mutual exclusion under hysteresis (hvac_thermostat), conflicting-greens
  exclusion via all-red clearance (traffic_light_ctrl), full-payment + change
  arithmetic invariants (vending_machine), compound stopped-AND-drained unlock guard
  (washing_machine).
- Two small specs (`traffic_light_ctrl`, `vending_machine`) additionally checked via
  `rfs mc-ref --mode smt-bounded --bound 16` (PASS WITHIN BOUND) - the k-step SMT
  safety proof on assumption-free control machines.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` (8 new fns) | changed | one pinned test leg per FSM - pins the failures so a regression in CE format or codegen is caught; suite grows 34 -> 42 legs |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (42) | OK |
| `cargo test --test docs_md_test` (5) | OK - the 8 new READMEs are in its file set and parse/render clean |
| `cargo test --test dev_docs_test` (9) | OK - after the TEST_STRUCTURE budget raise |
| `cargo test` (root, all binaries) | OK - 612 pass / 0 fail (594 post-task-2 baseline + 8 new legs; the 594 = 584 Phase-17 close-out + task 1's 10 legs) |
| `rfs-dev inventory audit` | EXACT - SRC 1023 fns / 215 types, TEST 1092 fns / 35 types, zero per-file diffs |
| `cargo clippy --all-targets` | OK - no errors; 71 warning lines = the exact pre-task baseline (64-line Phase 17 + 7 from task 1A); zero in this task's files |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- smt-bounded models UNHANDLED inputs as self-loops (the machine stays in its state),
  while exact MC kills the path (unexpected termination). A `next`-form property
  anchored on a state therefore passes exact MC but is "violated" by smt-bounded when an
  unhandled input intervenes between the anchor step and the expected successor. Hit on
  B6: the planned bounded-liveness cycle property (`always (bounded_eventually[40]
  (state == AllRed))`) failed under exact MC for a different reason (a stalled trace
  never sends TimerDone - liveness needs an assumption, which smt-bounded rejects), and
  its `next`-form replacement still tripped the SMT engine on an intervening unhandled
  Reset. Resolution: B6's properties are pure invariants / input-anchored implications
  (no `next`, no bounded_eventually) - robust under both engines' semantics. Any future
  Phase 18 spec that demos smt-bounded must follow the same rule.
- bit<8> subtraction underflow wraps: B5's band-edge arithmetic (`setpoint - 10`) wrapped
  to a huge value for setpoints < 10, letting the machine "heat" toward a bogus band and
  violating the hysteresis invariant. Fixed by range-guarding the Setpoint payload
  (20..=240, as a real thermostat would) - noted in the README.
- A terminal state with no outgoing transitions is a DEADLOCK configuration
  (reason=deadlock, exit 1), not a quiet end: B8's Done state deadlocked the exact MC
  until a Reset transition back to DoorLocked was added. Every state needs at least one
  enabled transition for some input.
- Overlapping-guard shadowing by source order (confirms task 2's coap finding): B8's
  first fail.rfs draft kept the guarded drain self-loop alongside the unguarded unlock -
  the drain shadowed the unlock whenever water > 0 and the variant PASSED. The defect had
  to remove the sibling transition, not just unguard the unlock.
- Sandbox environment: `/dev/null` is a regular file here and was again transiently
  overwritten with an ELF binary (8 KB), breaking clippy's target probe ("couldn't read
  from stdin, as it did not contain valid UTF-8"); truncating it restored clippy. Same
  issue as recorded in task 2's status - it recurs whenever a generated-binary run uses
  /dev/null as an output target.
- The full-width vending_machine model (bit<8> money/owed/change_out) exceeded the
  sandbox's ~1 GB RSS cap ("memory allocation of 12288 bytes failed" - the seccomp limit
  noted in task 2's status). Rescaled to bit<6>/bit<2> per the plan's simplify-and-note
  rule; noted in the README.

## Other important findings/information
- The SMT-engine property rule (no `next` / bounded_eventually in specs that will be
  smt-bounded-checked) generalises correction 1A's authoring guidance: input_is /
  output_has / plain state+var invariants are safe under both engines; temporal forms
  are exact-MC-only unless the spec also carries the assumptions that make them hold.
- Witness-var properties (align_done in B1, confirmed in B6, heater_on/compressor_on in
  B5, owed/change_out in B7) remain the established technique for ordering and
  arithmetic invariants that formula patterns cannot express (patterns do not bind
  variables in Phase 1).
- The 8 new legs reuse the existing run_script/repo_root/project_local_rustc_exists
  helpers - no new test infrastructure, no clippy warnings.
