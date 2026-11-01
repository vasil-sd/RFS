# Phase 3, Task 21 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 21 (+ correction 11a): sync ALL design-relevant docs with Phase 3 -
decisions D-P3-1..D-P3-6, monitor semantics, R1-R4, CLI flag, backend internals
(including the C.md/RUST.md updates deferred here), and the fixed property-formula
grammar from task 23.

## Context

- Decisions + R1-R4 source of truth: `docs/roadmap/PHASE_3_PLAN.md` section
  "Other information" (D-P3-1..D-P3-6, monitor-compilability restrictions, exact
  D-P3-3 deadline semantics with worked example). DESIGN.md section 20 shows the
  decision-table style to copy (comparison tables, D-numbering).
- `src/backends/C.md` and `src/backends/RUST.md` currently have ZERO monitor content
  (verified 2026-10-16) - correction 11a folds the C.md update into this task.
- The grammar fix from task 23 (formula-level `&&`/`||` operators; the `primary_formula`
  expression fallthrough is restricted to the non-boolean expression level, so mixed atom
  kinds like `state == S && output_has(O)` now parse) must be documented in KB.md and
  reflected in DESIGN.md's formula EBNF.
- Stale claims to hunt for: "post-hoc" property/assumption checks (removed from both
  backends, D-P3-4), "liveness properties skipped with warning" (now online-monitored),
  old test/example/training counts (use task 18/20 final numbers).
- Read large docs surgically (START.md rule 2: grep section map + sed ranges).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13.-20. | All implementation + training done | docs describe the final state; counts come from tasks 18/20 |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | section 5.10 `always[N]` + monitor semantics + formula EBNF update for task 23; section 7 IR note (additive variant); section 12 implemented subset; NEW section 21 "Phase 3 property/monitor decisions" with D-P3-1..D-P3-6 tables |
| `KB.md` | change | sections 2.6, 3.9, 4.2 (add src/monitor.rs), 1 state table, 7 limitations (post-hoc note -> online monitors in both backends) + task-23 grammar documentation |
| `BUILD.md` | change | `--with-property-monitors` CLI row, updated test counts (tasks 18/20), parity suite row |
| `CODEGEN.md` | change | BackendOptions.with_property_monitors field, monitor conventions (shared MonitorPlan rendering), testing conventions incl. cross-backend parity tests |
| `src/backends/RUST.md` | change | Sample struct, Mon_* monitor structs, StepError::PropertyViolation, debug gating, post-hoc check removal |
| `src/backends/C.md` | change | sections 4-5: monitor parity (deferred from task 11 per correction 11a), RFS_PROPERTY_VIOLATION status, RFS_PROPERTY_MONITORS define |
| `START.md` | change | section 1 snapshot: counts + "next work -> Phase 4" |

## Planned actions

### Action 1: DESIGN.md - decisions + semantics

Add `always[N]` to the property-language section (5.10) with exact D-P3-3 monitor
semantics (obligation trigger/deadline algorithm, worked example, `always[N]` window,
top-level `be[N]` pending-from-0, `next` delay register vacuous at sample 0); IR note in
section 7 (additive serde variant `bounded_always`, no version bump - D-P3-1); mark the
implemented subset in section 12; update the formula EBNF for the task-23 grammar fix
(primary formula = keyword atoms | parenthesized | non-boolean expression; `&&`/`||`
at the formula level); new section 21 "Phase 3 property/monitor decisions" with
comparison tables for D-P3-1..D-P3-6 in the D1-D10 style of section 20.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (sections 5.10, 7, 12; new section 21) | design-relevant decisions must live in DESIGN.md (KB 6.6 / START rule 8) |

### Action 2: KB.md - reference + limitations + task-23 grammar

Update sections 2.6/3.9/4.2 (property syntax, source map incl. `src/monitor.rs`),
section 1 state table (Phase 3 complete counts after task 20), section 7 limitations:
replace the post-hoc assumption-check note with online monitors in BOTH backends,
remove "liveness skipped" wording; document the task-23 grammar fix in the property
syntax section (`&&`/`||` are formula-level operators in properties; expression operands
parse at the non-boolean level, so mixed atom kinds like `state == S && output_has(O)`
parse; pinned by parser unit tests).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (sections 1, 2.6, 3.9, 4.2, 7) | KB is the syntax/source reference - must match the fixed grammar and code |

### Action 3: BUILD.md + CODEGEN.md - CLI, counts, conventions

BUILD.md: `--with-property-monitors` row in the CLI/commands section, updated test
counts (main suite, generated aggregate from task 18, training from task 20), a row for
the new parity + monitor integration suites. CODEGEN.md: document the new
BackendOptions field, the shared-MonitorPlan rendering convention (both backends render
the same plan; out-of-subset formulas warn with identical text), and testing conventions
incl. cross-backend differential tests.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change (CLI row, counts, suite rows) | build/test docs must reflect the new commands and actual counts |
| `CODEGEN.md` | change (options field, monitor conventions, testing conventions) | backend-plugin docs must cover the new option + parity testing |

### Action 4: Backend internals docs (RUST.md + C.md)

RUST.md: document `Sample`, the generated `Mon_*` structs + aggregate monitor state,
`StepError::PropertyViolation { label, step }`, `#[cfg(debug_assertions)]` gating for
production monitors, and the removal of post-hoc `property_check`/`assume_check`
(D-P3-4). C.md sections 4-5: the same content in C terms - monitor structs mirroring
the Rust rendering, `RFS_PROPERTY_VIOLATION` status + `prop_label`/`prop_step`/
`prop_trigger_step` fields, `#ifdef RFS_PROPERTY_MONITORS` gating (correction 11a).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/RUST.md` | change (monitor sections) | backend internals doc must describe the new generated code |
| `src/backends/C.md` | change (sections 4-5) | parity documentation owed by task 11, delivered here per correction 11a |

### Action 5: START.md snapshot

Update section 1: Phase 3 COMPLETE with final counts (tests, examples incl. 41/42 +
invalid_always_zero_bound, training), "Next work: Phase 4" pointer to
`docs/roadmap/PHASE_4.md`. Keep it short (token-economy file).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `START.md` | change (section 1 snapshot) | session entry point must show the new state for future sessions |

## Task deliverables
- All seven doc files updated; no stale post-hoc/skip-warning claims remain;
counts match actuals.

## Task exit criteria
- [ ] DESIGN.md has section 21 with D-P3-1..D-P3-6 tables + `always[N]` semantics in 5.10.
- [ ] KB.md limitations reflect online monitors in both backends; task-23 grammar documented (formula-level &&/||, non-boolean expr fallthrough).
- [ ] BUILD.md/CODEGEN.md document the flag, counts, and parity testing conventions.
- [ ] RUST.md and C.md describe the monitor rendering + gating for their language (grep `monitor` in both returns content).
- [ ] START.md section 1 shows Phase 3 complete + Phase 4 as next work.
- [ ] Grep audit: no remaining "post-hoc property_check/assume_check" or "liveness skipped" claims outside historical TASKS.md entries.

## Non-goals
- No code changes; no PHASE_*.md roadmap file changes (those are owned by task 22 /
  the protocol itself).

## Other information

- Use grep + sed ranges for KB.md/DESIGN.md (never read them fully - START.md rule 2).
- Decision tables: copy the structure of DESIGN.md section 20 (option comparison +
  chosen option + rationale), not free prose.
