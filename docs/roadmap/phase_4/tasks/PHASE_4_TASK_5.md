# Phase 4, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Add the liveness operators to the property language (D-P4-8): `eventually (p)` as a
new primary and infix `until` on a new precedence level - parser, additive IR,
printer round-trip, validator typing, and the monitor-subset boundary (identical
skip warning). NO example files yet; examples land with engine support in task 6 so
the tree stays green at every step.

## Context

- Pinned grammar (D-P4-8): `eventually "(" formula ")"` is a new `primary_formula`
  alternative; infix `until` sits on a NEW level between unary and primary:
  `until_formula = primary_formula { "until" primary_formula }` (left-associative;
  operands are primaries). Existing ladder (DESIGN section 6): or > impl > and >
  unary > primary, with `primary_formula` falling through to the non-boolean
  expression level. New full ladder: or > impl > and > unary > until > primary.

  EBNF (the precise form to implement):
  ```
  property_formula : or_formula ;
  or_formula       : impl_formula { "||" impl_formula } ;
  impl_formula     : and_formula { "->" and_formula } ;
  and_formula      : unary_formula { "&&" unary_formula } ;
  unary_formula    : "!" unary_formula | until_formula ;
  until_formula    : primary_formula { "until" primary_formula } ;   (NEW, left-assoc)
  primary_formula  : "eventually" "(" property_formula ")"            (NEW alternative)
                   | existing primary alternatives (incl. parenthesized formula,
                     atoms, non-boolean expression fallthrough) ;
  ```
  Parse consequences to PIN in tests:
  - `a && b until c` parses as `a && (b until c)` (until binds tighter than &&).
  - `a until b until c` parses as `(a until b) until c` (left-associative).
  - `!a until b` parses as `(!a) until b`.
  - Compound boolean operands to `until` require parentheses: `(a && b) until c`.

  OPEN QUESTION (confirm at task start): D-P4-8's illustrative example says
  "`a && b until c` parses as `(a && b) until c`", which is INCONSISTENT with its
  own production + level placement (under them it parses as `a && (b until c)`;
  the alternative reading would require `until` between and and unary with
  non-primary operands). This task file implements the PINNED PRODUCTION (tightest
  binding, standard PSL/CTL convention, and it keeps temporal operators out of
  `until` operands by syntax - which D-P4-9 relies on). If the user prefers the
  other reading, record a plan correction to D-P4-8 before implementing.

- IR (D-P4-8): additive `Formula::Eventually { inner }`, `Formula::Until { left,
  right }`; NO `IR_VERSION` bump (old JSON still deserializes).
- Semantics: WEAK until (`p U q`); strong until is expressible as
  `p until q && eventually q` (D-P4-8). Engine semantics are task 6's concern; this
  task only lands syntax/IR/typing.
- Scope restriction (D-P4-9): liveness obligations are top-level `eventually p` /
  `p until q` with BOOLEAN subformulas over atomic propositions (no temporal
  operators inside). Deeper nesting -> codegen diagnostic (task 6 emits it; the
  validator here only checks boolean typing).
- Monitor boundary (D-P4-9): ANY formula containing `eventually`/`until` is outside
  the monitor subset -> IDENTICAL skip warning in production/test codegen in both
  backends (MC-only properties). Extend the existing R1-R4 out-of-subset mechanism
  with a "contains liveness operator" check so the warning text is shared.
- Tests: lib-level round-trip + parser + validator tests with inline formulas. NO
  example files (task 6 adds the first one) - `examples_test` must stay at
  42 valid / 12 invalid.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Liveness engine - Rust | consumes the new IR variants; adds the first liveness example |
| 7. | Liveness engine - C | same |
| 8. | Benchmark specs | token ring / elevator use `eventually`/`until` properties |
| 12. | Training data update | SFT examples for the new syntax + weak-until semantics |
| 13. | Docs sync | DESIGN section 6 grammar + section 12 subset table |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/parser.rs` | change | `until_formula` level + `eventually` primary alternative (+ unit tests) |
| `src/ir.rs` | change | additive `Formula::Eventually { inner }` / `Formula::Until { left, right }`; no version bump |
| `src/printer.rs` | change | render both operators with minimal parentheses; round-trip stable |
| `src/validator.rs` | change | typing: operands must be boolean formulas (+ unit tests) |
| `src/monitor.rs` | change | liveness operators outside the subset -> identical skip warning |

## Planned actions

### Action 1: Parser
Add the `until_formula` level between unary and primary and the `eventually "(" 
property_formula ")"` primary alternative per the EBNF above. Unit tests pinning:
the parse consequences in Context (precedence, left-associativity, unary
interaction), `eventually` with nested boolean formulas, and rejection of malformed
shapes (e.g. `eventually p` without parentheses, trailing input).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/parser.rs` | change | new grammar level + primary alternative (+ `#[cfg(test)]` tests) |

