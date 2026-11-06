# Phase 11, Task 9 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_9.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | pure renames - no new code |

## Changed/removed source files

| Name | Operation | Description |
|---|---|
| `tests/library_comparison.rs` | removed (moved) | git mv to `tests/parser_comparison_test.rs`, contents byte-identical |
| `tests/examples_test.rs` | removed (moved) | git mv to `tests/example_corpus_test.rs`, contents byte-identical |
| `tests/integration_example_test.rs` | removed (moved) | git mv to `tests/endpoint_protocol_test.rs`, contents byte-identical |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `docs/dev/DECISIONS.md`, `docs/llm/TASK_MAP.md` | changed | GATE-FORCED (correction): backticked file-path refs to the old suite names that dev_docs_test resolves - `tests/examples_test.rs` -> `tests/example_corpus_test.rs` (DECISIONS D-P7-11 row) + the three Test-suites rows in TASK_MAP |
| `BUILD.md`, `KB.md`, `START.md`, `docs/dev/{parser,validator}.md`, `docs/llm/{parser,validator,CHANGE_IMPACT}.md` | changed | command-form (`--test <name>`) + bare suite-name mentions updated per the standing user direction (15 scripted refs + START.md's source-tree test list + two CHANGE_IMPACT backticked names) - task 10's sweep now only verifies zero leftovers |

## Other file changes

| Name | Operation | Description |
|---|---|
| (none) | - | no Cargo.toml / manifest edits needed: integration tests are auto-discovered by file name, and the three renamed files were never in tests/dev_docs_manifest.txt (verified pre-move) |

## Changed functionality

None - the three suites run byte-identical test logic under their new names; cargo discovers one binary per file, so a green run IS the discovery proof.

## Removed functionality

None (only the old file names).

## New functionality

- Suite names now match the responsibility categories the test inventory uses: `parser_comparison_test` (nom vs chumsky smoke), `example_corpus_test` (file-driven valid/invalid corpus), `endpoint_protocol_test` (realistic endpoint protocol -> production Rust + mpsc driver).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `tests/parser_comparison_test.rs`, `tests/example_corpus_test.rs`, `tests/endpoint_protocol_test.rs` | renamed | the three suites from their old names; zero content changes |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline; all three renamed binaries appear in the run output under their new names (`Running tests/{parser_comparison_test,example_corpus_test,endpoint_protocol_test}.rs`) |
| `cargo test --test dev_docs_test` (9) | OK - green after the four gate-forced doc refs were repointed |

## Found problems/issues/mismatches

- The task file's "source-only" assumption was wrong: dev_docs_test resolves backticked test-file paths in docs/dev + docs/llm, so `DECISIONS.md`'s `tests/examples_test.rs` ref broke the per-task green gate immediately. Pulled the doc re-sync into this commit (correction recorded); task 10 is now a verification-only sweep for these names.

## Other important findings/information

- None of the three files carried a //! Docs: line and none was manifest-listed - the rename is invisible to the manifest/pointer checks, only to the doc->code reference resolution.
- No .rs file referenced the old suite names (grep-verified pre-move), so no code edits were needed at all.
