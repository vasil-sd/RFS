# Phase 3, Task 24 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Resolve the example-13 / own-test-plan mismatch discovered by task 17 so that
`examples/valid/13_properties_safety.rfs`'s GENERATED `test` executable passes in BOTH
backends, and clear the pinned failing set in task 17's parity suite. Plan correction
24; executes after task 17, before tasks 18/22 (task 18's `cargo test-generated-code`
and the task 22 close-out verification both fail on example 13 until this lands).

## Context

- The problem (verified in task 17): the shared TestPlan's positive-sequence walk for
  example 13 takes ToBad -> Bad at step 1, violating the example's own `no_bad`
  property. Since `test` codegen ALWAYS includes property monitors (D-P3-6), the
  generated executable then aborts in BOTH backends identically with
  `assertion failed: steps > 0 || !EXPECT_PROGRESS || candidates.is_empty()` in
  `tests::positive_sequence_runs`. This is a spec-level mismatch - the property forbids
  a state the representative walk deterministically reaches, and no assumption prevents
  it - NOT a backend divergence (parity holds; task 17 pins this).
- Read `examples/valid/13_properties_safety.rfs` at execution time for the exact FSM
  shape (states, inputs, the `no_bad` property) before choosing a fix.
- Where the walk logic lives: shared TestPlan in `src/codegen_test.rs`; per-backend
  rendering in `src/backends/rust_test.rs` / the C test backend (verify names at
  execution time). Determine empirically how the positive sequence selects its
  representative input per state (declaration order? first candidate?) - this decides
  which fix option works.
- Pins that will be touched: `tests/backend_parity_test.rs` Action 3 currently pins
  `failing == vec!["13_properties_safety.rfs"]` -> must become an empty list; if the
  chosen fix changes example 13's random-MC behavior at the pinned parameters
  (`--bound 8 --runs 20 --seed 777`, e.g. reordering inputs changes the LCG index->input
  mapping, or an assumption adds `assumption_stops`), Action 2's natural-violation pin
  and/or summary expectations must be re-pinned WITH a documented reason.
- D-P3-6 STANDS: `test` codegen always includes monitors. Do not "fix" this by gating
  monitors out of test codegen or weakening/removing the `no_bad` property.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 17. | Parity suite | supplies the reproduction, the pins to update, and the verification harness |
| 13. | Examples work complete | the valid example set is stable; this task changes one existing example |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 18. | Generated test aggregate | `cargo generate-code-tests && cargo test-generated-code` must pass - blocked on this task |
| 22. | Close-out full verification | the BUILD.md section 5 checklist includes the full suite - blocked on this task |

### Affected source files (optional)

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/13_properties_safety.rfs` | change (preferred fix) | minimal spec change so the representative walk respects the example's own property (see Action 2, options A/B) |
| `src/codegen_test.rs` | change (LAST RESORT only) | shared TestPlan positive-sequence rule adjustment - option C; affects every example's tests and requires full re-verification |

### Affected test files (optional)

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | Action 3 pin: failing set -> empty; Action 2 pins updated only if the fix changes pinned-parameter behavior, with documented reason |

## Planned actions

### Action 1: Reproduce + determine the walk rule

Regenerate example 13's `test` executable in both backends (reuse task 17's harness
pattern: in-process codegen -> rustc `--test` / gcc), confirm the identical failure,
then read the positive-sequence logic in `src/codegen_test.rs` and determine exactly how
the representative input per state is chosen. Rationale: the fix option depends on this
rule (e.g. if the walk takes the FIRST declared candidate, reordering inputs suffices).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (read-only) `src/codegen_test.rs`, `examples/valid/13_properties_safety.rfs` | read | determine the walk rule and the exact spec shape |

### Action 2: Choose + apply the fix

Decision recorded in Task corrections at execution time, with rejected alternatives:
- **Option A (preferred): minimal spec change to example 13** - e.g. reorder inputs so
  the representative walk stays in Good (if the walk takes the first/earliest candidate).
  Preserves the pedagogical intent: `no_bad` still exists, MC can still find violations
  under random runs (the random environment may still pick ToBad), and the example's
  documentation/comments stay truthful.
- **Option B: add an assumption** that blocks the Bad path for test execution (e.g.
  `assume no_to_bad : input_is(ToBad) == false`). Check the MC impact first
  (`assumption_stops` in the summary, reachable-state change) and whether it defeats the
  example's purpose; re-pin task 17 Action 2 accordingly with a documented reason.
- **Option C (last resort): adjust the shared TestPlan positive-sequence rule** to skip
  candidate inputs that immediately violate a declared property. Affects EVERY example's
generated tests - requires regenerating + re-running all test executables and re-pinning
the full suite; only if A and B are infeasible, and it must be recorded as a design
decision change.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `examples/valid/13_properties_safety.rfs` (A/B) or `src/codegen_test.rs` (C) | change | the chosen fix |

### Action 3: Verify + update pins

Regenerate example 13's test executables in both backends -> BOTH pass. Run
`cargo test --no-fail-fast` fully green (incl. the parity suite). Update task 17's
Action 3 pin to an empty failing set (with a comment pointing at this task); re-check
example 13's MC runs at the pinned parameters and update Action 2 pins only if behavior
changed, with the reason recorded. Run `cargo run -- validate --input rfs examples/valid`
(exit 0) so the changed example still validates.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | pin updates per Action 3 text |

## Task deliverables
- Fixed example 13 (or, option C only, the shared TestPlan rule) such that its generated
  `test` executable passes in both backends.
- Updated pins in `tests/backend_parity_test.rs` (empty failing set; any re-pinned MC
  values with documented reasons).
- `docs/roadmap/PHASE_3_TASK_24_STATUS.md` per template 2.5, including the fix decision
  and rejected alternatives.

## Task exit criteria
- [ ] Example 13's generated test executable passes in BOTH backends (rustc + gcc).
- [ ] Task 17 Action 3 pin updated to an empty failing set; `cargo test --no-fail-fast`
      fully green including the parity suite.
- [ ] No regressions: all other examples' test executables and MC parity pins unchanged
      (or updated with documented reason); `cargo run -- validate --input rfs
      examples/valid` exits 0; clippy/fmt clean on any changed file.
- [ ] Chosen fix option + rejected alternatives recorded in this file's Task corrections.

## Non-goals
- No monitor-semantics changes: D-P3-6 (test codegen always includes monitors) stands;
  no gating monitors out of test codegen.
- Do not weaken, remove, or rename the `no_bad` property to make the failure go away.
- No benchmarking (Phase 4 scope).

## Task corrections

(to be filled during execution)

### Changes in design decisions

(None anticipated; option C would add one - record it there if chosen.)

## Other information

- gcc is at `/usr/bin/gcc` (plan assumption 5, verified since Phase 2).
- Toolchain: rustc 1.99.0 at `/opt/unsloth-studio/sandbox/__LOCALID_nkBnQA0/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin`; do NOT use `/usr/local/bin/rustc|cargo` (points at `/opt/rust-old`, 1.58.1). If a fresh cargo probe fails with corrupted probe stdin, use the FIFO workaround from `PHASE_3_TASK_17_STATUS.md` (`mkfifo /tmp/in.fifo; (sleep 20 > /tmp/in.fifo) & cargo <cmd> < /tmp/in.fifo`).
- Example 13's NATURAL MC violation at the pinned parameters (`no_bad`, run 0) may
  legitimately persist after option A (random runs can still reach Bad within bound 8)
  - that is fine for parity; only the generated test executable must pass.
