# Phase 7, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_6.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/json_schema.rs` | shared field model (`FieldType` incl. the correction-6.1 extensions `EnumStr`/`Ref`/`TaggedUnion`/`Nullable`) + JSON Schema draft-07 emitter (pretty JSON via json_rw, `additionalProperties:false`, nullable refs as `anyOf`) + md table renderer; `walk_check` enforces "no undocumented KEY at any object level" for fixed objects |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/info_format.rs` | changed - spec-json field model (full IR v1 document derived from `ir.rs` `to_json`) + entry registration (outputs: md, schema) |
| `src/lib.rs` | changed - `pub mod json_schema;` (test access) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 spec-json row; baseline re-pin (root 499 / workspace 523 / smt 501) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | task 6 entry recorded |
| `docs/roadmap/INTRO.md` | changed | Phase 7 row -> implementing (tasks 1-6 of 16 complete) |

## Changed functionality

None (new command surface only; existing behaviour untouched).

## Removed functionality

None.

## New functionality

- `rfs info format spec-json --output md|schema`: the IR v1 JSON document
  described in markdown + JSON Schema draft-07 (named definitions + `$ref`,
  internally-tagged unions as `oneOf` per variant, nullable fields).
- Shared schema emitter/walker infrastructure for tasks 7-10.

## Tests add/change/remove

- Added (6 for the root count): 3 `json_schema.rs` library unit tests (incl.
  a drift-rejection negative control) + 3 integration tests:
  `spec_json_schema_document_pin`, `spec_json_md_pins_all_sections`,
  `spec_json_walk_all_valid_examples` (walk of all 50 valid examples' parse
  JSON against the model).
- Changed: `list_pins_all_registered_formats` extended to the 2-entry registry.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 499 pass / 0 fail |
| `cargo test --workspace` | OK - 523 pass / 0 fail |
| `cargo test --features smt` | OK - 501 pass / 0 fail (z3 env per TOOLCHAIN.md 5.2) |
| `tests/info_format_test.rs` | OK - 13 tests (+3) |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- D-P7-8's compact field model sketch (`Str/Int/Bool/Null/Obj/Arr`) could not
  express what IR v1 JSON actually needs (named definitions + `$ref`,
  internally-tagged unions, nullable fields) - the shipped model adds
  `EnumStr`/`Ref`/`TaggedUnion`/`Nullable`; draft-07 constraints: 
  `additionalProperties:false` everywhere, nullable refs as `anyOf` ($ref
  siblings are ignored in draft-07) (plan correction 6.1).

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `6dfac95` message + diff, TASKS.md entry for task 6, BUILD.md baseline
  re-pins.
- Notes for later tasks: tasks 7-10 (ce-trace, mc-report, validate-json,
  info-json) reuse `json_schema::{SchemaDoc, Field, FieldType,
  emit_schema_pretty, render_md, walk_check}` - build each model against the
  code's own key constants and add a walk test over real generated documents.
  mc-report should `$ref` the ce-trace model (D-P7-8); the emitter supports
  cross-model refs only within one document, so task 8 inlines the ce-trace
  definitions into the mc-report schema under distinct names (shipped form:
  shared `counterexample_model()` builder - see PHASE_7_TASK_8.md corrections).
- Committed as `6dfac95` (2026-10-26), 9 files changed (+1320/-17).
