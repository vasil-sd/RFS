# Phase 5, Task 10 - Implementation status

> Written once when all exit criteria of `docs/roadmap/PHASE_5_TASK_10.md` are
> reached. Not rewritten afterwards; later fixes go into TASKS.md entries and
> follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | All changes extend existing modules; the JSON writer is embedded in the generated engines as string constants, not a new toolchain module. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen_mc.rs` | changed | `CounterexampleFormat` (wire names + strict parse) + pinned `COUNTEREXAMPLE_FORMAT_ERROR`; the JSON report model: `JSON_KEY_*` field-name constants with the key-ordering byte-parity contract documented in the module header, `JsonIo`/`JsonStep`/`JsonCounterexample` (`trace` / `lasso` prefix+cycle+cycle_length / `deadlock-config`) and `McJsonReport::to_json()` (canonical document builder over `json_rw::Json`). +7 unit pins: format wire/parse, document shape byte-pins for all five status/reason combinations, writer escaping contract. |
| `src/codegen.rs` | changed | `BackendOptions.counterexample_format` (default Text) + `CodegenRequest::Generate` field + `--counterexample-format json\|text` parsing (invalid value -> pinned error). |
| `src/main.rs` | changed | Destructure + pass the new option through to `BackendOptions`. |
| `src/backends/rust.rs` / `src/backends/c.rs` | changed | `mc` command help: usage line + options block + runtime-accepts list gain `--counterexample-format json\|text` (identical text in both backends). |
| `src/backends/rust_mc.rs` | changed | Emits `const DEFAULT_COUNTEREXAMPLE_FORMAT: u8`; new `JSON_WRITER_RUST` constant - embedded minimal copy of the `json_rw.rs` compact writer (same escaping contract, u64 integers) + canonical document builders (`build_mc_json_doc`, `mc_json_io/step`, `ce_trace_json`, `ce_lasso_json`, `ce_deadlock_json`); new `json_renderers_rust()` emits per-module `mc_json_input/output/vars` (names via `rust_ident`, the text printers' name source; bool -> Bool, bit<N> -> Int `as u64`, enum -> Str via Debug). `run_exact`: runtime `--counterexample-format` flag + JSON branch in BOTH report paths (sample-0 early exit and main report) - the text path is byte-for-byte untouched. |
| `src/backends/c_mc.rs` | changed | Mirror: `#define DEFAULT_COUNTEREXAMPLE_FORMAT`; new `JSON_WRITER_C` constant - direct-append port of the same writer contract (`mc_json_buf_t` + `mjb_*`, `mjb_escape` with the identical escaping table, `mjb_done` NUL-termination) + `mc_json_doc_head` (leaves the object open for the counterexample) + the three `ce_*_json` builders; new `json_renderers_c()` emits per-module `mc_json_input/output/vars` append functions (forward-declared before the writer block). `run_exact`: same flag + JSON branch in both report paths; text path untouched. |
| `src/mc_plan.rs` | changed | Pre-existing fix found during verification (C10-2): `ValueDomain::Enum` now carries the enum NAME (`{ name, variants }`) so the Rust engine can render qualified variant paths. |

## Added/changed/extended test files

