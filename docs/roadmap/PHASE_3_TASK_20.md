# Phase 3, Task 20 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 20: regenerate training snippets + aggregate after the task-19 additions,
run the training tests, and update BUILD.md expected counts if they changed.

## Context

- Pipeline (BUILD.md): `cargo extract-training-snippets` (pulls rfs-tagged code blocks
  from all training jsonl into `training/<cat>/snippets/`, classified by parser+
  validator) -> `cargo build-training` (writes `all_rfs_training.jsonl`) ->
  `cargo test-training` / `tests/training_test.rs` (validates format + counts).
- Current expected counts live in BUILD.md sections 3-4 and are asserted by the
  training tests - they WILL change with task 19's new examples/snippets.
- Requires task 19 done; also re-runs after any snippet-affecting change, so run it
  LAST among content tasks (only task 21 docs + task 22 close-out follow).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 19. | Training-set update | the new examples provide the new snippets/content |

### Affected training files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/<cat>/snippets/**` | change (regenerated) | snippets extracted from the updated jsonl files |
| `all_rfs_training.jsonl` | change (regenerated) | aggregate training file |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | sections 3-4 expected snippet/aggregate counts, only if they changed |

## Planned actions

### Action 1: Run the training pipeline

Run `cargo extract-training-snippets && cargo build-training && cargo test-training`
(toolchain env per START.md section 4). If snippet extraction rejects a new rfs block,
the block does not parse/validate - fix the BLOCK in the task-19 files (record as a
task correction there) and re-run; do not loosen the extractor.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/<cat>/snippets/**` | change (regenerated) | reflects task 19 content |
| `all_rfs_training.jsonl` | change (regenerated) | final aggregate for training runs |

### Action 2: Update BUILD.md counts if changed

Compare the new snippet/aggregate/category counts against BUILD.md sections 3-4 and
`tests/training_test.rs` expectations; update the doc numbers where they changed.
(If a test asserts old hard-coded counts, update the assertion to the new actuals -
that is part of keeping `cargo test` green.)

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change (sections 3-4 counts) | docs must state the new actual counts |
| `tests/training_test.rs` | change (only if it hard-codes old counts) | keep the suite green with the new actuals |

## Task deliverables
- Regenerated snippets + aggregate; BUILD.md counts current; training tests green.

## Task exit criteria
- [ ] All three pipeline commands exit 0.
- [ ] `cargo test --no-fail-fast` fully green (training_test included).
- [ ] BUILD.md sections 3-4 numbers match the regenerated actuals (record old->new in status file).

## Non-goals
- No changes to training content beyond regeneration; no training.py changes.

## Other information

- Record exact count deltas (per category + aggregate) in the status file - task 22
  copies them into PHASE_3_STATUS.md "Other file changes" / "Tests run status".
