# RFS Code Map

File-by-file map of the 71 hand-written source files: `src/` (49), `crates/`
(17), `fuzz/fuzz_targets/` (5). Each entry: responsibility, key items, doc
links both ways. The pointer scope is driven by `tests/dev_docs_manifest.txt`.

<a id="dev:code-map-frontend"></a>
## Frontend area - root + parser + validator/IR (5 files)

### Root (4)

- `src/lib.rs` - crate root: declares the 30-module tree and re-exports the
  public API (`src/lang/parser.rs::parse_source`, `src/lang/parser.rs::parse_file`,
  `src/lang/printer.rs::print_ir`, `src/lang/ir.rs::Ir`, `src/lang/error.rs::ParseError`,
  `src/lang/error.rs::Span`, plus the frontend conversion/validation surface).
  Co-owner of all cross-cutting fragments (`dev:index`, `llm:index`,
  architecture, decisions, task map, change impact).
- `src/main.rs` - CLI entry point: argument parsing and dispatch to the eight
  commands (`parse`, `validate`, `info`, `test`, `diagram`, `codegen`,
  `mc-ref`, `gen-spec`) + usage text; key item `src/main.rs::main`. Owner of
  `dev:code-map-cli`.
- `src/cli_log.rs` - minimal stderr logging for the CLI commands:
  `src/cli_log.rs::LogLevel` + `src/cli_log.rs::Logger`.
- `src/frontend.rs` - frontend CLI logic behind `rfs parse` / `rfs validate`:
  format detection (`src/frontend.rs::Format`), file collection, conversion
  through the IR (`src/frontend.rs::convert_file_to_ir`,
  `src/frontend.rs::render_ir`, `src/frontend.rs::run_conversion`) and
  validation dispatch (`src/frontend.rs::run_validate`,
  `src/frontend.rs::validate_one`).
### Parser (1)

- `src/ebnf_grammar.rs` - EBNF grammar model + renderers for IDE/tooling
  output: `src/ebnf_grammar.rs::RuleNode`, `to_ebnf_text`, `to_md`,
  `to_plantuml_ebnf`, `to_svg`.

<a id="dev:code-map-lang"></a>
## Language core (`src/lang/`) (7 files)

- `src/lang/mod.rs` - module root of the language core (parse -> IR ->
  validate -> print + the spec generator).
- `src/lang/parser.rs` - RFS v0 source parser (nom-based): spec text to IR with
  spans; entry points `src/lang/parser.rs::parse_source` /
  `src/lang/parser.rs::parse_file`, `src/lang/parser.rs::KEYWORDS` table. Deep
  dive: `dev:parser-overview` / `llm:parser`.
- `src/lang/error.rs` - shared source spans and parse-error types:
  `src/lang/error.rs::Span` (byte ranges attached to every IR node) and
  `src/lang/error.rs::ParseError`.
- `src/lang/ir.rs` - IR types + pinned JSON wire format: `src/lang/ir.rs::Ir`,
  `src/lang/ir.rs::Module`, `src/lang/ir.rs::IR_VERSION`. Deep dive:
  `dev:ir-json-rw-overview` / `llm:ir-json-rw`.
- `src/lang/printer.rs` - IR-to-RFS source printer (unparser):
  `src/lang/printer.rs::print_ir`, `print_module`, `print_expr`,
  `print_formula`, plus round-trip helpers `roundtrip_rfs` / `roundtrip_json`.
- `src/lang/validator.rs` - semantic validation and expression typing for RFS
  IR: `src/lang/validator.rs::validate_ir` returns the full error list
  (`src/lang/validator.rs::ValidationError`), expression types via
  `src/lang/validator.rs::ExprType`. Deep dive: `dev:validator-overview` /
  `llm:validator`.
- `src/lang/specgen.rs` - seeded random spec generator + incremental mutation:
  `SpecGen`, `Bounds`, `mutate_traced` (`MutOp`); draws through the shared
  `src/utils/rng.rs::Rng`.

<a id="dev:code-map-json"></a>
## JSON substrate (`src/json/`) (3 files)

- `src/json/mod.rs` - module root of the JSON substrate (I/O + schema emitter).
- `src/json/json_rw.rs` - hand-rolled JSON reader/writer (the only JSON impl in
  the toolchain): ordered objects, byte-identical output contract;
  `src/json/json_rw.rs::Json`, `src/json/json_rw.rs::parse`.
