# Phase 4, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Mirror task 6's liveness analysis in the generated C backend and extend the parity
suite with liveness coverage: identical status + counterexample fields for
liveness properties across backends.

## Context

- Direct mirror of task 6 (fixpoint equations, product combinations, bounded
  truncation, fairness filter) rendered into C from the same embedded plan tables -
  behavior must not depend on data structures (D-P4-2).
- The parity suite (`tests/backend_parity_test.rs`, migrated to exact mode in task
  4) runs over ALL property-bearing examples - example 43 is picked up
  automatically; this task adds explicit liveness assertions (status, reason,
  counterexample fields incl. prefix+cycle shape).
- `gcc -Wall` clean remains the hard bar for generated C.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Exact engine - C | the C runtime being extended |
| 4. | Parity suite migration | exact-mode parity harness to extend |
| 6. | Liveness engine - Rust | the analysis being mirrored; parity reference |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 9. | Benchmark suite test | liveness metrics asserted in both backends |
| 10. | Performance features | may touch the C fixpoint code for speed |
| 13. | Docs sync | C.md documents the liveness runtime |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/c_mc.rs` | change | mirror of task 6: fixpoint analysis + liveness counterexamples |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/backend_parity_test.rs` | change | explicit liveness coverage (status/reason/counterexample field parity on example 43) |

## Planned actions

### Action 1: C fixpoint mirror
Render task 6's analysis into generated C: per-obligation bad-set fixpoints,
product combinations over per-obligation bad sets, bounded truncation semantics,
`--nondet` fairness filter (or the deterministic-only restriction if task 6
recorded one). Same embedded plan tables as Rust.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | liveness analysis in generated C |

### Action 2: Liveness counterexamples in C
Prefix path + cycle reconstruction and printing with the Phase 3 trace format -
fields byte-identical to Rust for identical runs.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | counterexample rendering |

### Action 3: Parity suite liveness coverage
Extend `tests/backend_parity_test.rs`: on example 43 (and any later liveness
examples), assert identical status, reason, label, and counterexample fields
(prefix+cycle) across backends in exact mode; add an inline liveness-FAIL variant
check if cheap (full FAIL variants are task 9's job).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | liveness parity assertions |

## Task deliverables
- Liveness engine in generated C binaries; byte-identical wire formats vs Rust.
- Parity suite covering liveness examples in exact mode.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Generated C compiles with `gcc -Wall` without warnings.
- [ ] Example 43: Rust vs C exact-mode status + reason + counterexample fields identical (test-pinned).
- [ ] Bounded-mode liveness behavior matches Rust on a tiny `--max-states` run (sound FAIL or unknown, no false pass-complete).

## Non-goals
- More benchmark specs (task 8), benchmark assertions/JSON report (task 9),
  performance tuning (task 10).

## Other information
- If task 6 restricted liveness to deterministic mode via plan correction, mirror
  that restriction verbatim (identical CLI behavior + messages in C).
