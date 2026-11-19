# Phase 17, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Bring `docs/dev/cli_commands.md` in line with the shipped architecture: the
Overview and Dispatch sections still describe the pre-Phase 13 design (main.rs
carrying cmd_* bodies), the Library API surface section lists pre-Phase 11 flat
module names, and the mc-ref section is exact-only (pre-Phase 16).

## Context
- `src/main.rs` is a ~30-line PURE dispatcher: it resolves the command name via
  `rfs::cli::commands::find(name)` over the `ALL_COMMANDS` registry in
  `src/cli/commands/mod.rs` and calls the plugin's `run_cli`; unknown/missing
  subcommand prints usage + exits 1. Shared argument handling (usage text,
  format/value takers, error conventions) lives in `src/cli/args.rs`. ALL eight
  commands are plugins with a `run_cli` entry point.
- Stale spots (verified 2026-11-19): Overview ~lines 7-13 ("The first six are
  parsed in `src/main.rs` itself; `mc-ref` and `gen-spec` delegate to thin
  shells"); Dispatch section ~line 60 ("main() matches the first argument:
  parse/validate/info/test/diagram/codegen dispatch to their cmd_* wrappers in
  this file"); Library API surface ~lines 120-135 (module list "backends,
  cli_log, codegen_fsm/mc/test, diagram(s), ebnf_grammar, error, ext_tool,
  frontend, gen_spec_cmd, info, ... test_cmd, training, validator" - the
  pre-Phase 11 flat layout); mc-ref section ~lines 328-360 (`McRefOptions {
  max_states, timeout_sec, assumes, ce_format }` exact-only + "NOT applicable
  flags ... --seed/--steps/--violate ... --nondet").
- The CURRENT `src/lib.rs` declares: `pub mod cli; pub mod codegen; pub mod
  json; pub mod lang; pub mod mc; pub mod training; pub mod utils;` plus
  re-exports (validate fns from `cli::commands::validate`, conversion fns +
  `Format` from `cli::utils`, `ParseError`/`Span` from `lang::error`, `ir::*`,
  `parse_file`/`parse_source` from `lang::parser`, `print_ir` from
  `lang::printer`). Read the actual file - do not trust this list over the
  source.
- The CURRENT mc-ref command model: read `src/cli/commands/mc_ref.rs` FIRST for
  the actual options struct(s) and mode dispatch (`run()` delegates to
  `ref_mc_exact` / `ref_mc_fuzz` / `ref_mc_smt` per D-P16-1; `exit_code_for`
  is per-mode). The Phase 16 engine deep dives already exist in
  `docs/dev/mc_engine.md` (fragments dev:mc-engine-exact/fuzz/smt) - link to
  them, do not duplicate.
- The doc-sync gate (`tests/dev_docs_test.rs`) checks that every backticked
  `src/file.rs::item` reference resolves and that fragment anchors are owned -
  keep all existing valid references intact while rewriting prose.

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/cli_commands.md` | change | Overview, Dispatch, Library API surface, mc-ref section rewritten to the shipped state |

## Planned actions

### Rewrite the Overview dispatch description
Replace "The first six are parsed in `src/main.rs` itself; `mc-ref` and
`gen-spec` delegate to thin shells" with the plugin-registry architecture:
ALL eight commands are plugins registered in `ALL_COMMANDS`
(`src/cli/commands/mod.rs`), each exposing `run_cli(args) -> ExitCode`;
`main.rs` is a pure dispatcher over `find()`; shared argument handling lives
in `src/cli/args.rs`. Keep the command-to-module map table (it is still
accurate) and the exit-code convention paragraph.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/cli_commands.md` | change (Overview, ~lines 7-40) | architecture prose must match main.rs or it misleads every future session |

### Rewrite the Dispatch section
Describe `main()` as: resolve name via `find()`, call `cmd.run(&args[1..])`;
unknown subcommand -> `error: unknown subcommand '<x>'; expected ...` + usage,
exit 1; no args -> usage, exit 1. The "Per-command wrappers" paragraph becomes
a description of the uniform `run_cli` manual-parsing loop (shared helpers in
`src/cli/args.rs`: `take_format`, `take_value`, error conventions) - no more
cmd_*-in-main distinction.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/cli_commands.md` | change (Dispatch, ~lines 55-120) | same staleness class as the Overview |

### Rewrite the Library API surface section
Replace the flat module list with the current layout: the seven public modules
(`cli`, `codegen`, `json`, `lang`, `mc`, `training`, `utils`) + the re-exports
actually present in `src/lib.rs` (read it). Keep the "crate is a LIBRARY plus a
CLI" framing and the CLI-entry-point note.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/cli_commands.md` | change (Library API surface, ~lines 120-140) | pre-Phase 11 names reference files that do not exist |

### Rewrite the mc-ref section for mode dispatch
Read `src/cli/commands/mc_ref.rs`, then describe: the mode-dispatching entry
(`run()` delegating to `ref_mc_exact` / `ref_mc_fuzz` / `ref_mc_smt`, D-P16-1),
the ACTUAL options model (struct names from the source), per-mode exit codes via
`exit_code_for`, and not-applicable reduced to `--threads`/`--fast`. Link the
engine deep dives to `docs/dev/mc_engine.md` fragments (dev:mc-engine-exact,
dev:mc-engine-fuzz, dev:mc-engine-smt). Keep the gen-spec half of the section
unchanged unless stale (verify against `src/cli/commands/gen_spec.rs`).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/cli_commands.md` | change (mc-ref section, ~lines 328-360) | exact-only description contradicts the Phase 16 CLI |

## Task deliverables
- `docs/dev/cli_commands.md` matching the shipped architecture in all four
  stale spots.

## Task exit criteria
- [ ] No prose remains describing cmd_* bodies in main.rs or "the first six"
      being parsed there.
- [ ] The Library API surface module list matches `src/lib.rs` exactly (verify
      by reading the file).
- [ ] The mc-ref section names the actual options model + mode dispatch from
      `src/cli/commands/mc_ref.rs`, with per-mode exit codes.
- [ ] All backticked `src/file.rs::item` references still resolve: `cargo test
      --test dev_docs_test` green.
- [ ] `cargo fmt --check` clean.

## Non-goals
- No changes to other docs/dev/ files (mc_engine.md is already current from
  Phase 16).
- No source changes.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| exit_code_for scope | The task file said "per-mode exit codes via `exit_code_for`"; the actual source shows `exit_code_for` is EXACT-mode-only (it takes `ExactStatus`) - fuzz and smt-bounded exit codes are derived inline in `run_mc_ref`'s mode dispatch. The doc describes the actual mechanism; no work deviation, descriptive correction only |
