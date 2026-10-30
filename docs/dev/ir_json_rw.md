# RFS IR + JSON Read/Write (Class A)

The IR type system and the hand-rolled JSON stack that serializes it: the
pinned wire-format contract at the heart of the toolchain.

<a id="dev:ir-json-rw-overview"></a>
## Overview

RFS normalizes every spec into a single in-memory document - the `Ir` type
(`src/ir.rs::Ir`) - and pins its JSON wire format byte-for-byte. Four files
make up this area:

| File | Lines | Role |
|---|---|---|
| `src/ir.rs` | 1432 | IR types + hand-written per-type JSON (de)serialization (`IrJson`) |
| `src/json_rw.rs` | 961 (24 unit tests) | the ONLY JSON implementation in the toolchain: value model, writer, strict reader |
| `src/json_schema.rs` | 767 (5 unit tests) | draft-07 schema emitter + markdown renderer + sync walker for the JSON format entries |
| `src/spec_hash.rs` | 158 (2 unit tests) | hand-rolled SHA-256; the spec hash stamped into every generated artifact header |

Position in the pipeline, both directions:

- `.rfs` source is parsed straight into an `Ir` document by the parser
  (see `dev:parser-overview`); there is no intermediate on-disk stage between
  parse and the downstream consumers (codegen, model checking, diagrams,
  info).
- `.json` input loads the same type directly:
  `src/frontend.rs::convert_file_to_ir` runs `src/json_rw.rs::parse` and then
  `src/ir.rs::from_json`; JSON output is the mirror image in
  `src/frontend.rs::render_ir` (pretty rendering + trailing newline).
- The canonical whole-file rendering (`src/ir.rs::to_json`) feeds the spec
  hash stamped into every generated artifact header (`src/codegen.rs::run`,
  see the spec-hash section below), and the field model of the `spec-json`
  format entry is derived from the same writers
  (`src/info_format.rs::spec_json_schema`).

