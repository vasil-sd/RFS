# Phase 17, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (documentation-only task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/dev/cli_commands.md` | changed | four stale spots rewritten: Overview now describes the plugin-registry architecture (ALL_COMMANDS + find(), every command a run_cli plugin, main.rs a pure dispatcher); the Dispatch main() paragraph matches the shipped 30-line dispatcher; the Library API surface lists the current seven top-level modules with their verified submodules instead of the pre-Phase 11 flat names; the mc-ref section describes the actual model (McMode, the 11-field McRefOptions, D-P16-4 mode dispatch to ref_mc::run/run_fuzz/run_smt, per-mode exit codes, not-applicable reduced to --threads/--fast) and links the three Phase 16 engine fragments |

## Other file changes

None.

## Changed functionality

None - documentation only.

## Removed functionality

None.

## New functionality

- The developer CLI doc now matches the shipped architecture end to end (no
  pre-Phase 13/16 prose remains).

## Tests add/change/remove

None - existing gates re-run.

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- None recorded. The task file's hint proved right: the actual options model
  differs from the plan's shorthand - `exit_code_for` is EXACT-mode-only
  (takes `ExactStatus`); fuzz and smt-bounded exit codes are derived inline in
  `run_mc_ref`'s dispatch. The doc describes it that way.

## Other important findings/information
- `FuzzConfig` lives in `src/mc/ref_mc.rs` (imported into mc_ref.rs), not in
  the command module - referenced accordingly so the sync gate resolves it.
