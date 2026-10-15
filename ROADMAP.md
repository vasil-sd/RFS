# RFS Roadmap

This roadmap describes development from Phase 0 parser/IR baseline to a production-ready DSL and toolchain.

The guiding principle is:

> One constrained specification → one versioned IR → multiple Rust targets.

Targets include production automata, model-checking executables, tests, diagrams, and later IDE support.

How to build/test/run this project: `BUILD.md`. Rust toolchain locations and
activation: `TOOLCHAIN.md`. Session entry point for LLM-assisted work: `START.md`.

---

## Phase 0: Semantics, IR, parser baseline

### Goal

Freeze the core v0 language boundary and prove that RFS sources can be parsed into a stable, serializable IR.

### Scope

- `DESIGN.md`, `ROADMAP.md`, and `KB.md`;
- Rust Cargo project;
- `nom`-based recursive descent parser for RFS v0;
- IR v0 data structures;
- JSON serialization/deserialization;
- CLI parser entry point;
- unit tests;
- 20-30 valid RFS examples;
- extensible example-based integration test suite.

### v0 language subset

- modules;
- enum types;
- initial state;
- events and outputs, including grouped variants;
- finite variables: `bool`, `bit<N>`, enum types;
- transitions with event patterns, optional guards, actions;
- actions: `next`, `emit`, `set`;
- pure expression language;
- minimal properties and assumptions:
  - boolean formulas;
  - `next`;
  - `always`;
  - `bounded_eventually[N]`;
  - `input_is(...)`;
  - `output_has(...)`.

### Deliverables

- `/workspace/host/RFS/DESIGN.md`
- `/workspace/host/RFS/ROADMAP.md`
- `/workspace/host/RFS/Cargo.toml`
- `/workspace/host/RFS/src/**`
- `/workspace/host/RFS/examples/valid/*.rfs`
- `/workspace/host/RFS/tests/**`
- passing `cargo test`

### Exit criteria

- `cargo build` succeeds.
- `cargo test` succeeds.
- All valid examples parse successfully.
- IR can be serialized to JSON and deserialized back.
- Adding a new valid `.rfs` example does not require changing test code.
- Parser errors include useful spans/messages for basic malformed input.

### Non-goals

- semantic validation beyond parsing;
- type checking;
- production code generation;
- model checking;
- full PSL;
- use of priorities in generated/model-checked semantics (the parser accepts an optional `priority N;` action as a forward-compatible extension);
- composition.

---

## Phase 1: Minimal end-to-end demonstrator

### Goal

Prove the full idea:

```text
RFS spec -> Rust production code + Rust model-checking executable
```

### Scope

- small DSL subset, possibly narrower than Phase 0 parser;
- production code generation for a pure `Fsm::step` function;
- brute-force model checking up to small depth;
- one or two property forms:
  - invariant `always p`;
  - simple implication `always (a -> b)`;
  - maybe `next p`;
- counterexample trace printing;
- frontend CLI rework: `rfs parse` format conversion between RFS and IR JSON
  (all four combinations, target-dir support, collision guard) and `rfs validate`
  working on both `.rfs` and `.json` inputs with `.rfs-validation.err` reports.

### Production code generation

Generate:

```rust
pub enum State { ... }
pub enum Input { ... }
pub enum Output { ... }
pub struct Vars { ... }
pub struct Fsm { ... }
impl Fsm {
    pub fn new() -> Self;
    pub fn step(&mut self, input: Input) -> Result<Vec<Output>, UnexpectedEvent>;
}
```

### Model checking

- enumerate input sequences up to depth `K`, initially `K = 5` or `6`;
- execute generated FSM in Rust;
- check simple properties;
- print counterexample trace.

### Example MVP spec

A simple machine with a counter and timeout, for example:

```rfs
module demo {
  enum State { A, B }
  initial A

  event Go
  event Stop
  output Done

  var count: bit<8> init 0

  transition A on Go {
    next B;
    set count = count + 1;
  }

  transition B on Stop where count >= 3 {
    next A;
    emit Done;
    set count = 0;
  }

  prop done_returns_to_a :
    always (output_has(Done) -> state == A) ;
}
```