- `src/json/json_schema.rs` - draft-07 JSON-Schema emitter + markdown renderer +
  sync walker for the IR wire format: `src/json/json_schema.rs::SchemaDoc`.

<a id="dev:code-map-codegen"></a>
## Codegen core + backends (19 files)

- `src/codegen/backends/mod.rs` - backend plugin modules: registration entries per
  target language.

### Codegen core (5)

- `src/codegen/mod.rs` - codegen backend API + plugin registry: the `Backend`
  trait (name, description, commands), the static registry
  `src/codegen/mod.rs::BACKENDS`, lookup `src/codegen/mod.rs::get_backend`, shared
  option/output types and command argument parsing. Owner of
  `dev:code-map-codegen`. Deep dive: `dev:codegen-core-overview` /
  `llm:codegen-core`.
- `src/codegen/fsm.rs` - shared production-FSM codegen helpers: variant
  computation `src/codegen/fsm.rs::build_variants` (`VariantInfo`,
  `FsmVariants`), representative values `representative_values`
  (`RepValue`), liveness detection `contains_liveness`.
- `src/codegen/mc_concepts.rs` - bounded model-checker concepts shared by all
  backends: `McConfig`, violation reporting (`ViolationSpec`, `McPhase`,
  `McStatus`, `ExactStatus`), assume-selection parsing
  `parse_assume_selection`.
- `src/codegen/ident.rs` - identifier-mangling utilities shared by BOTH backends
  (`snake`, `rust_ident`, `rust_binding_ident` + the Rust keyword table) - the
  neutral home that resolves the former C->Rust-backend coupling.
- `src/codegen/test_plan.rs` - unit-test planning shared by all backends:
  `TestPlan`, input/output cases (`InputCase`, `OutputCase`, `VarValue`),
  transition expectations (`TransitionExpectation`, `OnCase`).

### Rust backend (4)

- `src/codegen/backends/rust/mod.rs` - Rust backend: plugin registration
  (`src/codegen/backends/rust/mod.rs::RUST_BACKEND` + command info
  `PRODUCTION_INFO`, `TEST_INFO`, `MC_INFO`) and shared Rust helpers. Deep dive:
  `dev:backend-rust-overview` / `llm:backend-rust`.
- `src/codegen/backends/rust/fsm.rs` - Rust backend: production FSM source
  generation (`src/codegen/backends/rust/fsm.rs::generate_production`).
- `src/codegen/backends/rust/mc.rs` - Rust backend: model-checker executable
  generation (`src/codegen/backends/rust/mc.rs::generate_model_checker`).
- `src/codegen/backends/rust/test.rs` - Rust backend: unit-test / fuzz /
  aggregate-crate generation (`generate_unit_tests`, `emit_fuzz_target`,
  `generate_aggregate_test_crate`).

### C backend (4)

- `src/codegen/backends/c/mod.rs` - C backend: plugin registration
  (`src/codegen/backends/c/mod.rs::C_BACKEND`) and shared C helpers. Deep dive:
  `dev:backend-c-overview` / `llm:backend-c`.
- `src/codegen/backends/c/fsm.rs` - C backend: production FSM source generation
  (`src/codegen/backends/c/fsm.rs::generate_production`).
- `src/codegen/backends/c/mc.rs` - C backend: model-checker executable
  generation (`src/codegen/backends/c/mc.rs::generate_model_checker`) - same
  runtime wire format as the Rust checker.
- `src/codegen/backends/c/test.rs` - C backend: unit-test / fuzz-target
  generation (`generate_unit_tests`, `emit_fuzz_target_c`).

### MC engine (4)

- `src/mc_plan.rs` - codegen-time plan for the exact MC engine, computed once
  and embedded in both backends' binaries: `src/mc_plan.rs::McPlan`. Deep
  dive: `dev:mc-engine-overview` / `llm:mc-engine`.
- `src/smt_plan.rs` - backend-neutral SMT spec for k-step unrolling over z3
  bitvectors: `SmtSpec`, expression types `Se`, `Sb`, `Sf`, variables
  `SmtVar`.
- `src/smt_emit.rs` - shared SMT emitter: renders the spec into Rust (z3
  crate) and C (libz3): `emit_rust_spec`, `emit_c_spec`.
