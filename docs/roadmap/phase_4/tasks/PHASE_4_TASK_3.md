# Phase 4, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Mirror the exact BFS engine of task 2 in the generated C backend: same algorithm,
same summary line, same counterexample fields, `gcc -Wall` clean, with a parity
spot check against Rust on all property-bearing examples before finishing.

## Context

- `src/backends/c_mc.rs` mirrors `rust_mc.rs` structurally (Phase 2/3 pattern); the
  C runtime is generated from the same shared concepts in `src/codegen_mc.rs` that
  task 2 established - render from those, do not re-invent formats.
- D-P4-2: determinism must be independent of data-structure internals. Rust uses a
  hash set; C starts with a SORTED VECTOR + BINARY SEARCH for the visited set
  (simplest correct choice). Behavior must not depend on the container - if task 9's
  benchmarks show it is too slow, task 10 may switch to open addressing (plan
  correction recording the switch).
- The plan tables from task 1 are embedded as C constants/arrays exactly as in Rust.
- Hard requirement: generated C compiles with plain `gcc -Wall` WITHOUT warnings
  (established Phase 2 bar; the parity suite compiles C output with gcc).
- Parity spot check before finishing: for EVERY property-bearing example, run the
  Rust and C MC binaries in exact mode and compare status + summary-line fields +
  counterexample fields - must be identical.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Exact engine - Rust | the algorithm, formats, and flags being mirrored; parity reference |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | Parity suite migration | both backends must be exact before the suites migrate |
| 7. | Liveness engine - C | mirrors task 6 on top of this engine |
| 9. | Benchmark suite test | drives both backends |
| 10. | Performance features | may switch the visited-set container |
| 11. | SMT bounded mode (optional) | C links libz3 alongside this engine |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/c_mc.rs` | change | mirror runtime: exact BFS default + fuzz kept behind `--mode fuzz` |
| `src/backends/c.rs` | change | MC_INFO options for the new flags |

## Planned actions

### Action 1: C exact BFS runtime
Mirror task 2's algorithm in generated C: configuration byte buffer from the
embedded plan tables; level-synchronous BFS; visited set = sorted vector of config
bytes + binary search (documented choice, behavior must not depend on it); parent
pointer arrays (parent config id + input index); canonical successor enumeration;
monitor stepping via the existing Phase 3 C monitor code; assumption pruning;
deadlock detection; `--max-states`/`--timeout-sec` handling identical to Rust.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | exact BFS runtime replacing the default; fuzz runtime kept under `--mode fuzz` |

### Action 2: Identical wire formats
Render statuses, the pinned summary line, exit codes (0/1/2), statistics fields
(incl. memory estimate with the SAME formula as Rust), and counterexamples (Phase 3
trace format) from the shared `src/codegen_mc.rs` concepts - byte-identical output
for identical runs.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | summary line / stats / counterexample rendering |
| `src/backends/c.rs` | change | MC_INFO options for the new flags |

### Action 3: gcc -Wall clean + parity spot check
Compile every generated C MC output with `gcc -Wall` - zero warnings. Then, for ALL
property-bearing examples: run Rust and C exact-mode binaries, diff status +
summary-line fields + counterexample fields; any mismatch is a task-3 bug (fix the
C rendering/algorithm), never a format change on the Rust side.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | fixes surfaced by the spot check |

## Task deliverables
- Exact BFS engine in generated C binaries (default mode); fuzz preserved.
- Byte-identical wire formats vs Rust; `gcc -Wall` clean.
- Parity spot check green on all property-bearing examples (recorded in status).

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] All generated C MC outputs compile with `gcc -Wall` without warnings.
- [ ] Parity spot check: status + summary-line fields + counterexample fields identical Rust vs C on ALL property-bearing examples (exact mode).
- [ ] Fuzz mode in C still works (same-seed spot check).

## Non-goals
- Liveness analysis (task 7), container switch / performance tuning (task 10),
  SMT (task 11).

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| `src/backends/c_fsm.rs` added to affected files | Nondet successor enumeration needs every enabled transition's effect, which the core's first-match `fsm_step` cannot expose. Added `pub(crate) fn render_mc_step_all_c(module)` (plus a private `mc_pattern_cond` mirroring the pattern part of `transition_block_c`) that renders a generated `mc_step_all(state, vars, in, out) -> int` from the same IR with the same helpers (`gen_expr_c`, `output_expr_c`, `Ctx`/`Env`) as `fsm_step`, so det (first effect) and nondet (all effects) agree by construction. |
| Pre-existing `gcc -Wall` warnings in core output fixed | The task's hard requirement is zero warnings on generated C MC output, but the Phase 3 core already produced `-Wmaybe-uninitialized` warnings (`output_t outs[MAX_OUTPUTS];` passed to `rfs_commit` with `n_out = 0` for emit-less transitions) - reproduced on the pristine tree. Fixed in `transition_block_c`/on_error rendering by zero-initializing: `output_t outs[MAX_OUTPUTS] = {0};`. Generated core text changes, behavior identical; all 513 generated tests still pass. |
| Parity suite bar vs task-3 bar | `tests/backend_parity_test.rs::compile_c` only asserts gcc success (not zero warnings); the stricter no-warning bar was enforced manually: all 36 generated C MC outputs compile with plain `gcc -Wall` at zero warnings. The suite itself is untouched (task 4 migrates it). |
| Shared printers moved ahead of the exact engine | `print_vars`/`print_trace` were defined inside the fuzz runtime; the exact engine references them, so they moved to a shared section emitted before both runtimes (single definition, same text as Phase 3 - C trace format unchanged). The fuzz runtime's `main` became `run_fuzz(argc, argv)` with a new dispatching `main`, mirroring task 2. |

## Other information
- If the sorted-vector visited set makes any example unreasonably slow (> ~60s),
  note it in the status file - task 10 decides on open addressing with benchmark
  data; do not switch preemptively.
- C integer widths: use fixed-width types (`uint32_t`/`uint64_t`) matching the plan
  encoding so config bytes are identical to Rust's.
