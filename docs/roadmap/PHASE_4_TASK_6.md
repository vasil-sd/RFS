# Phase 4, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Implement the liveness engine in the Rust MC backend: per-obligation bad-set
fixpoints on the completed reachable graph (D-P4-10), liveness-violation
counterexamples (prefix + cycle), bounded-mode truncation, and the `--nondet`
strong-fairness filter (D-P4-11) - plus the first protocol-like example with
liveness properties (`examples/valid/43_sliding_window.rfs`).

## Context

- Runs on top of task 2's exact BFS: the engine first completes the reachable graph
  (or hits `--max-states`), then runs liveness analysis on it.
- Obligation decomposition (D-P4-9, computed in `src/mc_plan.rs`): top-level
  property formulas decompose into obligations `eventually p` / `p until q` where
  p, q are BOOLEAN formulas over atomic propositions (no temporal operators inside);
  obligations may be combined at the top level with `&&`/`||`/`!`/`->`. Deeper
  nesting -> CODEGEN DIAGNOSTIC (clear message; not a parse error).
- Algorithm (D-P4-10), on the COMPLETED reachable graph:
  - neg(eventually p) = states from which an infinite path avoiding p exists
    (greatest fixpoint over `!p` with a successor staying in the set); G F p has
    the SAME failure set in finite systems (documented).
  - neg(p until q) = backward reachability, staying in `!q`, to T = greatest
    fixpoint(`!q && !p` with a successor in T).
  - Top-level boolean combinations via product fixpoints over per-obligation bad
    sets.
  VERIFICATION REQUIREMENT: before wiring into the engine, check each fixpoint
  equation against 2-3 hand-computed tiny graphs (record them in the status file);
  the WEAK-until semantics of D-P4-8 are ground truth - if an implemented equation
  diverges from D-P4-10's text, record a plan correction with the corrected
  equation. The full formalization is written into DESIGN section 11 during this
  task (task 13 finalizes all docs).
- Counterexample: liveness violation = PREFIX PATH + CYCLE, printed with the same
  Phase 3 trace format as safety violations (status `fail`, reason
  `liveness-violation`, label/step fields per D-P4-4).
- Bounded mode (state limit reached): TRUNCATED analysis - sound FAIL if a bad
  cycle is found inside the explored part, otherwise `unknown-state-limit` (D-P4-10).
- Fairness (D-P4-11): deterministic mode (default) - environment fairness by
  construction (all inputs explored adversarially), no scheduler nondeterminism.
  `--nondet` - every enabled transition is a separate successor; liveness checked
  under STRONG TRANSITION FAIRNESS (a bad cycle must be fair: every transition
  enabled at some state of the cycle is taken on the cycle). RISK: if the fairness
  filter proves too complex, plan correction restricts liveness to deterministic
  mode.
- First example (D-P4-13): `examples/valid/43_sliding_window.rfs` - sliding-window
  ARQ protocol: states for send/ack/retransmit phases, events with payloads (incl.
  an UNUSED `bit<32>` checksum field to exercise input abstraction), safety
  properties (e.g. no duplicate delivery within the window) + liveness properties
  (`eventually`/`until`, e.g. every sent frame is eventually acknowledged). The spec
  must be CORRECT (exact MC -> pass-complete); FAIL variants are inline in task 9's
  test file, not in examples/valid.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Shared MC plan module | extended here with obligation decomposition |
