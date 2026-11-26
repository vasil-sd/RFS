# Phase 18, Task 3J - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.
>
> **MERGED TASK.** This is the single documentation pass for the whole Phase 18 arc. It
> subsumes the CANCELLED task 3E (residual-differences documentation overhaul - see
> `PHASE_18_TASK_3E_STATUS.md`) together with the original 3J scope (syncing the docs to
> the restrictions lifted by 3F-3H). Rationale: it is meaningless to document an
> intermediate state of the project in the middle of the refactoring - one pass, written
> AFTER all behavior changes (3A-3D closed D1-D4/D8; 3F-3H lifted V17/S3/S4/E4-S1) have
> landed, describes the final ground truth and never lists an already-lifted restriction
> as a difference.

## Goal
Make the post-refactor capability of smt-bounded mode VISIBLE and ACTIONABLE to users,
and bring every document into line with BOTH the 3A-3D unification AND the 3F-3H
restriction lifts, in ONE pass. Concretely: (a) add a dedicated **"smt-bounded vs exact
mode: residual differences"** section to `docs/user/MODEL_CHECKING.md` - one subsection
per RESIDUAL difference (D5 completeness of PASS, D6 counterexample form, the flag-gated
omissions), each giving an exact-mode-supported RFS snippet, the smt-bounded alternative,
the detailed rationale (citing SEMANTICS.md), and step-by-step instructions to alter a
spec so smt-bounded gets as close to exact mode as possible, ending with a closing summary
table; and (b) update EVERY document under `docs/user/`, `docs/dev/`, `docs/llm/` plus
`DESIGN.md` / `KB.md` / `SEMANTICS.md` so no doc contradicts the new behavior - retiring
stale "rejects assumptions/on_error/enums" claims (closed by 3A-3D) AND stale
"formula patterns bind nothing / literal-only inits / always-wrapped top forms only"
claims (lifted by 3F-3H). Rationale: the whole arc exists so a user can write a natural
spec and check it under smt-bounded with confidence; without this single documentation
pass the user cannot tell what is supported, what is not, or how to rewrite an unsupported
snippet - and a two-pass approach would document an intermediate state that 3F-3H then
invalidates.

## Context

### The full audit lives in `V1_R_R_AUDIT.md` (repo root)
Read its Table 2 rows V17, S3, S4, E4/S1 (the lifted restrictions this task documents as
gone) and the "Implementation tasks" section (this is item 3J). The cancelled task 3E's
full original scope is preserved in `PHASE_18_TASK_3E.md` + its CANCELLED status file -
consult them for the residual-differences detail this task now carries.

### Depends on 3A-3D (closed) AND 3F-3I + 3L (lifted/changed) being complete
The docs describe behavior that must exist first. Do not start 3J before 3F, 3G, 3H,
3I and 3L are done and their exit criteria met - the lifted syntax and the new
typing/conversion semantics must validate/model-check and be test-pinned BEFORE they
are documented. 3A-3D are already complete (their STATUS files record what they
closed). Execution order (user decision): 3I -> 3L -> 3J.

### What 3A-3D closed (the unification ground truth this task documents)
- **D1 CLOSED (3A):** a `next`/shifted property body is weak at trace death - an unhandled
  input that freezes the successor config no longer false-fails it. smt-bounded matches
  exact mode SEMANTICS §8.3.
- **D2 CLOSED (3A):** a top-level `bounded_eventually[N]` is unconfirmable when the path
  dies before the deadline - matching §5.5. In-process and generated encodings identical.
- **D3 SUPPORTED (3C):** assumptions work. Cheap input-guard assumptions
  (`assume always (input_is(...))`) are default; expensive liveness/temporal assumptions
  are `--full-assumptions`-gated (a flag DISTINCT from `--full-properties`, so the two
  axes are independent) with per-assumption omission warnings +
  `assumptions_omitted=M`. `--assumes all|none|labels` honored on both smt surfaces.
- **D4 SUPPORTED (3C):** `on_error` blocks work - an unmatched input takes the fallback
  (§3.6-faithful), so specs with a safe-state fallback no longer need rewriting.
- **D8 SUPPORTED (3D):** enum-typed variables, enum payload fields, multi-variant wildcard
  transitions, enum paths in expressions, and enum-variant input/output patterns all encode
  as declaration-order indices (faithful to exact mode). Large-group wildcards may be
  `--full-properties`-gated if profiling triggered it.

### What 3F-3H lifted (the restriction lifts this task documents as GONE)
These were residual differences BEFORE 3F-3H; after them they are NOT differences and
MUST NOT appear in the residual-differences section:
- **V17 LIFTED (3F):** named fields in formula `input_is` patterns are legal - a labeled
  wildcard that constrains nothing (both MC backends render `PatternExpr::Bind` as a
  no-constraint match). The old "formula input patterns do not bind variables in Phase 1"
  rejection is gone.
- **S3 LIFTED (3F):** the dead "non-power-of-two domain" SMT check is deleted (no user-
  visible change - it was unreachable).
- **S4 LIFTED (3G):** smt-bounded accepts bare boolean and top-level `next` property/
  assumption forms, desugared to `always(...)` - matching exact mode's `Range::all()`
  anchoring. The old "supports top-level always/always[N]/eventually/bounded_eventually/
  until only" rejection is gone.
