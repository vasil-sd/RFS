# Phase 13 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

**The inconsistency (from the Analysis section of the working file).** The eight top-level
commands split into two handling patterns:
- Inline in `src/main.rs` (a ~965-line file): `parse`, `validate`, `codegen`, `info`,
  `test`, `diagram` each had a `cmd_<name>(args) -> ExitCode` body doing its own argument
  parsing and then calling into the engine. main.rs also held the shared boilerplate -
  `usage()` (the global help text), `take_format` (the `--input`/`--output` taker), and
  per-command copies of the "unknown option / unexpected extra argument / missing required"
  error conventions.
- Already plugin-shaped: `mc-ref` and `gen-spec` delegated to `src/cli/commands/mc_ref.rs`
  and `gen_spec.rs`, each exposing a `run_cli`.

So six commands followed one pattern and two followed another, and the shared argument
handling was duplicated across the six inline bodies. The diagram command already used a
plugin registry (`DiagramPlugin` + `ALL_DIAGRAMS`) and codegen a backend registry
(`Backend` + `BACKENDS`) - the two mechanisms this phase generalises to top-level commands.

**`frontend.rs` is two engines in one file (756 lines).** It mixed:
- Conversion/format utilities used by parse, codegen, info, test and diagram: `Format`,
  `normalize_abs`, `collect_files_with_ext`, `plan_conversion`, `output_path_for`,
  `convert_file_to_ir`, `render_ir`, `run_conversion`.
- Validation used only by validate: `FailedFile`, `ValidateResult`, `validate_one`,
  `validate_one_with_warnings`, `StructuredError`, `validate_one_structured`,
  `validate_json_doc`, `run_validate_structured`, `render_validate_json`, `run_validate`,
  `err_file_path`, `write_err_file`.

**Rename blast radius.** `crate::cli::frontend` was imported by the five conversion
consumers (gen_spec, test, mc_ref, diagram, info), by main.rs (both halves), and re-exported
at the crate root in lib.rs. Tests referenced it too (`tests/frontend_test.rs`,
`tests/info_format_test.rs`). Every one of those sites had to be re-pointed at the split.

**Planning decisions taken now:**
- The plugin mechanism is a `CommandPlugin { name, summary, run }` struct + an
  `ALL_COMMANDS: &[CommandPlugin]` registry + `find(name)`, in `src/cli/commands/mod.rs`.
  `main.rs` becomes: read argv, `find(name)`, call `(cmd.run)(rest)`; unknown/missing name
  prints usage and exits 1. This mirrors `DiagramPlugin`/`Backend` exactly.
- Common argument handling goes to a NEW `src/cli/args.rs`: `usage()`, `take_format`,
  `take_value` (a generic `--flag VALUE` taker), and the three error-convention helpers
  (`unknown_option`, `unexpected_arg`, `missing_required`). Each plugin's `run_cli` keeps
  its command-specific flag arms but calls these for the shared parts.
- The frontend split: `frontend.rs` is RENAMED to `utils.rs` (conversion half) and the
  validation half moves into the NEW `commands/validate.rs` plugin, which also carries
  `cmd_validate`'s argument parsing as its `run_cli`. Core semantic validation stays in
  `src/lang/validator.rs`, untouched.
- Behaviour-preservation contract: identical argv parsing, stdout/stderr and exit codes for
  all eight commands. The refactor moves code; it changes no observable behaviour.

**Token economy / workflow.** This phase ran on a single consolidated working file,
`new_cli_processing.md` (repo root), carrying the Analysis, Proposal and Implementation
details sections - NOT the per-task `PHASE_13_TASK_M.md` files the standard protocol uses.
That file was deleted at close-out; this plan + the status file are the durable record.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Analysis | `new_cli_processing.md` (new, `## Analysis`) | Survey the CLI surface: the two handling patterns, the duplicated argument boilerplate, the two-engines-in-one-file problem in frontend.rs, and the rename blast radius. Rationale: establish the exact inconsistency before proposing a fix. |
| 2. | Proposal | `new_cli_processing.md` (change, `## Proposal`) | The design: the `CommandPlugin` registry, the new source-tree layout, the command-dispatch diagram, what moves where for each of the six main.rs bodies, and the behaviour-preservation contract. Rationale: a clean, reviewable proposal for the developer before any code changes. |
| 3. | Implementation details | `new_cli_processing.md` (change, `## Implementation details`) | The exact file operations, the item-by-item frontend split assignment, every reference update, the args.rs contents, what each run_cli contains, and the registry definition. Rationale: a complete, unambiguous implementation spec a future session can execute without re-deriving scope. |
| 4. | Implementation | `src/main.rs` (change - dispatcher only), `src/cli/commands/mod.rs` (change - registry + find), `src/cli/args.rs` (new), `src/cli/utils.rs` (rename from frontend.rs, conversion half), `src/cli/commands/{parse,validate,codegen}.rs` (new plugins), `src/cli/commands/{info,test,diagram/mod}.rs` (change - run_cli extracted), `src/cli/commands/{gen_spec,mc_ref}.rs` (change - import re-point), `src/lib.rs` (change - re-export split), `src/cli/mod.rs` (change - module decls), `tests/frontend_test.rs` -> `tests/utils_test.rs` (rename + import re-point), `tests/info_format_test.rs` (change - import re-point), docs/dev/* + docs/llm/* + KB.md + tests/dev_docs_manifest.txt (change - rename blast radius) | Execute task 3: split frontend.rs, create args.rs + the three new plugins, extract run_cli into info/test/diagram, write the registry, reduce main.rs to the dispatcher, re-point every reference, and re-sync all docs. Rationale: the structural change itself, behaviour-preserving. |
| 5. | Close-out | `new_cli_processing.md` (remove), `docs/roadmap/phase_13/*` (new - this phase's bookkeeping), `docs/roadmap/CURRENT_STATE.md` (change - phase 13 row), `TASKS.md` (change - entry) | Run the full verification (cargo test 535/535, fmt clean, clippy at baseline, exit-code spot checks), delete the working file, write the phase bookkeeping, and commit. Rationale: PROTOCOL.md section 3 close-out - the phase exits only with docs synchronized and all tests green. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 2 | Proposal | change | The original proposal introduced a separate `src/cli/validate.rs` validation-engine module (a sibling of utils.rs), with `commands/validate.rs` as a thin plugin calling into it. User decision (turn after the proposal): NO `src/cli/validate.rs` - that was a typo in the request. ALL CLI-related validation functionality (the validation half of frontend.rs + cmd_validate's argument parsing) goes into `src/cli/commands/validate.rs` as engine + run_cli together, exactly like `mc_ref.rs`. Core semantic validation stays untouched in `src/lang/validator.rs`. The proposal was revised accordingly before implementation. |

## Other information

- The rename is a SPLIT, not a plain rename: git detects `frontend.rs -> commands/validate.rs`
  as the rename (the validation half is the larger, more-similar survivor) and records
  `utils.rs` as a new file. Both are correct views of the same operation.
- clippy must stay at the pre-phase baseline: the refactor moves code, so any existing
  warning relocates but no NEW warning may appear (verified - all lib warnings are in
  pre-existing files, none in the new modules).
- The dev_docs sync gate (tests/dev_docs_test.rs) is the authoritative check that the rename
  blast radius was fully re-synced; it also enforces the LLM-doc line budgets, which the
  inventory growth can push over (see the status file's findings).
