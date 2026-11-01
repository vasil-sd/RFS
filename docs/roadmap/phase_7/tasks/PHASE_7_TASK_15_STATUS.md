# Phase 7, Task 15 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_15.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/main.rs` | changed | the usage-text `info format` paragraph (written in task 5) was stale - it said "Currently: spec-rfs ... JSON-schema outputs land with the JSON format entries" although tasks 6-14 registered nine more formats; replaced with a one-line summary of the current 10-format registry (task correction, see `PHASE_7_TASK_15.md`) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `training/frontend_cli/rfs_training.jsonl` | changed | +3 SFT entries (two-message format, real tool output captured in this session): (1) `rfs diagram --renderer internal\|external` - in-process default for all three formats, zero-external-deps proof with every tool env var unresolvable, strict external semantics, internal-mode `--print-script`; (2) `rfs info format` basics - `--list` registry of 10 formats, per-format `--output` md/plantuml/svg/schema, embedded-model sync rationale, honest note on the installed plantuml's missing EBNF support; (3) CI/CD use of the emitted draft-07 JSON schemas - real end-to-end validation of an IR v1 document and an MC report with python jsonschema |
| `all_rfs_training.jsonl` | changed | regenerated aggregate: **264** lines (was 261; +3) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/*/snippets/*` | regenerated | consistency step only - no new `.rfs` snippets (D-P7-11/D-P7-12); counts unchanged: 138 valid + 128 invalid |

## Changed functionality

- None observable: the `src/main.rs` change is help text only (the usage block printed on unknown args), no behavior change.

## Removed functionality

None.

## New functionality

- SFT coverage of the Phase 7 CLI surface in `training/frontend_cli/`:
  - `rfs diagram --renderer internal|external` incl. the zero-external-deps rationale (in-process default for dot/puml/mmd; external tools now optional; availability-gated parity checks)
  - `rfs info format` usage (`--list`, per-format `--output` choices, embedded single source of truth + sync tests)
  - CI/CD use of the emitted draft-07 JSON schemas (validating IR v1 documents / MC reports against them with any draft-07 validator)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no test-code changes; the existing training tests verify the new data (`committed_all_rfs_training_file_is_fresh`, `aggregate_training_sets_matches_category_files`, ...) |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (root) | OK - **526 pass / 0 fail** (unchanged; no new tests this task) |
| `cargo clippy --all-targets` | OK - zero errors; 46 pre-existing warnings, none in `src/main.rs` |
| `cargo fmt --check` | OK |
| `cargo extract-training-snippets` | OK - 138 valid + 128 invalid snippets (unchanged) |
| `cargo build-training` | OK - aggregate **264** lines |
| `cargo test-training` (5) | OK - 5 pass / 0 fail |

(`--workspace` / `--features smt` not re-run: only the root package's help text and training data changed; the root suite covers both. Task 16 runs the full matrix.)

## Found problems/issues/mismatches

- The usage-text `info format` paragraph in `src/main.rs` was stale since task 5 ("Currently: spec-rfs ... JSON-schema outputs land with the JSON format entries") although tasks 6-14 registered nine more formats - fixed during this task (see Task corrections in `PHASE_7_TASK_15.md`). No test pinned the line.
- The apt plantuml in this environment (1.2020.02) predates PlantUML's EBNF diagram support: it answers "No diagram found" (exit 100) even for a minimal `@startebnf` file, so the Phase 7 exit criterion "`spec-rfs --output plantuml` ... accepted by plantuml when installed" cannot be verified with the installed binary. The test suite pins the EBNF structure instead (`@startebnf`/`@endebnf` wrapper, rule-line termination, quoted terminals). Task 16 should consider noting in TOOLCHAIN.md that real-tool acceptance of `spec-rfs --output plantuml` requires a newer PlantUML release.
- Transient environment issue (not an rfs bug): the first `--renderer external` mmdc run failed with "Could not find chrome-headless-shell" while the sandbox was still provisioning the puppeteer cache; all subsequent runs succeed (exit 0, non-empty SVG).

## Other important findings/information

- Counts for task 16's BUILD.md re-pin: aggregate **264** lines (was 261); snippets unchanged at **138 valid + 128 invalid**; `cargo test-training` = 5 pass.
- The three new entries quote verbatim tool output captured in this session (rfs 1.0.0, spec `examples/valid/04_tcp_handshake_like.rfs`): internal SVG sizes dot 3766 / puml 4391 / mmd 6366 bytes with all tools unresolvable; external dot 3335 / puml 7077 / mmdc 189407 bytes; `mc-ref` on spec 04 exits 1 (the known pinned deadlock) and its JSON report validates against the emitted mc-report schema.
