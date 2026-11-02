# RFS Monitor Compiler (Class A)

The language-agnostic PSL-to-monitor compiler: turns property/assumption
formulas into a `MonitorPlan` that every backend renders with identical
semantics.

<a id="dev:monitor-compiler-overview"></a>
## Overview

The monitor compiler (`src/monitor.rs`) is the Phase 3 shared plan for
property/assumption monitoring: at codegen time, each in-subset `prop` /
`assume` formula is compiled by `src/monitor.rs::compile_formula` into a
`src/monitor.rs::MonitorPlan` - a tree of monitor nodes that every backend
renders with identical semantics (backend parity principle). The plan is the
single source of truth: Rust and C codegen render the same tree, so both
backends step monitors identically on the same input sequence. Compilation
runs at codegen time and the resulting plan is baked into EVERY generated
artifact - production FSM (debug-gated monitors, D-P3-6), MC engine state
layout (dev:mc-engine-plan), test harness - so all of them agree by
construction.

Semantics are pinned by D-P3-3 ([DESIGN.md](../../DESIGN.md) section 21): samples carry post-step
values; sample 0 is the initial configuration (no input/outputs, so
`input_is`/`output_has` are false there); an obligation triggered at sample i
requires its body to hold at some sample j with i <= j <= i + N - 1 (N >= 1),
and the violation is CONFIRMED at the deadline sample i + N - 1 (the countdown
reaches 0) - not one sample past.

Monitor-compilability restrictions R1-R4 are enforced by
`src/monitor.rs::compile_formula`: formulas outside the subset still parse and
validate; codegen in every backend emits the identical skip warning
(`property '<name>' is outside the monitor subset; skipped` /
`assume '<name>' ...`, e.g. `src/backends/rust_fsm.rs::compile_monitors`) and
generates no monitor for that label. Liveness operators (`eventually` /
`until`, D-P4-8) are outside the subset entirely - MC-only.

Algorithm views L1-L3: dev:algorithms-monitor-compile; function-level call
graph: dev:deps-fn-monitor; backend rendering: [backend_rust.md](backend_rust.md) +
[backend_c.md](backend_c.md); ref_mc's OWN independent monitor compiler (ground
truth, not this plan): dev:mc-engine-exact.

Code: `src/monitor.rs::compile_formula`, `src/monitor.rs::MonitorPlan`,
`src/backends/rust_fsm.rs::compile_monitors`


<a id="dev:monitor-compiler-signals"></a>
## Signal layer + anchor ranges (`src/monitor.rs`)

**Per-sample signals.** `src/monitor.rs::SignalExpr` is a temporal-operator-free
subformula - the per-sample boolean signal every node evaluates. Atoms
(`src/monitor.rs::SignalAtom`) are: a boolean RFS expression over variables and
the post-step state (`VarExpr`), an input pattern just taken (`InputIs`), an
output pattern emitted (`OutputHas`), or the constant `true` (trigger of bare
obligations - renders as a plain boolean literal in every backend); combined
with Not/And/Or. No Eq/Hash derive: the embedded IR types only implement
PartialEq, and structural equality is what backends/tests need.

**compile_signal.** `src/monitor.rs::compile_signal` lowers a temporal-operator-
free subformula to a signal; it returns Option, NOT Result - None when a
temporal operator is present, and callers decide whether that is an error (the
R1-R4 messages) or a different shape. Implies desugars to Or(Not(l), r). The
public wrapper `src/monitor.rs::formula_to_signal` is what the Phase 4 liveness
engine and the test planner use to render obligation bodies / safety leaves as
per-sample evaluators.

**Anchor ranges.** `src/monitor.rs::AnchorRange` = [from, to] with to = None
unbounded: `all()` is the top-level range [0, inf), `contains(s)` tests
membership, and (private) `shifted()` adds one - the effect of `next`. Key
invariant: a bare signal compiles to a RangeCheck over the FULL range - an
invariant, not a one-shot. Sample 0 has no input/outputs, so a one-shot
reading would make I/O atoms vacuously false and assumptions like
`assume a : input_is(Tick)` would prune every trace (pinned by the in-module
test plan_structure_bare_signal_is_invariant).

Code: `src/monitor.rs::SignalExpr`, `src/monitor.rs::SignalAtom`,
`src/monitor.rs::AnchorRange`, `src/monitor.rs::compile_signal`,
`src/monitor.rs::formula_to_signal`


<a id="dev:monitor-compiler-nodes"></a>
## Node tree (`src/monitor.rs`)

**MonNode.** Five node kinds cover the whole monitor subset; every node exposes
a sticky `failed` flag, and the property is violated when the ROOT node fails
(first failing sample wins):

- DelayedCheck { antecedent, consequent, range }: for every anchor i in range,
  antecedent(i) -> consequent(i + 1) - checked at sample s = i + 1 with a
  one-sample delay register holding the antecedent evaluated at s - 1. The
  register starts false, so the first checkable sample is vacuous (D-P3-3).
  Stateless apart from the register; no trigger step.
- RangeCheck { expr, range }: expr must hold at every sample in range.
  Stateless: evaluated against the current sample index on every step;
  violation at the first failing sample inside the range.
- Obligation { bound, trigger, body, range }: for every anchor i in range where
  trigger(i) holds, body must hold at some j with i <= j <= i + bound - 1
  (bound >= 1); for bound == 0 the body must hold at i itself. State is the
  D-P3-3 pending model (pending, remaining, sticky trigger_step); a re-trigger
  while pending is ignored - safe because the pending obligation has the
  earliest deadline and the monitor stops at the first confirmed violation.
- And(children): fails when ANY child has failed.
- Or(children): fails when ALL children have failed (each child's failure is
  witnessed at a definite sample, so the conjunction of failures is confirmed
  when the last child fails).

