# Phase 18, Task 3L - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Give the expression language an EXACT, precise semantics for arithmetic and value
conversion, eliminating every implicit/silent conversion. Requirements:
(1) constant expressions (integer literals only) are semantically integer literals -
folded at compile time with FULL precision (signed i128 intermediates), no silent
truncation/overflow/underflow; the IR keeps unfolded expressions; the folding is a
separate shared IR-translation stage used by validation, mc-ref and ALL codegen
backends. Bitwise operations on negative constants are treated as operations on the
UNSIGNED TWO'S-COMPLEMENT representation of the constant (full precision): `2 + -3 ==
-1`, `2 + (-1) & 1 == 3`, `2 + (-4) & 3 == 2` - which is also why masking a negative
constant (`(-5) & 15`) folds to an ordinary POSITIVE constant (11) that then converts
under rule 4 without any diagnostic. (2) Typing stated explicitly as separate rules:
every arithmetic/bitwise operator requires EQUAL operand widths; shifts are the only
exception (result width = shifted value's width; if the value is a constant expression
and the shift amount is a variable, result width = shift amount's width). (3) A new
explicit conversion operator `bit<N>(expr)` - tightest binding, zero-extension or
high-bit truncation, operand must be a bitvector (a constant expression IS a legal
operand - the cast is an explicit conversion; for any constant C, `bit<N>(C)` = C mod
2^N, i.e. the low N bits of its two's-complement representation). (4) Constant-to-
bitvector conversion: a POSITIVE constant that does not fit the target width is an
ERROR unless explicitly cast (`var a: bit<2> init (2 + 2)` - error; `init bit<2>(2 +
2)` - OK, truncates 4 to 0); a NEGATIVE constant whose two's-complement form fits
(abs(C) <= 2^(N-1)) is a WARNING - "implicit conversion of negative constant to
unsigned bit-vector" - with the value being the two's-complement encoding, and an
explicit cast suppresses the warning (`var a: bit<4> init -5` - warning, value 11;
`init bit<4>(-5)` - OK); a NEGATIVE constant whose two's-complement form does NOT fit
is an ERROR unless explicitly cast (`var a: bit<3> init -5` - error; `init bit<3>(1 -
6)` - OK, truncates ...1011 to 011 = 3). The explicit cast is the ONLY explicitness
mechanism - no syntactic mask detection (a mask is just a bitwise op under rule 1).
(5) The task-3H residual difference (exact mode carries unmasked out-of-range inits,
SMT rejects them) is eliminated by rule 4 making both modes reject at validation.
Main principle: no implicit conversions, no silent precision loss, no silent
truncation - all effects visible and user-controllable.

## Context

### Current state of the typing machinery (verified 2026-11-25)
There are THREE parallel implementations of the typing rules, all currently agreeing:
1. `src/lang/validator.rs` - `ExprType` + `type_expr` (~line 505), `binary_type`
   (~578), `bit_op_type` (~611), `compatible_with`. This is the authoritative
   validator typing; codegen reuses it directly.
2. `src/mc/ref_mc.rs` - `RType` + `expr_type` (~line 100/140), its own
   `binary_type`/`bit_op_type`/`compatible` mirrors, used by `eval_expr` (~292)
   which wraps arithmetic at the expression's typed width (`mask(w)`, ~283).
3. `src/mc/smt_plan.rs` - `SeType` + `se_type` (~1392/1416), `bit_op_type` (~1470)
   over the lowered `Se` AST, used by `convert_expr` (~978) and both emitters.

Current literal rules (identical in all three): an integer literal has type
`IntLit`; `IntLit op Bit(b)` -> `Bit(b)` (the literal ADAPTS - the implicit
conversion this task removes); `IntLit op IntLit` -> `Bit(8)` (the "two bare
literals form a bit<8>" rule of SEMANTICS.md 2.2 that the user cites); shifts:
left operand determines the width (`IntLit` counts as 8), right operand may be any
bitvector or literal with NO width equality. Var+var mixed widths are ALREADY an
error in all three ("bitwidth mismatch: bit<a> vs bit<b>") - requirement 2's
equal-width rule exists in code but is NOT stated as a separate explicit rule in
SEMANTICS.md 2.2 (it is buried in one parenthetical).

`src/lang/ir.rs::simplify_expr` (~1449) is a BOOLEAN-ONLY simplification (constant
folding of `&&`/`||` with complement detection) applied at codegen entry - it is NOT
the arithmetic fold this task adds and must stay untouched.

### The task-3H residual difference to eliminate (requirement 5)
Exact mode (`ref_mc_exact.rs` ~336) evaluates each var init via `eval_expr` with an
empty vars map and carries the result UNMASKED: `var x: bit<1> init 1 + 1 ;` folds
at the expression's own type (bare literals default to Bit(8)) to Val::Bit(2) and is
ACCEPTED with initial value 2 in a bit<1> var. SMT's width check rejects it ("does
not fit in 1 bits"). Bare out-of-range literals (`bit<1> init 3`, `bit<8> init 300`)
are accepted by exact and rejected by SMT today as well. Under rule 4 ALL of these
become validation errors - both modes reject identically before any engine runs, and
the SMT width check becomes a defensive guard (kept) that validated specs never hit.
SEMANTICS.md 2.4's `[^init-order]` footnote ("initializer that references another
variable does not type-error but generates code that cannot compile") is also
retired: rule 4 + the existing exact-mode empty-vars rejection make init semantics
clean and stated.

### Where the new cast syntax lands
Parser: `src/lang/parser.rs` precedence chain `expr -> logic_or -> ... -> unary_expr
-> primary_expr` (~940-1222). The cast is a NEW arm in `primary_expr`: `p_tag("bit")`
+ `<` + int + `>` + exactly ONE primary operand (identifier, literal, or parenthesized
expression) - "binds most tightly" means `bit<4> a + b` = `(bit<4> a) + b` and
`bit<4> a * b` = `(bit<4> a) * b`; `bit<4> -a` does NOT parse (write `bit<4> (-a)`).
`bit` becomes reserved in EXPRESSION position (type positions like `var x: bit<8>`
are parsed by separate functions and are unaffected). IR: new variant
`Expr::Cast { width: u32, expr: Box<Expr> }` - ADDITIVE (old documents never contain
it; readers of old JSON are unaffected), so `IR_VERSION` stays 1 per the additive-
optional-field policy in START.md section 8. Printer (`src/lang/printer.rs`) renders
`bit<N>(operand)` with parentheses around the operand whenever it is not a bare
identifier (round-trip fidelity: parse(print(e)) == e structurally).

### Every Expr match site that gains a Cast arm (verified inventory)
`src/lang/ir.rs` (to_json/from_json + simplify_expr passthrough), `src/lang/parser.rs`,
`src/lang/printer.rs`, `src/lang/specgen.rs` (random expr generation - must at least
not break; generating casts is optional), `src/lang/validator.rs` (type_expr + the new
rules), `src/mc/ref_mc.rs` (expr_type + eval_expr), `src/mc/smt_plan.rs`
(convert_expr + se_type), `src/codegen/backends/rust/fsm.rs` (gen_expr ~1612 +
width_from_type ~1735), `src/codegen/backends/c/fsm.rs` (gen_expr_c ~796, IntLit ->
uint64_t at ~813), `src/mc/plan.rs` (one site), `src/cli/commands/info.rs` (two
sites - display), `src/codegen/test_plan.rs` (five sites), `tests/psl_translation_test.rs`
(one site). `src/codegen/monitor.rs` needs NO change: it wraps whole expressions in
`SignalAtom::VarExpr(expr.clone())` (~312) and the R1-R4 checks walk Formula structure,
so the new variant flows through to the backends' gen_expr automatically.

### The embedded grammar model
`src/cli/info_format/ebnf_grammar.rs` `GRAMMAR_RULES` (~579) mirrors DESIGN.md section
6 EXACTLY (rule names + structure, machine-checked by `tests/info_format_test.rs`
against the real parser and DESIGN.md). Adding the cast production means: +1 rule in
DESIGN.md section 6, +1 rule in GRAMMAR_RULES, and the sync test keeps both honest.

### Docs that state the current (to-be-replaced) rules
- `docs/SEMANTICS.md` 2.2 (~line 75): the operator table + the one-paragraph typing
  rules containing "an integer literal adapts to the other operand's width; two bare
  literals form a bit<8>" - REWRITTEN as an explicit numbered rule list.
- `docs/SEMANTICS.md` 2.4 (~145) + footnote `[^init-order]`: init semantics -
  rewritten (constant folding, rule 4 fit check, empty-vars rejection stated).
- `DESIGN.md` 5.9 (~485): supported-expression list + precedence table (cast = new
  tightest level) and 6: EBNF grammar (+1 production).
- `docs/user/LANGUAGE.md` section 9 (~320): operator table, precedence, typing rules -
  the user-facing mirror of SEMANTICS 2.2.
- `docs/dev/` fragments describing typing (mc_engine.md's SMT typing paragraph,
  validator.md if it names the literal-adaptation rule) - checked via
  `rfs-dev context type_expr` / grep; updated in this task because they state the
  OLD rules as current behavior (task 3J then only does the residual-differences +
  lifted-restriction pass).
- `KB.md` section 7 (known limitations) - check for literal-adaptation mentions.

### Corpus impact estimate (verified heuristically; the definitive sweep is action 6)
Only 4 committed specs use shifts; large literals in set/init contexts are few and all
in-range (`set w = 255` on bit<8>, `init 200` on bit<8>). Expect a SMALL number of
spec fixes (if any) across examples/valid (50), crates/examples/*/pass.rfs (28+),
and training snippets - the new validator run over the whole corpus is the sweep.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3H. | Unify variable-initializer handling across MC modes (E4/S1) | already complete - its residual difference (exact carries unmasked out-of-range inits, SMT width-check rejects) is exactly what requirement 5 eliminates at validation level |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3I. | Tests for the 3F-3H changes + test-suite adoption | recommended to run BEFORE 3L so its parity pins land on the 3F-3H semantics; if 3L runs first, 3I's specs must be written against the new typing rules (rule 4 may reject a draft spec) |
| 3J. | Documentation + README sync | scope GAINS the 3L doc sync only if this task leaves any doc stale - planned so that does not happen (this task updates every doc that states the old rules); 3J's residual-differences section must be written AFTER 3L and must NOT list rule-4/typing items as differences |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/lang/fold.rs` | new | the shared constant-folding stage: `is_const_expr(e) -> bool` (integer literals + operators only, no paths/bools), `fold_const(e) -> Result<i128, String>` (full-precision signed evaluation, overflow detected - NO silent wrap; bitwise ops on negative constants use native i128 two's-complement semantics, which IS the required rule-1 interpretation: `-5i128 & 15` = 11), `fold_tree(e) -> Expr` (replaces every maximal constant subexpression by its folded literal, fixpoint) |
| `src/lang/ir.rs` | change | new `Expr::Cast { width: u32, expr: Box<Expr> }` variant + to_json/from_json arms (additive - IR_VERSION stays 1) + simplify_expr passthrough arm |
| `src/lang/parser.rs` | change | `primary_expr` gains the `bit<N> <primary>` cast arm; `bit` reserved in expression position |
| `src/lang/printer.rs` | change | renders Cast as `bit<N>(operand)` with parens unless the operand is a bare identifier |
| `src/lang/validator.rs` | change | typing rules replaced: IntLit no longer adapts - a constant meeting a bit<N> operand goes through rule 4 (positive not fitting -> error naming the cast escape `bit<N>(...)`; negative two's-complement-fitting -> WARNING via the existing validator warning channel, value = two's complement; negative not fitting -> error; the constant wrapped in `bit<N>(...)` -> always OK, no diagnostic - the cast is the only explicitness mechanism); two bare literals form a FULL-PRECISION constant (no bit<8> cap) that folds; shift result width = shifted value's width, EXCEPT constant-value + variable-amount -> shift amount's width; Cast types as Bit(width), operand must be a bitvector (constants legal - explicit conversion; bool/enum rejected); init expressions get the rule-4 check against the var's declared width |
| `src/mc/ref_mc.rs` | change | `RType`/`expr_type` mirror the new rules (IntLit handling replaced by fold-aware typing; Cast -> Bit(width)); `eval_expr` folds constant subexpressions at full precision before any width masking (a validated spec's surviving constants always fit, so Val::Bit(u64) stays sufficient) |
| `src/mc/smt_plan.rs` | change | `SeType`/`se_type`/`convert_expr` mirror the new rules; folded constants render as single literals (no expression tree); the var-init width check stays as a defensive guard (validated specs never hit it after rule 4) |
| `src/codegen/backends/rust/fsm.rs` | change | `gen_expr` renders Cast (zero-extend: `x as u128`; truncate: `(x as u128) & mask(w)` - both already have the u128-intermediate idiom) and folded constants as literals; `width_from_type` gains the Cast arm |
| `src/codegen/backends/c/fsm.rs` | change | `gen_expr_c` mirrors the Rust backend (Cast + folded literals); C width cap (bit<8/16/32/64, audit row C1) still applies - a cast to bit<128> validates but fails C codegen with the existing C1 error |
| `src/lang/specgen.rs` | change | random expression generator: at minimum a Cast passthrough so generated specs stay valid; optionally emit casts at low probability (recorded if done) |
| `src/cli/info_format/ebnf_grammar.rs` | change | +1 GRAMMAR_RULES entry for the cast production, mirroring DESIGN.md section 6 |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/SEMANTICS.md` | change | 2.2 REWRITTEN: explicit numbered typing rules (equal-width rule as its own statement; constant expressions = full-precision integer literals with the folding stage named; shift width rule incl. the constant-value/variable-amount refinement; the cast operator with binding + zero-extension/truncation semantics; rule 4 constant conversion (positive fit-or-cast, negative two's-complement warning/error, bitwise two's-complement interpretation) with the cast as the only explicitness mechanism; no-implicit-conversion principle stated); 2.4 init semantics rewritten (folding, fit check, empty-vars rejection) retiring footnote `[^init-order]` |
| `DESIGN.md` | change | 5.9: cast added to the supported list + precedence table (new tightest level) + typing paragraph; 6: EBNF +1 production (cast_expr); section 26 or the next free design-decision section: D-P18-x decisions for this arc |
| `docs/user/LANGUAGE.md` | change | section 9: operator table + precedence + typing rules updated to the new explicit rule list, with a short "explicit conversion" subsection and worked examples (the cast, negative-constant warning, two's-complement bitwise ops) |
| `docs/dev/mc_engine.md` (+ any dev fragment naming the old literal rules) | change | SMT typing paragraph + validator deep-dive updated to the new rules |
| `KB.md` | change | section 7 known-limitations rows checked/updated (literal adaptation no longer a limitation; cast added to the supported surface if KB lists it) |
| `docs/llm/SRC_STRUCTURE.md` / `TEST_STRUCTURE.md` | change | inventory rows for fold.rs (new file, new fns) + changed rows (type_expr, expr_type, se_type, gen_expr, gen_expr_c, primary_expr, simplify_expr, to_json/from_json) - same commit per PROTOCOL.md section 5 |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` (or the parser/validator unit suites in `src/lang/{parser,validator}.rs` inline tests) | change | cast syntax + precedence pins (`bit<4> a + b` grouping), rule-4 error pins (out-of-range constant vs var, with and without mask), shift-width refinement pins, cast-of-constant rejection, two-bare-literals full-precision pin (`prop p : 200 * 200 == 40000 ;` must type-check and hold - impossible under the old bit<8> rule) |
| `tests/backend_parity_test.rs` / `tests/wide_ops_parity_test.rs` | change | new parity pins: cast zero-extension + truncation byte-identical across both backends; folded-constant rendering identical; shift-width-refinement boundary cases |
| `tests/ref_mc_smt_test.rs` | change | rule-5 proof: the task-3H divergent specs (`bit<1> init 1 + 1`, `bit<1> init 3`) now rejected at VALIDATION by both modes with the identical message - the old engine-level divergence is unreachable |
| `tests/info_format_test.rs` | change | grammar sync: the new cast rule in GRAMMAR_RULES matches DESIGN.md section 6 and the real parser |
| `examples/valid/*.rfs` (new) | new | at least one spec exercising casts + masked constants + the shift refinement (file-driven suite picks it up with no test change) |
| `examples/invalid/*.rfs` (new) | new | rule-4 violations (unmasked out-of-range constant), cast of a constant expression, mixed-width var+var (already invalid - pin stays), shift-by-bool |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/<cat>/rfs_training.jsonl` + snippets + aggregate | change | only if new examples are fenced into the training set (project rule: category-local, then `cargo extract-training-snippets && cargo build-training && cargo test-training`) |

## Planned actions

### 1. Add the shared constant-folding stage
Create `src/lang/fold.rs` with `is_const_expr`, `fold_const` (i128 arithmetic,
overflow -> Err, unary minus signed), and `fold_tree` (maximal constant
subexpressions replaced by literals, fixpoint). Unit tests inline: full-precision
pins (`200 * 200` = 40000, NOT 16; `-5` = -5; `(300 & 255)` = 44), overflow pin
(`(2^63) * (2^63)` errors), non-const detection (any Path/Bool breaks constness).
Rationale: one shared implementation is the whole point of requirement 1 - validator,
MC and every backend must fold identically, so they all call these three functions
instead of re-deriving widths from IntLit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/fold.rs` | new | the stage itself + inline unit tests |
| `src/lang/mod.rs` (or wherever lang submodules are declared) | change | declare `pub mod fold;` |

### 2. Add the Cast variant to the IR + parser + printer
New `Expr::Cast { width, expr }`; JSON arms (key order pinned per D-P5-2 - additive,
no IR_VERSION bump); parser `primary_expr` arm (`bit<N>` + one primary; `bit`
reserved in expression position); printer form with paren rules. Round-trip test:
parse -> print -> parse is structurally identical for cast expressions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/ir.rs` | change | variant + to_json/from_json + simplify_expr passthrough |
| `src/lang/parser.rs` | change | primary_expr cast arm + precedence tests |
| `src/lang/printer.rs` | change | Cast rendering + round-trip pin |

### 3. Rewrite the typing rules in all three implementations + validator rule 4
Validator first (authoritative): IntLit no longer adapts - a constant meeting a bit<N>
operand goes through rule 4: positive C >= 2^N -> error naming the value, the width,
and the cast escape `bit<N>(...)`; negative C with abs(C) <= 2^(N-1) -> WARNING
(existing validator warning channel) "implicit conversion of negative constant to
unsigned bit-vector: C becomes C mod 2^N (two's complement in bit<N>)"; negative C
with abs(C) > 2^(N-1) -> error (two's-complement form does not fit); the constant
wrapped in `bit<N>(...)` -> always OK with value C mod 2^N and NO diagnostic - the
cast is the ONLY explicitness mechanism, there is no syntactic mask detection (a mask
is just a bitwise op: under rule 1's two's-complement semantics `(-5) & 15` folds to
the positive constant 11, which then converts normally). Literal-vs-literal forms a
full-precision constant; shift width refinement (constant value + variable amount ->
amount's width); Cast typing (Bit(width); operand must be a bitvector - constants legal
as explicit conversion, bool/enum rejected). Then mirror EXACTLY in `ref_mc.rs`
(RType/expr_type) and `smt_plan.rs` (SeType/se_type) - the three must stay word-for-
word equivalent, and a test asserts the equivalence on a corpus of expressions. Rule 4
also applies to init expressions (fit against the var's declared width) - this is
requirement 5.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/validator.rs` | change | new typing rules + rule 4 + init fit check; error messages pinned by tests |
| `src/mc/ref_mc.rs` | change | RType/expr_type mirror + eval_expr folds constant subexpressions at full precision before masking |
| `src/mc/smt_plan.rs` | change | SeType/se_type/convert_expr mirror; folded constants render as single literals; init width check kept as defensive guard |

### 4. Update both codegen backends
`gen_expr` (Rust) + `gen_expr_c` (C): Cast rendering (zero-extend = widen to u128;
truncate = `& mask(w)` - the existing u128-intermediate idiom applies unchanged),
folded constants rendered as bare literals, `width_from_type` Cast arm. Parity pins:
cast + fold + shift-refinement specs produce byte-identical behavior across backends
(existing parity machinery). The C width cap (C1) still rejects casts to bit<128> at
codegen with the existing error.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/rust/fsm.rs` | change | gen_expr + width_from_type Cast/fold arms |
| `src/codegen/backends/c/fsm.rs` | change | gen_expr_c mirror |

### 5. Update the remaining Expr consumers + grammar model
`specgen.rs` (Cast passthrough so random specs stay valid), `plan.rs` / `info.rs` /
`test_plan.rs` match arms (mechanical - the variant must not be silently dropped),
`ebnf_grammar.rs` +1 rule mirroring DESIGN.md section 6. `monitor.rs` verified
unchanged (VarExpr clone-through).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/lang/specgen.rs`, `src/mc/plan.rs`, `src/cli/commands/info.rs`, `src/codegen/test_plan.rs` | change | mechanical Cast arms |
| `src/cli/info_format/ebnf_grammar.rs` | change | +1 GRAMMAR_RULES entry |

### 6. Examples first, then the corpus sweep
Add the new valid example (casts + masked constants + shift refinement) and invalid
examples (rule-4 violation, cast-of-constant, mixed-width var+var pin) BEFORE
finishing actions 3-5 where practical (project discipline). Then run `rfs validate`
over ALL of examples/valid (50), examples/invalid (13 - must still fail, ideally with
the same or a better message), crates/examples/*/pass.rfs + full.rfs (28+ twins), and
training snippets: every spec that now fails rule 4 gets the minimal fix (explicit
mask) and its README/captures regenerated if pinned output moved. Record every fix.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/<new>.rfs`, `examples/invalid/<new>.rfs` | new | the discipline examples |
| any corpus spec hit by rule 4 | change | minimal mask fix + capture regeneration where pinned |

### 7. Rewrite the normative docs
SEMANTICS.md 2.2 (explicit numbered rule list per the Context section) + 2.4 (init
semantics, retire `[^init-order]`); DESIGN.md 5.9 + 6 (+ design decisions);
LANGUAGE.md section 9; dev fragments naming the old rules; KB.md section 7 check.
The docs_md_test + dev_docs_test gates must pass on the rewritten text.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md`, `DESIGN.md`, `docs/user/LANGUAGE.md`, `docs/dev/*.md` (as found), `KB.md` | change | the new rules stated exactly once in SEMANTICS 2.2, mirrored briefly elsewhere |

### 8. Inventory sync + full matrix
`rfs-dev inventory add/update` for fold.rs + every changed row (same commit per
PROTOCOL.md section 5), `rfs-dev inventory audit` EXACT, then the full verification
matrix: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt`,
`cargo test --features smt`, `cargo test-generated-code` (1683 baseline - moves only
if a new valid example is fenced into training), docs gates. Every pre-existing pin
stays byte-identical EXCEPT corpus specs deliberately fixed in action 6 (recorded).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/SRC_STRUCTURE.md`, `docs/llm/TEST_STRUCTURE.md` | change | inventory rows for the new file + changed fns |

## Task deliverables
- `src/lang/fold.rs`: the shared full-precision constant-folding stage used by
  validation, mc-ref and all backends.
- The `bit<N>(expr)` cast: parser + IR variant + printer + JSON + both backends +
  grammar model.
- The explicit typing rule set (equal widths, shift refinement, rule 4 constant-fit
  with the cast as the only explicitness mechanism) implemented identically in
  validator + ref_mc + smt_plan and
  stated as a numbered rule list in SEMANTICS.md 2.2.
- The task-3H init divergence eliminated at validation level (requirement 5).
- New valid/invalid examples, corpus sweep fixes, rewritten docs, green full matrix.

## Task exit criteria
- [ ] `prop p : 200 * 200 == 40000 ;` validates and PASSes under exact AND
      smt-bounded (full-precision constants - impossible under the old bit<8> rule).
- [ ] `var x: bit<8>; set x = x + 300;` is a VALIDATION error naming 300, bit<8>,
      and the cast escape `bit<8>(300)`; `set x = x + (300 & 255);` validates (folds
      to positive 44) and both MC modes agree on the verdict.
- [ ] `var x: bit<1> init 1 + 1 ;` and `init 3` are validation errors in BOTH modes
      with the identical message (requirement 5 - the task-3H divergence is gone);
      `grep -rn "requires literal initial values" src/` still returns nothing.
- [ ] `bit<4> a + b` groups as `(bit<4> a) + b` (parser pin); cast zero-extension and
      truncation are byte-identical across both backends (parity pin); cast of a
      constant expression is ACCEPTED as an explicit conversion (`bit<2>(5 + 1)` = 2);
      `bit<4>(true)` / `bit<4>(Mode::Run)` rejected (bitvector operands only).
- [ ] Negative constants: `var a: bit<4> init -5 ;` validates with the WARNING
      "implicit conversion of negative constant to unsigned bit-vector" and initial
      value 11 under BOTH MC modes; `var a: bit<3> init -5 ;` is an error in both;
      `init bit<4>(-5)` and `init bit<3>(1 - 6)` produce NO warning (explicit cast);
      `init (-5) & 15` produces no warning because rule 1 folds the AND to the
      positive constant 11.
- [ ] Rule-1 bitwise pins: `prop p : 2 + -3 == -1 ;`, `prop p : 2 + (-1) & 1 == 3 ;`
      and `prop p : 2 + (-4) & 3 == 2 ;` all validate and PASS under exact AND
      smt-bounded (two's-complement interpretation of negative constants in bitwise
      ops).
- [ ] Shift refinement: `1 << k` with `k: bit<N>` types as bit<N> (was bit<8>) -
      pinned in all three typing implementations by an equivalence test.
- [ ] SEMANTICS.md 2.2 carries the explicit numbered rule list; DESIGN.md 5.9/6 +
      GRAMMAR_RULES + LANGUAGE.md agree with the parser (info_format sync test green);
      `[^init-order]` retired.
- [ ] Corpus sweep: every committed spec validates (or is minimally fixed + recorded);
      examples/invalid still all fail; full matrix green at or above baseline with all
      un-fixed pins byte-identical; inventory audit EXACT.

## Non-goals
- No division, no new operators beyond the cast (audit rows P6 etc. stay deferred).
- No change to pattern-matching literal rules (V8a/V8b - `on E(V(300))` fit checks are
  a separate surface and stay as-is).
- No C-backend width cap change (C1 stays: casts to bit<128> validate, C codegen
  rejects with the existing error).
- No monitor-subset (R1-R4) changes - monitors wrap whole expressions and inherit the
  new semantics through the backends.
- No IR_VERSION bump (additive variant per START.md section 8 policy).
- No training-run changes (`training.py` untouched; data only if examples are fenced).

## Other information
- RESOLVED design decisions (user answers, 2026-11-25, simplified 2026-11-25):
  (a) Negative constants: C < 0 converts to bit<N> WITHOUT explicit conversion only
  when abs(C) <= 2^(N-1) - i.e. the two's-complement code of C fits the width with the
  sign bit set - and that implicit step is a WARNING (value = two's complement);
  non-fitting negative -> ERROR; an explicit `bit<N>(C)` cast is always OK (value = C
  mod 2^N, low N bits of the two's-complement form) and suppresses the warning.
  Rationale: programmers often use -1 for all-bits-set - familiar and convenient, but
  the implicit step must be visible. The earlier "explicit target-length mask silences
  the warning" mechanism was REMOVED as fragile (it needed syntactic heuristics);
  masking now works naturally because rule (b)-below makes `(-5) & 15` fold to the
  positive constant 11.
  (b) Bitwise ops on negative constants use the unsigned two's-complement
  representation of the constant (full precision): `2 + -3 == -1`, `2 + (-1) & 1 ==
  3`, `2 + (-4) & 3 == 2`. Implementation note: native i128 bitwise ops already have
  exactly these semantics, so `fold_const` needs no special case.
  (c) Cast operands: bitvectors ONLY (bool/enum rejected); cast of a constant
  expression is LEGAL and is the ONLY explicit conversion mechanism.
  (d) Comparisons with out-of-range constants (`x == 300`, x: bit<8>): ERROR -
  "any operation" includes comparisons; truncating to `x == 44` would silently change
  meaning.
  (e) Execution order: 3I first, then 3L, then 3J (see the Dependent tasks table and
  the 3J task file's dependency note).
- The final rules-with-examples table (15 rows) lives in `PHASE_18_TASK_3J.md`
  (Context section "Final typing/conversion rules with examples") - task 3J records it
  in SEMANTICS.md 2.2 during the documentation pass; this task implements exactly what
  that table specifies.
- The three typing implementations must be kept word-for-word equivalent; if a future
  session finds them drifting, that is a bug to fix in the same commit (the
  equivalence test from action 3 is the guard).
- `fold_const` overflow policy: i128 with detected overflow -> validation error
  "constant expression overflows" - NEVER silent wrap (the main principle).
