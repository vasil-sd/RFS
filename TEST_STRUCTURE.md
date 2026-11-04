# RFS Test Structure - function and type inventory

Phase 10 analysis document (test source). Complete, dependency-annotated
inventory of every function and every data type in the project's TEST source,
classified into proposed component and functionality categories, with a
proposed new file-naming scheme and directory tree layout. Consumed by the
later refactoring phase that will actually reorganize the test tree - this
document is analysis and proposal only; no test source is moved or renamed
here. The dependency edges additionally support fine-grained impact analysis of
future test changes and precise scoping of refactoring/editing tasks. Non-test
source has its own inventory: `SRC_STRUCTURE.md`.

## Conventions (binding for all rows)

- Identifier formats: functions `<file>::<function>`, types `<file>::<Type>`,
  file paths relative to the repo root (e.g. `tests/frontend_test.rs::it_parses`,
  `fuzz/fuzz_targets/common.rs::Spec`). The All functions table's identifier
  column is named `Function`; the All types table's is named `Type`. A function
  nested inside another function body is `<file>::<parent_fn>::<child_fn>`
  (lowercase parent - distinct from the `<file>::<Type>::<method>` method form,
  whose middle segment is a capitalized type name); a type declared inside a
  function body follows the same form, `<file>::<parent_fn>::<Type>`.
- Inline `#[cfg(test)]` modules: functions and types declared inside an inline
  `#[cfg(test)] mod tests { ... }` in a `src/` file are enumerated HERE (not in
  SRC_STRUCTURE.md) with identifiers of the form `src/<file>.rs::tests::<fn>` -
  the host file is the identifier's file part and `tests` is the module segment.
  There are 25 such host files under `src/`.
- Depends column (BOTH tables, BOTH directions): comma-separated list of
  project identifiers - `<file>::<function>` AND `<file>::<Type>` - that the row
  depends on. A function row lists the functions it calls and the types it
  constructs/contains/matches; a type row lists the types it contains and the
  implementation functions its trait impls/methods delegate to. Cross-file and
  same-file alike; empty if none. Macro-generated call sites count once per
  distinct callee. External items (nom, std, dependency crates) are NOT
  dependencies - only project identifiers. Test rows frequently depend on
  NON-test source identifiers (those live in SRC_STRUCTURE.md's inventory); the
  edge still records the full `<file>::<item>` path.
- Component and Functionality cells may be multi-valued; values are
  comma-separated and must come from the `## Components` / `## Functionality`
  category tables (filled by task 31). They stay BLANK in every row until task
  32.
- Documented in column: copied verbatim from the file's single `//! Docs:`
  pointer line when it has one; blank otherwise. Most test files have no such
  line. A handful DO carry real `//! Docs:` pointers to fragments defined in
  `docs/` - the five fuzz targets (`fuzz/fuzz_targets/{common,parser,codegen,
  validator,ir_roundtrip}.rs`) and the three workspace-crate tests
  (`crates/rfs-adapters/tests/{adapters,nostd_minimal}.rs`,
  `crates/rfs-build/tests/integration.rs`) - and those fragments are copied like
  any other file's (these files are also in `tests/dev_docs_manifest.txt`). The
  three fixtures' `//! Docs:` lines are a separate case (see below). Where
  `docs/llm/TASK_MAP.md` has a suite entry for a test file, that is noted in the
  row's description instead of Documented-in.
- Fixtures: the three `tests/fixtures/dev_docs/*.rs` files
  (`good_code.rs`, `dangling_pointer.rs`, `missing_pointer.rs`) are
  negative-control FIXTURES for `dev_docs_test` - enumerated like any other file,
  but flagged as fixtures in their descriptions. Their "functions"/"types" are
  deliberate test subjects, not project code, and their `//! Docs:` lines point
  at fixture-only fragments (`dev:fixture-*` / `llm:fixture-*`) that are NOT
  defined in the doc set - they are the gate's own positive/negative controls,
  not real documentation pointers.
- Generated build output is NEVER enumerated: `crates/examples/fuzz_rust/fuzz/
  target/**` (and any other build-artifact tree) holds compiler/emitter output,
  not hand-written test source.
- Scope boundary (binding, from PHASE_10.md): this inventory covers `tests/**`,
  the inline `#[cfg(test)]` modules in `src/`, `crates/*/tests/**`, and
  `fuzz/fuzz_targets/**`. The fuzz targets' SOLE home is this document - they do
  not appear in SRC_STRUCTURE.md.
- Private (`fn`, no `pub`) items are included - the inventory is complete by
  construction, not public-API-only (most test functions are private).
- Every row stays on ONE physical line (grep-able, diff-friendly; future
  tooling and the refactoring phase rely on stable row identity).

## All functions

| Function | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|

## All types

| Type | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|

## Components

(filled by task 31 - the proposed test-side component categories)

| Name | Depends on | Description and rationale |
|---|---|---|

## Functionality

(filled by task 31 - the proposed test-side functionality axes)

| Name | Depends on | Description and rationale |
|---|---|---|

## File mapping

(filled by task 33 - one row per non-generated test source file: components,
functionalities, description + rationale incl. its proposed new path)

| Source file name | Components | Functionalities | Description + rationale |
|---|---|---|---|

## Proposed layout

(filled by task 33 - the proposed test directory tree as a mermaid diagram)
