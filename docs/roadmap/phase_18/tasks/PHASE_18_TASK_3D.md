# Phase 18, Task 3D - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Maximize the RFS LANGUAGE surface that smt-bounded mode accepts, by removing the six
**D8** rejections in `src/mc/smt_plan.rs` and encoding each construct FAITHFULLY to exact
mode: (1) enum-typed variables, (2) enum-typed concrete payload fields, (3) wildcard event
patterns over multi-variant groups in transitions, (4) enum paths in expressions,
(5) enum-variant output patterns, and (6) enum-variant input patterns. The encodings are
all index-based (an enum value is its variant's declaration-order index in a
`bit<ceil_log2(N)>`), which is CHEAP - so they are enabled BY DEFAULT. Any encoding that
profiling shows to greatly affect solver time/memory at large bounds (the formula-blowup
class, like the FULL property class) goes behind the existing `--full-properties` CLI
flag. Rationale: after tasks 3A-3C, the ONLY thing stopping a user from writing a natural
spec with enum states/vars/payloads and checking it under smt-bounded is these six
rejections; removing them makes smt-bounded a near-drop-in analysis mode for exact-mode
specs, which is the point of the whole 3A-3E arc.

## Context

### How exact mode already encodes enums (the model to mirror)
Exact mode represents an enum value as its variant's DECLARATION-ORDER INDEX in a
`bit<ceil_log2(N)>` word (`src/mc/ref_mc_exact.rs`: `Type::Enum { name } =>
ceil_log2(variant_count)` for the var width; `Val::Enum(name)` is replaced by the variant
index at configuration build time). A comparison `x == Color::Red` becomes an index
equality `x == <idx(Red)>`. The SMT encoder must reproduce EXACTLY this: same index order
(declaration order of the enum's variants), same bit width (`ceil_log2`, minimum 1 - the
existing `ceil_log2_pub` helper in `smt_plan.rs`). This guarantees a spec that PASSes
under exact mode encodes to the SAME boolean facts under smt-bounded.

