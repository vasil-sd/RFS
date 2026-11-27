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
(empty until execution appends tables 1 and 2.)
