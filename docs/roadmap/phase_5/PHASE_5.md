# Phase 5: Full toolchain and production integration

## Goal

Make the DSL usable by teams in real Rust projects.

## Scope

- stable CLI surface: `rfs parse`, `validate`, `codegen` (list/help/per-backend), `test`, `diagram`, `info` plus documented options (`--target-dir`, `--features`, `--no-std`, `--with-property-monitors`, `--counterexample-format`, `--state-limit`, `--time-limit`, `--log-level`);
- Cargo integration: `rfs-build` crate for `build.rs`, optional procedural macro crate later, example crates (sync handler, async handler, fuzzing harness);
- adapters: `ProtocolFsm` trait plus sync/async/callback/no_std adapter variants;
- test generation for ALL backends with identical coverage: per-transition unit tests, invariant tests, property monitor debug tests, random trace tests, fuzz target skeleton (Rust `libfuzzer_sys` / C libFuzzer);
- diagrams: Graphviz DOT, SVG/PNG state diagrams, transition tables, variable/event summaries;
- IDE support: initial syntax highlighting + CLI diagnostics integration; full LSP later;
- backend parity maintained (`INTRO.md` principle): every backend-generated deliverable ships for all backends with identical behavior; Rust-ecosystem-only integrations (`rfs-build` crate, futures/no_std adapters) are documented as such, with C-side equivalents where the functionality is language-neutral.

## CLI hardening

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

## Cargo integration

Provide:

- `rfs-build` crate for `build.rs`;
- optional procedural macro crate later;
- example crates:
  - sync protocol handler;
  - async protocol handler;
  - fuzzing harness.

## Adapters

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

The `ProtocolFsm` trait and the async/callback/no_std adapters are Rust
ecosystem integration, not backend deliverables - they do not require a 1:1 C
mapping of idioms. The C backend's equivalent surface is its generated plain-C
step API (same semantics as the Rust FSM); this phase documents it and adds C
example projects mirroring the Rust example crates.

## Test generation

Generate tests from specs, in ALL backends with identical coverage (same test
cases, same expected outcomes):

- unit tests for each transition;
- invariant tests;
- property monitor debug tests;
- random trace tests;
- fuzz target skeleton (Rust: `libfuzzer_sys`; C: libFuzzer).

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

## Diagrams

Generate:

- Graphviz DOT;
- SVG/PNG state diagrams;
- transition tables;
- variable/event summaries.

This helps review and documentation.

## LSP / IDE support

Initial IDE support:

- syntax highlighting for `.rfs`;
- CLI diagnostics integration;
- hover information from IR maybe later.

Full LSP later:

- go-to-definition;
- references;
- inline errors;
- completion.

## Deliverables

- Stable CLI surface with documented options (see Scope).
- `rfs-build` crate for `build.rs` integration; example crates (sync handler, async handler, fuzzing harness).
- Generated adapters and test suites including a fuzz target skeleton, for all backends with identical coverage.
- Diagram generation (Graphviz DOT, SVG/PNG, transition tables, summaries).
- Initial IDE support (syntax highlighting for `.rfs`, CLI diagnostics integration).

## Exit criteria

- a team can add RFS to an existing Rust project with `build.rs`;
- generated production code is stable and maintainable;
- CI can run model checking and generated tests;
- all backend-generated deliverables exist for every backend (Rust and C) and cross-backend comparison of their behavior passes;
- fuzzing harnesses can be generated;
- diagrams and docs are useful for protocol review;
- basic IDE support exists.

## Non-goals

- full LSP (go-to-definition, references, inline errors, completion) - explicitly "later" even within this phase;
- formal semantics document, toolchain fuzzing, benchmark suite in CI, v1.0 stability (Phase 6).
