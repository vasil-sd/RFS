# Phase 11, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/json/mod.rs` | module root of the JSON substrate (`pub mod json_rw; pub mod json_schema;`) + //! Docs: pointer to dev:code-map-json |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/json_rw.rs` | removed (moved) | git mv to `src/json/json_rw.rs`, contents byte-identical |
| `src/json_schema.rs` | removed (moved) | git mv to `src/json/json_schema.rs`; its 10 own `crate::json_rw::` references (sibling import + fully-qualified paths in inline tests) rewritten to `crate::json::json_rw::` - the only content change in either moved file |
| `src/lib.rs` | changed | `pub mod json_rw;` + `pub mod json_schema;` replaced by `pub mod json;` (same alphabetical position) |
| 21 source files with import sites | changed | 177 use-path rewrites scripted: `crate::json_rw::` -> `crate::json::json_rw::`, `crate::json_schema::` -> `crate::json::json_schema::`, `rfs::json_rw::` / `rfs::json_schema::` likewise - across src/, tests/ and the detached fuzz workspace (parser.rs, validator.rs, ir_roundtrip.rs targets) |
| `src/info_format.rs` | changed | grouped import `use crate::{ebnf_grammar, json_schema};` -> `use crate::{ebnf_grammar, json::json_schema};` - the one form the scripted rewrite missed (no trailing `::`), caught by the compiler |
| `src/ir.rs` | changed | intra-doc link `[`crate::json_rw`]` in the module doc comment updated to `[`crate::json::json_rw`]` |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | `src/json_rw.rs` + `src/json_schema.rs` -> `src/json/...`; new entry `src/json/mod.rs` (sorted position) |
| `docs/dev/CODE_MAP.md` | changed | NEW fragment dev:code-map-json + "JSON substrate (src/json/) (3 files)" section (mod.rs + both files); the two json entries removed from "Validator + IR" (now 2 files, together with task 1's leftover hash entry which is documented in the utils section); Frontend area count 13 -> 10 |
| `docs/dev/{cli_commands,ARCHITECTURE,DECISIONS,backend_c,ir_json_rw,DEPENDENCIES,ALGORITHMS}.md` | changed | all `src/json_rw.rs[::item]` / `src/json_schema.rs[::item]` doc->code refs rewritten to the new paths (per correction 1a - incremental, in this commit) |
| `docs/llm/{ir_json_rw,TASK_MAP}.md`, `docs/user/MIGRATION.md` | changed | same path rewrites; TASK_MAP gains the file-list entry for src/json/mod.rs (gate coverage requirement) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - pure move + import-path rewrite; 535/535 green.

## Removed functionality

None.

## New functionality

- `src/json/` directory: the JSON substrate (I/O + schema emitter) in one place,
  per proposal rule 8 (the schema emitter sits with the I/O it drives).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (after fixing the one grouped-import form the script missed) |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline |
| `cargo test --test dev_docs_test` (9) | OK - green on first run after the incremental re-sync (correction 1a workflow) |

## Found problems/issues/mismatches

- The scripted rewrite pattern (`crate::json_rw::`) missed two non-`::` forms:
  the grouped import in info_format.rs (caught by the compiler as E0432) and an
  intra-doc link in ir.rs's module comment (caught by the zero-old-refs grep).
  Both fixed; future move tasks should grep for the bare module-name forms too.

## Other important findings/information

- The fuzz workspace has three json import sites (parser/validator/ir_roundtrip
  targets) - they were rewritten in this commit, so no json lockstep remains for
  later tasks.
- CODE_MAP's area sections are being restructured incrementally (correction 1a
  pattern): each move task removes its files from the old area section and gives
  the new directory its own fragment + section. After all move tasks, CODE_MAP
  will mirror the proposed tree.
