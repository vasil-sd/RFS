# Phase 17, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_6.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `crates/examples/mc_deadlock_fail/04_tcp_handshake_like.rfs` | the broken spec (copy of the corpus file) - terminal Established deadlocks |
| `crates/examples/mc_deadlock_fail/fixed.rfs` | the Tick-teardown fix - PASS (complete), 62 states |
| `crates/examples/mc_property_violation_fail/47_http2_stream_rst_fail.rfs` | the pinned FAIL variant extracted from `tests/common/fail_variants.rs::http2_rst_fail_src()` (RstStream arm retargeted off Closed) |
| `crates/examples/mc_property_violation_fail/fixed.rfs` | the original spec 47 - PASS (complete), 434 states |
| `crates/examples/invalid_spec_diagnosis/broken.rfs` | copy of `examples/invalid/invalid_duplicate_priority.rfs` |
| `crates/examples/invalid_spec_diagnosis/fixed.rfs` | the self-loop demoted to priority 4 - validates |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | changed | + `run_rfs` helper + three pinned test legs (one per negative example) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/mc_deadlock_fail/README.md` | new | template README: real CE traces from mc-ref AND both generated backends (byte-identical), deadlock-lasso walkthrough, why a Recv(Syn) reset would break syn_ack, the fix + re-run proof |
| `crates/examples/mc_property_violation_fail/README.md` | new | template README: step-by-step violation-trace walkthrough (which property, which step, why), text + JSON CE, the one-line fix + re-run proof |
| `crates/examples/invalid_spec_diagnosis/README.md` | new | template README: validate text + --json diagnostic explained field by field (kind, line/col 0/0 module-level semantics, message grammar), three general fix strategies |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `docs/llm/TEST_STRUCTURE.md` | changed | +4 rows for the new test fns (same-commit rule); 1383/1430 after the task-3 budget extension |

## Changed functionality
None - example projects + tests only.

## Removed functionality
None.

## New functionality
- Three negative example projects teaching diagnosis: deadlock CE reading, property-violation CE reading, validation-diagnostic reading - each with the fix and re-run proof pinned by a test leg.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `mc_deadlock_fail_reports_deadlock_and_fix_passes` | new | exit 1 + pinned reason=deadlock CE ending in Established; fix exits 0 PASS complete |
| `mc_property_violation_fail_reports_rst_and_fix_passes` | new | exit 1 + pinned label=rst_goes_closed step=2 CE through HalfClosedLocal; original spec 47 passes |
| `invalid_spec_diagnosis_reports_duplicate_priority_and_fix_validates` | new | exit 1 + pinned diagnostic in text AND --json (ok=false, kind=semantic); fix validates exit 0 |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (6) | OK |
| `cargo test` (576 = 573 baseline + 3 new) | OK |
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5) | OK |
| `cargo clippy --all-targets` | no warnings in the new code |
| `cargo fmt --check` | clean |

## Found problems/issues/mismatches
- The generated MC artifacts reference their fsm file by the CODEGEN-TIME name (`#[path]` / `#include "..."`), so compiling them from a different CWD fails with doubled or missing paths - the flat-dir + plain-filename pattern (what the test suites use) is the reliable capture recipe. Worth a line in USAGE.md's codegen section (task 10).
- A first capture attempt left the specs in /tmp, which did not persist between tool calls - the example dirs now carry the canonical copies.

## Other important findings/information
- All three engines (mc-ref, Rust MC binary, C MC binary) produced BYTE-IDENTICAL deadlock reports on spec 04 - the parity claim in the README is verified, not assumed.
