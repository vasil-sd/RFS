# Phase 3, Task 23 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_23.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | all changes are to existing files |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/parser.rs` | changed | `primary_formula` fallthrough redirected from the full `expr` grammar to a new named wrapper `non_boolean_expr` (the `equality` level, i.e. non-boolean expressions) with a comment explaining why the full chain must not be used here; 6 new unit tests pinning the fixed grammar |
| `src/printer.rs` | changed | `needs_parens` re-derived for the fixed grammar: `Formula::And` bare in every formula context, `Formula::Or` parenthesized only in And-child/Implies-left positions, plain `Expr` children bare under `\|\|`/`&&`; new `expr_has_boolean_op` helper keeps old-JSON Expr shapes (carrying `LogicAnd`/`LogicOr`) parenthesized; `Not` printing arm extended to parenthesize `Or`/`And` inners; 1 test updated, 1 test added, 6 round-trip cases added |
| `src/monitor.rs` | changed | 1 new plan-structure test: `always (state == A && output_has(Send(Ack)))` compiles as an And of two per-sample RangeChecks (mixed conjunctions are in-subset, no R1-R4 change) |
| `src/validator.rs` | changed | stale correction-4a comment in `bounded_always_zero_bound_nested_is_error` updated (bare expression children of `&&` no longer need parentheses); no behavior change |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_3_TASK_23.md` | changed | exit criteria checked; "Task corrections" + "Changes in design decisions" sections appended (printer scope expansion, old-shape test updates) |
| `docs/roadmap/PHASE_3_TASK_23_STATUS.md` | new | this file |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| (none in the repo) | - | no examples, training sets, or build files touched; `examples/valid/42_bounded_liveness_protocol.rfs` verified unmodified (git: not in the change set) |

## Changed functionality

- Parsing of property/assumption formulas:
  - formula-level `&&`/`||` now accept any atom kinds on either side; previously a bare state comparison on the LEFT with a non-comparison right operand (e.g. `state == S && output_has(O)`) failed with "trailing input after RFS source" because the expr fallthrough absorbed `output_has` as a variable path
  - pure-expression conjunctions/disjunctions in properties (`x && y`) now parse as `Formula::And(Expr, Expr)` / `Formula::Or(Expr, Expr)` instead of `Expr(Binary(LogicAnd/LogicOr))`
- Printing of formulas:
  - minimal output for formula-level booleans with plain-expression operands (`state == A && state == B`, no redundant parens; previously printed as `(state == A) && (state == B)`)
  - `a || b -> c` now parses as `a || (b -> c)` (`->` binds tighter than `||`) and prints with the implication parenthesized

## Removed functionality

- None. The old Expr-level boolean shape is no longer produced by the parser, but old JSON IR still deserializes and prints semantically equivalently (retained parentheses); no `IR_VERSION` bump.

## New functionality

- Parseable: `state == A && output_has(O)`, `state == A || output_has(O)`, `state == A && (output_has(O))` and all previously-working shapes (reordered operands, left-parenthesized comparisons, comparison mixes)
- Formula-level precedence pinned: `a && b || c` -> `Or(And(a,b),c)`
- Printer round-trips (rfs->json->rfs, json->rfs->rfs) for all new shapes with minimal parentheses
- Monitor compiler acceptance of mixed conjunctions (`always (state == A && output_has(O))`) pinned by a plan-structure test
- `examples/valid/42_bounded_liveness_protocol.rfs` now parses UNMODIFIED (hand-off to task 13)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `parser::tests::mixed_conjunction_state_comparison_left_parses` | new | the exact pre-fix failing shape parses as `Formula::And(Expr(Eq), OutputHas)` + round-trip |
| `parser::tests::mixed_disjunction_state_comparison_left_parses` | new | `state == A \|\| output_has(O)` parses as `Formula::Or(Expr, OutputHas)` + round-trip |
| `parser::tests::pre_fix_working_shapes_still_parse` | new | regression guards: reordered, left-parenthesized, comparison/comparison mixes keep their top operator |
| `parser::tests::pure_expr_conjunction_parses_as_formula_and` | new | pins the new AST shape `Formula::And(Expr, Expr)` for `x && y` |
| `parser::tests::formula_level_precedence_and_tighter_than_or` | new | `a && b \|\| c` -> `Or(And(a,b),c)` |
| `parser::tests::comparison_mix_keeps_both_comparisons` | new | `x == y && z == w` keeps both comparisons as separate operands |
| `printer::tests::formula_precedence_roundtrips` | changed | 6 new cases for the mixed shapes; stale pre-fix comments updated |
| `printer::tests::formula_printing_keeps_implicit_precedence` | changed | now asserts the Formula-level shape: fully-bare `&&`/`\|\|` line + implication parenthesized right of `\|\|` (previously pinned the old Expr-level shape) |
| `printer::tests::formula_and_or_with_expr_children_print_minimal` | new | no redundant parens for And/Or with Expr children; Not keeps its inner group; both round-trip directions |
| `monitor::tests::plan_structure_mixed_conjunction_is_in_subset` | new | mixed conjunction compiles as And of two per-sample RangeChecks over the full range |
| `validator::tests::bounded_always_zero_bound_nested_is_error` | changed | comment only (correction-4a note obsolete); assertion unchanged |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --no-fail-fast` lib unittests (86) | OK |
| `tests/codegen_test.rs` (19; was 16/19 before the fix) | OK |
| `tests/examples_test.rs` (3; was 2/3 - example 42 now parses) | OK |
| `tests/frontend_test.rs` (41) | OK |
| `tests/integration_example_test.rs` (2) | OK |
| `tests/library_comparison.rs` (4) | OK |
| `tests/training_test.rs` (4) | OK |
| `cargo run -q -- parse --input rfs --output json examples/valid/42_bounded_liveness_protocol.rfs` | OK (exit 0, unmodified source) |

