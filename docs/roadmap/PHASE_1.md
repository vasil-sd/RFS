# Phase 1: Minimal end-to-end demonstrator

## Goal

Prove the full idea:

```text
RFS spec -> Rust production code + Rust model-checking executable
```

## Scope

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

## Production code generation

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

## Model checking

- enumerate input sequences up to depth `K`, initially `K = 5` or `6`;
- execute generated FSM in Rust;
- check simple properties;
- print counterexample trace.

## Example MVP spec

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

## Deliverables

- Frontend CLI rework: `rfs parse` (all four rfs/json combinations, `--target-dir`, collision guard) and `rfs validate` (file + directory modes, `.rfs-validation.err` reports) in `src/frontend.rs` / `src/main.rs`.
- Semantic validator with expression/formula typing (`src/validator.rs`) as a precondition for code generation.
- Plugin-based codegen: Backend API + `BACKENDS` registry (`src/codegen.rs`), shared analysis (`src/codegen_fsm.rs`, `src/codegen_test.rs`, `src/codegen_mc.rs`), Rust backend (`src/backends/rust*.rs`).
- Generated production FSM (pure `Fsm::step`) plus generated unit tests.
- Bounded model checker (brute force up to depth K) with counterexample trace printing.
- One protocol-like example working end-to-end.

## Exit criteria

- generated Rust compiles;
- generated FSM behaves correctly in unit tests;
- model checker finds an injected bug;
- model checker passes a correct spec within depth bound;
- counterexample trace is printed;
- one protocol-like example works end-to-end.

## Non-goals

- full PSL property language (only one or two simple property forms here; the full subset arrives in Phase 3);
- scalable state-space search (brute force up to small depth only; the exact engine is Phase 4);
- additional target languages beyond Rust (the C backend arrives in Phase 2);
- core DSL extensions: event payloads with bindings, semantic priorities, `on_error` (Phase 2).
