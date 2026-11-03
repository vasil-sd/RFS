# Phase 10, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Create `SRC_STRUCTURE.md` at the repo root as an empty six-section skeleton with a
binding "Conventions" preamble, so that all twelve enumeration chunk tasks (2-13)
fill rows in one identical format. This is the foundation of the whole phase.

## Context
- Section names and table column headers are fixed by `PHASE_10.md` (Scope +
  Inventory conventions) - copy them verbatim, do not invent variants.
- The six sections, in order: `## All functions`, `## All types`,
  `## Components`, `## Functionality`, `## File mapping`, `## Proposed layout`.
  (The file-mapping and proposed-layout sections are filled by tasks 17-18;
  Components/Functionality by task 14.)
- Table headers exactly:
  - All functions: `| Function | Depends | Component | Functionality | Short description | Documented in |`
  - All types: `| Type | Depends | Component | Functionality | Short description | Documented in |`
    (identifier column named `Function` in the functions table, `Type` in the
    types table; type rows use `<file>::<Type>` identifiers per the conventions).
  - Components / Functionality: `| Name | Depends on | Description and rationale |`
  - File mapping: `| Source file name | Components | Functionalities | Description + rationale |`
- The Conventions preamble must pin, in short bullets: both identifier formats (`<file>::<function>`, `<file>::<Type>`, repo-root-relative paths); Depends (both tables, both directions) = comma-separated list of project identifiers - functions AND types - the row depends on (function rows: called fns + constructed/contained/matched types; type rows: contained types + implementation fns), empty when there are none; multi-valued Component/Functionality cells are
  comma-separated; Documented-in = the `dev:`/`llm:` fragment names copied
  verbatim from the file's single `//! Docs:` line, blank for files without one;
  private items are included; every row stays on ONE physical line.
- From the moment the file exists, `tests/docs_md_test.rs` parses it (comrak
  GFM + mermaid pipeline + relative-link resolution) - keep it gate-clean.
- Component and Functionality cells stay BLANK in all rows until tasks 15-16.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2.-13. | All enumeration chunks | need the skeleton + frozen conventions to fill rows into |
| 14. | Categories proposal | fills the two empty category tables this task creates |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | new | six-section skeleton + Conventions preamble, no data rows yet |

## Planned actions

### Create the skeleton file
Write `SRC_STRUCTURE.md`: a one-line title (`# RFS Source Structure - function and type inventory`),
a short intro naming the phase and the later refactoring consumer, the
Conventions preamble (bullets listed in Context), then the six sections each
with its table header + separator row and no data rows. The `## Proposed
layout` section gets a placeholder note "filled by task 18".

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | new | the skeleton itself - single canonical inventory location at repo root, consistent with START.md/KB.md/BUILD.md placement |

### Gate check
Run `cargo test --test docs_md_test` and confirm the new file parses clean.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `SRC_STRUCTURE.md` at repo root: Conventions preamble + six empty sections
  with exact headers.

## Task exit criteria
- [ ] File exists at the repo root with all six section headings in the order
      fixed by PHASE_10.md.
- [ ] All three table shapes carry the exact column headers from Context.
- [ ] Conventions preamble pins: identifier format, Depends semantics,
      multi-value separator, Documented-in source, private-items inclusion,
      one-line rows.
- [ ] No data rows anywhere (Component/Functionality tables empty by design).
- [ ] `cargo test --test docs_md_test` green (5/5).

## Non-goals
- No function/type rows (tasks 2-13), no categories (task 14), no mapping or
  layout (tasks 17-18).
- No source-file changes of any kind.