- `src/ref_mc.rs` - brute-force reference model checker, the ground truth
  behind `rfs mc-ref`: `src/ref_mc.rs::run` with `RefConfig` / `RefResult`;
  independent of the generated checkers (shares only IR + report concepts).
- `src/codegen/monitor.rs` - PSL-to-monitor compiler: formulas to
  `src/codegen/monitor.rs::MonitorPlan` via `src/codegen/monitor.rs::compile_formula`, with
  an unsupported-formula fallback. Deep dive:
  `dev:monitor-compiler-overview` / `llm:monitor-compiler`.

<a id="dev:code-map-cli"></a>
## CLI + diagrams area (18 files)

### CLI commands (7)

- `src/info.rs` - `rfs info`: static spec introspection: `run_info` with text
  / JSON rendering (`render_text`, `render_json`). Deep dive:
  `dev:cli-commands-overview` / `llm:cli-commands`.
- `src/info_format.rs` - `rfs info format`: machine-readable input/output
  format descriptions: `FormatInfo`, embedded draft-07 schemas
  (`spec_json_schema`, `ce_trace_schema`, `mc_report_schema`) + the
  counterexample-model description (`counterexample_model`).
- `src/gen_spec_cmd.rs` - `rfs gen-spec`: write a random spec (two modes):
  `generate_random`, `mutate_base`, CLI `run_cli`.
- `src/test_cmd.rs` - `rfs test`: generate + compile + run test suites in one
  invocation: `TestSuite`, `TestOptions`, `run_test`, outcome `TestOutcome`;
  compiles via `src/ext_tool.rs::ExtTool` (rustc/gcc).
- `src/mc_ref_cmd.rs` - `rfs mc-ref`: run the reference model checker
  in-process: `run_mc_ref`, exit-code mapping `exit_code_for`, CLI `run_cli`.
- `src/ext_tool.rs` - external-tool resolution, invocation, deterministic
  script emission: `ExtTool`, `ResolvedTool`, `Script`, `sh_quote`.

### Diagrams (6)

- `src/diagram.rs` - `rfs diagram`: plugin-based state-diagram/table
  generation: `DiagramPlugin`, registry `DIAGRAMS`, render planning
  `RenderPlan`, `list_diagrams`.
- `src/diagrams/mod.rs` - diagram format plugins: registration entries per
  format + shared label helpers (`event_label`, `guard_label`,
  `emit_labels`, `states`).
- `src/diagrams/dot.rs` - Graphviz DOT state diagrams (pure-Rust generation):
  `generate`, external rendering `plan_render` / `render_svg`.
- `src/diagrams/mermaid.rs` - Mermaid state diagrams (verified syntax subset):
  `generate`, mmdc/Puppeteer argument handling (`puppeteer_args`,
  `DEFAULT_PUPPETEER_ARGS`), external rendering `plan_render` /
  `render_svg`.
- `src/diagrams/plantuml.rs` - PlantUML state diagrams (verified syntax
  subset): `generate`, external `plan_render` / `render_svg` + in-process
  `render_svg_internal`.
- `src/diagrams/tables.rs` - transition tables + variable/event summaries:
  `generate_table`, `generate_md`, plugins `TABLE_PLUGIN` / `MD_PLUGIN`.

### Bins + training (5)

- `src/training.rs` - training-set utilities (valid/invalid snippet
  extraction): `SnippetCounts`, category discovery (`training_root`,
  `category_dirs`, `category_jsonl_path`), JSONL I/O. Deep dive:
  `dev:toolchain-fuzz-overview` / `llm:toolchain-fuzz`.
- `src/bin/build_training.rs` - CLI entry: build the training dataset.
- `src/bin/extract_training_snippets.rs` - CLI entry: extract the training
  snippets from the examples.
- `src/bin/generate_code_tests.rs` - CLI entry: generate code tests for all
  valid examples (via `rfs::codegen` + the Rust test backend).
- `src/bin/test_generated_code.rs` - CLI entry: compile + run the generated
  code tests.

<a id="dev:code-map-utils"></a>
## Cross-cutting substrate (`src/utils/`) (3 files)

- `src/utils/hash.rs` - hand-rolled SHA-256 + `spec_hash16`, the D-P6-3 stamp in
  generated artifact headers; leaf primitive with no component of its own.
