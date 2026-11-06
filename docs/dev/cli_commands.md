# RFS CLI Commands + Diagrams (Class A)

The command layer of the toolchain: per-command logic modules, the diagram
plugin system, and the shared external-tool infrastructure.

<a id="dev:cli-commands-overview"></a>
## Overview

The `rfs` binary exposes EIGHT subcommands, dispatched from
`src/main.rs::main`: `parse`, `validate`, `info` (with the Phase 7
`info format` sub-mode), `test`, `diagram`, `codegen`, `mc-ref`, and
`gen-spec`. The first six are parsed in `src/main.rs` itself; `mc-ref` and
`gen-spec` delegate to thin shells (`src/mc_ref_cmd.rs::run_cli`,
`src/gen_spec_cmd.rs::run_cli`) that own their argument parsing. Command-to-
module map:

| Command | Logic entry | Deep-dive fragment |
|---|---|---|
| parse / validate | `src/frontend.rs::run_conversion` / `src/frontend.rs::run_validate` (+ `src/frontend.rs::run_validate_structured` for --json) | dev:cli-commands-frontend |
| info / info format | `src/info.rs::run_info` / `src/info_format.rs::FORMATS` | dev:cli-commands-info |
| test | `src/test_cmd.rs::run_test` | dev:cli-commands-test |
| diagram | `src/diagram.rs::run_diagram` + the plugins in `src/diagrams/` | dev:cli-commands-diagram |
| codegen | `src/codegen.rs` registry dispatch (via `src/main.rs::cmd_codegen`) | [codegen_core.md](codegen_core.md) |
| mc-ref | `src/mc_ref_cmd.rs::run_mc_ref` over `src/ref_mc.rs` | dev:cli-commands-mc-ref-gen-spec |
| gen-spec | `src/gen_spec_cmd.rs::run_cli` over `src/lang/specgen.rs` | dev:cli-commands-mc-ref-gen-spec |

**Exit-code convention.** 0 = success; 1 = ANY error (usage, parse,
validation, IO, missing tool for a SELECTED suite). The generated MC binaries
keep their own 0/1/2 status taxonomy (dev:mc-engine-exact); `rfs mc-ref`
mirrors it (`src/mc_ref_cmd.rs::exit_code_for`).

**Shared CLI conventions.** Argument parsing is manual (no clap): --help/-h
prints the usage text and exits 0; an unknown option or subcommand prints an
error + the usage and exits 1; extra positional arguments are rejected.
Generated text and scripts go to stdout with `print!` (NOT println!) so the
bytes are reproduced exactly when redirected (codegen convention). Fatal
diagnostics bypass the logger (always stderr, drive the exit code);
`src/cli_log.rs::Logger` (levels via `src/cli_log.rs::LogLevel`, --log-level
off|error|warn|info|debug, default info) carries informational progress only.
The file-input contract shared by info/test/diagram/mc-ref: the extension
selects the format (.rfs/.json), the spec is loaded + validated before any
work, and single-module commands run on the FIRST module. Every command that
invokes external tools goes through the shared resolver (dev:cli-commands-
ext-tool) and accepts --print-script.

CLI behavior rules at file granularity: [KB.md](../../KB.md) section 4.5; external user
documentation: docs/user/; codegen plugin API: [CODEGEN.md](../../CODEGEN.md) +
[codegen_core.md](codegen_core.md).

Code: `src/main.rs::main`, `src/cli_log.rs::LogLevel`, `src/cli_log.rs::Logger`


<a id="dev:cli-commands-dispatch"></a>
## Dispatch (`src/main.rs`)

**usage().** `src/main.rs::usage` is the full usage text - one line per
command plus a help paragraph per command (flags, semantics, exit codes) and
an examples block. It is printed by --help on every command and on unknown or
missing subcommand.

**main().** `src/main.rs::main` matches the first argument: parse/validate/
info/test/diagram/codegen dispatch to their cmd_* wrappers in this file;
mc-ref and gen-spec call their modules' run_cli shells; an unknown subcommand
prints `error: unknown subcommand '<x>'; expected ...` + usage and exits 1;
no arguments prints the usage and exits 1.

**Per-command wrappers.** All follow the same manual-parsing loop (a --
token is skipped, flags consume their values via helpers like
`src/main.rs::take_format`, anything else starting with - is an unknown
option):