- **E4/S1 LIFTED (3H):** smt-bounded accepts constant-expression var initializers
  (`init 5 + 1`), folding them to the same constant exact mode produces; a var init
  referencing another var is still rejected by BOTH modes with the identical "not a
  constant expression" message. The old "requires literal initial values" rejection is
  gone.

### Final typing/conversion rules with examples (from task 3L - record in SEMANTICS.md 2.2)
Task 3L replaces the old "integer literal adapts to the other operand's width; two bare
literals form a bit<8>" rule with an explicit, no-implicit-conversion rule set. This
table is the CONTENT of the SEMANTICS.md 2.2 rewrite (action 3J.3 records it in
normative form; LANGUAGE.md section 9 mirrors it briefly). Main principle: no implicit,
silent conversions, no silent value precision loss, no silent value truncation - all
effects visible and controllable by the user. The explicit `bit<N>(...)` cast is the
ONLY explicitness mechanism - there is no syntactic mask detection (a mask is just a
bitwise op under rule 10).

| # | Rule | Example | Result + description + rationale |
|---|------|---------|----------------------------------|
| 1 | Constant expressions (integer literals only) are full-precision integer literals - folded at compile time with signed i128 intermediates | `prop p : 200 * 200 == 40000 ;` | **OK** - folds to 40000; under the old "two bare literals form bit<8>" rule this would have been 16. Rationale: no silent precision loss in constants (3L requirement 1). |
| 2 | Constant-expression overflow is an error (never a silent wrap) | `var x: bit<8> init 18446744073709551615 * 18446744073709551615 ;` | **ERROR** "constant expression overflows" - the product exceeds i128. Rationale: no silent overflow/underflow (3L requirement 1). |
| 3 | Every arithmetic/bitwise operator requires EQUAL operand widths (stated as its own rule) | `var a: bit<2>; var b: bit<4>; set c = a + b ;` | **ERROR** "bitwidth mismatch: bit<2> vs bit<4>" - no silent widening; use an explicit cast. Rationale: 3L requirement 2 (already enforced in code; now stated explicitly). |
| 4 | Shifts are the sole width exception: result = shifted value's width | `var k: bit<3>; set x = x << k ;` (x: bit<8>) | **OK** - result is bit<8> regardless of k's width. Rationale: 3L requirement 2. |
| 5 | Shift refinement: a constant value shifted by a VARIABLE amount takes the shift amount's width | `var k: bit<3>; set x = 1 << k ;` (x: bit<3>) | **OK** - result is bit<3> (was bit<8> before this arc). Rationale: a bare literal has no width of its own; the only width present is the shift amount's (3L requirement 2). |
| 6 | Explicit conversion `bit<N>(expr)`: zero-extension or high-bit truncation, tightest binding, operand must be a bitvector; for any constant C, `bit<N>(C)` = C mod 2^N (the low N bits of its two's-complement representation) | `set x = bit<4>(y) ;` (y: bit<8>) / `set z = bit<16>(w) ;` (w: bit<8>) | **OK** - truncation to the low 4 bits / zero-extension to 16 bits; `bit<4> a + b` groups as `(bit<4> a) + b`. Rationale: 3L requirement 3 - the only way to change width. |
| 7 | Cast operand restriction: bitvectors only | `bit<4>(true)` / `bit<4>(Mode::Run)` | **ERROR** "cast operand must be a bitvector". Rationale: user decision (b) - bool/enum have their own encodings. |
| 8 | Positive constant conversion: must fit the target width or be explicitly cast | `var a: bit<4> init 3 ;` / `var a: bit<2> init (2 + 2) ;` / `var a: bit<2> init bit<2>(2 + 2) ;` | **OK** (3 fits bit<4>) / **ERROR** "constant 4 does not fit in bit<2>; use an explicit cast, e.g. bit<2>(2 + 2)" / **OK** - the explicit cast truncates 4 to 0 visibly. Rationale: user rule 1 - no silent truncation; the cast is the visible opt-in. |
| 9 | Negative constant conversion: two's-complement fit -> warning; explicit cast -> OK; else error | `var a: bit<4> init -5 ;` / `var a: bit<3> init -5 ;` / `var a: bit<3> init bit<3>(1 - 6) ;` | **WARNING** "implicit conversion of negative constant to unsigned bit-vector: -5 becomes 11 (two's complement in bit<4>)" - the value is the two's-complement encoding; the warning forces attention / **ERROR** "constant -5 does not fit in bit<3> (its two's-complement form needs 4 bits); use an explicit cast, e.g. bit<3>(1 - 6)" - abs(-5) = 5 > 2^(3-1) = 4 / **OK** - the explicit cast truncates the two's-complement form ...1011 to 3 bits: 011 = 3, no warning. Rationale: user rule 2 - programmers often use -1 for all-bits-set; familiar, but the implicit signed->unsigned step must be visible, and the cast is always a visible opt-in. |
| 10 | Bitwise operations on negative constants use the unsigned two's-complement representation of the constant (full precision) | `prop p : 2 + -3 == -1 ;` / `prop p : 2 + (-1) & 1 == 3 ;` / `prop p : 2 + (-4) & 3 == 2 ;` / `var a: bit<4> init (-5) & 15 ;` | All **OK** (the first three properties hold; the fourth validates with no warning) - `-3` folds signed (2 + -3 = -1); `(-1) & 1`: ...1111 & 1 = 1, so 2 + 1 = 3; `(-4) & 3`: ...1100 & 3 = 0, so 2 + 0 = 2; `(-5) & 15` folds to the POSITIVE constant 11, which then converts under rule 8 without any diagnostic. Rationale: user rule 3 - this is also why masking a negative constant works as an escape WITHOUT any syntactic mask detection. |
| 11 | Comparison with an out-of-range constant | `var x: bit<8>; transition ... where x == 300 { ... }` | **ERROR** - same rule as row 8 (positive out-of-range); truncating to `x == 44` would silently change meaning. Rationale: user decision (d) - "any operation" includes comparisons. |
| 12 | Init = constant expression: both modes reject identically at validation | `var x: bit<1> init 1 + 1 ;` | **ERROR** in exact AND smt-bounded with the identical message (2 does not fit bit<1>) - eliminates the task-3H divergence (exact used to carry 2 unmasked, SMT rejected). Rationale: 3L requirement 5. |

### RESIDUAL differences that REMAIN after the whole arc (the section's content)
These CANNOT be unified - they are inherent to bounded MC or deliberate performance gates:
- **D5 Completeness of PASS:** exact mode's `PASS (complete)` is an exhaustive proof over
  ALL reachable configurations; smt-bounded's `PASS WITHIN BOUND k (incomplete - no
  completeness claim)` only covers paths of length <= k. No bound makes bounded MC
  complete for an unbounded system. This is the FUNDAMENTAL difference and the reason the
  mode exists (it analyzes specs too large for exact MC).
- **D6 Counterexample form:** exact mode's counterexample is a REAL execution trace
  (every step an enabled transition, inputs concrete). smt-bounded's witness is a SOLVER
  MODEL: the state/var sequence is a valid unrolled path, but payload field values are
  solver-chosen free variables (consistent with the fired transitions, not "real" input
  data). After 3A/3C the violation-relevant PREFIX of the witness is executable (every
  step an enabled transition - or the on_error fallback), but steps past a path death in a
  spec WITHOUT on_error are frozen and non-executable, and payload values are still
  model-chosen.
- **Declared omissions (not silent divergences):** FULL properties without
  `--full-properties` are omitted with a per-property warning + `properties_omitted=M`
  (D7); expensive assumptions / large-group wildcards without their flag are likewise
  omitted with warnings. These are HONEST omissions the user can lift by passing the flag.

### The MODEL_CHECKING.md section structure (the centerpiece deliverable)
Add a NEW top-level section (number it after the existing sections, e.g. "## 10.
smt-bounded vs exact mode: residual differences" - renumber any trailing cross-reference
section and fix intra-doc anchors) with an intro paragraph stating that 3A-3D closed
D1/D2/D3/D4/D8 AND 3F-3H lifted V17/S3/S4/E4-S1, then listing what REMAINS (D5, D6,
declared omissions), then ONE SUBSECTION PER RESIDUAL DIFFERENCE. Each subsection follows
this EXACT shape so a user can scan it uniformly:
1. **The difference** - one or two sentences: what exact mode does that smt-bounded does
   not (or does differently).
2. **Exact-mode snippet** - a small, self-contained RFS snippet a user would naturally
   write and that exact mode fully supports.
3. **smt-bounded alternative** - what smt-bounded does with that snippet today: checks it
   identically (for the closed/lifted items, to reassure), omits it with a warning (for
   the flag-gated items, naming the flag that lifts the omission), or approximates it
   (for D5/D6, with the approximation stated).
4. **Rationale** - WHY the difference exists: inherent to k-step bounded unrolling (D5),
   to solver-model witnesses (D6), or a deliberate performance gate (the flag-gated
   omissions). Cite the SEMANTICS.md section.
5. **How to convert / how to get closer** - concrete, step-by-step instructions to alter
   the spec so smt-bounded's result is as trustworthy as exact mode's: e.g. for D5, "raise
   `--bound` until the verdict stabilizes across two consecutive bound increases, and run
   `--mode fuzz` as a complement; a PASS WITHIN BOUND that holds at bound k with no new
   violation found at k+1..k+m is strong (though not complete) evidence"; for D6, "add an
   `on_error` block so every input takes a real transition (the witness prefix becomes
   fully executable); read payload values in the witness as 'some consistent assignment',
   not measured data"; for a flag-gated omission, "pass the relevant flag
   (`--full-properties` for properties, `--full-assumptions` for assumptions)".

The subsections, in order: (1) D5 completeness of PASS; (2) D6 counterexample form;
(3) flag-gated FULL-property omission (D7); (4) flag-gated expensive-assumption /
large-group-wildcard omission (only if 3C/3D actually gated them - check their STATUS
files first). A short closing table summarizes: construct | exact mode | smt-bounded | how
to close the gap. This table is the quick-reference a user actually keeps open.

**CRITICAL:** because this pass runs AFTER 3F-3H, the section must NOT list S4 (top-level
forms) or E4/S1 (var inits) as differences - they are unified. If a draft subsection
starts to describe them, delete it; they belong in the intro's "closed/lifted" list only.

### Stale doc sites for the 3F-3H lifts (verified by grep + read during the audit)
1. **`docs/dev/validator.md` ~line 192** (the "Formula typing" fragment): "`input_is` /
   `output_has` delegate to `check_pattern_group` / `check_output_pattern_group` - both
   are Phase 1 stubs: formula patterns do NOT introduce bindings (a struct field with a
   name is rejected), only group/variant plausibility is checked". After 3F this is false:
   named fields are accepted and are labeled wildcards. Rewrite: formula patterns
   introduce no usable bindings - a named field is a labeled wildcard that constrains
   nothing (task 3F), mirroring transition-pattern binds which DO feed guards.
2. **`DESIGN.md` section 5.8 corner case (~line 450)**:
   "`on E(Mode::Normal)` # currently NOT supported as anonymous field pattern". STALE
   since task 3D added the unnamed-enum-variant parser arm (`parser.rs::pattern_field`,
   comment "Unnamed enum-variant field (task 3D)"). Verified: a spec with
   `transition A on E(V(Mode::Normal)) { next B; }` validates today. Fix the note to say
   the anonymous form IS supported (task 3D) and constrains the field to that variant
   without binding it.
3. **`DESIGN.md` section 5.10** (properties): the atomic-proposition list shows
   `input_is(Recv(Ack(_)))` with a wildcard. Add/adjust one line: named fields in formula
   patterns are legal and constrain nothing (a labeled wildcard) - task 3F - so
   `input_is(Recv(Ack(x)))` is equivalent to `input_is(Recv(Ack(_)))`.
4. **`docs/user/PROPERTIES.md` line ~71**: ALREADY says "Same pattern forms as transition
   patterns - wildcards and binds constrain nothing" - i.e. the user doc already describes
   the post-3F behavior (the doc/code mismatch the audit found). Verify the sentence is
   accurate as-is; change only if 3F's semantics differ from it (they should not).
5. **`docs/user/MODEL_CHECKING.md`**: after 3G/3H, smt-bounded accepts bare/`next`
   top-level properties and constant-expression var initializers - remove any statement
   that it requires `always(...)`-wrapped top forms or literal-only inits (this is folded
   into the new residual-differences section's intro "closed/lifted" list).
6. **`KB.md` section 7** (known limitations): verify no row cites a lifted restriction.
   The current rows (Spans in IR, Anonymous payloads, Ambiguity analysis, C backend
   widths, Feature flags, Error recovery) do NOT - confirm and leave unchanged unless a
   grep during execution finds one.
7. **`docs/llm/` set**: the validator card + `SRC_STRUCTURE.md` / `TEST_STRUCTURE.md`
   inventory rows for `check_pattern_group` (description text may still say "rejects named
   fields") + any `CHANGE_IMPACT.md` / `TASK_MAP.md` row naming the retired error strings
   ("do not bind variables in Phase 1", "requires literal initial values", "top-level
   always/always[N]/eventually/bounded_eventually/until ... only"). Update description
   text only - no fn/type was added or removed by 3F-3H, so row structure is stable;
   `rfs-dev inventory audit` must end EXACT.
8. **`V1_R_R_AUDIT.md`** (repo root): mark the 3F-3J task pointers as done (a short
   "Status: implemented by Phase 18 tasks 3F-3J (dates)" note under its Implementation
   tasks section) - the audit is a living record of what was removed and why.
9. **The new corpus example's README** (written by 3I): verify it states the lifted syntax
   correctly; fix wording if needed.

### The other docs that must be brought into line (the former 3E list)
Every file below currently states a limitation that 3A-3D closed, or omits a capability
that now exists. Each must be edited so it neither contradicts the new behavior nor leaves
a stale "rejected" / "unsupported" claim. (The roadmap `phase_N/` files are HISTORICAL
records - do NOT rewrite them; only the live `docs/user/`, `docs/dev/`, `docs/llm/` and
`docs/SEMANTICS.md` change.)
- `docs/user/USAGE.md` - flag tables: `--assumes` now applies to smt-bounded (remove the
  not-applicable note); the TWO flag scopes - `--full-properties` (FULL properties +
  large-group wildcards) and `--full-assumptions` (expensive assumptions), independent
  axes - per what 3C/3D actually gated.
- `docs/user/LANGUAGE.md` - any statement that smt-bounded rejects enum vars / wildcard
  groups / enum patterns is retired; the language is now uniformly checkable in both modes.
- `docs/user/PROPERTIES.md` - the property-language section: note that temporal properties
  (next / bounded_eventually / until) are checkable under smt-bounded (weak at trace death
  per 3A), with the FULL-class flag gate; plus site 4 above.
- `docs/user/FAQ.md` - the abstraction-and-assumptions Q&A: assumptions now work under
  smt-bounded (cheap default, expensive flag-gated); add/adjust a Q on "why is my
  smt-bounded PASS not 'complete'" pointing at the new MODEL_CHECKING section.
- `docs/SEMANTICS.md` - §9.9: rewrite the "v1 scope rejections" paragraph - D1/D2 closed
  (temporal obligations weak at trace death, matching §8.3/§5.5), D3/D4/D8 supported (with
  the flag-gated expensive classes), AND V17/S3/S4/E4-S1 lifted by 3F-3H; keep the D5
  incompleteness + D6 solver-model witness as the stated residual differences.
- `docs/dev/mc_engine.md` - the SMT engine description: on_error catch-all + per-step
  assumption pruning + enum index-encoding + the liveness-gated next / deadline-gated
  bounded_eventually; retire the "rejects assumptions/on_error/enums" notes.
- `docs/dev/ALGORITHMS.md` - the k-step unrolling algorithm: the alive[] liveness gate on
  temporal shifts, the on_error fallback in the next-state ITE, the per-step assumption
  base-constraints, the enum index encoding.
- `docs/dev/cli_commands.md` - flag reference: `--assumes` under smt-bounded, the TWO flag
  scopes (`--full-properties` for properties, `--full-assumptions` for assumptions), the
  omission-warning + summary-suffix behavior (`properties_omitted=M`,
  `assumptions_omitted=M`).
- `docs/dev/codegen_core.md` - if it describes the SMT spec model (`smt_plan.rs`): the new
  `SmtSpec.on_error` / `.assumptions` fields, `SmtTrans.classes`, the enum index encoding.
- `docs/dev/backend_rust.md` / `docs/dev/backend_c.md` - the emitted SMT runtime
  description: the multi-class enabled predicate, on_error fallback, assumption
  constraints, enum indices - both backends (parity).
- `docs/dev/DECISIONS.md` - if a decision record declared the smt-bounded v1 scope
  (assumptions/on_error/enums rejected), annotate it as SUPERSEDED by the Phase 18 3A-3D
  corrections (do not delete the historical record).
- `docs/llm/mc_engine.md` - LLM-facing SMT engine overview: same capability updates as
  dev/mc_engine.md, in the llm doc's style/budget.
- `docs/llm/cli_commands.md` - LLM-facing flag reference: `--assumes` + the two flag
  scopes (`--full-properties`, `--full-assumptions`) under smt-bounded.
- `docs/llm/SRC_STRUCTURE.md` - if it lists the smt_plan.rs / ref_mc_smt.rs
  responsibilities: the new fields/functions (on_error, assumptions, enum encoding,
  liveness gate).
- `docs/llm/TASK_MAP.md` - map the 3A-3J tasks to the source files they touch (so a future
  session finds the right code from the task number).
- `docs/llm/CHANGE_IMPACT.md` - the impact table: changing the SMT encoder/engine now
  affects assumption/on_error/enum specs, not just bit/bool ones.

### Machine-check gates that prove completeness
`cargo test --test dev_docs_test` (code<->doc pointers, fragment ownership, Class B
budgets), `cargo test --test docs_md_test` (user-doc markdown + mermaid validity, link
resolution), `rfs-dev sync-check` (docs<->source reference re-syncing - run it and apply
its fixes), `rfs-dev inventory audit` (string-masked completeness: zero per-file diffs).
All four must be green at exit.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3A. | Fix D1+D2 | closes the trace-death soundness gaps this task documents as resolved (complete) |
| 3C. | Implement D3+D4 | adds assumptions + on_error this task documents as supported, with the flag-gated expensive class (complete) |
| 3D. | Maximize language support | adds enums/wildcards/patterns this task documents as supported (complete) |
| 3F. | Remove obsolete validator/MC restrictions (V17 + S3) | the validator doc fragment describes the check this task retires |
| 3G. | Accept bare/`next` top-level properties in smt-bounded (S4) | MODEL_CHECKING.md must NOT list the top-form restriction as a difference |
| 3H. | Unify variable-initializer handling across MC modes (E4/S1) | MODEL_CHECKING.md must NOT list the literal-init restriction as a difference |
| 3I. | Tests for the 3F-3H changes + test-suite adoption | writes the new corpus example's README this task verifies |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Phase close-out | reads the final docs state this task produces; no change to its own scope |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/MODEL_CHECKING.md` | change | the centerpiece: new "smt-bounded vs exact mode: residual differences" section (intro naming 3A-3D closed + 3F-3H lifted, then one subsection per RESIDUAL difference D5/D6/flag-gated omissions, each with exact snippet / smt alternative / rationale / conversion instructions + a closing summary table); retire the "rejects assumptions/on_error" line and the D8 limitation notes; S4/E4-S1 must NOT appear as differences |
| `docs/user/USAGE.md` | change | flag tables: `--assumes` applies to smt-bounded; the TWO flag scopes (`--full-properties`, `--full-assumptions`) as independent axes |
| `docs/user/LANGUAGE.md` | change | retire any "smt-bounded rejects enum vars / wildcard groups / enum patterns" claim |
| `docs/user/PROPERTIES.md` | change | temporal properties checkable under smt-bounded (weak at trace death) + the FULL-class flag gate; verify site 4's binds sentence |
| `docs/user/FAQ.md` | change | assumptions Q&A updated; incomplete-PASS Q added pointing at the new section |
| `docs/SEMANTICS.md` | change | §9.9 "v1 scope rejections" paragraph rewritten to the closed (D1-D4, D8) + lifted (V17/S3/S4/E4-S1) + residual (D5, D6, declared omissions) split |
| `docs/dev/validator.md` | change | rewrite the "Formula typing" fragment's Phase-1-stub sentence (site 1) |
| `docs/dev/mc_engine.md` | change | SMT engine capability description (on_error, assumptions, enum encoding, liveness gate); retire rejection notes |
| `docs/dev/ALGORITHMS.md` | change | unrolling algorithm: liveness gate, on_error, assumptions, enum encoding |
| `docs/dev/cli_commands.md` | change | flag reference: `--assumes` + the two flag scopes + omission-warning behavior |
| `docs/dev/codegen_core.md` | change | SMT spec model fields (`SmtSpec.on_error`/`.assumptions`, `SmtTrans.classes`, enum encoding) |
| `docs/dev/backend_rust.md` / `docs/dev/backend_c.md` | change | emitted SMT runtime description (multi-class predicate, on_error, assumptions, enum indices) - both backends |
| `docs/dev/DECISIONS.md` | change | supersede-annotation on the old v1-scope decision record (if present); keep historical text |
| `DESIGN.md` | change | section 5.8 corner-case note (site 2) + section 5.10 named-field line (site 3) |
| `KB.md` | change (verify-only expected) | confirm section 7 cites no lifted restriction (site 6) |
| `docs/llm/mc_engine.md`, `docs/llm/cli_commands.md`, `docs/llm/SRC_STRUCTURE.md`, `docs/llm/TEST_STRUCTURE.md`, validator card, `CHANGE_IMPACT.md`, `TASK_MAP.md` | change | stale description text / retired error strings (site 7) + the 3A-3J task-to-source mapping |
| `V1_R_R_AUDIT.md` | change | mark 3F-3J done (site 8) |
| `examples/valid/<new>/README.md` (the 3I example) | change (verify-only expected) | correct lifted-syntax wording (site 9) |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/docs_md_test.rs` (via its file set) | change (indirect) | the edited markdown must still parse/render clean and all links resolve - the docs_md_test gate catches a broken link or malformed table introduced by this task |

## Planned actions

### 3J.1 - Write the MODEL_CHECKING.md residual-differences section
Add the new section per the Context structure: intro (3A-3D closed D1/D2/D3/D4/D8 AND
3F-3H lifted V17/S3/S4/E4-S1; what REMAINS is D5, D6, declared omissions), then
subsections for D5 (completeness of PASS), D6 (counterexample form), the flag-gated
FULL-property omission (D7), and the flag-gated expensive-assumption / large-group-
wildcard omission (only if 3C/3D actually gated them - check their STATUS files first).
Each subsection: the difference / exact-mode snippet / smt-bounded alternative /
rationale (cite SEMANTICS.md) / how to convert or get closer. End with the closing summary
table (construct | exact mode | smt-bounded | how to close the gap). Retire the stale
"rejects assumptions/on_error" and D8-limitation lines in the existing smt-bounded mode
description earlier in the file. Ensure S4/E4-S1 appear ONLY in the intro's closed/lifted
list, never as a difference subsection.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/MODEL_CHECKING.md` | change | the centerpiece section - gives the user the detailed supported/unsupported/conversion picture the arc exists to provide, written against the final post-3F-3I ground truth |