| 2. | Exact engine - Rust | the graph builder + runtime being extended |
| 5. | Liveness language extension | parser/IR/validator for `eventually`/`until` |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | Liveness engine - C | mirrors this analysis; parity on liveness examples |
| 9. | Benchmark suite test | asserts liveness statuses/metrics on the benchmark specs |
| 12. | Training data update | liveness counterexamples + semantics in SFT data |
| 13. | Docs sync | finalizes DESIGN section 11 (formalization written here) |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc_plan.rs` | change | liveness obligation decomposition per D-P4-9 (+ nesting diagnostic spec) |
| `src/backends/rust_mc.rs` | change | fixpoint analysis + liveness counterexamples on the completed graph |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | section 11: full liveness formalization (fixpoint equations, fairness policy, abstraction soundness argument) - task 13 finalizes |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `examples/valid/43_sliding_window.rfs` | new | first protocol-like example: safety + liveness properties, unused `bit<32>` checksum field |

## Planned actions

### Action 1: Obligation decomposition in mc_plan.rs
Extend the plan with per-property obligation extraction: decompose top-level
formulas into `eventually p` / `p until q` obligations (p, q boolean over atoms) +
the top-level boolean combination structure; emit the codegen diagnostic for deeper
nesting (D-P4-9). Unit tests with inline formulas.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | change | obligation decomposition + nesting diagnostic (+ tests) |

### Action 2: Fixpoint analysis in rust_mc.rs
After BFS completes the graph, run per-obligation bad-set fixpoints (D-P4-10
equations, verified per Context), combine via product fixpoints over per-obligation
bad sets; emit `fail`/`liveness-violation` with label + counterexample. Bounded
mode: truncated analysis - sound FAIL on a bad cycle inside the explored part, else
`unknown-state-limit`.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | fixpoint engine + bounded truncation |

### Action 3: Liveness counterexamples
Reconstruct prefix path + cycle from the bad set (cycle = strongly connected part
of the bad set reachable from the initial state); print with the Phase 3 trace
format; fields identical in shape to safety counterexamples (parity-checked in
task 7).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | prefix+cycle reconstruction and printing |

### Action 4: --nondet strong-fairness filter
Implement the fairness filter per D-P4-11 (bad cycle must take every transition
enabled at some of its states). If it proves too complex: plan correction
restricting liveness to deterministic mode (safety unaffected either way).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | fairness filter for `--nondet` liveness |

### Action 5: DESIGN section 11 formalization
Write the full liveness formalization into DESIGN section 11: fixpoint equations
(as implemented), weak-until semantics, G F p = F p failure-set equivalence in
finite systems, strong transition fairness policy, and the input-abstraction
soundness argument (D-P4-3). Task 13 finalizes wording with the rest of the docs.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change | section 11 liveness formalization |

### Action 6: Example 43 - sliding-window ARQ
Design and write `examples/valid/43_sliding_window.rfs`: medium protocol spec with
safety + `eventually`/`until` properties and an unused `bit<32>` checksum payload
field (exercises input abstraction). Must parse+validate+round-trip (file-driven
example suite) and MC to `pass-complete` in Rust exact mode; record the state count
in the status file (input for tasks 9/10 sizing).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/43_sliding_window.rfs` | new | first liveness-bearing benchmark spec |

## Task deliverables
- Liveness engine in Rust MC binaries (fixpoints, counterexamples, bounded
  truncation, `--nondet` fairness or documented restriction).
- DESIGN section 11 formalization (draft for task 13).
- `examples/valid/43_sliding_window.rfs`, pass-complete in exact mode.

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Fixpoint equations verified against 2-3 hand-computed tiny graphs (recorded in status file); any divergence from D-P4-10 text recorded as a plan correction.
- [ ] Example 43: example suite green (43 valid / 12 invalid), exact MC -> pass-complete, state count recorded.
- [ ] Liveness-violation counterexample = prefix + cycle in the Phase 3 trace format (hand-run on an inline broken variant).
- [ ] Bounded mode: tiny `--max-states` on a liveness spec -> sound FAIL or `unknown-state-limit`, never a false pass-complete.
- [ ] `--nondet` liveness works with the fairness filter, OR plan correction restricting liveness to deterministic mode is recorded.
- [ ] Deeper-nesting formulas produce the codegen diagnostic (not a crash).

## Non-goals
- C mirror + parity (task 7), more benchmark specs (task 8), benchmark assertions
  (task 9), docs beyond the DESIGN section 11 formalization (task 13).

## Other information
- Keep safety and liveness code paths separate in the generated runtime so a
  liveness bug cannot corrupt safety results (and vice versa).
- The example's unused `bit<32>` checksum field is deliberate: it must be ABSTRACTED
  by task 1's analysis - verify the plan marks it abstracted (spot check via
  generated constants or a debug print during development, removed before commit).
