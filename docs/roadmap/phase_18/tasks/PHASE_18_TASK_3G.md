# Phase 18, Task 3G - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Lift audit row **S4**: make smt-bounded mode accept the top-level property/assumption
forms that exact mode already accepts - a bare boolean formula (`prop p : state == A ;`),
a top-level `next p`, and any boolean combination of atoms/temporals that is not one of
the five currently-accepted top forms - by desugaring them to `always(...)` BEFORE the
existing kind match. Rationale: today the identical spec PASSes under exact mode but is
rejected by smt-bounded with "smt-bounded mode supports top-level
always/always[N]/eventually/bounded_eventually/until properties only" - a cross-mode
asymmetry that misrepresents the mode; the fix is a wrap with zero formula-size change
for the common cases.

## Context

### The full audit lives in `V1_R_R_AUDIT.md` (repo root)
Read its Table 2 row for S4 before starting. This task implements exactly that row.

### What exact mode does with these forms (the semantics to mirror)
Exact mode compiles EVERY property through `src/mc/ref_mc.rs::compile_formula` ->
`compile_check(f, Range::all(), false)`. A bare signal (`Expr`/`InputIs`/`OutputHas`,
or a boolean combination of them) becomes a `RangeCheck` anchored at `Range::all()` -
i.e. it is checked at EVERY sample. A top-level `next p` becomes a `RangeCheck` at
`Range::all().shifted()` - checked at every sample from 1 on. So in exact mode:
- `prop p : state == A ;` means "A at every sample";
- `prop p : next (state == A) ;` means "A at every sample from 1 on";
- `prop p : input_is(E) -> bounded_eventually[5] (state == A) ;` is the canonical R2
  obligation, checked at every sample where E fires.
All three are legal and monitored. smt-bounded must reproduce these verdicts.

### The two rejection sites in `src/mc/smt_plan.rs`
**Properties** - `build_smt_spec`, the property loop (~line 745):

```rust
let (kind, bound) = match &p.formula {
    Formula::Always { .. } => (SmtPropKind::Always, None),
    Formula::BoundedAlways { bound, .. } => (SmtPropKind::Always, Some(*bound as usize)),
    Formula::Eventually { .. } => (SmtPropKind::Eventually, None),
    Formula::BoundedEventually { bound, .. } => (SmtPropKind::BoundedEventually(*bound as usize), None),
    Formula::Until { .. } => (SmtPropKind::Until, None),
    other => return Err(format!(
        "smt-bounded mode supports top-level always/always[N]/eventually/bounded_eventually/until properties only (property '{}' is {:?})",
        p.name, formula_tag(other)))
};
```
followed by a conversion match that destructures the top form and converts its INNER
(`Formula::Always { inner } | ... => convert_formula(inner, &ctx)?`, `Until` handled
explicitly, `_ => unreachable!("top form matched above")`).

