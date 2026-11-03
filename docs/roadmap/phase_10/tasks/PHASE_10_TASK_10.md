# Phase 10, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Enumerate every function and type of the introspection + self-describing-format
cluster - `src/info.rs`, `src/info_format.rs`, `src/ebnf_grammar.rs`,
`src/json_schema.rs` (~85 fns / ~7 types) - into `SRC_STRUCTURE.md`.

## Context
- Conventions: SRC_STRUCTURE.md preamble + PHASE_10.md. Component/Functionality
  cells stay BLANK.
- Cluster notes:
  - `src/info.rs` (498 lines, 20 fns / 1 type): the `rfs info` command - static
    per-module introspection (states, events/outputs with variants + fields,
    vars, transition count, properties/assumptions with monitor-subset status);
    text rendering + the `--json` document (stable key order, all keys present).
  - `src/info_format.rs` (1,623 lines, 24 fns / 1 type): the Phase 7
    `rfs info format` registry - static `FORMATS: &[FormatInfo]` (10 entries:
    spec-rfs, spec-json, ce-trace, mc-report, validate-json, info-json,
    diagram-dot, diagram-puml, diagram-mmd, artifact-header) + per-entry
    renderers; the embedded machine-readable models live here and are pinned to
    the code by the D-P7-9 sync tests.
  - `src/ebnf_grammar.rs` (902 lines, 8 fns / 1 type): const-constructible
    `RuleNode` model + 61-rule `GRAMMAR_RULES` mirroring the ACTUAL parser;
    renderers `to_md()` / `to_plantuml_ebnf()` / `to_svg()`; `to_ebnf_text()`
    is the canonical text of DESIGN.md section 6's ebnf fence.
  - `src/json_schema.rs` (767 lines, 33 fns / 4 types): shared draft-07
    JSON-Schema emitter + md table renderer over a compact field model
    (`Str/Int/Bool/Null/Obj/Arr` + `EnumStr`, `Ref`, `TaggedUnion`, `Nullable`,
    `FreeObj`, `Union`); the D-P7-9 walk check shared by five format entries.
- Re-verify counts first (~85/~7); mismatch -> plan correction.
- Documented-in from each file's `//! Docs:` line (all four are manifest files).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Skeleton + conventions | tables + row format |
| 2., 3. | Chunks A-B | info rows depend on parser/IR identifiers already in the tables |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15., 16. | Classification fills | classify these rows |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | ~85 function rows + ~7 type rows appended |

## Planned actions

### Re-verify chunk counts
Fn/type greps over the four files vs ~85/~7; mismatch -> plan correction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### Enumerate functions
Grep signatures, read bodies, one row per fn in `## All functions` with bidirectional project function/type Depends and one-line descriptions.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All functions`) | ~85 rows for the introspection/format cluster |

### Enumerate types
One row per type in `## All types` (~7): FormatInfo, RuleNode, schema field-model
types.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | change (`## All types`) | ~7 type rows |

### Gate check
docs_md_test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Complete function + type rows for `src/info.rs`, `src/info_format.rs`,
  `src/ebnf_grammar.rs`, `src/json_schema.rs`.

## Task exit criteria
- [ ] Row counts equal re-verified grep counts per file.
- [ ] Every row: Depends, one-line description, Documented-in verbatim.
- [ ] Component/Functionality blank; rows one-per-line.
- [ ] docs_md_test green.

## Non-goals
- No classification, no other clusters, no source changes.
