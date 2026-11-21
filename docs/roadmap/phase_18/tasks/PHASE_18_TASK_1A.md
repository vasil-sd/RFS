# Phase 18, Task 1A - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Extend `--mode smt-bounded` (in-process `ref_mc_smt` AND both generated SMT binaries,
parity-pinned) toward exact-mode property support: enable the CHEAP operators
(`input_is`, `output_has`, `next`) by default, put the EXPENSIVE class
(`bounded_eventually[N]` under a universal anchor, `until`, temporal-under-temporal
nesting) behind a new `--full-properties` CLI flag with per-property omission warnings,
and support RENAMED event parameters unconditionally. Rationale: smt-bounded is the
analysis mode for the large-state-space twins; it should check the same properties exact
mode checks, with only the formula-blowup class opt-in.

## Context
- **The fire predicates already exist.** Both unrolling engines (in-process
  `src/mc/ref_mc_smt.rs::run_unrolled` and the emitted runtimes in
  `src/codegen/backends/{rust,c}/mc.rs`) compute, per step i and per transition t,
  `enabled_t(i)` = state-match AND class-match AND guard, and `fire_t(i)` = first-match
  (enabled AND no earlier transition enabled), then build next-state/next-var ITE chains.
  So `output_has(X)(i) = OR over t of fire_t(i) AND emit-matches(t, X)` reuses existing
  machinery - the only missing data is the emit list, which `SmtTrans` currently DROPS
  (`src/mc/smt_plan.rs:104-117`: state/next_state/class/guard/assignments only).
- **Property functions are plain fn pointers** (`fn(usize) -> Bool`, one per property,
  rendered by `smt_emit.rs`). They cannot capture engine locals. Therefore:
  - `OutputHas` is served by PRECOMPUTED named z3 constants `oh_{p}_{i}` created inside
    the unrolling loop (where `fires` live), referenced by name from the property fn
    (z3 hash-conses same-named constants to the same object). No signature change.
  - `InputIs` renders directly over the step's input-class selector `ev_i` and payload
    fields `fld_{i}_{g}` (free z3 constants that already exist).
  - `Next(inner)` renders `inner` at step i+1; a codegen-time per-property flag
    `PROP_HAS_NEXT` makes the query loop check such properties only at steps 0..k-2
    (at i=k-1 the i+1 variables do not exist; dropping the last anchor is SOUND - it can
    only miss a violation, never create a false FAIL).
  - Full-class properties (`bounded_eventually`/`until`/nested temporals) need the
    runtime horizon k inside their expansion, so they get a SEPARATE emitted expansion
    function `fn expand_p{p}(k: usize, fires: &[Vec<Bool>]) -> Bool` (Rust) / equivalent
    C helper, called from the query section instead of `props[p](i)`. Default-class
    property fns keep their exact current signature - existing generated binaries for
    existing specs stay BYTE-IDENTICAL.
- **Matching semantics to mirror EXACTLY** (`src/mc/ref_mc.rs::input_matches` /
  `output_matches`, lines ~567-640): group must match; variant None = every variant of
  the group; Unit = variant-name equality; Struct = variant-name equality + field-count
  equality + per-field: Wildcard/Bind constrain nothing, Int/Bool literals compare,
  EnumVariant compares the full qualified path. In SMT v1 (no enum vars): a literal
  constraint on an ABSTRACTED payload field folds at conversion time against the field's
  representative value; an emitted output argument that is an enum path is rejected with
  the shared "enum paths unsupported" error (consistent with existing v1 scope).
- **Renamed parameters are positional.** The parser (`src/lang/parser.rs::pattern_field`)
  records `PatternField { name: Some(<written name>), pattern: Bind { name: <written
  name> } }` for a bare `k`, and matching zips pattern fields against DECLARED fields
  positionally - the written name is NOT the IR field name. The SMT encoder builds its
  guard context only from declared names (`smt_plan.rs` ~line 339), so a guard referencing
  the binding dies with "unknown identifier". Fix: when building a transition's context,
  zip `t.pattern.variant`'s struct fields against the class's declared fields positionally
  and alias each `Bind { name }` to the declared field name at that position; resolve the
  alias at the TOP of `convert_expr`'s Path arm (before the payload-field lookup) so the
  width/domain lookup still uses the declared name. Precedence stays payload-bindings >
  vars > state-variants (the existing order), matching exact mode.
