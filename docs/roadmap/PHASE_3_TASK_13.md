# Phase 3, Task 13 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Complete planned task 13 (valid examples with monitorable properties): verify BOTH new
examples parse and compile to online monitors in both backends with zero warnings.
Example 41 already parses; example 42's parse failure is fixed by the TASK 23 grammar
fix (plan correction 23, added per user request) - example 42 keeps its ORIGINAL text;
this task performs no example edits unless verification reveals a residual issue.

## Context

- Baseline (2026-10-16, working tree): lib unit tests 78/78 OK; `tests/codegen_test.rs`
  16/19 and `tests/examples_test.rs` 2/3 - ALL four failures are caused by example 42
  not parsing ("trailing input after RFS source"). No other suite is affected.
- Root cause of the parse failure (verified empirically, plan correction 13a):
  `prop retry_responds` antecedent `state == Sending && output_has(Sent)` hit a parser
  limitation - `primary_formula`'s expr fallthrough absorbed `output_has` as a variable
  reference, leaving `(Sent)` as trailing input. FIXED BY TASK 23 (plan correction 23,
  added per user request): the fallthrough is restricted to the non-boolean expression
  level so formula-level `&&`/`||` parse each operand via `primary_formula`. After task
  23, example 42 parses AS-IS - no reorder or other edit (correction 13a's workaround
  is superseded).
- Example 41 (`examples/valid/41_property_monitors.rfs`, 5 prop/assume labels) parses;
  every formula is inside the R1-R4 monitor subset.
- This task runs AFTER task 23 and is verification-only: no file changes expected.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 23. | Grammar fix | example 42 parses only after the formula-level boolean operator fix |
| 6., 7. | Monitor compiler + test battery | zero-warning check requires `monitor::compile_formula` and its pinned semantics |
| 8., 11. | Rust + C backend monitor rendering | generated output must contain the monitors for both backends |

No file changes are expected in this task (verification only). The parser pin tests
originally planned here moved to task 23 together with the grammar fix.

## Planned actions

All planned actions needed to reach the task goal, in execution order.

### Action 1: Verify example 42 parses unchanged (post task 23)

Run `cargo run -q -- parse --input rfs --output json examples/valid/42_bounded_liveness_protocol.rfs`
on the UNMODIFIED file and confirm success; also confirm the parsed AST for
`retry_responds` is `Formula::Always(Implies(And(OutputHas, Expr(Eq)), BoundedEventually))`
- i.e., the original antecedent order is preserved (no reorder applied). If it still
fails after task 23: STOP and record a plan correction - do not edit the example to
dodge a parser bug.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - verification only) | - | results recorded in PHASE_3_TASK_13_STATUS.md |

### Action 2: Verify zero warnings + monitor presence in both backends

For examples 41 AND 42, run codegen for BOTH backends on commands production/test/mc
(via `cargo run -q -- codegen <backend> <cmd> <file> --out /tmp/...` or the test-API
pattern from `tests/codegen_test.rs`) and assert: no "outside monitor subset" warning
for any label; generated output contains a monitor struct for EVERY prop/assume label
(Rust: `Mon_` structs; C: `prop_monitors`/assumption-monitor state). Record the exact
warning-free check in the task status file.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - verification only) | - | results recorded in PHASE_3_TASK_13_STATUS.md |

## Task deliverables
- Both examples parse (example 42 unchanged) and compile to monitors with zero
  warnings in Rust AND C (all commands).
- Parser-constraint coverage lives in task 23 (not here).

## Task exit criteria
- [ ] Task 23 completed first; example 42 parses UNMODIFIED with the expected AST shape.
- [ ] `cargo run -q -- validate --input rfs examples/valid` exits 0 (all valid examples).
- [ ] codegen rust + c, production/test/mc, on examples 41 and 42: zero "outside monitor subset" warnings; monitor structs present for every prop/assume label in both backends.
- [ ] `cargo test --no-fail-fast` fully green (codegen_test 19/19, examples_test 3/3, lib incl. task 23's new tests).

## Non-goals
- No file changes expected; if verification fails, stop and record a correction
  instead of patching examples or code here.
- The grammar fix itself is task 23 (added per user request).
- No changes to example 41 or any other example.

## Other information

- If Action 2 finds ANY in-subset formula skipped with a warning in either backend,
  that is a backend rendering bug: record it as a plan correction and fix via a new
  task - do not silently change the example to dodge it.
- Conjunction order is irrelevant to monitor evaluation (R2 trigger signal =
  implication antecedent, evaluated per sample), so the original antecedent order in
  example 42 compiles to the same plan as any reordering.
