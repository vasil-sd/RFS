# Phase 18, Task 3A - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Fix the two SOUNDNESS-relevant divergences between smt-bounded mode and exact mode -
**D1** (unhandled inputs become frozen self-loops, so `next`/shifted property bodies are
checked against a non-executable artifact and can produce FALSE FAILs) and **D2** (a
top-level `bounded_eventually[N]` whose deadline lies past the point where the path died
is still evaluated over possibly-frozen configurations and can produce a FALSE FAIL).
Both fixes make smt-bounded's temporal obligations WEAK at trace death, exactly matching
exact mode (SEMANTICS §8.3 "path terminates on an unexpected input; temporal obligations
are weak at trace end" and §5.5 "no confirmation if the deadline sample does not exist").
Rationale: until these are fixed, any spec that is demoed under smt-bounded must avoid
`next` and `bounded_eventually` (the authoring rule recorded in task 3's status), which
misrepresents the mode's capability and blocks natural property writing.

## Context

### The shared liveness signal already exists
All three unrolling engines compute, per step i, an `alive[i]` boolean:
`alive[0] = true`, `alive[i+1] = alive[i] AND any_fire(i)`, where `any_fire(i)` is the
disjunction of the per-transition first-match `fire_t(i)` predicates. When NO transition
fires at step i (an unhandled input - one whose class/guard matches no enabled
transition), `any_fire(i)` is false, so `alive[i+1]` is false and the next-state / next-var
ITE chains fall through to their frozen default (`s_{i+1} = s_i`, `v_{i+1,j} = v_{i,j}`) -
a self-loop. This frozen tail is what exact mode would instead TERMINATE.
- In-process: `src/mc/ref_mc_smt.rs::run_unrolled` builds `alive: Vec<Bool>` (the
  `alive.push(b_and(&alive[i], &any_fire))` line) and the frozen fallback is the
  `any_fire.ite(&chain, &s_vars[i])` / `any_fire.ite(&a, &v_vars[i][j])` terms.
- Generated Rust runtime: `src/codegen/backends/rust/mc.rs` `run_smt()` - the same
  `alive` vector and `any_fire.ite(...)` frozen fallback.
- Generated C runtime: `src/codegen/backends/c/mc.rs` - `Z3_ast *alive = malloc(k * ...)`
  with `alive[i+1] = Z3_mk_and(c, alive[i], any_fire)` and the `Z3_mk_ite(c, any_fire,
  chain, s_vars[i])` frozen fallback.

### D1 mechanics (the `next` false FAIL)
The property query loop for `SmtPropKind::Always` gates each ANCHOR by `alive[i]`
(`bad = alive[i] AND NOT phi(i)`), which is correct for a plain invariant. But a formula
containing `Sf::Next(inner)` evaluates the body at step i+1 WITHOUT consulting
`alive[i+1]`. Concretely:
- In-process: `src/mc/ref_mc_smt.rs::sf_to_z3`, arm `Sf::Next(inner) =>
  sf_to_z3(inner, spec, ctx, step + 1)` - no liveness gate. The `SfCtx` struct carries
  `s_vars/v_vars/fld_vars/ii_steps/oh_steps/k` but NOT `alive`.
- Generated Rust: the emitted property fns (`expr_p{p}`) and full-class expansions
  (`expand_p{p}`) have signature `(i: usize, k: usize, ii_steps: &[Vec<Bool>],
  oh_steps: &[Vec<Bool>]) -> Bool` - no `alive` parameter. The renderer
  `src/mc/smt_emit.rs::render_sf_rust`, arm `Sf::Next(inner) => render_sf_rust(inner,
  spec, &shifted(step), r)`, emits the body at step+1 with no gate.
- Generated C: `src/mc/smt_emit.rs::render_sf_c`, arm `Sf::Next(inner)` - same ungated
  shift; the emitted C expr/expand fns take `(c, i, k, s0, v0, fld0, ii_flat, oh_flat)`
  with no `alive` array.

So for `always (input_is(E) -> next (f))`: at an anchor i where a transition consumed E
(`alive[i]` true), the body `f` is checked at config i+1. If the step-i input was
unhandled, config i+1 is a FROZEN copy of config i and `alive[i+1]` is false - yet `f`
is still evaluated there. If `f` does not hold in that frozen state, the engine reports a
violation that exact mode would never report (exact mode terminated the path at the
unhandled input, so `next(f)` is vacuously satisfied). This is precisely the B6 failure
recorded in task 3's status: "its `next`-form replacement still tripped the SMT engine on
an intervening unhandled Reset."

### D2 mechanics (the `bounded_eventually` false FAIL + a parity gap)
A top-level `bounded_eventually[N](f)` is one-shot from sample 0: violated iff `f` holds
at no step in `0..N`. The correct WEAK semantics (exact mode §5.5): if the path dies
before step N, the deadline sample does not exist, so there is NO violation claim.
- In-process (`src/mc/ref_mc_smt.rs`, `SmtPropKind::BoundedEventually(n)` arm): computes
  `sat = OR over j in 0..hi of (alive[j] AND f(j))` and asserts `NOT sat`. If the path
dies at d < N, every disjunct with j >= d is false (alive[j] false), so `sat` ranges only
over j < d; if `f` never held in 0..d, `NOT sat` is true -> a FALSE FAIL. The deadline-
aliveness is never required.
- Generated Rust (`src/codegen/backends/rust/mc.rs`, kind-2 arm) and generated C
  (`src/codegen/backends/c/mc.rs`, `PROP_KINDS[p] == 2` arm): the top-level BE is rendered
  by the emitter as a SYNTHETIC `Sf::BoundedEventually` node (see
  `src/mc/smt_emit.rs` - the "wrap it in a synthetic node to get the clipped OR loop"
  block), whose expansion is `OR over j of f(j)` with NO `alive[j]` gating at all, and the
  engine asserts `NOT phi`. So the generated runtimes already DISAGREE with the
  in-process engine on this property (in-process gates each disjunct by alive[j];
  generated does not gate at all) - a latent parity bug that this task must close.

### The fix (unified across all three engines)
- **D1:** thread the existing `alive` array into the property-formula evaluation context
  and render `Sf::Next(inner)` at step i as `NOT alive[i+1] OR inner(i+1)` (equivalently
  `alive[i+1].implies(inner(i+1))`). A next-obligation is then vacuously satisfied when
  the path has died at the successor - weak-at-trace-end, matching exact mode. Because the
  anchor-level clipping already guarantees `i + next_depth < k` (so every shifted step is
  in range), no out-of-bounds access is introduced. The same gate applies to any temporal
  node that shifts a step (`Sf::Next` is the only DEFAULT-class shifter; the FULL-class
  nodes are handled by their own horizon-truncated loops and are out of scope here).
- **D2:** unify all three engines on `violation = alive[hi-1] AND NOT(OR over j in 0..hi
  of f(j))`, where `hi = N` (the property is PENDING - no violation claim - when
  `N > k.saturating_sub(next_depth)`, the existing rule). The single `alive[hi-1]`
  conjunction requires the path to SURVIVE to the deadline before the obligation is
  confirmable. Because `alive` is monotone-decreasing (once false it stays false - the
  frozen self-loop keeps it false), `alive[hi-1]` being true implies every earlier
  `alive[j]` is true, so the disjunction needs NO per-disjunct gating. This makes the
  in-process and generated encodings IDENTICAL (both compute the ungated `OR_j f(j)` then
  AND with `alive[hi-1]`), closing the latent parity gap.

### Why this is sound and breaks no existing PASS pin
The D1 gate only ADDS a vacuous-satisfaction on traces where the path is dead at the
successor step; on every trace that stays alive through that step, behavior is byte-for-
byte unchanged. The D2 change only SUPPRESSES violations on traces that die before the
deadline; on traces alive to the deadline it is unchanged. Hence no spec that currently
PASSES can start failing, and no genuine violation on an alive path is masked. The only
pins that could move are FAIL pins whose counterexample trace contains a frozen (dead)
step at the reported violation point - those were D1/D2 false FAILs and must be re-examined
(see Planned action 3A.5).

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1A. | SMT-bounded property-language extension | introduced the `next`/`bounded_eventually` support and the `alive` unrolling this task gates; its pins must stay byte-identical except where a D1/D2 false FAIL is corrected |
| 3. | Control FSMs B1-B8 | its status recorded the B6 false FAIL that motivates this task; B6/B7 are reworked by the dependent task 3B |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3B. | Rework B6 + B7 specs | restores the `next`/`bounded_eventually` properties that this task makes safe, on the two control-FSM examples that demo smt-bounded |
| 3E. | Documentation overhaul | records the (now-closed) D1/D2 divergences and the residual D5/D6 differences in MODEL_CHECKING.md |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/ref_mc_smt.rs` | change | add `alive: &[Bool]` to `SfCtx`; gate the `Sf::Next` arm by `NOT alive[step+1] OR ...`; change the top-level `BoundedEventually(n)` violation to `alive[hi-1] AND NOT(OR_j f(j))` (drop the per-disjunct alive gate, add the deadline-alive conjunction) |
| `src/mc/smt_emit.rs` | change | both renderers (`render_sf_rust` / `render_sf_c`): emit the D1 liveness gate in the `Sf::Next` arm; give every emitted property/expand fn an `alive` parameter (Rust `&[Bool]`, C `const Z3_ast *`); update the `sf_uses_*` walkers if they must account for the new param; emit the D2 deadline-alive conjunction in the top-level BE check of both runtimes |
| `src/codegen/backends/rust/mc.rs` | change | emitted Rust SMT runtime: pass `alive` into every `expr_p{p}` / `expand_p{p}` call; assert `alive[hi-1] AND NOT phi` for top-level BE (kind 2) |
| `src/codegen/backends/c/mc.rs` | change | emitted C SMT runtime: same - pass the `alive` array into every expr/expand call; assert the D2 deadline-alive conjunction for `PROP_KINDS[p] == 2` |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/SEMANTICS.md` | change | normative: close the §9.9 gap - smt-bounded temporal obligations are weak at trace death (D1) and a top-level bounded_eventually is unconfirmable past path death (D2); state that these now MATCH exact mode |
| `docs/user/MODEL_CHECKING.md` | change | user-facing: the smt-bounded "What it checks" note about `next` anchors gains the trace-death weak rule; the bounded_eventually pending rule gains the path-death clause (task 3E expands this into the full residual-differences section) |
| `KB.md` | change | section 7 (known limitations): remove/annotate the D1/D2 false-FAIL gap now that it is closed |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_smt_test.rs` | change | new negative-control pins: a `next`-form property that false-fails TODAY (unhandled input between anchor and successor) now PASSES; a top-level `bounded_eventually[N]` whose deadline exceeds an early path death now PASSES; a genuine violation on an ALIVE path still FAILS (positive control) |
| `tests/smt_bounded_test.rs` | change | ref-vs-generated byte-parity for every new pin (the D1 gate and the D2 conjunction must render identically in rust + c); re-examine any pre-existing FAIL pin whose CE trace has a frozen step at the violation point |

## Planned actions

### 3A.1 - Gate `next` by liveness in the in-process engine (`src/mc/ref_mc_smt.rs`)
Add `alive: &[Bool]` to the `SfCtx` struct and populate it in `run_unrolled` (the vector
already exists). In `sf_to_z3`, change the `Sf::Next(inner)` arm from
`sf_to_z3(inner, spec, ctx, step + 1)` to
`b_or(&ctx.alive[step + 1].not(), &sf_to_z3(inner, spec, ctx, step + 1))`. No other arm
changes (the FULL-class nodes keep their horizon-truncated loops). Verify `step + 1 < k`
is guaranteed by the existing anchor clipping (`hi = k.min(bound).min(k - next_depth)`).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_smt.rs` | change | `SfCtx` gains `alive`; the `Sf::Next` arm is liveness-gated - weak-at-trace-end for DEFAULT-class next properties |

### 3A.2 - Fix the top-level bounded_eventually violation in the in-process engine (`src/mc/ref_mc_smt.rs`)
In the `SmtPropKind::BoundedEventually(n)` arm, replace the per-disjunct-gated
`sat = OR_j (alive[j] AND f(j))` + `assert(sat.not())` with the unified encoding:
`sat = OR_j f(j)` (ungated) and `assert(alive[hi-1].and(sat.not()))`, keeping the existing
pending rule (`if hi < n { continue; }`). This requires the path to survive to the
deadline before a violation is confirmable.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_smt.rs` | change | top-level BE violation now requires deadline-aliveness - closes D2 and matches the generated encoding |

### 3A.3 - Mirror both fixes in the emitters (`src/mc/smt_emit.rs`)
In `render_sf_rust` and `render_sf_c`, change the `Sf::Next` arm to emit the liveness
gate (Rust: `b_or(&alive[{step}+1].not(), &<inner>)`; C: `Z3_mk_or(c, 2, { Z3_mk_not(c,
alive[{step}+1]), <inner> })`). Thread an `alive` parameter through EVERY emitted property
fn and expansion fn: Rust signature becomes `(i: usize, k: usize, alive: &[Bool],
ii_steps: &[Vec<Bool>], oh_steps: &[Vec<Bool>]) -> Bool`; C becomes
`(Z3_context c, unsigned i, unsigned k, const Z3_ast *alive, Z3_ast s0, const Z3_ast
*v0, const Z3_ast *fld0, const Z3_ast *ii_flat, const Z3_ast *oh_flat)`. Update the
`expand_fns()` / `EXPAND_FNS` table types and the `expand_dummy` accordingly. In the
top-level BE check of BOTH emitted runtimes, assert the deadline-alive conjunction
(`alive[hi-1] AND NOT phi`) instead of bare `NOT phi`. Keep every non-temporal property
fn's BODY byte-identical (only the signature gains the unused `_alive` param when the
body has no `next`), so specs without temporal properties generate near-identical
binaries.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_emit.rs` | change | both renderers emit the D1 gate; all emitted property/expand fns gain the `alive` param; the top-level BE check emits the D2 conjunction - parity by construction |

### 3A.4 - Mirror both fixes in the generated runtimes (`src/codegen/backends/{rust,c}/mc.rs`)
Update the emitted Rust `run_smt()` and C `main()` query sections to pass the `alive`
vector/array into every `expr_p{p}` / `expand_p{p}` / `EXPAND_FNS[p]` call, and to assert
the D2 deadline-alive conjunction for top-level BE. The unrolling loop that builds
`alive` is unchanged (it already exists). These are the emitter's OUTPUT templates - edit
them so newly generated binaries carry the fix; already-generated committed binaries for
existing specs are regenerated by the test legs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/mc.rs` | change | emitted Rust runtime passes `alive` to property fns and asserts the D2 conjunction |
| `src/codegen/backends/c/mc.rs` | change | emitted C runtime: same |

### 3A.5 - Tests + pin re-examination (`tests/ref_mc_smt_test.rs`, `tests/smt_bounded_test.rs`)
Add pinned specs: (a) a `next`-form property on a machine with an unhandled input that
freezes the config between anchor and successor - assert it PASSES (today it false-fails;
this is the B6 reproduction); (b) a top-level `bounded_eventually[N]` where a path dies
before N - assert it PASSES (today it false-fails); (c) POSITIVE controls: a genuine
`next` violation and a genuine `bounded_eventually` violation on ALIVE paths still FAIL
with pinned CEs. Pin every case on ref AND both generated backends with byte-parity
assertions. Then re-examine every pre-existing smt FAIL pin: if its counterexample trace
contains a frozen (dead) step at the reported violation point, it was a D1/D2 false FAIL -
update the pin to the corrected verdict and record the change in this task's corrections
table.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change | in-process negative + positive control pins for D1 and D2 |
| `tests/smt_bounded_test.rs` | change | generated-binary pins + ref-vs-generated parity; re-examined pre-existing FAIL pins |

### 3A.6 - Docs sync
Update `docs/SEMANTICS.md` (normative: D1/D2 closed - smt-bounded temporal obligations are
weak at trace death, matching exact mode §8.3/§5.5), `docs/user/MODEL_CHECKING.md`
(the `next` anchor note + the bounded_eventually pending rule gain the path-death clause),
and `KB.md` section 7 (annotate the closed gap). The full residual-differences section is
task 3E's deliverable - do not build it here.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | normative closure of D1/D2 |
| `docs/user/MODEL_CHECKING.md` | change | user-facing weak-at-trace-death note |
| `KB.md` | change | known-limitations annotation |

## Task deliverables
- smt-bounded mode (ref + both generated backends, parity-pinned) no longer false-fails a
  `next`/shifted property when an unhandled input freezes the successor config (D1 closed).
- A top-level `bounded_eventually[N]` is unconfirmable when the path dies before the
  deadline (D2 closed), with the in-process and generated encodings now IDENTICAL.
- New negative-control pins reproduce the B6 false FAIL and assert it is gone; positive
  controls confirm genuine violations on alive paths are still caught.
- Docs synchronized (SEMANTICS normative + MODEL_CHECKING user note + KB annotation).

## Task exit criteria
- [ ] A `next`-form property with an unhandled input between anchor and successor PASSES in smt-bounded (ref + both backends, parity-pinned) - the B6 false FAIL is gone.
- [ ] A top-level `bounded_eventually[N]` whose deadline exceeds an early path death PASSES (ref + both backends, parity-pinned).
- [ ] Genuine `next` and `bounded_eventually` violations on ALIVE paths still FAIL with pinned CEs (no masking).
- [ ] The in-process and generated top-level BE encodings are byte-identical in effect (the latent parity gap is closed - a parity pin asserts identical verdicts).
- [ ] Every pre-existing smt PASS pin stays byte-identical; any FAIL pin whose CE trace had a frozen step at the violation point is re-pinned and recorded in the corrections table.
- [ ] `cargo test --features smt` green; full checklist `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Docs synchronized (SEMANTICS.md + MODEL_CHECKING.md + KB.md).

## Non-goals
- No assumption support (D3) or on_error support (D4) - those are task 3C.
- No enum-typed var / wildcard-group / enum-pattern support (D8) - that is task 3D.
- No changes to exact-mode or fuzz-mode behavior.
- No rework of the B6/B7 example specs - that is task 3B (depends on this task).
- No building of the MODEL_CHECKING.md residual-differences section - that is task 3E.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The `alive` parameter must be parity-critical in its POSITION within every emitted fn
  signature (fixed order), exactly like the existing `s_{i}`/`v_{i}_{j}`/`ev_{i}`/
  `fld_{i}_{g}`/`ii_steps`/`oh_steps` convention, so rust and c render identically.
- For a property fn whose body contains NO `next`, the emitted param is named `_alive`
  (underscore) to avoid an unused-variable warning, matching the existing `_ii_steps` /
  `_oh_steps` convention for unreferenced atom params.
- The D2 conjunction uses `alive[hi-1]` where `hi = n.min(k.saturating_sub(next_depth))`;
  when `hi == 0` the property is already skipped (pending), so `hi-1` is never negative.
- If profiling shows the liveness-gated `next` expansion measurably slows the solver at
  large bounds, that is a 3D concern (flag-gating expensive encodings) - do NOT gate the
  D1/D2 soundness fixes themselves; they are correctness, not performance.