- `src/main.rs::cmd_parse`: --input/--output (both required) + path + optional
  --target-dir; calls `src/frontend.rs::run_conversion`; prints one `wrote`
  line per output + a summary.
- `src/main.rs::cmd_validate`: optional --input + --json. Text mode prints
  `valid` lines, warnings to stderr, and `invalid ... (N error(s), written
to ...)` for directory-mode failures (file mode reports on stderr). JSON mode
  renders the pinned document via `src/frontend.rs::render_validate_json`
  with print!; exit codes are unchanged by --json.
- `src/main.rs::cmd_codegen`: delegates to `src/codegen.rs::parse_codegen_args`
  -> List / Help / BackendHelp / Generate. Generate resolves the backend,
  maps the input extension to a Format, loads via
  `src/frontend.rs::convert_file_to_ir`, builds `src/codegen.rs::BackendOptions`
  (spec_hash starts empty - it is stamped from the whole-file canonical IR
  JSON by the command's run per D-P6-3), and calls
  `src/codegen.rs::CodegenCommandKind::run`. With --out: parent directories
  are created, the artifact + any extra files (e.g. the rust test command's
  fuzz skeleton) are written next to it. Without --out: stdout mode REFUSES
  commands that emit additional files (strict - no silent file loss).
- `src/main.rs::cmd_info`: --json/--log-level + path; dispatches on the BARE
  first token `format` BEFORE any path handling, so a spec file literally
  named format cannot collide (D-P7-6); otherwise calls
  `src/info.rs::run_info` and prints the result.
- `src/main.rs::run_info_format`: --list prints
  `src/info_format.rs::list_markdown`; a name is required otherwise; unknown
  names get the registry's own diagnostic (it lists the remedy); the default
  --output is the entry's first; `src/info_format.rs::render` output goes to
  stdout via print!.
- `src/main.rs::cmd_test`: builds `src/test_cmd.rs::TestOptions` (default
  backend rust, suites [unit]); --suite is repeatable or comma-separated -
  the FIRST occurrence replaces the default, later ones append with order-
  preserving dedupe; tool path/args flags map onto the options; calls
  `src/test_cmd.rs::run_test` - a script result is printed via print!, a
  success note goes through logger.info.
- `src/main.rs::cmd_diagram`: builds `src/diagram.rs::DiagramOptions`
  (default format dot, renderer internal); --render accepts only svg;
  --renderer internal|external; tool overrides are routed by
  `src/main.rs::tool_override_mut` (--dot/--plantuml/--mermaid with or
  without the -args suffix); calls `src/diagram.rs::run_diagram`.

Code: `src/main.rs::usage`, `src/main.rs::main`, `src/main.rs::take_format`,
`src/main.rs::cmd_parse`, `src/main.rs::cmd_validate`, `src/main.rs::cmd_codegen`,
`src/main.rs::cmd_info`, `src/main.rs::run_info_format`, `src/main.rs::cmd_test`,
`src/main.rs::cmd_diagram`, `src/main.rs::tool_override_mut`


<a id="dev:cli-commands-library"></a>
## Library API surface (`src/lib.rs`)

The crate is a LIBRARY plus a CLI: every module is public (backends, cli_log,
codegen, codegen_fsm/mc/test, diagram(s), ebnf_grammar, error, ext_tool,
frontend, gen_spec_cmd, info, info_format, ir, json_rw, json_schema, mc_plan,
mc_ref_cmd, monitor, parser, printer, ref_mc, smt_emit/plan, spec_hash,
specgen, test_cmd, training, validator); the CLI entry point is
`src/main.rs`. Re-exports for external consumers: `src/lang/parser.rs::parse_source`
/ `src/lang/parser.rs::parse_file`, `src/lang/printer.rs::print_ir`,
`src/lang/error.rs::ParseError` / Span, the whole IR model (ir::*), and the
frontend API (`src/frontend.rs::Format`, run_conversion, plan_conversion,
convert_file_to_ir, render_ir, run_validate, validate_one,
validate_one_with_warnings, ValidateResult, ValidateOneResult, FailedFile,
normalize_abs, collect_files_with_ext, err_file_path, write_err_file).

`src/lib.rs::collect_rfs_files` accepts a .rfs file or a directory; directories
are traversed recursively and results are sorted - this is what makes the
file-driven example test suite extensible without changing test code.

Consumers: the integration tests call library functions directly (e.g. the
info-format walk runs `src/info.rs::run_info` in-process with json=true);
`crates/rfs-build` invokes the BINARY (RFS_BIN resolution), not the library;
external tooling goes through IR JSON (hand-rolled `src/json/json_rw.rs`). Full API
list: [KB.md](../../KB.md) section 4.3; codegen plugin API: [CODEGEN.md](../../CODEGEN.md) +
[codegen_core.md](codegen_core.md).

Code: `src/lib.rs::collect_rfs_files`, `src/lang/parser.rs::parse_source`,
`src/lang/printer.rs::print_ir`


<a id="dev:cli-commands-frontend"></a>
## parse + validate (`src/frontend.rs`)

**Format.** `src/frontend.rs::Format` (Rfs | Json): name()/ext(),
from_extension (case-insensitive), FromStr ("rfs"/"json", a clear error
otherwise).

**Path helpers.** `src/frontend.rs::normalize_abs` lexically normalizes a path
to absolute form, resolving . and .. components WITHOUT requiring the path to
exist - the collision guard's comparison basis. `src/frontend.rs::collect_files_with_ext`
recursively collects files with a case-insensitive extension match, sorted
(subdirectories visited in sorted order for determinism).

**Conversion.** `src/frontend.rs::plan_conversion` computes the input/output
pairs and returns ALL mapping errors (an empty error list is never returned
with Err): a file input must carry the --input extension; a directory input is
scanned recursively for that extension only (an empty result is an error);
output naming REPLACES the original extension, never stacks it (a.rfs ->
a.json); without --target-dir each output is written next to its input, with
it the input directory structure is preserved inside the target; the
collision guard compares normalized absolute paths and refuses any conversion
whose output would overwrite its input. `src/frontend.rs::convert_file_to_ir` /
`src/frontend.rs::render_ir` / `src/frontend.rs::run_conversion` execute the
plan - per-file parse errors in a directory run are collected and do not stop
the remaining files.

**Validation (text).** `src/frontend.rs::validate_one` /
`src/frontend.rs::validate_one_with_warnings`: parse +
`src/lang/validator.rs::validate_ir`; warnings come from the Phase 2 non-fatal
channel (`src/lang/validator.rs::module_warnings` per module). Result shapes:
`src/frontend.rs::ValidateResult { ok_files, warnings, failed, write_errors,
is_directory }` and `src/frontend.rs::FailedFile { path, errors, err_file }`
(err_file is None in file mode). `src/frontend.rs::run_validate`: file mode
reports to stderr with no side files; directory mode writes a
<name>.rfs-validation.err text file next to each failing source
(`src/frontend.rs::err_file_path` / `src/frontend.rs::write_err_file`); exit 1
iff any file failed or an err file could not be written.

**Validation (structured, --json).** `src/frontend.rs::StructuredError {
kind, line, col, message }`: kind is "parse" or "semantic"; parse errors
carry the real 1-based line/col from ParseError::line_col (which can read
1:1 when the first module fails wholesale); semantic errors carry no span and
are pinned to 0/0. `src/frontend.rs::validate_one_structured` mirrors the text
path's contract (Ok = structured errors, empty on success; Err = CLI-level
misuse). `src/frontend.rs::validate_json_doc` builds the PINNED document shape
- one object with file / ok / errors (each error: kind, line, col, message) -
through json_rw. `src/frontend.rs::run_validate_structured` mirrors
run_validate's traversal/extension contract but reports only FAILING files
and writes NO .err side files (editors consume stdout).
`src/frontend.rs::render_validate_json`: file mode = one object; directory
mode = an array with one object per failing file.