Other consumers: `src/printer.rs::roundtrip_rfs` (rfs -> json -> IR equality
behind the printer's round-trip guarantee), the generated MC engines'
embedded writer copies (see the reader/writer section), and the fuzz target
`fuzz/fuzz_targets/ir_roundtrip.rs` (to_json -> from_json -> to_json byte-
stability on random valid IRs).

Where to go next:

- algorithm levels (L1 abstract / L2 structures + invariants / L3 details):
  `dev:algorithms-ir-codegen` in ALGORITHMS.md;
- file entries: `dev:code-map-frontend` in CODE_MAP.md;
- decision rationale: D10, D-P5-2, D-P6-2, D-P6-3, D-P7-8, D-P7-9 in
  DECISIONS.md; the user-facing version policy in docs/user/MIGRATION.md.

Code: `src/ir.rs::Ir`, `src/ir.rs::IR_VERSION`, `src/json_rw.rs::Json`,
`src/json_rw.rs::parse`, `src/json_schema.rs::SchemaDoc`,
`src/spec_hash.rs::spec_hash16`, `src/frontend.rs::convert_file_to_ir`,
`src/codegen.rs::run`

<a id="dev:ir-model"></a>
## IR document model

The type tree, top down (every type derives Debug/Clone/PartialEq; the parser
assembles these while parsing - see `dev:parser-grammar` - and the validator
types them - see `dev:validator-typing`):

```text
Ir { ir_version, modules }
Module { name, version?, on_error?, enums, initial_state?, events, outputs,
         vars, transitions, properties, assumptions }        (11 fields)
  OnError { next_state, emits }            - fallback, no pattern bindings
  EnumDecl { name, variants }
  ConstructorGroup { name, variants }      - one per event/output group
    ConstructorVariant: Unit{name} | Struct{name, fields}
      FieldDecl { name, ty }
        Type: Bool | Bit{width} | Enum{name}
  VarDecl { name, ty, init }
  Transition { id, state, pattern, guard?, next_state, emits, assignments,
               priority? }
    EventPattern { group, variant? }       - OutputPattern has the same shape
      PatternVariant: Unit{name} | Struct{name, fields}
        PatternField { name?, pattern }
          PatternExpr: Wildcard | Bind{name} | Int{value} | Bool{value}
                       | EnumVariant{path}
    OutputExpr { group, variant?, args }   - emit arguments are Exprs
    Assignment { var, value }
  Property { name, formula }               - properties AND assumptions
    Formula: InputIs | OutputHas | Expr | Not | And | Or | Implies | Next
             | Always | BoundedAlways{bound} | BoundedEventually{bound}
             | Eventually | Until          (13 variants)
Expr: Int{value} | Bool{value} | Path{path} | Unary{op, expr}
     | Binary{op, left, right}
  UnaryOp: Not | BitNot | Neg
  BinOp: Plus .. LogicAnd                  (16 operators)
```

Two tagging styles in the JSON encoding (details in the wire-format section):
the six internally tagged enums (`Type`, `ConstructorVariant`,
`PatternVariant`, `PatternExpr`, `Formula`, `Expr`) serialize as objects with
a leading "kind" key, while `UnaryOp` and `BinOp` serialize as BARE strings.

**Versioning.** `src/ir.rs::IR_VERSION` is 1 (v1.0, Phase 6). The read policy
(D-P6-2) is enforced in `src/ir.rs::from_json`: documents with `ir_version`
in 0..=IR_VERSION load (older toolchains' output keeps working); a HIGHER
version is rejected with the pinned error "unsupported IR version {n} at
$.ir_version (this toolchain reads 0..={IR_VERSION})". `to_json` always
writes the current version. The evolution rule (D10): additive optional
fields and new enum variants land WITHOUT a version bump - within one version
the schema only grows, and a document carrying a kind an older reader does
not know fails with a pinned unknown-kind error rather than a version error;
breaking changes must bump the constant.

Code: `src/ir.rs::Ir`, `src/ir.rs::Module`, `src/ir.rs::Transition`,
`src/ir.rs::Formula`, `src/ir.rs::Expr`, `src/ir.rs::IR_VERSION`

<a id="dev:ir-wire-format"></a>
## Pinned JSON wire format

Every IR type implements the private `src/ir.rs::IrJson` trait (to_json /
from_json, with a path argument that lands in every error message). The wire
format is a PINNED CONTRACT, byte-identical to what the pre-migration
external-crate format produced. The rules:

1. **Structs: keys in DECLARATION order** - the exact sequence the code
   writes, never alphabetized (the golden tests pin it byte-for-byte).
2. **`Option` fields serialize as `null` when absent**, EXCEPT
   `Module.version` and `Module.on_error`, which are OMITTED entirely when
   None (both optional on read) - the two fields the external-crate format
   skipped.
3. **Internally tagged enums**: the "kind" key comes FIRST with the
   snake_case variant name, followed by the variant's fields in declaration
   order.
4. **Unit op variants** (`UnaryOp`, `BinOp`): bare JSON strings.
5. **Integers**: unsigned IR fields (u32/u64/usize) render as non-negative
   JSON numbers; `src/ir.rs::uint_json` FAILS (rather than clamps) when a
   u64 value exceeds i64::MAX - the value model is 64-bit signed - and
   `src/ir.rs::uint_field` rejects negative integers on read.

Key order per struct type:

| Type | Key order |
|---|---|
| `src/ir.rs::Ir` | ir_version, modules |
| `src/ir.rs::Module` | name, [version], [on_error], enums, initial_state, events, outputs, vars, transitions, properties, assumptions |
| `src/ir.rs::OnError` | next_state, emits |
| `src/ir.rs::EnumDecl` | name, variants |
| `src/ir.rs::FieldDecl` | name, ty |
| `src/ir.rs::ConstructorGroup` | name, variants |
| `src/ir.rs::VarDecl` | name, ty, init |
| `src/ir.rs::Transition` | id, state, pattern, guard, next_state, emits, assignments, priority |
| `src/ir.rs::EventPattern` / `src/ir.rs::OutputPattern` | group, variant |
| `src/ir.rs::PatternField` | name, pattern |
| `src/ir.rs::OutputExpr` | group, variant, args |
| `src/ir.rs::Assignment` | var, value |
| `src/ir.rs::Property` | name, formula |

([key] = omitted when None; every other Option field renders null.)

Kind values per tagged enum (fields beyond "kind", in declaration order):

| Type | kind values |
|---|---|
| `src/ir.rs::Type` | bool - / bit: width / enum: name |
| `src/ir.rs::ConstructorVariant` | unit: name / struct: name, fields |
| `src/ir.rs::PatternVariant` | unit: name / struct: name, fields |
| `src/ir.rs::PatternExpr` | wildcard - / bind: name / int: value / bool: value / enum_variant: path |
| `src/ir.rs::Formula` | input_is: pattern / output_has: pattern / expr: expr / not: inner / and, or, implies, until: left, right / next, always, eventually: inner / bounded_always, bounded_eventually: bound, inner |
| `src/ir.rs::Expr` | int: value / bool: value / path: path / unary: op, expr / binary: op, left, right |

Bare-string enums: `src/ir.rs::UnaryOp` = not, bit_not, neg;
`src/ir.rs::BinOp` = plus, minus, star, shift_left, shift_right, eq, not_eq,
lt, le, gt, ge, bit_or, bit_xor, bit_and, logic_or, logic_and.

**Error shapes.** The small helper set (`src/ir.rs::field`, `str_field`,
`opt_str_field`, `arr_field`, `uint_field`, `bool_field`,
`opt_uint32_field`, `str_list`) produces path-qualified messages: "expected
{type} at {path}, got {actual}" (`src/ir.rs::type_err` with
`src/ir.rs::json_type_name`), "missing field '{key}' at {path}", and for
tagged enums "unknown {what} kind '{got}' at {path} (expected one of: ...)"
(`src/ir.rs::kind` + `src/ir.rs::unknown_kind`). Paths are JSONPath-style,
rooted at "$" by the public `src/ir.rs::from_json`, e.g.
$.modules[0].transitions[2].guard.

**Pins.** `tests/frontend_test.rs` pins the exact compact and pretty text of
a representative spec (GOLDEN_SPEC_RFS: enums, bit types, unit + struct
event/output variants, guards, emits with args, assignments, a liveness
property), the version read policy (v0 loads, v2 rejected with the pinned
message), and the round-trip property parse rfs -> to_json -> from_json ==
original IR over ALL valid examples; `src/printer.rs::roundtrip_rfs`
re-checks it through the printer.

Code: `src/ir.rs::IrJson`, `src/ir.rs::uint_json`, `src/ir.rs::uint_field`,
`src/ir.rs::kind`, `src/ir.rs::unknown_kind`, `src/ir.rs::to_json`,
`src/ir.rs::from_json`, file-level `tests/frontend_test.rs`

<a id="dev:json-rw"></a>
## Hand-rolled JSON reader/writer

`src/json_rw.rs` is the ONLY JSON implementation in the toolchain (D-P5-2):
every JSON document it produces or consumes - IR round-trips, all --json CLI
outputs, training JSONL validation, benchmark reports, and the generated MC
engines' reports - goes through it. The four contractual properties that
justified hand-rolling (key-ordering guarantee, always byte-identical text,
zero external dependencies, stable wire format under our control) are written
up in `dev:algorithms-ir-codegen` L3; this section covers the mechanics.

**Value model.** `src/json_rw.rs::Json` has six variants: Null, Bool, Int
(i64), Str, Arr(Vec<Json>), and Obj(Vec<(String, Json)>) - objects are
ORDERED key/value vectors, so serialization order is exactly declaration
order and the reader rejects duplicate keys. There is NO float variant
(documented scope limit: no current or planned RFS JSON contains floats).

**Writer.** Two layouts, both pinned by tests:

- compact (`src/json_rw.rs::write_compact`, also the Display form):
  "key":value with no whitespace outside strings;
- pretty (`src/json_rw.rs::write_pretty`): 2-space indent, "key": value, one
  element per line; empty objects/arrays stay inline ({} / []).

String escaping (`src/json_rw.rs::write_escaped`): the seven short escapes
(quote, backslash, b, t, n, f, r); every other control character
U+0000..=U+001F as \u00XX with lowercase hex; everything else raw UTF-8
(non-ASCII passes through unescaped). Integers render via i64 Display - no
padding, no exponents.

**Reader.** `src/json_rw.rs::parse` is a strict single-pass reader
(`src/json_rw.rs::Reader`) whose errors carry byte offsets into the input
(`src/json_rw.rs::JsonError`). Strictness rules, each pinned by a unit test:

- no trailing garbage after the document;
- integers only - a decimal point or exponent is a syntax error;
- no leading zeros (0 must stand alone);
- i64 range with an ASYMMETRIC negative limit: -9223372036854775808 parses,
  +9223372036854775808 does not;
- no NaN/Infinity (unexpected character);
- no duplicate object keys (the error points at the second occurrence's
  opening quote) and no trailing commas;
- nesting depth capped at `src/json_rw.rs::MAX_DEPTH` (512);
- strings: raw control characters rejected; \uXXXX escapes decode surrogate
  pairs to non-BMP chars, and ALL escape diagnostics point at the backslash.

**Typed extraction.** Option-returning helpers for consumers that do not want
to pattern-match: get / as_str / as_int / as_bool / as_arr / as_obj on
`src/json_rw.rs::Json`.

**Tests.** The 24 unit tests pin writer escaping/layout, reader error classes
with exact byte offsets, and the round-trip property (compact AND pretty re-
parse to the same value, key order preserved - never alphabetized) over a
corpus covering every value kind.

**Generated engines.** From Phase 5 task 10 on, each generated MC engine
embeds a minimal copy of the compact writer with the SAME escaping contract
and u64 integers (`src/backends/rust_mc.rs::JSON_WRITER_RUST`,
`src/backends/c_mc.rs::JSON_WRITER_C`) so the binary stays self-contained;
cross-backend byte-parity tests pin that toolchain-side and generated-side
rendering are identical for the same run data.

Code: `src/json_rw.rs::Json`, `src/json_rw.rs::parse`,
`src/json_rw.rs::MAX_DEPTH`, `src/json_rw.rs::JsonError`,
`src/json_rw.rs::write_escaped`, `src/backends/rust_mc.rs::JSON_WRITER_RUST`,
`src/backends/c_mc.rs::JSON_WRITER_C`

<a id="dev:json-schema"></a>
## Schema emitter + sync walker

`src/json_schema.rs` (Phase 7, D-P7-8/D-P7-9) is the shared machinery behind
the `rfs info format` JSON entries: each entry builds a model of its document
against the code's own key constants/writers, and this module renders it as
a draft-07 JSON Schema or a markdown field-table document - and verifies real
generated documents against the model.

**The model.** `src/json_schema.rs::SchemaDoc` = title + description + root
value model + named definitions (recursion/reuse; order = md section order)
+ extra_sections (free-form markdown appended after the definitions, for
format aspects with no schema - never emitted into the schema). A field is a
`src/json_schema.rs::Field` (name, type, required flag, one-line note; req /
opt constructors); `src/json_schema.rs::FieldType` has twelve shapes: Str,
Int, Bool, Null, EnumStr (string restricted to a fixed set), Obj (named
fields), Arr (homogeneous), Ref ($ref to a definition), TaggedUnion
(`src/json_schema.rs::TaggedVariant`: kind + fields beyond "kind"), Nullable
(key present, value may be null), FreeObj (ARBITRARY string keys, every value
must conform - for name->value maps whose names come from the spec, e.g. the
ce-trace vars), and Union (anyOf, no discriminator).

**Draft-07 emission.** `src/json_schema.rs::emit_type` maps the model to
schema fragments: simple types -> "type"; EnumStr -> type + enum; Obj ->
`src/json_schema.rs::emit_object` (properties + required +
additionalProperties:false at EVERY object level, so CI validators reject
undocumented keys - the one exception is FreeObj, whose additionalProperties
carries the inner value schema); Arr -> type + items; Ref -> $ref to
#/definitions/name; TaggedUnion -> oneOf of objects, each with a kind enum
field plus the variant's fields; Nullable -> simple types merge into a type
array with null, complex types (refs/unions) use anyOf because draft-07
ignores siblings of $ref; Union -> anyOf. `src/json_schema.rs::emit_schema`
wraps it in the document envelope ($schema draft-07 URI, title, description):
an object root inlines its properties at the top level (the standard shape for
document schemas), any other root is wrapped in allOf; definitions are always
present. `src/json_schema.rs::emit_schema_pretty` renders through json_rw
(project rule: all JSON goes through json_rw).

**Markdown rendering.** `src/json_schema.rs::render_md` produces the md
output: title + description + a legend ([name] = the ## name section below;
"or null" = key present with a JSON null; "no (key omitted)" = key absent),
a ## document section (field table via `src/json_schema.rs::field_table`,
kind table via `src/json_schema.rs::kind_table` for tagged-union roots, or a
single type line), one ## section per definition, then the extra sections.
The Type-column text comes from `src/json_schema.rs::type_md`.