### Exit criteria

- generated Rust compiles;
- generated FSM behaves correctly in unit tests;
- model checker finds an injected bug;
- model checker passes a correct spec within depth bound;
- counterexample trace is printed;
- one protocol-like example works end-to-end.

---

## Phase 2: Core DSL and production code generation

### Goal

Make the DSL practical for real endpoint protocol FSMs.

### Scope

- full finite types:
  - `bool`;
  - enums;
  - `bit<N>`;
- event payload patterns with bindings;
- guards using bound fields;
- output payloads;
- multiple variable updates;
- simultaneous update semantics enforced in generated code;
- explicit priorities for overlapping transitions;
- error handling policy;
- optional `on_error` or default transition support;
- module metadata and version header.

### Validator improvements

Add checks for:

- unknown states/events/vars;
- non-finite types;
- ambiguous transitions;
- unreachable states;
- dead events;
- unused variables;
- duplicate variable assignment in one transition;
- missing `next`;
- invalid expression types;
- impossible guards, at least syntactically and optionally via bounded checking.

### Production code generation improvements

Generate idiomatic Rust with:

- derives where appropriate;
- clear error type;
- simultaneous variable updates using temporary next variables;
- priority-ordered match arms;
- optional feature flags:
  - `std` / `no_std` later;
  - `property-monitors`;
  - logging/tracing hooks.

### Additional target languages (C backend)

The plugin architecture (Phase 1) makes new targets additive: implement the three command functions for C in `src/backends/c*.rs`, reuse all analysis from `codegen_fsm` / `codegen_test` / `codegen_mc`, register one static in `BACKENDS`, and document it (`CODEGEN.md` checklist §6). Candidate scope:

- production C source (state/input/output enums, struct vars, `fsm_step`);
- C unit tests (e.g. a plain-assert harness, no framework dependency);
- C bounded model checker with the same runtime wire format as the Rust one;
- identifier mangling and fixed-width type mapping for C (`uint8_t..uint64_t`, masks).

### Exit criteria

- a realistic endpoint protocol spec compiles to production Rust;
- generated code is deterministic and testable;
- ambiguous transitions produce clear diagnostics;
- generated code passes `cargo fmt`, `cargo clippy`, and unit tests in a sample crate;
- production integration example exists, for example reading events from a channel and emitting outputs.

---

## Phase 3: PSL-like properties and monitors

### Goal

Add the requested PSL-like temporal property language in a monitor-friendly form.

### Property subset

Support:

```rfs
not p
p && q
p || q
p -> q
next p
always p
always[N] p
bounded_eventually[N] p
```

Atoms:

```rfs
state == S
state != S
var_expr
input_is(EventPattern)
output_has(OutputPattern)
```

Assumptions:

```rfs
assume name : formula ;
```

### Monitor code generation

Each property becomes a Rust monitor.

Example conceptual monitor for:

```rfs
always (input_is(Recv(Fin)) -> bounded_eventually[3] output_has(Send(Ack)))
```

Monitor state:

```rust
struct Monitor {
    pending: bool,
    remaining: u8,
}
```

Step logic:

```text
if input_is(Recv(Fin)):
    pending = true
    remaining = 3

if pending:
    if output_has(Send(Ack)):
        pending = false
    else if remaining == 0:
        violation
    else:
        remaining -= 1
```

For general formulas, the tool should generate monitors from a restricted PSL-to-monitor compiler.

### Model checker integration

The model-checking product state includes:

```text
system_state + system_vars + property_monitor_states + assumption_monitor_states
```

Assumption monitors prune invalid environment branches. Property monitors detect violations.

### Counterexample enhancement

Counterexamples should show:

- which property failed;
- which step triggered the obligation;
- when the deadline expired;
- relevant variable values;
- input/output sequence.

### Production debug monitors

Optional feature:

```bash
rfs codegen production protocol.rfs --with-property-monitors
```

Generated code can call monitors in debug builds:

```rust
#[cfg(debug_assertions)]
self.property_monitors.step(sample)?;
```

