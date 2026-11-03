# Phase 10, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the IR-model + serialization cluster -
`src/ir.rs`, `src/printer.rs`, `src/json_rw.rs` (~115 fns / ~27 types) - into
`SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/ir.rs` (1,432 lines, 63 fns / 23 types): the IR v0 data model (module,
    state, event/output variants + fields, variables, transitions, properties/
    assumptions, version header) with HAND-WRITTEN `to_json`/`from_json` per
    type (serde removed in Phase 5 task 2 - D-P5-2); `to_json` is fallible for
    u64-range values; key order is pinned (byte-identical output contract).
    The 63 fns are dominated by these per-type pairs - describe each pair once
    accurately, do not pad.
  - `src/printer.rs` (828 lines, 20 non-test fns + 14 inline tests in task 29 / 1 type): IR -> formatted RFS source
    rendering (the rfs->rfs reformat path).
  - `src/json_rw.rs` (962 lines, 32 non-test fns + 26 inline tests in task 29 / 3 types): the hand-rolled JSON
    reader-writer ALL toolchain JSON goes through - ordered-object value model,
    compact + pretty writers with pinned escaping/number formatting, reader with
    typed extraction; an embedded copy of the writer ships inside generated MC
    binaries (note that in the description, it is not a separate row).
- Re-verify counts first (~115/~27); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all three are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2. | Chunk A (parser core) | `ParseError`/`Span` rows already exist, so ir.rs/parser-facing Depends entries can reference them by identifier |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | classify these rows |
| 18. | File mapping table | maps these files |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~155 function rows + ~29 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the three files vs ~155/~29; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional Depends on project functions AND types (ir.rs constructs json_rw value types and calls its writer fns - record both directions; the cross-file edges are the point of this chunk), one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~155 rows for the IR/serialization cluster |

### Enumerate types
One row per type in `## All types` (~29): IR model types, printer internals,
json_rw value model. Depends = contained/constructed project items.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~29 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/ir.rs`, `src/printer.rs`,
  `src/json_rw.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends (project fns only), one-line description, Documented-in
      verbatim from `//! Docs:`.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
