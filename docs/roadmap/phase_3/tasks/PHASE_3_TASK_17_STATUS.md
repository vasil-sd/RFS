# Phase 3, Task 17 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_17.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `tests/backend_parity_test.rs` | new cross-backend differential parity suite (3 tests): same-seed MC runs over ALL property-bearing valid examples, violation parity (natural + forced `--violate`), generated test-executable pass/fail parity - the backend parity principle made executable |

## Changed/removed source files

| Name | Operation | Description |
|---|---|
| (none) | - | non-goal honored: no backend/runtime changes; the suite is pure observation over generated code |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `docs/roadmap/PHASE_3_TASK_17.md` | changed | exit criteria checked (all four); "Task corrections" section appended (Action 2/3 calibration, violation wire format) |
| `docs/roadmap/PHASE_3_TASK_17_STATUS.md` | new | this file |
| `docs/roadmap/PHASE_3_PLAN.md` | changed | plan correction 24: NEW follow-up task (example 13 self-violation under always-on monitors) |
| `docs/roadmap/PHASE_3_TASK_24.md` | new | follow-up task file per template 2.4 (discovered during this task's execution) |

## Other file changes

| Name | Operation | Description |
|---|---|
| (none in the repo) | - | generated artifacts written to `target/rfs_backend_parity/` (self-cleaning with `cargo clean`) and /tmp scratch (discarded); examples, training sets, build files untouched |

## Changed functionality

- None.

## Removed functionality

- None.

## New functionality

- Cross-backend differential parity coverage (`tests/backend_parity_test.rs`), run parameters pinned SMALL for determinism + speed (`--bound 8 --runs 20 --seed 777`):
  - **Action 1** - `mc_same_seed_runs_parity_all_property_bearing_examples`: dynamic enumeration of ALL valid examples carrying `prop `/`assume ` (currently 15: top-level 04, 13, 14, 15, 16, 24, 37, 41 + `tla_adapted/` 31-36, 42; anchors assert both traversal levels) - generate MC in both backends (in-process codegen, same path as the CLI), compile with rustc / `/usr/bin/gcc`, run with identical flags; equal exit codes and field-by-field equal summary lines (`runs= bound= completed= unexpected= assumption_stops= property_violations=`). Future property-bearing examples are covered automatically.
  - **Action 2** - `mc_violation_parity_natural_and_forced`: natural violations must be field-identical across backends (run, label, trigger step, violation step) and the currently-observed set is PINNED: exactly `13_properties_safety/no_bad` (run 0) and `41_property_monitors/fin_acked_bounded` (run 19, trigger 1, violation 5). Forced violation via `--violate fin_acked_bounded` on example 41 in BOTH backends: both exit 1, counterexample field-identical (trigger step 1, violation step 5 = deadline at 1 + 5 - 1).
  - **Action 3** - `generated_test_executables_parity_all_property_bearing_examples`: for the same example set, generate the `test` command in both backends, compile (rustc `--test` / gcc), run; pass/fail must MATCH per example. Pinned current state: only `13_properties_safety.rfs` fails - and it fails in BOTH backends identically BY DESIGN (D-P3-6): its representative-input walk ToBad -> Bad violates its own `no_bad` property, and `test` codegen always includes monitors. Follow-up task 24 resolves the example/test-plan mismatch; when it lands, this pin becomes an empty list.
  - Shared harness: compile-once binary cache under one mutex (MC binaries shared between Actions 1-2), in-process codegen via `rfs::codegen`, output parsers for the stable MC wire format.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| `mc_same_seed_runs_parity_all_property_bearing_examples` | new | Action 1: same-seed differential MC comparison over the dynamic property-bearing example set |
| `mc_violation_parity_natural_and_forced` | new | Action 2: natural-violation field parity + pinned set; forced `--violate` counterexample parity on example 41 with pinned steps |
| `generated_test_executables_parity_all_property_bearing_examples` | new | Action 3: generated test executables, both backends, pass/fail parity + pinned failing set (currently `13_properties_safety.rfs` only) |
| helpers `property_bearing_examples`, `cached_binary`, `generate`, `compile_rust`, `compile_c`, `mc_binary`, `test_binary`, `run_mc`, `parse_mc_violation`, `parse_mc_summary`, `McViolation`, `McOutcome` | new | shared differential harness (see New functionality) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --no-fail-fast` lib unit tests (86) | OK |
| `tests/backend_parity_test.rs` (3; new in this task) | OK (~6s: cached compiles + 30 MC runs + 30 test-binary runs) |
| `tests/codegen_test.rs` (23), `examples_test.rs` (3), `frontend_test.rs` (41), `integration_example_test.rs` (2), `library_comparison.rs` (4), `monitor_integration_test.rs` (4), `training_test.rs` (4) | OK - full suite 170 passed / 0 failed |
| `cargo clippy --all-targets` | no NEW warnings from this change (0 findings reference `backend_parity_test.rs`; the pre-existing e0bc3cb dirt remains: 11 lib warnings in `src/backends/c_fsm.rs` + 1 in `tests/codegen_test.rs`, owed before task 22) |
| `rustfmt --edition 2021 --check tests/backend_parity_test.rs` | clean (file formatted with rustfmt; remaining repo-wide diffs are the known pre-existing `src/backends/c_fsm.rs` sites, untouched here) |

## Found problems/issues/mismatches

- **Example 13 fails its own generated tests in BOTH backends (by design, not a divergence).** `examples/valid/13_properties_safety.rfs`: the shared TestPlan's positive-sequence walk takes ToBad -> Bad at step 1, violating the example's own `no_bad` property; since `test` codegen always includes monitors (D-P3-6), the generated executable aborts with `assertion failed: steps > 0 || !EXPECT_PROGRESS || candidates.is_empty()` in `tests::positive_sequence_runs`. Both backends fail identically (parity holds - this is what the suite pins). Root cause is a spec-level mismatch (the property forbids a state the environment deterministically reaches on the representative walk, with no assumption preventing it), first surfaced by this task; it was not previously recorded in any roadmap doc. Consequence: `cargo test-generated-code` (task 18) and the close-out full verification (task 22) will fail on example 13 until resolved. **Follow-up task 24 opened** (plan correction 24, `PHASE_3_TASK_24.md`); executes after task 17, before tasks 18/22.
- **Environmental incident: cargo's rustc target probe reads corrupted stdin (recurrence of the task 23 incident class).** In this session, fresh toolchain probes (`rustc - ...` / `clippy-driver rustc - ...`) intermittently failed with "failed to run `rustc` to learn about target-specific information" because the probe's stdin delivered stale text from earlier session commands (e.g. a stray `ls: cannot access 'rustfmt.toml'` line) instead of EOF, even when cargo's own stdin was /dev/null; already-cached probes (plain `cargo build/test`) were unaffected. **Working workaround found and verified**: run the cargo command with stdin from an OPEN EMPTY FIFO - `mkfifo /tmp/in.fifo; (sleep 20 > /tmp/in.fifo) & cargo clippy --all-targets < /tmp/in.fifo` - the probe blocks on the held-open pipe, gets clean EOF when the writer exits, and the command completes normally (this is how the clippy check above was run). Also re-confirmed: `/usr/local/bin/rustc|cargo` point at `/opt/rust-old` (1.58.1) and must NOT be used (task 23 status warning); the working toolchain is rustc 1.99.0 at `/opt/unsloth-studio/sandbox/__LOCALID_nkBnQA0/.rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin`. No repo impact; recorded so a future session does not re-diagnose from scratch.

## Other important findings/information

- **Natural violations at the pinned parameters are legitimate environment behavior** (task file's "Other information" case): `13_properties_safety/no_bad` (run 0 - Bad is reachable within 8 steps with no assumption blocking it) and `41_property_monitors/fin_acked_bounded` (run 19). Both are identical in both backends, so parity is unaffected; the example-41 case matches task 16's finding that bound >= 5 lets the random environment legitimately expire the bounded obligation (task 16 pins the no-false-positive claim at bound 4 for that reason). No bound fix needed for parity purposes; if a future task wants example 13 to MC-pass deterministically, that is part of task 24's resolution.
- **Forced-violation label choice**: `fin_acked_bounded` (example 41) is the only practically forcible property of that FSM - `closed_implies_acked` requires reaching Closed without ack_received (impossible: Closed is only reachable via an Ack that sets it) and `ack_sets_flag_next` is guaranteed by the Ack transition itself. Forcing `fin_acked_bounded` yields field-identical counterexamples in both backends (run 1, trigger 1, violation 5).
- **Dynamic enumeration already exceeds the planning-time set** (33, 41, 42): 15 examples are covered today, so the suite future-proofs against new property-bearing examples exactly as planned; the two anchor assertions keep both traversal levels (top-level + `tla_adapted/`) from silently dropping out.
- **Performance**: full parity suite ~6s in a warm tree (15 examples x 2 backends, MC + test binaries cached per process run); no benchmarking done (Phase 4 scope, non-goal honored).
