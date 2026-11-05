# Phase 11, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the JSON substrate (`json_rw.rs`, `json_schema.rs`) into `src/json/` and
rewrite its import sites once, before any consumer moves.

## Context
- json_rw has 134 incoming edges - the largest internal rename after ir's 576;
  doing it here (before lang/codegen/cli/mc move) means no later task ever edits
  a json use-line again.
- json_schema is pulled out of the info_format grouping: it emits a draft-07
  schema that feeds json_rw, so it belongs with the I/O it drives (proposal rule 8).
- Scripted rewrite patterns: `crate::json_rw` -> `crate::json::json_rw`,
  `crate::json_schema` -> `crate::json::json_schema`, external `rfs::json_rw` /
  `rfs::json_schema` likewise (1 external test file each - re-verify by grep).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | utils/ substrate | establishes the move-first-then-consumers ordering discipline |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3.-8. | all later move tasks | their files' json import lines are already final |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/json_rw.rs` | remove (moved) | git mv to src/json/json_rw.rs |
| `src/json_schema.rs` | remove (moved) | git mv to src/json/json_schema.rs |
| `src/json/mod.rs` | new | module root declaring json_rw + json_schema |
| all import sites of json_rw / json_schema | change | scripted use-path rewrite |

## Planned actions

### Move both files + create the module root
git mv both files; write src/json/mod.rs; script every use-path rewrite across
src/, src/bin/, tests/ (no fuzz-workspace site imports these - verify by grep).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/json_rw.rs` | remove | git mv to src/json/json_rw.rs (contents untouched) |
| `src/json_schema.rs` | remove | git mv to src/json/json_schema.rs (contents untouched) |
| `src/json/mod.rs` | new | `pub mod json_rw; pub mod json_schema;` |
| import sites of json_rw / json_schema | change | scripted rewrite, one pass |

### Gate
cargo build && cargo test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/json/` with both files; every json import site at its final path.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] No file outside src/json/ references the old paths (grep for `json_rw` / `json_schema` use-sites returns only crate::json:: forms).
- [ ] File contents byte-identical to before (git diff shows moves + use-line edits only).

## Non-goals
- No change to JSON behavior or output format; no other file moved.
