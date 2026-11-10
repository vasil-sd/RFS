# Phase 13 - Implementation status

> Written once when all exit criteria of `PHASE_13.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/cli/args.rs` | Shared CLI argument handling extracted from main.rs: `usage()` (the global help text for all eight subcommands), `take_format` / `take_value` flag takers, and the `unknown_option` / `unexpected_arg` / `missing_required` error-convention helpers. Every plugin's `run_cli` calls these instead of re-implementing them. |
| `src/cli/commands/parse.rs` | The `rfs parse` plugin: `cmd_parse`'s argument parsing moved here as `run_cli`, calling the conversion engine in `utils.rs`. |
| `src/cli/commands/codegen.rs` | The `rfs codegen` plugin: `cmd_codegen`'s argument parsing moved here as `run_cli`, a thin wrapper over the `crate::codegen` backend registry. |
| `src/cli/utils.rs` | The conversion half of the old frontend.rs (renamed/split): `Format` + `normalize_abs`, `collect_files_with_ext`, `plan_conversion`, `output_path_for`, `convert_file_to_ir`, `render_ir`, `run_conversion`. Consumed by parse/codegen/info/test/diagram. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/cli/frontend.rs` | removed (split) | The 756-line two-engine file no longer exists. Its conversion half became `utils.rs`; its validation half moved into the new `commands/validate.rs` plugin. Git records it as a rename to `commands/validate.rs` (R059). |
| `src/cli/commands/validate.rs` | new (from frontend.rs) | The validation half of frontend.rs (`FailedFile`, `ValidateResult`, `validate_one[_with_warnings]`, `StructuredError`, `validate_one_structured`, `validate_json_doc`, `run_validate_structured`, `render_validate_json`, `run_validate`, `err_file_path`, `write_err_file`) PLUS `cmd_validate`'s argument parsing as `run_cli`. Engine + entry point together, like `mc_ref.rs`. |
| `src/main.rs` | changed | Reduced from ~965 lines to a ~30-line pure dispatcher: read argv, `commands::find(name)`, call `(cmd.run)(rest)`; unknown/missing subcommand prints usage and exits 1. All six `cmd_*` bodies + the shared argument helpers are gone (moved to plugins / args.rs). |
| `src/cli/commands/mod.rs` | changed | Now carries the `CommandPlugin { name, summary, run }` struct, the `ALL_COMMANDS: &[CommandPlugin]` registry (all eight commands) and `find(name)` - the command plugin mechanism mirroring `DiagramPlugin` / `Backend`. |
| `src/cli/commands/info.rs` | changed | `cmd_info`'s argument parsing + its `run_info_format` helper moved in as `run_cli`; self-references re-pointed to local. |
| `src/cli/commands/test.rs` | changed | `cmd_test`'s argument parsing moved in as `run_cli`. |
| `src/cli/commands/diagram/mod.rs` | changed | `cmd_diagram`'s argument parsing + its `tool_override_mut` helper moved in as `run_cli`. |
| `src/cli/commands/gen_spec.rs` | changed | Import re-pointed `crate::cli::frontend` -> `crate::cli::utils` (conversion consumer). |
| `src/cli/commands/mc_ref.rs` | changed | Import re-pointed `crate::cli::frontend` -> `crate::cli::utils` (conversion consumer). |
| `src/lib.rs` | changed | The crate-root re-export of `cli::frontend::{...}` split into `cli::utils::{...}` (conversion items) + `cli::commands::validate::{...}` (validation items); the public API surface is unchanged. |
| `src/cli/mod.rs` | changed | Module declarations updated: `frontend` -> `utils`, added `args`, and the new command submodules. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `KB.md` | changed | Section 4.2 file map: the `frontend.rs` row split into `utils.rs` + `commands/validate.rs` + `args.rs`; the main.rs row rewritten as a pure dispatcher; inline `frontend.rs` references re-pointed. |
| `docs/dev/cli_commands.md` | changed | The command-dispatch documentation re-synced to the plugin registry + args.rs layout (the largest doc delta, ~108 lines). |
| `docs/dev/{ALGORITHMS,ARCHITECTURE,CODE_MAP,DECISIONS,ir_json_rw,parser,validator}.md` | changed | Backticked source-path references to moved/renamed items re-pointed (frontend.rs -> utils.rs / commands/validate.rs; main.rs cmd_* -> plugin run_cli). |
| `docs/llm/SRC_STRUCTURE.md` | changed | Inventory rows re-keyed to the new paths (args.rs, utils.rs, commands/{parse,validate,codegen}.rs, the per-plugin run_cli entries); the llm-budget header raised 1710 -> 1750 for the net-new required rows. |
| `docs/llm/TEST_STRUCTURE.md` | changed | Test-inventory rows re-keyed to the renamed test file + moved inline test modules (the largest doc delta, ~292 lines). |
| `docs/llm/{TASK_MAP,CHANGE_IMPACT,cli_commands,ir_json_rw}.md` | changed | Source-path references and file-map entries re-pointed across the rename. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/dev_docs_manifest.txt` | changed | The code->doc pointer manifest re-synced to the new file set (frontend.rs removed; args.rs, utils.rs, commands/{parse,validate,codegen}.rs added). |

