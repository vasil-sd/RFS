# Phase 18, Task 4A - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Two deliverables from task 4's pattern findings: (1) a MEANINGFUL parser error for
anonymous event payloads (`event E(x: bit<8>)` - audit row P1), which today fails with
the misleading "trailing input after RFS source"; and (2) a RESEARCH record of every
pattern class accepted in transition definitions and in `input_is(...)` /
`output_has(...)` formula patterns, plus a table of proposals to extend patterns toward
payload-agnostic forms (e.g. `input_is(Group(_))` meaning "any variant"), each with an
effort estimate. Rationale: the confusing error cost real debugging time in task 4, and
the pattern-class map is the prerequisite for any future pattern-extension decision.

## Context
- **The parser failure mechanism (verified by reading `src/lang/parser.rs`):**
  `event_decl` -> `opt_constructor_payload` sees the `(` and calls `constructor_variant`,
  which parses `x` as a UNIT-VARIANT name; the following `:` then breaks the `)`
  expectation, `event_decl` fails, and top-level recovery swallows the failure into
  "trailing input after RFS source" (`parse_source`, parser.rs ~line 81). This is the
  SAME failure class as `eventually p` (missing parens), which already has a dedicated
  post-hoc diagnostic: `find_eventually_missing_paren` in `parse_source` reports the
  specific error with its exact position when the file fails AND contains the pattern.
- **The `Group(_)` trap (task 4 finding, mechanism now confirmed):** in FORMULA patterns,
  `group_pattern` (parser.rs ~line 735) parses `_` as a unit-variant NAME (`ident_start`
  accepts `_`), producing `PatternVariant::Unit { name: "_" }` - the matcher then looks
  for a variant literally named `_`, finds none, and NEVER matches. Under an assumption
  this makes the obligation unsatisfiable -> false deadlock at depth 6 (task 4's B9
  incident). In TRANSITION definitions the same form is REJECTED at validation:
  "unknown event variant '_' in group 'Wake'" (`validator.rs` ~line 386, the V7 check in
  `validate_event_pattern`) - a cross-surface asymmetry: formulas accept what transitions
  reject, then silently misbehave.
