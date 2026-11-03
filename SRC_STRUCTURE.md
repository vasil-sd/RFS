# RFS Source Structure - function and type inventory

Phase 10 analysis document (non-test source). Complete, dependency-annotated
inventory of every function and every data type in the project's non-test
source files, classified into proposed component and functionality categories,
with a proposed new file-naming scheme and directory tree layout. Consumed by
the later refactoring phase that will actually reorganize the source tree -
this document is analysis and proposal only; no source is moved or renamed
here. The dependency edges additionally support fine-grained impact analysis
of future source changes and precise scoping of refactoring/editing tasks.
Test source has its own inventory: `TEST_STRUCTURE.md`.

## Conventions (binding for all rows)

- Identifier formats: functions `<file>::<function>`, types `<file>::<Type>`,
  file paths relative to the repo root (e.g. `src/parser.rs::parse_source`,
  `src/ir.rs::Ir`). The All functions table's identifier column is named
  `Function`; the All types table's is named `Type`.
- Depends column (BOTH tables, BOTH directions): comma-separated list of
  project identifiers - `<file>::<function>` AND `<file>::<Type>` - that the
  row depends on. A function row lists the functions it calls and the types it
  constructs/contains/matches; a type row lists the types it contains and the
  implementation functions its trait impls/methods delegate to. Cross-file and
  same-file alike; empty if none. Macro-generated call sites count once per
  distinct callee. External items (nom, std, dependency crates) are NOT
  dependencies - only project identifiers.
- Component and Functionality cells may be multi-valued; values are
  comma-separated and must come from the `## Components` / `## Functionality`
  category tables (filled by task 14; blank in rows until tasks 15-16).
- Documented in column: the `dev:` / `llm:` fragment names copied verbatim
  from the file's single `//! Docs:` pointer line (the 71-file manifest);
  blank for files without one.
- Private (`fn`, no `pub`) items are included - the inventory is complete by
  construction, not public-API-only.
- Every row stays on ONE physical line (grep-able, diff-friendly; future
  tooling and the refactoring phase rely on stable row identity).

## All functions

| Function | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|

## All types

| Type | Depends | Component | Functionality | Short description | Documented in |
|---|---|---|---|---|---|

## Components

| Name | Depends on | Description and rationale |
|---|---|---|

## Functionality

| Name | Depends on | Description and rationale |
|---|---|---|

## File mapping

| Source file name | Components | Functionalities | Description + rationale |
|---|---|---|---|

## Proposed layout

(filled by task 18 - the proposed directory tree as a mermaid diagram)
