# Phase 4 - Implementation plan

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

### Current state (verified 2026-10-17)

- The `mc` command generates a SELF-CONTAINED binary = production FSM core +
  seeded random fuzz runtime (`MC_RUNTIME` in `src/backends/rust_mc.rs`, mirror
  in `c_mc.rs`). It explores ONE representative input per event variant (zero/
  default payload via `representative_input_expr`), runs `runs` x `bound`
  random steps with the `McPhase` strategy, and reports per-run outcomes
  (`completed`/`stuck`/`unexpected-event`/`assumption-violated:<label>`/
  `property-violation:<label>`, exit 1 on property violation). No visited set,
  no payload enumeration, no completeness.
- Phase 3 online monitors are always included in MC binaries; the sticky
  `PropertyViolation { label, step, trigger_step }` record + trace printing
  already exist and are parity-pinned: `tests/backend_parity_test.rs` (same-
  seed runs over all property-bearing examples must be field-identical) and
  `tests/monitor_integration_test.rs` (injected bug driven with
  `--bound 10 --runs 5 --seed 42`).
- Property language: boolean atoms, `next`/`always`/`always[N]`/
  `bounded_eventually[N]`, formula-level `&&`/`||`/`!`/`->` (task-23 grammar;
  ladder in DESIGN section 6: or > impl > and > unary > primary, with
  `primary_formula` fallthrough to the non-boolean expression level). No
  unbounded liveness operators yet.
- `src/monitor.rs` = shared PSL-to-monitor compiler with restrictions R1-R4;
  out-of-subset formulas produce IDENTICAL skip warnings in both backends.
  New liveness operators will be outside the monitor subset (MC-only).
- Environment: `cmake` present, z3 NOT installed (no headers/lib) - SMT mode
  is a heavy optional task with a feasibility gate (D-P4-12).
- Generated binaries must stay self-contained (no dependency on the RFS
  toolchain crate; C output compiles with plain `gcc -Wall`).

### Design decisions (pinned for implementation)

- D-P4-1 Architecture: per-backend generated engines + ONE shared codegen-time
  plan module `src/mc_plan.rs` (mirrors the Phase 3 MonitorPlan pattern).
  The plan is computed once in the Rust tool and embedded as constants/tables
  in both generated binaries, so behavior is identical by construction.
- D-P4-2 Exact engine: level-synchronous BFS over configurations =
  (state index, packed vars, monitor states). Monitors stay part of the
  configuration (bounded operators need history). Visited check at ENQUEUE
  time; parent pointers (parent config id + input index) for counterexample
  reconstruction. Canonical successor order: events in declaration order,
  payload fields in declaration order, values ascending (bool false<true,
  int 0..max, enum declaration order). Determinism is independent of data-
  structure internals, so Rust and C may use different containers but must
  report identical status/counterexample.
- D-P4-3 Input abstraction: a payload field is abstracted (single
  representative value) iff it is not referenced by any transition guard,
  output payload expression, variable update expression, or property/
  assumption formula (`input_is` patterns with field values count as
  references). Soundness argument goes into DESIGN section 11.
- D-P4-4 Statuses: `pass-complete`, `fail` (reason: `property-violation` |
  `liveness-violation` | `deadlock`), `unknown-state-limit`. Machine-readable
  summary line (parity-checked, exact format):
  `model-check summary: status=<s> states=N transitions=M max_depth=D`
  plus, on fail, ` reason=<r> label=<L> step=S`. Human-facing text maps to
  the phase's `PASS (complete)` / `FAIL` / `UNKNOWN (state limit reached)`.
- D-P4-5 Deadlock/unexpected semantics: a reachable configuration with NO
  successor at all (no enabled transition and no on_error) = FAIL(deadlock).
  Inputs matching no transition while other inputs are enabled terminate that
  path; they are recorded as `unexpected_terminations` in statistics + first
  counterexample but do NOT change the status (RFS treats unexpected events
  as a normal Result-based outcome).
- D-P4-6 Assumptions: active assumptions PRUNE inputs from exploration
  (consistent with Phase 3); `--assumes none` explores everything. Fuzz-mode-
  only flags (`--runs/--seed/--violate/--unexpected-prob`) stay in fuzz mode.