### Action 2: IR variants
Add `Formula::Eventually { inner }` and `Formula::Until { left, right }` (additive;
serde round-trip; NO `IR_VERSION` bump - verify old JSON still deserializes).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/ir.rs` | change | two additive Formula variants |

### Action 3: Printer
Render `eventually (p)` with its mandatory parentheses; render `a until b` without
extra parentheses (operands are primaries by grammar; nested `until` chains print
left-associatively bare). Round-trip tests both directions (rfs->json->rfs and
json->rfs->rfs) for all new shapes.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/printer.rs` | change | rendering + round-trip tests |

### Action 4: Validator typing
`eventually`/`until` produce boolean formulas; their operands must type-check as
boolean (e.g. `eventually (x)` where `x` is a non-boolean expression -> validation
error with a clear message). Unit tests for accept + reject cases.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/validator.rs` | change | typing rules (+ `#[cfg(test)]` tests) |

### Action 5: Monitor-subset boundary
Extend the shared out-of-subset detection (R1-R4 mechanism in `src/monitor.rs`)
with "formula contains `eventually`/`until`" so production/test codegen emits the
IDENTICAL skip warning in both backends for any property using liveness operators.
Add a codegen-level test pinning the identical warning text.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/monitor.rs` | change | liveness-operator out-of-subset check (shared warning) |
| `tests/codegen_test.rs` | change | identical-skip-warning test for both backends |

## Task deliverables
- `eventually (p)` + infix `until` in the property language: parser, IR, printer,
  validator, monitor boundary - all green, no examples yet.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Parser tests pin the precedence table (Context) incl. left-associativity and unary interaction.
- [ ] Round-trip stable for all new shapes; old IR JSON still deserializes (no version bump).
- [ ] Validator rejects non-boolean operands with clear diagnostics.
- [ ] Identical skip warning in both backends' production/test codegen (test-pinned).
- [ ] `examples_test` unchanged: 42 valid / 12 invalid.

## Non-goals
- Engine/fixpoint analysis (tasks 6/7), first liveness example (task 6), benchmark
  specs (task 8), docs (task 13), training data (task 12).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| EBNF block contradicted its own pinned consequence for `!a until b` | Context's EBNF (`unary_formula : "!" unary_formula \| until_formula` with `until_formula : primary_formula { "until" primary_formula }`) parses `!a until b` as `!(a until b)` - after consuming `!`, the recursive call descends into `until_formula` and absorbs the whole chain. But Context's pinned consequence, the stated ladder ("or > impl > and > unary > until > primary", i.e. `!` binds TIGHTER than `until`) and standard PSL/CTL all require `(!a) until b`. The exit criteria pin the consequences, so the implementation follows them: negation moved BELOW the until level - `unary_formula = until_formula`, `until_formula = neg_operand { "until" neg_operand }` (left-assoc), `neg_operand = "!" neg_operand \| primary_formula`. All four pinned consequences hold; for every input without a bare `until` token the parse is structurally identical to the pre-task grammar (full suite green). |
| OPEN QUESTION resolved with the default | D-P4-8's illustrative example ("`a && b until c` parses as `(a && b) until c`") contradicts its own production; per this file's stated default, the PINNED PRODUCTION was implemented: `until` binds tighter than `&&`, so `a && b until c` = `a && (b until c)` (test-pinned). The user was informed at task start and did not object; no plan correction recorded. |

## Other information
- Resolve the OPEN QUESTION (D-P4-8 example inconsistency) at task start before
  writing parser code - one line to the user is enough; default = implement the
  pinned production as written in this file.
- Keep `until` a keyword at the formula level only - it must not shadow identifiers
  in expression contexts (check the lexer/keyword handling first).