- **Property classification (default vs full).** A property is FULL (needs the flag) iff
  its formula contains `until`, or `bounded_eventually` NOT at top level, or any temporal
  operator (`always`/`always[N]`/`bounded_eventually`) nested under another temporal
  operator (including under `next`). `next` alone never triggers FULL (it is a pure step
  shift). Default class = top-level `always`/`eventually`/`always[N]`/`bounded_eventually`
  whose bodies use expr/not/and/or/implies/input_is/output_has/next only. The spec is
  already validator-legal when it reaches the SMT encoder - no re-validation needed.
- **Full-class expansion semantics (quantifier-free, horizon-truncated, SOUND):**
  - `bounded_eventually[N](f)` at anchor step q: `OR over j in q..min(q+N, k) of f(j)`.
    Under a universal anchor (the query loop's forall-i), anchors with q+N > k are VACUOUS
    (their window exceeds the horizon - the obligation is still pending, unconfirmable):
    render them as `true` to avoid false FAILs. Top-level one-shot `bounded_eventually[N]`:
    violation iff N <= k AND no j in 0..N satisfies f; if N > k there is no violation
    claim (incomplete).
  - `p until q` over the horizon: `(OR over j < k of q(j) AND AND over i<=j of p(i)) OR
    (AND over i < k of p(i))` - the second disjunct is horizon-truncated (weaker than true
    "p forever"), which is sound (no false FAILs; no completeness claim either way).
  - Nested temporals expand recursively with step offsets and the same horizon clipping.
- **CLI flag.** `--full-properties` on BOTH smt surfaces: `rfs mc-ref --mode
  smt-bounded --full-properties` (usage error under other modes, like the other
  mode-specific flags) and the generated SMT binaries (which currently parse ONLY
  `--bound` - that pinned contract changes; update every pin/doc that asserts it).
  Without the flag, FULL properties are OMITTED with a per-property warning on stderr:
  `warning: smt-bounded mode omits property '<name>' by default - it uses <bounded_
  eventually/until/nested temporal operators>, which greatly affect solver time/memory at
  large bounds (O(k*N) formula growth). Pass --full-properties to include it.` The
  summary line gains `properties_omitted=M` ONLY when M > 0, so every existing capture
  and pin stays byte-identical.
- **v1 scope that STAYS rejected** (shared errors unchanged): assumptions, on_error
  blocks, enum-typed vars, wildcard event patterns over multi-variant groups in
  transitions. This task does not add assumption support to smt-bounded.
- **Parity is by construction:** the shared encoding model (`smt_plan.rs`) feeds both
  emitters and the in-process engine; the unrolling changes must be mirrored identically
  in `ref_mc_smt.rs` and both backend runtimes, and `tests/smt_bounded_test.rs` +
  `tests/ref_mc_smt_test.rs` pin ref-vs-generated byte-parity.

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Protocol FSMs A1-A10 | its correction 1a (plain-boolean twin workaround) is what this task supersedes; its three twins' captures must stay byte-identical after the change |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1B. | dns_resolver twin rewrite | uses this task's output_has + renamed-parameter support |
| 2.-6. | remaining FSM batches | their twins may now use natural properties (correction 1a's constraint lifted) |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/smt_plan.rs` | change | `SmtTrans.emits` (+`SmtEmit`), new `Sf` nodes (`InputIs`/`OutputHas`/`Next`/full-class), property conversion accepting the default class, FULL classification, binding-name aliases in the transition context |
| `src/mc/smt_emit.rs` | change | render the new `Sf` nodes for rust + c; per-property `PROP_HAS_NEXT`; precomputed `oh_{p}_{i}` constants; emitted full-class expansion functions |
| `src/mc/ref_mc_smt.rs` | change | retain per-step fires, build `oh_` constants, evaluate `Next` at i+1 with boundary skip, full-class expansions, `--full-properties` handling + omission warnings + summary suffix |
| `src/codegen/backends/rust/mc.rs` | change | emitted SMT runtime: same engine changes + `--full-properties` CLI flag (replaces the "only --bound" contract) |
| `src/codegen/backends/c/mc.rs` | change | same for the C emitted runtime |
| `src/cli/commands/mc_ref.rs` | change | accept `--full-properties` under `--mode smt-bounded` (usage error otherwise); pass it to `ref_mc::run_smt` |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/SEMANTICS.md` | change | normative: smt-bounded property support - default class, full class + flag, omission warning, horizon-truncation soundness rule, renamed-parameter resolution |
| `docs/user/MODEL_CHECKING.md` | change | user-facing: what smt-bounded checks now, the flag, the warning |
| `docs/user/USAGE.md` | change | `rfs mc-ref` + generated-MC flag tables: `--full-properties` row; drop the "generated SMT binary parses only --bound" statement where it appears |
| `KB.md` | change | section 7 (known limitations) if it lists the smt-bounded property restriction - update to the new default/full split |
| `BUILD.md` | change | test-count rows if they shift; the smt suite description if the flag changes expected output |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_smt_test.rs` | change | new pins: input_is/output_has/next default support, renamed parameters, full-class omission warning + summary suffix, --full-properties expansion, boundary-skip soundness |
| `tests/smt_bounded_test.rs` | change | ref-vs-generated parity for every new feature; generated-binary `--full-properties` flag behavior; the old "only --bound" usage-error pins updated |

## Planned actions

### 1A.1 - Extend the shared encoding model (`src/mc/smt_plan.rs`)
Add `SmtEmit { group, variant: Option<String>, args: Vec<Se> }` and `SmtTrans.emits`
(convert each emitted output's argument expressions with the transition's context; an
enum-path argument is rejected with the shared enum error). Add `Sf::InputIs { classes:
Vec<usize>, constraints: Vec<(usize, u64, bool)> }` (pre-resolved matching class set +
literal field constraints in GLOBAL field indices; abstracted-field constraints fold
against their representative at conversion time), `Sf::OutputHas { group, variant:
Option<String>, constraints: Vec<(usize, u64, bool)> }` (pattern positionally aligned to
the output declaration's fields), and `Sf::Next(Box<Sf>)`. Extend `convert_formula` to
accept these in property bodies (keeping the existing top-level always/eventually gate).
Classify each property FULL per the Context rule and store it on `SmtProp`. Build the
binding-name alias map from `t.pattern`'s struct fields zipped positionally against the
class's declared fields, and resolve aliases at the top of `convert_expr`'s Path arm.
Update the module header's v1-scope comment (input/output references now supported;
assumptions/on_error still rejected).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change | the shared model + conversion changes above - single source for both emitters and the in-process engine (parity by construction) |

### 1A.2 - Extend the in-process engine (`src/mc/ref_mc_smt.rs`)
Retain the per-step `fires` vectors across the unrolling loop; for each property whose
formula contains `OutputHas`, precompute `oh_{p}_{i}` = OR over t of fire_t(i) AND the
emit-match conjunction (rendered from the stored emit data at step i) and register it as
a named z3 Bool that `sf_to_z3` resolves for `Sf::OutputHas`. Render `Sf::InputIs` over
`ev_i`/`fld_{i}_{g}`. Evaluate `Sf::Next` at step i+1; set a per-property has-next flag
and check such properties only at steps 0..k-2. Implement the full-class expansions
(bounded_eventually/until/nested) as horizon-truncated quantifier-free formulas per the
Context semantics. Handle the `full_properties: bool` parameter: without it, skip FULL
properties, print the per-property stderr warning, and append `properties_omitted=M` to
the summary line iff M > 0. Thread the flag from `run_smt`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_smt.rs` | change | in-process engine changes above; `run_smt` gains the flag parameter |

### 1A.3 - Extend the emitters (`src/mc/smt_emit.rs`)
Render `Sf::InputIs`/`Sf::OutputHas`/`Sf::Next` in BOTH the rust and c renderers
(`render_sf_rust` / `render_sf_c` + the `se_uses_step`/`sf_uses_step` walkers). Emit the
per-property `PROP_HAS_NEXT` table. Emit the `oh_{p}_{i}` constant creation inside the
unrolling loop of BOTH emitted runtimes (the emitters generate that loop's code). Emit
full-class expansion functions (`expand_p{p}(k, fires)`) for FULL properties and route
the query section through them. Keep default-class property fns signature-identical so
existing specs' generated binaries stay byte-identical.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_emit.rs` | change | rust + c rendering of the new nodes, the oh_ constants, PROP_HAS_NEXT, and full-class expansion functions |

### 1A.4 - Extend the generated SMT runtimes + CLI (`src/codegen/backends/{rust,c}/mc.rs`, `src/cli/commands/mc_ref.rs`)
Mirror the 1A.2 engine changes in the emitted rust and c runtimes (fires retention, oh_
constants, next boundary skip, full-class expansions, warning + summary suffix). Add
`--full-properties` to the generated SMT binaries' arg parsing (replacing the "only
--bound" contract) and to `rfs mc-ref` under `--mode smt-bounded` (usage error under
other modes). Update every pin/doc asserting the old contract.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/mc.rs` | change | emitted rust SMT runtime + its CLI flag |
| `src/codegen/backends/c/mc.rs` | change | emitted c SMT runtime + its CLI flag |
| `src/cli/commands/mc_ref.rs` | change | `--full-properties` accepted under smt-bounded, passed to run_smt; usage-error pins updated |

### 1A.5 - Tests (`tests/ref_mc_smt_test.rs`, `tests/smt_bounded_test.rs`)
Add pinned specs exercising: (a) `input_is` in a property (default mode); (b)
`output_has` in a property (default mode) with a PASS and a FAIL variant whose CE is
pinned; (c) `next` (default mode) incl. the boundary-skip soundness pin (a violation only
reachable at the last step is NOT reported - no false FAIL); (d) renamed event parameters
in patterns/guards (default mode, must now encode and check correctly); (e) a FULL
property WITHOUT the flag: omission warning text pinned + `properties_omitted=1` summary
suffix + the remaining properties still checked; (f) the same spec WITH
`--full-properties`: the expansion runs and its verdict is pinned. Every feature is
pinned on ref AND both generated backends with byte-parity assertions (the existing
parity harness). Update the old "generated SMT binary rejects every flag but --bound"
pins to the new contract.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change | in-process pins for (a)-(f) |
| `tests/smt_bounded_test.rs` | change | generated-binary pins + ref-vs-generated parity for (a)-(f); old flag-contract pins updated |

### 1A.6 - Docs sync
Update `docs/SEMANTICS.md` (normative smt-bounded property support: default class, full
class + `--full-properties`, the omission warning, horizon-truncation soundness rule,
renamed-parameter resolution), `docs/user/MODEL_CHECKING.md` + `docs/user/USAGE.md`
(flag rows for both CLIs; retire the "parses only --bound" statement), `KB.md` section 7
if it lists the old restriction, and `BUILD.md` count rows if they shift.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | normative semantics of the extended smt-bounded property support |
| `docs/user/MODEL_CHECKING.md` | change | user-facing mode description + flag + warning |
| `docs/user/USAGE.md` | change | CLI flag tables for mc-ref and generated MC |
| `KB.md` | change | known-limitations update if the old restriction is listed |
| `BUILD.md` | change | test-count / suite-description rows if shifted |

## Task deliverables
- smt-bounded mode (ref + both generated backends) checks `input_is`/`output_has`/`next`
  properties by default and renamed-parameter specs unconditionally.
- `--full-properties` flag on both smt surfaces; FULL properties omitted with a pinned
  per-property warning otherwise; `properties_omitted=M` summary suffix only when M > 0.
- All pre-existing smt pins (incl. the three Phase 18 task-1 twins' captures) stay
  byte-identical; new features parity-pinned across ref + rust + c.
- Docs synchronized (SEMANTICS normative + user docs + KB/BUILD where applicable).

## Task exit criteria
- [ ] `input_is`, `output_has`, `next` properties encode and check correctly in smt-bounded (ref + both backends, parity-pinned), PASS and FAIL variants pinned.
- [ ] Renamed event parameters (`R(k)` for a differently-named IR field) encode and check correctly - no "unknown identifier" - parity-pinned.
- [ ] FULL properties (bounded_eventually under a universal anchor / until / nested temporals) are omitted WITHOUT `--full-properties` with the pinned per-property warning + `properties_omitted=M` suffix, and checked WITH it (expansion verdicts pinned).
- [ ] The next boundary-skip is sound: no false FAIL at the horizon edge (pinned negative control).
- [ ] Every pre-existing smt pin stays byte-identical (tcp/quic/dns twin captures, all ref_mc_smt + smt_bounded suite pins).
- [ ] `cargo test --features smt` green; full checklist `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Docs synchronized (SEMANTICS.md normative section + MODEL_CHECKING.md + USAGE.md + KB.md/BUILD.md where applicable).

## Non-goals
- No assumption support in smt-bounded (stays rejected, shared error unchanged).
- No on_error support in smt-bounded (stays rejected).
- No enum-typed vars / enum-path emitted arguments (v1 scope unchanged).
- No changes to exact-mode or fuzz-mode behavior.
- No dns_resolver rewrite (that is task 1B).

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The `oh_{p}_{i}` naming must be parity-critical (fixed order: property index, then
  step) exactly like the existing `s_{i}`/`v_{i}_{j}`/`ev_{i}`/`fld_{i}_{g}` convention.
- If a spec has NO OutputHas properties, no `oh_` constants are created and the emitted
  runtime is byte-identical to today's output (verify with a diff of a generated binary
  for an existing spec).
