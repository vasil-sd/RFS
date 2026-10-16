# Phase 2: Core DSL and production code generation

## Goal

Make the DSL practical for real endpoint protocol FSMs.

## Scope

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

## Validator improvements

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

## Production code generation improvements

Generate idiomatic Rust with:

- derives where appropriate;
- clear error type;
- simultaneous variable updates using temporary next variables;
- priority-ordered match arms;
- optional feature flags:
  - `std` / `no_std` later;
  - `property-monitors`;
  - logging/tracing hooks.

## Additional target languages (C backend)

The plugin architecture (Phase 1) makes new targets additive: implement the three command functions for C in `src/backends/c*.rs`, reuse all analysis from `codegen_fsm` / `codegen_test` / `codegen_mc`, register one static in `BACKENDS`, and document it (`CODEGEN.md` checklist §6). Candidate scope:

- production C source (state/input/output enums, struct vars, `fsm_step`);
- C unit tests (e.g. a plain-assert harness, no framework dependency);
- C bounded model checker with the same runtime wire format as the Rust one;
- identifier mangling and fixed-width type mapping for C (`uint8_t..uint64_t`, masks).

## Deliverables

- Extended RFS v0 syntax: module version headers, `on_error` fallback blocks, semantic use of `priority N;` - all parse/validate/print round-tripped.
- Validator: duplicate-priority and shadowed-duplicate-transition errors; non-fatal warning channel for unreachable states, dead events, unused variables, impossible guards.
- Rust codegen: priority-ordered transition arms, on_error fallbacks, `SPEC_VERSION` constant; generated negative tests adapt to on_error modules.
- C backend plugin (`src/backends/c*.rs`): production/test/mc commands, fixed-width type mapping, plain-assert unit tests, bounded MC with the Rust wire format.
- Realistic endpoint-protocol example plus integration test compiling it with rustc and driving it over a channel; new invalid examples pinning the validator errors.

## Exit criteria

- a realistic endpoint protocol spec compiles to production Rust;
- generated code is deterministic and testable;
- ambiguous transitions produce clear diagnostics;
- generated code passes `cargo fmt`, `cargo clippy`, and unit tests in a sample crate;
- production integration example exists, for example reading events from a channel and emitting outputs.

## Non-goals

- PSL-like properties and monitors (Phase 3);
- exact reachable-state MC beyond the bounded runtime (Phase 4);
- `no_std`/property-monitor/logging codegen feature flags (deferred, D9; Phase 5 candidates);
- multi-party composition and unbounded data (v1 exclusions).