### 3J.2 - Bring the user docs into line
Edit `docs/user/USAGE.md` (flag tables), `docs/user/LANGUAGE.md` (retire enum/wildcard
rejection claims), `docs/user/PROPERTIES.md` (temporal properties checkable under
smt-bounded, weak at trace death; verify site 4's binds sentence), and `docs/user/FAQ.md`
(assumptions now work; a Q on incomplete PASS pointing at the new section). Each edit
removes a stale limitation or adds the new capability - no doc may still say smt-bounded
"rejects" assumptions/on_error/enums, nor that it needs always-wrapped top forms or
literal-only inits.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | change | flag tables reflect `--assumes` + the two flag scopes (`--full-properties`, `--full-assumptions`) |
| `docs/user/LANGUAGE.md` | change | retire the enum/wildcard/pattern rejection claims |
| `docs/user/PROPERTIES.md` | change | temporal properties under smt-bounded (weak at trace death) + the flag gate; site 4 verification |
| `docs/user/FAQ.md` | change | assumptions Q&A updated; incomplete-PASS Q added |

### 3J.3 - Bring the normative + dev docs into line
Rewrite `docs/SEMANTICS.md` §9.9's "v1 scope rejections" paragraph (D1/D2 closed,
D3/D4/D8 supported with flag-gated expensive classes, V17/S3/S4/E4-S1 lifted by 3F-3H,
D5/D6 residual). Update `docs/dev/mc_engine.md`, `docs/dev/ALGORITHMS.md`,
`docs/dev/cli_commands.md`, `docs/dev/codegen_core.md`, `docs/dev/backend_rust.md`,
`docs/dev/backend_c.md` to describe the new engine behavior (on_error catch-all, per-step
assumption pruning, enum index-encoding, liveness-gated temporal shifts). Annotate any
`docs/dev/DECISIONS.md` record that declared the old v1 scope as SUPERSEDED (keep the
historical text). Rewrite `docs/dev/validator.md`'s "Formula typing" fragment (site 1).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | normative §9.9 rewrite - the single source of truth the user docs derive from |
| `docs/dev/mc_engine.md` | change | SMT engine capability description |
| `docs/dev/ALGORITHMS.md` | change | unrolling algorithm: liveness gate, on_error, assumptions, enum encoding |
| `docs/dev/cli_commands.md` | change | flag reference |
| `docs/dev/codegen_core.md` | change | SMT spec model fields |
| `docs/dev/backend_rust.md` | change | emitted Rust runtime description |
| `docs/dev/backend_c.md` | change | emitted C runtime description |
| `docs/dev/DECISIONS.md` | change | supersede-annotation on the old v1-scope decision (if present) |
| `docs/dev/validator.md` | change (~190-196) | the stale Phase-1-stub sentence replaced with the post-3F truth |

