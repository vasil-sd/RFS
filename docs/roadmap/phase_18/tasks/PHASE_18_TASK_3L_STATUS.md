# Phase 18, Task 3L - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3L.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/lang/fold.rs` | the shared constant-folding stage (task 3L requirement 1): `is_const_expr`, `fold_const` (signed i128, overflow -> error, native two's-complement bitwise/shift), `fold_tree`, and `const_fit` (rule 4 - moved here from validator.rs as the shared rule-4 conversion check) + inline unit tests (full-precision pins, two's-complement pins, overflow pin, non-const detection) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/lang/ir.rs` | changed | new `Expr::Cast { width, expr }` variant + to_json/from_json arms (additive - IR_VERSION stays 1) + simplify_expr passthrough |
| `src/lang/parser.rs` | changed | `primary_expr` gains the `bit<N> <primary>` cast arm (tightest binding, exactly one primary; `bit<4> -a` does not parse); +5 parser tests (grouping pins, operand pin, round-trip) |
| `src/lang/printer.rs` | changed | renders Cast as `bit<N>(operand)` with parens unless the operand is a bare identifier (round-trip fidelity) |
| `src/lang/mod.rs` | changed | declares `pub mod fold;` |
| `src/lang/validator.rs` | changed | typing rewritten to the explicit no-implicit-conversion rules: IntLit is full-precision (no bit<8> default), rule-4 `const_fit` at compare/arith/shift sites, Cast typing (bitvector operands only - constants legal), shift refinement; validate_module gains the rule-4 init fit check (requirement 5); module_warnings gains the same check so the negative-constant WARNING actually surfaces (it was discarded before); const_fit moved to fold.rs; +12 tests |
| `src/mc/ref_mc.rs` | changed | RType/expr_type/binary_type/compatible/bit_op_type mirror the validator WORD-FOR-WORD (incl. rule-4 fit checks and Cast operand rejection - previously missing); eval_expr folds maximal constant subexpressions at full precision before masking, const-const comparisons compare signed, shift arm uses the expression's type width; RType::width returns None for IntLit; mask/expr_type made pub(crate); +equivalence test (27-expression corpus) |
| `src/mc/ref_mc_exact.rs` | changed | init values masked to the var's declared width - a two's-complement-fitting negative init (`init -5` on bit<4>) stores its encoding (11), not the unmasked 64-bit value |
| `src/mc/smt_plan.rs` | changed | SeType/se_type/bit_op_type mirror the new rules (full-precision constants, shift refinement, Cast debug_assert); convert_expr folds constant subexpressions to single literals + lowers Cast; init masked to width before the defensive fit check (which validated specs never hit) |
| `src/mc/smt_emit.rs` | changed | full-precision literal rendering (mask to context width), cmp_widths_full widening for oversized constants, shift_amt_width for folded amounts; render_shift_rust/c result width fixed to the EXPRESSION's type (was the left operand's - a constant-value shift rendered 8 bits wide and mismatched the parent ite) |
| `src/mc/ref_mc_smt.rs` | changed | se_to_z3 shift arm: same result-width fix as the emitters (the executor panicked via Z3_mk_ite NULL on `set x = 1 << k` with x narrower than bit<8>) |
| `src/codegen/backends/rust/fsm.rs` | changed | gen_expr renders Cast (u128 intermediate + mask = zero-extension or truncation) and folds constant subexpressions to bare literals; width_from_type IntLit fallback documented as unreachable-defensive |
| `src/codegen/backends/c/fsm.rs` | changed | gen_expr_c mirrors the Rust backend (Cast + folded literals); C width cap (C1) unchanged |
| `src/mc/plan.rs` | changed | collect_paths recurses through Cast (a cast-wrapped var reference still counts as a use) |
| `src/cli/commands/info.rs` | changed | render_expr Cast arm (display) |
| `src/cli/info_format/ebnf_grammar.rs` | changed | +1 GRAMMAR_RULES entry for the cast production, mirroring DESIGN.md section 6 |
| `src/cli/info_format/mod.rs` | changed | grammar-model registration for the new rule |
| `src/codegen/test_plan.rs` | changed | Cast arms in the static evaluator (planned test cases inherit the new semantics) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/SEMANTICS.md` | changed | 2.2 REWRITTEN as explicit numbered rules R1-R5 (full-precision constants + folding stage named, equal-width rule standalone, shift refinement, the cast operator, rule-4 constant conversion with the cast as the only explicitness mechanism, two's-complement bitwise interpretation); 2.4 init semantics rewritten (constant expressions, empty-vars rejection stated, rule-4 fit check) retiring footnote `[^init-order]` |
| `DESIGN.md` | changed | 5.9: cast added to the supported list + precedence table (new tightest level, rows renumbered 1-12); 6: EBNF +1 production (cast in primary_expr) - machine-checked by info_format_test |
| `docs/user/LANGUAGE.md` | changed | section 9: operator table gains the cast row, precedence list updated (cast tightest), typing rules replaced with the explicit no-implicit-conversion rule list, new "Explicit conversion and constant-to-bitvector rules" subsection with worked examples |
| `docs/dev/validator.md` | changed | type-lattice paragraph rewritten (IntLit full-precision, rule 4, cast); const_fit references moved to fold.rs |
| `docs/dev/backend_rust.md` | changed | width_from_type paragraph updated (IntLit unreachable in eval arms) |
| `docs/dev/ALGORITHMS.md` | changed | expression type-lattice paragraph updated (full-precision IntLit) |
| `docs/dev/mc_engine.md` | changed | SMT typing paragraph updated (full-precision constants, shift refinement; the v1-scope-diagnostics staleness is task 3J's) |
| `docs/llm/SRC_STRUCTURE.md` | changed | inventory: const_fit row moved validator.rs -> fold.rs; ~15 rows updated (RType, RType::width, compatible, bit_op_type x2, binary_type x2, eval_expr, se_type, convert_expr, type_expr, validate_module, module_warnings, primary_expr, width_from_type, collect_paths) - same commit per PROTOCOL.md section 5 |
| `docs/llm/TEST_STRUCTURE.md` | changed | +23 rows for the new tests; llm-budget 1475 -> 1494 |
| `docs/llm/TASK_MAP.md` | changed | fold.rs entry + cast-related pointers |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/valid/52_explicit_casts.rfs` | new | the discipline example: casts (truncation + zero-extension), a masked constant `(200 & 15)`, the shift refinement, and the full-precision property `200 * 200 == 40000` |
| `examples/invalid/invalid_rule4_out_of_range_constant.rfs` (+ `.rfs-validation.err`) | new | rule-4 violation: `set count = count + 300` on bit<8> - the error names the value, width, and cast escape |
| `examples/valid/44_token_ring.rfs`, `examples/valid/45_elevator.rfs` | changed | corpus sweep fixes: `1 << node`/`1 << pos` now `bit<8>(1) << ...` - the shift refinement types a bare-literal shift as the amount's width, so the explicit cast restores the bit<8> mask value (recorded, minimal) |
| `crates/examples/json_schema_ci/work/schemas/spec-json.json` | changed | regenerated: the IR JSON schema gains the additive `cast` expression kind |
| `crates/examples/mc_walkthrough/captures/fail-rust.json` | changed | regenerated capture (byte-identical semantics; re-emitted by the suite) |

