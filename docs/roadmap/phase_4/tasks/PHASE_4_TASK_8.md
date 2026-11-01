# Phase 4, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Add two more protocol-like medium-size benchmark specs with safety + liveness
properties and abstraction-stress payload fields: `examples/valid/44_token_ring.rfs`
(4-node token ring) and `examples/valid/45_elevator.rfs` (elevator). The file-driven
example suite picks them up automatically.

## Context

- D-P4-13: three protocol-like benchmark specs total (43 landed in task 6); each
  must be CORRECT (exact MC -> pass-complete) - FAIL variants are inline in task 9's
  test file, never in examples/valid.
- Sizing target: exact-mode state spaces in the low thousands to low hundreds of
  thousands so the benchmark suite (task 9) runs in seconds per spec, not minutes.
  Record each spec's actual state count (Rust exact mode) in the status file - task
  9 sets sanity bounds from these numbers and task 10 tunes `--max-states`.
- Abstraction stress: include payload fields that are genuinely UNUSED by guards/
  actions/properties (e.g. wide bitvectors carried for realism) so input
  abstraction (D-P4-3) is exercised at scale; also keep some CONCRETE wide fields to
  show the contrast in state counts.
- Liveness coverage: each spec carries at least one `eventually`/`until` property
  (language from task 5, engine from tasks 6/7).
- The parity suite auto-covers new property-bearing examples once added - no test
  code change needed here beyond the example files.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | Liveness language extension | specs use `eventually`/`until` |
| 6. | Liveness engine - Rust | MC smoke runs (pass-complete) need the liveness engine |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 9. | Benchmark suite test | asserts statuses/metrics on these specs in both backends |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `examples/valid/44_token_ring.rfs` | new | 4-node token ring: request/grant/token-pass events, liveness "every request is eventually granted", abstraction-stress payload fields |
| `examples/valid/45_elevator.rfs` | new | elevator: floors, up/down calls, serve events, liveness "every call is eventually served", medium state space |

## Planned actions

### Action 1: Token ring spec (44)
Design and write the 4-node token ring: nodes hold/request the token; events for
token pass, request, grant; safety properties (e.g. at most one token holder) +
liveness (`eventually`/`until`: every request is eventually granted). Include
unused wide payload fields (abstraction stress). Verify: parse+validate+round-trip
green; Rust exact MC -> pass-complete; record state count.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/44_token_ring.rfs` | new | second benchmark spec |

### Action 2: Elevator spec (45)
Design and write the elevator: position + direction state, call events per floor,
serve transitions; safety (serves only pending calls, no overshoot) + liveness
(every call eventually served). Medium state space (tune floor count to hit the
sizing target). Same verification as Action 1.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/45_elevator.rfs` | new | third benchmark spec |

### Action 3: Record sizing data
Capture per spec (Rust exact mode): states/transitions/max_depth, with and without
abstraction where the contrast is instructive; write into the status file as input
for task 9's sanity bounds and task 10's default tuning.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - data recorded in PHASE_4_TASK_8_STATUS.md) | - | input for tasks 9/10 |

## Task deliverables
- `examples/valid/44_token_ring.rfs` and `examples/valid/45_elevator.rfs`, both
  correct (pass-complete) with safety + liveness properties.
- Sizing data recorded for tasks 9/10.

## Task exit criteria
- [ ] `cargo test --test examples_test` green: 45 valid / 12 invalid (parse+validate+round-trip).
- [ ] Both specs MC to pass-complete in Rust exact mode within ~30s each; state counts recorded.
- [ ] Each spec carries >= 1 liveness property and >= 1 deliberately unused payload field (abstraction stress).
- [ ] Parity suite (auto-pickup) still green with the new examples.

## Non-goals
- Benchmark assertions/JSON report (task 9), FAIL variants (inline in task 9's test
  file), performance tuning (task 10).

## Other information
- If a spec blows past the sizing target, reduce parameters (nodes/floors) rather
  than dropping liveness or abstraction coverage - record the choice.
- Keep specs readable: they are also future training-data and documentation
  material (tasks 12/13).
