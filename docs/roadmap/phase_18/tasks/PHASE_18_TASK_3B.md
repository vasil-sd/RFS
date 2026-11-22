# Phase 18, Task 3B - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Rework the two Group-B control-FSM examples that demo smt-bounded - **B6
`traffic_light_ctrl`** and **B7 `vending_machine`** - to USE the property forms that
task 3A made safe: `next`-form implications (and, where it holds without assumptions,
a top-level `bounded_eventually[N]`). Before 3A these examples were forced onto "pure
invariants / input-anchored implications only" (the authoring rule in task 3's status)
because `next` false-failed on an intervening unhandled input (B6) and the planned
bounded-liveness cycle could not be stated. After 3A, restoring the natural temporal
properties PROVES the fix on real example specs and retires the workaround rule for these
two machines. Rationale: the examples are the user-facing demonstration that smt-bounded
now checks exact-mode-style temporal properties on control machines.

## Context

### B6 `traffic_light_ctrl` - current state
`crates/examples/traffic_light_ctrl/pass.rfs` currently carries three properties, all of
the workaround class (no `next`, no `bounded_eventually`):
- `no_conflicting_greens : always (!(ns_green && ew_green))` - pure invariant.
- `preemption_safe : always (input_is(Emergency) -> state == Flasher && !ns_green &&
  !ew_green)` - input-anchored implication.
- `allred_is_the_only_clearance : always (state == AllRed -> !ns_green && !ew_green)` -
  pure invariant.
The machine has states NSGreen/NSYellow/AllRed/EWGreen/EWYellow/Flasher and inputs
TimerDone/PedRequest/Emergency/Reset. **Flasher handles ONLY Reset** - TimerDone,
PedRequest and Emergency are unhandled there, so under smt-bounded a trace that reaches
Flasher and then receives any of those inputs FREEZES the config (the D1 scenario). The
capture `captures/smt-pass.txt` pins `PASS WITHIN BOUND 16`. The README's smt subsection
explicitly notes the spec "carries no assumptions ... which is why the bounded-cycle claim
is stated by construction (fixed phase lengths) rather than as a liveness property."

### B7 `vending_machine` - current state
`crates/examples/vending_machine/pass.rfs` carries four properties, all output-anchored
implications (no `next`, no `bounded_eventually`):
- `no_dispense_underpaid : always (output_has(Dispense) -> money >= price)`
- `correct_change : always (output_has(ReturnChange) -> change_out == owed)`
- `no_loss_on_refund : always (output_has(Refund) -> money == 0)`
- `no_dispense_out_of_stock : always (output_has(Dispense) -> stock > 0)`
The machine accumulates money in cents, dispenses only on full payment, and refunds
otherwise. Its capture `captures/smt-pass.txt` pins `PASS WITHIN BOUND 16`. (B7's vars
were rescaled to bit<6>/bit<2> for the sandbox RSS cap - keep that rescaling; do not
widen them here.)

