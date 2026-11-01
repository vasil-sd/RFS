# Phase 1 - Implementation status

> Source: "Current Phase 1 implementation status" of the former `ROADMAP.md`,
> including the two 2026-10-14 preparation/training bullets listed there,
> reorganized on 2026-10-16 into template 2.3 format during the docs split.

## Added source files

| Name | Description |
|---|---|
| `src/validator.rs` | semantic validation + expression/formula typing; precondition for code generation |
| `src/codegen.rs` | Backend plugin API, compile-time `BACKENDS` registry, `rfs codegen` CLI parsing, help rendering |
| `src/codegen_fsm.rs`, `src/codegen_test.rs`, `src/codegen_mc.rs` | language-agnostic analysis shared by all backends (production FSM / unit-test planning / MC wire format) |
| `src/backends/rust*.rs` | Rust backend: production, test, mc commands; architecture in `src/backends/RUST.md` |
| `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs` | generate + run the aggregate generated-code unit-test crate (`target/rfs_codegen_tests`) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/frontend.rs` | changed | new CLI logic: `rfs parse` (all four rfs/json combinations, extension replacement never stacking, `--target-dir` structure preservation, normalized-absolute-path collision guard) and `rfs validate` (file or directory tree; `<name>.rfs-validation.err` next to each failing source) |
| `src/main.rs` | changed | argument parsing/exit codes: 0 on success, 1 for parse, planning, collision, or validation failures |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | new | backend plugin API reference, registry, how to add a target language, testing conventions (documents the plugin architecture introduced in this phase) |
| `src/backends/RUST.md` | new | Rust backend internals: module split, type mapping, value representation, MC wire format |
| `KB.md` | changed | codegen/MC decisions, CLI behavior sections (2.7, 4.5, 13) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/frontend_cli/rfs_training.jsonl` | new | new category: parse/validate/codegen CLI behavior, format round-trips, error reporting, exit codes (2026-10-14 actualization) |
| `training/codegen_rust/rfs_training.jsonl` | changed | now covers the backend plugin architecture, the `rfs codegen` CLI, and adding new target languages |
| `all_rfs_training.jsonl` | changed | regenerated aggregate: 11 categories / 219 examples after actualization |

## Changed functionality

- frontend CLI: Phase 0 positional parse mode replaced by explicit `rfs parse`/`rfs validate` subcommands with a strict exit-code contract (the removed mode itself is listed under Removed functionality)

## Removed functionality

- Phase 0 positional parser CLI mode (`cargo run -- <paths> [--json]`) - superseded by `rfs parse`/`rfs validate` (2026-10-14 preparation cleanup, recorded here because the former ROADMAP listed it under Phase 1)
- `validate_rfs` and `rfs_codegen` cargo bins - superseded by `rfs validate` and `rfs codegen rust ...`; auxiliary bins/tests now call backend functions directly (e.g. `rfs::backends::rust_test::generate_aggregate_test_crate`)
- pre-plugin monolithic codegen API in `src/codegen.rs` - replaced by the Backend plugin registry

## New functionality

- semantic validation and expression/formula typing for code generation
- plugin-based codegen: three commands per backend (production/test/mc), compile-time registry, `rfs codegen` CLI
  - generated production FSM with trace recording, assumption checks, property-check helpers
  - generated unit tests per example/snippet
  - bounded model checker: production core + runtime (random bounded runs, forced assumption violations, unexpected-event injection)
- `rfs codegen --list | --help | <backend> --help` CLI surface with documented exit codes
- aggregate generated-code test pipeline (`cargo generate-code-tests`, `cargo test-generated-code`)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/frontend_test.rs` (41 tests) | new | format parsing, strict extension handling, output placement/structure, collision guard, err-file generation, positive/negative validation, end-to-end binary exit-code checks; round-trips rfs->json->rfs and json->rfs->json |
| `tests/codegen_test.rs` (12 tests) | new | registry, per-command generation for all valid examples/training snippets, option sensitivity, every documented CLI parse case |
| generated aggregate crate (`target/rfs_codegen_tests`) | new | compile + run unit tests for all valid examples and valid training snippets (486 at Phase 1 time; 492 after Phase 2) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (main suite at phase completion) | OK |
| `cargo generate-code-tests` + `cargo test-generated-code` | OK |
| `cargo run -- validate --input rfs examples/valid` | OK |

## Found problems/issues/mismatches

- liveness formulas containing `bounded_eventually` are skipped with warnings in the Phase 1 translated subset - limitation carried into Phase 3.

## Other important findings/information

- SFT training sets actualized on 2026-10-14 to the current state: 11 categories / 219 examples; extracted snippets 120 valid / 125 invalid; regenerate with `cargo extract-training-snippets` + `cargo build-training`.
- CLI behavior rules are documented in `KB.md` section 4.5; codegen plugin API in `CODEGEN.md`.
