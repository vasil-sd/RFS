# Phase 16, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_16_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/mc/ref_mc_exact.rs` | the exact-mode engine moved verbatim from ref_mc.rs: BNode/CfgKey/val_key/bnode_key, run() BFS (now pub(crate)), path_to/state_name/io_in/io_out/trace_steps, liveness analysis (LivWitness/LeafSig/leaf_signals/skel_eval/liv_analyze/prod_delta/leaf_good/leaf_bits/liv_note), minimal_violation_labels/collect_min_labels + the 16 engine unit tests (with duplicated module/cfg helpers). The six liveness helpers the CE-verification re-derives + LeafSig fields are pub(crate) for import back into ref_mc.rs. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/mc/ref_mc.rs` | changed (4971 -> 3347 lines) | now the common core + entry points: a one-line delegating pub fn run forwards to ref_mc_exact::run; imports back state_name/leaf_signals/leaf_bits/prod_delta/leaf_good/skel_eval for counterexample verification. Widened to pub(crate): state_variants, eval_expr, Sig, eval_sig, flatten, Mon, compile_formula, compile_signal, MonSt, mon_init, mon_step, mon_key, mon_bits, ceil_log2, all_inputs, Effect, match_effect, Leaf, Skel, LivProp, contains_liveness, decompose_properties, Range, MonSt::trigger_step. Header rewritten (common-core role + D-P16-1); //! Docs: now owns overview/verify only; the 16 engine tests removed from its test module; independence_import_audit extended to read ALL src/mc/ref_mc_*.rs with the per-file exceptions (smt: mc::plan + smt_plan; fuzz: utils::rng) - final-form for tasks 3-4. |
| `src/mc/mod.rs` | changed | + pub mod ref_mc_exact; doc comment scoped per file (exact/fuzz: common core only; smt: plan + smt_plan model chain). |
| `tests/ref_mc_diff_test.rs` | changed | the one external call site of the moved pub API minimal_violation_labels updated to rfs::mc::ref_mc_exact:: (doc comment + line 444). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed | 20 moved rows re-filed under src/mc/ref_mc_exact.rs; the engine run row added there and the old ref_mc.rs::run row re-described as the delegator (depends = ref_mc_exact::run); nested run::St type row moved. All depends-column references to moved items re-pointed (incl. the staying verify_lasso/check_* rows). |
| `docs/llm/TEST_STRUCTURE.md` | changed | 16 engine test rows moved under ref_mc_exact.rs (their depends re-pointed to the same file's module/cfg/run); duplicated module/cfg helper rows added; llm-budget 1325 -> 1327. |
| `docs/llm/TASK_MAP.md` | changed | exact/liveness/verify functionality rows now reference ref_mc_exact.rs (the gate requires every manifest file to be covered). |
| `docs/dev/mc_engine.md`, `docs/dev/ALGORITHMS.md`, `docs/dev/DEPENDENCIES.md`, `docs/llm/mc_engine.md` | changed | 31 doc->code references re-pointed from ref_mc.rs to ref_mc_exact.rs for the moved items (item-level rewrite; ref_mc.rs::run references kept - the delegating entry point still resolves). |
| `tests/dev_docs_manifest.txt` | changed | + src/mc/ref_mc_exact.rs (the manifest is a filesystem scan - the new file must be listed). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | appended | the task-1 entry. |

## Changed functionality
- None observable: this is a pure relocation. rfs::mc::ref_mc::run, RefConfig,
  render_text, to_json_report keep their signatures and behavior; the CLI and all
  differential suites compile against them unchanged (only the one
  minimal_violation_labels call site moved with the item).

## Removed functionality
- None.

## New functionality
- The per-file independence audit now covers every src/mc/ref_mc_*.rs file with
  the documented D-P16-2/D-P16-3 exceptions encoded (tasks 3-4 cannot import a
  forbidden module without a red test).

## Tests add/change/remove
- Added: none new (the 16 engine tests moved files; the duplicated module/cfg
  helpers are test-local, not #[test]).
- Changed: tests/ref_mc_diff_test.rs call site for minimal_violation_labels.
- Removed: none.

## Tests run status
- cargo test 557/0 (UNCHANGED from the Phase 15 baseline)
- cargo test --workspace 685/0 (UNCHANGED)
- cargo test --features smt 559/0 (UNCHANGED)
- cargo test-generated-code 1683/1683 (UNCHANGED)
- cargo fmt clean; cargo clippy --all-targets 55 warning lines / 0 errors (baseline)
- rfs-dev inventory audit EXACT (SRC 965 fns / 200 types, TEST 980 fns / 34 types);
  rfs-dev sync-check clean; dev_docs_test 9/0

## Found problems/issues/mismatches
- The inventory row-move initially landed all 21 rows in the All-types table
  (the last ref_mc.rs row in the file is a type row) - caught by inventory audit
  ([fn] missing / [type] stale), fixed by re-filing the 17 fn rows into the
  All-functions table and moving the nested run::St row. No source impact.
- dev_docs_test's TASK_MAP coverage check requires the new file to be referenced
  from TASK_MAP.md - added with the exact/liveness/verify row updates.

## Other important findings/information
- The move is textually verifiable: git diff shows only relocation + visibility
  keywords + the delegator, which is what makes "zero behavior change" provable by
  the unchanged counts rather than by re-reading the code.