This allows runtime detection of environment or implementation violations during testing/fuzzing.

### Exit criteria

- PSL-like properties parse and validate;
- monitors are generated in Rust;
- model checker can verify safety and bounded liveness properties;
- assumptions restrict input sequences correctly;
- counterexamples include property-specific information;
- optional runtime property monitors compile and work in debug mode.

---

## Phase 4: Scalable model checking

### Goal

Move from toy exhaustive checking to a useful verification tool.

### Exact reachable-state engine

Implement proper state-space search in generated Rust or in `rfs-mc-core`:

- visited set;
- parent pointers for counterexamples;
- memory limits;
- state count reporting;
- input abstraction for unused payload fields;
- deterministic and nondeterministic modes.

Statuses:

```text
PASS (complete)
FAIL
UNKNOWN (state limit reached)
```

### Input abstraction

If an event payload field is not used by any guard, action, or property, the model checker can treat it as a don't-care value.

Example:

```rfs
event Recv(Packet(len: bit<8>, tag: bit<32>))
```

If `tag` is never used in transitions or properties, MC does not need to iterate over all `tag` values. This is sound when the field is truly behaviorally irrelevant.

### Optional symbolic/SMT bounded mode

For larger bitvectors, add an optional backend:

- SMT-based bounded model checking;
- k-liveness for bounded liveness;
- clearly labeled as incomplete unless completion criteria are met.

Possible Rust dependencies later:

- `z3` bindings;
- custom bitvector encodings;
- SAT/SMT solver backend trait.

This mode is for bug hunting and larger-data exploration, not a replacement for exact finite MC.

### Liveness engine

Add strong liveness support:

```rfs
eventually p
p until q
```

Implementation options:

1. Büchi product + fair cycle search in Rust.
2. k-liveness transformation for bounded checking.

Fairness policy must be documented.

Recommended default fairness for deterministic systems:

- environment assumptions define input fairness;
- system behavior is deterministic;
- no hidden scheduler nondeterminism.

For nondeterministic MC mode:

- use strong transition fairness or explicitly mark fair transitions;
- document clearly because liveness under arbitrary unfair scheduling can be misleading.

### Performance features

Add:

- state-space statistics;
- transition count;
- property monitor size;
- memory usage estimate;
- timeout and state-limit options;
- parallel search where safe;
- incremental MC cache for CI, maybe later.

### Exit criteria

- exact MC completes on medium-sized finite specs;
- bounded MC works on larger bitvector specs;
- liveness properties can be checked in at least bounded mode;
- counterexamples are actionable;
- tool reports complete vs bounded vs unknown clearly;
- benchmark suite exists with several protocol-like examples.

---

## Phase 5: Full toolchain and production integration

### Goal

Make the DSL usable by teams in real Rust projects.

### CLI hardening

Provide stable commands:

```bash
rfs parse
rfs validate
rfs codegen --list
rfs codegen <backend> --help
rfs codegen <backend> production <spec.rfs|spec.json>
rfs codegen <backend> test <spec.rfs|spec.json>
rfs codegen <backend> mc <spec.rfs|spec.json>
rfs test
rfs diagram
rfs info
```

Options:

```bash
--target-dir
--features
--no-std
--with-property-monitors
--counterexample-format json|text
--state-limit
--time-limit
--log-level
```

### Cargo integration

Provide:

- `rfs-build` crate for `build.rs`;
- optional procedural macro crate later;
- example crates:
  - sync protocol handler;
  - async protocol handler;
  - fuzzing harness.

### Adapters

Generate or provide traits:

```rust
pub trait ProtocolFsm {
    type Input;
    type Output;
    type Error;

    fn new() -> Self;
    fn step(&mut self, input: Self::Input) -> Result<Vec<Self::Output>, Self::Error>;
}
```

Optional adapters:

- sync channel-based;
- `futures`-based async;
- callback-based;
- embedded/no_std.

### Test generation

Generate tests from specs:

- unit tests for each transition;
- invariant tests;
- property monitor debug tests;
- random trace tests;
- fuzz target skeleton.

Example generated fuzz target skeleton:

```rust
#![no_main]
libfuzzer_sys::fuzz_target!(|data: &[u8]| {
    let mut fsm = tcp_handshake::Fsm::new();
    for byte in data {
        if let Ok(input) = decode_input(*byte) {
            let _ = fsm.step(input);
        }
    }
});
```

The decoding strategy depends on the protocol, but the tool can generate a skeleton.

### Diagrams

Generate:

- Graphviz DOT;
- SVG/PNG state diagrams;
- transition tables;
- variable/event summaries.

This helps review and documentation.

### LSP / IDE support

Initial IDE support:

- syntax highlighting for `.rfs`;
- CLI diagnostics integration;
- hover information from IR maybe later.

Full LSP later:

- go-to-definition;
- references;
- inline errors;
- completion.

### Exit criteria

- a team can add RFS to an existing Rust project with `build.rs`;
- generated production code is stable and maintainable;
- CI can run model checking and generated tests;
- fuzzing harnesses can be generated;
- diagrams and docs are useful for protocol review;
- basic IDE support exists.

---

## Phase 6: Production hardening and v1.0

### Goal

Make the toolchain trustworthy enough for production protocol work.

### Formal semantics document

Write a precise semantics document covering:

- automaton model;
- transition relation;
- deterministic vs nondeterministic modes;
- property sampling;
- PSL operator semantics;
- strong/weak liveness;
- fairness;
- assumptions;
- error/deadlock semantics;
- abstraction rules, if any.

This is essential for user trust.

### Soundness testing

Add differential testing:

- compare brute-force MC with reachable-state MC on small specs;
- compare generated monitors against hand-written reference monitors;
- compare PSL translation against an external model checker or library where possible;
- fuzz random specs and check invariants of code generation.

### Fuzzing the toolchain itself

Fuzz:

- parser;
- validator;
- IR serialization;
- code generator;
- model checker on small generated specs.

Goals:

- no crashes on malformed but parseable input;
- no invalid Rust generated from valid specs;
- no false `PASS (complete)` when search was incomplete;
- stable diagnostics.

### Benchmark suite

Create benchmark specs:

- simple handshake;
- timeout/retry protocol;
- sliding window control abstraction;
- HTTP/2-like stream state machine;
- USB-like control transfer FSM;
- CAN-like arbitration/state machine, if appropriate.

Track:

- parse time;
- codegen time;
- MC states explored;
- MC time;
- memory usage;
- counterexample quality.

### Stability and versioning

Define:

- RFS syntax version header;
- IR schema version;
- generated code compatibility policy;
- migration guide process.

Example:

```rfs
# rfs 1.0
```

Generated code should include the spec hash and toolchain version.

### Documentation

Provide:

- language reference;
- property language reference;
- model-checking modes reference;
- production integration guide;
- examples;
- migration guide from earlier versions;
- FAQ on abstraction and assumptions.

### Exit criteria

- v1.0 syntax is stable;
- toolchain passes fuzzing and differential testing;
- model checker reports are trustworthy and clearly labeled;
- production code generation is used in at least one real or realistic protocol project;
- documentation is sufficient for external users;
- benchmark suite runs in CI.

---

## Recommended v1 scope

To keep the first production-ready version manageable, v1 should include:

- external `.rfs` files;
- single endpoint module;
- finite states as enums;
- events and outputs with finite payload fields;
- variables of type `bool`, enum, `bit<N>`;
- pure guarded transitions;
- simultaneous variable updates;
- explicit priorities for overlapping transitions;
- deterministic production semantics;
- `Result`-based unexpected event handling;
- optional explicit error transition;
- `Tick` event and timer variables;
- PSL-like safety properties:
  - boolean atoms;
  - `next`;
  - `always`;
  - `always[N]`;
  - `bounded_eventually[N]`;
- assumptions using the same property language;
- exact reachable-state MC for small systems;
- bounded MC mode;
- counterexample JSON/text output;
- production Rust code generation;
- optional debug property monitors;
- CLI and `build.rs` integration;
- generated unit tests.

v1 should not include:

- multi-party composition;
- real-time timed automata;
- unbounded integers;
- arbitrary Rust function calls in transitions;
- full strong unbounded liveness unless implementation goes smoothly;
- LSP;
- advanced symbolic/SMT MC;
- byte-level protocol codecs.

These can be added later without changing the core architecture.

---

## Suggested implementation stack

| Component | Recommended technology |
|---|---|
| Parser | `nom` 8 recursive descent parser for Phase 0; `chumsky` remains a future candidate for richer diagnostics/recovery |
| Diagnostics | Custom span-based diagnostics first; `miette` later if useful |
| IR serialization | `serde` + `serde_json` initially; protobuf later if needed |
| CLI | `clap` later; simple `std::env::args` for Phase 0 |
| Code generation | Direct Rust string generation first; `proc-macro2`/`quote` later if useful |
| Formatting generated Rust | `rustfmt` programmatic invocation or `rustfmt` crate |
| Testing | `cargo test`, snapshot tests later, property-based tests later |
| Fuzzing | `cargo-fuzz` / `libfuzzer` later |
| LSP later | `tower-lsp` |

## Model-checking library stack

| Component | Recommendation |
|---|---|
| Exact search core | Custom Rust graph search with hash sets/bitsets |
| State packing | Generated bit-packing for states/vars/monitors |
| Counterexample storage | Parent pointers or trace recording |
| Bounded/SMT later | Optional `z3` bindings behind feature flag |
| Symbolic later | Custom BDD or SAT-based backend, optional |

---

## Current status

Phase 0 is being implemented in this repository:

- project root: `/workspace/host/RFS/`
- design document: `DESIGN.md`
- roadmap: `ROADMAP.md`
- knowledge base for future LLM-assisted work: `KB.md`
- Rust source: `src/`
- examples: `examples/valid/`, `examples/invalid/`
- tests: `tests/`

Current Phase 0 implementation status:

- RFS parser is implemented with `nom` 8 in `src/parser.rs`.
- The hand-rolled lexer module was removed after the parser-library evaluation.
- `chumsky` 0.13 is retained as a dev-dependency for the comparison smoke test; it is not used by the main RFS parser.
- SFT training data is organized under `training/<category>/rfs_training.jsonl`; extracted RFS snippets are stored under `training/<category>/snippets/` and tested by the parser suite.
- TLA-inspired protocol examples are stored under `examples/valid/tla_adapted/`, and TLA adaptation reasoning is stored in `training/tla_adaptation/`.
- `cargo build-training` aggregates category training files into `all_rfs_training.jsonl`; `cargo extract-training-snippets` regenerates snippet files.
- `cargo test` covers parser unit tests, valid/invalid example suites, IR JSON round-trip, nom/chumsky smoke comparison tests, and training-set structure/snippet/aggregation tests.
- `cargo run -- validate --input rfs examples/valid` passes: all valid Phase 0 examples parse and validate.
- Invalid examples fail as expected, covered by the example test suite (`cargo test --test examples_test`).

Current Phase 1 implementation status:

- Frontend CLI rework is implemented in `src/frontend.rs` (logic) and `src/main.rs` (argument parsing/exit codes):
  - `rfs parse --input <rfs|json> --output <rfs|json> <file-or-dir> [--target-dir DIR]` converts between all four RFS/JSON format combinations through the IR; output names replace (never stack) the input extension; without `--target-dir` outputs are written next to their inputs, with it the input directory structure is preserved; a normalized-absolute-path collision guard refuses conversions that would overwrite the input file.
  - `rfs validate <file>` works on `.rfs` or `.json` files (extension selects the format) and prints errors to stderr; `rfs validate --input <rfs|json> <dir>` scans the directory tree and writes a `<name>.rfs-validation.err` file next to each failing source.
  - Exit code is 0 on success and 1 for parse, planning, collision, or validation failures.
  - `tests/frontend_test.rs` (41 tests) covers format parsing, strict extension handling, output placement/structure, the collision guard, err-file generation, positive/negative validation, and end-to-end binary exit-code checks; round-trip tests cover rfs -> json -> rfs and json -> rfs -> json.