### What 3A enables
After task 3A, a `next`-form property is weak at trace death: an unhandled input that
freezes the successor config no longer false-fails the body. So both examples can state
the temporal facts they actually demonstrate - "this input in this state lands in THAT
state next" - which is the natural way to specify a control FSM's step semantics, and
which exact mode already checks (so the new properties must PASS under exact MC too).
A top-level `bounded_eventually[N]` is likewise safe, BUT it only HOLDS without
assumptions if the machine is guaranteed to reach the target within N steps on every
infinite input sequence. B6 is NOT so guaranteed (it can sit in Flasher until Reset, or
stall if TimerDone stops - both need an assumption, which is task 3C's territory), so B6
gains a `next`-form property in this task and defers the bounded-liveness cycle to 3C.
B7 IS guaranteed to return to Idle within a bounded number of steps on any input
sequence (every branch terminates in Idle or a dispensing state that returns to Idle), so
B7 may additionally gain a top-level `bounded_eventually[N]` - verify it holds under exact
MC before pinning, and if it does not hold assumption-free, keep B7 to the `next`-form
property only and note why.

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3A. | Fix D1+D2 | makes `next`/`bounded_eventually` safe under smt-bounded - this task's restored properties false-fail without it |
| 3. | Control FSMs B1-B8 | created the B6/B7 specs, captures, READMEs and test legs this task reworks |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3C. | Implement D3+D4 | once assumptions are supported, B6's bounded-liveness cycle (`always (bounded_eventually[N](state == AllRed))` under an `assume always (input_is(TimerDone))`) can be added and checked |
| 3E. | Documentation overhaul | cites the reworked B6/B7 as worked examples of smt-bounded temporal properties |

### Affected source files (optional)

None - example specs + captures + test legs only; no `src/` change.

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/traffic_light_ctrl/README.md` | change | smt-bounded subsection: the restored `next`-form property is now checked directly; retire the "no next / no bounded_eventually" workaround note for this spec (the bounded-cycle-by-construction caveat stays until 3C adds assumptions) |
| `crates/examples/vending_machine/README.md` | change | smt-bounded subsection: the restored `next`-form property (and the bounded_eventually one if it holds assumption-free) is checked directly; retire the workaround note accordingly |

### Affected example files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/traffic_light_ctrl/pass.rfs` | change | add the `next`-form property (keep the existing three); verify PASS complete under exact MC before pinning |
| `crates/examples/traffic_light_ctrl/captures/smt-pass.txt` | change | regenerated smt-bounded verdict (still PASS WITHIN BOUND 16, now with the extra property checked) |
| `crates/examples/vending_machine/pass.rfs` | change | add the `next`-form property (and bounded_eventually if it holds assumption-free); keep the existing four; verify PASS complete under exact MC |
| `crates/examples/vending_machine/captures/smt-pass.txt` | change | regenerated smt-bounded verdict |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | the B6 and B7 legs re-pin if any asserted excerpt moved (the PASS summary state count is unchanged - adding a property does not add states; the smt verdict line is unchanged in form). No new legs. |

## Planned actions

### 3B.1 - Rework B6 `traffic_light_ctrl`
Add to `pass.rfs` a `next`-form property that states a step semantic the machine actually
has and that exact mode verifies. A natural choice (adjust to the real transition table):
`prop yellow_completes_to_allred : always (input_is(TimerDone) && state == NSYellow &&
timer >= 3 -> next (state == AllRed)) ;` - "a TimerDone that completes the N-S yellow
lands in AllRed." This is EXACTLY the form that false-failed before 3A (a trace that
reaches Flasher and then receives an unhandled input freezes the config, and the old
engine checked the next-body against it). Run `rfs mc-ref` (exact) on the updated spec -
it must report PASS complete. Then run `rfs mc-ref --mode smt-bounded --bound 16` and
regenerate `captures/smt-pass.txt` (expect PASS WITHIN BOUND 16). Update the README's
smt subsection: the `next` property is now checked directly; retire the workaround note
for `next` (keep the bounded-cycle-by-construction caveat - that needs 3C's assumptions).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/traffic_light_ctrl/pass.rfs` | change | the restored `next`-form property |
| `crates/examples/traffic_light_ctrl/captures/smt-pass.txt` | change | regenerated smt verdict |
| `crates/examples/traffic_light_ctrl/README.md` | change | smt subsection reflects the restored property |

### 3B.2 - Rework B7 `vending_machine`
Add to `pass.rfs` a `next`-form property stating a step semantic, e.g.
`prop full_payment_dispenses : always (input_is(DispenseCmd) && state == ProductSelected
&& money >= price -> next (state == Dispensing)) ;` (adjust the state/input names to the
real spec). Verify PASS complete under exact MC. THEN test whether a top-level
`bounded_eventually[N]` holds assumption-free - e.g. `prop returns_to_idle :
bounded_eventually[20] (state == Idle)` - by running exact MC: if it PASSES, add it and
note that B7 demonstrates the safe top-level bounded_eventually; if it FAILS (some input
sequence avoids Idle beyond N), do NOT add it and record why in the README. Regenerate
`captures/smt-pass.txt`. Update the README's smt subsection accordingly.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/vending_machine/pass.rfs` | change | the restored `next`-form property (+ bounded_eventually only if it holds assumption-free) |
| `crates/examples/vending_machine/captures/smt-pass.txt` | change | regenerated smt verdict |
| `crates/examples/vending_machine/README.md` | change | smt subsection reflects the restored property(ies) |

### 3B.3 - Re-pin the test legs (`tests/example_projects_test.rs`)
Run the B6 and B7 legs. Adding properties does not change the exact-MC state count or the
smt verdict line's form, so most pins stay byte-identical; re-pin ONLY any excerpt that
actually moved (e.g. a property-count field if the summary reports one). No new legs,
no inventory rows (no new test fns).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change | re-pinned excerpts for the B6/B7 legs if any moved |

## Task deliverables
- B6 and B7 specs state their step semantics with `next`-form properties (and B7 with a
  top-level `bounded_eventually[N]` if it holds assumption-free), checked by BOTH exact MC
  (PASS complete) and smt-bounded (PASS WITHIN BOUND).
- The captures are regenerated and the READMEs retire the "no next / no bounded_
eventually" workaround rule for these two specs.
- The test legs re-pin cleanly; the full suite stays green.

## Task exit criteria
- [ ] B6 `pass.rfs` carries a `next`-form property; `rfs mc-ref` (exact) reports PASS complete; `--mode smt-bounded --bound 16` reports PASS WITHIN BOUND 16; capture regenerated.
- [ ] B7 `pass.rfs` carries a `next`-form property; exact MC PASS complete; smt-bounded PASS WITHIN BOUND 16; capture regenerated. A top-level `bounded_eventually[N]` is added ONLY if exact MC confirms it holds assumption-free (otherwise the README records why it was not added).
- [ ] Both READMEs' smt subsections reflect the restored properties and retire the workaround note (B6 keeps the bounded-cycle caveat pending 3C).
- [ ] The B6/B7 test legs pass; any moved excerpt is re-pinned; no new legs/inventory rows.
- [ ] Full checklist `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green (the smt legs need the smt-enabled binary per TOOLCHAIN.md section 5).

## Non-goals
- No engine change - that is task 3A (this task only consumes its fix).
- No assumption-based bounded-liveness cycle in B6 - that needs task 3C's assumption support.
- No changes to the other six Group-B examples (B1-B5, B8) - they do not demo smt-bounded.
- No widening of B7's rescaled bit widths (the sandbox RSS cap rule stands).

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The restored properties must be TRUE under exact mode - verify with `rfs mc-ref` before
  pinning, per the phase's spec-authoring guidance. A `next`-form property that exact MC
  rejects is a spec bug, not an engine bug.
- Keep each example's defect variant (`fail.rfs`) and its pinned counterexample UNTOUCHED -
  this task only extends the PASSING spec's property set. The negative-example discipline
  (one realistic defect, pinned CE) is unaffected.
- If B7's candidate `bounded_eventually[N]` holds under exact MC but the required N makes
  the smt-bounded formula large at bound 16, prefer the smallest N that still demonstrates
  the return-to-Idle guarantee and note the choice in the README.