## Changed functionality

None observable. The CLI's external contract - every command's argv parsing, stdout/
stderr output and exit codes - is byte-for-byte unchanged. What changed is internal
structure only:
- Command dispatch moved from six inline `cmd_*` bodies in main.rs to a uniform
  plugin-per-command registry (`ALL_COMMANDS` + `find`).
- The shared argument boilerplate moved from being duplicated across those six bodies to
  the single `args.rs` module.
- The two engines fused in frontend.rs were separated into `utils.rs` (conversion) and
  `commands/validate.rs` (validation).

## Removed functionality

None. No command, flag, output or exit code was dropped.

## New functionality

- The `CommandPlugin` registry mechanism (`src/cli/commands/mod.rs`): a uniform
  plugin-per-top-level-command structure with `ALL_COMMANDS` + `find()`, mirroring the
  existing diagram-format and codegen-backend registries. Adding a command is now one
  module + one registry line.
- `src/cli/args.rs`: centralised shared argument handling (usage text, format/value
  takers, error conventions) - previously copy-pasted across six main.rs bodies.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/frontend_test.rs` -> `tests/utils_test.rs` | renamed + changed | The conversion-engine test suite renamed to match the module it now tests (utils.rs); its imports re-pointed from `cli::frontend` to `cli::utils` / `cli::commands::validate`. 114 tests, all pass. |
| `tests/info_format_test.rs` | changed | Import re-pointed from `cli::frontend` to the split modules (validation fns -> `cli::commands::validate`, Format -> `cli::utils`). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root rfs, 535) | OK - exact baseline, unchanged. |
| `cargo fmt --check` | OK - clean. |
| `cargo clippy --all-targets` | OK - at the pre-phase baseline; zero new warnings (all lib warnings are in pre-existing files, none in the new modules). |
| Exit-code spot checks (no-arg/bogus=1, --help=0, validate/info/diagram/codegen=0) | OK - behaviour preserved. |

## Found problems/issues/mismatches

- The dev_docs sync gate (`tests/dev_docs_test.rs::sync_gate_passes_on_current_tree`)
  FAILED on the first full-suite run after the refactor: `docs/llm/SRC_STRUCTURE.md` had
  grown to 1716 lines, exceeding its declared `<!-- llm-budget: 1710 -->`. The cause was
  legitimate - the rename added 6 net new required inventory rows (the args.rs helpers and
  per-plugin entries), each a mandatory one-row-per-function entry. Fixed by raising the
  budget to 1750 (a round ceiling with headroom, matching every other LLM doc's
  convention) rather than dropping required coverage. After the fix the sync gate is green
  (9/9).

## Other important findings/information

- The refactor is a pure MOVE of code: no logic was rewritten, only relocated and re-
  imported. This is why clippy stayed exactly at baseline - every pre-existing warning
  simply changed file location, and none landed in the new modules.
- Git's rename detection records `frontend.rs -> commands/validate.rs` (R059, the larger
  validation survivor) with `utils.rs` as a separate new file; both are valid views of the
  same split operation.
- `src/lang/validator.rs` (core semantic validation) is byte-for-byte untouched, per the
  planning decision - the validate plugin consumes it, it was not moved.
- Phase 13 ran on a consolidated working file (`new_cli_processing.md`) rather than
  per-task files; that file was deleted at close-out and this status + the plan are the
  durable record. **Phase 13 is COMPLETE.** All exit criteria of `PHASE_13.md` are met.
