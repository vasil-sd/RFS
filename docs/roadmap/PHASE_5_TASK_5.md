# Phase 5, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Render the extended test plan (task 4) in the RUST backend's `test` command
output - per-transition unit tests, invariant checks, property-monitor debug
tests, random-trace test - plus a libfuzzer_sys fuzz-target skeleton file, and
regenerate the aggregate generated-code crate.

## Context

- `src/backends/rust_test.rs` currently appends `mod tests { ... }` with the
  module-level tests (positive drive, negative probe). The extended plan adds
  four new test classes rendered into the same artifact; the fuzz skeleton is a
  SEPARATE file (`fuzz_target.rs`) because it depends on `libfuzzer_sys` -
  the main test artifact must STAY dependency-free (design invariant) and the
  aggregate crate must keep compiling without external deps.
- Aggregate crate: `cargo generate-code-tests` regenerates it over all valid
  examples + valid training snippets (179 modules / 537 tests at phase start).
  New per-transition/invariant/etc. tests GROW this count - re-pin in BUILD.md
  and KB.md wherever the number appears (final re-pin happens again at task 16
  close-out; keep docs accurate as of THIS task).
- Test naming convention: `t<transition_id>_<state>_on_<event>` for
  per-transition tests, `invariant_<property_label>`, `monitor_<label>_fires_on_violation`,
  `random_trace_seed12345_stepsK` - stable names (parity compares them).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | Extended shared test planner | provides the plan data this task renders |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | C test renderer + libFuzzer skeleton | mirrors this rendering; parity compares against it |
| 7. | `rfs test` command | compiles+runs exactly what this task generates |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/rust_test.rs` | change | render the four new test classes + emit the separate fuzz skeleton file |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | generated-code count re-pin (537 -> new number, 179 modules) |
| `KB.md` | change | Tests row count if it references the aggregate (section 1) |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | option-sensitivity + new-class presence checks for property-bearing examples; fuzz skeleton file emitted check |
| generated aggregate crate (regenerated) | change | the grown test count, all green |

## Planned actions

### Action 1: render the four new test classes
Extend `rust_test.rs` to consume the extended plan: per-transition tests
(assert next-state + outputs; not-taken case asserts no progress), invariant
checks (re-check atom each drive step, fail with step index), monitor debug
tests (injected violating trace must set the sticky violation record for the
target label only), random-trace test (fixed seed/steps, coverage counts
printed). Stable test names per Context.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_test.rs` | change | the renderers for the four classes |

### Action 2: fuzz skeleton emission
Emit `fuzz_target.rs` (separate file, next to the test artifact or in a
`fuzz/` subdir when `--out` is used): `#![no_main]` + `libfuzzer_sys::fuzz_target!`
looping decoded inputs over `Fsm::step`, using the planner's decode strategy.
Documented header: how to build with cargo-fuzz (task 12 wires the example).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_test.rs` | change | skeleton emission + CLI note in test-command help text |

### Action 3: regenerate aggregate + re-pin counts
`cargo generate-code-tests && cargo test-generated-code`; update BUILD.md
(537 -> actual) and KB.md section 1 if it cites the number.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | count re-pin |
| `KB.md` | change | count re-pin (if present) |

### Action 4: codegen_test updates
Assert the new classes appear for property-bearing examples (e.g. 41, 43),
the fuzz skeleton file is emitted, and test names follow the convention.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change | the assertions above |

## Task deliverables
- Rust `test` output with all four new test classes (stable names).
- libfuzzer_sys fuzz skeleton file emission.
- Regenerated aggregate crate, green; counts re-pinned in docs.

## Task exit criteria
- [ ] All four new test classes present and passing in the regenerated aggregate crate for property-bearing examples.
- [ ] Fuzz skeleton emitted as a SEPARATE file; main test artifact + aggregate crate still dependency-free (compile without libfuzzer_sys).
- [ ] Test names follow the pinned convention (asserted by codegen_test).
- [ ] `cargo test-generated-code` green at the new count; BUILD.md/KB.md re-pinned.
- [ ] Full main suite green, clippy/fmt clean.

## Non-goals
- C rendering (task 5), running the tests via a CLI command (task 6).
- Building/running the fuzz target here (cargo-fuzz wiring is task 12).
- Changing the module-level tests (additive only).

## Other information
- If per-transition counts explode for large examples (44 token ring has many
  transitions), that is expected and fine - they are cheap unit tests; watch
  aggregate compile time at task 16.
