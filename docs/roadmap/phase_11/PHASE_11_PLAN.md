# Phase 11 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation immediately without re-deriving scope.

## Context

- The single source of truth for WHAT moves where is the "Target directory tree"
  block in `SRC_STRUCTURE.md` (section "Proposed naming scheme") - read it fully
  before task 1. The old->new path mapping below is the planning-time extraction
  of that tree; if they ever disagree, the tree wins and this table is a plan
  correction.
- Complete old->new path mapping (42 relocated files + 3 test renames):

| Old path | New path | Note |
|---|---|---|
| src/spec_hash.rs | src/utils/hash.rs | renamed spec_hash -> hash |
| src/json_rw.rs | src/json/json_rw.rs | |
| src/json_schema.rs | src/json/json_schema.rs | pulled out of info_format grouping |
| src/cli_log.rs | src/cli/log.rs | renamed cli_log -> log |
| src/frontend.rs | src/cli/frontend.rs | |
| src/ext_tool.rs | src/cli/ext_tool.rs | |
| src/test_cmd.rs | src/cli/commands/test.rs | _cmd suffix dropped |
| src/gen_spec_cmd.rs | src/cli/commands/gen_spec.rs | _cmd suffix dropped |
| src/mc_ref_cmd.rs | src/cli/commands/mc_ref.rs | _cmd suffix dropped |
| src/info.rs | src/cli/commands/info.rs | |
| src/diagram.rs | src/cli/commands/diagram/mod.rs | absorbed as mod.rs (module path cli::commands::diagram) |
| src/diagrams/mod.rs | (merged into src/cli/commands/diagram/mod.rs) | label helpers event_label/guard_label/emit_labels/states/render_check move as items |
| src/diagrams/dot.rs | src/cli/commands/diagram/dot.rs | |
| src/diagrams/plantuml.rs | src/cli/commands/diagram/plantuml.rs | |
| src/diagrams/mermaid.rs | src/cli/commands/diagram/mermaid.rs | |
| src/diagrams/tables.rs | src/cli/commands/diagram/tables.rs | |
| src/info_format.rs | src/cli/info_format/mod.rs | absorbed as mod.rs (module path cli::info_format) |
| src/ebnf_grammar.rs | src/cli/info_format/ebnf_grammar.rs | |
| src/parser.rs | src/lang/parser.rs | |
| src/error.rs | src/lang/error.rs | |
| src/ir.rs | src/lang/ir.rs | 576 incoming edges - biggest rename |
| src/printer.rs | src/lang/printer.rs | |
| src/validator.rs | src/lang/validator.rs | |
| src/specgen.rs | src/lang/specgen.rs | lockstep: 5 external import sites incl. 2 in the detached fuzz workspace |
| src/codegen.rs | src/codegen/mod.rs | ZERO-CHURN: module path `codegen` unchanged |
| src/codegen_fsm.rs | src/codegen/fsm.rs | |
| src/codegen_test.rs | src/codegen/test_plan.rs | not test.rs - that name belongs to the rfs-test command |
| src/codegen_mc.rs | src/codegen/mc_concepts.rs | |
| src/monitor.rs | src/codegen/monitor.rs | no rename |
| src/backends/mod.rs | src/codegen/backends/mod.rs | re-exports updated to new sub-paths |
| src/backends/rust.rs | src/codegen/backends/rust/mod.rs | absorbed as mod.rs; snake/rust_ident leave for codegen/ident.rs |
| src/backends/rust_fsm.rs | src/codegen/backends/rust/fsm.rs | |
| src/backends/rust_test.rs | src/codegen/backends/rust/test.rs | |
| src/backends/rust_mc.rs | src/codegen/backends/rust/mc.rs | |
| src/backends/c.rs | src/codegen/backends/c/mod.rs | absorbed as mod.rs |
| src/backends/c_fsm.rs | src/codegen/backends/c/fsm.rs | |
| src/backends/c_test.rs | src/codegen/backends/c/test.rs | |
| src/backends/c_mc.rs | src/codegen/backends/c/mc.rs | |
| src/mc_plan.rs | src/mc/plan.rs | renamed mc_plan -> plan |
| src/smt_plan.rs | src/mc/smt_plan.rs | |
| src/smt_emit.rs | src/mc/smt_emit.rs | inseparable pair with smt_plan |
| src/ref_mc.rs | src/mc/ref_mc.rs | independence contract: no intra-mc/ imports |
| tests/library_comparison.rs | tests/parser_comparison_test.rs | test-side rename 1 |
| tests/examples_test.rs | tests/example_corpus_test.rs | test-side rename 2 |
| tests/integration_example_test.rs | tests/endpoint_protocol_test.rs | test-side rename 3 |

