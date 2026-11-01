# Phase 0: Semantics, IR, parser baseline

## Goal

Freeze the core v0 language boundary and prove that RFS sources can be parsed into a stable, serializable IR.

## Scope

- `DESIGN.md`, the roadmap documents (`docs/roadmap/`, originally a single `ROADMAP.md`), and `KB.md`;
- Rust Cargo project;
- `nom`-based recursive descent parser for RFS v0;
- IR v0 data structures;
- JSON serialization/deserialization;
- CLI parser entry point;
- unit tests;
- 20-30 valid RFS examples;
- extensible example-based integration test suite.

## v0 language subset

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

## Deliverables

- `DESIGN.md`
- `docs/roadmap/` (the development plan, originally a single `ROADMAP.md`)
- `Cargo.toml`
- `src/**`
- `examples/valid/*.rfs`
- `tests/**`
- passing `cargo test`

## Exit criteria

- `cargo build` succeeds.
- `cargo test` succeeds.
- All valid examples parse successfully.
- IR can be serialized to JSON and deserialized back.
- Adding a new valid `.rfs` example does not require changing test code.
- Parser errors include useful spans/messages for basic malformed input.

## Non-goals

- semantic validation beyond parsing;
- type checking;
- production code generation;
- model checking;
- full PSL;
- use of priorities in generated/model-checked semantics (the parser accepts an optional `priority N;` action as a forward-compatible extension);
- composition.