- `src/utils/rng.rs` - the project's single LCG generator (`Rng` + the shared
  `LCG_MULT` / `LCG_ADD` constants): specgen, test planning and both generated
  fuzz runtimes stay in one RNG family by construction (algorithm documented in
  the unit-test-planning fragment).

<a id="dev:code-map-workspace"></a>
## Workspace crates + fuzz (23 files)

### rfs-build (2)

- `crates/rfs-build/src/lib.rs` - build.rs helper for RFS: drives `.rfs`
  compilation from dependent crates' build scripts:
  `crates/rfs-build/src/lib.rs::RfsBuild` (`new`, `backend`, `compile`), tool
  resolution `resolve_rfs_bin`, errors `RfsBuildError`. Owner of
  `dev:code-map-workspace`. Deep dive:
  `dev:workspace-crates-overview` / `llm:workspace-crates`.
- `crates/rfs-build/tests/integration.rs` - end-to-end test running
  `RfsBuild` against a real spec with the workspace toolchain.

### rfs-adapters (7)

- `crates/rfs-adapters/src/lib.rs` - adapter layer root: the
  `crates/rfs-adapters/src/lib.rs::ProtocolFsm` trait (associated types
  `Input` / `Output` / `Error`, `new()`, `step()`) that generated FSMs
  implement.
- `crates/rfs-adapters/src/async_adapter.rs` - futures-based async adapter:
  `crates/rfs-adapters/src/async_adapter.rs::AsyncFsm`, outcomes
  `AsyncOutcome`.
- `crates/rfs-adapters/src/callback.rs` - callback-based adapter: driving
  loop `crates/rfs-adapters/src/callback.rs::drive`, results
  `CallbackResult`.
- `crates/rfs-adapters/src/channel.rs` - sync channel-based adapter (FSM on
  its own thread): `crates/rfs-adapters/src/channel.rs::ChannelFsm`, outcomes
  `ChannelOutcome`.
- `crates/rfs-adapters/src/minimal.rs` - no_std-minimal adapter: pure
  step-forwarding wrapper `crates/rfs-adapters/src/minimal.rs::Minimal`.
- `crates/rfs-adapters/tests/adapters.rs` - adapter tests over a small
  in-crate mock FSM (no generated code needed).
- `crates/rfs-adapters/tests/nostd_minimal.rs` - no_std-minimal compile
  checks (the Phase 5 no_std exit criterion).

### Examples (8)

- `crates/examples/async_handler/build.rs` - build script: generates the
  spec-43 ARQ sender at build time via rfs-build.
- `crates/examples/async_handler/src/lib.rs` - example lib: `include!`s the
  generated sliding-window FSM body from OUT_DIR.
- `crates/examples/async_handler/src/main.rs` - demo binary: one ARQ session
  over the futures stream.
- `crates/examples/sync_handler/build.rs` - build script: generates the
  spec-43 ARQ sender at build time via rfs-build.
- `crates/examples/sync_handler/src/lib.rs` - example lib: `include!`s the
  generated sliding-window FSM body from OUT_DIR.
- `crates/examples/sync_handler/src/main.rs` - demo binary: one ARQ session
  over std mpsc channels.
- `crates/examples/fuzz_rust/build.rs` - build script: generates the spec-43
  ARQ sender for the lib AND wires the emitted libFuzzer skeleton into the
  cargo-fuzz layout; the ARQ fuzz-target skeleton it produces is git-ignored
  (a build artifact, not a mapped file).
- `crates/examples/fuzz_rust/src/lib.rs` - example lib: `include!`s the
  generated ARQ body from OUT_DIR.

### Fuzz targets (5)

- `fuzz/fuzz_targets/parser.rs` - fuzz target: the RFS v0 parser. Deep dive:
  `dev:toolchain-fuzz-overview` / `llm:toolchain-fuzz`.
- `fuzz/fuzz_targets/validator.rs` - fuzz target: semantic validation + IR
  JSON deserialization.
- `fuzz/fuzz_targets/ir_roundtrip.rs` - fuzz target: IR JSON serialization
  stability.
- `fuzz/fuzz_targets/codegen.rs` - fuzz target: the code generators.
- `fuzz/fuzz_targets/common.rs` - shared helpers for the toolchain fuzz
  targets.
