# Phase 5, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_5_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/json_rw.rs` | The toolchain's ONLY JSON implementation (hand-rolled, no external crates). Value model `Json { Null, Bool, Int(i64), Str, Arr, Obj(Vec<(String, Json)>) }` with ORDERED objects (declaration order by construction; duplicate keys rejected on read). Writer: compact + 2-space pretty layouts, pinned escaping table (`" \ \b \t \n \f \r`, other U+0000..=U+001F as `\u00XX` lowercase hex, DEL/non-ASCII raw), `i64` number rendering; `Display` = compact form. Reader: strict JSON (no trailing garbage, no NaN/Infinity, no leading zeros, no floats - integer-only scope documented, no duplicate keys, depth limit 512, surrogate-pair handling) with byte-offset errors (`JsonError { message, offset }`) and typed extraction helpers (`get`, `as_str/as_int/as_bool/as_arr/as_obj`). The module doc opens with the full decision rationale (key-ordering guarantee, always byte-identical text output across platforms/Rust versions/dependency updates, zero external dependencies consistent with the generated-code constraint, wire format under our control) - the source text task 15 copies into DESIGN.md section 23 and KB.md. 24 unit tests pin escaping/layout/number rendering, every reader error class with exact offsets, and the round-trip property on an IR-shaped + CLI-shaped corpus. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/lib.rs` | changed | registered `pub mod json_rw;`. |
| `src/ir.rs` | changed | ALL serde derives/attributes removed (was on every IR type). Hand-written `to_json`/`from_json` for all 20 IR types behind a private `IrJson` trait + field helpers with path-carrying error messages (`missing field 'x' at $.modules[0].transitions[1].guard`, `unknown Formula kind '...' (expected one of: ...)`). Public API: `Ir::to_json(&self) -> Result<Json, String>` and `Ir::from_json(&Json) -> Result<Ir, String>`. Wire format is a PINNED CONTRACT byte-identical to the pre-migration external-crate output: declaration-order keys, `"kind"` first for internally-tagged enums, unit variants of `UnaryOp`/`BinOp` as bare strings, `null` for absent `Option` fields EXCEPT `Module.version`/`Module.on_error` which are omitted when `None`. Deviation from the task sketch (plan correction C3): `to_json` is FALLIBLE - unsigned IR fields (`u64` bounds/int literals, `usize` ids) render as non-negative JSON numbers and serialization fails (no clamping) when a value exceeds `i64::MAX`; `from_json` rejects negative integers for those fields. |
| `src/frontend.rs` | changed | `.json` input parsing (`convert_file_to_ir`) and pretty `.json` output (`render_ir`, trailing newline preserved) now go through `json_rw::parse` + `Ir::to_json`/`to_string_pretty`. |
| `src/training.rs` | changed | JSONL validation/snippet extraction use `json_rw::Json` + typed helpers instead of the external `Value` type. |
| `src/printer.rs` | changed | round-trip helpers (`roundtrip_rfs`, `roundtrip_json`) and their tests use `Ir::to_json`/`from_json`. |
| `src/parser.rs` | changed | test-only: `ir_json_roundtrip` uses the new API. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| (none) | - | Per the finalized plan, DESIGN.md section 23 + KB.md receive the rationale in task 15 (docs sync); this task's module doc is the source text. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` | changed | `serde` (+ derive) and `serde_json` dependencies REMOVED; only `nom` + optional `z3` remain. |
| `Cargo.lock` | changed | regenerated; no serde entries for the `rfs` crate (the lock still lists serde transitively via the dev-dependency `chumsky`, which is external and unaffected). |

## Changed functionality