### The six rejections and their encodings
All six live in `src/mc/smt_plan.rs::build_smt_spec` / its helpers. Each is removed and
replaced with a faithful encoding:
1. **enum-typed variables** (the `Type::Enum { name } => return Err(...)` arm in the var
   loop, ~line 307): encode the var as `bit<ceil_log2(variant_count)>` holding the variant
   index; its literal init (`init Red`) becomes the index of `Red`. A var UPDATE to an enum
   literal in a transition assignment becomes an index constant. Comparisons in guards/
   properties become index comparisons (handled by #4).
2. **enum-typed concrete payload fields** (~line 367, the field-width loop): a concrete
   (non-abstracted) payload field of enum type is encoded as `bit<ceil_log2(N)>` like a var.
   An ABSTRACTED enum field (collapsed to a representative by the plan) keeps the existing
   representative-literal folding - but the representative must now be an INDEX, not a
   rejected enum. Confirm how `plan.abstraction` presents an enum field's representative
   and fold it to the index.
3. **wildcard event patterns over multi-variant groups** (~line 448, the transition
   pattern-to-class match): today a bare group pattern (`on Group`, `variant: None`) must
   name EXACTLY one variant or it errors. To support a multi-variant group, the transition
   fires on ANY of the group's variants: change `SmtTrans.class: usize` to
   `SmtTrans.classes: Vec<usize>` (the set of matching input classes) and render
   `enabled_t(i) = OR over c in classes of (state-match AND ev_i == c AND guard_c)` where
   `guard_c` is the guard converted in class c's payload context (each variant may expose
   different fields). A single-variant pattern becomes a one-element `classes` vec - the
   common case is unchanged in formula size. This is the one D8 item that can GROW the
   formula (proportional to the group's variant count); profile it and gate under
   `--full-properties` ONLY if a large group measurably hurts solver time at the bounds the
   twins use.
4. **enum paths in expressions** (~line 669, the `convert_expr` Path arm rejecting an enum
   path): resolve an enum-path literal (`Color::Red`) to its variant index and emit an
   integer constant of the enum's bit width, so `x == Color::Red` renders as an index
   equality. This is what makes #1's var comparisons work.
5. **enum-variant output patterns** (~line 816, the `output_has` field-pattern loop):
   `output_has(Color(Red))` - the emitted enum argument is encoded as its index (via #4's
   path resolution on the emit's argument expression); the pattern constrains that index to
   `idx(Red)`. Add the index constraint to the `Sf::OutputHas.constraints` list (a new
   constraint kind for enum-index equality, or reuse the existing `(pos, value, is_bool)`
   triple with a width tag).
6. **enum-variant input patterns** (~line 904, the `input_is` field-pattern loop):
   `input_is(Color(Red))` - an enum-typed payload field constrained to a variant. Encode
   as an index constraint on the class's concrete field (via #2 + #4), added to the
   `Sf::InputIs` arm's literal-field-constraint list.

### Why this is sound and cheap
Every encoding is a bijection between an enum value and a bit-vector index, applied
IDENTICALLY to the var/field domain, the transition guards/assignments/emits, and the
property formulas - so a boolean fact is true under smt-bounded iff it is true under exact
mode (same index order, same width). No new paths are created and no violations masked.
The formula growth is bounded by `ceil_log2(N)` bits per enum value and by the group's
variant count for #3 - both small for realistic specs. The ONLY item that can blow up is
#3 with a large multi-variant group, which is why it alone is a `--full-properties`
candidate.

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1A. | SMT-bounded property-language extension | provides the `Sf::InputIs`/`Sf::OutputHas` constraint lists and the `convert_expr` Path arm this task extends for enum paths; its pins must stay byte-identical for specs without enums |
| 3C. | Implement D3+D4 | an on_error emit or assumption guard with an enum-path argument hits THIS task's #4 encoding - the two are independent but should be tested together once both land |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3E. | Documentation overhaul | records D8 as now-supported and removes it from the residual-differences list; documents the enum index-encoding so users know an enum var is a bit-vector under the hood |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/smt_plan.rs` | change | remove all six rejections; encode enum vars/fields as index bit-vectors; `SmtTrans.classes: Vec<usize>` for multi-variant wildcards; enum-path resolution in `convert_expr`; enum-index constraints in the input_is/output_has pattern loops; update the module-header v1-scope comment |
| `src/mc/smt_emit.rs` | change | render the multi-class `enabled_t` OR (rust + c); render enum-index constants and constraints; carry any `--full-properties` gating for large-group wildcards |
| `src/mc/ref_mc_smt.rs` | change | in-process: build `enabled_t` as the OR over a transition's classes; evaluate enum-index comparisons/constraints; gate large-group wildcards behind `full_properties` if profiling requires |
| `src/codegen/backends/rust/mc.rs` | change | emitted Rust runtime: same multi-class enabled + enum-index changes |
| `src/codegen/backends/c/mc.rs` | change | emitted C runtime: same |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/SEMANTICS.md` | change | normative: smt-bounded now accepts enum-typed vars/fields, multi-variant wildcard transitions, and enum patterns/paths - encoded as declaration-order indices (faithful to exact mode) |
| `docs/user/MODEL_CHECKING.md` | change | the smt-bounded "What it checks" / limitations note retires the D8 rejections; notes the index encoding (task 3E expands into the residual-differences section) |
| `KB.md` | change | section 7: remove the enum/wildcard/pattern limitations (or annotate the large-group wildcard flag-gating if profiling triggers it) |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_smt_test.rs` | change | new pins: an enum-typed var spec (PASS + a FAIL variant whose CE shows the enum value); an enum payload field; a multi-variant wildcard transition (fires on any variant); an enum path in a guard/property; enum-variant output and input patterns. Each pinned PASS and FAIL where applicable |
| `tests/smt_bounded_test.rs` | change | ref-vs-generated byte-parity for every new pin; a large-group wildcard spec under and over the `--full-properties` threshold if gating is triggered |

## Planned actions

### 3D.1 - Encode enum values as indices in the shared model (`src/mc/smt_plan.rs`)
Add an enum-index helper: given an enum name + variant, return `(index, bit_width)` where
index is declaration order and width is `ceil_log2_pub(variant_count)`. In the var loop,
replace the `Type::Enum` rejection with an index-encoded `SmtVar` (width = the enum's bit
width; init = the init variant's index; record `is_enum` + the variant names for witness
display so a counterexample prints `Color::Red`, not `2`). In the payload-field loop,
encode a concrete enum field as an index bit-vector and fold an abstracted enum field's
representative to its index. In `convert_expr`'s Path arm, resolve an enum-path literal to
an index constant of the enum's width (replacing the rejection). Update the module-header
v1-scope comment (enums now supported).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change | enum vars/fields/paths encoded as declaration-order indices - faithful to exact mode, single source for both emitters and the in-process engine |

### 3D.2 - Support multi-variant wildcard transitions (`src/mc/smt_plan.rs`)
Change `SmtTrans.class: usize` to `SmtTrans.classes: Vec<usize>`. In the transition
pattern-to-class match, a bare group pattern collects ALL of the group's variant classes
into `classes` (removing the `candidates.len() != 1` rejection); a named-variant pattern
yields a one-element vec. Convert the guard ONCE per class in `classes` (each may expose
different payload fields) and store the per-class guards. A single-variant transition is a
one-element vec - formula size unchanged for the common case.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change | `SmtTrans.classes` + per-class guards; multi-variant wildcards accepted |

### 3D.3 - Encode enum patterns in input_is / output_has (`src/mc/smt_plan.rs`)
In the `input_is` field-pattern loop, replace the enum-variant rejection with an
index-constraint on the class's concrete enum field (the pattern variant's index). In the
`output_has` field-pattern loop, do the same for the emitted enum argument. Extend the
constraint representation if needed to carry an enum-index equality distinct from a
bit/bool literal (or tag the existing `(pos, value, is_bool)` triple with the field width
so an index equality is unambiguous).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change | enum-variant input/output patterns become index constraints |

### 3D.4 - Render the multi-class enabled predicate + enum indices (`src/mc/smt_emit.rs`, `src/mc/ref_mc_smt.rs`, both runtimes)
In ALL THREE engines, build `enabled_t(i)` as the OR over a transition's `classes` of
(state-match AND `ev_i == c` AND guard_c) - a one-class transition renders exactly as
today. Render enum-index constants (from #3D.1) and enum-index constraints (from #3D.3)
in both the rust and c renderers and the in-process `se_to_z3`/atom builders. If profiling
shows a large-group wildcard (#3D.2) greatly affects solver time/memory at the twins'
bounds, gate transitions whose `classes.len()` exceeds a threshold behind
`--full-properties` (omitted with a per-transition warning otherwise) - otherwise enable
all by default.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_emit.rs` | change | rust + c rendering of the multi-class enabled OR + enum indices/constraints |
| `src/mc/ref_mc_smt.rs` | change | in-process multi-class enabled + enum evaluation |
| `src/codegen/backends/rust/mc.rs` | change | emitted Rust runtime: same |
| `src/codegen/backends/c/mc.rs` | change | emitted C runtime: same |

### 3D.5 - Tests (`tests/ref_mc_smt_test.rs`, `tests/smt_bounded_test.rs`)
Add pinned specs, each with a PASS and (where meaningful) a FAIL variant whose counterexample
shows the enum value rendered by name: (a) an enum-typed var with a guard comparing it to a
variant; (b) an enum-typed concrete payload field consumed by a transition; (c) a
multi-variant wildcard transition that fires on any of the group's variants (and a FAIL
variant where one variant leads to a violation); (d) an enum path in a property
(`output_has(Color(Red))` / `input_is(Color(Red))`). Pin every case on ref AND both
generated backends with byte-parity assertions. If large-group wildcard gating is
triggered, pin the omission warning + the over/under-threshold behavior.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change | in-process pins for (a)-(d) |
| `tests/smt_bounded_test.rs` | change | generated-binary pins + ref-vs-generated parity for (a)-(d); gating pins if triggered |

### 3D.6 - Docs sync
Update `docs/SEMANTICS.md` (normative enum support + the index encoding),
`docs/user/MODEL_CHECKING.md` (retire the D8 limitations; note the encoding), and `KB.md`
section 7. The full residual-differences section is task 3E's deliverable.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | normative enum support |
| `docs/user/MODEL_CHECKING.md` | change | user-facing limitations retired |
| `KB.md` | change | known-limitations update |

## Task deliverables
- smt-bounded mode (ref + both generated backends, parity-pinned) accepts and faithfully
  encodes enum-typed variables, enum payload fields, multi-variant wildcard transitions,
  enum paths in expressions, and enum-variant input/output patterns - all as
  declaration-order indices, matching exact mode.
- Any encoding that profiles as a formula-blowup (large-group wildcards) is
  `--full-properties`-gated with a per-transition omission warning; everything else is
default.
- Every v1-scope note listing the D8 rejections is updated.

## Task exit criteria
- [ ] An enum-typed var spec checks correctly (PASS + FAIL variant, CE shows the enum value by name) - ref + both backends, parity-pinned.
- [ ] An enum-typed concrete payload field is consumed correctly by a transition - parity-pinned.
- [ ] A multi-variant wildcard transition fires on any of the group's variants (PASS + a FAIL variant where one variant violates) - parity-pinned.
- [ ] Enum paths in guards/properties and enum-variant input/output patterns encode and check correctly - parity-pinned.
- [ ] Every pre-existing smt pin for a spec WITHOUT enums stays byte-identical (the index encoding is inactive for bit/bool-only specs).
- [ ] If large-group wildcard gating is triggered: the omission warning + over/under-threshold behavior are pinned; otherwise all D8 is default and no new flag surface is added.
- [ ] `cargo test --features smt` green; full checklist `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Docs synchronized (SEMANTICS.md + MODEL_CHECKING.md + KB.md).

## Non-goals
- No assumption/on_error changes - that is task 3C (though an on_error emit or assumption
  guard with an enum argument exercises this task's #4 encoding; test them together once
  both land).
- No changes to exact-mode or fuzz-mode behavior (they already support all of D8).
- No rework of example specs to USE enums - that is follow-up work; this task adds the
  capability + tests.
- No building of the MODEL_CHECKING.md residual-differences section - that is task 3E.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The enum index order MUST be declaration order of the enum's variants, identical to exact
  mode (`src/mc/ref_mc_exact.rs`), or a spec could PASS under one engine and FAIL under the
  other. Pin a spec with a multi-variant enum whose variant ORDER matters (a guard that
  distinguishes adjacent variants) to catch an ordering regression.
- Witness display: a counterexample must print the enum VARIANT NAME (`Color::Red`), not the
  raw index. Carry the variant names on the `SmtVar`/field encoding for this (the existing
  `is_bool` witness-display flag is the model - add an `enum_variants: Option<Vec<String>>`
  or equivalent).
- A `bit<1>` and a `bool` are distinct today (`is_bool`); an enum with exactly 2 variants
  encodes to `bit<1>` but must STILL display by variant name - do not conflate it with a
  bool.
- If #3D.2's per-class guard conversion makes a transition's guard differ across its classes
  (different payload fields), the `enabled_t` OR must pair each class with ITS guard - do
  not share one guard across classes.
