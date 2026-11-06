<!-- llm-budget: 400 -->
# RFS Task Map (Class B)

Functionality to files/functions for EVERY CLI command, subsystem, and test
suite; the "read this fragment for this task" index extending `START.md`
section 6.

**How to use.** Find your task type below -> read the named Class A fragment
(docs/dev/) -> locate code via the file::function references -> before
editing, check llm:change-impact for affected files/tests/fragments. The
`START.md` section 6 table stays the workflow authority (read/do/verify);
this map is the locator.

<a id="llm:task-map"></a>
## Map

### CLI commands (dispatch `src/main.rs::main`; deep dive dev:cli-commands-overview)

Shared infrastructure: external tools `src/ext_tool.rs::ExtTool` / `resolve`
/ `Script` (dev:cli-commands-ext-tool); logging `src/cli_log.rs::Logger` /
`LogLevel`; library API surface `src/lib.rs` (dev:cli-commands-library).

| Command | Read fragment | Files/functions |
|---|---|---|
| parse / validate | dev:cli-commands-frontend | `src/frontend.rs::run_conversion`, `plan_conversion`, `run_validate`, `run_validate_structured`, `render_validate_json`; dispatch `src/main.rs::cmd_parse` / `cmd_validate` |
| info | dev:cli-commands-info | `src/info.rs::run_info`, `render_text`, `render_json`, `monitor_status`; dispatch `src/main.rs::cmd_info` |
| info format | dev:cli-commands-info | `src/info_format.rs::FORMATS`, `find`, `list_markdown`, `render`; dispatch `src/main.rs::run_info_format` |
| test | dev:cli-commands-test | `src/test_cmd.rs::run_test`, `run_unit_suite`, `run_fuzz_c`, `run_fuzz_rust`, `render_script`; dispatch `src/main.rs::cmd_test` |
| diagram | dev:cli-commands-diagram | `src/diagram.rs::run_diagram`, `DIAGRAMS`, `DiagramPlugin`; plugins `src/diagrams/dot.rs::DOT_PLUGIN`, `src/diagrams/plantuml.rs::PUML_PLUGIN`, `src/diagrams/mermaid.rs::MMD_PLUGIN`, `src/diagrams/tables.rs::TABLE_PLUGIN` / `MD_PLUGIN`; dispatch `src/main.rs::cmd_diagram` |
| codegen | dev:codegen-plugin-api | `src/codegen/mod.rs::parse_codegen_args`, `get_backend`, `list_backends`, `CodegenCommandKind::run`; backends `src/codegen/backends/rust/mod.rs` / `src/codegen/backends/c/mod.rs`; dispatch `src/main.rs::cmd_codegen` |
| mc-ref | dev:cli-commands-mc-ref-gen-spec + dev:mc-engine-exact | `src/mc_ref_cmd.rs::run_mc_ref`, `exit_code_for`, `run_cli`; engine `src/ref_mc.rs::run` |
| gen-spec | dev:cli-commands-mc-ref-gen-spec | `src/gen_spec_cmd.rs::run_cli`, `generate_random`, `mutate_base`; engine `src/lang/specgen.rs::SpecGen`, `mutate_traced` |

### Frontend core (source -> IR -> validation)

| Concern | Read fragment | Files/functions |
|---|---|---|
| Parsing RFS v0 | dev:parser-overview (+ -grammar / -expressions / -entry-error / -ebnf-model) | `src/lang/parser.rs::parse_source`, `parse_file`; grammar model `src/ebnf_grammar.rs` (canonical text of DESIGN.md section 6) |
| Semantic validation | dev:validator-overview (+ -fatal-pass / -typing / -warnings) | `src/lang/validator.rs::validate_ir`, `validate_module`, `module_warnings` |
| IR + wire format | dev:ir-model, dev:ir-wire-format | `src/lang/ir.rs::Ir`, `to_json`, `from_json`; `IR_VERSION` read policy {0,1} (D-P5-2) |
| JSON reader/writer | dev:json-rw | `src/json/json_rw.rs::parse`, `Json` (hand-rolled; the only JSON impl in the toolchain) |
| JSON Schema emitter | dev:json-schema | `src/json/json_schema.rs` (draft-07 emitter + markdown renderer + sync walker, used by rfs info format) |
| Printing IR -> source | dev:code-map-frontend (file entry) | `src/lang/printer.rs::print_ir` (unparser; round-trip pinned in tests) |
| Spans / parse errors | dev:code-map-frontend (file entry) | `src/lang/error.rs::ParseError`, `Span` |
| Spec hash | dev:spec-hash | `src/utils/hash.rs` (hand-rolled SHA-256; generated artifact headers) |
| Shared LCG generator | dev:code-map-utils, dev:codegen-test-plan | `src/utils/rng.rs::Rng` (single RNG family: specgen + test planning + both generated runtimes) |

