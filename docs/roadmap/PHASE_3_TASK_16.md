# Phase 3, Task 16 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 16: new integration test `tests/monitor_integration_test.rs` proving
end-to-end, in BOTH backends: (a) MC catches an injected bug with property-specific
output; (b) the correct spec passes within bound; (c) production debug monitors surface
violations at the step API.

## Context

- Rust MC runtime: `src/backends/rust_mc.rs` - after each successful step checks
  `fsm.property_violation`; on hit `report_property_violation` (line ~89) prints the
  property-specific counterexample (label, trigger step, deadline/confirmation step,
  variable values, I/O sequence), sets status `property-violated:<label>`, exit 1.
- C MC runtime mirrors it: `src/backends/c_mc.rs` `report_property_violation` (~line
  159) via `fsm.prop_label[0] != 0`; same summary-line field names
  (`runs= bound= completed= unexpected= assumption_stops= property_violations=`).
- Step API violation surfaces: Rust generated `StepError::PropertyViolation { label,
  step }` (rust_fsm.rs ~line 199) when production codegen ran with
  `--with-property-monitors` (debug builds); C `step_result_t.status ==
  RFS_PROPERTY_VIOLATION` with `prop_label`/`prop_step`/`prop_trigger_step`
  (`-DRFS_PROPERTY_MONITORS`).
- Pattern to mirror: `tests/integration_example_test.rs` (Phase 1/2) - generate into a
  temp dir, compile (rustc / `/usr/bin/gcc`), run the binary, assert on output.
- Example 41 is the reference spec (Fin/Ack-style with props). The buggy variant drops
  the Ack-sending transition so `start_responds`-style obligations time out.
- Requires task 13 green (example 42 parsing) for the full suite, but this task itself
  only needs example 41.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8., 9. | Rust monitors + StepError/flag | Rust side of (a) and (c) |
| 10. | Rust MC online detection | the counterexample output asserted in (a) |
| 11., 12. | C monitors + C MC parity | C side of (a) and (c) |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/monitor_integration_test.rs` | new | the whole integration suite for this task |

## Planned actions

### Action 1: Buggy spec -> MC detects violation in BOTH backends

Write a buggy variant of example 41 to a temp dir (drop the transition that sends the
Ack/response so the bounded obligation expires). For EACH backend: generate MC
(`cargo run -q -- codegen <backend> mc <buggy.rfs> --out ...`), compile (rustc /
`/usr/bin/gcc -Wall`), run with a bound covering the deadline and a fixed seed. Assert:
exit code 1; stdout contains `property-violated:<label>` with the SAME property label
in both backends; trigger/deadline step numbers present and EQUAL across backends;
summary line field names identical. Rationale: end-to-end proof of online detection +
counterexample parity.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/monitor_integration_test.rs` | new (test fn 1) | MC bug-catching, both backends, cross-compared output |

### Action 2: Correct spec passes within bound in both backends

Same harness on the UNMODIFIED example 41: both MC binaries exit 0 within a reasonable
bound/runs; summary lines have `property_violations=0` (or equivalent zero counts) and
identical field sets. Rationale: no false positives from the new monitors.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/monitor_integration_test.rs` | change (test fn 2) | green-run baseline for both backends |

### Action 3: Rust production debug monitors surface PropertyViolation

Generate example-41 production with `--with-property-monitors`; compile the generated
Rust as a small driver crate/binary in a temp dir (debug profile); drive the FSM along
the violating input sequence; assert `Err(StepError::PropertyViolation { label, step })`
with the expected label and step. Rationale: D-P3-6 debug-mode runtime detection.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/monitor_integration_test.rs` | change (test fn 3) | Rust step-API violation surface |

### Action 4: C production monitors via -DRFS_PROPERTY_MONITORS

Generate example-41 C production with `--with-property-monitors`; compile with
`/usr/bin/gcc -Wall -DRFS_PROPERTY_MONITORS`; drive the same violating sequence; assert
`step_result_t.status == RFS_PROPERTY_VIOLATION` and `prop_label`/`prop_step` equal to
the Rust values from Action 3. Rationale: per-language gating, identical semantics.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/monitor_integration_test.rs` | change (test fn 4) | C step-API violation surface, parity with Action 3 values |

## Task deliverables
- `tests/monitor_integration_test.rs` with 4 test functions (or merged where the
  harness naturally shares setup - record final count in status file).

## Task exit criteria
- [ ] Buggy spec: both backends' MC exit 1, same label + same trigger/deadline steps.
- [ ] Correct spec: both backends' MC exit 0 within bound, zero property violations.
- [ ] Rust debug production build returns `Err(StepError::PropertyViolation)` on the violating sequence.
- [ ] C `-DRFS_PROPERTY_MONITORS` build reports `RFS_PROPERTY_VIOLATION` with label/step equal to the Rust values.
- [ ] `cargo test --no-fail-fast` fully green including the new suite.

## Non-goals
- No changes to MC runtimes or backends. Any behavioral divergence found here is a
  bug: record a plan correction and open a follow-up task - do not patch backends in
  this task file's execution.
- No release-build (zero-cost) assertion for Rust: `cfg(debug_assertions)` gating is
  pinned by task 15's codegen-level tests; driving a release build here adds little.

## Other information

- Temp-dir + compile harness: copy the structure of `tests/integration_example_test.rs`
  (it already compiles generated Rust and drives an mpsc event channel).
- Use a FIXED seed everywhere so runs are deterministic across backends and machines.
- gcc is at `/usr/bin/gcc` (plan assumption 5, verified since Phase 2).