- **Planning-stage probes (already run, results pinned here):** bare `input_is(Group)`
  and `output_has(Group)` parse AND match every variant of the group (the "event name
  only" form ALREADY WORKS - q1/q2/q3/q4 probes); `Group(V(_))` wildcard-field and
  `Group(V(literal))` constraint forms work in formulas (q5/q6 probes); `on Group(_)` in
  a transition is rejected at validation (q7 probe).
- **Pattern grammar locations:** transitions parse their pattern via the same
  `event_pattern`/`group_pattern`/`pattern_field_list`/`pattern_field` functions as
  formulas (`input_is` -> `event_pattern`, `output_has` -> `output_pattern`, both
  delegating to `group_pattern`). Field patterns: `_` wildcard, `name` bind, integer /
  boolean literal, `Enum::Variant` path, named `name: <pattern>` (bind while
  constraining). Formula-level binds are LABELED WILDCARDS since task 3F (V17 lifted -
  the bound name constrains nothing; both MC backends render `Bind` as no-constraint).
- **Effort-estimate scale** (for the proposals table): OBSOLETE-class = small deletion /
  one-function change with tests; MODERATE = parser + IR/validator + both MC engines +
  docs, a bounded multi-file change; HARD = new matcher semantics or grammar redesign.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | Control FSMs B9-B15 | source of both findings (the confusing parser error hit during spec authoring; the `Group(_)` false deadlock hit in B9's assumption) |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/lang/parser.rs` | change | dedicated anonymous-payload diagnostic (the `find_eventually_missing_paren` precedent) + unit tests |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/roadmap/phase_18/tasks/PHASE_18_TASK_4A.md` | change (append) | the two research tables land in this file's "Other information" section during execution |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/lang/parser.rs` inline tests | change | pins for the new error message (event + output forms, exact position) |

## Planned actions

### 4A.1 - Meaningful error for anonymous event payloads
Add a dedicated diagnostic for the anonymous-payload shape, following the EXISTING
`find_eventually_missing_paren` precedent in `parse_source`: when the file fails to
parse AND contains an `event <Name>(<ident> :` (or `output <Name>(<ident> :`) shape -
i.e. a field declaration directly under the group name with no variant constructor -
report at that exact position: "anonymous event payloads are not supported - name the
variant: event E(E(x: bit<8>))" (and the `output` analog). Rationale: the generic
"trailing input after RFS source" names neither the construct nor the fix; the
eventually-parens case proved this post-hoc-scan pattern is the established remedy for
failures that top-level recovery swallows. Alternative considered and rejected: a direct
check inside `constructor_variant` (where `identifier` succeeds but the next char is `:`)
- cleaner in isolation, but the swallowed-failure behavior shows the error does not
propagate from that depth, so the scan-in-parse_source approach is the reliable one.
Unit tests: the event form, the output form, exact message + position pins, and a
negative control proving `event E(E(x: bit<8>))` still parses.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/parser.rs` | change | new `find_anonymous_payload` scan + the error branch in `parse_source`, mirroring `find_eventually_missing_paren`; inline unit tests for both decl kinds + position pin + the named-variant negative control |

### 4A.2 - Pattern-class survey (table 1)
Enumerate EVERY pattern class accepted in (a) transition definitions (`on ...`) and
(b) formula patterns (`input_is(...)` / `output_has(...)`), recording the table
| <pattern> | <example> | <description + rationale> |. Method: read `group_pattern` /
`pattern_field_list` / `pattern_field` (the shared grammar) plus the matchers
(`ref_mc.rs::input_matches` / `input_matches_variant`, `smt_plan.rs::resolve_input_is`)
to state for each class whether it parses, what it means, and how it matches; then grep
`examples/valid/`, `crates/examples/*/pass.rfs`, and the test suites for real uses of
each class (cite the example). The survey MUST cover at minimum: bare group; unit
variant; struct variant with `_` wildcard fields; struct variant with `name` binds
(labeled wildcards since 3F); integer / boolean literal field constraints (V8a fit
rule); `Enum::Variant` path field patterns; named `name: <pattern>` fields; and the
`Group(_)` trap (parses in formulas as a phantom unit variant named "_", never matches;
rejected at validation in transitions - the asymmetry). Append the finished table to
this file's "Other information" section.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_18/tasks/PHASE_18_TASK_4A.md` | change (append) | table 1 in "Other information" - the durable record of the accepted pattern surface |

### 4A.3 - Extension proposals with effort estimates (table 2)
From table 1's gaps, propose ways to extend patterns toward payload-agnostic forms -
the user's goal: leave only the event name in a pattern, without payload mention.
Record the table | <proposal> | <description + rationale> | <effort estimate> |, covering at
minimum: (P1) bare `input_is(Group)` / `output_has(Group)` - ALREADY WORKS (planning
probes q1-q4): proposal = document it prominently in PROPERTIES.md/LANGUAGE.md as THE
payload-agnostic form (docs-only, OBSOLETE-class effort); (P2) make formula-level
`Group(_)` mean "any variant" (parser: treat `_` in variant position as a group-level
wildcard -> `PatternVariant::None`) - closes the trap by giving the natural shorthand
its intended meaning; soundness argument required (nothing can rely on the phantom
variant since it never matches); (P3) reject formula-level `Group(_)` at validation with
the transition-side error ("unknown event variant '_'") - the asymmetry-closing
alternative to P2, zero matcher change; (P4) any further shorthand the survey surfaces
(e.g. `Group(Variant(_))` already works - record it). For each: exact files touched
(parser / validator / both MC matchers / docs), test plan, and an effort estimate on
the Context scale (OBSOLETE-class / MODERATE / HARD). NO proposal is implemented in this
task - the table is the decision input for a future scheduling task. Append the finished
table to this file's "Other information" section.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/phase_18/tasks/PHASE_18_TASK_4A.md` | change (append) | table 2 in "Other information" - proposals + effort estimates, the input to future scheduling |

## Task deliverables
- The anonymous-payload parser error: meaningful message naming the fix, exact position,
  unit-tested (event + output forms).
- Table 1 (pattern classes) and table 2 (extension proposals + effort estimates) appended
  to this task file's "Other information" section.

## Task exit criteria
- [ ] `rfs validate` on a spec with `event E(x: bit<8>)` reports the anonymous-payload error naming the variant-naming fix at the exact position (not "trailing input after RFS source"); same for `output O(v: bit<8>)`; `event E(E(x: bit<8>))` still parses.
- [ ] Table 1 covers every pattern class of the shared `group_pattern` grammar, each with a real example from the corpus or an explicit "no corpus use" note, and states parse/meaning/match behavior per surface (transition vs formula).
- [ ] Table 2 carries at least the P1-P4 proposals above, each with description + rationale + effort estimate; no proposal is implemented.
- [ ] Both tables appended to this file's "Other information" section.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green (the parser change must move no existing pin - all 50+35 example specs keep validating identically).

## Non-goals
- No IMPLEMENTATION of any pattern extension (P2/P3/etc. are proposals only - a future
  task schedules the chosen one).
- No matcher/semantics change beyond the error message: every currently-valid spec keeps
  parsing and model-checking byte-identically.
- No changes to `examples/`, training sets, or the docs/user set (documenting the bare
  group form is P1's content - a future docs task or 8A's territory).

## Other information

### Execution record (task 4A, 2026-11-27)

**4A.1 (parser error) - DONE.** `find_anonymous_payload` + two helpers (`whole_word_at`,
`anonymous_field_offset`, `skip_ws_bytes`) added to `src/lang/parser.rs`, wired into
`parse_source` after the `find_eventually_missing_paren` branch. `event E(x: bit<8>)`
now reports `anonymous event payloads are not supported - name the variant: event
E(E(x: bit<8>))` at the field identifier's offset; the `output` analog is symmetric.
Three unit tests pin message + position + the named-variant negative control. Verified
end-to-end: both anonymous forms give the dedicated error (EXIT 1), `event E(E(x:
bit<8>))` still validates (EXIT 0).

**4A.2 / 4A.3 - tables below.** Every claim was checked against the source (`parser.rs`
`group_pattern`/`pattern_field_list`/`pattern_field`; `ref_mc.rs::input_matches` /
`input_matches_variant`/`output_matches`; `smt_plan.rs::resolve_input_is`; `validator.rs`
`validate_event_pattern` vs `check_pattern_group`) and against the corpus (a
paren-balanced scan of all 214 `.rfs` files) plus fresh `mc-ref --mode exact` runs.

---

### Table 1 - Pattern classes accepted in transition definitions and formula patterns

The shared grammar (`group_pattern` -> `pattern_field_list` -> `pattern_field` /
`pattern_expr`) is used by BOTH surfaces: a transition's `on <pattern>` and a formula's
`input_is(<pattern>)` / `output_has(<pattern>)`. The matchers agree field-for-field
(`ref_mc.rs::input_matches` == `input_matches_variant`; `smt_plan.rs::resolve_input_is`
mirrors them). The ONE structural difference is validation: transitions run
`validate_event_pattern` (V7 variant-existence, field-count, V8a literal-fit,
duplicate-bind); formula patterns run `check_pattern_group`, which is a **no-op** -
formula patterns are entirely unvalidated. That asymmetry is what makes the last row
possible.

| <pattern> | <example> | <description + rationale> |
|-----------|-----------|----------------------------|
| Bare group `Group` | `input_is(Wake)` / `on Wake` (182 formula uses; 171 transition files, e.g. `03_timeout_retry.rfs` `on Send`, `on Ack`, `on Tick`) | Matches **every** variant of the group - the matcher's `None => true` arm. No variant name, no fields: the group identity is the only constraint. This is THE payload-agnostic form and it ALREADY WORKS on both surfaces (probes q1-q4). |
| Unit variant `Group(V)` | `input_is(Recv(Syn))` / `on Recv(Syn)` (`04_tcp_handshake_like.rfs`, `41_property_monitors.rfs`; 16 formula uses) | Matches only the unit variant named V (`vi.variant == name`). Names a specific payload-less variant. |
| Struct, `_` wildcard fields `Group(V(_, ...))` | `input_is(E(V(_, _, _, 1, _, _)))` (`plan.rs` test); transitions use the same field grammar | Matches struct variant V with the `_` positions unconstrained (`Wildcard => true`). `_` means "any value here". No shipped-example uses `_` in a FORMULA; it is a test-suite form. |
| Struct, `name` binds `Group(V(x, y))` | `input_is(Job(J(n)))` (`51_lifted_syntax.rfs`), `input_is(M06(M(t)))` (`cnc_tool_change`); transitions: `on Temp(T(n))`, `on Wake(W(src))` (the dominant payload form) | A bare identifier in field position is a **labeled wildcard** since task 3F (V17 lifted): in FORMULAS the bound name constrains nothing and is not referenceable - both backends render `Bind` as no-constraint. In TRANSITIONS the same bind IS usable in guards/assignments/emits. Same syntax, different power per surface. |
| Struct, integer literal `Group(V(5))` / `Group(V(_, 3))` | `input_is(E(V(1, _)))` (`plan.rs:1125`); the `1`/`0` in `plan.rs:825-826` | Constrains a numeric field to an exact value. **V8a fit rule** (transition-side only): the literal must fit the field's bit width (`literal N does not fit bit<W>`); an int cannot match a bool or enum field. Test-suite form; no shipped-example use. |
| Struct, boolean literal `Group(V(true))` / `Group(V(_, false))` | `input_is(Go(F(_, false)))` (`ref_mc_smt_test.rs:554`) | Constrains a bool field to an exact value; a bool literal cannot match a non-bool field. Test-suite form. |
| Struct, `Enum::Variant` path `Group(V(Color::Red))` | `output_has(Painted(P(Color::Red)))`, `input_is(Go(G(Color::Green)))` (`ref_mc_smt_test.rs`) | Task 3D: a qualified `Type::Variant` path constrains an enum field to that variant **without binding it** (the matcher compares the whole qualified value). Test-suite form. |
| Struct, named `name: <pattern>` `Group(V(x: 5))` / `Group(V(x: _))` | *no corpus or test use found* | The grammar's `pattern_field` `:` arm binds a name AND constrains it to `<pattern>` (which may itself be `_`, an int/bool literal, or an enum path). The only form that both names and constrains a field; in practice subsumed by a separate bind + a literal. Documented here for completeness - the accepted surface includes it even though nothing ships it. |
| Struct, empty field list `Group(V())` | `output_has(O0(S0()))` (`specgen_mutation_loop`, 6 uses, all specgen-generated) | A struct variant with zero fields; matches only variant V. Degenerate - equivalent to a unit match on V. |
| **The `Group(_)` trap** `input_is(Group(_))` | *no correct corpus use* - it is the bug (task 4 B9 incident) | In FORMULAS, `_` in VARIANT position is parsed by `group_pattern` as a **unit-variant name** (`ident_start` accepts `_`) -> `PatternVariant::Unit { name: "_" }`. The matcher then looks for a variant literally named `_`, finds none, and **never matches**. Under a requiring assumption this is unsatisfiable -> false deadlock. In TRANSITIONS the same form is REJECTED at validation (V7: `unknown event variant '_' in group '...'`). Cross-surface asymmetry: formulas accept what transitions reject, then silently misbehave. Root cause: `_` means "wildcard" at the FIELD level but "unit-variant name" at the VARIANT level - the two readings collide. |

**Empirical confirmation (fresh `mc-ref --mode exact` runs this session):** a spec whose
only difference is `input_is(Wake)` vs `input_is(Wake(_))` under the same requiring
assumption `always (bounded_eventually[3] (input_is(...)))` gives `status=pass-complete`
(EXIT 0) for the bare group and `deadlock: configuration at depth 1 has no enabled
transition` (EXIT 1) for the phantom - reproducing the B9 false deadlock exactly. A
separate run confirmed a unit variant CAN be declared literally named `_`
(`event E(_)` -> generated input `Input::E__`) and IS matched by both `on E(_)` and
`input_is(E(_))` - so the "phantom" is only phantom when no `_` variant exists (this
caveat feeds P2 below).

---

### Table 2 - Extension proposals toward payload-agnostic patterns (+ effort estimates)

Goal (from the task): leave only the event name in a pattern, without payload mention.
Effort scale (from Context): **OBSOLETE-class** = small deletion / one-function change
with tests; **MODERATE** = parser + IR/validator + both MC engines + docs, a bounded
multi-file change; **HARD** = new matcher semantics or grammar redesign. **No proposal
is implemented in this task** - this table is the decision input for a future scheduling
task.

| <proposal> | <description + rationale> | <effort estimate> |
|------------|----------------------------|-------------------|
| **P1 - Document the bare-group form as THE payload-agnostic pattern.** | `input_is(Group)` / `output_has(Group)` already parse and match every variant (probes q1-q4; 182 corpus uses). No code change is needed to "leave only the event name" - the form exists. Action: document it prominently in PROPERTIES.md / LANGUAGE.md as the recommended payload-agnostic idiom. Per this task's non-goals the writing itself is deferred to a docs task / 8A; P1 records that nothing must be built. | OBSOLETE-class (docs-only, zero code). |
| **P2 - Reinterpret formula-level `Group(_)` as "any variant" (`PatternVariant::None`).** | Parser change in `group_pattern`: when the variant-position identifier is `_`, emit `PatternVariant::None` instead of `Unit { name: "_" }`. Gives the natural shorthand its intended meaning and closes the trap at the source. **Soundness:** (a) when no variant is named `_`, `Group(_)` matches nothing today, so every current use is already a bug (false deadlock under assumptions, vacuous under props) - reinterpreting fixes it and no correct spec loses behavior; (b) **caveat** - a unit variant CAN be declared literally named `_` (`event E(_)` -> `Input::E__`, verified), and today `Group(_)` matches THAT variant specifically; P2 would broaden it to all variants, changing that degenerate spec. A fully-sound P2 applies the reinterpretation only when no `_` variant exists, or also forbids declaring `_` variants. Files: parser (`group_pattern`) + a validator guard for the caveat + docs; **no matcher change** (`None` already matches all). | OBSOLETE-class if the caveat is ignored (one parser branch + tests); MODERATE once the declared-`_`-variant guard is added to keep it sound. |
| **P3 - Reject formula-level `Group(_)` at validation (close the asymmetry the other way).** | Make `check_pattern_group` run the same V7 variant-existence check as transitions, so `input_is(Group(_))` fails validation with `unknown event variant '_' in group '...'` instead of silently never matching. Zero matcher change; turns a silent runtime trap into an early, actionable error. Rationale: cheaper than P2 and no behavior reinterpretation, but it forces authors to the bare-group form (P1) for "any variant" rather than teaching `_` the meaning they expect. Files: validator (`check_pattern_group`) + tests. | OBSOLETE-class (one function, mirrors the existing transition check). |
| **P4 - Record `Group(Variant(_))` wildcard-field as the constrained-any-value form.** | Already works in formulas (probe q5; `plan.rs` tests) and transitions: name the variant, wildcard the fields you do not care about. No implementation needed - document alongside P1 so authors know the two payload-agnostic tiers (bare group = any variant; `V(_)` = this variant, any field values). | OBSOLETE-class (docs-only). |
| **P5 (surfaced by the survey) - Validate formula patterns like transitions.** | The survey shows formulas skip ALL pattern validation (`check_pattern_group` is a no-op), so `input_is(Group(V(wrong_field_count)))` and non-fitting literals (V8a) silently never match - the SAME failure class as the `Group(_)` trap, just broader. Proposal: run the transition-side checks (variant existence, field count, literal fit) on formula patterns too, while preserving the labeled-wildcard/no-bind rule so it does not over-reject. Rationale: eliminates the whole silent-never-matches family in one move rather than only the `_` instance; subsumes P3. Files: validator (reuse `validate_event_pattern`'s field checks for formulas) + tests. | MODERATE (reuse the existing validation path for a second surface; must special-case the no-bind formula semantics). |

**Recommendation shape for the future scheduling task:** P1+P4 are pure documentation
and can ship immediately with 8A. P2 and P3 are mutually exclusive fixes for the same
trap - P2 teaches `_` the meaning authors expect (with the declared-`_` caveat), P3
rejects it early; pick one. P5 is the generalization that makes both unnecessary by
closing the validation gap outright.