### 3J.4 - Fix the DESIGN.md language-surface notes + KB.md check
Section 5.8: rewrite the corner-case comment so `on E(Mode::Normal)` is documented as
SUPPORTED (task 3D) - an anonymous enum-variant field pattern that constrains without
binding. Section 5.10: add the one-line note that named fields in formula patterns are
legal and constrain nothing (task 3F), with the `input_is(Recv(Ack(x)))` ==
`input_is(Recv(Ack(_)))` equivalence. Both edits keep DESIGN.md the single source of truth
for the language surface (Phase 9 principle). Verify `KB.md` section 7 cites no lifted
restriction (expected: none - leave unchanged).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (~450, section 5.8) | stale "NOT supported" corner-case note corrected |
| `DESIGN.md` | change (section 5.10 atomic-proposition list) | named-field legality noted |
| `KB.md` | change (verify-only expected) | section 7 confirmation |

### 3J.5 - Sync the LLM-assistant set + inventories
Grep `docs/llm/` for the three retired error strings and for "Phase 1 stub" / "binds
nothing" wording; update any card/inventory description that still asserts the old
behavior (site 7). Update `docs/llm/mc_engine.md`, `docs/llm/cli_commands.md`,
`docs/llm/SRC_STRUCTURE.md`, `docs/llm/TASK_MAP.md`, and `docs/llm/CHANGE_IMPACT.md` per
the Scope table, in the llm docs' style and within their token budgets (the dev_docs_test
budget gate applies - raise it only if a file genuinely grows, per the established
convention). Map the 3A-3J tasks to their source files in TASK_MAP.md so a future session
navigates from task number to code. Run `rfs-dev sync-check` (apply its fixes) and
`rfs-dev inventory audit` (must end EXACT - zero per-file diffs).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/mc_engine.md` | change | LLM-facing engine overview |
| `docs/llm/cli_commands.md` | change | LLM-facing flag reference |
| `docs/llm/SRC_STRUCTURE.md`, `docs/llm/TEST_STRUCTURE.md` | change (conditional) | description text for `check_pattern_group` / planner rows if stale + the new SMT fields/functions |
| `docs/llm/` validator card + `CHANGE_IMPACT.md` + `TASK_MAP.md` | change | retired error strings / old-behavior wording + 3A-3J task-to-source mapping |

### 3J.6 - Close the audit record + verify the example README
In `V1_R_R_AUDIT.md`, under "Implementation tasks (Phase 18 plan corrections)", append a
status line: tasks 3F-3J implemented (with completion dates from their status files), so
the audit reads as a closed loop from finding to removal to documentation. Verify the new
corpus example's README (written by 3I) states the lifted syntax correctly; fix wording if
needed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `V1_R_R_AUDIT.md` | change | task pointers marked done - the audit closed out as a completed loop |
| `examples/valid/<new>/README.md` (the 3I example) | change (verify-only expected) | correct lifted-syntax wording |

### 3J.7 - Run the doc gates + full matrix
`cargo test --test dev_docs_test && cargo test --test docs_md_test && rfs-dev sync-check
&& rfs-dev inventory audit`, then the standard `cargo build && cargo test && cargo clippy
--all-targets && cargo fmt` to prove no doc edit broke a pointer, link, table, or budget.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `docs/user/MODEL_CHECKING.md` carries the new "smt-bounded vs exact mode: residual
differences" section: intro naming 3A-3D closed + 3F-3H lifted, then one subsection per
RESIDUAL difference (D5, D6, flag-gated omissions) each with exact-mode snippet / smt
alternative / rationale / conversion instructions, plus a closing summary table. S4/E4-S1
appear only in the intro's closed/lifted list.
- No document under `docs/user/`, `docs/dev/` or `docs/llm/` still claims smt-bounded
  "rejects" assumptions, on_error, enum vars, wildcard groups, or enum patterns; nor that
  it needs always-wrapped top forms or literal-only inits; the flag-gated expensive classes
  are described with their flags and omission warnings.
- `docs/SEMANTICS.md` §9.9 is the normative statement of what is closed (D1-D4, D8), what
  is lifted (V17/S3/S4/E4-S1), and what remains (D5, D6, declared omissions).
- `DESIGN.md` sections 5.8/5.10 document the anonymous enum-variant pattern (task 3D) and
  named-field formula patterns (task 3F) correctly.
- `V1_R_R_AUDIT.md` closed out as a completed audit.
- All four doc gates green (dev_docs_test, docs_md_test, sync-check, inventory audit).

## Task exit criteria
- [ ] MODEL_CHECKING.md has the new residual-differences section with a subsection per
      RESIDUAL difference (D5 completeness, D6 counterexample form, flag-gated omissions),
      each containing an exact-mode snippet, the smt-bounded alternative, the rationale
      (citing SEMANTICS.md), and concrete conversion/get-closer instructions; a closing
      summary table is present.
- [ ] S4 (top-level forms) and E4/S1 (var inits) appear ONLY in the section's intro
      closed/lifted list - NOT as difference subsections (they are unified by 3G/3H).
- [ ] No file under docs/user/, docs/dev/ or docs/llm/ still states that smt-bounded
      rejects assumptions, on_error blocks, enum-typed variables, multi-variant wildcard
      transitions, or enum patterns; nor that it requires always-wrapped top forms or
      literal-only inits (grep for the old rejection phrasing returns only historical
      roadmap files).
- [ ] `grep -rn "do not bind variables\|Phase 1 stub" docs/ DESIGN.md KB.md` returns
      nothing (the old V17 behavior wording is gone from every live doc).
- [ ] `grep -rn "requires literal initial values\|top-level always/always" docs/` returns
      nothing outside the phase-18 task/plan history files (those are append-only record
      and stay as-is).
- [ ] DESIGN.md section 5.8 documents `on E(Mode::Normal)` as supported; section 5.10
      notes named-field formula patterns constrain nothing.
- [ ] The flag-gated expensive classes (FULL properties; expensive assumptions and/or
      large-group wildcards if 3C/3D gated them) are documented with their flag and
      omission-warning behavior in USAGE.md, MODEL_CHECKING.md, cli_commands.md (dev + llm).
- [ ] SEMANTICS.md §9.9 "v1 scope rejections" paragraph is rewritten to the closed /
      lifted / residual split.
- [ ] `cargo test --test dev_docs_test`, `cargo test --test docs_md_test`,
      `rfs-dev sync-check`, `rfs-dev inventory audit` all green (inventory EXACT).
- [ ] Full matrix (`cargo build && cargo test && cargo clippy --all-targets && cargo fmt
      --check`) still green after the doc edits.

## Non-goals
- No source-code change - documentation only. A suspected code/doc mismatch is recorded as
  a task correction, not silently resolved.
- No rewriting of historical roadmap files (`docs/roadmap/phase_N/...`) - they are records
  of what was done, including the old limitations that motivated 3A-3D and the cancelled
  3E. The 3E task file + its CANCELLED status stay as-is.
- No new examples or test legs - those are task 3I; this task only documents (and verifies
  3I's example README).
- No change to the exact-mode or fuzz-mode documentation beyond cross-references to the new
  section.
- No intermediate-state documentation: this is the ONE documentation pass for the whole
  arc, written after all behavior changes land. Do NOT document S4/E4-S1 as differences -
  they are lifted.
- No training-set changes (3I owns those; the docs do not feed the SFT set directly).

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| Merged with cancelled task 3E | This task file was rewritten (2026-11-24) to subsume the entire scope of the CANCELLED task 3E (residual-differences documentation overhaul). The former 3J scope (syncing the 3F-3H lifted restrictions) is retained and combined with 3E's scope into a single end-of-arc documentation pass. Rationale: documenting an intermediate state mid-refactoring is meaningless - one pass after all behavior changes (3A-3D closed, 3F-3H lifted) describes the final ground truth and avoids re-editing the same files twice. See `PHASE_18_TASK_3E_STATUS.md` for the cancellation record and the plan-corrections `remove` (3E) + `change` (3J) rows in `PHASE_18_PLAN.md`. |
| DEPENDENCIES.md added to scope (execution finding) | The exit-criterion grep `grep -rn "do not bind variables\|Phase 1 stub" docs/ DESIGN.md KB.md` caught three stale sites in `docs/dev/DEPENDENCIES.md` (the prose "both are Phase 1 stubs (formula patterns do not bind variables yet)" + two mermaid node labels) that were NOT in this task file's site list. Fixed per the task's own rule for out-of-scope sync fixes (apply + record, do not silently expand scope). |
| Sample-0 subsection 10.5 added beyond the four planned | Task 3C's status explicitly nominated the sample-0 assumption divergence (vacuous PASS under smt-bounded vs "violated at initial configuration" rejection under exact mode) as "a candidate for the MODEL_CHECKING.md residual-differences section (task 3E)". The merged 3J carries 3E's scope, so the divergence is documented as subsection 10.5 with a verified snippet and the `always (next (input_is(...)))` workaround - an addition to the planned D5/D6/flag-gated-omission subsections, not a deviation from them. |
| SRC_STRUCTURE.md rephrased for the verbatim grep | Two legitimate task-3C inventory descriptions contained the literal string "top-level always/always" that the exit-criterion grep targets (they describe the NEW behavior - outer wrappers dropped by the per-sample assertion loop - not the old S4 rejection). Rephrased to "outer always / always[N] wrappers dropped/DROPPED" so the verbatim grep passes; meaning unchanged, no row structure touched. |
| DESIGN.md 5.8 corner-case form corrected | The old doc line showed `on E(Mode::Normal)` as the (then-unsupported) anonymous form. Re-verification during execution found the bare form does NOT parse at all ("trailing input after RFS source") - the constructor name is required: `on E(V(Mode::Normal))` validates (task 3D's parser arm). The corrected note documents the constructor-required supported form and adds the anonymous `Enum::Variant` row to the pattern-field table. |

## Other information
- Check the 3C and 3D STATUS files BEFORE writing the flag-gated-omission subsections: if
  3C did not gate expensive assumptions (all default) or 3D did not gate large-group
  wildcards, those subsections collapse into the "supported by default" statement - do not
  document a flag that does not exist.
- The closing summary table is the highest-value artifact: a user deciding "can I check my
  spec under smt-bounded, and how close will it be to exact mode?" reads that table. Keep
  every row's "how to close the gap" cell actionable (a flag, a bound increase, an on_error
  addition, or "inherent - use fuzz as a complement").
- The D5 conversion guidance must NOT overclaim: bounded MC is never complete. Phrase it as
  "stronger evidence with a larger bound + fuzz complement", never "equivalent to exact
  mode".
- Renumbering: if the new section is inserted before an existing numbered section (e.g. the
  cross-references section), renumber the trailing sections and fix any intra-doc anchors
  that reference them - docs_md_test catches a dangling anchor.
- The Class B line budgets in `docs/llm/` are machine-checked (dev_docs_test): if an edit
  pushes a card over budget, trim elsewhere in the same card - do not raise the budget
  without recording it as a task correction.
- If `rfs-dev sync-check` reports fixes in files outside this task's scope (e.g. a stale
  reference introduced by an earlier phase), apply them and record them as findings in the
  status file rather than expanding this task's stated scope silently.
