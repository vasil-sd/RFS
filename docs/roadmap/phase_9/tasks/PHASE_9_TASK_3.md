# Phase 9, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Merge the RFS v0 syntax reference (KB section 3) into DESIGN section 5 so that
DESIGN.md alone describes the language surface - actualizing DESIGN where KB
is newer (version header, `on_error`, priorities, literal patterns) and fixing
DESIGN 5.9's wrong expression-precedence list against the verified parser
chain. KB section 3 becomes a reference stub. Rationale: the syntax reference
is core language design; the two copies had already diverged (and one of them
states a precedence order the parser does not implement).

## Context
Verified at planning by reading KB 3.1-3.9 and DESIGN 5.1-5.10 in full:
- **KB is newer on**: the optional `version "X.Y[.Z]"` module header (KB 3.1 -
  DESIGN 5.1 shows only `module name { ... }`); the `on_error` fallback
  (KB 3.1a - absent from DESIGN 5 entirely); transition `priority N` + the
  Phase 2 priority semantics (KB 3.5 - DESIGN 5.8's general form has no
  priority action); literal patterns `on Cmd(Value(0))` / `on Flag(Value(true))`
  and the `on E(Mode::Normal)` NOT-supported corner case plus the pattern-field
  table (KB 3.6 - DESIGN 5.8 only shows wildcard/bind/named forms and says
  literal patterns "may be supported"); the anonymous-payload limitation
  (`event E(x: bit<8>)` NOT supported - KB 3.3, absent from DESIGN 5.5).
- **DESIGN is newer on**: property operators + monitor semantics (DESIGN 5.10
  covers `always[N]`, `bounded_eventually[N]`, `eventually`, WEAK `until`,
  D-P3-3 deadline semantics, R1-R4 - KB 3.9 only has examples + a pointer).
  Keep DESIGN 5.10 as-is.
- **PRECEDENCE CONFLICT (must fix)**: KB 3.8's table matches the ACTUAL parser
  chain in `src/parser.rs` - verified function-by-function at planning:
  `expr -> logic_or -> logic_and -> equality -> relational -> bitwise_or ->
  bitwise_xor -> bitwise_and -> shift -> additive -> multiplicative ->
  unary_expr -> primary_expr`, and `multiplicative` contains NO `/` operator
  (grep-verified: zero occurrences). DESIGN 5.9's list is WRONG: it places
  `<< >>` TIGHTER than `* /` and lists a nonexistent `/`. Replace DESIGN 5.9's
  precedence block with KB 3.8's table (tightest-first: 1 unary `! ~ -`; 2
  `*`; 3 `+ -`; 4 `<< >>`; 5 `&`; 6 `^`; 7 `|`; 8 relational `< <= > >=`; 9
  equality `== !=`; 10 `&&`; 11 `||`), keeping KB's note that relational binds
  tighter than equality and that this is pinned against the parser (re-verify
  the chain at execution time before writing - it is the source of truth).
- **Cross-reference stability**: keep the KB heading `## 3. RFS v0 syntax
  reference` (live docs cite "KB.md section 3" - though the training-count
  citations are wrong and fixed by task 5, the heading must survive this task).
- DESIGN 5's subsection numbering: fold the new material into existing
  subsections where possible (5.1 source file + version header; 5.2 module
  items + `on_error`; 5.5 events + limitation; 5.8 transitions + patterns +
  priorities) to avoid renumbering. If `on_error` needs its own subsection,
  insert it as 5.2a-style text inside 5.2 (no new number).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | KB->DESIGN dedup: purpose/semantics/constraints/diagram | edits the same two files - sequential execution avoids conflicting edits and keeps DESIGN's section flow coherent |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | KB->DESIGN dedup: IR/parser/property/codegen | same files, next in the chain |
| 5. | KB.md stale-section cleanup | needs the final KB stub structure |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | section 5 actualized: version header + on_error (5.1/5.2), anonymous-payload limitation (5.5), priorities + literal patterns + pattern table + corner case (5.8), precedence list REPLACED with the verified parser chain (5.9) |
| `KB.md` | change | section 3 body -> reference stub to DESIGN section 5 (+ SEMANTICS.md), heading kept |

## Planned actions

### Action 1: Re-verify the parser precedence chain
Read the expression functions in `src/parser.rs` (`expr`, `logic_or`,
`logic_and`, `equality`, `relational`, `bitwise_or`, `bitwise_xor`,
`bitwise_and`, `shift`, `additive`, `multiplicative`, `unary_expr`,
`primary_expr`) and confirm the chain order and the absence of `/`. Rationale:
the precedence table being written into DESIGN.md must match the implementation
- this is the whole point of fixing 5.9.

#### Affected files

None (read-only verification).

