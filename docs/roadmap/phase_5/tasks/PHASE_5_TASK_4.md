# Phase 5, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Extend the SHARED, language-agnostic test planner (`src/codegen_test.rs`) so
that ONE plan - rendered by both backends in tasks 5/6 - covers: per-transition
unit tests, invariant tests, property-monitor debug tests, seeded random-trace
tests, and fuzz-target skeleton planning (parity principle: identical coverage
in Rust and C).

## Context

- Today the planner produces module-level tests only: positive drive
  (`expect_progress`), negative unexpected-input probe (`definite_unexpected`),
  deadlock-style coverage (`is_deadlocked_initial`). `TestPlan { steps,
  expect_progress, definite_unexpected, has_on_error }`.
- Design decisions taken during planning (recorded here so they are not
  re-litigated):
  - **Per-transition tests**: one test per transition `t` of the module.
    Drive from `t.state` with a representative input matching `t.pattern`; if
    `t.guard` is non-trivial, also drive with representative variable values
    that make the guard true AND false (the planner computes both cases
    statically; where no representative assignment satisfies the guard, the
    test asserts the transition was NOT taken). Assert next-state + emitted
    outputs for the taken case. Deterministic, no MC needed at codegen time.
  - **Invariant tests**: for each in-subset SAFETY property (atoms under
    `always`/`always[N]` with no liveness operators), re-check the atom at
    every step of a driven trace (the positive drive trace); violation ->
    test failure with step index. Liveness properties are excluded here (they
    belong to the model checker).
  - **Property-monitor debug tests**: for each in-subset property with a
    monitor, generate an INJECTED violating trace that must fire the monitor
    (sticky `property_violation` record / `RFS_PROPERTY_VIOLATION`). Scope:
    properties of the shape `always (p)` / `always[N] (p)` where p is a boolean
    atom - the planner constructs the minimal violating drive; other shapes are
    skipped with a planner note (the Phase 3 monitor integration suite already
    covers deeper shapes via MC).
  - **Random-trace tests**: deterministic LCG (SAME constants as the fuzz
    wire format: 6364136223846793005 / 1442695040888963407, top-53-bit double
    mapping) with a FIXED seed constant (e.g. 12345), K = `--steps` drive
    steps; assert no panic/error beyond expected `UnexpectedEvent` handling and
    report coverage counts (states visited, transitions taken) as test output.
  - **Fuzz skeleton planning**: per event variant, a decode strategy placeholder
    (representative value per field, byte-wise for payloads) consumed by the
    renderers to emit libfuzzer_sys / libFuzzer skeletons (tasks 5/6).
- The planner must stay language-agnostic: it emits data (cases, expected
  outcomes, labels), never code.

## Scope

### Depends on tasks

(none)

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | Rust test renderer + libfuzzer_sys skeleton | renders this plan |
| 6. | C test renderer + libFuzzer skeleton | renders the SAME plan (identical coverage) |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen_test.rs` | change | extended TestPlan + build_test_plan: per-transition cases, invariant checks, monitor-violating traces, random-trace config, fuzz decode plan |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| lib unit tests in `src/codegen_test.rs` | change | exhaustive planner coverage: every transition of example 43 planned; guard true/false cases; invariant selection (safety only); monitor trace construction; LCG determinism pinning; fuzz decode plan per variant |

## Planned actions

### Action 1: extend TestPlan + builder
Add the five new plan components per Context with their data types. Keep the
existing fields (backwards compatible - the current module-level rendering
stays). Unit-test each component against pinned examples (01, 15, 41, 43).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen_test.rs` | change | new plan types + builder logic + unit tests |

### Action 2: LCG + determinism pinning
Implement the seeded random-trace config (fixed seed constant, steps) and PIN
the first N generated inputs for a reference spec in a unit test (guards
against accidental constant/algorithm changes).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen_test.rs` | change | LCG helper + pinned-sequence unit test |

## Task deliverables
- Extended `TestPlan` with the five new components (data only, no code).
- Exhaustive planner unit tests incl. LCG determinism pinning.

## Task exit criteria
- [ ] Planner emits per-transition cases for EVERY transition of a pinned example (count asserted in a test).
- [ ] Guarded transitions produce both taken and not-taken expectations where statically decidable; otherwise the documented fallback.
- [ ] Invariant selection includes only safety (no liveness) atoms - asserted by a test on example 43 (which has liveness properties).
- [ ] Monitor-violating trace fires exactly the target property's monitor in the Phase 3 reference interpreter (test-only check).
- [ ] LCG output sequence byte-pinned; fuzz decode plan covers every event variant.
- [ ] Full suite green, clippy/fmt clean. NO backend changes in this task.

## Non-goals
- Rendering (tasks 5/6), the `rfs test` command (task 7).
- Reachability-based test selection (the planner is static; MC-backed
  selection would be a later-phase idea).
- Changing existing module-level tests (they stay; new components are additive).

## Other information
- Where a guard cannot be statically satisfied by representative values, prefer
  the not-taken assertion over skipping - it still pins behavior.
- Keep plan data serializable-friendly (plain structs) even though nothing
  serializes it yet - task 10's JSON work may reuse shapes later.