- `src/validator.rs` provides semantic validation and expression/formula typing for code generation.
- Codegen is a plugin system: `src/codegen.rs` (Backend API, `BACKENDS` registry, `rfs codegen` CLI parsing, help rendering) + language-agnostic analysis in `src/codegen_fsm.rs` / `codegen_test.rs` / `codegen_mc.rs` + the Rust backend under `src/backends/rust*.rs`. API reference: `CODEGEN.md`; backend architecture: `src/backends/RUST.md`.
- The generated Rust FSM code includes trace recording, assumption checks, property-check helpers, and generated unit tests; the bounded model checker is the production core plus a runtime (random bounded runs, forced assumption violations, unexpected-event injection).
- CLI: `rfs codegen --list | --help | <backend> --help` and `rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json> [--out FILE] [--assumes ...] [--steps N]`; exit 0 on success, 1 on unknown backend, bad extension, parse, or generation errors.
- `tests/codegen_test.rs` (12 tests) covers the registry, per-command generation for all valid examples/training snippets, option sensitivity, and every documented CLI parse case.
- `cargo generate-code-tests` and `cargo test-generated-code` generate, compile, and run aggregate unit tests for all valid examples and valid training snippets.
- The generated bounded model checker supports random sequence generation, active safety assumptions, forced assumption violations, and unexpected-event injection.
- Liveness formulas containing `bounded_eventually` are skipped with warnings in the Phase 1 translated subset.
- Phase 2 preparation cleanup (2026-10-14): removed legacy functionality before core-DSL work - the Phase 0 positional parser CLI mode (`cargo run -- <paths> [--json]`), the `validate_rfs` and `rfs_codegen` cargo bins (superseded by `rfs validate` and `rfs codegen rust ...`), and the pre-plugin monolithic codegen API in `src/codegen.rs`; auxiliary bins and tests now call backend functions directly (e.g. `rfs::backends::rust_test::generate_aggregate_test_crate`).
- SFT training sets are actualized to the current state: 11 categories / 219 examples, including a new `frontend_cli` category (parse/validate/codegen CLI behavior, format round-trips, error reporting, exit codes); `codegen_rust` now covers the backend plugin architecture, the `rfs codegen` CLI, and adding new target languages; `project_tooling` keeps a current implementation-status example. Extracted snippets: 120 valid / 125 invalid; regenerate with `cargo extract-training-snippets` + `cargo build-training`.
- Phase 2 (core DSL and production code generation) is COMPLETE:
  - Language: module version headers (`module Name version "x.y.z" {`), `on_error { next S; emit ...; }` fallback blocks, and semantic use of the previously parsed `priority N;` action. All three are parse/validate/print round-tripped (examples 39-40).
  - Validator: duplicate-priority and shadowed-duplicate-transition errors; non-fatal warning channel (`validator::module_warnings`, surfaced by `rfs validate` and codegen) for unreachable states, dead events, unused variables, and impossible guards. Decisions D1-D7 in `DESIGN.md` section 20.
  - Rust codegen: priority-ordered transition arms per state (effective priority, default 0), on_error fallback replacing `Err(StepError::UnexpectedEvent)`, `SPEC_VERSION` constant; generated negative unit tests adapt to on_error modules.
  - C backend: `src/backends/c*.rs` registered as the second plugin (`rfs codegen c production|test|mc`); fixed-width type mapping (bit<8/16/32/64> -> uintN_t, decision D8), plain-assert unit tests, bounded model checker with the same CLI wire format and LCG as the Rust MC; architecture in `src/backends/C.md`.
  - Tests: main suite 125 (was 95: +13 validator, +4 parser, +4 C helpers, +7 codegen, +2 integration); generated aggregate crate 492 (164 modules); new `tests/integration_example_test.rs` compiles the realistic endpoint-protocol example with rustc and drives it through an mpsc channel (determinism + on_error behavior).
  - Examples: +2 valid (39_endpoint_protocol, 40_on_error_recovery), +5 invalid (duplicate priority, shadowed duplicate, on_error without next, bad version header, duplicate set).
