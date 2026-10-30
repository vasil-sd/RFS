<!-- llm-budget: 150 -->
# RFS IR + JSON Read/Write (Class B)

Compact card for LLM assistants working on the IR model, JSON wire format,
schemas, or spec hashing.

<a id="llm:ir-json-rw"></a>
## Card

**Responsibility.** The IR document model and the hand-rolled JSON stack that
serializes it - the PINNED wire-format contract at the heart of the
toolchain. Four files: `src/ir.rs` (types + per-type to_json/from_json),
`src/json_rw.rs` (the only JSON impl: value model, writer, strict reader),
`src/json_schema.rs` (draft-07 emitter + md renderer + sync walker),
`src/spec_hash.rs` (hand-rolled SHA-256 for artifact headers).

**Key items.**

| Item | Role |
|---|---|
| `src/ir.rs::Ir` / `src/ir.rs::Module` | the document: ir_version + modules; Module has 11 fields (name, version?, on_error?, enums, initial_state?, events, outputs, vars, transitions, properties, assumptions) |
| `src/ir.rs::IR_VERSION` | = 1; read policy D-P6-2: from_json loads 0..=1, rejects higher with a pinned error |
| `src/ir.rs::IrJson` | private per-type to_json/from_json trait; every error carries a JSONPath-style location |
| `src/ir.rs::uint_json` / `src/ir.rs::uint_field` | u64 value model: to_json FAILS above i64::MAX (no clamping); from_json rejects negatives |
| `src/json_rw.rs::Json` | value model: Null/Bool/Int(i64)/Str/Arr/Obj - objects are ORDERED pairs; integers only (no float) |
| `src/json_rw.rs::parse` | strict reader: no trailing garbage, no leading zeros, no duplicate keys, depth <= MAX_DEPTH (512), integer-only; byte-offset errors |
| `src/json_schema.rs::SchemaDoc` / `src/json_schema.rs::FieldType` | one documented JSON document: root model + named definitions + extra md sections; 12 field-type shapes incl. TaggedUnion, Nullable, FreeObj, Union |
| `src/json_schema.rs::emit_schema_pretty` | draft-07 schema (additionalProperties:false everywhere except FreeObj) rendered through json_rw |
| `src/json_schema.rs::render_md` | markdown field/kind tables for rfs info format --output md |
| `src/json_schema.rs::walk_check` | D-P7-9 sync: required keys present, no undocumented key, types/enums agree; returns ALL violations |
| `src/spec_hash.rs::spec_hash16` | first 16 hex of SHA-256 over the whole-file canonical IR JSON (D-P6-3); stamped into every artifact header |
| `src/info_format.rs::spec_json_schema` | the spec-json field model, derived from ir.rs to_json - must stay in lockstep with it |

**Invariants that must not break.**

- Declaration-order keys, byte-pinned by golden tests (frontend_test compact + pretty pins) - never alphabetize.
- Option fields render null EXCEPT Module.version / Module.on_error, which are omitted when None (both optional on read).
- Internally tagged enums: "kind" FIRST, snake_case; UnaryOp/BinOp serialize as bare strings.
- The u64 value model is 64-bit signed end to end: fail above i64::MAX, reject negatives - never clamp.
- Read policy: 0..=IR_VERSION loads, higher rejected (D-P6-2); additive changes do NOT bump IR_VERSION (D10).
- Strict reader rules are pinned per-error with byte offsets (24 unit tests in json_rw).
- Schemas: additionalProperties:false at every object level; FreeObj is the only exception (its keys are data).
- Spec hash input is the WHOLE file's canonical JSON - multi-module files share one hash.

**Change impact (wire format).** The most constrained area in the repo:
update `src/ir.rs` to_json/from_json AND `src/info_format.rs::spec_json_schema`
together (the walk test fails on drift); expect the golden pins + round-trip
property + version-policy test in tests/frontend_test.rs to fire; json_rw
formatting changes also hit cross-backend byte parity (generated MC engines
embed writer copies). Verify: cargo test --lib json_rw, cargo test --test
frontend_test, cargo test --test info_format_test. Full table:
llm:change-impact (task 13.2).

**Deep dive.** dev:ir-json-rw-overview + dev:ir-model / dev:ir-wire-format /
dev:json-rw / dev:json-schema / dev:spec-hash in docs/dev/ir_json_rw.md;
algorithm levels: dev:algorithms-ir-codegen.
