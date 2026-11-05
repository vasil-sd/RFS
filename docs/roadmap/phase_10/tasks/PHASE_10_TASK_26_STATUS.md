# Phase 10, Task 26 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_26.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TEST_STRUCTURE.md` | changed | 47 function rows + 5 type rows appended to `## All functions` / `## All types` for the reference-MC soundness cluster: 6 fns + 3 types for `tests/ref_mc_sweep_test.rs` (byte-parity sweep vs the generated Rust MC - fail-fast N-way parallel, per-sample timeout, failed-first reordering), 5 fns + 1 type for `tests/ref_mc_diag_test.rs` (the non-fail-fast diagnostic companion enumerating every mismatch in one shot), 24 fns for `tests/ref_mc_diff_test.rs` (ref-MC vs both generated engines over eligible examples + 20 pinned-seed random specs: status agreement, exact states/max_depth, minimal-violation-label ground truth, CE replay through ref_mc's evaluators, no-false-PASS negative control, the rfs mc-ref CLI itself), 12 fns + 1 type for `tests/toolchain_fuzz_test.rs` (the seeded compile-and-run CI layer: 3 seeds x 10 specgen specs -> all six artifacts generated + compiled + MC exact/fuzz vs ref_mc ground truth, stderr stability, the trigger-step-0 regression pin). Every row carries bidirectional project Depends (heavy rfs::ref_mc / rfs::specgen library-API edges per the task's expectation - mapped to real identifiers across src/{parser,validator,printer,ir,json_rw,codegen,codegen_mc,ref_mc,specgen,lib}.rs plus the task-20 tests/common/mc_json.rs zero_peak_rss helper), a one-line description, and blank Component/Functionality/Documented-in cells. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The reference-MC soundness cluster is now in the test inventory:
  - `tests/ref_mc_sweep_test.rs` (6 fns / 3 types): ref-MC byte-parity sweep vs the generated Rust MC over all tractable valid examples + fail-fast N-way parallelism (RFS_SWEEP_PAR), per-sample timeout (RFS_SWEEP_TIMEOUT_SECS), failed-first reordering via target/rfs_refmc_sweep/failed_order.txt, RFS_SWEEP_ONLY debug subset; in-process phases run in a watchdog thread so a stuck ref_mc cannot stall the suite.
  - `tests/ref_mc_diag_test.rs` (5 fns / 1 type): the same sweep without early exit - 8 workers enumerate every ref_mc-vs-generated-mc mismatch (OK/HANG/MISMATCH/ERROR per sample) in one shot.
  - `tests/ref_mc_diff_test.rs` (24 fns): the Phase 6 differential suite - eligible examples (dynamic enumeration minus two pinned exclusions, >=40 guard) + 20 small specgen random specs (pinned seeds) through both generated engines vs ref_mc: status agreement, EXACT states, max_depth agreement, label ground truth via minimal_violation_labels (label+step equality for unique-minimal specs), CE replay through verify_counterexample, the no-false-PASS negative control on 44_token_ring, and the rfs mc-ref CLI pinned against the in-process render + both engines.
  - `tests/toolchain_fuzz_test.rs` (12 fns / 1 type): the slow compile-and-run layer of the two-layer toolchain fuzzing - 3 pinned seeds x 10 small specgen specs (24099 = plan-correction-5a regression seed) -> render/re-parse/validate, all six artifacts generated + compiled (rustc / gcc -Wall), MC exact & fuzz vs ref_mc ground truth with NO FALSE PASS + stderr stability, the documented sample-0 assumption asymmetry, and the deterministic trigger-step-0 report pin (plan correction 5b).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written (the new rows are plain-text table cells, no backticked paths or mermaid, so they do not trip the gate's link/mermaid checks) |

## Found problems/issues/mismatches

- None recorded. The re-verified grep counts matched the task file's per-file Goal figures EXACTLY (6/3 + 5/1 + 24/0 + 12/1 = 47 fns / 5 types) and the read-through found no phantoms in any of the four files - no plan correction needed (the same outcome as task 22). The Context's "~47/~8" and the plan table's "~54/~10" were rougher planning-time estimates; the task file's exact per-file numbers govern and are confirmed.

## Other important findings/information

- The insertion used FULL-row anchors (identifier + complete Depends cell + description) instead of identifier-only anchors: once this chunk's own fn rows reference types like LivOut in their Depends cells, an identifier-only anchor such as `| tests/liveness_mc_test.rs::LivOut |` matches the reference inside another row's cell as well (the first attempt failed its uniqueness assertion before writing anything - the file was untouched). Future chunk tasks should anchor on full rows.
- All 52 rows have blank Component/Functionality (task 32) and Documented-in (none of the four files carries a `//! Docs:` line; all four DO have docs/llm/TASK_MAP.md suite entries, noted per the preamble's rule rather than repeated row by row - same practice as tasks 21-25).
