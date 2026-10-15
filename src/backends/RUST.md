# Rust Codegen Backend

Architecture and design decisions of the Rust target-language backend
(`src/backends/rust*.rs`). For the generic plugin API see `CODEGEN.md` at the
repository root; for semantics see `DESIGN.md`.

## 1. File layout

| File | Role |
|---|---|
| `rust.rs` | Plugin entry: `RUST_BACKEND` static (name, description, command descriptors, function pointers) plus shared Rust-specific helpers: `RUST_KEYWORDS`, `rust_ident` (keyword-safe identifier), `snake` (case conversion), `bit_rust_type` / `mask_expr` (fixed-width integer mapping), `rust_type` (IR type → Rust type), `representative_input_expr`. |
| `rust_fsm.rs` | `production` command: emits the standalone FSM source. Also the core reused by `test` and `mc`. |
| `rust_test.rs` | `test` command: appends a unit-test module to the production code; plus the aggregate test-crate helpers used by the cargo tooling (`generate_aggregate_test_crate`, `write_generated_crate`, `GeneratedCrate`). |
| `rust_mc.rs` | `mc` command: production FSM core + an appended model-checker runtime (`MC_RUNTIME`) with its own `main`. |

Dependency direction inside the backend: `rust_mc → rust_fsm (+ MC_RUNTIME)`,
`rust_test → rust_fsm (via generate_with_tests)`, all three → shared helpers in
`rust.rs`, and all of them → the language-agnostic core modules
(`codegen_fsm`, `codegen_test`, `codegen_mc`).

## 2. Generated artifact design

Every command emits **one self-contained `.rs` file**:

- no dependency on the `rfs` crate or any external crate; compiles with stable
  Rust, edition 2021;
- header `#![allow(dead_code, unused_variables, ...)]` so generated code is
  warning-clean even for degenerate modules (no variables, no outputs);
- deterministic: same IR + options ⇒ byte-identical output.

### 2.1 Production FSM anatomy (`rust_fsm.rs`)

The generated file contains, in order:

1. `pub enum State { ... }` — one variant per RFS state.
2. `pub struct Vars { ... }` — module variables (unit struct when there are
   none).
3. `pub enum Input { ... }` / `pub enum Output { ... }` — **flattened**
   constructors: each variant of every event/output group becomes a top-level
   variant named by `VariantInfo::unique_name` (mangled through
   `rust_ident`), carrying the constructor's fields. Flat enums keep the
   transition `match` exhaustive and simple for downstream users.
4. Trace runtime: `TraceStep { input, outputs, from_state, to_state, vars }`,
   `Trace` with `state_at / vars_at / input_at / outputs_at` accessors — a
   complete execution record used by tests, property checks, and the model
   checker.
5. Assumption runtime: `AssumptionPolicy { Continue, Stop, Panic }`,
   `trait AssumptionHandler`, and the three built-in handlers
   (`Panic/Stop/Continue`). `Fsm::step` is the plain entry point (panic on
   violation); `Fsm::step_with_handler` delegates policy decisions to a caller
   handler.
6. `pub enum StepError { UnexpectedEvent, AssumptionViolation { label } }`.
7. Property and assumption check functions operating on the recorded trace
   (the translated safety subset; liveness constructs outside it are skipped
   with a warning at generation time).
8. `pub struct Fsm { state, vars, trace, active_assumes }` with `new()` and
   `step(input) -> Result<Vec<Output>, StepError>`: the transition block is a
   `match` on `(state, input)`; guards are lowered expressions over old
   values; variable updates are **simultaneous** (all assignments read pre-step
   values); outputs are collected per step.

### 2.2 Expression lowering

`gen_expr(e: &Expr, env: &Env, ctx: &ExprCtx) -> Result<String, String>` lowers
the IR expression language to Rust expressions using the validator's typing
environment (`crate::validator::{Env, ExprType}`). Fixed-width integers are
mapped to `u1..=u64` via `bit_rust_type`, with masking helpers (`mask_expr`) so
wraparound semantics match the DSL. Width inference for literals is
context-sensitive (`width_from_type`).

