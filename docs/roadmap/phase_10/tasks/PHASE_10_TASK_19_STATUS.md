# Phase 10, Task 19 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_19.md` are reached. Not
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
| `TEST_STRUCTURE.md` | new | the six-section test-inventory skeleton at the repo root, mirroring SRC_STRUCTURE.md exactly (same fixed section order and table headers; identifier columns named Function / Type). Conventions preamble adapted for test source: inline `#[cfg(test)]` modules enumerated as `src/<file>.rs::tests::<fn>` (25 host files under src/); the binding scope boundary (tests/**, inline #[cfg(test)] in src/, crates/*/tests/**, fuzz/fuzz_targets/** as the fuzz targets' SOLE home); generated build output (crates/examples/fuzz_rust/fuzz/target/**) never enumerated; the three tests/fixtures/dev_docs/*.rs flagged as negative-control fixtures; Component/Functionality cells blank until task 32; File mapping + Proposed layout carry task-33 placeholders. No data rows anywhere |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_19.md` | changed | CORRECTION 19a in place: the Context's claim that tests/dev_docs_test.rs is "the only one with a //! Docs: line" is wrong - see Found problems |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase (skeleton + conventions, no data rows).

## Removed functionality

None.

## New functionality

- The canonical test-inventory location now exists: TEST_STRUCTURE.md with a test-adapted conventions preamble that the enumeration tasks (20-30) and the category proposal (task 31) build on.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green; the new file has no mermaid blocks or links, so it parses cleanly under comrak GFM and adds no render/link checks |

## Found problems/issues/mismatches

- CORRECTION 19a: the task Context stated "Documented-in is blank for all test files EXCEPT tests/dev_docs_test.rs (the only one with a //! Docs: line)". That is wrong on both counts. tests/dev_docs_test.rs has NO `//! Docs:` pointer line (its `Docs:` mentions are prose describing the convention) and is not in tests/dev_docs_manifest.txt, so its Documented-in is blank. The test files that DO carry real `//! Docs:` pointers to fragments defined in docs/ are the five fuzz targets (fuzz/fuzz_targets/{common,parser,codegen,validator,ir_roundtrip}.rs) and the three workspace-crate tests (crates/rfs-adapters/tests/{adapters,nostd_minimal}.rs, crates/rfs-build/tests/integration.rs) - all eight are in the manifest. The three fixtures' `//! Docs:` lines point at fixture-only fragments (dev:fixture-* / llm:fixture-*, not defined in the doc set) as deliberate negative controls. The preamble pins this accurate rule.

## Other important findings/information

- 25 files under src/ contain inline `#[cfg(test)]` modules - these contribute `src/<file>.rs::tests::<fn>` rows to this inventory (their host files' non-test items stay in SRC_STRUCTURE.md).
- docs/llm/TASK_MAP.md exists (16.7 KB) and carries suite entries; per the convention, where it names a test suite that is noted in the row's description rather than Documented-in.
- The scope boundary makes fuzz/fuzz_targets/** the fuzz targets' SOLE home - they never appear in SRC_STRUCTURE.md (consistent with the task-14 boundary rule that dropped "toolchain fuzz targets" from the source inventory).