## Changed functionality
- Integer-literal typing: literals no longer ADAPT to the other operand's width and two bare literals no longer form a bit<8> - constant expressions are full-precision integer literals folded at compile time (signed i128, overflow is an error); a constant meeting a bit<N> operand goes through rule 4 (positive out-of-range = error naming the cast escape; negative two's-complement-fitting = warning, value = two's complement; negative not fitting = error; explicit `bit<N>(...)` cast = always OK, no diagnostic).
- Shift typing: result = shifted value's width, EXCEPT a constant value shifted by a variable amount, which now takes the shift amount's width (was bit<8>) - `set x = 1 << k` with x/k: bit<3> validates where it was rejected before.
- Variable initializers: constant-expression inits are fit-checked against the declared width at VALIDATION (both MC modes reject identically - the task-3H divergence is gone); two's-complement-fitting negative inits store their encoding in both engines (exact mode no longer carries the unmasked 64-bit value; SMT no longer rejects what exact accepted).
- Comparison/assignment with an out-of-range constant (`x == 300` on bit<8>) is now a validation error (user decision d: "any operation" includes comparisons).

## Removed functionality
- The implicit "integer literal adapts to the other operand's width; two bare literals form a bit<8>" conversion rule (SEMANTICS.md 2.2 old text) - replaced by the explicit R1-R5 rule set.
- The `[^init-order]` footnote claim that a var-referencing init "does not type-error but generates code that cannot compile" - both engines now reject such inits at entry with the identical message, and out-of-range constant inits are validation errors.