**Assumptions** - `convert_assumption` (~line 856): top-level `Always`/`BoundedAlways`
are DROPPED (the per-sample assertion loop IS the universal quantification - see the
function's doc comment), `Eventually`/`BoundedEventually`/`Until` are carried as `Sf`
nodes with `full = true`, and the `other` arm (~line 888) rejects everything else with
"...assumptions only (assumption '{}' is {:?})".

### The desugar (design decision, already made)
**Properties:** normalize before the kind match - if the top node is not one of the
five accepted forms, wrap the whole formula: `Formula::Always { inner: Box::new(original) }`.
The wrapped case then takes the existing `Always` arm: kind = `SmtPropKind::Always`,
bound = None, and the conversion match converts the ORIGINAL top form as a nested node
via `convert_formula` - which already handles every node kind (`Next` -> `Sf::Next`,
atoms -> atoms, boolean connectives -> logical nodes, nested `BoundedEventually`/`Until`/
`Always`/`BoundedAlways` -> their `Sf` forms). No new `SmtPropKind`, no engine change.

**Assumptions:** make the `other` arm behave exactly like the `Always` arm -
`(convert_formula(&a.formula, ctx)?, None, nested_temporal(&a.formula))`. A bare boolean
assumption asserted per-sample means "holds at every sample" (exact-mode semantics);
a top-level `next p` asserted per-sample with horizon truncation means "p at every
sample from 1 on" - identical to exact mode.

### Why the performance classification stays honest
`classify_full` and `nested_temporal`/`contains_temporal` operate structurally:
- bare boolean wrapped -> `Always{atoms}`: `contains_temporal` = false -> DEFAULT class
  (checked without `--full-properties`), zero formula growth beyond the per-step atom;
- `next p` wrapped -> `Always{Next{p}}`: still no temporal inside -> DEFAULT class;
- `input_is(E) -> bounded_eventually[5] (q)` wrapped -> `Always{Implies{.., BoundedEventually}}`:
  `contains_temporal` = true -> FULL class, gated behind `--full-properties`, exactly as
  a genuinely per-sample obligation should be (the O(k*N) expansion is real here).
So the wrap never hides an expensive formula behind the default gate - and
`next_depth(wrapped) == next_depth(original)` (wrapping in `Always` does not change it),
so the horizon-extension logic needs no change.

### Interaction with tasks 3F/3H
All three edit `src/mc/smt_plan.rs` in DISJOINT regions: 3G touches the property kind
match (~745) + its conversion match (~772) + `convert_assumption`'s `other` arm (~888);
3F deletes the S3 `if` block (~478); 3H rewrites the var-init match (~425-446). Separate
commits.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3C. | Implement assumptions (D3) + on_error blocks (D4) in smt-bounded | already complete - `convert_assumption` and the per-sample assertion loop it feeds exist; this task only widens its accepted top forms |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3I. | Tests for the 3F-3H changes + test-suite adoption | adds the exact-vs-smt parity pins for bare/`next` top forms |
| 3J. | Documentation + README sync | updates MODEL_CHECKING.md's residual-differences section (task 3E's) so S4 no longer appears as a difference |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/smt_plan.rs` | change | property-loop normalization wrap + `convert_assumption` `other` arm widened; the two error strings retire |

## Planned actions

### Normalize property top forms before the kind match
In `build_smt_spec`'s property loop, insert a normalization step before the existing
`(kind, bound)` match: if `p.formula` is not one of `Always`/`BoundedAlways`/
`Eventually`/`BoundedEventually`/`Until`, work with `Formula::Always { inner: Box::new(p.formula.clone()) }`
instead (a local `let formula = ...;` the rest of the loop - kind match, conversion
match, `classify_full`, `next_depth` - then reads). The five accepted forms pass through
unchanged, so every existing pin stays byte-identical. Update the loop's leading comment
(~line 739) to state that other top forms are desugared to `always(...)` (task 3G),
mirroring exact mode's `Range::all()` anchoring.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change (~739-790) | normalization + comment; the `other => Err(...)` property arm becomes unreachable but is kept as a defensive `unreachable!`-style guard OR deleted - prefer deleting it and letting the conversion match's existing `_ => unreachable!("top form matched above")` cover the invariant, since after normalization the top form is always one of the five |

### Widen `convert_assumption`'s rejected arm
Replace the `other => Err("...assumptions only...")` arm with the Always-arm behavior:
`(convert_formula(&a.formula, ctx)?, None, nested_temporal(&a.formula))`. Update the
function's doc comment: top forms other than the listed ones are asserted per-sample as
their converted `Sf` node (task 3G) - a bare boolean means "every sample", a top-level
`next` shifts by one, both matching exact mode; FULL classification still comes from
`nested_temporal`, so expensive shapes stay behind `--full-assumptions`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change (`convert_assumption`, ~843-895) | `other` arm widened; doc comment updated |

### Verify parity on the lifted forms
Scratch specs (in /tmp, not committed): (1) `prop p : state == A ;` and (2)
`prop p : next (state == B) ;` and (3) `assume a : input_is(E) ;` - each must model-check
under `rfs mc-ref --mode exact` and `--mode smt-bounded` with the SAME verdict, and the
smt run must NOT print an omission warning for them (they are DEFAULT class). Then run
the full matrix: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt`.
All pre-existing pins (including the 3A-3D twins' captures) must stay byte-identical -
none of them uses a non-five top form, so nothing may move.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/mc/smt_plan.rs` accepting every top-level property/assumption form exact mode
  accepts, via the `always(...)` desugar, with honest FULL-class gating preserved.
- A green full test matrix with all pre-existing pins byte-identical.

## Task exit criteria
- [ ] `rfs mc-ref --mode smt-bounded` on a scratch spec with `prop p : state == A ;`
      returns the same verdict as `--mode exact` and prints no omission warning for it.
- [ ] Same for `prop p : next (state == B) ;` and `assume a : input_is(E) ;`.
- [ ] `input_is(E) -> bounded_eventually[5] (state == A)` as a top-level property is
      accepted but reported as FULL class (omitted without `--full-properties`, checked
      with it) - the gate was not weakened.
- [ ] `grep -rn "top-level always/always" src/mc/smt_plan.rs` returns nothing (both old
      error strings retired).
- [ ] `cargo test`, `cargo clippy --all-targets`, `cargo fmt --check` green; every
      pre-existing capture/pin byte-identical (the file-driven suites + the 3A-3D twin
      captures are the regression net).

## Non-goals
- No new tests/examples committed (task 3I owns suite adoption and parity pins).
- No doc updates (task 3J).
- No change to the `--full-properties`/`--full-assumptions` gating logic itself - only
  which formulas reach it.
- No engine (`ref_mc_smt.rs` / `smt_emit.rs`) change: the desugar happens entirely in
  the planner, so generated binaries and the in-process engine see only the five top
  forms they already implement.
- E1-E3 (exact-mode liveness decomposition limits) are untouched - this task does not
  make exact mode accept anything new.

## Other information
- If a wrapped formula's conversion hits a node kind `convert_formula` does not handle,
  that is a planner bug to fix in `convert_formula` (it should already handle all of
  them - verify by reading its match before editing); do NOT add a new rejection.
- The assumption desugar deliberately reuses `nested_temporal(&a.formula)` (computed on
  the ORIGINAL, unwrapped formula) - wrapping an assumption in `Always` would make
  `nested_temporal` see the wrapper and misclassify; the Always arm already has this
  exact shape, so copy it verbatim.