## 3. Unit-test generation (`rust_test.rs`)

The `test` command runs the shared planner from `codegen_test::build_test_plan`
and renders:

- **positive test**: drive the FSM with representative inputs up to K steps
  (`--steps`, default 4); when `expect_progress` is set, assert at least one
  step succeeded;
- **negative test (unexpected-input search)**: probe input variants for an
  unexpected event. When `definite_unexpected` holds (a variant no transition
  matches), the test *asserts* the failure is found; otherwise it records the
  attempt without asserting, so fully-covered modules still get a passing test;
- deadlock-style modules (`is_deadlocked_initial`) are covered by the same
  negative path.

Test code is appended to the production source as `mod tests { ... }`, so the
`test` artifact always contains the full production API plus its tests.

### Aggregate tooling

`generate_aggregate_test_crate(files, options) -> GeneratedCrate` renders one
crate with a `pub mod` per module (name-mangled to be collision-free across
files); `write_generated_crate(root, crate_)` writes it to disk. This powers
`cargo generate-code-tests` / `cargo test-generated-code`, which compile and run
all generated tests for every valid example and training snippet.

## 4. Model-checker generation (`rust_mc.rs`)

`generate_model_checker` reuses the production core (`generate_with_tests(...,
false)`) and appends a runtime that:

- embeds its own RNG (deterministic given `--seed`), so runs are reproducible;
- parses its own CLI (wire format defined in `codegen_mc::McConfig`):
  `--bound N`, `--runs N`, `--seed N`, `--assumes all|none|labels`,
  `--violate label:prob:percent%[,label2:...]`, `--unexpected-prob P`;
- at each step selects an input by the phase order from `codegen_mc::McPhase`:
  forced assumption violation first, then unexpected-event injection, then a
  normally allowed random input;
- checks every property label against the recorded trace after each run and
  reports outcomes using the `codegen_mc::McStatus` taxonomy (completed,
  property violation, unexpected event, assumption stop); counterexample traces
  are printed for failures.

Because the generated binary cannot depend on this crate, `rust_mc.rs` contains
embedded copies of the parsing helpers (`parse_violation_spec`,
`parse_assume_selection`). **The formats in `src/codegen_mc.rs` are the source
of truth** — when a wire format changes, update both the core definition and
the embedded copy, and extend the tests.

## 5. Key design decisions

1. **Static registry, function pointers.** Backends are compile-time statics in
   `codegen::BACKENDS`; dispatch is a plain function-pointer call through
   `CodegenCommandKind::run`. No trait objects, no dynamic loading — simplest
   possible plugin model for a single-binary toolchain.
2. **Flat Input/Output enums** instead of nested per-group types: exhaustiveness
   and pattern matching stay trivial in user code; the flattening logic is
   shared with all backends via `codegen_fsm::build_variants`.
3. **Trace recording built into production code**, not only into tests/MC:
   property checks, unit tests, and the model checker all consume the same
   trace type, guaranteeing they observe identical behavior.
4. **Assumption handling as a pluggable policy** (`AssumptionHandler` trait) so
   production users can choose panic/stop/continue while the model checker uses
   stop semantics to keep exploring.
5. **MC = production core + runtime**, not a separate translation: the checked
   system and the shipped system are literally the same code, which is what
   makes model-checking results meaningful for production.
6. **Warnings, not errors, for the untranslated property subset** (e.g. liveness
   with `bounded_eventually`): generation succeeds and tells the user exactly
   what was skipped.

## 6. Known limitations

- Only the first module of a multi-module file is generated by the CLI;
  aggregate tooling handles multi-file crates instead.
- The translated property subset is safety-oriented; liveness constructs outside
  it are dropped with warnings (see `DESIGN.md` §Model-checking constraints).
- Identifier mangling escapes Rust keywords but does not perform full name
  resolution against user code that includes the generated file.
