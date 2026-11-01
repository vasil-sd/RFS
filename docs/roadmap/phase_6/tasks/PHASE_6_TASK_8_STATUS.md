# Phase 6, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (training-data-only task per the task non-goals).

## Changed/removed source files

None (`training.py` untouched - no GRPO/reward changes, per non-goals).

## Added/changed/extended documentation files

None (docs sync is task 9's scope; the BUILD.md baseline update consumes the
counts recorded in the task file's Other information).

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/syntax_semantics/rfs_training.jsonl` | changed | +3 entries: version header meaning/format with the REAL pinned validator error, optionality (legacy v0.x no-header specs), and why D-P6-1 formalized the existing mechanism instead of a comment-style header |
| `training/design/rfs_training.jsonl` | changed | +3 entries: stale-artifact detection via the line-1 header (real rust+c headers from spec 46, same hash both backends), what the spec hash covers + hand-rolled SHA-256 rationale (D-P6-3 / D-P5-2 stance), IR_VERSION 0->1 read policy with the REAL version-2 rejection error |
| `training/model_checking/rfs_training.jsonl` | changed | +4 entries: when a PASS is trustworthy (status taxonomy + no-false-PASS invariant, real spec-46 output), `--violate label:prob:percent%` mechanics with a real spec-48 run (161/200 assumption stops) + why bounded obligations only confirm at the deadline, `rfs mc-ref` quick-check workflow (flags, exit codes 0/1/2, real spec-43 output), `unknown-state-limit` semantics with a real spec-44 `--max-states 100` run |
| `training/examples_explain/rfs_training.jsonl` | changed | +3 entries: full-spec walkthroughs of the new benchmark specs 47 (HTTP/2-like stream) / 48 (USB control transfer) / 49 (CAN arbitration) in the category's established structured format, each with its REAL exact-mode MC result (434/21857/7, 104/340/15, 240/448/41 - matching the benchmark pins) |
| `training/frontend_cli/rfs_training.jsonl` | changed | +4 entries: `rfs mc-ref` flags + exit codes (real missing-file error), `rfs gen-spec` both modes with real random (seed 42 small) and mutation (base 03, seed 7) outputs + the validity guarantee, the mutation workflow with a real diff, and mc-ref vs generated-binary choice guidance |
| `training/<cat>/snippets/{valid,invalid}/` (5 categories) | regenerated | auto-extracted snippets: 135 valid + 127 invalid -> **138 valid + 128 invalid** (+3 valid / +1 invalid; most quoted outputs are ```text/```sh blocks which snippet extraction does not collect - only RFS code blocks count) |
| `all_rfs_training.jsonl` | regenerated | aggregate SFT set: **244 -> 261 lines** |

## Changed functionality

None - training data only; no toolchain behavior change.

## Removed functionality

None (append-only within the five category files, per the task non-goals).

## New functionality

- SFT coverage of the new v1.0 surface in the established category-local format
  (two messages, `< think>` numbered-reasoning block, verbatim REAL tool output):
  - version header + versioning policy/spec hash (`syntax_semantics`, `design`);
  - trustworthy MC reports + forced violations + `rfs mc-ref` (`model_checking`);
  - the new benchmark specs 47/48/49 (`examples_explain`);
  - the two new CLI commands `mc-ref` / `gen-spec` with flags, exit codes and
    real example runs (`frontend_cli`).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | the existing training suites verify the new data: structure/freshness/aggregation + the codegen aggregate test over the new valid snippets |

## Tests run status

| Name | Status |
|---|---|
| `cargo test-training` (5 = 4 dedicated + codegen aggregate) | OK |
| `cargo extract-training-snippets` (138 valid / 128 invalid) | OK |
| `cargo build-training` (261 aggregate lines) | OK |
| `cargo generate-code-tests && cargo test-generated-code` (192 modules, 1683 pass) | OK - regenerated for tree consistency; count feeds task 9's BUILD.md re-pin |

## Found problems/issues/mismatches

None recorded. All quoted outputs captured live from the v1.0.0 toolchain
during execution (or reused verbatim from earlier same-session captures that
were re-verified); no entry documents fiction.

## Other important findings/information

- **Counts for task 9's BUILD.md baseline** (also in the task file's Other
  information): aggregate 244 -> 261 lines; snippets 135+127 -> 138+128;
  category line counts syntax_semantics 11->14, design 12->15,
  model_checking 27->31, examples_explain 36->39, frontend_cli 11->15;
  generated-code aggregate crate 189 modules / 1601 tests -> 192 modules /
  1683 tests.
- The +3 valid snippets each generate a FULL extended test module (per-
  transition, invariant, monitor-violating-trace, random-trace classes), which
  is why the generated-code count grew by 82 tests for 3 snippets.
- No new categories were created and no existing entry was reworded
  (category-local + append-only rules held; git status confirms all changes are
  under `training/` + `all_rfs_training.jsonl`).
