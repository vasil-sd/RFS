# Phase 9, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Move the remaining core-design content out of KB sections 2.4-2.7 into their
DESIGN.md homes: the IR type table + hand-rolled-JSON decision into DESIGN 7,
the parser check lists into DESIGN 8, the property-language corner cases into
DESIGN 12, and the codegen architecture (fixing the stale "C is planned next"
claim) into DESIGN 2/10. KB sections 2.4-2.7 become reference stubs.
Rationale: IR schema, parser design, property language, and codegen
architecture are core design decisions - DESIGN.md is their single source of
truth, and KB 2.7 currently states something false (the C backend shipped in
Phase 2).

## Context
Verified at planning by reading KB 2.4-2.7 against DESIGN 2/7/8/10/12:
- **KB 2.4 "IR decisions" vs DESIGN 7**: DESIGN 7 has the JSON schema (top-
  level/module/enum/event shapes) + four IR design rules. KB 2.4 adds two
  things DESIGN 7 lacks: the decision that serialization is the hand-rolled
  JSON reader-writer `src/json_rw.rs` (Phase 5 task 2 - serde removed,
  ordered objects, byte-identical output contract) and the "Important IR
  types" table (Ir, Module, EnumDecl, Type, ConstructorGroup, ConstructorVariant,
  FieldDecl, VarDecl, Transition, EventPattern/OutputPattern, PatternVariant,
  PatternField, PatternExpr, OutputExpr, Assignment, Property/Formula, Expr -
  the Rust-type-level view). Merge both into DESIGN 7 (extend the design rules
  with the json_rw decision; add the type table after the schema).
- **KB 2.5 "Parser decisions" vs DESIGN 8**: the nom/chumsky comparison table
  is ALREADY in DESIGN 8.1 (more detailed - do not duplicate it back). KB 2.5's
  UNIQUE content is the two lists: "Current parser validations" (duplicate
  enum declarations/variants, duplicate initial state, duplicate event/output
  variant in a group, duplicate variables, duplicate property/assumption
  names, duplicate struct field names, exactly-one-next rule, duplicate
  assignment rejection, duplicate priority rejection) and "Current parser
  non-validations" (unknown state names, unknown groups/variants, pattern
  field count/type compatibility, expression typing, bit-width overflow,
  initial-state membership, output argument count/type). Move both lists into
  DESIGN 8 as a new subsection 8.5 "Parser-level checks" (the split between
  parser-enforced and validator-enforced checks is design-relevant).
- **KB 2.6 "Property language decisions" vs DESIGN 12 (+ 5.10)**: DESIGN 12
  covers the v0 subset, planned operators, and strong/weak liveness; DESIGN
  5.10 covers monitor semantics (D-P3-3) and MC-only liveness. KB 2.6's UNIQUE
  content: (a) formula-level boolean operators - the task 23 grammar fix:
  `&&`/`||` are FORMULA-level operators (`and_formula`/`or_formula`); a formula
  operand that is not a keyword atom or parenthesized formula falls through to
  the NON-BOOLEAN expression level (`non_boolean_expr = equality_expr`), never
  to the full `expr` ladder - this is why mixed atoms like `state == S &&
  output_has(O)` parse; (b) the known corner case `a || b -> c` parsing as
  `a || (b -> c)` with the parenthesize-implications guidance; (c) Phase 4
  liveness precedence/scope rules (`!` binds tighter than `until`; `&&`/`||`/
  `->` looser; top-level obligations over boolean subformulas; deeper nesting
  -> codegen diagnostic + skip). BEFORE merging (c), check DESIGN section 11
  and section 22 (D-P4-8/D-P4-9) - some of it may already be there; add only
  what is missing so no NEW duplication is created inside DESIGN.md. (a) and
  (b) go into DESIGN 12 as a new subsection (e.g. "Formula-level operators and
  precedence corner cases").
- **KB 2.7 "Code generation architecture (implemented)" vs DESIGN 2**: STALE -
  it says "The Rust backend ... is the only registered backend so far; C is
  planned next" (C shipped in Phase 2). DESIGN 2's "Code generation as a
  plugin system" subsection already states both backends are shipped and
  parity-pinned, and covers core/shared-analysis/backends + five design
  decisions. KB 2.7's UNIQUE content: the generated-Rust shape (one self-
  contained file per command: `enum State/Input/Output`, `struct Vars`, trace
  runtime, `AssumptionHandler` trait with policy enum, `StepError`,
  `Fsm::new() / Fsm::step(input) -> Result<Vec<Output>, StepError>`) and the
  design-intent bullets (deterministic or priority-ordered production code;
  simultaneous variable updates; MC = production core + runtime consuming the
  same IR; adding a backend = new module + one registry entry). Merge the
  shape into DESIGN 2's plugin subsection (it fits the "Backends" bullet) or
  into section 10 if it reads better there; drop the stale claim entirely.
