
## Phase 4 - Scalable model checking (task 4 of 14)

### Task 4: Parity suite migration — COMPLETE (2026-10-18)
- `tests/backend_parity_test.rs` + `tests/monitor_integration_test.rs` migrated from same-seed fuzz to seedless exact mode (D-P4-14); values re-pinned from actual deterministic output.
- Re-pin: 6 pass-complete / 9 fail across all 15 property-bearing examples - 8 NEW deadlock findings (terminal states the fuzz walk never reported), 1 property violation (`13_properties_safety`/`no_bad`, step 1). `41_property_monitors` now reports its depth-3 deadlock before any property violation.
- Correct-spec test reinterpreted: exact mode explores the full graph, so example 41's terminal state is a legitimate FAIL(deadlock); the no-false-positive claim is preserved via reason + absence of property violations.
- Verification: cargo test 184/184; field-identity Rust vs C on all 15 examples; no fuzz assertions remain in either suite; clippy/fmt at baseline.
- Details: `docs/roadmap/PHASE_4_TASK_4_STATUS.md` (full old->new diff table).
