# Phase 4, Task 13 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Sync all design-relevant documentation to the FINAL Phase 4 state: exact engine
architecture, statuses, liveness algorithm + fairness policy, input-abstraction
soundness, SMT outcome (implemented or deferred), benchmark suite.

## Context

- Docs per plan: `DESIGN.md` (sections 5/6/7; section 11 REWRITTEN for the exact
  engine - task 6 already wrote the liveness formalization draft, this task
  finalizes it with the rest; section 12 subset table; NEW section 22 "Phase 4
  model-checking decisions" D-P4-1..N), `KB.md`, `BUILD.md`, `CODEGEN.md`,
  `src/backends/RUST.md`, `src/backends/C.md`, `START.md` (section 1 snapshot).
- Use the KB.md doc map / task routing to find each doc's relevant sections; read
  targeted chunks only (grep for headers first) - do not load whole large docs.
- Ground truth = the implemented behavior + the pinned decisions D-P4-1..N in
  PHASE_4_PLAN.md + any plan corrections recorded during execution (tasks 5/6/10/11
  may have amended details - check the plan's "Plan corrections" table first).
- Stale-content sweep: after writing, grep the docs for pre-Phase-4 claims that
  must go: MC as "seeded random fuzz / one representative input per event" as the
  default description, old status names (`completed`/`stuck`/...) as the primary
  taxonomy, "z3 not installed", fuzz-as-default flag descriptions.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2.-12. | All implementation tasks | docs describe the FINAL state, including the SMT outcome and any plan corrections |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | sections 5/6/7 updated; section 11 REWRITTEN for the exact engine (finalizes task 6's liveness formalization + abstraction soundness); section 12 subset table (liveness operators outside the monitor subset); NEW section 22 "Phase 4 model-checking decisions" D-P4-1..N |
| `KB.md` | change | MC architecture/statuses/flags entries; doc-map rows if sections moved |
| `BUILD.md` | change | exact-mode commands/flags, benchmark suite row (added in task 9 - verify), SMT feature note if implemented |
| `CODEGEN.md` | change | new codegen flags + MC_INFO options documented |
| `src/backends/RUST.md` | change | Rust MC runtime: exact engine, statuses, counterexamples, (SMT mode if implemented) |
| `src/backends/C.md` | change | C MC runtime mirror notes (containers, gcc -Wall bar, libz3 link if SMT) |
| `START.md` | change | section 1 state snapshot: Phase 4 complete summary + known-good baseline numbers |

## Planned actions

### Action 1: Read the final state
Check PHASE_4_PLAN.md's "Plan corrections" table (all amendments from tasks
5/6/10/11), the current flag surface (`src/codegen.rs`), and the benchmark suite
output - docs must match what actually shipped, not what was originally planned.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - reading only) | - | ground-truth collection |

### Action 2: DESIGN.md
Update sections 5/6/7 (property language incl. `eventually`/`until` + the until
precedence level; MC section for exact engine); REWRITE section 11 around the exact
engine (finalizing task 6's liveness formalization, fairness policy, abstraction
soundness argument, statuses/summary line); update section 12's monitor-subset
table (liveness operators outside the subset with the identical skip warning); add
NEW section 22 "Phase 4 model-checking decisions" recording D-P4-1..N as shipped
(incl. any corrections).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (sections 5/6/7) | language + MC descriptions current |
| `DESIGN.md` | change (section 11, rewrite) | exact engine + liveness formalization final |
| `DESIGN.md` | change (section 12) | monitor subset table: liveness outside |
| `DESIGN.md` | change (new section 22) | D-P4-1..N as shipped |

### Action 3: KB.md + CODEGEN.md
KB.md: MC architecture/statuses/flags entries + doc-map consistency. CODEGEN.md:
new codegen flags, MC_INFO options, per-backend notes.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change | MC-related entries current |
| `CODEGEN.md` | change | flag surface documented |

### Action 4: Backend docs + BUILD.md
`src/backends/RUST.md` + `src/backends/C.md`: exact engine runtime descriptions,
statuses/counterexamples, container notes (C), SMT mode if implemented. BUILD.md:
verify the benchmark suite row (task 9) + exact-mode flag examples + SMT feature
note if implemented.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/RUST.md` | change | Rust MC runtime docs |
| `src/backends/C.md` | change | C MC runtime docs |
| `BUILD.md` | change | commands/flags verification + SMT note if applicable |

### Action 5: START.md snapshot + stale sweep
Update START.md section 1 state line (Phase 4 complete, new baseline numbers -
final counts come from task 14's verification run; use the latest known numbers and
let task 14 confirm). Then grep all docs for stale pre-Phase-4 claims (Context
list) and fix any remainder.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `START.md` | change | section 1 state snapshot |
| (any doc with stale claims) | change | stale-content sweep fixes |

## Task deliverables
- All listed docs consistent with the shipped Phase 4 behavior; DESIGN section 22
  records the decisions as shipped.

## Task exit criteria
- [ ] DESIGN.md sections 5/6/7/11/12 current + new section 22 present (D-P4-1..N as shipped, incl. corrections).
- [ ] KB.md, CODEGEN.md, RUST.md, C.md, BUILD.md, START.md updated per the tables above.
- [ ] Stale sweep: no doc describes MC as fuzz-default, old status taxonomy as primary, or "z3 not installed" (unless describing history explicitly).
- [ ] Cross-references between docs resolve (section numbers cited actually exist).

## Non-goals
- Roadmap bookkeeping (INTRO.md state table row + PHASE_4_STATUS.md are task 14),
  training data (task 12).

## Other information
- If a plan correction changed a pinned decision, section 22 records the FINAL
  value with a pointer to the correction - do not silently overwrite history.