## New functionality
- The explicit conversion operator `bit<N>(expr)`: tightest binding (exactly one primary operand), zero-extension or high-bit truncation, bitvector operands only (a constant expression IS a legal operand - the cast is the only explicitness mechanism; for any constant C, `bit<N>(C)` = C mod 2^N). Parser + IR variant (additive, no IR_VERSION bump) + printer + JSON + both codegen backends + SMT encoding + grammar model.
- Full-precision constant folding as a shared stage (`src/lang/fold.rs`) used by validation, both MC engines, and both codegen backends - identical folding everywhere, overflow detected (never a silent wrap).
- Two's-complement interpretation of negative constants in bitwise operations (full precision): `2 + ((-1) & 1) == 3`, `2 + (-4) & 3 == 2`; masking a negative constant folds to an ordinary positive constant (`(-5) & 15` = 11) with no diagnostic - no syntactic mask detection needed.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `src/lang/fold.rs` inline tests | new | full-precision pins (200*200=40000), two's-complement bitwise pins, overflow pin, non-const detection, fold_tree fixpoint |
| `src/lang/parser.rs` tests (5) | new | cast grouping (`bit<4> a + b` = `(bit<4> a) + b`, same for `*`), operand-is-one-primary pin, `bit<4> -a` rejection, printer round-trip |
| `src/lang/validator.rs` tests (12) | new | rule-4 error/warning/cast-suppression pins, masked-negative no-warning pin, comparison rule-4 pin, cast-operand rejection, two-bare-literals full-precision pin, overflow pin, shift-refinement pin, bitwise two's-complement pins |
| `src/mc/ref_mc.rs::tests::typing_equivalence_validator_vs_ref_mc` | new | the three-implementation equivalence guard: validator type_expr vs ref_mc expr_type word-for-word on a 27-expression corpus (smt se_type pinned end-to-end through smt-bounded runs) |
| `tests/ref_mc_smt_test.rs` rule5 tests (2) | new | requirement 5 proof: the task-3H divergent inits (`bit<1> init 1 + 1`, `init 3`) rejected at validation with identical messages; engine-level divergence gone (both mask 2 -> 0) |
| `tests/wide_ops_parity_test.rs` cast parity test | new | generated Rust and C render casts + folded constants + the shift refinement byte-identically; pinned values 0 0 0 0 / 100 0 0 0 / 100 4 0 0 / 5 4 1 1 |
| `tests/backend_parity_test.rs` | changed | sweep pin for example 52 (pass-complete, 2752 states); invalid corpus count 13 -> 15 |
| `tests/info_format_test.rs` | changed | grammar sync: the cast rule in GRAMMAR_RULES matches DESIGN.md section 6 and the real parser; invalid corpus pin 15 |
| `tests/codegen_test.rs`, `tests/common/fail_variants.rs`, `tests/psl_translation_test.rs` | changed | Cast arms / updated pins for the new expression variant |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (main, 639) | OK |
| `cargo test --features smt` (672) | OK |
| `cargo test-generated-code` (1734) | OK (baseline 1683; +51 from new valid examples fenced into training snippets - recorded, not a regression; all pre-existing modules byte-identical) |
| `cargo clippy --all-targets` (+ `--features smt`) | OK - no errors; warning baseline unchanged by this task (remaining warnings are pre-existing patterns in shared test modules / c/fsm.rs etc.) |
| `cargo fmt --check` | OK - clean |
| `rfs-dev inventory audit` | OK - EXACT: SRC 1037 fns / 217 types, TEST 1146 fns / 35 types, zero per-file diffs |
| `rfs-dev sync-check` | OK - clean |
| corpus sweep (`rfs validate` over examples/valid 52 + crates/examples pass.rfs/full.rfs) | OK - all validate; all 15 invalid examples fail |

## Found problems/issues/mismatches
- The rule-4 negative-constant WARNING was unreachable: validate_module discarded its warning vector and module_warnings never applied the init-width check - `init -5` on bit<4> validated silently. Fixed in module_warnings (see Task corrections).
- Both engines mishandled two's-complement-fitting negative inits: exact mode carried the unmasked 64-bit value (2^64-5 instead of 11), SMT rejected it with "does not fit in N bits". Fixed by masking at both init sites.
- SMT shift rendering computed the result width from the left operand's type - a constant-value shift (`1 << k`) rendered 8 bits wide and mismatched the parent assignment ite: the executor panicked (Z3_mk_ite NULL) and the generated backends would have emitted mismatched-width code. The shift refinement made this path reachable for the first time. Fixed in the executor + both emitters.
- eval_expr's shift arm read the width from the left operand's type via RType::width(), which became None for IntLit under rule 1 - `set x = 1 << k` deadlocked (the set evaluation errored, disabling the only transition). Fixed to use the expression's type.
- ref_mc's typing accepted constant-vs-bitvector forms without the rule-4 fit check the validator applies (`x == 300` on bit<8> typed Ok there, Err in the validator) - a word-for-word divergence on invalid input. Fixed by mirroring const_fit at the same sites.
- Precedence mismatch: the user's rule-1 example `2 + (-1) & 1 == 3` assumes bitwise AND binds tighter than addition (mathematical convention), but RFS's documented precedence is C-style (`+` tighter). Pinned the parenthesized equivalent; a precedence change would be a separate breaking task (see Task corrections).

## Other important findings/information
- `specgen.rs` needed NO change: its expression walkers carry wildcard arms, so Cast flows through harmlessly and generated specs stay valid (the task file's "at minimum not break" bar); emitting random casts was optional and was NOT done.
- The 3J rules table (row 10) must use the parenthesized form `2 + ((-1) & 1) == 3` when recorded in SEMANTICS.md 2.2 - noted in the Task corrections so the documentation pass does not re-introduce the unprefixed form.
- Environmental: `diagram_render_parity_test`'s external mmd half needs `PUPPETEER_CACHE_DIR=/workspace/host/.cache/puppeteer` exported (the sandbox HOME cache is ephemeral - TOOLCHAIN.md section 6.4); not a code issue.
- The C width cap (C1) still rejects casts to bit<128> at codegen with the existing error while validation accepts them - unchanged by design (Non-goals).
