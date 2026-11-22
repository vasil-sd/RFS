# Phase 18, Task 3C - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Implement the two remaining LANGUAGE-feature divergences in smt-bounded mode -
**D3** (assumptions, currently rejected with a shared error) and **D4** (`on_error`
fallback blocks, currently rejected) - so that smt-bounded can check specs that use
environment contracts and unmatched-input fallbacks, the way exact mode does. The CHEAP
encodings (an `on_error` catch-all transition; an input-guard assumption that constrains
which inputs the environment may send at each step) are enabled BY DEFAULT; the EXPENSIVE
assumption forms (liveness assumptions such as `assume eventually (...)`, and assumptions
with temporal nesting) go behind a NEW `--full-assumptions` CLI flag - DISTINCT from
`--full-properties`, so the expensive/cheap choice can be made INDEPENDENTLY for properties
and assumptions (a single shared flag would only allow expensive+expensive or cheap+cheap,
never the mixed combinations expensive-properties+cheap-assumptions or the reverse). Rationale: assumptions are how a
control spec states "the timer always fires" / "the peer always acknowledges", and
`on_error` is how it states "an unexpected input sends the machine to a safe state" -
both are core to writing realistic specs, and their absence is what forced the Phase 18
twins onto assumption-free rewrites (task 2's nvme note) and blocked B6's bounded-liveness
cycle (task 3B defers it here). Update every v1-scope note that currently declares these
rejected.

## Context

### The IR already carries both constructs
- `Module.assumptions: Vec<Property>` (`src/lang/ir.rs`) - each is a named formula with
  the SAME `Formula` structure as a property (so `assume always (input_is(TimerDone))`
parses and validates exactly like a property). Exact mode compiles each selected
  assumption to a monitor and PRUNES any successor whose input violates it
  (`src/mc/ref_mc_exact.rs` - the `assume_mons` are stepped before properties and a
  violated assumption terminates the path, SEMANTICS §7.x).
- `Module.on_error: Option<OnError>` where `OnError { next_state: String,
  emits: Vec<OutputExpr> }` (`src/lang/ir.rs`) - a fallback executed in ANY state when no
  transition matches, with no pattern bindings (SEMANTICS §3.6). Exact mode takes it as the
  successor for an unmatched input.
- Both are REJECTED today by `src/mc/smt_plan.rs::build_smt_spec` with shared errors:
  `"smt-bounded mode does not support assumptions (spec has N)"` and `"smt-bounded mode
  does not support on_error blocks"`. The module-header v1-scope comment lists them.

### D4 encoding (on_error) - CHEAP, default
The unrolling engines already compute, per step i, the first-match `fire_t(i)` predicates
and `any_fire(i) = OR_t fire_t(i)`, then build next-state/next-var ITE chains with a frozen
self-loop fallback when `any_fire(i)` is false. An `on_error` block is exactly a
catch-all, lowest-priority transition from ANY state: define
`on_error_fire(i) = NOT any_fire_real(i)` (no real transition matched). When the module has
an `on_error`, the next state becomes
`ite(on_error_fire(i), on_error.next_state, <real ITE chain or frozen>)`, the next vars are
unchanged by the fallback (OnError carries no assignments), and the fallback's `emits`
feed the step's `output_has` atoms exactly like a real transition's emits. Crucially,
`alive[i+1] = alive[i] AND (any_fire_real(i) OR on_error_fire(i))` - which is `alive[i]`
(itself true on a live path) because the disjunction is now always true: an unmatched input
takes the fallback instead of killing the path. This MATCHES exact mode §3.6 and, as a
side benefit, removes the frozen-self-loop artifact for specs that HAVE an on_error (the
D1 scenario no longer arises for them - though 3A's liveness gate still protects specs
without one). The encoder needs ONE new concept: a transition whose source is ANY state.
Represent it as a dedicated `SmtOnError { next_state: usize, emits: Vec<SmtEmit> }` field
on `SmtSpec` (NOT a `SmtTrans` with a fake state index), so the three engines and both
emitters handle it uniformly and the real-transition firing order is untouched.

### D3 encoding (assumptions) - cheap default, expensive flag-gated
An assumption is a formula that must hold at every INPUT-CONSUMING step (steps 0..k-1;
step k-1 consumes no input). The encoder converts each SELECTED assumption's formula with
the SAME `convert_formula` path as properties (so `input_is`/`output_has`/expr/guards all
work), storing it on a new `SmtSpec.assumptions: Vec<SmtAssumption>` where
`SmtAssumption { name, formula: Sf, full: bool }`. The unrolling loop then asserts,
for each step i in 0..k-1 and each assumption, the assumption formula evaluated at step i
(`sf_to_z3(&a.formula, spec, &ctx, i)`) as a BASE constraint. This prunes the input space:
only inputs satisfying every assumption are allowed - exactly exact mode's successor
pruning, expressed as per-step SMT constraints.
- **CHEAP form (default):** `assume always (f)` where `f` is an input-constraining body -
  `input_is(...)` patterns and boolean guards over payload fields, NO temporal nesting.
  This renders to a per-step constraint over `ev_i`/`fld_{i}_{g}`/the step's fire
  predicates - O(1) formula growth per step. The top-level `always` is dropped (the
  per-step assertion IS the universal quantification over steps).
- **EXPENSIVE form (--full-assumptions):** `assume eventually (f)`, `assume p until q`,
  or any assumption whose body nests a temporal operator under another. These need the
  full-class expansion machinery (horizon-truncated liveness reasoning) and blow up at
  large bounds - so they are carried in the spec but asserted only when the engine runs
  with `--full-assumptions`; without it, an expensive assumption is OMITTED with a
  per-assumption stderr warning (mirroring the property-omission warning) and the summary
  line gains `assumptions_omitted=M` only when M > 0.
- **Selection:** exact mode honors `--assumes all|none|labels`. smt-bounded must too:
  add `--assumes` to BOTH smt surfaces (it is currently a not-applicable flag for
  `rfs mc-ref --mode smt-bounded` and absent from the generated SMT binaries). Only
  SELECTED assumptions are encoded; `none` encodes zero (the current assumption-free
  behavior, so existing pins stay byte-identical).

### Why this is sound
The on_error catch-all only ADDS a successor (the fallback) where exact mode also takes it
- no new paths, no masked violations. The per-step assumption constraints only REMOVE
inputs that exact mode would prune - again no new paths. Both are exact-mode-faithful.
The expensive-assumption omission is SOUND in the same sense as the property omission:
 dropping an assumption can only ADD paths (weaker environment contract), so a PASS
without it is a valid PASS-within-bound for the relaxed spec, and the warning names what
was dropped. No false FAILs are introduced.

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1A. | SMT-bounded property-language extension | provides the `convert_formula` path, the omission-warning machinery (reused here for the NEW `--full-assumptions` flag - distinct from 1A's `--full-properties`, which stays the property gate), and the per-step fire/atom storage this task reuses for assumptions and on_error emits |
| 3A. | Fix D1+D2 | its liveness-gated unrolling is what on_error composes with (the fallback keeps `alive` true); the two are independent but 3A's pins must stay green after on_error changes the frozen-fallback behavior for on_error specs |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3B. | Rework B6 + B7 | B6's deferred bounded-liveness cycle (`always (bounded_eventually[N](state == AllRed))` under `assume always (input_is(TimerDone))`) becomes checkable once assumptions land - re-visit B6 in a follow-up if the user wants the cycle stated as a real property |
| 3E. | Documentation overhaul | records D3/D4 as now-supported (with the cheap/expensive assumption split) and removes them from the residual-differences list |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/smt_plan.rs` | change | drop the two rejections; add `SmtSpec.on_error: Option<SmtOnError>` and `SmtSpec.assumptions: Vec<SmtAssumption>`; convert selected assumptions via `convert_formula` (cheap vs expensive classification); update the module-header v1-scope comment |
| `src/mc/smt_emit.rs` | change | emit the on_error catch-all into both runtimes' unrolling loops; emit per-step assumption base-constraints; carry the `--assumes` selection and the expensive-assumption omission into the emitted CLI + summary |
| `src/mc/ref_mc_smt.rs` | change | in-process: apply the on_error fallback in the next-state/next-var/alive computation; assert per-step assumption constraints as base facts; honor `--assumes`; emit expensive-assumption omission warnings + `assumptions_omitted=M` |
| `src/codegen/backends/rust/mc.rs` | change | emitted Rust runtime: same on_error + assumption changes; add `--assumes` to its arg parsing |
| `src/codegen/backends/c/mc.rs` | change | emitted C runtime: same; add `--assumes` |
| `src/cli/commands/mc_ref.rs` | change | accept `--assumes all\|none\|labels` under `--mode smt-bounded` (it is currently not-applicable there); pass the selection to `ref_mc::run_smt` |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/SEMANTICS.md` | change | normative: smt-bounded now supports on_error (§3.6-faithful catch-all) and assumptions (§7.x-faithful per-step pruning); the expensive-assumption class is flag-gated with omission warnings |
| `docs/user/MODEL_CHECKING.md` | change | the smt-bounded flag table gains `--assumes`; the "specs with assumptions or on_error are rejected" line is retired; the cheap/expensive assumption split + omission warning documented (task 3E expands into the residual-differences section) |
| `docs/user/USAGE.md` | change | `rfs mc-ref` + generated-MC flag tables: `--assumes` now applies to smt-bounded; drop the not-applicable note |
| `KB.md` | change | section 7: remove the "smt-bounded rejects assumptions/on_error" limitation (or annotate it as now-supported with the flag-gated expensive class) |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_smt_test.rs` | change | new pins: an on_error spec (unmatched input takes the fallback - PASS and a FAIL variant where the fallback leads to a violation); a cheap-assumption spec (the assumed input is forced - a property that would fail on a non-assumed input now passes because that input is pruned); an expensive assumption WITHOUT the flag (omission warning + `assumptions_omitted=1`); the same WITH `--full-assumptions`; `--assumes none` reproduces the assumption-free verdict |
| `tests/smt_bounded_test.rs` | change | ref-vs-generated byte-parity for every new pin; generated-binary `--assumes` flag behavior; the old "smt-bounded rejects assumptions/on_error" shared-error pins updated to the new contract |

## Planned actions

### 3C.1 - Extend the shared encoding model (`src/mc/smt_plan.rs`)
Remove the two `build_smt_spec` rejections. Add `SmtOnError { next_state: usize,
emits: Vec<SmtEmit> }` and `SmtSpec.on_error: Option<SmtOnError>` (convert
`module.on_error`, resolving `next_state` to an index and converting each emit's argument
expressions in a no-binding context - an enum-path argument still hits the shared enum
rejection until 3D). Add `SmtAssumption { name: String, formula: Sf, full: bool }` and
`SmtSpec.assumptions: Vec<SmtAssumption>`: for each assumption in the module, convert its
formula with `convert_formula` (the property path) and classify it FULL iff its body
contains a temporal operator nested under another, or is top-level `eventually`/`until`
(the liveness forms); a top-level `always (input-guard)` is DEFAULT. Take the selection
from a new `assumes: &AssumptionSelection` parameter threaded into `build_smt_spec` (the
same type exact mode uses) and encode only the selected assumptions. Update the
module-header v1-scope comment (assumptions + on_error now supported; enum-typed vars /
wildcard groups / enum patterns stay rejected until 3D).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change | the shared model gains on_error + assumptions; the two rejections are removed - single source for both emitters and the in-process engine (parity by construction) |

### 3C.2 - Apply on_error + assumptions in the in-process engine (`src/mc/ref_mc_smt.rs`)
In `run_unrolled`, after computing the real transitions' `fires`/`any_fire`: if
`spec.on_error` is present, compute `on_error_fire = any_fire.not()`, set the next state to
`on_error_fire.ite(on_error.next_state, <real chain or frozen>)`, leave next vars unchanged
by the fallback, feed the fallback's emits into the step's `output_has` atoms, and set
`alive[i+1] = alive[i]` (the disjunction is always true). Assert each SELECTED assumption's
formula at every input-consuming step i in 0..k-1 as a base constraint (cheap form
directly; expensive form only when `full_properties`, else omit with the per-assumption
warning and count it for `assumptions_omitted=M`). Thread the `--assumes` selection from
`run_smt`. Append `assumptions_omitted=M` to the summary line iff M > 0 (existing pins
stay byte-identical when no expensive assumption is omitted).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_smt.rs` | change | in-process on_error fallback + per-step assumption constraints + `--assumes` + omission warnings |

### 3C.3 - Mirror in the emitters (`src/mc/smt_emit.rs`)
Emit the on_error catch-all into BOTH runtimes' unrolling loops (the `on_error_fire`
term, the next-state ITE, the emits feeding the oh_ atoms, and the `alive[i+1] = alive[i]`
update). Emit the per-step assumption base-constraints (a loop over steps asserting each
selected assumption's formula; expensive ones guarded by the `full_properties` flag with
the omission warning + count). Carry the `--assumes` selection into the emitted CLI
parsing and the summary suffix. Keep specs with NO on_error and NO assumptions generating
byte-identical binaries to today (guard every new emission behind the presence of the
construct).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_emit.rs` | change | rust + c emission of on_error + assumptions + `--assumes`; no-construct specs stay byte-identical |

### 3C.4 - Mirror in the generated runtimes + CLI (`src/codegen/backends/{rust,c}/mc.rs`, `src/cli/commands/mc_ref.rs`)
Update the emitted Rust `run_smt()` and C `main()` to apply the on_error fallback and the
per-step assumption constraints, parse `--assumes all|none|labels`, and emit the omission
warnings + summary suffix. Add `--assumes` to `rfs mc-ref` under `--mode smt-bounded`
(removing its not-applicable status) and pass the selection through to `run_smt`. Update
every pin/doc asserting the old "rejects assumptions/on_error" contract.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/mc.rs` | change | emitted Rust runtime: on_error + assumptions + `--assumes` |
| `src/codegen/backends/c/mc.rs` | change | emitted C runtime: same |
| `src/cli/commands/mc_ref.rs` | change | `--assumes` accepted under smt-bounded, passed to run_smt; not-applicable note removed |

### 3C.5 - Tests (`tests/ref_mc_smt_test.rs`, `tests/smt_bounded_test.rs`)
Add pinned specs: (a) an on_error spec where an unmatched input takes the fallback - a PASS
variant and a FAIL variant whose counterexample shows the fallback transition (pinned CE);
(b) a cheap-assumption spec (`assume always (input_is(TimerDone))`) where a property that
would fail on a non-TimerDone input now PASSES because that input is pruned, plus a FAIL
variant where the assumed input itself leads to a violation; (c) an expensive assumption
(`assume eventually (...)`) WITHOUT `--full-assumptions`: omission warning text pinned +
`assumptions_omitted=1` suffix + the remaining checks unaffected; (d) the same WITH
`--full-assumptions`: the expansion runs and its verdict is pinned; (e) `--assumes none`
reproduces the assumption-free verdict byte-for-byte. Pin every case on ref AND both
generated backends with byte-parity assertions. Update the old shared-error pins.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change | in-process pins for (a)-(e) |
| `tests/smt_bounded_test.rs` | change | generated-binary pins + ref-vs-generated parity for (a)-(e); old rejection pins updated |

### 3C.6 - Docs sync
Update `docs/SEMANTICS.md` (normative on_error + assumption support, the cheap/expensive
split, the omission warning), `docs/user/MODEL_CHECKING.md` + `docs/user/USAGE.md`
(`--assumes` row for both CLIs; retire the rejection line), and `KB.md` section 7. The full
residual-differences section is task 3E's deliverable.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | normative on_error + assumption semantics |
| `docs/user/MODEL_CHECKING.md` | change | user-facing flag + supported-constructs update |
| `docs/user/USAGE.md` | change | CLI flag tables |
| `KB.md` | change | known-limitations update |

## Task deliverables
- smt-bounded mode (ref + both generated backends, parity-pinned) accepts and checks specs
  with `on_error` blocks (the fallback is taken on unmatched inputs, §3.6-faithful) and
  with assumptions (selected via `--assumes`, pruned per-step, §7.x-faithful).
- Cheap input-guard assumptions are default; expensive liveness/temporal assumptions are
  `--full-assumptions`-gated with per-assumption omission warnings +
  `assumptions_omitted=M`.
- `--assumes all|none|labels` works on both smt surfaces; `none` reproduces today's
  assumption-free verdicts byte-for-byte.
- Every v1-scope note that declared assumptions/on_error rejected is updated.

## Task exit criteria
- [ ] An on_error spec checks correctly: unmatched inputs take the fallback (PASS + FAIL variants pinned, CE shows the fallback step) - ref + both backends, parity-pinned.
- [ ] A cheap assumption (`assume always (input_is(...))`) prunes non-conforming inputs (a property that fails on a pruned input now passes; a violation on the assumed input still fails) - parity-pinned.
- [ ] An expensive assumption is omitted WITHOUT `--full-assumptions` (pinned warning + `assumptions_omitted=1`) and checked WITH it (verdict pinned).
- [ ] `--assumes none` reproduces the assumption-free verdict byte-for-byte; `--assumes all|labels` selects correctly - both smt surfaces.
- [ ] Every pre-existing smt pin for a spec with NO on_error and NO assumptions stays byte-identical.
- [ ] The old "smt-bounded rejects assumptions/on_error" shared-error pins are updated to the new contract.
- [ ] `cargo test --features smt` green; full checklist `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Docs synchronized (SEMANTICS.md + MODEL_CHECKING.md + USAGE.md + KB.md).

## Non-goals
- No enum-typed var / wildcard-group / enum-pattern support (D8) - that is task 3D (an on_error emit or assumption guard with an enum-path argument still hits the shared enum rejection until then).
- No changes to exact-mode or fuzz-mode behavior (they already support both constructs).
- No rework of example specs to USE assumptions/on_error - that is follow-up work (e.g. B6's bounded-liveness cycle) once this lands; this task only adds the capability + tests.
- No building of the MODEL_CHECKING.md residual-differences section - that is task 3E.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The on_error fallback has NO variable assignments (`OnError` carries only `next_state`
  + `emits`), so the next-var ITE chains are unchanged by it - only the next-state and the
  oh_ atoms change. Do not invent assignments.
- An assumption's top-level `always` is DROPPED during conversion (the per-step assertion
  loop IS the universal quantification); a top-level `eventually`/`until` or any temporal
  nesting marks it FULL. A bare input-guard assumption (no `always`) is treated as
  `always (guard)` - confirm this matches how the validator/parser present assumptions and
  note any discrepancy in the corrections table.
- The `assumptions_omitted=M` suffix mirrors `properties_omitted=M`: appended only when
  M > 0, so no existing summary line changes.
- If an on_error spec ALSO has a D1-style unhandled input, the fallback (not the frozen
  self-loop) is taken, so 3A's liveness gate and this task compose cleanly: `alive` stays
  true through the fallback, and a `next` body is checked against the real fallback
  successor.