**The sync walker (D-P7-9).** `src/json_schema.rs::walk_check` verifies that
a REAL generated document matches the model: every required key present, no
undocumented key at any object level (`src/json_schema.rs::check_object`),
types and enum constraints agree (`src/json_schema.rs::check_type`, which
resolves Refs through the definitions map, checks a tagged union's kind then
its variant fields, checks every value of a FreeObj, and for a Union accepts
if ANY alternative passes cleanly - otherwise reporting the shortest error
list). It returns ALL violations; empty means in sync.

**Consumers.** The five JSON format entries in `src/info_format.rs` build
their models here: spec-json (the IR v1 document - its field model is derived
from `src/ir.rs` to_json, the single source of truth), ce-trace, mc-report,
validate-json, info-json. Each renders as md or schema via rfs info format;
`tests/info_format_test.rs` pins the emitted documents and runs walk_check
over real data - spec_json_walk_all_valid_examples walks EVERY valid
example's parse JSON against the spec-json model (the guarantee that the model
cannot drift from the writers), plus walks over real ce-trace / mc-report /
validate-json / info-json documents.

Code: `src/json_schema.rs::SchemaDoc`, `src/json_schema.rs::FieldType`,
`src/json_schema.rs::emit_type`, `src/json_schema.rs::emit_schema`,
`src/json_schema.rs::render_md`, `src/json_schema.rs::walk_check`,
`src/info_format.rs::spec_json_schema`, file-level `tests/info_format_test.rs`

