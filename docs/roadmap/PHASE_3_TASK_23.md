# Phase 3, Task 23 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Fix the property-formula grammar so formula-level boolean operators work with any
atom kinds on either side (e.g. `state == S && output_has(O)`), and add tests pinning
the new parse shapes, printer round-trips, and monitor acceptance. Added per user
request (plan correction 23); supersedes the workaround recorded in 13a - example 42
keeps its original text.

## Context

- Root cause (verified empirically during the resumption audit): `primary_formula`
  (src/parser.rs) falls through to the FULL expression grammar for non-keyword starts
  (`let (i2, e) = expr(i)?` at the end of the function). The expr chain is
  `expr = logic_or > logic_and > equality > relational > ...`, so its top levels include
  `||`/`&&`, and `primary_expr` accepts bare identifier paths. In
  `state == Sending && output_has(Sent)` the fallthrough therefore absorbs
  `state == Sending && output_has` (with `output_has` consumed as `Expr::Path`) and
  leaves `(Sent)` as unparseable trailing input -> "trailing input after RFS source".
- Pre-fix empirical constraint map: FAILS - a bare `state == S` on the LEFT of
  `&&`/`||` with a non-comparison right operand (incl. parenthesized); WORKS - reordered
  operands, left-parenthesized state comparison, comparison/comparison mixes.
  Post-fix, ALL these shapes must parse.
- Fix design: formula-level `&&`/`||` belong to the FORMULA grammar (`and_formula` /
  `or_formula` already build `Formula::And`/`Formula::Or`). Restrict the
  `primary_formula` fallthrough to the NON-BOOLEAN expression level - parse at
  `equality` (the level directly below `logic_and`) instead of `expr`. Each boolean
  operand is then parsed independently by `primary_formula`, so `output_has(...)` hits
  its keyword branch. Add a named wrapper (e.g. `fn non_boolean_expr(i) -> IResult<&str, Expr> { equality(i) }`)
  with a comment explaining WHY the full expr grammar must not be used here.
- Consequence to pin in tests: pure-expression conjunctions in properties
  (`prop p : x && y`) now parse as `Formula::And(Expr(x), Expr(y))` instead of
  `Expr(Binary(LogicAnd, ...))`. This is additive for JSON (the old IR shape still
  deserializes - no `IR_VERSION` bump); transition guards and all non-formula expression
  contexts are UNAFFECTED (they call `expr()` directly, not `primary_formula`).
- Precedence is unchanged: formula `||` looser than `&&`, matching the expression levels;
  `a && b || c` -> `Or(And(a, b), c)` at the formula level.
- Printer: check how `print_formula_ctx` renders `Formula::And`/`Or` with bare `Expr`
  children - the output must be minimal (`x == y && z == w`, no redundant parens) and
  round-trip (rfs->json->rfs and json->rfs->rfs). Adjust only if it over-parenthesizes.
- Monitor compiler already accepts `Formula::And`/`Or` of per-sample signals (e.g.
  `input_is(E) && state == A` compiles today - see P14 in the audit), so mixed
  conjunctions become in-subset with NO R1-R4 change; add one plan-structure test to pin it.
- Runs BEFORE task 13 (task 13 verifies example 42 parses unchanged).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-5. | always[N] IR/parser/printer/validator/tests | parser foundation already in place (implemented) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Valid examples verification | example 42 parses only after this fix |
| 21. | Docs sync | KB/DESIGN must document the fixed grammar (formula-level &&/||, non-boolean expr fallthrough) |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/parser.rs` | change | restrict `primary_formula` fallthrough to the non-boolean expression level (+ `#[cfg(test)]` unit tests) |
| `src/printer.rs` | change (only if needed) | minimal-paren printing of `Formula::And`/`Or` with bare `Expr` children (+ round-trip tests) |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/parser.rs`, `src/printer.rs`, `src/monitor.rs` | change (`#[cfg(test)]`) | new unit tests per Planned actions |

## Planned actions

All planned actions needed to reach the task goal, in execution order.

### Action 1: Restrict the primary_formula expr fallthrough (parser)

