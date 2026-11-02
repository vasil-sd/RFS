# Phase 9, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_3.md` are reached. Not
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
| `DESIGN.md` | changed | section 5 actualized: 5.1 gained the optional `version "X.Y[.Z]"` module header (round-trips through `Module.version`, becomes `SPEC_VERSION`); 5.2 gained the `on_error` fallback item + rules block; 5.5 gained the anonymous-payload limitation (`event E(x: bit<8>)` NOT supported); 5.8 gained the `priority N;` action + body rules + Phase 2 priority semantics (D1-D5 cross-ref) and the literal-pattern forms + pattern-field table + `on E(Mode::Normal)` corner case; 5.9's WRONG precedence list (shifts tighter than `*`, nonexistent `/`) replaced with the verified parser-chain table (11 rows, tightest-first, no division, relational tighter than equality) |
| `KB.md` | changed | section 3 (253 lines, subsections 3.1-3.9) replaced by an 11-line reference stub under the kept heading "## 3. RFS v0 syntax reference", pointing at DESIGN section 5 + docs/SEMANTICS.md |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- The duplicated syntax reference in KB section 3 and the incorrect
  expression-precedence list in DESIGN section 5.9 - the language surface now
  has a single, parser-verified home in DESIGN section 5.

## New functionality

- DESIGN section 5 as the complete, actualized RFS v0 language-surface
  description (version header, on_error, priorities, literal patterns + corner
  case, anonymous-payload limitation, pinned precedence table).
- KB section 3 as a stable reference stub.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5) | OK |

## Found problems/issues/mismatches
- None recorded. Action 1's re-verification of the `src/parser.rs` expression
  chain matched the planning finding exactly (`expr -> logic_or -> logic_and ->
  equality -> relational -> bitwise_or -> bitwise_xor -> bitwise_and -> shift ->
  additive -> multiplicative -> unary_expr -> primary_expr`; zero `/`
  occurrences in `multiplicative`), so DESIGN 5.9 was rewritten from KB 3.8's
  table without deviation.

## Other important findings/information
- One sentence beyond the precedence list was actualized inside the same 5.9
  block: "For Phase 0, the parser accepts the grammar. Semantic type checking
  is a later validator concern." -> "The parser accepts this grammar; semantic
  type checking is the validator's job (`src/validator.rs`, section 13)." -
  the old wording was stale (the validator has existed since Phase 1) and sat
  in the exact block being replaced.
- DESIGN 5.10 (property operators + monitor semantics) was left untouched as
  planned - it is already newer than the removed KB 3.9.
- The KB section 3 replacement was done with a boundary-based script (heading
  to heading) because the block spans 253 lines; the stub text matches the
  task file's specified wording.
