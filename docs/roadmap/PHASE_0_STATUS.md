# Phase 0 - Implementation status

> Reconstructed on 2026-10-16 from the "Current status" section of the former
> `ROADMAP.md` during the docs split (template 2.3); per-file phase attributions
> follow that section.

## Added source files

| Name | Description |
|---|---|
| `Cargo.toml` | Cargo project `rfs` v0.1.0, edition 2021; deps `nom`, `serde`/`serde_json`; dev-dep `chumsky` 0.13 retained only for the parser-library comparison smoke test |
| `src/parser.rs` | `nom` 8 recursive-descent parser for the RFS v0 subset - the Phase 0 core deliverable |
| `src/ir.rs` | versioned IR v0 data structures with serde JSON (de)serialization |
| `src/error.rs` | `ParseError`/`Span` types so parser errors carry useful spans/messages |
| `src/main.rs` | CLI entry point for parsing files/directories (reworked in Phase 1 into `rfs parse`/`validate`) |
| `src/printer.rs` | IR -> formatted RFS output, needed for parse round-trips |
| `src/bin/build_training.rs`, `src/bin/extract_training_snippets.rs` | training pipeline: aggregate category JSONL into `all_rfs_training.jsonl`; extract + classify RFS snippets (both cargo commands already listed in the Phase 0 status) |
| `tests/` initial suites | parser unit tests, file-driven valid/invalid example suite, IR JSON round-trip, nom/chumsky smoke comparison, training-set structure tests |
| `examples/valid/*.rfs` (30) | the 20-30 valid RFS v0 examples required by the phase scope; file-driven suite needs no test-code change for new ones |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| hand-rolled lexer module | removed | dropped after the parser-library evaluation in favor of `nom` 8; `chumsky` kept only as a dev-dep for the comparison smoke test |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | new | RFS v0 design baseline: architecture, core semantics, EBNF grammar, IR schema, parser/codegen/MC/PSL plans |
| `ROADMAP.md` (now `docs/roadmap/`) | new | development plan Phase 0 -> production toolchain; split into this directory on 2026-10-16 |
| `KB.md` | new | knowledge base for future LLM-assisted work: decisions, syntax reference, source map, workflow experience |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/<category>/rfs_training.jsonl` + `snippets/` | new | SFT training data organized per category; snippets auto-extracted and tested by the parser suite |
| `examples/valid/tla_adapted/`, `training/tla_adaptation/` | new | TLA-inspired protocol examples + adaptation reasoning |

## Changed functionality

- None (first phase of the project).

## Removed functionality

- None (first phase of the project).

## New functionality

- RFS v0 parsing: modules, enums, initial state, events/outputs with grouped variants, finite variables (`bool`, `bit<N>`, enum), transitions with patterns/guards/actions (`next`/`emit`/`set`), pure expressions
- minimal properties and assumptions: boolean formulas, `next`, `always`, `bounded_eventually[N]`, `input_is(...)`, `output_has(...)`
- versioned IR with JSON round-trip
- file-driven example test suite (new `.rfs` works without test-code changes)
- parser diagnostics with useful spans/messages for basic malformed input

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| parser unit tests (`cargo test --lib`) | new | grammar coverage of the v0 subset |
| `tests/examples_test.rs` (file-driven) | new | all valid examples parse+validate+round-trip; invalid examples rejected |
| IR JSON round-trip tests | new | serialize/deserialize stability of IR v0 |
| nom/chumsky smoke comparison (`tests/library_comparison.rs`) | new | parser-library evaluation evidence behind the `nom` choice |
| training-set structure/snippet/aggregation tests | new | keep training data consistent with the tree |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (full main suite at phase completion) | OK |
| `cargo run -- validate --input rfs examples/valid` | OK - all valid Phase 0 examples parse and validate |
| invalid example rejection (`cargo test --test examples_test`) | OK |

## Found problems/issues/mismatches

- The hand-rolled lexer did not survive the parser-library evaluation - replaced by `nom` 8; `chumsky` 0.13 retained as a dev-dep for comparison only.

## Other important findings/information

- Span support is at the "useful for basic malformed input" level (Phase 0 exit criterion); span-rich IDE diagnostics remain a later-phase item (`KB.md` section 7).
