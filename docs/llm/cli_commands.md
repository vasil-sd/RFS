<!-- llm-budget: 200 -->
# RFS CLI Commands + Diagrams (Class B)

Compact card for LLM assistants working on CLI commands, diagram formats, or
external-tool handling.

<a id="llm:cli-commands"></a>
## Card

**Responsibility.** The `rfs` CLI: eight subcommands dispatched from
`src/main.rs::main` (parse / validate / info [+ info format] / test /
diagram / codegen / mc-ref / gen-spec), plus the library API surface in
`src/lib.rs` that integration tests and external tooling call directly.
Argument parsing is manual (no clap); generated text goes to stdout via
print! so redirected bytes are exact.

**Command-to-module map.**

| Command | Logic entry | Deep-dive fragment |
|---|---|---|
| parse / validate | `src/cli/frontend.rs::run_conversion` / `run_validate` (+ `run_validate_structured` for --json) | dev:cli-commands-frontend |
| info / info format | `src/cli/commands/info.rs::run_info` / `src/cli/info_format/mod.rs::FORMATS` | dev:cli-commands-info |
| test | `src/cli/commands/test.rs::run_test` | dev:cli-commands-test |
| diagram | `src/cli/commands/diagram/mod.rs::run_diagram` + plugins in `src/cli/commands/diagram/` | dev:cli-commands-diagram |
| codegen | `src/codegen/mod.rs` registry dispatch (via `src/main.rs::cmd_codegen`) | docs/dev/codegen_core.md |
| mc-ref | `src/cli/commands/mc_ref.rs::run_mc_ref` over `src/ref_mc.rs` in-process | dev:cli-commands-mc-ref-gen-spec |
| gen-spec | `src/cli/commands/gen_spec.rs::run_cli` over `src/lang/specgen.rs` | dev:cli-commands-mc-ref-gen-spec |

**Key items.**

| Item | Role |
|---|---|
| `src/main.rs::usage` / `main` | full usage text (printed by --help and on unknown/missing subcommand); the dispatch match - mc-ref/gen-spec delegate to their run_cli shells |
| `src/cli/frontend.rs::plan_conversion` | parse/validate mapping rules: extension replaces (never stacks), recursive dir scan, collision guard on normalized absolute paths; per-file errors collected in directory runs |
| `src/cli/frontend.rs::render_validate_json` | the PINNED --json document shape (file = one object, directory = array of failing files); semantic errors pinned to line/col 0/0 |
| `src/cli/info_format/mod.rs::FORMATS` | ten self-describing-format entries (D-P7-6); D-P7-9 sync tests walk REAL tool output against the embedded models - drift caught in both directions |
| `src/cli/commands/test.rs::run_test` | load+validate -> codegen via registry -> sequential suites (every SELECTED suite runs; failures joined); --print-script = deterministic POSIX sh, no execution |
| `src/cli/commands/test.rs::run_fuzz_rust` / `run_fuzz_c` | cargo-fuzz scaffold (nightly pin, RUSTUP_TOOLCHAIN stripped) vs clang -fsanitize=fuzzer,address over the skeleton ONLY (#include pattern); ASAN_OPTIONS=detect_leaks=0 for C |
| `src/cli/commands/mc_ref.rs::exit_code_for` | mirrors the generated exact binaries: 0 pass-complete / 1 fail / 2 unknown-state-limit; NOT-applicable flags documented in usage, not silently accepted |
| `src/lang/specgen.rs::SpecGen` / `mutate_traced` | validity by construction (raw IR types deliberately NOT Arbitrary); mutation commits only if the candidate validates (100 redraws/step then skip); same LCG constants as the fuzz wire format |
| `src/cli/commands/diagram/mod.rs::DiagramPlugin` / `DIAGRAMS` | five plugins (dot/puml/mmd/table/md): generate is pure-Rust deterministic text; adding a format = one module + one registry line |
| `src/cli/commands/diagram/mod.rs::render_check` | STRICT error semantics: any spawn error or non-zero exit -> Err with the tool's stderr; NO graceful degradation (plantuml exits 200 on syntax errors and still writes an SVG - gate on exit status) |
| `src/cli/ext_tool.rs::ExtTool::resolve` | PATH precedence CLI flag > env var > bare name; args = env *_ARGS first, then --*-args; seven presets incl. CARGO_FUZZ (subcommand of cargo) |

**Invariants that must not break.**

- Exit codes: 0 success; 1 ANY error (usage, parse, validation, IO, missing
  tool for a SELECTED suite). Generated MC binaries keep their own 0/1/2
  taxonomy (dev:mc-engine-exact); rfs mc-ref mirrors it.
- --print-script and execution must AGREE: every precondition (tool
  availability, renderer support) is validated UP FRONT in both modes;
  scripts embed resolved tool paths + generated sources in quoted heredocs
  and are byte-pinned in tests/frontend_test.rs.
- Determinism: diagram generators (states in State-enum order, transitions in
  IR order, on_error last), gen-spec from the seed (same args =>
  byte-identical file), info --json key order with all keys always present.
- File-input contract shared by info/test/diagram/mc-ref: extension selects
  format, spec loaded + validated before any work, single-module commands run
  on the FIRST module; a directory is an error for those commands.
- External-tool strictness (render_check + unit/fuzz suites): failures are
  never downgraded to warnings; missing tools give diagnostics naming the
  RFS_* env var / CLI flag that fixes them.

**Change impact.** A new subcommand = main.rs dispatch line + usage text +
this card's map + a dev-doc fragment with //! Docs: owners (ORPHAN RULE -
dev_docs_test fails on unowned fragments). A new diagram format = one module
+ DIAGRAMS line + byte-pins. Verify with cargo test --test dev_docs_test +
--test docs_md_test + tests/frontend_test.rs, then the full BUILD.md section
5 checklist. Full table: llm:change-impact.

**Deep dive.** dev:cli-commands-overview + dev:cli-commands-dispatch /
dev:cli-commands-library / dev:cli-commands-frontend / dev:cli-commands-info
/ dev:cli-commands-test / dev:cli-commands-mc-ref-gen-spec /
dev:cli-commands-diagram / dev:cli-commands-ext-tool in
docs/dev/cli_commands.md; codegen command registry: docs/dev/codegen_core.md;
MC engines: dev:mc-engine-overview; CLI behavior rules at file granularity:
KB.md section 4.5.
