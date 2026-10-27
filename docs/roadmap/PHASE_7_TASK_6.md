# Phase 7, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `spec-json` format entry (D-P7-8): the machine-readable description of the
IR v1 JSON document (`rfs parse` / `--json` output) in md + JSON Schema
draft-07, plus the SHARED schema emitter/walker that tasks 7-10 reuse.

## Context

- Phase 7 plan: rows 6, 6.1-6.3; decision D-P7-8 (JSON Schema draft-07,
  required/optional marking, enum constraints, widest validator support),
  D-P7-9 (walk checks: no undocumented key at any object level).
- Task 5 landed the `rfs info format` registry + CLI; this task adds the first
  JSON-schema entry and the shared infrastructure for the remaining four.

## Scope

### In scope
- Shared JSON-Schema emitter + md table renderer (`src/json_schema.rs`, new).
- spec-json field model derived from `ir.rs` `to_json` (`src/info_format.rs`).
- Tests: schema document pin, md sections pin, walk of all 50 valid examples.
- BUILD.md row update.

### Out of scope
- ce-trace / mc-report / validate-json / info-json entries (tasks 7-10).

## Planned actions

1. **Shared JSON-Schema emitter** (`src/json_schema.rs`): `FieldType` model
   (Str/Int/Bool/Null/EnumStr/Obj/Arr/Ref/TaggedUnion/Nullable) + `Field`
   (name, type, required, note) + `SchemaDoc` (title, description, root,
   named definitions) - the D-P7-8 compact model extended with named
   definitions + `$ref` and internally-tagged unions, which IR v1 needs for
   recursion (`expr`, `formula`) and reuse (see plan correction). draft-07
   emitter: `$schema`/title/description, root object inlined at top level,
   `properties` + `required` + `additionalProperties: false` at every object
   level, `"kind"` enum per tagged-union variant via `oneOf`, nullable refs as
   `anyOf` (draft-07 ignores `$ref` siblings), pretty JSON through json_rw
   (project rule). Shared md renderer: intro + one `##` section per
   root/definition; field tables (Field/Type/Required/Description) for
   objects, kind tables for tagged unions; `[name]` ref notation. Plus
   `walk_check` - the D-P7-9 sync walker: required keys present, NO
   undocumented key at any object level, types + enum constraints agree;
   collects all errors with JSON paths.
2. **spec-json field model** (`src/info_format.rs`): full IR v1 document model
   derived from `ir.rs` `to_json` (verified against every `IrJson` impl): root
   `ir_version` + `modules[]`; module (name, optional version/on_error keys,
   enums, initial_state string|null, events/outputs constructor groups +
   unit|struct variants + field decls, vars name/type/init, transitions with
   id/state/pattern/guard|null/next_state/emits/assignments/priority|null,
   properties/assumptions); 21 named definitions incl. recursive `expr`
   (int/bool/path/unary/binary) and `formula` (13 kinds), `unary_op` (3) /
   `bin_op` (16) string enums. Register the entry: outputs md, schema (md
   first = default).
3. **Tests** (`tests/info_format_test.rs`): schema document pin (draft-07
   header, strict root object, all 21 definitions, recursive expr/formula
   ($ref to themselves), bin_op 16-value enum, nullable-ref anyOf shape,
   version/on_error optional - not in required); md section pin (one `##`
   section per definition + document table); D-P7-9 walk: ALL valid examples
   (dynamic enumeration incl. tla_adapted/) parsed in-process ->
   `Ir::to_json()` -> `walk_check` against the model - required keys present +
   no undocumented key at any object level; `--list` pin extended to the
   2-entry registry.
4. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/json_schema.rs` | new - shared field model + draft-07 emitter + md table renderer (extended in task 7 with `FreeObj`/`Union`/`extra_sections`) |
| `src/info_format.rs` | changed - spec-json field model + entry registration (outputs: md, schema) |
| `src/lib.rs` | changed - `pub mod json_schema;` (test access) |
| `tests/info_format_test.rs` | changed - 3 new tests (schema pin, md sections pin, walk of all 50 valid examples) |
| `BUILD.md` | changed - section 3 spec-json row; baseline re-pin |

## Task deliverables

- `rfs info format spec-json --output md|schema` works.
- Shared emitter/walker in place for tasks 7-10.
- Full test matrix green.

## Task exit criteria

- [x] Shared draft-07 emitter + md renderer built (DONE: `src/json_schema.rs`; reused unchanged by tasks 7-10)
- [x] spec-json model covers the full IR v1 document (DONE: derived from `ir.rs` `to_json`; extended field model per correction 6.1)
- [x] Schema document pin + md sections pin (DONE: `spec_json_schema_document_pin`, `spec_json_md_pins_all_sections`)
- [x] Walk of all 50 valid examples' parse JSON: required keys present, no undocumented keys at any object level (DONE: `spec_json_walk_all_valid_examples`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 499 / workspace 523 / smt 501, info_format_test 13)

## Non-goals

- No changes to the IR v1 JSON wire format itself.
- No if/then conditionals in the schema (draft-07 widest-validator-support
  choice - D-P7-8).

## Task corrections

- **6.1** (2026-10-26): D-P7-8's compact field model sketch (`Str/Int/Bool/
  Null/Obj/Arr`) could not express what IR v1 JSON actually needs: named
  definitions + `$ref` (recursion in `expr`/`formula`, reuse of
  `type`/`output_expr`/...), internally-tagged unions (`kind` discriminator ->
  `oneOf` per variant), and nullable fields (`string|null`, `expr|null`). The
  shipped model adds `EnumStr`, `Ref`, `TaggedUnion`, `Nullable` to the sketch;
  draft-07 constraints: `additionalProperties:false` everywhere, nullable refs
  as `anyOf` ($ref siblings are ignored in draft-07). The sketch was a starting
  point, not a contract - D-P7-8's own requirements (required/optional marking,
  enum constraints) plus IR v1's recursive tagged unions forced the extension.
  Emitter + md renderer + walk_check stay shared for tasks 7-10.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (work notes existed as `docs/roadmap/tasks/task_6.md`, since removed).
  Sources: commit `6dfac95` message + diff, TASKS.md entry for task 6,
  `PHASE_7_PLAN.md` corrections table, BUILD.md baseline re-pins.
- Committed as `6dfac95` (2026-10-26); the commit also carries the TASKS.md
  entry and the INTRO.md row update (tasks 1-6 of 16 complete).