### Codegen (registry + shared plans + backends)

| Concern | Read fragment | Files/functions |
|---|---|---|
| Plugin API + registry | dev:codegen-plugin-api, dev:codegen-core-overview | `src/codegen/mod.rs::BACKENDS`, `get_backend`, `list_backends`, `parse_codegen_args`, `BackendOptions`, `CodegenCommandKind` |
| Production FSM helpers | dev:codegen-fsm-helpers | `src/codegen/fsm.rs` (variants, representative values shared by both backends) |
| MC concepts | dev:codegen-mc-concepts | `src/codegen/mc_concepts.rs` (ExactStatus taxonomy, EXACT_DEFAULT_* constants, report wire format) |
| Test planning | dev:codegen-test-plan | `src/codegen/test_plan.rs` (TestPlan shared by both backends + fuzz skeletons) |
| Rust backend | dev:backend-rust-overview (+ -plugin / -fsm / -mc / -test) | `src/codegen/backends/rust/mod.rs`, `rust_fsm.rs`, `rust_mc.rs`, `rust_test.rs` (aggregate crate: `generate_aggregate_test_crate`, `write_generated_crate`) |
| C backend | dev:backend-c-overview (+ -plugin / -fsm / -mc / -test) | `src/codegen/backends/c/mod.rs`, `c_fsm.rs`, `c_mc.rs`, `c_test.rs` (same wire format as Rust) |

### Model-checking engines + monitor compiler

| Concern | Read fragment | Files/functions |
|---|---|---|
| Codegen-time plan | dev:mc-engine-plan | `src/mc_plan.rs::McPlan::compute` (pure function of module + assumption selection; config encoding = the visited-set buffer) |
| Exact bounded checker (reference) | dev:mc-engine-exact | `src/ref_mc.rs::run`, `render_text`, `to_json_report` (ground truth, D-P6-5) |
| Liveness analysis | dev:mc-engine-liveness | `src/ref_mc.rs` (position graph, product automaton, SCCs, lasso witness) |
| Counterexample verification | dev:mc-engine-verify | `src/ref_mc.rs::verify_counterexample`, minimal violation labels |
| SMT bounded mode | dev:mc-engine-smt | `src/smt_plan.rs` (k-step unrolling spec), `src/smt_emit.rs` (Rust z3 + C libz3 emitter) |
| PSL -> monitor compiler | dev:monitor-compiler-overview (+ -signals / -nodes / -compile / -reference) | `src/codegen/monitor.rs::compile_formula`, `MonitorPlan`, `ReferenceMonitor` |

### Workspace crates (deep dive dev:workspace-crates-overview)

| Concern | Read fragment | Files/functions |
|---|---|---|
| build.rs helper | dev:workspace-crates-build | `crates/rfs-build/src/lib.rs::RfsBuild`, `resolve_rfs_bin` (RFS_BIN > PATH); real end-to-end in `crates/rfs-build/tests/integration.rs` |
| ProtocolFsm + adapters | dev:workspace-crates-adapters | `crates/rfs-adapters/src/lib.rs::ProtocolFsm`; `minimal.rs::Minimal`, `callback.rs::drive`, `channel.rs::ChannelFsm`, `async_adapter.rs::AsyncFsm`; tests `crates/rfs-adapters/tests/adapters.rs` (mock FSM), `crates/rfs-adapters/tests/nostd_minimal.rs` (no_std compile checks) |
| Examples (Rust + C) | dev:workspace-crates-examples | `crates/examples/sync_handler/src/lib.rs::run`, `crates/examples/async_handler/src/lib.rs::run` (same scripted ARQ scenario, pinned ArqReport); `crates/examples/fuzz_rust/build.rs` (adapted fuzz skeleton + detached cargo-fuzz workspace); C projects c_sync_handler / fuzz_c are Makefile-driven, NOT cargo members |