- None user-visible: every JSON the tool produced before (IR round-trips, `parse --output json` files) is byte-identical after the migration - verified against a captured pre-migration golden dump for ALL 45 valid examples in both compact and pretty layouts (temporary comparison test, deleted after verification). Error messages for invalid `.json` input change (they now come from `json_rw` with byte offsets instead of the external crate's messages) - no test pins the old wording.

## Removed functionality

- serde/serde_json as toolchain dependencies and code usage: `grep -ri serde src/ tests/ Cargo.toml` returns nothing.

## New functionality

- `src/json_rw.rs`: full hand-rolled JSON reader/writer (see Added source files) with pinned formatting, byte-offset diagnostics, and typed extraction - the foundation for task 3 (`--json` CLI plumbing), task 10 (MC counterexample JSON via an embedded copy), and task 13 (`validate --json`).
- Strict-reader guarantees: duplicate keys, trailing commas, trailing garbage, NaN/Infinity, leading zeros, floats, and >512 nesting are all rejected with byte-offset errors.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `json_rw::tests::*` (24 tests) | new | writer escaping/layout/number pins; reader success cases (all value kinds, whitespace tolerance, escapes, surrogate pairs, number boundaries); every error class with exact byte offsets (trailing garbage, duplicate keys, number syntax, NaN/Infinity, bad strings/escapes/surrogates, structural errors incl. trailing commas, depth limit); typed extraction; round-trip property on an IR-shaped + CLI-shaped corpus (compact + pretty, key-order preservation). |
| `frontend_test::ir_json_golden_compact_pins_key_order` | new | representative spec (enums, bit types, unit+struct variants, vars, guards, emits with args, assignments, liveness property) -> EXACT compact JSON text pinned. |
| `frontend_test::ir_json_golden_pretty_pins_layout` | new | same spec -> EXACT pretty (2-space) layout pinned. |
| `frontend_test::ir_json_roundtrip_property_all_valid_examples` | new | exit criterion: for all 45 valid examples, parse rfs -> to_json -> from_json == original IR. |
| `frontend_test` serde usages (6 sites) | changed | switched to `json_rw` (`ir_json` helper + new `json_ir` helper). No re-pinning needed - output byte-identical. |
| `examples_test`, `training_test`, `benchmark_mc_test`, `parser.rs`/`printer.rs` tests | changed | switched to the new API; benchmark report now built with ordered `Json::obj` (same keys/values as before). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` | OK (**257/257**; lib 149 incl. 24 new json_rw tests; frontend 44 incl. 3 new golden/round-trip tests; all other suites unchanged: parity 7, benchmark MC 5, codegen 26, examples 3, integration 2, library comparison 4, liveness MC 9, monitor integration 4, training 4) |
| `cargo generate-code-tests && cargo test-generated-code` | OK (**537/537** in 179 modules - baseline unchanged; no generated-code changes in this task) |
| `cargo clippy --all-targets` | OK (zero new warnings; pre-existing baseline now 11 lib + 1 codegen_test, down from 14+1 because the replaced serde-derive code carried 3 of them) |
| `cargo fmt --check` | OK (clean) |
| byte-identity vs pre-migration format | OK - temporary golden-dump test compared new output against captured external-crate output for all 45 valid examples, compact AND pretty: identical (test deleted after verification; the permanent pins are the two golden tests above) |

## Found problems/issues/mismatches

- `to_json` fallibility (plan correction C3): the task file's sketch said `to_json(&T) -> Json`, but unsigned IR fields can hold values up to `u64::MAX` (the parser accepts them), which an i64 value model cannot represent. Chosen: fallible `to_json` with a clear error instead of silent clamping; `from_json` is strict about non-negativity. No current example or training snippet exceeds the range, so no behavior change in practice.
- The reader initially accepted trailing commas and missed whitespace after array commas in pretty layout (caught by the round-trip test on pretty output); fixed with strict key-after-comma handling and a skip at the top of the array loop.
- `Cargo.lock` still contains serde entries: transitive dependency of the dev-dependency `chumsky` (parser comparison suite only) - expected and outside this task's scope; the exit-criterion grep covers `src/ tests/ Cargo.toml`, all clean.

## Other important findings/information

- The migration was byte-exact on the FIRST full-suite run: no existing test required re-pinning (the plan anticipated possible IR JSON text changes; declaration order already matched serde's struct-field ordering, and kind-first tagging was reproduced exactly).
- Baseline for task 3+ (CLI infrastructure): `json_rw` is dependency-free and embeddable - task 10 will copy the writer module verbatim into generated MC engines.
