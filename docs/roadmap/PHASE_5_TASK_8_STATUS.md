# Phase 5, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_5_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/diagram.rs` | `rfs diagram` plugin API + registry + orchestration: `DiagramPlugin` (static data + fn pointers, mirrors `codegen::Backend`), `PlanRenderFn`/`RenderSvgFn` type aliases, `ALL_DIAGRAMS`/`DIAGRAMS` registry with `list_diagrams()`/`get_diagram()`, `RenderPlan` (prelude steps + rendered cmdline for --print-script), `DiagramOptions`/`ToolOverride`, `run_diagram(path, opts)` = load+validate (same contract as info) -> first module -> generate -> optional strict render or self-contained script; `render_script` builds the POSIX sh (heredoc source step, mermaid puppeteer-JSON step, render invocation). |
| `src/diagrams/mod.rs` | Plugin modules wiring + shared label helpers over the IR: `event_label` (`Group`/`Group(Variant(...))` reusing info's renderer), `guard_label`, `emit_labels` (`render_output`), `states` (State-enum variants, declaration order), `render_check` (shared strict exit-status gate for all renderers). |
| `src/diagrams/dot.rs` | Graphviz DOT plugin: digraph, initial state `[style=filled]`, edge label `Event [guard] -> next`, on_error dashed per-state edges; `plan_render`/`render_svg` via resolved dot (`-Tsvg <src> -o <out>`). |
| `src/diagrams/plantuml.rs` | PlantUML plugin: `@startuml`/`title`/`[*] --> init`; edge label `event \n guard` (literal `\n`, user-specified split); emitted data via `note right on link` (single-line for 1 emit, multi-line block for many); on_error as `-[dashed]->` per state; render via `plantuml -tsvg <src>` (output name derived from input = the --out file by construction). |
| `src/diagrams/mermaid.rs` | Mermaid plugin: `stateDiagram-v2`; ALL transition info in multi-line edge labels (`event \n guard \n emits X`, `[on_error]` prefix - mermaid has no notes-on-links and no dotted edges); render via `mmdc -i <src> -o <out> -p <pup> -q` with auto-generated temp puppeteer config `{"args":["--no-sandbox","--disable-dev-shm-usage"]}` (overridable `RFS_MERMAID_PUPPETEER_ARGS`), cleaned up after the run. |
| `src/diagrams/tables.rs` | Two plugins sharing one data layer: `table` (plain-text state x event matrix with computed column widths + on_error line + variables/events/outputs summary) and `md` (same data as a Markdown document); no renderers (`svg_tool: None`). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/ext_tool.rs` | changed | +`ExtTool::PLANTUML` (RFS_PLANTUML/RFS_PLANTUML_ARGS) and `ExtTool::MERMAID` (RFS_MERMAID/RFS_MERMAID_ARGS) presets; module doc updated. Existing behaviour/tests unchanged. |
| `src/info.rs` | changed | `render_type`/`render_expr`/`render_pattern_variant_group` made `pub(crate)` (reused by diagram labels so formats never disagree with `rfs info`); new `pub(crate) render_output` (`Group`, `Group(V)`, `Group(V(a, b))`). |
| `src/main.rs` | changed | `diagram` subcommand dispatch + `cmd_diagram` (flags: `--list`, `--format`, `--out`, `--render svg`, `--dot/--plantuml/--mermaid PATH`, `--*-args`, `--print-script`, `--log-level`; unknown options rejected with usage); usage() new line + paragraph; stdout via `print!` (byte-exact, codegen convention). |
| `src/lib.rs` | changed | `pub mod diagram; pub mod diagrams;` registered. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TOOLCHAIN.md` | changed | New section 6: install commands (apt graphviz/plantuml, npm mermaid-cli) with verified versions, parameterization table (env vars + CLI overrides), by-hand invocations per tool, gotchas (mmdc --no-sandbox as root + -o extension restriction; plantuml exit-200 + error-SVG quirk; NO concurrent mmdc in this container). |
| `BUILD.md` | changed | Section 3 counts actualized (299 root / 306 workspace, frontend suite note incl. the diagram tests); section 4 new `rfs diagram` command row. |
| `docs/roadmap/PHASE_5_TASK_8.md` | changed | Extended plan per user requirement (plugin architecture, 5 formats, strict render errors) - see TASKS.md entry for the decision record. |

## Other file changes

- `TASKS.md`: one new entry (2026-10-21).
- `docs/roadmap/INTRO.md`: phase-state row 5 -> "tasks 1-3 + 8 of 16 done".

## Changed functionality

- None to existing commands. (`rfs info` renderers gained pub(crate) visibility only.)

## Removed functionality

- None.

## New functionality

- `rfs diagram` stable command on a plugin registry (adding a format = one module + one registry line): formats `dot`, `puml`, `mmd`, `table`, `md`; PRIMARY behaviour = pure-Rust text generation with NO external tool calls (stdout or --out); `--list`.
- `--render svg`: requires `--out FILE.svg`; writes the generated source next to it (same stem, plugin extension) and invokes the resolved renderer. STRICT error semantics: missing/unresolvable tool or non-zero renderer exit => rfs exit 1 + diagnostic incl. captured tool stderr - no graceful degradation (CI/CD requirement, user decision 2026-10-21).
- Unified external-tool parameterization: `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID` (+ `*_ARGS`) env vars, overriding `--dot/--plantuml/--mermaid PATH` + `--*-args "..."` flags (FLAG > ENV > PATH; env args first, CLI appended).
- `--print-script`: self-contained deterministic POSIX sh script (heredoc source step; mermaid: puppeteer-JSON step; render invocation) - executable as-is.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `diagram_pinned_{dot,puml,mmd,table,md}_output` (5) | new | byte-pinned generation for all 5 formats on a pinned spec (3 states, initial, unit + struct-variant events, guard, emits, on_error) |
| `diagram_all_formats_sweep_valid_examples` | new | all 45 valid examples x 5 formats generate without error |
| `e2e_diagram_valid_exits_zero_and_prints` / `e2e_diagram_invalid_spec_exits_one` | new | binary exit codes + stdout content |
| `e2e_diagram_render_{dot,puml,mmd}_real_tool` (3) | new | gated on tool availability: --render svg exits 0, source written next to out, SVG contains expected state/event/guard/emits strings |
| `diagram_puml_negative_control_exit_200` / `diagram_mmd_negative_control_parse_error` | new | real tools through the plugin harness reject known-bad input (plantuml exit 200 "Error line"; mmdc exit 1 "Parse error") - proves the acceptance tests have teeth |
| `e2e_diagram_stub_{dot,plantuml}_receives_env_and_cli_args` / `e2e_diagram_flag_path_beats_env_for_mermaid` (3) | new | stub tools prove argv reaches the process (env args first + CLI appended; full arg order incl. format-specific args) and FLAG > ENV precedence |
| `e2e_diagram_missing_dot_tool_exits_one` / `e2e_diagram_failing_plantuml_propagates_stderr` / `e2e_diagram_failing_mmdc_exits_one` (3) | new | strict render errors: missing tool, failing renderer with captured stderr propagation |
| `e2e_diagram_print_script_{puml_render,mmd_render,no_tool}_pinned` (3) | new | byte-pinned scripts incl. the mermaid puppeteer-config step; no-tool script for text formats |
| `e2e_diagram_list_pinned` / `e2e_diagram_rejects_unknown_format` / `e2e_diagram_render_requires_out_svg` / `e2e_diagram_table_md_have_no_renderer` (4) | new | CLI surface pins + rejection cases |

All e2e diagram tests run under one process-wide lock and clear the RFS_* tool env first: two concurrent mmdc/Chrome launches fail nondeterministically in this container, and env leaks between parallel tests would corrupt tool resolution.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root package) | OK (**299/299**; 158 lib + 77 frontend incl. the 26 new diagram tests; was 273) |
| `cargo test --workspace` | OK (**306/306**; was 280) |
| `cargo test-generated-code` | OK (537/537 - no codegen changes in this task) |
| `cargo clippy --all-targets` | OK (zero new warnings; 12 pre-existing baseline remain) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- C8-1: plantuml's render_svg initially had a defensive "succeeded but SVG not produced" file check - it broke legitimate stub/wrapper tools; removed (exit status is the strict contract).
- C8-2: `--print-script` stdout initially carried a double trailing newline (script text + println); fixed by using `print!` in cmd_diagram (codegen convention) so redirection is byte-exact.
- rustc 1.99 accepts multi-line string literals (actual newlines inside `"..."`) - the pre-existing `usage()` in main.rs relies on this; noted so future sessions don't "fix" it away.

## Other important findings/information

- Empirically verified tool facts recorded in TOOLCHAIN.md section 6: plantuml 1.2020.02 exits 200 on syntax error AND still writes an error SVG (gate on exit status); mmdc/mmd 12.2.0 has no notes-on-links and no dotted edges in stateDiagram-v2 (hence label-carrying design), needs a puppeteer config with --no-sandbox when running as root, and must not run two instances concurrently in this container.
- For task 14 (training) / task 15 (docs sync): the new CLI surface is `rfs diagram` with the options above; DESIGN section 23 should record D-P5 decisions for the plugin architecture + strict render semantics.
