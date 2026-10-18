# Phase 4, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Replace the default MC runtime in the Rust backend with a level-synchronous EXACT
BFS engine (visited set, parent pointers, statuses, counterexamples, statistics)
driven by the task-1 plan; keep the legacy fuzz runtime behind `--mode fuzz`
(D-P4-2..7).

## Context

- Current runtime (`MC_RUNTIME`, `src/backends/rust_mc.rs`): seeded random execution,
  one representative input per event variant, `runs` x `bound` steps with the
  `McPhase` strategy, per-run outcomes, exit 1 on property violation. It stays
  available unchanged under `--mode fuzz`.
- Exact engine (D-P4-2): level-synchronous BFS over configurations = (state index,
  packed vars, monitor states) using the task-1 encoding. Visited check at ENQUEUE
  time; parent pointers = (parent config id + input index) for counterexample
  reconstruction. Canonical successor order from the plan. Determinism is
  independent of data-structure internals - Rust may use a hash set; C (task 3)
  may use different containers; results must be identical.
- Statuses + wire format (D-P4-4), exact summary line (parity-checked):
  `model-check summary: status=<s> states=N transitions=M max_depth=D`
  plus, on fail, ` reason=<r> label=<L> step=S`.
  `<s>` in {`pass-complete`, `fail`, `unknown-state-limit`};
  `<r>` in {`property-violation`, `liveness-violation`, `deadlock`} (liveness lands
  in task 6). Human-facing text maps to `PASS (complete)` / `FAIL` /
  `UNKNOWN (state limit reached)`.
- Exit codes (pinned here): 0 = pass-complete, 1 = fail (any reason), 2 =
  unknown-state-limit.
- Deadlock/unexpected semantics (D-P4-5): a reachable configuration with NO
  successor at all (no enabled transition and no on_error) = FAIL(deadlock).
  Inputs matching no transition while other inputs are enabled terminate that path;
  they are counted in statistics (`unexpected_terminations`) + shown in the first
  counterexample but do NOT change the status.
- Assumptions (D-P4-6): active assumptions prune inputs from exploration;
  `--assumes none` explores everything. Fuzz-only flags (`--runs/--seed/--violate/
  --unexpected-prob`) stay in fuzz mode only.
- Modes/flags (D-P4-7): `mc` default = exact. Codegen-time flags embedded as
  defaults, overridable at runtime (existing pattern): `--mode exact|fuzz`,
  `--max-states N` (default 1000000), `--nondet`, `--timeout-sec N` (default 0 =
  off; documented as NOT parity-guaranteed, never used by parity tests).
- Counterexamples: reconstruct the input sequence via parent pointers and print it
  with the EXISTING Phase 3 trace format (sticky `PropertyViolation { label, step,
  trigger_step }` record + trace printing already exist and are parity-pinned -
  reuse, do not redesign).
- Shared concepts live in `src/codegen_mc.rs` (status taxonomy, summary-line
  format, flag wire formats) so task 3 renders identical C from the same source of
  truth.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Shared MC plan module | provides encoding layout + canonical traversal spec |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Exact engine - C | mirrors this runtime; parity is the acceptance bar |
| 4. | Parity suite migration | migrates both suites to the exact mode built here |
| 6. | Liveness engine - Rust | fixpoint analysis runs on the graph this engine builds |
| 9. | Benchmark suite test | drives exact MC in both backends |
| 10. | Performance features | tunes limits/stats on top of this engine |
| 11. | SMT bounded mode (optional) | adds a third mode alongside exact/fuzz |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/backends/rust_mc.rs` | change | rewrite runtime: exact BFS default + fuzz kept behind `--mode fuzz` |
| `src/codegen_mc.rs` | change | shared exact-mode concepts: status taxonomy, summary-line format, flag wire formats |
| `src/backends/rust.rs` | change | MC_INFO options for the new flags |
| `src/codegen.rs` | change | new codegen-time flags (`--mode/--max-states/--nondet/--timeout-sec`) |
| `src/main.rs` | change | usage text |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | flag sensitivity tests for the new flags |

## Planned actions

### Action 1: Shared exact-mode concepts in codegen_mc.rs
Define the status taxonomy, the exact summary-line format (D-P4-4 string), and the
flag wire formats (codegen-time default + runtime override) as shared constants/
helpers used by both backends' renderers. This is the single source of truth task 3
mirrors from.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen_mc.rs` | change | status enum, summary-line format, flag wire formats |

### Action 2: Exact BFS runtime (rust_mc.rs)
Rewrite the generated Rust runtime: configuration as plan-encoded byte buffer;
level-synchronous BFS with a visited hash set (check at ENQUEUE) + parent-pointer
arrays (parent config id + input index); canonical successor enumeration from the
embedded plan tables; monitor stepping via the existing Phase 3 monitor code;
assumption pruning; deadlock detection (config with no successors); `--max-states`
limit -> `unknown-state-limit`; `--timeout-sec` wall-clock check (non-parity,
documented). Statistics: states, transitions, max_depth, unexpected_terminations,
memory estimate.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | exact BFS runtime replacing the default; fuzz runtime kept intact under `--mode fuzz` |

### Action 3: Statuses, summary line, counterexamples
Emit the pinned summary line + human-facing text; exit codes 0/1/2 (Context).
Counterexample reconstruction: walk parent pointers to the root, render the input
sequence with the existing Phase 3 trace format; property violations reuse the
sticky record fields (`label/step/trigger_step`); deadlock prints the path to the
dead configuration.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | summary line, exit codes, counterexample printing |

### Action 4: Flag wiring
New codegen-time flags in `src/codegen.rs` (embedded as defaults, runtime-overridable
per the existing pattern), MC_INFO options in `src/backends/rust.rs`, usage text in
`src/main.rs`. Fuzz-only flags remain valid only in fuzz mode.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen.rs` | change | `--mode/--max-states/--nondet/--timeout-sec` codegen flags |
| `src/backends/rust.rs` | change | MC_INFO options |
| `src/main.rs` | change | usage text |

### Action 5: Flag sensitivity tests
Extend `tests/codegen_test.rs`: each new flag changes the generated output as
expected (codegen-time default embedded; runtime override present); fuzz mode
generation unchanged for existing flag combinations.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change | new flag sensitivity tests |

## Task deliverables
- Exact BFS MC engine in generated Rust binaries (default mode); fuzz preserved.
- Pinned summary line + statuses + exit codes; counterexamples in the Phase 3 trace
  format; statistics incl. memory estimate.
- New flags wired through codegen/CLI with sensitivity tests.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Hand-run on a small spec: `pass-complete` (exit 0); injected-bug spec -> `fail` + property-violation counterexample (exit 1); tiny `--max-states` -> `unknown-state-limit` (exit 2).
- [ ] Summary line matches the D-P4-4 format byte-for-byte.
- [ ] Fuzz mode: same-seed run reproduces the pre-task behavior (spot check one property-bearing example).
- [ ] Flag sensitivity tests pass; usage text updated.

## Non-goals
- C backend mirror (task 3), parity-suite migration (task 4), liveness analysis
  (tasks 5-7), `--threads` (task 10), SMT mode (task 11).

## Other information
- Do NOT change the Phase 3 trace-print format - it is parity-pinned by
  `tests/backend_parity_test.rs`; task 4 re-pins VALUES, not formats.
- Memory estimate: bytes for visited set + parent arrays at peak level; keep the
  formula simple and identical in C (task 3).
- Keep the fuzz runtime byte-compatible where possible so `--mode fuzz` output is
  unchanged (helps task 4's migration diff stay readable).
