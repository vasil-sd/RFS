# Phase 6, Task 2a - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_2A.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (surgical two-expression fix in the existing C backend).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/backends/c_fsm.rs` | changed | bitvector shift lowering: uint64-wide operands with a runtime guard, `({type})((((uint64_t)(r)) < W ? ((uint64_t)(l) op (uint64_t)(r)) : ((uint64_t)0)) & mask)` - one uniform form for literal and variable amounts implementing `amount >= width -> 0` (Rust/SMT semantics); multiplication: uint64-wide intermediate `({type})(((uint64_t)(l) * (uint64_t)(r)) & mask)` so bit<16> products above INT_MAX are not signed overflow; `+`/`-`/bitwise unchanged (no defect there) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `src/backends/C.md` | changed | type-mapping/semantics note updated: shifts render uint64-wide with the runtime guard, multiplication in a uint64_t-wide intermediate - doc matches the rendering |
| `docs/SEMANTICS.md` | changed | footnote c-ub marked RESOLVED by task 2a (historical note kept, "until then" wording adjusted) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/valid/50_wide_ops.rfs` | new | variable wide shifts (`w << k`, `w >> k`, `k: bit<8>`) + guarded bit<16> multiplication with product > INT_MAX; exact-MC-tractable safety spec |
| aggregate generated-code crate | regenerated | 184 -> 186 modules, 1501 -> 1519 tests (the new example module) |

## Changed functionality

- Generated C bitvector arithmetic is now WELL-DEFINED for all inputs: no promotion-based UB remains for `*`/`<<`/`>>`; byte-parity with the Rust backend on every input, including shift amounts >= width and wide products. Byte churn confined to C artifacts containing `<<`/`>>`/`*` (examples 20_shifts, 44_token_ring, 45_elevator + their aggregate modules).

## Removed functionality

- None.

## New functionality

- File-driven coverage of the fixed cases in ALL future sweeps (`examples/valid/50_wide_ops.rfs`).
- Targeted cross-backend differential pin on the exact divergent inputs (k = 8/31/32/64/255 + wide multiply) - deterministic regression, since random fuzzing alone may not hit k >= 32 quickly.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/wide_ops_parity_test.rs` | new | two tests: production drivers for BOTH backends compiled (rustc / gcc -Wall with a zero-warning assertion) and driven over identical boundary sequences, per-step state byte-identical + pinned; exact-MC status parity on the spec |
| `tests/codegen_test.rs` | changed | C byte re-pins where shift/mul artifacts are asserted (20/44/45) |
| `src/codegen_test.rs` | changed | lib random-trace corpus pin 46 -> 47 valid examples |
| `tests/backend_parity_test.rs` | changed | C pins affected by the lowering change re-pinned |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test --test wide_ops_parity_test` | OK - 2/2 (boundary sequences k = 8/31/32/64/255 + wide multiply; gcc -Wall zero warnings asserted in-test) |
| `cargo test` full suite (22 targets) | OK - 429 passed / 0 failed (re-verified 2026-10-24 on the post-task-3 tree) |
| `cargo clippy --all-targets` | OK - no errors; 13 warnings = the BUILD.md documented baseline |
| `cargo fmt --check` | OK |
| `cargo generate-code-tests && cargo test-generated-code` | OK - aggregate crate regenerated: 186 modules, 1519/1519 pass |

## Found problems/issues/mismatches
- None recorded during implementation (the motivating divergence was found in task 1's cross-check). The uniform shift guard did NOT trip gcc -Wall, so the "Other information" contingency (literal-only fast path) was not taken.

## Other important findings/information

- This status was written during a post-commit re-check pass (2026-10-24): the task was implemented and committed in `076b315`, but the status file was never written. All exit criteria were re-verified green on the current tree before writing this file.
- The re-pin sweep was one pass as required: as of this re-check (tree includes task 3), no later Phase 6 task needed to re-pin C arithmetic bytes - task 3's backend changes were report-format/config-key only (D-P6-10 unified reports).