Code: `src/frontend.rs::Format`, `src/frontend.rs::normalize_abs`,
`src/frontend.rs::collect_files_with_ext`, `src/frontend.rs::plan_conversion`,
`src/frontend.rs::convert_file_to_ir`, `src/frontend.rs::render_ir`,
`src/frontend.rs::run_conversion`, `src/frontend.rs::FailedFile`,
`src/frontend.rs::ValidateResult`, `src/frontend.rs::validate_one`,
`src/frontend.rs::validate_one_with_warnings`, `src/frontend.rs::StructuredError`,
`src/frontend.rs::validate_one_structured`, `src/frontend.rs::validate_json_doc`,
`src/frontend.rs::run_validate_structured`, `src/frontend.rs::render_validate_json`,
`src/frontend.rs::run_validate`, `src/frontend.rs::err_file_path`,
`src/frontend.rs::write_err_file`


<a id="dev:cli-commands-info"></a>
## info + info format (`src/info.rs`, `src/info_format.rs`)

**rfs info.** `src/info.rs::InfoOptions { json }`; `src/info.rs::run_info`
applies the shared file contract (extension selects format, load + validate -
a directory is an error) and renders a PER-MODULE static view: version header,
initial state, enums, events/outputs with variants + field types, variables
with types + init expressions (`src/info.rs::render_formula`), the on_error
block, transition count, and properties/assumptions each with their monitor-
subset status - `src/info.rs::monitor_status` returns "monitored" or
"skipped: <reason>" from `src/monitor.rs::compile_formula` (the R1-R4 gate;
identical classification to the backends). NO model checking, NO code
generation. Text via `src/info.rs::render_text`; --json via
`src/info.rs::render_json` - a machine-readable document with stable key order
and ALL keys always present (empty arrays and null instead of omitted fields)
for IDE/tooling reuse.

