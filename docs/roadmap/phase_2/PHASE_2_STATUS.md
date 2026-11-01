# Phase 2 - Implementation status

> Source: "Current status" (Phase 2 bullets) of the former `ROADMAP.md` plus the
> 2026-10-15 `TASKS.md` entry, reorganized on 2026-10-16 into template 2.3 format
> during the docs split.

## Added source files

| Name | Description |
|---|---|
| `src/backends/c.rs`, `c_fsm.rs`, `c_test.rs`, `c_mc.rs` | C backend registered as second plugin (`rfs codegen c production\|test\|mc`): tagged-union inputs/outputs, fixed-width mapping bit<8/16/32/64> -> uintN_t (other widths = codegen error, D8), plain-assert unit tests, bounded MC with the same CLI wire format + LCG constants as the Rust MC |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/parser.rs` | changed | module version headers (`module Name version "x.y.z" {`) and `on_error { next S; emit ...; }` blocks parse + round-trip; semantic use of the previously parsed `priority N;` action |
| `src/validator.rs` | changed | duplicate-priority and shadowed-duplicate-transition errors; non-fatal warning channel (`validator::module_warnings`, surfaced by `rfs validate` and codegen) for unreachable states, dead events, unused variables, impossible guards (decisions D1-D7, `DESIGN.md` section 20) |
| `src/printer.rs` | changed | prints version headers / on_error / priority to keep parse/validate/print round-trip parity (examples 39-40) |
| `src/codegen_fsm.rs` + Rust backend | changed | priority-ordered transition arms per state (effective priority, default 0), on_error fallback replacing `Err(StepError::UnexpectedEvent)`, `SPEC_VERSION` constant; generated negative unit tests adapt to on_error modules |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` sections 19-20 | changed | Phase 1 renumbered; Phase 2 decisions D1-D10 with comparison tables |
| `KB.md` | changed | state table, section 3.1/3.5 syntax (version header, on_error, priority), section 7 limitations rewritten - most Phase 0/1 items now resolved |
| `CODEGEN.md` | changed | C backend rows, testing conventions, invariants |
| `src/backends/C.md` | new | C backend architecture: module split, type mapping, value representation, MC wire format |
| `BUILD.md`, `START.md` | changed | counts/snapshot updated to Phase 2 state (125 main / 492 generated tests, 40 valid + 11 invalid examples) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/valid/39_endpoint_protocol.rfs` | new | realistic endpoint protocol: version header, payloads, binding guards, priorities, on_error - pins the new syntax and serves as the integration-test target |
| `examples/valid/40_on_error_recovery.rfs` | new | on_error recovery behavior example |
| 5 new files in `examples/invalid/` | new | duplicate priority, shadowed duplicate transition, on_error without next, unquoted version header, duplicate set - each must fail parse or validation |

## Changed functionality

- transition matching: explicit priorities now order overlapping arms (effective priority, default 0) instead of being parsed-but-ignored
- unexpected events in modules with `on_error`: the fallback block executes instead of returning `Err(StepError::UnexpectedEvent)`
- validation reports: non-fatal warning channel added alongside errors

## Removed functionality

- None in Phase 2 itself (the legacy CLI/bin removals happened in the 2026-10-14 preparation cleanup, recorded in `PHASE_1_STATUS.md`).

## New functionality

- language: module version headers; `on_error` fallback blocks; semantic priorities - all parse/validate/print round-tripped
- validator: duplicate-priority + shadowed-duplicate errors; warnings for unreachable states / dead events / unused variables / impossible guards
- Rust codegen: priority-ordered arms, on_error fallbacks, `SPEC_VERSION`, adaptive negative tests
- C backend: production/test/mc with parity to the Rust MC wire format (D8 fixed-width mapping)
- integration test: realistic endpoint example compiled with rustc and driven over an mpsc channel (determinism + on_error behavior)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| +13 validator unit tests | new | priority/ambiguity/duplicate rules, all warning classes |
| +4 parser unit tests | new | version header; on_error parse + round-trip; missing-next rejection |
| +7 codegen tests | new | Rust priority ordering / SPEC_VERSION / on_error markers; C registry / per-command generation over all 40 valid examples; C-vs-Rust semantic parity |
| `tests/integration_example_test.rs` (+2) | new | generate example 39 -> rustc compile -> driver reading events from an mpsc channel; asserts the full handshake output sequence, determinism across two runs, on_error landing |
| +4 C helper unit tests | new | C backend helpers (mangling/type mapping) |
| `tests/examples_test.rs::invalid_examples_fail_to_parse` | changed | accepts parse-OR-validation failure - validator errors are first-class now |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` | OK - 125/125 (was 95: +13 validator, +4 parser, +4 C helpers, +7 codegen, +2 integration... sums to +30 including example-suite growth) |
| `cargo generate-code-tests` | OK - aggregate crate 164 modules (was 162) |
| `cargo test-generated-code` | OK - 492/492 (was 486) |
| `cargo clippy --all-targets` | OK - 0 errors |
| `cargo fmt --check` | OK |
| `rfs validate --input rfs examples/valid` | OK - exit 0, all 40 files |
| C output with `gcc -Wall` | OK - production run reproduces Rust behavior (syn -> data_ok -> rst); `c test` passes; `c mc --bound 6 --runs 10` completes with the same summary format |

## Found problems/issues/mismatches

- None recorded for Phase 2 in the former ROADMAP.md current status.

## Other important findings/information

- Known-good baseline (2026-10-15): 125/125 main tests, 492/492 generated tests, clippy without errors (two pre-existing warnings: `from_str` naming in lib, `&PathBuf` arg in `tests/codegen_test.rs`), fmt clean.
- Design decisions D1-D10 are documented with comparison tables in `DESIGN.md` section 20.