### Fuzz + training + code-test harness (deep dive dev:toolchain-fuzz-overview)

| Concern | Read fragment | Files/functions |
|---|---|---|
| Fast fuzz layer | dev:toolchain-fuzz-targets | `fuzz/fuzz_targets/common.rs::RandSpec` (+ `HEAD_LEN`, `split_head_rest`, `rand_spec`); targets `parser.rs`, `validator.rs`, `ir_roundtrip.rs`, `codegen.rs` (detached cargo-fuzz workspace; nightly) |
| Slow CI fuzz layer | dev:toolchain-fuzz-targets | `tests/toolchain_fuzz_test.rs` (seeded; compiles all six artifacts; ref_mc ground truth) |
| Training pipeline | dev:toolchain-fuzz-training | `src/training.rs::aggregate_training_sets`, `validate_training_line`, `extract_category_snippets`; bins `src/bin/build_training.rs`, `src/bin/extract_training_snippets.rs` |
| Generated-code harness | dev:toolchain-fuzz-code-tests | `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs` (aggregate crate under target/rfs_codegen_tests) |

## Test suites (what each pins)

| Suite | Pins | Related fragments |
|---|---|---|
| tests/examples_test.rs | file-driven valid/invalid example suite (adding examples needs no test-code change) | dev:parser-overview, dev:validator-overview |
| tests/frontend_test.rs | parse/validate/diagram/test-script byte pins + .err side files | dev:cli-commands-frontend / -diagram / -test |
| tests/info_format_test.rs | D-P7-9 sync walk: real tool output vs the embedded format models | dev:cli-commands-info |
| tests/codegen_test.rs | per-command generation (all backends x commands) | dev:codegen-plugin-api |
| tests/backend_parity_test.rs | forced-violation parity + production-FSM cross-backend parity | dev:backend-rust-overview, dev:backend-c-overview |
| tests/prod_parity_test.rs | production-FSM cross-backend byte parity | dev:backend-rust-fsm, dev:backend-c-fsm |
| tests/benchmark_mc_test.rs | MC benchmark specs 43/44/45 + 47/48/49 (peak_rss_kb / ce_steps) | dev:mc-engine-exact |
| tests/liveness_mc_test.rs | liveness eventually/until + lasso counterexamples | dev:mc-engine-liveness |
| tests/smt_bounded_test.rs | SMT bounded mode (feature-gated) | dev:mc-engine-smt |
| tests/ref_mc_sweep_test.rs | ref-MC byte-parity sweep | dev:mc-engine-exact |
| tests/ref_mc_diag_test.rs | ref-MC diagnostic companion | dev:mc-engine-exact |
| tests/ref_mc_diff_test.rs | ref-MC vs generated-MC differential (no false pass) | dev:mc-engine-verify |
| tests/monitor_integration_test.rs | monitor compiler integration | dev:monitor-compiler-overview |
| tests/monitor_ref_test.rs | hand-written monitor references | dev:monitor-compiler-reference |
| tests/psl_translation_test.rs | PSL translation pins | dev:monitor-compiler-compile |
| tests/wide_ops_parity_test.rs | wide-ops cross-backend pins | dev:backend-rust-overview, dev:backend-c-overview |
| tests/diagram_render_parity_test.rs | internal vs external renderer syntax parity | dev:cli-commands-diagram |
| tests/example_projects_test.rs | workspace examples + C projects (wall-clean gcc, scenario parity, fuzz layouts) | dev:workspace-crates-examples |
| tests/integration_example_test.rs | Phase 2 exit criterion: realistic endpoint protocol -> production Rust + mpsc driver | dev:backend-rust-fsm |
| tests/toolchain_fuzz_test.rs | slow fuzz layer (seeded compile-and-run vs ref_mc) | dev:toolchain-fuzz-targets |
| tests/training_test.rs | training tree: JSONL validity, snippet classification, aggregate sum | dev:toolchain-fuzz-training |
| tests/dev_docs_test.rs | the doc sync gate (this tracker; manifest-driven) | dev:index |
| tests/docs_md_test.rs | mdr-stack doc-quality gate over docs/user/ | dev:index |
| tests/library_comparison.rs | parser-library comparison (nom vs chumsky smoke) - records the decision | dev:parser-overview |