**rfs info format.** The self-describing-format registry (Phase 7, D-P7-6):
`src/info_format.rs::FormatInfo { name, summary, outputs, render }` - outputs
in preference order, the first is the default when --output is omitted. The
static `src/info_format.rs::FORMATS` holds the ten entries: spec-rfs (the RFS
v0 EBNF grammar in md/plantuml/svg, rendered from the in-binary model in
`src/ebnf_grammar.rs` - the canonical text of [DESIGN.md](../../DESIGN.md) section 6's fence),
spec-json / ce-trace / mc-report / validate-json / info-json (md + draft-07
JSON Schema via the shared emitter in `src/json/json_schema.rs`; schema builders
`src/info_format.rs::spec_json_schema`, `ce_trace_schema`, `mc_report_schema`,
`validate_json_schema`, `info_json_schema`; the mc-report entry's
counterexample definition REUSES the ce-trace root through the shared
`src/info_format.rs::counterexample_model()` so the two documents cannot
drift), diagram-dot / diagram-puml / diagram-mmd (the exact generated-subset
line grammars, md), artifact-header (the line-1 stamp template, md).
`src/info_format.rs::find` / `list_markdown` / `render` - unknown names and
unsupported --output values are clear errors. The D-P7-9 sync tests walk REAL
tool output against these embedded models (drift caught in both directions):
tests/info_format_test.rs.

Code: `src/info.rs::InfoOptions`, `src/info.rs::run_info`,
`src/info.rs::render_text`, `src/info.rs::render_json`,
`src/info.rs::monitor_status`, `src/info.rs::render_formula`,
`src/info_format.rs::FormatInfo`, `src/info_format.rs::FORMATS`,
`src/info_format.rs::find`, `src/info_format.rs::list_markdown`,
`src/info_format.rs::render`, `src/info_format.rs::spec_json_schema`,
`src/info_format.rs::ce_trace_schema`, `src/info_format.rs::mc_report_schema`,
`src/info_format.rs::validate_json_schema`, `src/info_format.rs::info_json_schema`,
`src/info_format.rs::counterexample_model`, `tests/info_format_test.rs`


<a id="dev:cli-commands-test"></a>
## test (`src/test_cmd.rs`)

**Options.** `src/test_cmd.rs::TestSuite` (Unit | Fuzz) + from_str;
`src/test_cmd.rs::TestOptions { backend, suites, steps, tool paths/args,
fuzz_max_total_time, fuzz_flags, print_script }` with Default (backend rust,
suites [unit], 30 s fuzz budget). `src/test_cmd.rs::resolved_fuzz_flags`
merges the RFS_FUZZ_FLAGS env value first, then --fuzz-flags (both backends).

**run_test pipeline.** `src/test_cmd.rs::run_test`: (1) load + validate under
the shared file contract; (2) resolve the backend via
`src/codegen.rs::get_backend` and the compiler preset (`src/ext_tool.rs::ExtTool`
RUSTC for rust, GCC for c) through ExtTool::resolve with the CLI overrides;
(3) codegen the test artifact via the `src/codegen.rs::CodegenCommandKind`
UnitTests registry dispatch (no renderer logic duplicated); the fuzz-skeleton
extra file is consumed by the fuzz suite when selected; (4) when --suite fuzz:
`src/test_cmd.rs::build_fuzz_context` - a SECOND registry dispatch for the
production source + the emitted skeleton, generated BEFORE --print-script so
both modes share them; (5) --print-script -> `src/test_cmd.rs::render_script`
(+ `src/test_cmd.rs::render_fuzz_steps`) - deterministic POSIX sh, no
execution; (6) execution in a temp dir (`src/test_cmd.rs::make_temp_dir`; a
Drop guard cleans it on EVERY exit path): suites run SEQUENTIALLY and every
SELECTED suite gets to run so one invocation surfaces all problems; failures
are joined into the error.

**Unit suite.** `src/test_cmd.rs::run_unit_suite`: write the artifact, compile
with the resolved tool (rustc --edition 2021 --test / gcc -Wall), run the
executable; per-test results stream on stdout (libtest lines for rust,
PASS/FAIL lines for c).

**Fuzz pipelines.** `src/test_cmd.rs::run_fuzz_c`: clang -O1
-fsanitize=fuzzer,address over fuzz_target.c ONLY - the skeleton #includes
the production source (rfs_generated_fsm.c) under its hardcoded name, so
compiling both files is a multiple-definition link error; a missing clang
gives a diagnostic naming RFS_CLANG; the bounded run merges
ASAN_OPTIONS=detect_leaks=0 (`src/test_cmd.rs::c_asan_options` - the generated
C FSM has no free API). `src/test_cmd.rs::run_fuzz_rust`: a temp cargo-fuzz
scaffold (the task-12 layout) - lib root = the generated production source,
the emitted skeleton adapted into fuzz/fuzz_targets/ (its use-crate import
rewritten to the lib name), [package.metadata] cargo-fuzz = true, libfuzzer-sys
pinned to the local registry cache for offline builds, rust-toolchain.toml
pinning nightly (`src/test_cmd.rs::RUST_FUZZ_TOOLCHAIN`) because cargo-fuzz
needs -Zsanitizer=address, with a non-nightly inherited RUSTUP_TOOLCHAIN
stripped from the subprocess; run via cargo fuzz build + cargo fuzz run.
`src/test_cmd.rs::run_fuzz_binary`: bounded run with -max_total_time=<N> +
the resolved extra flags; fuzzer exit 0 within budget = pass. Missing
cargo-fuzz / a failing cargo-fuzz invocation get dedicated diagnostics
(`src/test_cmd.rs::cargo_fuzz_missing_diagnostic`,
`src/test_cmd.rs::cargo_fuzz_failure_diagnostic`).

**Scripts.** `src/test_cmd.rs::render_script` + `src/test_cmd.rs::heredoc`
emit a self-contained POSIX sh script (shebang, set -e, header with rfs
version + spec + RESOLVED tool paths/args, generated sources embedded in
quoted heredocs); the output is byte-pinned in tests/frontend_test.rs.

Code: `src/test_cmd.rs::TestSuite`, `src/test_cmd.rs::TestOptions`,
`src/test_cmd.rs::resolved_fuzz_flags`, `src/test_cmd.rs::run_test`,
`src/test_cmd.rs::run_unit_suite`, `src/test_cmd.rs::build_fuzz_context`,
`src/test_cmd.rs::run_fuzz_c`, `src/test_cmd.rs::run_fuzz_rust`,
`src/test_cmd.rs::run_fuzz_binary`, `src/test_cmd.rs::c_asan_options`,
`src/test_cmd.rs::cargo_fuzz_missing_diagnostic`,
`src/test_cmd.rs::cargo_fuzz_failure_diagnostic`,
`src/test_cmd.rs::render_script`, `src/test_cmd.rs::render_fuzz_steps`,
`src/test_cmd.rs::heredoc`, `tests/frontend_test.rs`


<a id="dev:cli-commands-mc-ref-gen-spec"></a>
## mc-ref + gen-spec (`src/mc_ref_cmd.rs`, `src/gen_spec_cmd.rs`, `src/lang/specgen.rs`)

**rfs mc-ref.** `src/mc_ref_cmd.rs::CeFormat` (Text | Json) + parse;
`src/mc_ref_cmd.rs::McRefOptions { max_states, timeout_sec, assumes,
ce_format }` with Default from the shared exact-mode constants
(`src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES`,
`src/codegen_mc.rs::EXACT_DEFAULT_TIMEOUT_SEC`). `src/mc_ref_cmd.rs::run_mc_ref`
applies the shared file contract on the FIRST module, builds a RefConfig, and
calls `src/ref_mc.rs::run` IN-PROCESS - no compilation (rapid prototyping in
shell/REPL; engine deep dive: dev:mc-engine-exact). Reports are UNIFIED with
the generated exact-mode binaries: render_text or to_json_report (one compact
line + trailing newline - the wire contract the parity suites check).
`src/mc_ref_cmd.rs::exit_code_for` mirrors the generated binaries: 0 pass-
complete, 1 fail, 2 unknown-state-limit. NOT applicable flags are documented
in the usage text: --seed/--steps/--violate (fuzz-mode concepts),
--threads/--fast (single-threaded by construction), --nondet (ref_mc explores
all enabled transitions by construction). `src/mc_ref_cmd.rs::run_cli` is the
thin shell main.rs dispatches to.

**rfs gen-spec.** `src/gen_spec_cmd.rs::DEFAULT_SEED` = 12345 (pinned);
`src/gen_spec_cmd.rs::RandomOptions { seed, bounds }`. Two modes:
`src/gen_spec_cmd.rs::generate_random` - SpecGen::new(seed, bounds).generate()
-> print_ir -> `src/gen_spec_cmd.rs::verify_rendered`; and
`src/gen_spec_cmd.rs::mutate_base` - load + validate the base spec, apply K
seeded mutation steps to its first module (RFS modules are independent),
render, verify. The output is ALWAYS parsed + validated before writing
(verify_rendered re-parses + validates the rendered text; on any failure exit
1 with a diagnostic). `src/gen_spec_cmd.rs::run_cli`: fully random mode
(--seed N default 12345 + --size small|medium or explicit --max-* caps)
or incremental mutation (--base SPEC --mutations K; K=0 re-renders the base
unchanged); --out FILE.rfs required; deterministic from the seed (same args =>
byte-identical file).

**specgen engine.** `src/utils/rng.rs::Rng`: an LCG with the SAME constants as
the fuzz wire format (6364136223846793005 / 1442695040888963407); next_f64 is
the same top-53-bit double mapping as the fuzz wire format. `src/lang/specgen.rs::Bounds`
{ max_states, max_events, max_outputs, max_vars, max_transitions, max_props }
with small() (<= 6 states - reachable sets stay tiny) and medium()
(benchmark-like) presets; explicit --max-* caps override individual fields.
`src/lang/specgen.rs::SpecGen::generate`: validity by construction - fuzzer/seed
bytes drive bounds + choices, and raw IR types are deliberately NOT made
Arbitrary. `src/lang/specgen.rs::MutOp`: 17 ops (Add/Remove/Rename for State /
Event / Output, Add/RemoveTransition, TweakGuard/TweakPriority/
TweakOnError/TweakPayloadWidth, Add/RemoveProperty). `src/lang/specgen.rs::mutate_traced`:
per step pick an op, apply it to a CANDIDATE module clone, commit iff
`src/lang/validator.rs::validate_module` passes - invalid candidates are rolled
back and redrawn up to 100 attempts per step, then the step is skipped;
returns (ir, applied ops). `src/lang/specgen.rs::mutate` = mutate_traced without
the trace. Consumers: rfs gen-spec, the root fuzz/ libFuzzer targets (fuzzer
bytes -> seed + bounds), and tests/toolchain_fuzz_test.rs.

Code: `src/mc_ref_cmd.rs::CeFormat`, `src/mc_ref_cmd.rs::McRefOptions`,
`src/mc_ref_cmd.rs::run_mc_ref`, `src/mc_ref_cmd.rs::exit_code_for`,
`src/mc_ref_cmd.rs::run_cli`, `src/gen_spec_cmd.rs::DEFAULT_SEED`,
`src/gen_spec_cmd.rs::RandomOptions`, `src/gen_spec_cmd.rs::generate_random`,
`src/gen_spec_cmd.rs::mutate_base`, `src/gen_spec_cmd.rs::verify_rendered`,
`src/gen_spec_cmd.rs::run_cli`, `src/utils/rng.rs::Rng`, `src/lang/specgen.rs::Bounds`,
`src/lang/specgen.rs::SpecGen`, `src/lang/specgen.rs::MutOp`, `src/lang/specgen.rs::mutate_traced`,
`src/lang/specgen.rs::mutate`, `src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES`,
`src/codegen_mc.rs::EXACT_DEFAULT_TIMEOUT_SEC`, `tests/toolchain_fuzz_test.rs`


<a id="dev:cli-commands-diagram"></a>
## diagram + plugins (`src/diagram.rs`, `src/diagrams/`)

**Renderers.** `src/diagram.rs::Renderer`: Internal (pure-Rust in-process -
the default since Phase 7, D-P7-1; no external binaries are resolved or
invoked) | External (the format's tool resolved through ext_tool - the pre-
Phase 7 behaviour byte-for-byte). The validated renderer for a --render svg
run is chosen per mode with availability checked UP FRONT so execution and
--print-script agree.

**Plugin shape.** `src/diagram.rs::DiagramPlugin { name, description, ext,
generate, svg_tool, plan_render, render_svg, internal_render_svg,
internal_renderer_name }`: generate is pure-Rust text generation (deterministic;
no I/O, no tool calls); svg_tool/render_svg/plan_render are the external path
(None = the format has no external renderer); internal_render_svg is the Phase
7 hook (None = no in-process renderer) with a human-readable crate name for
--print-script comments. The `src/diagram.rs::DIAGRAMS` registry holds five
plugins: dot, puml, mmd, table, md (`src/diagrams/dot.rs::DOT_PLUGIN`,
`src/diagrams/plantuml.rs::PUML_PLUGIN`, `src/diagrams/mermaid.rs::MMD_PLUGIN`,
`src/diagrams/tables.rs::TABLE_PLUGIN`, `src/diagrams/tables.rs::MD_PLUGIN`).
Adding a format = one module + one registry line. `src/diagram.rs::list_diagrams`
/ `get_diagram`; per-tool CLI overrides in `src/diagram.rs::ToolOverride` /
`src/diagram.rs::DiagramOptions` (format default dot; renderer default
Internal).

**run_diagram flow.** `src/diagram.rs::run_diagram`: shared file contract on
the FIRST module -> plugin.generate(module) pure text -> the --render svg
preconditions are validated UP FRONT for BOTH execution and --print-script:
--out FILE ending in .svg is required, and per-mode renderer availability
(external: the format must have an SVG tool - "use dot, puml or mmd";
internal: the list of formats that have one) -> --print-script renders via
`src/diagram.rs::render_script` (heredoc writing the source + a comment naming
the in-process crate in internal mode / the tool invocation in external mode,
plus a temp puppeteer-config step for mermaid) -> execution: the source is
written NEXT TO the SVG with the same stem (plantuml derives its output name
from the input, so this keeps every format's render target exactly at out),
then the renderer runs.

**Strict error semantics.** `src/diagrams/mod.rs::render_check` (user decision
2026-10-21): any failure - spawn error or non-zero exit - becomes Err with
diagnostic lines naming the tool and carrying its captured stderr; NO graceful
degradation (CI/CD requirement). plantuml exits 200 on a syntax error and
still writes an error SVG, so the gate is on the exit status, not file
presence.

**Plugins.** Shared label helpers in `src/diagrams/mod.rs` (event_label /
guard_label / emit_labels via the info renderers; states() = State-enum
variants in declaration order). dot: `src/diagrams/dot.rs::generate` (fixed
3-line header, node lines with filled/fillcolor IFF initial state, edge labels
Event [guard] -> next via `src/diagrams/dot.rs::transition_label`, dashed
on_error edges) + `src/diagrams/dot.rs::render_svg_internal` (layout-rs:
DotParser -> GraphBuilder -> VisualGraph::do_it -> SVGWriter). plantuml:
`src/diagrams/plantuml.rs::generate` (@startuml frame, [*] --> initial IFF
present, event \n guard labels with the literal two-char \n, one note on link
per emitted output - the BARE form because plantuml-core rejects the position
word, dashed bracketed on_error edges) + render_svg_internal (plantuml-core;
known gap: note-on-link text is not drawn). mermaid:
`src/diagrams/mermaid.rs::generate` (stateDiagram-v2, multi-line edge labels
carrying event/guard/emits, [on_error] prefix) + `src/diagrams/mermaid.rs::puppeteer_args`
/ `puppeteer_json` ({"args":["--no-sandbox","--disable-dev-shm-usage"]} - mmdc
as root cannot launch Chrome without --no-sandbox; overridable via
RFS_MERMAID_PUPPETEER_ARGS) + `src/diagrams/mermaid.rs::script_pup_path` (the
temp config step in scripts) + render_svg_internal (mermaid-rs-renderer
render_strict). tables: build -> a state x event transition matrix + variable/
event summaries; `src/diagrams/tables.rs::generate_table` /
`generate_md`; no SVG renderer in either mode.

All generators are deterministic: states in State-enum declaration order,
transitions in IR order, on_error edges last. Every format's output for pinned
specs is byte-pinned in tests/frontend_test.rs; cross-renderer syntax parity
(internal vs external - parse/render success only, never visual) lives in
tests/diagram_render_parity_test.rs.

Code: `src/diagram.rs::Renderer`, `src/diagram.rs::DiagramPlugin`,
`src/diagram.rs::DIAGRAMS`, `src/diagram.rs::list_diagrams`,
`src/diagram.rs::get_diagram`, `src/diagram.rs::ToolOverride`,
`src/diagram.rs::DiagramOptions`, `src/diagram.rs::run_diagram`,
`src/diagram.rs::render_script`, `src/diagrams/mod.rs::event_label`,
`src/diagrams/mod.rs::guard_label`, `src/diagrams/mod.rs::emit_labels`,
`src/diagrams/mod.rs::states`, `src/diagrams/mod.rs::render_check`,
`src/diagrams/dot.rs::DOT_PLUGIN`, `src/diagrams/dot.rs::generate`,
`src/diagrams/dot.rs::transition_label`, `src/diagrams/dot.rs::render_svg_internal`,
`src/diagrams/plantuml.rs::PUML_PLUGIN`, `src/diagrams/plantuml.rs::generate`,
`src/diagrams/plantuml.rs::render_svg_internal`, `src/diagrams/mermaid.rs::MMD_PLUGIN`,
`src/diagrams/mermaid.rs::generate`, `src/diagrams/mermaid.rs::puppeteer_args`,
`src/diagrams/mermaid.rs::puppeteer_json`, `src/diagrams/mermaid.rs::script_pup_path`,
`src/diagrams/mermaid.rs::render_svg_internal`, `src/diagrams/tables.rs::TABLE_PLUGIN`,
`src/diagrams/tables.rs::MD_PLUGIN`, `src/diagrams/tables.rs::generate_table`,
`src/diagrams/tables.rs::generate_md`, `tests/frontend_test.rs`,
`tests/diagram_render_parity_test.rs`


<a id="dev:cli-commands-ext-tool"></a>
## External-tool layer (`src/ext_tool.rs`)

**ExtTool.** `src/ext_tool.rs::ExtTool { name, env_path_var, env_args_var,
subcommand }`: the bare tool name is the PATH-lookup default and display name;
the fixed subcommand is inserted right after the resolved path, before the
extra args ("fuzz" for CARGO_FUZZ - cargo-fuzz is a SUBCOMMAND of cargo, so
the resolved path is the cargo binary; empty for plain tools). Seven presets:
RUSTC / GCC / DOT / PLANTUML / MERMAID (mmdc) / CLANG / CARGO_FUZZ with their
RFS_* env vars.

**Resolution.** `src/ext_tool.rs::ExtTool::resolve`: PATH precedence = CLI flag
> env var > bare name on PATH (empty strings are treated as unset); args = the
env *_ARGS value (whitespace-split) FIRST, then the CLI --*-args appended. The
result is a `src/ext_tool.rs::ResolvedTool { name, path, subcommand,
extra_args }` with display_name / render (the command line) / run / run_in /
run_in_without (env removal - how the RUSTUP_TOOLCHAIN strip works for
cargo-fuzz). `src/ext_tool.rs::ToolOutput { status, stdout, stderr }` +
success.

**Scripts.** `src/ext_tool.rs::sh_quote` (POSIX sh quoting) and
`src/ext_tool.rs::Script` - the deterministic POSIX sh builder behind every
--print-script: new(command_desc, spec) writes the shebang, set -e, and a
header comment with the rfs version + spec; with_tool records the RESOLVED
tool paths/args in the header; step(comment, cmdline) appends one commented
step; render emits the script. Self-contained: generated sources are embedded
in quoted heredocs by the callers (rfs test, rfs diagram --render svg);
output is byte-pinned in tests/frontend_test.rs.

Code: `src/ext_tool.rs::ExtTool`, `src/ext_tool.rs::ResolvedTool`,
`src/ext_tool.rs::ToolOutput`, `src/ext_tool.rs::sh_quote`,
`src/ext_tool.rs::Script`, `tests/frontend_test.rs`
