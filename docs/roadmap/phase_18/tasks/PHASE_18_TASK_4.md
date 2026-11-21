# Phase 18, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Create the 7 Control FSMs B9-B15 (automotive / financial / aerospace / machining / safety) example projects (`vehicle_power_modes`, `atm_transaction`, `uav_flight_modes`, `cnc_tool_change`, `printer_thermal`, `plc_conveyor_seq`, `watchdog_reset`) - each a real FSM spec that
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
| 8. | EXAMPLES.md "Other examples" section | needs this task's READMEs to link |
| 9. | Training-set update | references these examples |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | 7 new test legs (one per FSM) |
| `docs/llm/TEST_STRUCTURE.md` | change | 7 new rows (same-commit inventory rule) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
`crates/examples/vehicle_power_modes/`, `crates/examples/atm_transaction/`, `crates/examples/uav_flight_modes/`, `crates/examples/cnc_tool_change/`, `crates/examples/printer_thermal/`, `crates/examples/plc_conveyor_seq/`, `crates/examples/watchdog_reset/` | new | pass.rfs + fail.rfs + fixed.rfs + run.sh + captures/ + README.md each |

## Planned actions

### 4.0 - The per-FSM workflow (applies to EVERY FSM in this task)
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
6. Add a test leg (see 4.8): assert positive PASS, the negative PINNED CE excerpt
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

### 4.1 - `vehicle_power_modes`
Vehicle power manager: ACCESSORY->ON->RUN->SLEEP->HIBERNATE; wake-source bitmask, battery voltage, rail enables.
- `fail.rfs` defect: Enable the infotainment rail in SLEEP by removing the load gate - pinned property-violation CE on 'no high-load rail in sleep'.
- `fixed.rfs`: Re-add the SLEEP load gate.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B9.

### 4.2 - `atm_transaction`
ATM: idle->card-read->PIN-entry->authorized->dispensing/printing; timeout, jam, lockout; PIN retry count, cash amount, card-present.
- `fail.rfs` defect: Dispense before authorization by removing the auth guard - pinned property-violation CE on 'no cash without authorization'.
- `fixed.rfs`: Re-add the authorization guard on dispense.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B10.

### 4.3 - `uav_flight_modes`
Drone: disarmed->armed-idle->takeoff->hover->mission->RTL->landing->failsafe; GPS sat count, battery level, geofence distance.
- `fail.rfs` defect: Allow arming with fewer than the required GPS sats by lowering the arm guard - pinned property-violation CE on 'no arming without GPS ready'.
- `fixed.rfs`: Restore the GPS-sat arm threshold.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B11.

### 4.4 - `cnc_tool_change`
Machining center: home->move-to->engage->operate/cut->retract->return; M06 tool change; tool-in-spindle index, axis positions.
- `fail.rfs` defect: Engage a second tool while one is in the spindle by removing the single-tool guard - pinned property-violation CE on 'exactly one tool engaged'.
- `fixed.rfs`: Re-add the single-tool guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B12.

### 4.5 - `printer_thermal`
3D printer: idle->heating->leveling->printing->paused->cooling->done; thermal-runaway fault; nozzle/bed temps, target temps, deviation.
- `fail.rfs` defect: Ignore a rising-temperature deviation by removing the runaway guard - pinned property-violation CE on 'thermal runaway aborts'.
- `fixed.rfs`: Re-add the thermal-runaway abort guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B13.

### 4.6 - `plc_conveyor_seq`
Multi-station line: per station enable->load-detect->process->unload; occupancy bits, upstream-clear flags, interlocks.
- `fail.rfs` defect: Let a station process into an occupied downstream by removing the clear-check - pinned property-violation CE on 'no process into occupied downstream'.
- `fixed.rfs`: Re-add the downstream-clear check.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B14.

### 4.7 - `watchdog_reset`
Watchdog: running->window-violation->timeout-reset; brown-out path; feed timestamp, window bounds, supply voltage.
- `fail.rfs` defect: Accept an out-of-window feed by widening the window guard - pinned property-violation CE on 'no spurious reset / window enforced'.
- `fixed.rfs`: Restore the valid-feed window.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group B, section B15.


### 4.8 - Test legs + inventory rows
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
| `tests/example_projects_test.rs` | change | 7 new test fns - pin the failures so a regression in CE format or codegen is caught |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- 7 example projects under `crates/examples/` (pass/fail/fixed specs + run script + pinned
  captures + template README each).
- 7 pinned test legs + 7 TEST_STRUCTURE inventory rows.

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
