# Phase 3, Task 20 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_20.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (regeneration task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `BUILD.md` | changed | section 4 counts updated to regenerated actuals: aggregate codegen test crate 162 -> **171 modules**, generated unit tests 492 -> **513**, snippets 120 valid / 125 invalid -> **125 valid / 125 invalid**, aggregate 219 -> **224 examples**. (Section 3's main-test count line and section 5's dated Phase 2 baseline are left for task 21's broader BUILD.md sync.) |
| `docs/roadmap/PHASE_3_TASK_20_STATUS.md` | new | this file. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/properties_temporal_logic/snippets/valid/` (new dir, 3 files) | new (regenerated) | first VALID snippets ever in this category - the three task-19 full-module examples (startup, ack_window, subset_check). |
| `training/model_checking_properties/snippets/valid/snippet_0037.rfs`, `snippet_0038.rfs` | new (regenerated) | the two task-19 full-module examples (ticked_service, never_ack). |
| `all_rfs_training.jsonl` | changed (regenerated) | 219 -> 224 lines (task 19's 5 new SFT examples). |

## Changed functionality

None.

## Removed functionality

None.

## New functionality

- Regenerated training data: +5 valid snippets (120 -> 125), aggregate 219 -> 224; the generated-code test aggregate now covers 171 modules / 513 tests (was 166/498 after task 18, +5 snippet modules x 3 tests).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/training_test.rs` | unchanged | no hard-coded counts - expectations are computed dynamically from the category files, so no assertion updates were needed. |

## Tests run status

| Name | Status |
|---|---|
| `cargo extract-training-snippets` | OK - exit 0, "total: 125 valid snippet(s), 125 invalid snippet(s)" |
| `cargo build-training` | OK - exit 0, "aggregated 224 training examples" |
| `cargo test-training` (4) | OK - all pass, incl. `committed_all_rfs_training_file_is_fresh` which was red during task 19 by design |
| `cargo generate-code-tests && cargo test-generated-code` | OK - 171 modules, **513 passed / 0 failed** (new snippets' generated tests all pass) |
| `cargo test --no-fail-fast` (170) | OK - fully green |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- None. All task-19 rfs blocks extracted as VALID snippets on the first run (no extractor rejections, no block fixes needed).

## Other important findings/information

- **Exact count deltas for task 22** (old -> new): valid snippets 120 -> 125 (+5: properties_temporal_logic 0 -> 3, model_checking_properties 36 -> 38; all other categories unchanged); invalid snippets 125 -> 125 (unchanged); aggregate examples 219 -> 224 (+5); generated-code test crate 166 -> 171 modules, 498 -> 513 tests (+15 = 5 x 3). BUILD.md section 4 now states the new actuals.
- Remaining stale BUILD.md numbers (main-test count line in section 3: "125 pass" vs current 170; section 5's dated Phase 2 baseline) are task 21's scope per the plan ("BUILD.md ... updated test counts").
