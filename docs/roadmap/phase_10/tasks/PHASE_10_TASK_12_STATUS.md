# Phase 10, Task 12 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_12.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | 156 function rows (ref_mc.rs 100, specgen.rs 53, spec_hash.rs 3) + 31 type rows (ref_mc.rs 26 incl. the nested-in-run St enum, specgen.rs 5: Rng/Bounds/SpecGen/MutOp/Env) appended to `## All functions` / `## All types`; Component/Functionality still blank; preamble extended with the nested-type identifier form `<file>::<parent_fn>::<Type>` (first use: ref_mc.rs::run::St) |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_12.md` | changed | stale raw-grep figures in Affected documentation files + Planned actions (~193/~32, ~124, ~69, ~32) corrected to the test-aware ~156/~31, ~100, ~56, ~31 - the Goal and cluster notes were already right, no plan correction needed |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The independent reference-infrastructure cluster is now dependency-annotated: ref_mc.rs is the brute-force ground-truth engine with DELIBERATELY different data structures (Vec frontier, structured CfgKey tuples - no bit-packing) and its own evaluator/monitor compiler/input abstraction/liveness SCC analysis implementing SEMANTICS.md sections 2-9 directly, while reusing codegen_mc's DATA TYPES + pure report helpers for D-P6-10 byte-comparability; specgen.rs is the seeded generator sharing the toolchain's LCG lineage (same constants as codegen_test::Lcg) with validity by construction + the 17-op incremental mutate API behind a per-step validator gate; spec_hash.rs is the hand-rolled SHA-256 (FIPS 180-4, no dependencies) behind the D-P6-3 artifact stamping.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run caught two false relative links (`[N](input_is)` / `[N](output_has)` in random_formula's template list parsed as markdown links); fixed by code-span-wrapping per task 2's precedent, second run green. Literal pipes in ref_mc's operator-list descriptions (bin_str, expr_has_boolean_op, print_formula_ctx, eval_expr) were escaped up front |

## Found problems/issues/mismatches

- None vs the plan: correction 2a's "T12 156/31" held exactly per file (ref_mc.rs 100/26, specgen.rs 53/5, spec_hash.rs 3/0). The task file's own Planned-actions figures (~193/~32) were raw-grep counts INCLUDING the inline test fns (100+24=124 for ref_mc alone) - corrected in place, same class as tasks 8-10.

## Other important findings/information

- Line-based counting hazard found and resolved during re-verification: specgen.rs's byte-pinned generated-spec raw string contains a column-0 `}` (the module's closing brace in the pinned source text), which naively closes the test region early - a naive counter reports 54 non-test / 10 test fns instead of the authoritative 53/11. The miscounted "extra" fn is the #[test] generated_source_byte_pin_seed_one_small after the raw string. Read-through (correction 2a's own rule) establishes 53/11.
- ref_mc.rs::run::St (the BFS status enum declared inside run()) is the first TYPE nested in a function body in the inventory - the preamble's nested-fn convention extended to `<file>::<parent_fn>::<Type>`; no other row references it (it is private to run).
- With this chunk, EVERY forward edge in the inventory resolves: zero unresolved Depends identifiers remain (verified by full-table scan). The remaining tasks are classification fills (15-16), file mapping + proposed layout (17-18), and the test inventory (29-30).
- ref_mc.rs's independence contract is visible in its rows: no depends on mc_plan/monitor/codegen_fsm/printer/backend modules - only ir, validator-free construction, codegen_mc data types + report helpers, and json_rw for the JSON report.
