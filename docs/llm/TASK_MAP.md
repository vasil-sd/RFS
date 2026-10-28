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
