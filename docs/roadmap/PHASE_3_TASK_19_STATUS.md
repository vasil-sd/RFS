# Phase 3, Task 19 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_19.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (training-data task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/PHASE_3_TASK_19_STATUS.md` | new | this file. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/properties_temporal_logic/rfs_training.jsonl` | changed (appended 3 lines, 12 -> 15) | new SFT examples: (13) `always[N]` semantics on a minimal startup module (window samples 0..N-1, violation at the first failing sample, N >= 1); (14) monitor-compilation concept + D-P3-3 deadline walkthrough (obligation trigger/body/bound, confirmed at i+N-1); (15) R1-R4 restrictions with one accepted prop and three skipped props (verified against actual codegen warnings). |
| `training/model_checking_properties/rfs_training.jsonl` | changed (appended 2 lines, 37 -> 39) | new SFT examples: (38) online monitors in the MC product state + online assumption pruning incl. liveness assumptions; (39) enhanced counterexample format with real verified Rust MC output (trigger step 1, deadline 3, exit 1, Rust/C parity). |

## Changed functionality

None.

## Removed functionality

None.

## New functionality

- 5 new SFT examples covering the final Phase 3 behavior: `always[N]` semantics, online monitor compilation (D-P3-3), R1-R4 restrictions with exact warning text, online MC monitoring + assumption pruning, enhanced counterexample format.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no test-code changes; `cargo test-training` re-validates the new lines via `aggregate_training_sets_matches_category_files`. |

## Tests run status

| Name | Status |
|---|---|
| all 5 rfs blocks of the new examples via `cargo run -- validate` | OK - all parse + validate (full modules, not fragments) |
| line-15 module through `codegen rust production` | OK - exactly 3 skip warnings with the exact text (`property '<name>' is outside the monitor subset; skipped`); `ok_obligation` compiled (`PROPERTY_LABELS = ["ok_obligation"]`) |
| line-39 module through the Rust MC (`--bound 8 --runs 20`, seeds 1/7/42/777) | OK - deterministic counterexample quoted in the example (trigger 1, violation 3), exit 1 |
| `cargo test --test training_test` (4) | 3 passed; 1 EXPECTED failure: `committed_all_rfs_training_file_is_fresh` (all_rfs_training.jsonl stale: 219 vs 224) - by design until task 20 runs `cargo build-training` |

## Found problems/issues/mismatches

- RFS v0 formula grammar has no `not` keyword - negation is the prefix `!`; and `next` requires a parenthesized operand (`next (...)`). The first draft of line 15 used `not be[N] (...)` / `next be[N] (...)`, which failed to parse ("trailing input after RFS source"); fixed to `!be[N] (...)` / `next (be[N] (...))` before appending.
- All pre-existing rfs blocks in `properties_temporal_logic` are bare formula fragments (the category has 0 valid snippets, 16 invalid). Per the task file's "must parse + validate" discipline and "minimal module + property under discussion", the new examples use complete minimal modules instead - line 13 will be the first block in this category to extract as a VALID snippet (task 20 re-extracts).

## Other important findings/information

- Expected transient state: `cargo test-training` is red on `committed_all_rfs_training_file_is_fresh` until task 20 regenerates snippets + `all_rfs_training.jsonl` (219 -> 224 lines; valid snippets expected to grow 120 -> 125 since all 5 new blocks are complete modules). Task 20 records the final numbers.
- The counterexample quoted in line 39 is REAL output from the Rust MC backend for the `never_ack` module (deterministic across seeds because the trace is forced: only Req steps from Idle, only Tick from Pending).