- D-P4-7 Modes and flags: `mc` default = exact; legacy fuzz behind
  `--mode fuzz`. New codegen-time flags (embedded as defaults, overridable at
  runtime - existing pattern): `--mode exact|fuzz`, `--max-states N`
  (default 1000000), `--nondet`, `--timeout-sec N` (default 0 = off;
  documented as NOT parity-guaranteed, never used by parity tests). Optional
  `--threads N` in task 10.
- D-P4-8 Liveness language: `eventually "(" formula ")"` as a new
  `primary_formula` alternative; infix `until` on a NEW level between unary
  and primary: `until_formula = primary_formula { "until" primary_formula }`
  (left-associative; operands are primaries, so `a && b until c` parses as
  `(a && b) until c`). IR: additive `Formula::Eventually { inner }`,
  `Formula::Until { left, right }`; no `IR_VERSION` bump. Semantics: WEAK
  until (`p U q`); strong until is expressible as `p until q && eventually q`.
- D-P4-9 Liveness scope: top-level property obligations `eventually p` /
  `p until q` where p, q are BOOLEAN formulas over atomic propositions (no
  temporal operators inside liveness subformulas); obligations may be
  combined at the top level with `&&`/`||`/`!`/`->`. Deeper nesting ->
  codegen diagnostic (future phase). Any formula containing eventually/until
  is outside the monitor subset -> identical skip warning in production/test
  codegen (MC-only properties).
- D-P4-10 Liveness algorithm (exact, on the COMPLETED reachable graph):
  per-obligation bad-set fixed points. neg(F p) = states from which an
  infinite path avoiding p exists (greatest fixpoint over `!p` with a
  successor staying in the set); G F p has the SAME failure set in finite
  systems (documented). neg(p U q) = backward reachability, staying in `!q`,
  to T = greatest fixpoint(`!q && !p` with a successor in T). Top-level
  boolean combinations via product fixpoints over per-obligation bad sets
  (full formalization written into DESIGN section 11 during task 6).
  Bounded mode (state limit reached): TRUNCATED analysis - sound FAIL if a
  bad cycle is found inside the explored part, otherwise UNKNOWN.
- D-P4-11 Fairness policy: deterministic mode (default) - environment fairness
  by construction (all inputs explored adversarially), no scheduler
  nondeterminism. `--nondet` mode - every enabled transition is a separate
  successor; liveness checked under STRONG TRANSITION FAIRNESS (a bad cycle
  must be fair: every transition enabled at some state of the cycle is taken
  on the cycle). Documented in DESIGN section 11. RISK: if the fairness
  filter proves too complex, plan correction restricts liveness to
  deterministic mode.
- D-P4-12 SMT bounded mode (optional task 11): feasibility gate first (build
  the `z3` crate from source via cmake in a scratch project; C backend would
  link libz3). If infeasible -> plan correction deferring to Phase 5/6 with
  rationale; exit criterion "bounded MC works on larger bitvector specs" is
  then satisfied by bounded exact MC + input abstraction (honest UNKNOWN for
  used large bitvectors, full checking when abstractable), documented in
  PHASE_4_STATUS.md.
- D-P4-13 Benchmarks: three protocol-like specs in `examples/valid` (task 6:
  sliding-window ARQ with an UNUSED `bit<32>` checksum field; task 8: 4-node
  token ring, elevator) + inline FAIL variants inside
  `tests/benchmark_mc_test.rs` (established pattern from
  monitor_integration). Metrics: deterministic counts (states/transitions/
  max_depth/monitor size) asserted for parity + sanity bounds; wall-clock
  recorded as informational only. Report written to `target/mc_benchmarks.json`
  (not checked in).
- D-P4-14 Test migration: `backend_parity_test.rs` +
  `monitor_integration_test.rs` migrate from fuzz (same-seed) to exact mode;
  pinned violation values CHANGE deliberately (deterministic traversal finds
  the first violation in BFS order, not random discovery).

### Exit-criteria -> task mapping

