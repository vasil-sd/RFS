# Phase 4, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Add `tests/benchmark_mc_test.rs`: run exact MC on all benchmark specs (43/44/45) in
BOTH backends, assert expected statuses + parity of deterministic metrics + sanity
bounds, exercise inline FAIL variants with known violations, and write
`target/mc_benchmarks.json` + a per-backend table.

## Context

- D-P4-13: metrics = DETERMINISTIC counts (states/transitions/max_depth/monitor
  size) asserted for parity + sanity bounds; wall-clock recorded as INFORMATIONAL
  only (never asserted). Report written to `target/mc_benchmarks.json` (not checked
  in).
- Inline FAIL variants with known violations follow the established pattern from
  `tests/monitor_integration_test.rs` (injected bug, driven through MC): here the
  variants are modified copies of benchmark specs (e.g. a token ring that drops a
  grant -> liveness violation; an elevator that skips a call; a safety break like
  duplicate delivery) with known expected status/reason/label.
- Harness pattern: reuse `tests/backend_parity_test.rs` helpers (root/rustc/gcc
  paths, cached binaries, generate+compile per backend) - do not duplicate them.
- Sanity bounds come from task 8's recorded state counts (e.g. states within
  [recorded/2, recorded*2] guards against silent engine regressions while allowing
  container changes in task 10).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Exact engine - C | both backends under test |
| 7. | Liveness engine - C | liveness metrics/statuses under test |
| 8. | Benchmark specs | the specs being driven (43 from task 6) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Performance features | tunes `--max-states` default + container choice from this data |
| 14. | Close-out | full verification includes this suite |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/benchmark_mc_test.rs` | new | benchmark MC suite (both backends, statuses, metric parity, FAIL variants, JSON report) |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | section 3 suite row for the new test |

## Planned actions

### Action 1: Suite harness
For each benchmark spec (43/44/45) x backend (rust/c): generate MC code, compile
(rustc / gcc -Wall), run exact mode, parse the pinned summary line. Assert:
expected status `pass-complete`; deterministic metrics (states/transitions/max_depth
+ monitor size) IDENTICAL across backends; sanity bounds from task 8's recorded
counts.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/benchmark_mc_test.rs` | new | harness + pass-complete assertions |

### Action 2: Inline FAIL variants
Add modified-spec variants with known violations (safety + liveness, per Context):
assert `fail` + correct reason + label in BOTH backends and identical counterexample
fields; variants live inline in the test file (no new example files).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/benchmark_mc_test.rs` | change | FAIL-variant cases |

### Action 3: JSON report + table
Write `target/mc_benchmarks.json` (per spec x backend: status, deterministic
metrics, wall-clock informational) and print a per-backend summary table to the
test output.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/benchmark_mc_test.rs` | change | report writer + table printer |

### Action 4: BUILD.md suite row
Add the new suite to BUILD.md section 3 with its command and expected result.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | section 3 table row |

## Task deliverables
- `tests/benchmark_mc_test.rs` green: statuses, metric parity, sanity bounds, FAIL
  variants, JSON report.
- BUILD.md suite row.

## Task exit criteria
- [ ] `cargo test` green including the new suite (record its pass count for task 14).
- [ ] All three benchmarks pass-complete in BOTH backends with identical deterministic metrics.
- [ ] FAIL variants: correct status/reason/label + counterexample parity in both backends.
- [ ] `target/mc_benchmarks.json` written; per-backend table printed; wall-clock informational only.
- [ ] BUILD.md section 3 updated.

## Non-goals
- Default tuning / container switch (task 10), SMT mode (task 11), docs beyond the
  suite row (task 13).

## Other information
- Keep per-spec runtime under ~30s in the test (the suite runs inside `cargo test`);
  if a spec is slower, reduce its parameters via plan correction and re-record
  sizing data.
- The JSON report is a development artifact - never checked in, never asserted on
  byte-for-byte (assert on parsed fields).
