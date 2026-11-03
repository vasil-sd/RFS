# Phase 10, Task 11 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the diagram cluster - `src/diagram.rs` +
`src/diagrams/{mod,dot,plantuml,mermaid,tables}.rs` (~41 fns / ~12 types) - into
`SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/diagram.rs` (413 lines, 6 fns / 10 types): plugin registry mirroring
    the codegen backend pattern; `--render svg` dispatch between internal and
    external renderers.
  - `src/diagrams/mod.rs` (66 lines, 5 fns / 0 types): the diagram plugin trait.
  - `src/diagrams/dot.rs` (130 lines, 7 fns / 0 types): Graphviz DOT state-
    diagram generation + in-process SVG via layout-rs
    (`render_svg_internal`: DotParser -> GraphBuilder -> VisualGraph::do_it ->
    SVGWriter).
  - `src/diagrams/plantuml.rs` (132 lines, 6 fns / 0 types): PlantUML state-
    diagram generation + in-process SVG via plantuml-core `render`.
  - `src/diagrams/mermaid.rs` (202 lines, 9 fns / 0 types): Mermaid
    `stateDiagram-v2` generation + in-process SVG via mermaid-rs-renderer
    `render_strict`.
  - `src/diagrams/tables.rs` (383 lines, 8 fns / 2 types): the `table` + `md`
    plugins (state x event transition matrix + variable/event summaries); no
    SVG renderer in either mode.
- Re-verify counts first (~41/~12); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all six are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2., 3. | Chunks A-B | diagram rows depend on parser/IR identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~41 function rows + ~12 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the six files vs ~41/~12; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~41 rows for the diagram cluster |

### Enumerate types
One row per type in `## All types` (~12): registry/trait types, table-plugin
internals.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~12 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/diagram.rs` and all five
  `src/diagrams/*.rs` files.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
