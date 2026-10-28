<!-- llm-budget: 400 -->
# RFS Task Map (Class B)

Functionality to files/functions for EVERY CLI command, subsystem, and test
suite; the "read this fragment for this task" index extending `START.md`
section 6.

<a id="llm:task-map"></a>
## Map

TBD (task 13.1): full functionality-to-source map over all 72 hand-written
files + every test suite, at file/function granularity.

## Coverage (growing stub)

Sync-gate requirement: every manifest-listed source file must appear below
as a doc→code reference. Chunks 1.3-1.10 append their files here; task 13.1
rewrites this section into the full functionality-to-source map.

- `src/lib.rs` - crate root: module tree, public API surface
- `src/main.rs` - CLI entry point: argument parsing and command dispatch
- `src/error.rs` - shared source spans and parse-error types
- `src/cli_log.rs` - minimal stderr logging for the new CLI commands
- `src/frontend.rs` - frontend CLI logic: RFS/JSON conversion, validation dispatch
- `src/printer.rs` - IR-to-RFS source printer (unparser) and round-trip helpers
- `src/parser.rs` - RFS v0 source parser (nom-based): spec text to IR
- `src/ebnf_grammar.rs` - EBNF grammar model + renderers for IDE/tooling output
- `src/validator.rs` - semantic validation and expression typing for RFS IR
- `src/ir.rs` - IR types + hand-written JSON (de)serialization (pinned wire format)
- `src/json_rw.rs` - hand-rolled JSON reader/writer (only JSON impl in the toolchain)
- `src/json_schema.rs` - draft-07 JSON-Schema emitter + markdown renderer + sync walker
- `src/spec_hash.rs` - hand-rolled SHA-256; spec hash for generated artifact headers
- `src/codegen.rs` - codegen backend API + plugin registry (language-agnostic core)
- `src/codegen_fsm.rs` - shared production-FSM codegen helpers (variants, rep values)
- `src/codegen_mc.rs` - bounded model-checker concepts shared by all backends
- `src/codegen_test.rs` - unit-test planning (TestPlan) shared by all backends
- `src/backends/mod.rs` - backend plugin modules: registration entries per target language
- `src/backends/rust.rs` - Rust backend: plugin registration + shared Rust helpers
- `src/backends/rust_fsm.rs` - Rust backend: production FSM source generation
- `src/backends/rust_mc.rs` - Rust backend: model-checker executable generation
- `src/backends/rust_test.rs` - Rust backend: unit-test / fuzz / aggregate-crate generation
- `src/backends/c.rs` - C backend: plugin registration + shared C helpers
- `src/backends/c_fsm.rs` - C backend: production FSM source generation
- `src/backends/c_mc.rs` - C backend: model-checker executable generation (same wire format as Rust)
- `src/backends/c_test.rs` - C backend: unit-test / fuzz-target generation
- `src/mc_plan.rs` - codegen-time plan for the exact MC engine (embedded in both binaries)
- `src/smt_plan.rs` - backend-neutral SMT spec for k-step unrolling over z3 bitvectors
- `src/smt_emit.rs` - shared SMT emitter: renders the spec into Rust (z3 crate) and C (libz3)
- `src/ref_mc.rs` - brute-force reference model checker (ground truth, rfs mc-ref)
- `src/monitor.rs` - PSL-to-monitor compiler: formulas to MonitorPlan
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
- `src/specgen.rs` - seeded random spec generator + incremental mutation
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