- exact MC completes on medium-sized finite specs in all backends -> tasks 2, 3, 6, 8, 9
- cross-backend differential runs identical on the benchmark suite -> tasks 4, 7, 9
- bounded MC works on larger bitvector specs -> task 10 (limits) + task 11 (SMT, optional; fallback per D-P4-12)
- liveness checkable in at least bounded mode -> tasks 5, 6, 7
- actionable counterexamples -> tasks 2, 3, 6, 7
- complete vs bounded vs unknown reported clearly -> task 2 (D-P4-4)
- benchmark suite with several protocol-like examples -> tasks 8, 9

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Shared MC plan module | `src/mc_plan.rs` (new); `src/lib.rs` (change); `src/codegen_fsm.rs` (change - reuse variant/field info) | Input-abstraction analysis (collect fields referenced by guards, output payload exprs, var update exprs, property/assumption formulas), state-encoding layout (var packing offsets/widths, config byte size), traversal spec (canonical input order, det/nondet successor rules, assumption pruning). Unit tests. Codegen-time foundation computed ONCE so both engines are identical by construction (D-P4-1..3). |
| 2. | Exact engine - Rust backend | `src/backends/rust_mc.rs` (change - rewrite runtime: exact BFS default + fuzz kept); `src/codegen_mc.rs` (change - shared exact-mode concepts: status taxonomy, summary-line format, flag wire formats); `src/backends/rust.rs` (change - MC_INFO options); `src/codegen.rs` (change - new flags); `src/main.rs` (change - usage text); `tests/codegen_test.rs` (change - flag sensitivity tests) | Level-synchronous BFS over configurations with visited set + parent pointers (D-P4-2); statuses + pinned summary line (D-P4-4); deadlock/unexpected semantics (D-P4-5); counterexample reconstruction reusing the Phase 3 trace print format; statistics incl. memory estimate; `--max-states/--nondet/--timeout-sec`; fuzz mode kept behind `--mode fuzz` (D-P4-7). |
| 3. | Exact engine - C backend | `src/backends/c_mc.rs` (change - mirror runtime); `src/backends/c.rs` (change - MC_INFO) | C mirror of task 2: same algorithm, same summary line, same counterexample fields; `gcc -Wall` clean; spot-check parity vs Rust on all property-bearing examples before finishing. Data-structure choice (sorted vector + binary search vs open addressing) decided here from benchmark sizes - behavior must not depend on it (D-P4-2). |
| 4. | Parity suite migration to exact mode | `tests/backend_parity_test.rs` (change); `tests/monitor_integration_test.rs` (change) | Migrate both suites from same-seed fuzz to exact mode: identical status + counterexample fields across backends on ALL property-bearing examples; re-pin violation values from the deterministic traversal (D-P4-14); injected-bug test runs seedless. |
| 5. | Liveness language extension | `src/parser.rs` (change - `until_formula` level + `eventually` primary); `src/ir.rs` (change - additive `Formula::Eventually`/`Until`); `src/printer.rs` (change - round-trip); `src/validator.rs` (change - typing); `src/monitor.rs` (change - liveness operators outside the subset, identical warning) | Grammar per D-P4-8 (EBNF written into the task file); additive IR variants, no version bump; weak-until semantics; lib-level round-trip + parser + validator tests with inline formulas. NO example files yet - examples land with engine support in task 6 so the tree stays green at every step. |
| 6. | Liveness engine - Rust + first liveness example | `src/mc_plan.rs` (change - liveness obligation decomposition per D-P4-9); `src/backends/rust_mc.rs` (change - fixpoint analysis + counterexamples); `examples/valid/43_sliding_window.rfs` (new) | Per-obligation bad-set fixpoints on the completed graph (D-P4-10); liveness-violation counterexample = prefix path + cycle, same print format; bounded mode = truncated analysis (sound FAIL else UNKNOWN); `--nondet` strong-fairness filter per D-P4-11 (or plan correction). First protocol-like example: sliding-window ARQ with safety + `eventually`/`until` properties and an unused `bit<32>` checksum field (exercises input abstraction). |
| 7. | Liveness engine - C + parity | `src/backends/c_mc.rs` (change - mirror analysis); `tests/backend_parity_test.rs` (change - liveness coverage) | C mirror of task 6; liveness counterexample fields identical to Rust; parity suite now covers the liveness example(s) in exact mode. |
| 8. | Benchmark specs | `examples/valid/44_token_ring.rfs` (new); `examples/valid/45_elevator.rfs` (new) | Two more protocol-like medium-size specs with safety + liveness properties and abstraction-stress payload fields; the file-driven example suite picks them up automatically (parse+validate+round-trip). |
| 9. | Benchmark suite test | `tests/benchmark_mc_test.rs` (new); `BUILD.md` (change - suite row) | Runs exact MC on all benchmark specs in BOTH backends: asserts expected statuses, parity of deterministic metrics (states/transitions/max_depth), sanity bounds; inline FAIL variants with known violations (D-P4-13); writes `target/mc_benchmarks.json` + prints the per-backend table. |
| 10. | Performance features | `src/backends/rust_mc.rs` (change); `src/backends/c_mc.rs` (change); `tests/benchmark_mc_test.rs` (change) | Statistics polish (memory estimate, monitor size), `--max-states` default tuned from benchmark data, `--timeout-sec` documented as non-parity. OPTIONAL subtask: `--threads N` level-parallel BFS with canonical violation selection + parity test at threads 1 vs 2; defer via plan correction if parity risk emerges. |
| 11. | SMT bounded mode (OPTIONAL) | `Cargo.toml` (change - z3 dep, feature-gated); `src/backends/rust_mc.rs` or `src/backends/rust_smt.rs` (new/change); `src/backends/c_mc.rs` (change); docs per task 13 | Feasibility gate FIRST: build the `z3` crate from source in a scratch project; if it fails -> plan correction deferring to Phase 5/6 with rationale (D-P4-12). If proceeding: `--mode smt-bounded`, k-liveness for liveness, results clearly labeled incomplete, identical CLI semantics + results in both backends. |
| 12. | Training data update | `training/properties_temporal_logic/rfs_training.jsonl` (change); `training/model_checking_properties/rfs_training.jsonl` (change); `training/<cat>/snippets/**` + `all_rfs_training.jsonl` (regenerate) | SFT examples: `eventually`/`until` syntax + weak-until semantics, exact MC statuses (`pass-complete`/`fail`/`unknown-state-limit`), input abstraction, liveness counterexamples; regenerate snippets + aggregate; `cargo test-training` green. |
| 13. | Docs sync | `DESIGN.md` (change - sections 5/6/7, section 11 REWRITTEN for the exact engine, section 12 subset table, NEW section 22 "Phase 4 model-checking decisions" D-P4-1..N); `KB.md` (change); `BUILD.md` (change); `CODEGEN.md` (change); `src/backends/RUST.md` (change); `src/backends/C.md` (change); `START.md` (change - section 1 snapshot) | Sync all design-relevant docs to the final Phase 4 state: exact engine architecture, statuses, liveness algorithm + fairness policy, input-abstraction soundness, SMT outcome (implemented or deferred), benchmark suite. |
| 14. | Close-out | `docs/roadmap/PHASE_4_STATUS.md` (new); `docs/roadmap/INTRO.md` (change - state table row 4); `TASKS.md` (change) | Full verification per BUILD.md section 5 + `gcc -Wall` spot check of the NEW C MC output; `PHASE_4_STATUS.md` per template 2.3 with exact counts; state-table row 4 -> COMPLETE; TASKS.md entry; single commit; clean tree. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| - | - | - | (none yet) |