| Name | Operation | Description |
|---|---|---|
| `tests/backend_parity_test.rs` | changed | +3 tests: (1) `mc_json_report_parity_all_property_bearing_examples` - for EVERY property-bearing example the JSON report is byte-identical across backends, strict-parses with the toolchain reader, round-trips to identical bytes (canonical form), and every field agrees with the TEXT run's parsed data (status/metrics/reason/label/step/statistics; counterexample present iff fail, kind matches reason, trace step count / lasso prefix+cycle vs text steps + cycle_length / deadlock depth); (2) `mc_json_five_status_reason_cases` - the five combinations (pass-complete ex.43, property-violation ex.13, liveness/lasso inline spec with cycle_length 2, deadlock ex.04, unknown-state-limit ex.43 `--max-states 1`) byte-identical across backends; (3) `mc_json_golden_document_pin` - full-document byte pin on a small spec exercising enum vars + an enum input field + an enum output field in one trace counterexample. The text-unchanged guard is the pre-existing pinned suite (summary-line pins, liveness stdout byte-pins), which passes unmodified. |
| `tests/codegen_test.rs` | changed | `mc_exact_flags_embedded_as_runtime_defaults` extended (`DEFAULT_COUNTEREXAMPLE_FORMAT` 0/1 in both backends); `mc_flag_parsing_roundtrip` extended (json parse, text default, pinned invalid-value error); +2 tests: `mc_json_writer_embedded_and_dependency_free` (writer + document builder + all three kind builders present in BOTH backends' generated MC source; no `serde` reference) and `mc_enum_input_field_domain_qualified_both_backends` (C10-2 regression: concrete enum input field -> qualified paths in Rust, both engines compile and run byte-identically; rustc/gcc-gated). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | changed | 4.1: `Generate` struct + defaults + parse-error list gain the flag; 5.3: `CounterexampleFormat` + JSON report model with the key-ordering byte-parity contract and the embedded-writer rationale (u64 integers, dependency-free). |
| `BUILD.md` | changed | Counts re-pinned (root 342 -> 354, workspace 350 -> 362, smt 344 -> 356; lib 179 -> 186, parity 7 -> 10, codegen 43 -> 45), known-good baseline updated (2026-10-22, after Phase 5 task 10). |
| `KB.md` | changed | 4.5: MC report-format paragraph (flag semantics, document shape, exact-mode-only scope); 4.2: `codegen_mc.rs` row extended. |
| `docs/roadmap/INTRO.md` | changed | Phase-state row 5 -> "tasks 1-7 + 8-10 of 16 done". |

## Other file changes

- `TASKS.md`: one new entry.

## Changed functionality

- None to existing output: the default (`text`) report is byte-identical to pre-task (all pre-existing text pins pass unmodified); fuzz and smt-bounded modes keep their own output; production/test codegen untouched.

## Removed functionality

- None.

## New functionality

- `rfs codegen <backend> mc ... --counterexample-format json|text` (codegen-time default, embedded as `DEFAULT_COUNTEREXAMPLE_FORMAT`, runtime-overridable by the generated binary): the exact engine prints ONE compact JSON document instead of the text report - `{status, metrics{states,transitions,max_depth}, [reason,label,step], statistics{unexpected_terminations,monitor_size_bits,memory_estimate_bytes}, [counterexample]}` with `counterexample.kind` in `trace` | `lasso` (prefix + cycle + cycle_length) | `deadlock-config`; step objects carry `{input{variant,fields}, outputs[], from_state, to_state, vars}` with values rendered as bool/int/enum-name strings. Key ordering is the byte-parity contract; JSON is byte-identical across backends for all five status/reason combinations (parity suite + golden pin). Generated binaries stay dependency-free (embedded minimal writer copy; no serde).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| C10-1: embedded writer uses u64 integers (shared writer is i64) | Every value in the document is non-negative, and bit<64> fields reach 2^64-1 - an i64 copy would render those as negative JSON numbers while the text report prints unsigned decimal. Rendering is byte-identical to the shared `json_rw.rs` writer for all values <= i64::MAX (the range the toolchain-side builder and its pins use); the C port uses `unsigned long long`. The exact engine already reduces bit values to u64 in config packing (`vars.x as u64`), so this matches existing semantics. |
| C10-2: pre-existing Rust MC bug fixed - concrete enum input fields enumerated BARE variant names | Found while verifying the JSON renderers on a spec with an enum-typed input field: `all_inputs()` emitted `[Red, Green]` (does not compile) instead of `[Color::Red, Color::Green]`; the C engine was already qualified. No valid example in the corpus uses enum input fields, so no test caught it before. Fix: `ValueDomain::Enum` carries the enum name (`mc_plan.rs`); `rust_mc.rs` renders qualified paths; C destructure sites updated (behavior unchanged). Regression test: `mc_enum_input_field_domain_qualified_both_backends`. |
| C10-3: JSON scope is exact mode only | The planned document shape (states/transitions/max_depth, trace/lasso/deadlock counterexamples) is exact-mode data. Fuzz mode keeps its per-run summary output (explicit non-goal); smt-bounded mode has its own `mode=smt-bounded` summary lines and no reachable-state metrics - the flag is parsed but not embedded in those engines. Documented in help text + CODEGEN.md 5.3. |

## Verification

- Default output byte-identical to pre-task: all pre-existing text pins pass unmodified (summary-line field pins over all property-bearing examples, liveness stdout byte-pins, benchmark MC suite).
- JSON strict-parses (toolchain reader) and round-trips to identical bytes on every property-bearing example (canonical form); values agree with the text run's parsed fields; counterexample kind/step data agrees per reason.
- JSON byte-identical across backends: all property-bearing examples + the five status/reason cases + golden full-document pin (enum vars + enum input field + enum output field).
- Generated binaries dependency-free: no `serde` reference in either backend's generated MC source (marker test); both compile standalone (rustc / gcc -Wall, warning-free) including fuzz-mode binaries.
- cargo test 354/354 root (was 342; +7 lib codegen_mc pins, +2 codegen_test, +3 parity), 362/362 workspace (was 350), 356/356 `--features smt` (was 344); generated-code 1453/1453 unchanged; clippy identical to the 12-warning baseline (diff-verified); fmt clean.
