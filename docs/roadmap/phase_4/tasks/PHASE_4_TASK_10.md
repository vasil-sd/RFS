# Phase 4, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Performance features on top of the benchmark data: statistics polish (memory
estimate, monitor size), `--max-states` default tuned from task 9's data,
`--timeout-sec` documented as non-parity, and the OPTIONAL `--threads N`
level-parallel BFS with canonical violation selection.

## Context

- D-P4-7 pinned `--max-states` default = 1000000. Changing it is a PLAN CORRECTION
  (record rationale + benchmark data in the status file); if the data says 1000000
  is fine, keep it and say so.
- Statistics must stay IDENTICAL across backends (parity-pinned fields); any new
  stat field lands in both `rust_mc.rs` and `c_mc.rs` in the same shape.
- `--timeout-sec`: wall-clock only, documented as NOT parity-guaranteed (two
  machines/timings can differ); never used by parity/benchmark assertions.
- OPTIONAL subtask `--threads N`: level-parallel BFS - process one level's
  successors in parallel, but violation selection stays CANONICAL (first violation
  in canonical traversal order wins), so results must be identical to threads=1.
  Parity test at threads 1 vs 2. RISK: if canonical selection proves fragile (e.g.
  race-dependent tie-breaks), DEFER via plan correction - the phase file lists
  parallel search as "where safe".
- Container decision (from task 3's note): if benchmark data shows the sorted-vector
  visited set is the bottleneck, switch C to open addressing here - behavior must
  not change (D-P4-2); record the switch as a plan correction with before/after
  numbers.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 9. | Benchmark suite test | the data driving default tuning + container decision |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Docs sync | documents final stats/flags/timeouts |
| 14. | Close-out | verification includes the (possibly extended) benchmark suite |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/rust_mc.rs` | change | statistics polish; optional `--threads N` level-parallel BFS |
| `src/backends/c_mc.rs` | change | identical statistics polish; container switch if data demands |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/benchmark_mc_test.rs` | change | assert new stat fields parity; threads 1 vs 2 parity (if implemented) |

## Planned actions

### Action 1: Statistics polish
Finalize the statistics block in both backends: memory estimate (same formula),
monitor size, unexpected_terminations, max_depth - identical field set and format.
Extend the benchmark suite to assert parity of every stat field.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | stats polish (Rust) |
| `src/backends/c_mc.rs` | change | stats polish (C, identical fields) |
| `tests/benchmark_mc_test.rs` | change | stat-field parity assertions |

### Action 2: --max-states default decision
Compare task 9's benchmark state counts against the 1000000 default. Keep it (and
record why) or change it via plan correction with before/after data; update usage
text + codegen defaults consistently.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen.rs` | change (only if the default changes) | new default embedded |
| `src/main.rs` | change (only if the default changes) | usage text |

### Action 3: --timeout-sec documentation
Document `--timeout-sec` as wall-clock, non-parity-guaranteed, never used by parity
assertions (usage text; DESIGN via task 13). Behavior already exists from task 2 -
no engine change unless a bug surfaces.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/main.rs` | change | usage text clarification |

### Action 4 (OPTIONAL): --threads N level-parallel BFS
Implement level-parallel successor expansion with CANONICAL violation selection
(first in canonical order wins regardless of thread scheduling); wire the flag in
both backends; add a parity test at threads 1 vs 2 on all benchmark specs. If
canonical selection proves fragile: plan correction deferring `--threads` (phase
file: "parallel search where safe").

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | level-parallel BFS + canonical selection (if proceeding) |
| `src/backends/c_mc.rs` | change | mirror (if proceeding) |
| `tests/benchmark_mc_test.rs` | change | threads 1 vs 2 parity test (if proceeding) |

## Task deliverables
- Identical polished statistics in both backends (test-pinned).
- Documented `--max-states` default decision + non-parity `--timeout-sec`.
- `--threads N` with parity, OR a recorded deferral.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Benchmark suite asserts parity of every statistics field across backends.
- [ ] `--max-states` default: kept (recorded) or changed via plan correction with data.
- [ ] `--timeout-sec` documented as non-parity in usage text.
- [ ] `--threads`: implemented with threads 1 vs 2 parity green, OR deferral recorded as a plan correction.

## Non-goals
- Incremental MC cache for CI (explicitly deferred in the phase file), SMT mode
  (task 11).

## Other information
- Any container switch in C must be validated by re-running the FULL benchmark
  suite (metrics identical, only wall-clock may change) before commit.