## Other information

- Verification commands (toolchain activation per START.md section 4, multi-line):
  ```sh
  cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check
  cargo generate-code-tests && cargo test-generated-code
  cargo run -- validate --input rfs examples/valid   # exit 0 expected
  gcc -Wall <generated C mc output>                  # spot check per task
  ```
- Commit grouping: one commit per executed task (protocol step t), established
  Phase 3 practice; the close-out commit covers bookkeeping files only.
- Risks:
  1. C engine complexity (visited set in generated C) - keep it simple first
     (sorted vector + binary search); switch to open addressing only if
     benchmarks demand it (task 3 decision).
  2. `--nondet` strong-fairness filter complexity - fallback: liveness in
     deterministic mode only (plan correction, D-P4-11).
  3. z3 build feasibility in this environment (cmake present, no z3 installed)
     - gated in task 11 (D-P4-12).
  4. Pinned test values change deliberately in task 4 (D-P4-14) - re-pin from
     actual deterministic output, never hand-computed.
- Explicitly deferred within this phase: incremental MC cache for CI (phase
  file says "maybe later"), wall-clock timeout in parity-guaranteed mode
  (`--timeout-sec` exists but is documented as non-parity, D-P4-7).
- Open questions: none blocking. The `until` precedence choice (operands are
  primaries; left-associative) and weak-until default are pinned by D-P4-8;
  if a user-facing surprise emerges during examples work, record it as a plan
  correction rather than silently changing the grammar.