### Action 2: Merge KB 3.1/3.1a/3.3 into DESIGN 5.1/5.2/5.5
Add to DESIGN 5.1 the optional version header (`module name version "1.2.3"
{ ... }`, quoted string without escapes, round-trips through `Module.version`,
becomes `SPEC_VERSION` in generated code - per KB 3.1 and D7 in DESIGN section
20). Add to DESIGN 5.2 the `on_error` fallback (KB 3.1a: at most one per
module, `next` mandatory, executed in any state when no transition matched, no
pattern bindings, variable values carry over; without it behavior is
`StepError::UnexpectedEvent` / `RFS_UNEXPECTED_EVENT`). Add to DESIGN 5.5 the
anonymous-payload limitation (KB 3.3: `event E(Value(x: bit<8>))` supported,
`event E(x: bit<8>)` NOT - payload constructors require a named variant).
Rationale: these are shipped language features (Phase 2) that DESIGN's v0
subset description never received.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (5.1, ~lines 179-195) | add the optional version header form |
| `DESIGN.md` | change (5.2, ~lines 196-210) | add the on_error fallback item + rules |
| `DESIGN.md` | change (5.5, ~lines 243-274) | add the anonymous-payload limitation note |

### Action 3: Merge KB 3.5/3.6 into DESIGN 5.8
Extend DESIGN 5.8's transition general form with the `priority N;` action and
the Phase 2 priority semantics (KB 3.5: within one state + identical event
pattern, arms are checked in descending priority, default 0; equal priorities
on the same pattern are a validation error; different patterns may reuse
numbers; without priorities source order decides with a warning for guarded
pairs - cross-reference DESIGN section 20 D1-D5). Extend the event-patterns
part with the literal-pattern forms (`on Cmd(Value(0))`, `on Flag(Value(true))`),
the full pattern-field table (wildcard / bind / literal match / named pattern /
named enum-variant pattern) and the corner case (`on E(Mode::Normal)` NOT
supported as an anonymous field pattern; `on E(x: Mode::Normal)` supported)
from KB 3.6. Rationale: patterns and priorities are core syntax that DESIGN's
subset description understates ("may be supported").

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (5.8, ~lines 307-394) | add priority action + semantics; extend event patterns with literal forms, the pattern-field table, and the enum-variant corner case |

### Action 4: Replace DESIGN 5.9's precedence list
Delete DESIGN 5.9's "Precedence, from lowest to highest" text block (the wrong
one) and insert KB 3.8's tightest-first table (11 rows as listed in Context)
with the note: relational operators bind tighter than equality; the table is
pinned against the `src/parser.rs` expression chain (action 1); use parentheses
when mixing comparison/equality nontrivially. Keep 5.9's operator list and
examples; drop the stale "// division may be added later" comment from the
precedence context if it appears there. Rationale: DESIGN must state the
precedence the parser actually implements.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (5.9, ~lines 395-429) | wrong precedence list -> verified parser-chain table |

### Action 5: Turn KB section 3 into a reference stub
Replace the bodies of KB 3.1-3.9 with a single short block under the kept
heading `## 3. RFS v0 syntax reference`: "The complete RFS v0 language surface
(module header + version, on_error, enums, events/outputs, variables,
transitions + priorities, event patterns, output expressions, the expression
language with its pinned precedence, properties and assumptions) is documented
in `[DESIGN.md](DESIGN.md)` section 5 - the single source of truth. Normative
semantics of every construct: `docs/SEMANTICS.md`." Delete the now-duplicated
subsections 3.1-3.9 (their headings go with their bodies). Rationale: one
syntax reference, no drift.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 3, ~lines 406-658) | subsections 3.1-3.9 -> one reference stub under the kept section heading |

## Task deliverables
- DESIGN section 5: complete, actualized language-surface description
  (version header, on_error, priorities, literal patterns + corner case,
  anonymous-payload limitation, verified precedence table).
- KB section 3: reference stub pointing at DESIGN section 5 + SEMANTICS.md.

## Task exit criteria
- [ ] DESIGN 5.1 shows the optional `version "X.Y[.Z]"` header; 5.2 documents
      `on_error`; 5.5 carries the anonymous-payload limitation; 5.8 has the
      priority action + Phase 2 semantics and the literal-pattern forms +
      pattern-field table + `on E(Mode::Normal)` corner case.
- [ ] DESIGN 5.9's precedence matches the `src/parser.rs` chain re-verified in
      action 1 (no `/`, shifts looser than arithmetic, relational tighter than
      equality); the old wrong list is gone.
- [ ] KB section 3's heading survives; its body is a short pointer to DESIGN 5
      + SEMANTICS.md; no syntax content remains in KB section 3.
- [ ] `cargo test --test docs_md_test --test dev_docs_test` passes.

## Non-goals
- No changes to DESIGN sections other than 5 (section 6's EBNF fence is the
  parser's canonical grammar text - do NOT touch it here; if the merged syntax
  creates a visible mismatch with section 6, record it in the status file for
  the phase close-out rather than editing the fence).
- No changes to KB sections other than 3.
- No source/test/training changes.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- If DESIGN 5.8's existing "Literal patterns may be supported" sentence
  conflicts with the merged content, replace the sentence - do not leave both
  wordings.