- **Cross-reference stability**: keep KB headings `### 2.4 IR decisions`,
  `### 2.5 Parser decisions`, `### 2.6 Property language decisions`,
  `### 2.7 Code generation architecture (implemented)` - bodies become stubs.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | KB->DESIGN dedup: syntax reference | same two files - sequential execution keeps DESIGN's section flow coherent and avoids edit conflicts |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | KB.md stale-section cleanup | needs the final KB stub structure (all of section 2 stubbed) before renumbering/removing other sections |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | section 7 extended (json_rw decision + IR type table); new section 8.5 parser-level checks; section 12 extended (formula-level operators + corner cases, liveness rules only if not already in 11/22); section 2 or 10 extended (generated-Rust shape + design intent) |
| `KB.md` | change | sections 2.4-2.7 bodies -> reference stubs, headings kept |

## Planned actions

### Action 1: IR decisions -> DESIGN 7 (subtask 4.1)
Extend DESIGN 7's "IR design rules" with the hand-rolled-JSON serialization
decision (`src/json_rw.rs`, Phase 5 task 2 - serde/serde_json removed from the
whole toolchain; ordered objects; byte-identical output contract; key order
pinned in `to_json`/`from_json`) and append KB 2.4's "Important IR types"
table after the schema blocks. Then stub KB 2.4: "IR versioning, the
hand-rolled JSON serialization decision, and the IR type inventory live in
`[DESIGN.md](DESIGN.md)` section 7 (single source of truth)." Rationale: the IR
schema is core design; the type table is the Rust-level complement of the JSON
schema.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 7, ~lines 591-751) | add json_rw serialization decision to the design rules + the Important IR types table |
| `KB.md` | change (section 2.4, ~lines 197-231) | body -> reference stub to DESIGN 7, heading kept |

### Action 2: Parser decisions -> DESIGN 8 (subtask 4.2)
Add a new subsection "### 8.5 Parser-level checks" to DESIGN 8 containing KB
2.5's two lists (parser validations / parser non-validations) with a one-line
intro explaining the split: the parser enforces structural duplicates and body
shape; everything semantic is the validator's job (`src/validator.rs`). Do NOT
copy the nom/chumsky table (DESIGN 8.1 is canonical). Then stub KB 2.5:
"The parser implementation choice (nom 8 vs chumsky comparison) is in
`[DESIGN.md](DESIGN.md)` section 8.1; the parser-level check lists are in its
section 8.5." Rationale: one home for parser design facts.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (after section 8.4, ~line 804) | new subsection 8.5 with the two check lists from KB 2.5 |
| `KB.md` | change (section 2.5, ~lines 232-274) | body -> reference stub to DESIGN 8.1/8.5, heading kept |

