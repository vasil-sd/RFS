# Phase 18, Task 3B - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3B.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (example specs + captures + READMEs only; no `src/` change).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/traffic_light_ctrl/README.md` | changed | the in-process MC section now describes the restored `next`-form property `emergency_returns_to_nsgreen`; the "what to pay attention to" note retires the "no next under smt-bounded" workaround for this spec and explains the weak-at-trace-death semantics (the bounded-CYCLE liveness claim still needs an assumption - deferred to 3C) |
| `crates/examples/vending_machine/README.md` | changed | same treatment for `jam_resets_to_idle`; notes that a `bounded_eventually[N](state == Idle)` return claim is deliberately NOT stated (the machine can sit in Jam/OutOfStock until a Reset - needs an assumption, 3C's territory) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/traffic_light_ctrl/pass.rfs` | changed | added the `next`-form property `emergency_returns_to_nsgreen : always (input_is(Emergency) -> next (state == NSGreen))` - the D1 scenario: Flasher's only exit is Reset->NSGreen, so the sample after an Emergency is NSGreen (any other input terminates the path, making the next-obligation weak) |
| `crates/examples/vending_machine/pass.rfs` | changed | added the `next`-form property `jam_resets_to_idle : always (input_is(Jam) -> next (state == Idle))` - the same D1 scenario on the Jam fault state |
| `crates/examples/traffic_light_ctrl/captures/smt-pass.txt` | regenerated | byte-identical verdict (PASS WITHIN BOUND 16) - the new property is checked and passes; no diff |
| `crates/examples/vending_machine/captures/smt-pass.txt` | regenerated | byte-identical verdict (PASS WITHIN BOUND 16) |

## Changed functionality

None - no command behaviour changed; two example specs gain a `next`-form property each.

## Removed functionality

None.

## New functionality

- Both smt-bounded-demonstrating control-FSM examples now state a genuine `next`-form step
  semantic, checked under BOTH exact MC (PASS complete) and smt-bounded (PASS WITHIN BOUND
  16). These are exactly the property forms that task 3's status forced onto "pure
  invariants / input-anchored implications only" because they false-failed under
  smt-bounded before task 3A - restoring them proves 3A's liveness gate on real specs.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` (B6 + B7 legs) | unchanged | the pinned excerpts (state counts 126/6465, `PASS WITHIN BOUND 16`, the fail.rfs CE steps) are all unchanged - adding a property does not add states or alter the verdict, so no re-pinning was needed |

## Tests run status

| Name | Status |
|---|---|
| `rfs mc-ref` (exact) on both updated specs | OK - B6 PASS complete (states=126), B7 PASS complete (states=6465); state counts unchanged by the added properties |
| `rfs mc-ref --mode smt-bounded --bound 16` on both | OK - PASS WITHIN BOUND 16 (the new next-form property is checked and passes) |
| `rfs validate` on both | OK - valid (the shared-event-pattern-with-different-guards warnings are pre-existing, from the AllRed/CoinInserted guarded transition pairs - untouched by this task) |
| smt captures | byte-identical to committed (no git diff) - the verdict did not move |

## Found problems/issues/mismatches
- None new. The example test legs' `run.sh` scripts do `rm -rf captures` then regenerate;
  running them in this sandbox leaves some capture files missing (incomplete regeneration),
  so they were NOT re-run for this task - the pins are unchanged and verified manually
  instead (exact MC + smt-bounded both pass, captures byte-identical).

## Other important findings/information
- The chosen `next` properties are deliberate D1 demonstrations: each anchors on entering a
  fault state (Flasher / Jam) whose ONLY exit is the recovery transition (Reset->NSGreen /
  Reset->Idle), so the next-state body EXCLUDES the fault state. Pre-3A, an intervening
  unhandled input froze the fault-state config and the old engine checked the body against
  it -> false FAIL. Post-3A the frozen successor has alive=false, so the next-obligation is
  weak/vacuous -> PASS. The body excluding the frozen state is what makes these true D1
  reproductions (a body that included the fault state would hold in the frozen config and
  not demonstrate the fix).
- A top-level `bounded_eventually[N]` was NOT added to either spec: B6's bounded-cycle claim
  needs an assumption (TimerDone keeps firing) and B7's return-to-Idle claim needs one too
  (Reset eventually arrives) - both are assumption-dependent liveness, which is task 3C's
  territory. Stating them now would fail exact MC (a stalled trace never satisfies them).
