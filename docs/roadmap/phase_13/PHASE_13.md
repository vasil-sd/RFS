# Phase 13: CLI source refactoring - unified command plugins

## Goal
Unify how the `rfs` CLI processes its top-level commands. Before this phase the eight
subcommands were handled inconsistently: six of them (`parse`, `validate`, `codegen`,
`info`, `test`, `diagram`) carried their full argument-parsing + dispatch bodies inline
in `src/main.rs`, while two (`mc-ref`, `gen-spec`) already delegated to plugin modules.
This phase gives EVERY top-level command its own plugin module under `src/cli/commands/`
with a single `run_cli(args) -> ExitCode` entry point, reduces `main.rs` to a pure
name-dispatcher over a command registry (mirroring the existing diagram-format and
codegen-backend plugin mechanisms), extracts the common CLI-argument handling that every
command duplicated into `src/cli/args.rs`, and splits the 756-line `src/cli/frontend.rs`
- which fused two unrelated engines - into `utils.rs` (format + conversion/file
utilities) and a `commands/validate.rs` plugin (all validation). The refactor is
behaviour-preserving: identical argv parsing, stdout/stderr and exit codes for all eight
commands.

## Rationale
The inconsistency is a maintenance hazard: adding or changing a command meant editing one
of two different patterns (inline-in-main vs delegate-to-module), and the shared argument
boilerplate (usage text, `--input`/`--output` format takers, the "unknown option /
unexpected argument / missing required" error conventions) was copy-pasted across six
functions in main.rs. A single plugin-per-command registry makes the pattern uniform (one
module + one registry line per command), centralises the shared boilerplate so it changes
in one place, and separates the two engines that frontend.rs fused - conversion (used by
parse/codegen/info/test/diagram) from validation (used only by validate) - so each can
evolve independently. This is the CLI half of the Phase 10/11 source-refactoring effort:
structural cleanup with no functionality change.

## Scope
1. A `CommandPlugin` registry in `src/cli/commands/mod.rs` (`ALL_COMMANDS` + `find()`),
   mirroring `DiagramPlugin` / `Backend`.
2. One plugin module per top-level command, each exposing `run_cli(args) -> ExitCode`:
   new modules for the three that had no module of their own (`parse`, `validate`,
   `codegen`), plus the argument-parsing halves of `info`, `test` and `diagram` moved into
   their existing modules as `run_cli`; `mc-ref` and `gen-spec` already conform.
3. `src/main.rs` reduced to a pure dispatcher: resolve the command name via `find()`, call
   its `run_cli`; an unknown or missing subcommand prints usage and exits 1.
4. `src/cli/args.rs`: the shared argument handling extracted from main.rs - `usage()` (the
   global help text), the `take_format` / `take_value` flag takers, and the
   `unknown_option` / `unexpected_arg` / `missing_required` error-convention helpers.
5. Split `src/cli/frontend.rs`: rename to `src/cli/utils.rs` keeping only `Format` + the
   conversion/file utilities (`normalize_abs`, `collect_files_with_ext`,
   `plan_conversion`, `output_path_for`, `convert_file_to_ir`, `render_ir`,
   `run_conversion`); move ALL validation functionality into the new
   `src/cli/commands/validate.rs` plugin (the validation fns + `cmd_validate`'s argument
   parsing as `run_cli`). Core semantic validation stays untouched in
   `src/lang/validator.rs`.
6. Re-sync every doc that references a moved or renamed path (the rename blast radius):
   the living docs under docs/dev + docs/llm, KB.md, the dev_docs manifest, and the two
   structure inventories.

## Deliverables
- `src/main.rs` as a ~30-line pure dispatcher.
- `src/cli/commands/mod.rs` with the `CommandPlugin` struct, the `ALL_COMMANDS` registry
  and `find()`.
- New plugin modules `src/cli/commands/{parse,validate,codegen}.rs`; the argument-parsing
  of `info`, `test`, `diagram` moved into their existing modules as `run_cli`.
- `src/cli/args.rs` (shared argument handling).
- `src/cli/utils.rs` (renamed from frontend.rs, conversion half) and
  `src/cli/commands/validate.rs` (validation half + run_cli).
- Re-synced docs: KB.md, docs/dev/*, docs/llm/* (incl. SRC_STRUCTURE / TEST_STRUCTURE /
  TASK_MAP), tests/dev_docs_manifest.txt.

## Exit criteria
- [ ] `main.rs` contains no `cmd_*` bodies - only the dispatcher over `ALL_COMMANDS`.
- [ ] All eight commands are registered plugins with a `run_cli` entry point.
- [ ] `frontend.rs` no longer exists; conversion lives in `utils.rs`, validation in
      `commands/validate.rs`; `src/lang/validator.rs` is byte-for-byte untouched.
- [ ] Common argument handling (usage, format/value takers, error conventions) lives in
      `args.rs`, not duplicated across commands.
- [ ] Behaviour preserved: identical argv parsing, stdout/stderr and exit codes for all
      eight commands (spot-checked).
- [ ] Full verification green: `cargo test` 535/535 (exact baseline), `cargo fmt --check`
      clean, `cargo clippy` at the pre-phase baseline (no new warnings).
- [ ] Docs re-synced: dev_docs sync gate green; no stale reference to any moved path.

## Non-goals
- No CLI behaviour change: no new or removed commands, no flag changes, no output or
  exit-code changes. This is a structural refactor only.
- No change to core semantic validation (`src/lang/validator.rs`) - it is consumed by the
  validate plugin, not moved.
- No change to the diagram-format or codegen-backend plugin mechanisms (they are the
  models this phase copies, not targets).
- No per-task `PHASE_13_TASK_M.md` files: this phase ran on a single consolidated working
  file (`new_cli_processing.md`, carrying Analysis + Proposal + Implementation details)
  that was deleted at close-out; the plan below records its task structure.
