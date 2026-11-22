# Phase 18, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Create the 8 Control FSMs B1-B8 (power / energy / building / consumer) example projects (`blc_motor_foc`, `bms_protection`, `solar_inverter`, `wind_turbine_ctrl`, `hvac_thermostat`, `traffic_light_ctrl`, `vending_machine`, `washing_machine`) - each a real FSM spec that
model-checks to PASS, a one-defect variant that fails with a PINNED counterexample,
the fix, a run script, pinned captures, and a template README - plus one test leg per
FSM. Rationale: this is the B-group slice of the Phase 18 real-world-FSM catalogue;
together the six batch tasks cover all 50 machines.

## Context
- All examples are PLAIN DIRS under `crates/examples/` (NOT workspace members - that is
  task 7's job for the three promoted FSMs). Each dir carries: `pass.rfs`, `fail.rfs`,
  `fixed.rfs`, `run.sh`, `captures/` (pinned text + JSON CE/diagnostic), `README.md`.
- README template (BINDING, from PHASE_18.md "Shared example README template"): What it
  demonstrates / The machine (mermaid `stateDiagram-v2` ALWAYS - the subset rfs generates,
  guaranteed to parse under docs_md_test) / RFS modes demonstrated (ONE subsection per mode
  with description + rationale) / Files layout / Build-run-test / Expected results (incl. the
  negative CE walkthrough + fix proof). Relative links to user docs are `../../../docs/user/...`.
- Test-leg conventions (from the existing `tests/example_projects_test.rs`): `repo_root()` /
  `rfs_bin()` helpers, availability gating via `tests/common/toolchain.rs` (skip-with-note,
  never fail), scratch dirs under `target/`, pinned output excerpts asserted with `contains`.
  Compile generated artifacts from the example dir with BARE filenames (the generated MC
  artifacts reference their fsm by codegen-time name).
- Inventory rule (PROTOCOL.md section 5): every new test fn gets a TEST_STRUCTURE.md row in
  the SAME commit (match the existing row format exactly; use `rfs-dev inventory add --doc
  TEST ...` if available).
- Spec-authoring guidance: model each machine at its DISCRETE level - bit-vector vars for
  sequence numbers / timers / windows / counters, guarded transitions, emitted outputs. Use
  `assume` for environment contracts ("peer acknowledges", "network delivers", "responder
  replies") and `bounded_eventually[N]` for liveness properties. Keep the state count in the
  tens-to-low-thousands so exact MC is fast (cf. http2_stream ~434, sliding_window 94). The
  `fail.rfs` defect must produce a DETERMINISTIC, pin-able counterexample (a specific
  reason/label/step + CE steps) - avoid defects that only surface under fuzz. If a real
  protocol is too large to model cleanly within the state budget, SIMPLIFY it (fewer states /
  coarser abstraction) and note the simplification in its README - do not drop the example.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | Cargo-member production-integration subset | needs the `bms_protection` spec from this task |
| 8. | EXAMPLES.md "Other examples" section | needs this task's READMEs to link |
| 9. | Training-set update | references these examples |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | 8 new test legs (one per FSM) |
| `docs/llm/TEST_STRUCTURE.md` | change | 8 new rows (same-commit inventory rule) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
`crates/examples/blc_motor_foc/`, `crates/examples/bms_protection/`, `crates/examples/solar_inverter/`, `crates/examples/wind_turbine_ctrl/`, `crates/examples/hvac_thermostat/`, `crates/examples/traffic_light_ctrl/`, `crates/examples/vending_machine/`, `crates/examples/washing_machine/` | new | pass.rfs + fail.rfs + fixed.rfs + run.sh + captures/ + README.md each |

## Planned actions

### 3.0 - The per-FSM workflow (applies to EVERY FSM in this task)
For each FSM below, do these six steps in order:
1. Write `pass.rfs` - a COMPLETE, VALID module modelling the real machine at its discrete
   level. Run `rfs mc-ref pass.rfs` and confirm PASS complete BEFORE pinning anything.
2. Write `fail.rfs` - introduce the ONE specific defect listed for that FSM. Run mc-ref,
   capture the REAL text FAIL report + counterexample AND the JSON form
   (`--counterexample-format json`) into `captures/`.
3. Write `fixed.rfs` - the one-line correction. Re-run mc-ref and confirm PASS complete.
4. Write `run.sh` - drives the demonstrated RFS modes: `rfs validate`, `rfs info`,
   `rfs mc-ref` (exact; plus fuzz / smt-bounded where noted for that FSM),
   `rfs codegen rust mc` + compile + run, `rfs codegen c mc` + compile + run (both must
   agree with mc-ref), `rfs codegen rust production` + compile, `rfs codegen c production`
   + compile (warning-free).
5. Write `README.md` per the PHASE_18.md shared template - one RFS-mode subsection per mode
   demonstrated, the mermaid stateDiagram, and the negative CE walkthrough + fix proof.
6. Add a test leg (see 3.9): assert positive PASS, the negative PINNED CE excerpt
   (copied from the real capture), the fix PASS, and both-backends production compile.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/<fsm>/pass.rfs` | new | the valid spec that model-checks to PASS |
| `crates/examples/<fsm>/fail.rfs` | new | the one-defect variant (pinned CE) |
| `crates/examples/<fsm>/fixed.rfs` | new | the corrected spec that re-runs to PASS |
| `crates/examples/<fsm>/run.sh` | new | drives all demonstrated RFS modes |
| `crates/examples/<fsm>/captures/` | new | pinned text + JSON CE/diagnostic excerpts |
| `crates/examples/<fsm>/README.md` | new | template-conformant walkthrough |

### 3.1 - `blc_motor_foc`
BLDC FOC MODE machine: standstill->precharge->alignment->running->braking->fault; angle estimate, current magnitude, fault flags (analog loops abstracted to threshold guards).
- `fail.rfs` defect: Clear the overcurrent fault flag immediately instead of latching it - pinned property-violation CE on 'safe-stop latches'.
- `fixed.rfs`: Make the fault latch until an explicit reset.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B1.

### 3.2 - `bms_protection`
Battery pack manager: idle->charging->full->discharging->balancing->protection/fault; SOC, per-cell voltage, pack temperature.
- `fail.rfs` defect: Disable the over-voltage guard so charging continues past Vmax - pinned property-violation CE on 'never above Vmax'.
- `fixed.rfs`: Re-enable the over-voltage guard.
Extra RFS mode: Also becomes a cargo-member crate in task 7 (property monitors).
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B2.

### 3.3 - `solar_inverter`
Grid-tied inverter: off->standby->MPPT->grid-tied->islanded->fault; anti-islanding on grid loss; grid V/f, DC bus, MPPT point.
- `fail.rfs` defect: Remove the anti-islanding trip so the inverter keeps feeding a de-energized grid - pinned property-violation CE on 'no backfeed'.
- `fixed.rfs`: Re-add the anti-islanding trip + relay open.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B3.

### 3.4 - `wind_turbine_ctrl`
Turbine controller: parked->starting->ramp-up->rated->pitch-control->cut-out->emergency-stop; rotor speed, pitch angle, power.
- `fail.rfs` defect: Widen the pitch-response guard so the rotor exceeds rated RPM - pinned property-violation CE on 'never above rated RPM'.
- `fixed.rfs`: Tighten the pitch-response guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B4.

### 3.5 - `hvac_thermostat`
Thermostat: idle->heating->cooling->fan-only->defrost->fault; hysteresis bands; room temp, setpoint, mode.
- `fail.rfs` defect: Allow the heating and cooling valves to open together by removing the mode-exclusion guard - pinned property-violation CE on 'no simultaneous heat+cool'.
- `fixed.rfs`: Re-add the heat/cool mutual exclusion.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B5.

### 3.6 - `traffic_light_ctrl`
Signal controller: per-phase green->yellow->red + all-red clearance; pedestrian request, emergency preemption, flasher fault; phase index, timer, requests.
- `fail.rfs` defect: Remove the all-red clearance so two diagonal greens overlap on a phase change - pinned property-violation CE on 'no conflicting greens'.
- `fixed.rfs`: Re-insert the all-red interval before every phase change.
Extra RFS mode: Also demonstrate `--mode smt-bounded` (small spec).
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B6.

### 3.7 - `vending_machine`
Vending machine: idle->coin->product-selected->dispensing->change-due; out-of-stock, jam; money-in, price, stock counts.
- `fail.rfs` defect: Dispense on partial payment by removing the price-meets-money guard - pinned property-violation CE on 'no product without full payment'.
- `fixed.rfs`: Re-add the full-payment guard.
Extra RFS mode: Also demonstrate `--mode smt-bounded` (small spec).
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B7.

### 3.8 - `washing_machine`
Washer: door-locked->filling->agitating->draining->spinning->rinsing->done; water level, spin speed (ramped), door state.
- `fail.rfs` defect: Unlock the door while water remains by removing the drained-check from the unlock guard - pinned property-violation CE on 'door unlocks only when stopped and drained'.
- `fixed.rfs`: Re-add the drained+stopped check to the unlock guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B8.


### 3.9 - Test legs + inventory rows
Add one test fn per FSM to `tests/example_projects_test.rs`: run the real commands via
`rfs_bin()`, assert exit 0 on `pass.rfs` (PASS complete), assert exit 1 on `fail.rfs` WITH a
PINNED excerpt of the failure output (the CE reason/label/step line or diagnostic message -
copied from the real capture, never hand-written), assert exit 0 on `fixed.rfs`, and assert
both-backends production codegen compiles warning-free. The MC + production legs are
availability-gated on rustc/gcc like the existing c_sync_handler leg. Append a
TEST_STRUCTURE.md row per new fn in the same commit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change | 8 new test fns - pin the failures so a regression in CE format or codegen is caught |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- 8 example projects under `crates/examples/` (pass/fail/fixed specs + run script + pinned
  captures + template README each).
- 8 pinned test legs + 8 TEST_STRUCTURE inventory rows.

## Task exit criteria
- [ ] Every `pass.rfs` model-checks to PASS complete under `rfs mc-ref` exact (verified before pinning).
- [ ] Every `fail.rfs` produces the pinned counterexample/diagnostic and every `fixed.rfs` re-runs to PASS - both asserted by test legs with excerpts copied from REAL captures (no hand-written approximations).
- [ ] Every spec compiles warning-free in BOTH backends (`rfs codegen rust production` + `rfs codegen c production`) - asserted by test legs.
- [ ] Every README follows the PHASE_18.md shared template (all sections; mermaid stateDiagram present; one RFS-mode subsection per demonstrated mode with description + rationale).
- [ ] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [ ] `cargo test --test example_projects_test` green; `cargo test --test docs_md_test` green (the new READMEs are in its file set).
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No changes to `examples/valid/` or `examples/invalid/` (the corpus stays as is).
- No new workspace members (task 7 promotes the three chosen FSMs).
- No training-set changes (task 9) and no EXAMPLES.md changes (task 8).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| B1 ordering property via witness var | The planned "alignment completes before running" ordering was first written as `always (state == Running -> input_is(AlignDone))`, which is too strong: the Running Tick self-loop also targets Running, so the property fails on a healthy trace. Re-formulated with the established witness-var technique: `align_done` is set on the single edge into Running and the property is `always (state == Running -> align_done)` - still pins that no shortcut can enter the running mode before alignment completes. |
| B5 setpoint range guard | The band-edge arithmetic (`setpoint - 10`) wraps in bit<8> for setpoints < 10, letting the machine chase a bogus band and violating the hysteresis invariant. Added a payload range guard on the Setpoint event (`where p >= 20 && p <= 240`, as a real thermostat rejects out-of-band setpoints) - keeps the arithmetic wraparound-free; noted in the README. |
| B6 property set: no temporal forms | The planned bounded-liveness cycle property failed under exact MC (a stalled trace never sends TimerDone - liveness needs an assumption, which smt-bounded rejects for this demo spec), and its `next`-form replacement was "violated" by the SMT engine because smt-bounded models unhandled inputs as self-loops while exact MC kills the path. Replaced with pure invariants / input-anchored implications only (`no_conflicting_greens`, `allred_is_the_only_clearance`, `preemption_safe` - no `next`, no bounded_eventually), robust under both engines' semantics; the all-red insertion is pinned jointly by the first two, and the bounded cycle time is stated by construction (fixed 8/3/2 phase lengths) in the README. |
| B7 bit-width rescale | The full-width model (bit<8> money/owed/change_out) exceeded the sandbox's ~1 GB RSS cap under exact MC ("memory allocation of 12288 bytes failed"). Rescaled to bit<6> money/price/owed/change_out and bit<2> stock per the plan's simplify-and-note rule; noted in the README. |
| B8 deadlock + defect shadowing fixes | Two authoring corrections: (1) the Done state initially had no outgoing transitions, which exact MC reports as reason=deadlock (exit 1) - added `Done on Reset { next DoorLocked }`; (2) the first fail.rfs draft kept the guarded drain self-loop alongside the unguarded unlock, and the drain shadowed the unlock by source order whenever water > 0, so the variant PASSED - the defect removes the sibling transition as well (the overlapping-guard pitfall recorded in task 2's coap finding). |

### Changes in design decisions
- None (no comparison tables were altered; the corrections above are property-formulation and modelling-scope adjustments within the planned actions).