<a id="dev:spec-hash"></a>
## Spec hash

`src/spec_hash.rs` implements SHA-256 (FIPS 180-4) by hand - no dependencies,
consistent with the dependency-light stance of D-P5-2. The implementation is
the standard one: the 64 round constants `src/spec_hash.rs::K`, message
padding (append 0x80, zeros until the length is 56 mod 64, then the 64-bit
big-endian bit length), the FIPS initial hash values, the 64-word message
schedule, and the 64-round compression (`src/spec_hash.rs::sha256`).

- `src/spec_hash.rs::sha256_hex` - full 64-character lowercase hex digest.
- `src/spec_hash.rs::spec_hash16` (D-P6-3) - the first 16 hex chars, computed
  over the CANONICAL IR JSON text of the WHOLE input file (the
  `src/ir.rs::to_json` rendering - deterministic key order and formatting per
  D-P5-2). Stable across runs and platforms; any change to the spec changes
  it.

**Consumers.** `src/codegen.rs::run` stamps it into the shared options bag
before generating (CLI rule: the first module of the file; multi-module files
share the file's hash), and `src/backends/rust_test.rs` does the same per
file for unit-test artifacts. Every generated artifact carries the result in
its line-1 header - // Generated by RFS v<toolchain_version> | spec
sha256:<hash16> | module <name> (the artifact-header format entry documents
the full contract) - so users can detect spec/toolchain drift in checked-in
code by regeneration + diff.

**Tests.** NIST FIPS 180-4 vectors are pinned (empty input, "abc", the
56-byte message that forces an extra padding block, and a 112-byte multi-
block message), plus spec_hash16_is_prefix_and_stable: 16 lowercase hex
chars, a prefix of sha256_hex, deterministic, and sensitive to content.

Code: `src/spec_hash.rs::sha256_hex`, `src/spec_hash.rs::spec_hash16`,
`src/spec_hash.rs::sha256`, `src/codegen.rs::run`

## Changing the wire format

This is the most constrained area in the repo - byte-pinned golden tests,
cross-backend parity, and schema sync walks all fail on drift. Checklist:

1. **Evolve per D10.** Additive optional fields (and new enum variants) do
   NOT bump `src/ir.rs::IR_VERSION` - within one version the schema only
   grows, and an older reader meets a new kind with the pinned unknown-kind
   error. A BREAKING change must bump the constant, update the round-trip
   tests, and update docs/user/MIGRATION.md (the read policy is D-P6-2).
2. **Update the model in lockstep.** Any change to `src/ir.rs` to_json /
   from_json must be mirrored in `src/info_format.rs::spec_json_schema` -
   spec_json_walk_all_valid_examples walks every valid example's parse JSON
   and fails on drift (D-P7-9).
3. **Expect the golden pins to fire.** ir_json_golden_compact_pins_key_order
   / ir_json_golden_pretty_pins_layout in `tests/frontend_test.rs` pin exact
   text; the round-trip property runs over all valid examples; the version-
policy test pins the rejection message.
4. **json_rw changes are cross-cutting.** The formatting contract (escaping,
   layout, number rendering) is pinned by the 24 unit tests in
   `src/json_rw.rs` AND by cross-backend byte parity - the generated MC
   engines embed copies of the writer and must stay byte-identical. A reader
   strictness change can break .json inputs that used to load (or accept
   input that should be rejected): update the scope-limits note in the module
   docs.
5. **Verify:** `cargo test --lib json_rw`, `cargo test --test frontend_test`,
   `cargo test --test info_format_test`, then the full BUILD.md section 5
   checklist.
6. **Docs sync:** DESIGN.md (D10 / D-P5-2 / D-P6-2), KB.md; training snippets
   if the change is user-visible.