- NEW files: src/utils/mod.rs, src/json/mod.rs, src/cli/mod.rs,
  src/cli/commands/mod.rs, src/lang/mod.rs, src/mc/mod.rs (six empty-ish module
  roots), src/utils/rng.rs (LCG consolidation), src/codegen/ident.rs (the
  flagged cross-backend coupling resolution - snake/rust_ident/rust_binding_ident
  move here from backends/rust.rs; the actualized tree in task 11 shows it).
- Kept exactly in place: src/main.rs, src/lib.rs, src/training.rs, all of
  src/bin/, all of crates/, fuzz/ layout (only its two specgen import lines are
  edited), Makefile-driven example projects.
- LCG consolidation (task 1, the only non-mechanical source edit): the new
  utils/rng.rs holds ONE generator with the union of specgen Rng's and codegen
  test_plan Lcg's methods (new/next_u64/next_f64/range/pick/chance/pick_index)
  plus the two constants 6364136223846793005 / 1442695040888963407 as shared
  `pub const`s. specgen's Rng and codegen_test's Lcg structs are DELETED; their
  call sites use utils::rng. The two GENERATED-runtime copies (rust_mc.rs
  FUZZ_RUNTIME string, c_mc.rs C seed function) cannot `use` the crate - they
  interpolate the same pub const values, keeping byte-identical output and one
  RNG family by construction.
