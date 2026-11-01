# Phase 5, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_5_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/ext_tool.rs` | External-tool resolver + invocation + `--print-script` emission (shared CLI infrastructure for tasks 7/8): `ExtTool` presets (rustc/gcc/dot with `RFS_*` env vars), `resolve(cli_path, cli_args)` implementing FLAG > ENV > PATH precedence (empty values = unset; missing RFS_BIN-style file = error) with env args first and CLI args appended verbatim; `ResolvedTool::render` (POSIX-sh quoting of paths/args with spaces) and `run` (captures stdout+stderr for diagnostics); `sh_quote`; deterministic `Script` builder (shebang, version header, per-tool lines, numbered steps - no timestamps). 7 inline unit tests: precedence matrix, args ordering, quoting rules, byte-pinned script output (with/without tools), stub-tool end-to-end proving env/flag parameters reach the invoked process. |
| `src/cli_log.rs` | Minimal stderr logger shared by the new CLI commands: `LogLevel::parse("off\|error\|warn\|info\|debug")` + `Logger` with level-filtered `error/warn/info/debug`. Fatal command diagnostics deliberately BYPASS the logger (always printed, exit 1) - the filter applies to progress output only. |
| `src/info.rs` | `rfs info` logic: load spec (.rfs/.json via extension), parse + validate (exit-1 diagnostic on failure), then render TEXT (per-module sections: version/initial state/enums/events/outputs with variants+field types/variables with init/transition count/on-error/properties+assumptions each with `monitored` or `skipped: <reason>` via the monitor compiler's R1-R4 check) or `--json` (stable schema, all keys present, null for absent values, rendered by task 2's json_rw writer). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/main.rs` | changed | `info` subcommand dispatch + `cmd_info` (arg parsing: `--json`, `--log-level <level>`, spec path; unknown options/levels rejected with usage) + usage text (new line + description paragraph); "expected parse, validate or codegen" message now lists info. |
| `src/lib.rs` | changed | `pub mod cli_log; pub mod ext_tool; pub mod info;` registered (alphabetical). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `BUILD.md` | changed | Section 1: workspace note carried over from task 1 unchanged. Section 3: counts actualized to the post-task-3 state (273 root / 280 workspace; stale "part of the 230" line fixed). Section 4: new `rfs info` command row. |
| `docs/roadmap/PHASE_5_TASK_3.md` | changed | Task corrections T3-1 (stale 230 baseline in exit criteria) + T3-2 (ext_tool unit tests inline in lib, not frontend_test). |

## Other file changes

None.

## Changed functionality

- None (new command + shared infrastructure only; existing commands untouched).

## Removed functionality

- None.

## New functionality

- `rfs info` stable command: static spec introspection with monitor-subset status per property/assumption (`monitored` / `skipped: <reason>` reusing the monitor compiler's R1-R4 check); `--json` machine-readable mode (stable schema, all keys present, task 2 writer) and `--log-level off|error|warn|info|debug`.
- Shared CLI infrastructure for tasks 7/8/10/13: external-tool resolver with the user-required env-var parameterization (`RFS_RUSTC`/`RFS_GCC`/`RFS_DOT` + `RFS_*_ARGS`, precedence FLAG > ENV > PATH), captured-output invocation, POSIX-sh quoting, and deterministic `--print-script` emission (version header, per-tool lines, numbered steps).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `ext_tool::tests::*` (7) | new | precedence matrix (flag > env > default), args ordering (env first, CLI appended), sh quoting rules, byte-pinned script output (with and without tools/args), stub-tool end-to-end (resolved path + all args reach the process; CLI flag beats env var) |
| `cli_log::tests::*` (2) | new | level parsing (all five names + rejection of unknowns), filter behavior per level |
| `info_text_on_all_valid_examples` | new | all 45 valid examples: non-empty text starting with the module header, every section header present |
| `info_json_parses_and_matches_pinned_example` | new | 44_token_ring (exercises BOTH monitor statuses): JSON parses via json_rw, fields data-consistent with the IR parsed from the same source, monitor_status taxonomy pinned, pretty layout check |
| `e2e_info_valid_exits_zero_and_prints_module` | new | binary exit 0 + module header + transition count on stdout |
| `e2e_info_invalid_spec_exits_one_with_diagnostics` | new | validation failure (unknown state) -> exit 1, stderr names the bad state |
| `e2e_info_usage_lists_info_and_rejects_bad_options` | new | no-args usage lists `rfs info`; unknown option and bad `--log-level` value both exit 1 with usage |
| `e2e_info_log_level_debug_prints_progress_on_stderr` | new | default level hides debug output; `--log-level debug` prints it to stderr (stdout stays the data) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root package) | OK (**273/273**; 158 lib incl. 7 ext_tool + 2 cli_log, 51 frontend incl. the 7 new info tests) |
| `cargo test --workspace` | OK (**280/280**) |
| `cargo test-generated-code` | OK (537/537 - no codegen changes in this task) |
| `cargo clippy --workspace --all-targets` | OK (zero new warnings; pre-existing baseline only) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- One test-authoring slip caught by the suite on first run: the e2e assertion expected `transitions: 2` for VALID_RFS which has a single transition - fixed to 1.
- `04_tcp_handshake_like.rfs` initially chosen as the pinned JSON-consistency example turned out to have only monitorable (safety) properties, so it could not exercise the `skipped:` status; switched the pin to `44_token_ring.rfs` which has both (recorded in the test comment).

## Other important findings/information

- For task 7 (`rfs test`): the resolver/run/script API is ready - construct `Script::new("rfs test ...", spec)`, add resolved tools via `.with_tool()`, steps via `.step()`, print `.render()` under `--print-script`.
- For task 8 (`rfs diagram`): same, with `ExtTool::DOT`; the no-external-tool case (dot/md formats) is a script with zero tool lines (shape covered by the `script_without_tools_or_args` pin).
- The info JSON schema (all keys always present, null for absent values) is the convention task 13's `validate --json` should follow.