**MonitorPlan.** `src/monitor.rs::MonitorPlan { label, is_assumption, root }` - one
plan per property or assumption. The MC engine's configuration encoding walks
the same tree in preorder (node ids matching the backends' n{id} naming; only
nodes with persistent state occupy bits): dev:mc-engine-plan.

Code: `src/monitor.rs::MonNode`, `src/monitor.rs::MonitorPlan`


<a id="dev:monitor-compiler-compile"></a>
## Compilation + R1-R4 gate (`src/monitor.rs`)

**Entry.** `src/monitor.rs::compile_formula`(label, is_assumption, f) wraps one
recursive pass - `src/monitor.rs::compile_check`(f, range, in_temporal) - over
the formula structure with two threading parameters: the anchor range (the
samples at which the check applies) and in_temporal (whether an Always / Next
/ BoundedAlways ancestor exists - affects self-anchoring of bounded
constructs, R2/R3). Err(msg) means the formula is outside the monitor subset:
no monitor is generated and codegen warns with identical text in every
backend.

**compile_check dispatch.** Per arm (anchor = the threaded range):

- bounded_eventually[N] b: body must be temporal-operator-free (R2); trigger =
  constant true; anchor = the threaded range when in_temporal, else a ONE-SHOT
  [0, 0] - a top-level bare obligation is pending from sample 0.
- l -> bounded_eventually[N] b: the implication antecedent is the obligation
  trigger (R2); l must be temporal-operator-free; same one-shot-vs-threaded
  anchoring.
- l -> next q: DelayedCheck over the SHIFTED range (q at i + 1 when l at i).
- l -> r (both signal-level): per-sample check !l || r as a RangeCheck.
- always psi: recurse with in_temporal = true (range unchanged).
- always[N] psi: R3 - rejected under any temporal ancestor; N >= 1 required
  (D-P3-2); recurses over the window [0, N - 1].
- next psi: R4 - body must be temporal-operator-free; RangeCheck over the
  range shifted by +1.
- and / or: And/Or node trees over the same range.
- !psi: R1 - inner must be temporal-operator-free (negation of a temporal
  formula would be unbounded liveness); RangeCheck of Not(signal).
- eventually / until: always Err - MC-only (D-P4-8/D-P4-9).
- bare signal: RangeCheck over the full range.

**Diagnostics.** The rejection messages are shared compiler output (identical
in every backend by construction): "bounded_eventually body must be
temporal-operator-free", "implication antecedent must be temporal-operator-
free", "next body must be temporal-operator-free", "implication consequent
must be temporal-operator-free or bounded_eventually", "always[N] is not
allowed under a temporal operator", "always[N] requires N >= 1", "next
requires a temporal-operator-free body", "negation of a temporal formula is
outside the monitor subset", "liveness operators (eventually/until) are
outside the monitor subset", "formula is outside the monitor subset".

Code: `src/monitor.rs::compile_formula`, `src/monitor.rs::compile_check`,
`src/monitor.rs::true_signal`


<a id="dev:monitor-compiler-reference"></a>
## Reference interpreter + test pins (`src/monitor.rs`)

**MonState.** The reference interpreter (tests only) mirrors what backends
generate, node for node: `src/monitor.rs::MonState` carries DelayedCheck {
prev_antecedent, failed }, RangeCheck { failed }, Obligation { bound, pending,
remaining, failed, trigger_step }, And/Or { children, failed, trigger_step }.
`src/monitor.rs::initial_state` builds the state tree before sample 0 (delay
register false; obligation not pending).

**Stepping.** `src/monitor.rs::ReferenceMonitor::step`(s, eval) steps one sample
and returns true when the failure is NEWLY confirmed here (root failed && !
before). `src/monitor.rs::step_node` returns the node's CURRENT-SAMPLE failure
status ("now failing"): a per-sample check fails while its expression holds
false; an obligation fails only once its deadline has been confirmed
(terminal); And/Or combine their children's current statuses. The sticky
failed flag is set when now is true and stays set - it is what the root
reports as the property violation. Using per-sample (NOT sticky) child status
for Or/And composition is essential: disjuncts may fail at different samples
and recover, so a disjunction fails only while ALL of its children fail at the
SAME sample. `src/monitor.rs::min_trigger_at` picks the earliest trigger step
among the currently failing children (-1 when none carries one).

**Test pins.** The in-module suite pins plan structure (R1-R4 rejections,
liveness-operator rejection, anchor shapes: bare-signal full range, always[N]
window [0, N-1], next shift +1, obligation one-shot vs recurring anchors)
and D-P3-3 semantics (deadline confirmation exactly at i + N - 1 with the
trigger step reported; same-step and deadline-sample satisfaction; one sample
past the deadline is a violation; periodic obligations under always; be[0]
same-step rule; next vacuous at sample 0; per-sample Or/And recovery -
children failing at different samples never violate; delay-register pairing
under bounded windows).

**Consumers.** Backends render the plan into generated monitor code (Rust:
render_monitor / render_node in [backend_rust.md](backend_rust.md) - nodes numbered DFS
preorder, step body evaluated post-order; C: the mirror in
[backend_c.md](backend_c.md)); the MC engine's configuration encoding walks the same
tree (dev:mc-engine-plan); the test planner builds monitor-violation cases
from it ([codegen_core.md](codegen_core.md)). ref_mc carries its OWN independent monitor
compiler as ground truth - same documented semantics, different implementation
(dev:mc-engine-exact).

Code: `src/monitor.rs::MonState`, `src/monitor.rs::initial_state`,
`src/monitor.rs::ReferenceMonitor`, `src/monitor.rs::step_node`,
`src/monitor.rs::min_trigger_at`
