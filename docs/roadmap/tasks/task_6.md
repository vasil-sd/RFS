# Task 6 - spec-json format entry (md + schema)

## Subtasks

### 6.1 Shared JSON-Schema emitter (`src/json_schema.rs`, new)
- [x] `FieldType` model (Str/Int/Bool/Null/EnumStr/Obj/Arr/Ref/TaggedUnion/Nullable) + `Field` (name, type, required, note) + `SchemaDoc` (title, description, root, named definitions) - the D-P7-8 compact model extended with named definitions + `$ref` and internally-tagged unions, which IR v1 needs for recursion (`expr`, `formula`) and reuse (see plan correction 2026-10-26 / task 6)
- [x] draft-07 emitter: `$schema`/title/description, root object inlined at top level, `properties` + `required` + `additionalProperties: false` at every object level, `"kind"` enum per tagged-union variant via `oneOf`, nullable refs as `anyOf` (draft-07 ignores `$ref` siblings), pretty JSON through json_rw (project rule)
- [x] Shared md renderer: intro + one `##` section per root/definition; field tables (Field/Type/Required/Description) for objects, kind tables for tagged unions; `[name]` ref notation
- [x] `walk_check` - the D-P7-9 sync walker: required keys present, NO undocumented key at any object level, types + enum constraints agree; collects all errors with JSON paths

### 6.2 spec-json field model (`src/info_format.rs`)
- [x] Full IR v1 document model derived from `ir.rs` `to_json` (verified against every `IrJson` impl): root `ir_version` + `modules[]`; module (name, optional version/on_error keys, enums, initial_state string|null, events/outputs constructor groups + unit|struct variants + field decls, vars name/type/init, transitions with id/state/pattern/guard|null/next_state/emits/assignments/priority|null, properties/assumptions); 21 named definitions incl. recursive `expr` (int/bool/path/unary/binary) and `formula` (13 kinds), `unary_op` (3) / `bin_op` (16) string enums
- [x] Entry registered: outputs md, schema (md first = default)

### 6.3 Tests (`tests/info_format_test.rs`)
- [x] Schema document pin: draft-07 header, strict root object, all 21 definitions, recursive expr/formula ($ref to themselves), bin_op 16-value enum, nullable-ref anyOf shape, version/on_error optional (not in required)
- [x] md section pin: one `##` section per definition + document table
- [x] D-P7-9 walk: ALL valid examples (dynamic enumeration incl. tla_adapted/) parsed in-process -> `Ir::to_json()` -> `walk_check` against the model - required keys present + no undocumented key at any object level; zero drift found (model was built from the writer, walk proves it)
- [x] `--list` pin extended to the 2-entry registry

### 6.4 Docs (`BUILD.md`)
- [x] Section 3: dedicated row updated (13 pass + spec-json coverage); root re-pinned **499** (+6), workspace **523**; section 5 baseline re-measured (499/523/501-smt)

## Status

COMPLETE (2026-10-26).

Verification: info_format_test **13 pass / 0 fail** (was 10); full root suite **499 pass / 0 fail** (was 493: +3 json_schema unit tests incl. a drift-rejection negative control, +3 spec-json integration tests); workspace **523 pass / 0 fail**; `--features smt` **501 pass / 0 fail**; clippy zero warnings in the new/changed files (36 pre-existing elsewhere, unchanged); fmt clean; docs gate green.

Notes for later tasks:
- Tasks 7-10 (ce-trace, mc-report, validate-json, info-json) reuse `json_schema::{SchemaDoc, Field, FieldType, emit_schema_pretty, render_md, walk_check}` - build each model against the code's own key constants (`mc_report_json.rs` `JSON_KEY_*`, `ir.rs` `render_json`) and add a walk test over real generated documents.
- mc-report should `$ref` the ce-trace model via `definitions` (D-P7-8) - the emitter already supports cross-model refs only within one document, so task 8 will inline the ce-trace definitions into the mc-report schema under distinct names (or extend the emitter with external refs if cleaner).