## Coverage (all 71 manifest files)

Sync-gate requirement: every source file listed in tests/dev_docs_manifest.txt
must appear below as a doc→code reference (tests/dev_docs_test.rs enforces
this). One-line role per file; for functionality-level pointers use the Map
above.

- `src/lib.rs` - crate root: module tree, public API surface
- `src/main.rs` - CLI entry point: argument parsing and command dispatch
- `src/lang/error.rs` - shared source spans and parse-error types
- `src/cli_log.rs` - minimal stderr logging for the new CLI commands
- `src/frontend.rs` - frontend CLI logic: RFS/JSON conversion, validation dispatch
- `src/lang/printer.rs` - IR-to-RFS source printer (unparser) and round-trip helpers
- `src/lang/parser.rs` - RFS v0 source parser (nom-based): spec text to IR
- `src/ebnf_grammar.rs` - EBNF grammar model + renderers for IDE/tooling output
- `src/lang/validator.rs` - semantic validation and expression typing for RFS IR
- `src/lang/mod.rs` - module root of the language core (parser + error + ir + printer + validator + specgen)
- `src/lang/ir.rs` - IR types + hand-written JSON (de)serialization (pinned wire format)
- `src/json/mod.rs` - module root of the JSON substrate (json_rw + json_schema)
- `src/json/json_rw.rs` - hand-rolled JSON reader/writer (only JSON impl in the toolchain)
- `src/json/json_schema.rs` - draft-07 JSON-Schema emitter + markdown renderer + sync walker
- `src/utils/mod.rs` - module root of the cross-cutting substrate (hash + rng)
- `src/utils/rng.rs` - the project's single LCG generator (shared constants, top-53-bit mapping)
- `src/codegen/mod.rs` - codegen backend API + plugin registry (language-agnostic core)
- `src/codegen/ident.rs` - identifier mangling shared by both backends (snake, rust_ident, rust_binding_ident)
- `src/codegen/fsm.rs` - shared production-FSM codegen helpers (variants, rep values)
- `src/codegen/mc_concepts.rs` - bounded model-checker concepts shared by all backends
- `src/codegen/test_plan.rs` - unit-test planning (TestPlan) shared by all backends
- `src/codegen/backends/mod.rs` - backend plugin modules: registration entries per target language
- `src/codegen/backends/rust/mod.rs` - Rust backend: plugin registration + shared Rust helpers
- `src/codegen/backends/rust/fsm.rs` - Rust backend: production FSM source generation
- `src/codegen/backends/rust/mc.rs` - Rust backend: model-checker executable generation
- `src/codegen/backends/rust/test.rs` - Rust backend: unit-test / fuzz / aggregate-crate generation
- `src/codegen/backends/c/mod.rs` - C backend: plugin registration + shared C helpers
- `src/codegen/backends/c/fsm.rs` - C backend: production FSM source generation
- `src/codegen/backends/c/mc.rs` - C backend: model-checker executable generation (same wire format as Rust)
- `src/codegen/backends/c/test.rs` - C backend: unit-test / fuzz-target generation
- `src/mc_plan.rs` - codegen-time plan for the exact MC engine (embedded in both binaries)
- `src/smt_plan.rs` - backend-neutral SMT spec for k-step unrolling over z3 bitvectors
- `src/smt_emit.rs` - shared SMT emitter: renders the spec into Rust (z3 crate) and C (libz3)
- `src/ref_mc.rs` - brute-force reference model checker (ground truth, rfs mc-ref)
- `src/codegen/monitor.rs` - PSL-to-monitor compiler: formulas to MonitorPlan
- `src/info.rs` - rfs info: static spec introspection
- `src/info_format.rs` - rfs info format: machine-readable input/output format descriptions
- `src/gen_spec_cmd.rs` - rfs gen-spec: write a random spec (two modes)
- `src/test_cmd.rs` - rfs test: generate + compile + run test suites in one invocation
- `src/mc_ref_cmd.rs` - rfs mc-ref: run the reference model checker in-process
- `src/diagram.rs` - rfs diagram: plugin-based state-diagram/table generation
- `src/diagrams/mod.rs` - diagram format plugins: registration entries per format
- `src/diagrams/dot.rs` - Graphviz DOT state diagrams (pure-Rust generation)
- `src/diagrams/mermaid.rs` - Mermaid state diagrams (verified syntax subset)
- `src/diagrams/plantuml.rs` - PlantUML state diagrams (verified syntax subset)
- `src/diagrams/tables.rs` - transition tables + variable/event summaries (table/md formats)
- `src/lang/specgen.rs` - seeded random spec generator + incremental mutation
- `src/ext_tool.rs` - external-tool resolution, invocation, deterministic script emission
- `crates/rfs-build/src/lib.rs` - rfs-build: the build.rs helper API for RFS codegen
- `crates/rfs-build/tests/integration.rs` - rfs-build end-to-end test against a real spec
- `crates/rfs-adapters/src/lib.rs` - rfs-adapters root: ProtocolFsm trait + generic adapter core
- `crates/rfs-adapters/src/async_adapter.rs` - futures-based async adapter (submit inputs, await outputs)
- `crates/rfs-adapters/src/callback.rs` - callback-based adapter: driving loop with per-event callbacks
- `crates/rfs-adapters/src/channel.rs` - sync channel-based adapter: FSM on its own thread
- `crates/rfs-adapters/src/minimal.rs` - no_std-minimal adapter: pure step-forwarding surface
- `crates/rfs-adapters/tests/adapters.rs` - adapter tests over an in-crate mock FSM
- `crates/rfs-adapters/tests/nostd_minimal.rs` - no_std-minimal compile checks
- `crates/examples/async_handler/build.rs` - example build script: spec-43 ARQ sender via rfs-build
- `crates/examples/async_handler/src/lib.rs` - async_handler example lib (generated sliding-window FSM)
- `crates/examples/async_handler/src/main.rs` - async_handler demo binary (futures stream)
- `crates/examples/sync_handler/build.rs` - example build script: spec-43 ARQ sender via rfs-build
- `crates/examples/sync_handler/src/lib.rs` - sync_handler example lib (generated sliding-window FSM)
- `crates/examples/sync_handler/src/main.rs` - sync_handler demo binary (std mpsc channels)
- `crates/examples/fuzz_rust/build.rs` - example build script: ARQ sender + libFuzzer skeleton wiring
- `crates/examples/fuzz_rust/src/lib.rs` - fuzz_rust root crate: generated spec-43 ARQ body
- `fuzz/fuzz_targets/parser.rs` - fuzz target: the RFS v0 parser
- `fuzz/fuzz_targets/validator.rs` - fuzz target: semantic validation + IR JSON deserialization
- `fuzz/fuzz_targets/ir_roundtrip.rs` - fuzz target: IR JSON serialization stability
- `fuzz/fuzz_targets/codegen.rs` - fuzz target: the code generators
- `fuzz/fuzz_targets/common.rs` - shared helpers for the toolchain fuzz targets
- `src/training.rs` - training-set utilities (valid/invalid snippet extraction)
- `src/bin/build_training.rs` - CLI entry: build the training dataset
- `src/bin/extract_training_snippets.rs` - CLI entry: extract training snippets from examples
- `src/bin/generate_code_tests.rs` - CLI entry: generate code tests for all valid examples
- `src/bin/test_generated_code.rs` - CLI entry: compile + run the generated code tests