## Found problems/issues/mismatches

- Pre-existing clippy + rustfmt dirt from commit e0bc3cb (previous session's Phase 3 work), OUT of task 23 scope (backend files): 11 lib warnings in `src/backends/c_fsm.rs` (9) / `src/codegen.rs:232` / `src/validator.rs:754`, 1 test warning in `tests/codegen_test.rs:25` (`&PathBuf`), and rustfmt diffs across `src/backends/c_fsm.rs` (18 sites). None are in the files changed by this task. MUST be fixed before the task 22 close-out checklist (`cargo clippy --all-targets && cargo fmt --check`).
- Toolchain incident during this execution: cargo's rustc target probe began failing ("failed to run `rustc` to learn about target-specific information") with corrupted probe stdin; `target/.rustc_info.json` already held a failed probe result dated 2026-10-16 22:58 (previous session). While diagnosing with a temporary rustc wrapper, the toolchain's `rustc` binary was accidentally deleted. Restored by copying the same-build binary (rustc 1.99.0 b940084d7) from `/opt/unsloth-studio/sandbox/__LOCALID_nkBnQA0/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc`; verified `--version`, standalone compilation against the toolchain libstd, and that cargo's probe + full build/test then succeed. Note: `/opt/rust-old/usr/bin/rustc` is 1.58.1 despite looking like a candidate - do NOT use it.

## Other important findings/information

- The fix unblocked the whole remaining Phase 3 chain at the parse level: all four baseline failures recorded in `PHASE_3_TASK_13.md` (codegen_test 16/19, examples_test 2/3) were caused by example 42 not parsing and are now gone.
- `TOOLCHAIN.md` section 3 only covers reinstalling the whole rustup/cargo tree; single-binary recovery (as needed here) is undocumented - candidate for a close-out doc update (task 21 scope).
- Correction 13a's workaround (reordering example 42's antecedent) is now obsolete: example 42 keeps its original text and parses as-is.
