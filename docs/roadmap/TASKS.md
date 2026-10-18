
## Phase 4 - Scalable model checking (task 4 of 14)

### Task 4: Parity suite migration — COMPLETE (2026-10-18)
- `tests/backend_parity_test.rs` + `tests/monitor_integration_test.rs` migrated from same-seed fuzz to seedless exact mode (D-P4-14); values re-pinned from actual deterministic output.
- Re-pin: 6 pass-complete / 9 fail across all 15 property-bearing examples - 8 NEW deadlock findings (terminal states the fuzz walk never reported), 1 property violation (`13_properties_safety`/`no_bad`, step 1). `41_property_monitors` now reports its depth-3 deadlock before any property violation.
- Correct-spec test reinterpreted: exact mode explores the full graph, so example 41's terminal state is a legitimate FAIL(deadlock); the no-false-positive claim is preserved via reason + absence of property violations.
- Verification: cargo test 184/184; field-identity Rust vs C on all 15 examples; no fuzz assertions remain in either suite; clippy/fmt at baseline.
- Details: `docs/roadmap/PHASE_4_TASK_4_STATUS.md` (full old->new diff table).

## Phase 4 - Scalable model checking (task 5 of 14)

### Task 5: Liveness language extension — COMPLETE (2026-10-18)
- `eventually (p)` primary + infix `until` (new level between unary and primary, left-associative) across parser/IR/printer/validator; additive IR variants, no IR_VERSION bump (old JSON deserializes - test-pinned).
- Precedence pinned by tests: `a && b until c` = `a && (b until c)`; `a until b until c` left-assoc; `!a until b` = `(!a) until b` (`!` tighter than `until`, standard PSL/CTL); compound operands need parens; `until` never shadows identifiers in expression contexts.
- Monitor boundary: any formula containing eventually/until is outside the subset -> identical skip warning in both backends' production codegen (exact text test-pinned).
- Task correction: the task file's EBNF block contradicted its own pinned consequence for `!a until b`; implemented per the pinned consequence/ladder (negation below the until level) - see "Task corrections" in PHASE_4_TASK_5.md. OPEN QUESTION resolved with the default (pinned production).
- Verification: cargo test 201/201 (was 184, +17 new); examples_test unchanged 42 valid / 12 invalid; clippy at 12-warning baseline (zero from this task); fmt clean.
- Details: `docs/roadmap/PHASE_4_TASK_5_STATUS.md`.
