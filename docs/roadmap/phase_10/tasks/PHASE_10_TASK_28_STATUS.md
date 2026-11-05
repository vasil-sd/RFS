# Phase 10, Task 28 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_28.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | 85 function rows + 2 type rows appended - the `tests/` tree is now COMPLETE: dev_docs_test.rs 37 fns / 2 types (FragDef, PointerParse; its three nested walk fns get `<file>::<parent_fn>::<child_fn>` identifiers), diagram_render_parity_test.rs 27 fns, example_projects_test.rs 9 fns, integration_example_test.rs 4 fns, training_test.rs 5 fns, and the three tests/fixtures/dev_docs/*.rs fixtures (1 fn each, flagged as fixtures in their descriptions). Every row carries bidirectional project Depends (mapped to real identifiers across src/{diagram,diagrams/*,parser,codegen,validator,ir,json_rw,printer,training,lib}.rs), a one-line description, and blank Component/Functionality cells. Documented-in is blank for every row - see Found problems (the Context's dev_docs_test //! Docs: claim was already settled by correction 19a). |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | Plan correction 28a: the Goal's "7 fns" for integration_example_test.rs was inflated by three template phantoms (fn input_from + fn main in the DRIVER_SRC raw string, a second fn main in the on_error inline driver string); authoritative chunk total 85 fns / 2 types (was 88/2). |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_28.md` | changed | Task corrections section: row counts 88/2 -> 85/2 + the Documented-in claim note. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The `tests/` tree is fully enumerated in TEST_STRUCTURE.md (tasks 20-28 complete):
  - `tests/dev_docs_test.rs` (37 fns / 2 types): the Phase 8 doc-sync gate - fragment-ID grammar, doc-to-code reference scanning (comrak AST walk), fence-aware line tracking, fragment-definition collection, //! Docs: pointer parsing, item-declaration detection (visibility stripping, keyword/impl matching), the five checks (doc-to-code resolution, code-to-doc pointers, TASK_MAP coverage, Class B budgets, no orphans), manifest validation + filesystem-scan completeness, and the positive/negative-control fixture tests.
  - `tests/diagram_render_parity_test.rs` (27 fns): cross-renderer syntax parity - internal half all 50 valid examples x dot/puml/mmd in-process; external half pinned 4-spec subset via real tools (availability-gated, serialized under TOOL_LOCK); unresolvable-tools e2e; --print-script internal-mode structural pins; three garbage-rejection negative controls.
  - `tests/example_projects_test.rs` (9 fns): c_sync_handler Makefile rebuild + C/Rust scenario parity with pinned values, fuzz_rust skeleton cargo check in the detached workspace, fuzz_c generation without clang.
  - `tests/integration_example_test.rs` (4 fns): the Phase 2 endpoint-protocol integration example - channel-driver determinism + pinned protocol behavior + version header, on_error fallback handling.
  - `tests/training_test.rs` (5 fns): training-set structure/snippets/aggregation/freshness.
  - The three fixtures: negative-control inputs for the sync gate, flagged as fixtures.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written (plain-text table cells, no backticked paths or mermaid) |

## Found problems/issues/mismatches

- PLAN CORRECTION 28a: the Goal's "7 fns" for integration_example_test.rs was a grep count inflated by THREE template phantoms - `fn input_from` + `fn main` inside the DRIVER_SRC raw string and a second `fn main` inside the on_error test's inline driver string (all at column 0 of the string content). Authoritative: 4 fns / 0 types; chunk total 85 fns / 2 types (was 88/2).
- The Context's claim that dev_docs_test.rs is "the ONLY test file with a //! Docs: line" repeats the error correction 19a already settled: the file has NO pointer line (its `Docs:` mentions are prose) and is not in tests/dev_docs_manifest.txt, so every row's Documented-in is blank per the pinned preamble rule. No new correction needed - 19a governs.
- The type-row anchor had to be derived from the file rather than quoted from memory: the last All-types row (psl_translation_test.rs::Ctx) carries a Depends order/description that differed from the task-27 draft, and "## Components" also occurs in the Conventions preamble - both caught by assertions before any write.

## Other important findings/information

- dev_docs_test.rs's three nested `fn walk` helpers (in scan_doc_refs, collect_md, manifest_matches_filesystem_scan) are REAL functions and get `<file>::<parent_fn>::<child_fn>` identifiers - a col-0-only grep would miss them (34 vs the authoritative 37).
- The fixture rows are flagged "FIXTURE (never compiled by cargo)" in their descriptions per the preamble convention; their single stub function is a deliberate test subject, not project code.
- All 85 fn rows + 2 type rows have blank Component/Functionality (task 32) and Documented-in (no manifest file in this chunk).
