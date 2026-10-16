# Phase 3, Task 17 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 17: new cross-backend differential parity test
`tests/backend_parity_test.rs` - the backend parity principle made executable: Rust and
C model checkers run on identical specs/seeds must produce indistinguishable results.

## Context

- MC CLI: `rfs codegen <backend> mc <spec.rfs> --out FILE`; runtime flags include
  `--bound`, `--runs`, `--seed` and `--violate` (force a violation) - verify exact flag
  names against `src/backends/rust_mc.rs`/`c_mc.rs` arg parsing at execution time.
- Summary line field names are STABLE (plan assumption 3): `runs= bound= completed=
  unexpected= assumption_stops= property_violations=` - docs/tests reference them.
- Violation output: status `property-violated:<label>` + trigger/deadline step info
  (task 10/12); identical format required in both backends.
- Property-bearing valid examples at planning time: 33 (german_transfer_tla),
  41, 42 - enumerate them DYNAMICALLY (scan `examples/valid` for `prop `/`assume `)
  so future additions are covered automatically.
- Generated test executables: both backends' `test` command produce standalone
  executables from the shared TestPlan; both must pass on the same examples.
- Requires tasks 10, 12 (MC parity) and 13 (example 42 parsing).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10., 12. | Rust + C MC online detection | identical violation output under comparison |
| 13. | Valid examples fixed | the example set must parse before generation |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/backend_parity_test.rs` | new | differential parity suite (MC runs + forced violations + generated test executables) |

## Planned actions

### Action 1: Same-seed MC runs over all property-bearing examples

For every valid example containing properties or assumptions: generate MC in both
backends into a temp dir, compile (rustc + `/usr/bin/gcc`), run BOTH with identical
`--bound --runs --seed`. Assert equal exit codes and equal summary-line fields
(`runs= bound= completed= unexpected= assumption_stops= property_violations=`).
Rationale: the core parity assertion - same spec + seed => same results.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | new (test fn 1) | differential MC comparison, dynamic example discovery |

### Action 2: Violation parity (natural + forced)

When any run in Action 1 reports a violation, assert equal property label and
equal trigger/deadline step numbers between backends. Additionally force violations in
BOTH backends via `--violate` on at least one property-bearing example and compare the
emitted counterexample information (label + step fields) field by field.
Rationale: parity must hold exactly where it matters most - on failure.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change (test fn 2) | forced-violation comparison via --violate |

### Action 3: Generated test executables both pass

For the same example set, generate the `test` command in both backends, compile, run;
require BOTH executables to exit 0. Rationale: identical coverage from the shared
TestPlan must hold for every backend (exit criterion: backends never left behind).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change (test fn 3) | test-executable parity |

## Task deliverables
- `tests/backend_parity_test.rs` with the three test functions above.

## Task exit criteria
- [ ] Every property-bearing valid example covered by same-seed MC comparison (both backends, equal exit codes + summary fields).
- [ ] Any violation observed: label + trigger/deadline steps equal across backends; `--violate` comparison passes for at least one example.
- [ ] Generated test executables of both backends pass on the same examples.
- [ ] `cargo test --no-fail-fast` fully green including the parity suite.

## Non-goals
- No backend/runtime changes. A divergence found here is a real bug: record a plan
correction and open a follow-up task with the minimal reproducing spec - do not fix
backends inside this task's execution.
- No benchmarking (Phase 4 scope).

## Other information

- Keep run parameters SMALL but sufficient (e.g. `--bound 8 --runs 20`) so the suite
  stays fast; the point is determinism + equality, not coverage depth.
- If a property-bearing example legitimately violates under random runs in BOTH
  backends, that is fine for parity - but record it; an example that violates may need
  its own bound fix (like correction 10a did for example 33).