### Action 3: Property language decisions -> DESIGN 12 (subtask 4.3)
First read DESIGN section 11 (liveness part) and section 22 entries D-P4-8/
D-P4-9 to see which of KB 2.6's Phase 4 precedence/scope rules are already
documented. Add a new subsection to DESIGN 12 (e.g. "### Formula-level
operators and precedence corner cases") containing: the task 23 grammar fix
(formula-level `&&`/`||`; non-keyword operands fall through to
`non_boolean_expr = equality_expr`, never the full `expr` ladder; why mixed
atom kinds parse), the `a || b -> c` = `a || (b -> c)` corner case with the
parenthesize-implications guidance, and any Phase 4 liveness precedence/scope
rule NOT already covered by 11/22. Then stub KB 2.6: "The property language
subset, monitor semantics, and formula-level operator rules live in
`[DESIGN.md](DESIGN.md)` sections 5.10 + 12 (single source of truth); normative
operator semantics: `docs/SEMANTICS.md`." Rationale: property-language design
belongs with the PSL plan; no new duplication inside DESIGN.md.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 12, ~lines 1270-1318) | new subsection: formula-level operators, the ||/-> corner case, missing Phase 4 rules only |
| `KB.md` | change (section 2.6, ~lines 275-364) | body -> reference stub to DESIGN 5.10/12 + SEMANTICS, heading kept |

### Action 4: Codegen architecture -> DESIGN 2/10 (subtask 4.4)
Delete KB 2.7's stale sentence ("The Rust backend ... is the only registered
backend so far; C is planned next"). Merge its generated-Rust shape paragraph
and design-intent bullets into DESIGN 2's "Code generation as a plugin system"
subsection (extend the "Backends" bullet with the emitted-file shape: enum
State/Input/Output, struct Vars, trace runtime, AssumptionHandler trait +
policy enum, StepError, Fsm::new()/Fsm::step() signature) - or into section 10
if the plugin subsection becomes unwieldy; state explicitly that BOTH backends
are shipped and parity-pinned. Then stub KB 2.7: "The codegen plugin
architecture (core registry, shared analysis, per-language backends - Rust AND
C, both shipped and parity-pinned) is documented in `[DESIGN.md](DESIGN.md)`
section 2; the backend-author API reference is `CODEGEN.md`; backend internals:
`src/backends/RUST.md` / `src/backends/C.md`." Rationale: removes a false
statement and consolidates codegen architecture in DESIGN.md.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 2 plugin subsection, ~lines 83-101, or section 10) | add the generated-Rust shape + design-intent bullets; both-backends-shipped stated |
| `KB.md` | change (section 2.7, ~lines 365-405) | stale claim removed; body -> reference stub to DESIGN 2 + CODEGEN.md, heading kept |

## Task deliverables
- DESIGN 7: json_rw serialization decision + Important IR types table.
- DESIGN 8.5: parser-level checks (validations / non-validations).
- DESIGN 12: formula-level operators + precedence corner cases (+ any missing
  Phase 4 rules).
- DESIGN 2 (or 10): generated-Rust shape + design intent, both backends stated
  as shipped.
- KB 2.4-2.7: reference stubs with unchanged headings.

## Task exit criteria
- [ ] DESIGN 7 contains the hand-rolled-JSON decision and the IR type table;
      no "serde" remains as the current serialization mechanism anywhere in
      DESIGN 7.
- [ ] DESIGN 8 has a parser-level-checks subsection with both lists; the
      nom/chumsky table appears exactly once (in 8.1).
- [ ] DESIGN 12 documents the formula-level `&&`/`||` fall-through and the
      `a || b -> c` corner case; grep shows no duplicated Phase 4 precedence
      rule between sections 11, 12, and 22.
- [ ] The string "C is planned next" (or equivalent stale claim) no longer
      exists in KB.md or DESIGN.md; both backends are stated as shipped.
- [ ] KB headings 2.4/2.5/2.6/2.7 survive with stub bodies pointing at the
      correct DESIGN sections.
- [ ] `cargo test --test docs_md_test --test dev_docs_test` passes.

## Non-goals
- No changes to DESIGN sections other than 2 (or 10), 7, 8, 12.
- No renumbering of DESIGN subsections except ADDING 8.5 (no existing number
  moves).
- No source/test/training changes.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The D-P4-8/D-P4-9 check in action 3 is the one place this task could create
  a new duplication - do that read BEFORE writing, and record in the status
  file which rules were already covered vs added.
