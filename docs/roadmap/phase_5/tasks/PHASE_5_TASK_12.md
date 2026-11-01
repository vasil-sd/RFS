# Phase 5, Task 12 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Complete the example-crate matrix per the parity principle: a plain-C sync-style
handler project (the C mirror of the Rust examples) and fuzzing harness examples
for BOTH ecosystems wiring the generated fuzz skeletons (Rust libfuzzer_sys via
cargo-fuzz, C libFuzzer via clang).

## Context

- Parity principle: language-neutral functionality ships for all backends. The
  sync handler semantics (drive the ARQ protocol with a scripted peer) are
  mirrored in plain C against generated C code - same scenario, same expected
  outcomes, `gcc -Wall` clean. Rust-only idioms (futures async) stay documented
  as Rust-ecosystem-only (task 11).
- Fuzz harnesses: task 5/6 already EMIT the skeletons; this task wires them into
  buildable example crates:
  - `crates/examples/fuzz_rust/`: cargo-fuzz layout (`fuzz/Cargo.toml` with
    libfuzzer_sys, `fuzz/fuzz_targets/arq.rs` = the emitted skeleton adapted to
    the generated module), README with `cargo fuzz run` instructions.
  - `crates/examples/fuzz_c/`: the emitted `fuzz_target.c` + a Makefile invoking
    clang with `-fsanitize=fuzzer,address`, README documenting the CLANG
    requirement (generation works without clang; building does not).
- Both fuzz examples use spec 43 (sliding-window ARQ) for consistency.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | C test renderer + libFuzzer skeleton | emits the C fuzz skeleton this task wires |
| 11. | ProtocolFsm trait + adapter crate + Rust example crates | the Rust examples it mirrors; workspace layout conventions |

### Dependent tasks

(none directly; tasks 14/15/16 document, train, and verify)

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/c_sync_handler/` (Makefile, src/, README.md) | new | plain-C sync-style ARQ handler project on generated C code |
| `crates/examples/fuzz_rust/` (fuzz/ layout, README.md) | new | cargo-fuzz harness wiring the Rust skeleton |
| `crates/examples/fuzz_c/` (Makefile, src/, README.md) | new | clang libFuzzer harness wiring the C skeleton |
| `Cargo.toml` | change | workspace members (the Rust-side example crates; C project is Makefile-driven and NOT a cargo member) |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` or a new `tests/examples_test` extension | change | build+run checks: c_sync_handler via gcc (scenario passes, -Wall clean); fuzz_rust skeleton compiles (`cargo check` in the fuzz crate if feasible without a fuzzer runtime); fuzz_c generation-only check (no clang needed) |

## Planned actions

### Action 1: C sync handler project
Plain-C mirror of the Rust sync scenario on spec 43: generated C FSM + scripted
peer driving send/ack/retransmit; Makefile (`gcc -Wall`), README with build/run
+ expected output. Scenario outcomes identical to the Rust example (pinned in a
test that runs both and compares).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/c_sync_handler/` (new dir) | new | Makefile, src/, README.md |

### Action 2: Rust fuzz harness example
cargo-fuzz crate layout wiring task 5's skeleton to the generated ARQ module;
README with `cargo fuzz run arq -- -max_total_time=10`; a smoke test that the
fuzz target COMPILES (building a real fuzzer binary may need libfuzzer runtime
- gate on availability, document).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/fuzz_rust/` (new dir) | new | fuzz crate + README |

### Action 3: C fuzz harness example
Wire task 6's `fuzz_target.c`: Makefile with the clang `-fsanitize=fuzzer,address`
recipe, README documenting the clang requirement; generation-only test (no clang
needed to generate).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/fuzz_c/` (new dir) | new | Makefile, src/, README.md |

### Action 4: build+run verification tests
Extend the test suite: c_sync_handler builds with gcc -Wall and its scenario
runs to the pinned outcome; Rust/C scenario outcomes compared; fuzz examples'
compile/generation checks per availability.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/frontend_test.rs` (or new test file) | change | the verification tests |

## Task deliverables
- c_sync_handler (plain C, gcc -Wall clean, scenario pinned).
- fuzz_rust (cargo-fuzz layout) + fuzz_c (clang libFuzzer recipe) examples.
- Build/run/parity verification tests.

## Task exit criteria
- [ ] c_sync_handler builds with `gcc -Wall` at zero warnings and its scenario reaches the pinned outcome; outcome matches the Rust sync example (test).
- [ ] fuzz_rust skeleton compiles in its cargo-fuzz layout (or availability-gated with documentation); README instructions verified verbatim.
- [ ] fuzz_c generates without clang; Makefile recipe documented; build test gated on clang availability.
- [ ] Full main suite green, clippy/fmt clean.

## Non-goals
- Actually RUNNING fuzz campaigns to completion in tests (smoke/compile checks only - campaigns are user work).
- ASan/UBSan for the C sync handler (fuzz target gets sanitizers; the handler stays plain gcc).
- More example protocols (one protocol - spec 43 - is enough to prove the matrix).

## Other information
- Check clang availability in this environment before writing the fuzz_c build
  test; if absent, the gate + documentation path applies.