- Manifest re-sync scope: ALL 42 relocated src files are listed in
  tests/dev_docs_manifest.txt (verified at planning: 42/42). NOTE: the
  PHASE_10_STATUS.md figure "28 of the 71 manifest paths" is stale - it predates
  the final tree; dev_docs_test's output after each move is the authoritative
  update list. The three test renames are NOT in the manifest (test files carry
  no //! Docs: pointers except via their suite entries - verify, do not assume).
- Import-site counts measured at proposal time (re-verify with grep at task
  start): ir 2 external / 576 internal, validator 8, codegen_mc 5, ref_mc 5,
  parser 2, info 2, diagram 2, diagrams 3, gen_spec_cmd 1, mc_ref_cmd 1,
  ebnf_grammar 1, json_schema 1, specgen 5 (3 tests + 2 fuzz-workspace),
  backends::* 3 (2 bins + 1 test), info_format 1, spec_hash 2, json_rw 1,
  frontend 1, ext_tool 1, cli_log 0 external. Internal churn is larger but
  purely mechanical `use`-path edits - script them, do not hand-edit file by
  file (token budget).
- Verification gate per refactoring task: `cargo build && cargo test` green
  before committing (the full 535-test suite is the gate - determinism-pinning
  suites make codegen byte-identity a tested invariant, so a green run proves
  "no functionality change" for that step). clippy + fmt run at close-out only.
- Token-budget design: every task is grep-driven + scripted edits (python for
  bulk `use` rewrites and inventory path rewrites), no full-file reads except
  the one merge (diagrams/mod.rs items into diagram/mod.rs) and the LCG call-site
  edits. Each task fits a ~200k-token session with margin.
- Actualization identifier convention: a file that becomes a directory's mod.rs
  is identified by its real path (src/codegen/mod.rs::<item>); the merged
  diagram label helpers become src/cli/commands/diagram/mod.rs::<item>. The
  LCG consolidation deletes the specgen Rng + codegen_test Lcg rows (type +
  method rows) and adds utils/rng.rs rows; every Depends entry naming them is
  rewritten to the utils/rng.rs identifiers.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | utils/ substrate + LCG consolidation | src/spec_hash.rs (move->src/utils/hash.rs); src/utils/mod.rs (new); src/utils/rng.rs (new); src/specgen.rs (change - Rng deleted, uses utils::rng); src/codegen_test.rs (change - Lcg deleted, uses utils::rng); src/backends/rust_mc.rs (change - FUZZ_RUNTIME interpolates the pub consts); src/backends/c_mc.rs (change - C seed fn interpolates the pub consts); import sites of spec_hash (change) | create the cross-cutting substrate directory; execute the proposal's only non-mechanical step first so every later move is pure path renaming. Byte-identity proven by the determinism-pinning suites in the per-task cargo test gate |
| 2. | json/ substrate | src/json_rw.rs (move->src/json/json_rw.rs); src/json_schema.rs (move->src/json/json_schema.rs); src/json/mod.rs (new); import sites of json_rw + json_schema (change, scripted) | the JSON substrate moves before its consumers so later moves never touch these import lines again (134 incoming edges to json_rw - done once, here) |
| 3. | lang/ core | src/{parser,error,ir,printer,validator,specgen}.rs (move->src/lang/); src/lang/mod.rs (new); import sites of all six (change, scripted - ir's 576 edges); fuzz/fuzz_targets/{common,ir_roundtrip}.rs (change - specgen lockstep); tests/common/prod_driver.rs, tests/ref_mc_diff_test.rs, tests/toolchain_fuzz_test.rs (change - specgen lockstep) | the language core pipeline groups into one directory; the detached fuzz workspace is edited in the SAME commit as the specgen move (separate cargo project - nothing else catches this break) |
| 4. | codegen/ shared layer | src/codegen.rs (move->src/codegen/mod.rs, zero-churn); src/codegen_fsm.rs (move->src/codegen/fsm.rs); src/codegen_test.rs (move->src/codegen/test_plan.rs); src/codegen_mc.rs (move->src/codegen/mc_concepts.rs); src/monitor.rs (move->src/codegen/monitor.rs); import sites (change, scripted) | the shared pre-backend layer; codegen's absorption keeps module path `codegen` so rfs::codegen::* importers (incl. fuzz workspace) survive untouched |
| 5. | codegen/backends/{rust,c}/ + ident extraction | src/backends/mod.rs (move->src/codegen/backends/mod.rs, re-exports updated); src/backends/rust.rs (move->src/codegen/backends/rust/mod.rs); src/backends/c.rs (move->src/codegen/backends/c/mod.rs); the six emitters (move per mapping); src/codegen/ident.rs (new - snake/rust_ident/rust_binding_ident extracted from rust/mod.rs); c/* emitters (change - import idents from codegen::ident); import sites of backends::* (change, scripted) | the parity axis becomes two sibling directories; the flagged cross-backend coupling is resolved into the neutral module the proposal named |
| 6. | cli/ root + commands | src/cli_log.rs (move->src/cli/log.rs); src/frontend.rs (move->src/cli/frontend.rs); src/ext_tool.rs (move->src/cli/ext_tool.rs); src/test_cmd.rs (move->src/cli/commands/test.rs); src/gen_spec_cmd.rs (move->src/cli/commands/gen_spec.rs); src/mc_ref_cmd.rs (move->src/cli/commands/mc_ref.rs); src/info.rs (move->src/cli/commands/info.rs); src/cli/mod.rs + src/cli/commands/mod.rs (new); src/main.rs (change - dispatch paths) | the CLI surface in one directory; _cmd suffixes dropped per rule 2; main.rs is the only kept-root file edited |
| 7. | diagram cluster + info_format cluster | src/diagram.rs (move->src/cli/commands/diagram/mod.rs); src/diagrams/mod.rs (remove - items merged into diagram/mod.rs); src/diagrams/{dot,plantuml,mermaid,tables}.rs (move->src/cli/commands/diagram/); src/info_format.rs (move->src/cli/info_format/mod.rs); src/ebnf_grammar.rs (move->src/cli/info_format/ebnf_grammar.rs); import sites (change) | the one genuine MERGE of the phase (five label-helper items relocate + their use-paths fix); info_format's registry absorbs its EBNF leaf, the schema emitter already left for json/ in task 2 |
| 8. | mc/ | src/mc_plan.rs (move->src/mc/plan.rs); src/smt_plan.rs (move->src/mc/smt_plan.rs); src/smt_emit.rs (move->src/mc/smt_emit.rs); src/ref_mc.rs (move->src/mc/ref_mc.rs); src/mc/mod.rs (new); import sites (change, scripted) | model-checking planning + the independent reference engine; ref_mc's independence contract stays visible (no intra-mc/ imports) |
| 9. | Test-suite renames | tests/library_comparison.rs (move->tests/parser_comparison_test.rs); tests/examples_test.rs (move->tests/example_corpus_test.rs); tests/integration_example_test.rs (move->tests/endpoint_protocol_test.rs) | the TEST_STRUCTURE.md proposal executed; cargo discovers one binary per file so no manifest/Cargo edits are needed - green cargo test proves discovery |
| 10. | Docs<->source reference re-sync | tests/dev_docs_manifest.txt (change - 42 paths); every docs/**/*.md carrying a backticked source path (change, scripted - 109 distinct references at planning time) | dev_docs_test's output after the moves IS the manifest update list; the same old->new mapping rewrites all backticked src/tests .rs references in docs/ so no stale pointer survives (exit criterion: fresh grep for every old path returns nothing) |
| 11. | SRC_STRUCTURE.md actualization | SRC_STRUCTURE.md (change - all row identifiers + Depends entries rewritten to new paths via the mapping; "Proposed naming scheme" -> "Naming scheme" with the tree updated for ident.rs; "Proposed layout" -> "Layout"; file-mapping table rows show actual locations; LCG rows swapped: specgen Rng + codegen_test Lcg rows deleted, utils/rng.rs rows added, Depends fixed) | the inventory must describe the ACTUAL tree after the refactor - it is the impact-analysis substrate the protocol will mandate, so a fresh string-masked grep audit (the Phase 10 close-out method) must come back exact: zero per-file diffs |
| 12. | TEST_STRUCTURE.md actualization | TEST_STRUCTURE.md (change - same treatment: identifiers + Depends rewritten for the three renames; "Proposed layout" -> "Layout"; file-mapping rows show actual names) | same rationale as task 11 for the test side; audit exact against the refactored tests/ tree |
| 13. | Inventories to docs/llm/ + protocol rules | SRC_STRUCTURE.md (move->docs/llm/SRC_STRUCTURE.md); TEST_STRUCTURE.md (move->docs/llm/TEST_STRUCTURE.md); PROTOCOL.md (change - new sections: impact calculation rule linking both inventories' Depends columns as the mandatory first step of any source-change scoping; docs<->source correspondence rule: every source change updates the affected inventory rows in the same commit); PRINCIPLES.md (change - doc-set rule pointing at the two protocol sections) | the user's close-out requirements 4-5: the inventories become living docs/llm/ references, and the protocol codifies both their use (impact calculation) and their maintenance (correspondence) |
| 14. | Full verification + close-out | PHASE_11_STATUS.md (new); CURRENT_STATE.md (change - row 11 COMPLETE); TASKS.md (change - entries) | cargo test 535/535 + clippy --all-targets + fmt --check on the final tree; git diff --stat over the phase confirms only in-scope files changed; status file per FORMATTING.md section 4 |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|

## Other information

- Execution order is dependency-safe by construction: substrates (utils, json)
  move before their consumers (lang, codegen, cli, mc), so a file's import
  lines are rewritten at most once - when the module it imports moves. The one
  exception is deliberate: specgen's five external import sites (task 3) include
  two in the detached fuzz workspace and must land in the same commit as the
  move or that workspace is left broken between commits.
- Every refactoring task commits only on a green `cargo build && cargo test`;
  a red gate means fix-in-place (import miss), never a functionality workaround.
- The phase is docs+source mechanical by design: if any task discovers a file
  whose contents must change beyond use-paths / the LCG swap / the ident
  extraction / the diagram merge, that is a plan correction and a stop-for-user
  moment - it would imply the proposal misread the source.
