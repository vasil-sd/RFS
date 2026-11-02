# Phase 9, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

(none - docs-only task)

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | changed | section 7: new "Important IR types (Rust-level view)" table (18 types from KB 2.4) + design rule 5 (hand-rolled json_rw serialization, serde removed toolchain-wide, ordered objects, byte-identical contract, key order pinned, IR_VERSION per D10); section 8: new subsection 8.5 "Parser-level checks" (parser validations / non-validations lists + the parser-vs-validator split intro); section 12: new subsection "Formula-level operators and precedence corner cases" (task 23 grammar fix mechanism + the `a \|\| b -> c` corner case + one-line cross-ref to D-P4-8/D-P4-9/section 11 for liveness precedence/scope); section 2: Backends bullet now names `src/backends/C.md` and cross-references section 10 for the emitted production shape |
| `KB.md` | changed | sections 2.4/2.5/2.6/2.7 bodies replaced by short reference stubs pointing at DESIGN 7 / 8.1+8.5 / 5.10+12 / 2+10 respectively (+ CODEGEN.md, SEMANTICS.md, backend docs where applicable); all four headings kept for cross-reference stability |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- The duplicated IR/parser/property/codegen design content in KB sections
  2.4-2.7 and the FALSE statement "The Rust backend ... is the only registered
  backend so far; C is planned next" (the C backend shipped in Phase 2).

## New functionality

- DESIGN.md as the single home for: the IR type inventory + serialization
  decision (section 7), parser-level check lists (section 8.5), formula-level
  operator rules + precedence corner cases (section 12), and the complete
  codegen architecture picture incl. both shipped backends (sections 2 + 10).
- KB sections 2.4-2.7 as stable reference stubs.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK (run after each subtask 4.1-4.4) |
| `cargo test --test docs_md_test` (5) | OK (run after each subtask 4.1-4.4) |

## Found problems/issues/mismatches
- None recorded. All four subtasks executed as planned; the two pre-checks
  that could have created NEW duplication inside DESIGN.md both came back
  clean: (4.2) the nom/chumsky comparison table already exists exactly once in
  DESIGN 8.1 - not copied; (4.3) Phase 4 liveness precedence/scope rules are
  already fully covered by D-P4-8/D-P4-9 (section 22) + section 11 Scope -
  cross-referenced, not restated.

## Other important findings/information
- Subtask 4.4 resolved as a CROSS-REFERENCE rather than a text move: every
  KB 2.7 bullet is already documented in DESIGN.md (emitted shape: section 10
  "As shipped" paragraph - more complete than KB's; design-intent bullets:
  section 2 decisions 1+4, section 3 determinism, section 5.7 simultaneous
  updates; the orphaned bitvector bullet: D8 in section 20, which states it
  more precisely). Moving the text would have duplicated DESIGN.md against
  itself - exactly what this phase eliminates. The only DESIGN edit was adding
  the missing `src/backends/C.md` pointer to section 2's Backends bullet.
- Subtask commits (per the user's commit-after-each-subtask rule): 4.1 =
  ab68aa6, 4.2 = 14b2943, 4.3 = 99c9775, 4.4 = 2cf2b44.
- The CJK artifact in DESIGN section 7 rule 4 ("should not污染 the core IR")
  was deliberately left untouched by subtask 4.1 - it is assigned to task 8
  Action 2 per the plan's corrections table.