In `src/parser.rs`, replace the final fallthrough of `primary_formula`
(`let (i2, e) = expr(i)?;`) with a call to a new named wrapper around the `equality`
level (non-boolean expressions: comparisons, relational, arithmetic, literals,
identifiers, parenthesized groups). Add a comment explaining why the full expr grammar
must not be used here (formula-level `&&`/`||` are handled by and_formula/or_formula;
the full chain would mis-consume identifier-looking formula atoms as variable paths).
Verify immediately: the previously failing shapes now parse (`state == A && output_has(O)`,
`state == A || output_has(O)`, `state == A && (output_has(O))`) and transition-guard
parsing is unchanged (existing guard tests cover it).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/parser.rs` | change (`primary_formula`, end of function; new small helper fn) | the actual grammar fix - one line redirected + documented wrapper |

### Action 2: Printer round-trip for the new shapes

Generate `Formula::And`/`Or` with bare `Expr` children through the printer and check the
output is minimal (`x == y && z == w`) and re-parses to the same shape (rfs->json->rfs,
json->rfs->rfs). If `print_formula_ctx` currently wraps such children in redundant
parens, adjust its precedence handling so formula-level boolean operands that are plain
expressions print bare. Add/extend round-trip tests in `src/printer.rs #[cfg(test)]`
in the style of `formula_precedence_roundtrips`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/printer.rs` | change (print_formula_ctx / precedence helper, only if over-parenthesizing) | keep generated/printed specs stable and round-trip-safe |
| `src/printer.rs` | change (`#[cfg(test)]`) | round-trip tests for the new formula-level boolean shapes |

### Action 3: Parser + monitor unit tests pinning the fix

Add to `src/parser.rs #[cfg(test)]`:
- mixed conjunction with state comparison on the left parses as
  `Formula::And(Expr(Eq), OutputHas)` (the exact pre-fix failing shape);
- mixed disjunction parses (`state == A || output_has(O)`);
- previously-working shapes still parse unchanged: reordered operands,
  left-parenthesized state comparison, comparison/comparison mix;
- pure-expression conjunction `x && y` (variables) now parses as
  `Formula::And(Expr(x), Expr(y))` - pins the new AST shape;
- precedence: `a && b || c` -> `Or(And(a, b), c)` at formula level; and
  `x == y && z == w` keeps both comparisons intact.
Add to `src/monitor.rs #[cfg(test)]`: one plan-structure test that
`always (state == A && output_has(O))` compiles (accepted by R1-R4) as a per-sample
signal conjunction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/parser.rs` | change (`#[cfg(test)]`) | pins the fixed grammar + new AST shapes + regression guards |
| `src/monitor.rs` | change (`#[cfg(test)]`) | pins that mixed conjunctions are monitor-compilable (in-subset) |

### Action 4: Full-suite verification

Run `cargo test --no-fail-fast` (toolchain env per START.md section 4). If any existing
assertion pinned the OLD Expr-level shape for formula-level boolean expressions (e.g. a
printer/validator test expecting `Expr(Binary(LogicAnd))` from a property), update that
assertion to the new Formula-level shape and record it in the Task corrections table.
Also run `cargo run -q -- parse --input rfs --output json examples/valid/42_bounded_liveness_protocol.rfs`
to confirm example 42 now parses UNMODIFIED (hand-off to task 13).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/*.rs`, `tests/*.rs` | change (only if an old-shape assertion is found) | keep the whole suite green under the new AST shapes |

## Task deliverables
- Fixed `primary_formula` fallthrough (non-boolean expression level) with explanatory comment.
- Printer round-trips minimal for formula-level boolean expressions with Expr children.
- New unit tests: parser (mixed conjunction/disjunction, pure-expr conjunction shape,
  precedence, regression guards), monitor plan-structure acceptance, printer round-trips.
- `cargo test --no-fail-fast` fully green; example 42 parses unmodified.

## Task exit criteria

- [ ] `state == A && output_has(O)`, `state == A || output_has(O)`, and `state == A && (output_has(O))` parse (previously all failed).
- [ ] Pure-expression conjunction in a property parses as `Formula::And(Expr, Expr)`; precedence `a && b || c` -> `Or(And(a,b),c)`.
- [ ] Previously-working shapes still parse identically (reordered, parenthesized, comparison mixes); transition-guard tests unchanged and green.
- [ ] Printer round-trips (rfs->json->rfs, json->rfs->rfs) for the new shapes with no redundant parens.
- [ ] `always (state == A && output_has(O))` compiles in the monitor compiler (plan-structure test).
- [ ] `cargo test --no-fail-fast` fully green; `examples/valid/42_bounded_liveness_protocol.rfs` parses unmodified.

## Non-goals
- No IR_VERSION bump (additive parse-shape change only; old JSON still deserializes).
- No changes to transition guards or any non-formula expression context.
- No documentation updates here - task 21 documents the fixed grammar in KB/DESIGN.
- No changes to examples (example 42 keeps its original text).

## Other information

- If Action 4 finds an existing test asserting the old Expr-level shape, that is EXPECTED
  and handled by updating the assertion (record it in Task corrections for traceability).
- The fix makes correction 13a's workaround obsolete: do NOT reorder example 42.
- Suggested first verification command after Action 1:
  `cargo run -q -- parse --input rfs --output json examples/valid/42_bounded_liveness_protocol.rfs`