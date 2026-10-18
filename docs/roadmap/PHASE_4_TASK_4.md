# Phase 4, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Migrate `tests/backend_parity_test.rs` and `tests/monitor_integration_test.rs` from
same-seed fuzz to exact mode (D-P4-14): identical status + counterexample fields
across backends on ALL property-bearing examples, with violation values re-pinned
from the deterministic traversal.

## Context

- Current state: the parity suite runs same-seed MC over all property-bearing
  examples and requires field-identical results; `monitor_integration_test.rs`
  drives an injected bug with `--bound 10 --runs 5 --seed 42`. Both pin FUZZ-mode
  values.
- D-P4-14: pinned violation values CHANGE deliberately - exact BFS finds the first
  violation in deterministic traversal order, not random discovery. Re-pin from
  ACTUAL output of the new engine; never hand-compute (plan risk #4).
- Exact mode is seedless: `--runs/--seed/--violate/--unexpected-prob` no longer
  apply to these suites. The injected-bug test runs plain exact mode.
- Formats are unchanged (tasks 2/3 pinned them); only the driving mode and the
  expected VALUES change.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Exact engine - Rust | exact mode under test |
| 3. | Exact engine - C | exact mode under test (parity side) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | Liveness engine - C | extends the migrated parity suite with liveness coverage |
| 9. | Benchmark suite test | builds on the exact-mode harness patterns |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/backend_parity_test.rs` | change | exact-mode differential runs; re-pinned values |
| `tests/monitor_integration_test.rs` | change | injected bug in seedless exact mode; re-pinned expectations |

## Planned actions

### Action 1: Migrate backend_parity_test.rs
Switch the MC invocations from same-seed fuzz to exact mode (default). Assert per
example: identical status, identical summary-line fields (states/transitions/
max_depth), and identical counterexample fields across backends, on ALL
property-bearing examples.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | exact-mode invocations + assertions |

### Action 2: Re-pin violation values
Run the migrated suite, capture the ACTUAL deterministic outputs (first-violation
steps/labels per example), and pin those values in the test expectations. Record
old->new value changes in the status file (they are expected to differ from the
fuzz-era pins).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | re-pinned expected values |
| `tests/monitor_integration_test.rs` | change | re-pinned injected-bug expectations (seedless exact mode) |

### Action 3: Migrate monitor_integration_test.rs
Drive the injected bug in seedless exact mode; assert the fail status, reason,
label, and counterexample fields per the re-pinned values; keep the debug-gated
production-monitor coverage intact.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/monitor_integration_test.rs` | change | exact-mode injected-bug flow |

## Task deliverables
- Both suites green in exact mode with values pinned from actual deterministic
  output.
- Old->new value changes recorded (status file).

## Task exit criteria
- [ ] `cargo test` green, including both migrated suites.
- [ ] Parity suite covers ALL property-bearing examples in exact mode (status + summary fields + counterexample fields identical across backends).
- [ ] No same-seed fuzz assertions remain in these two suites (fuzz stays covered by codegen flag-sensitivity tests + task 2's spot check).
- [ ] Re-pinned values captured from actual runs, old->new diff recorded.

## Non-goals
- Liveness coverage (task 7), benchmark specs/assertions (tasks 8/9), fuzz-mode
  removal (it stays behind `--mode fuzz`).

## Other information
- If an example's exact-mode result is `unknown-state-limit` at the default
  `--max-states`, raise the limit for that example in the test invocation and note
  it - do not lower the engine default here.
- The generated-test-executable half of the parity suite (both backends' `test`
  command outputs) is unaffected by this migration - leave it as is.
