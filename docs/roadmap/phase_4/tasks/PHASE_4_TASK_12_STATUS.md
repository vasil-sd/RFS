# Phase 4, Task 12 - Implementation status

> Written once when all exit criteria of `PHASE_4_TASK_12.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-19

## Added source files

None (training-data task; no `src/` changes).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| - | - | none (design-doc sync is task 13's scope) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/properties_temporal_logic/rfs_training.jsonl` | changed | +3 SFT entries: (1) eventually/until syntax + precedence + WEAK-until semantics with a valid 2-state ARQ-lite spec; (2) invalid variants - `eventually` without parens (parse error) and non-boolean operand (validator "must be bool"), verbatim diagnostics; (3) D-P4-9 scope: deeper nesting (`always (eventually (...))`) -> codegen diagnostic + skip, and the monitor-subset boundary (liveness = MC-only) |
| `training/model_checking_properties/rfs_training.jsonl` | changed | +4 SFT entries: (1) exact-MC status taxonomy pass-complete/fail/unknown-state-limit with exit codes 0/1/2 + pinned summary-line format + statistics block; (2) liveness counterexample = lasso (prefix + one cycle iteration), `step` = cycle start, human fields formula/violation step/cycle length/note; (3) input abstraction D-P4-3 (unused bit<4>+bit<32> Ack payload -> single representative values, 6-state run); (4) unknown-state-limit report + truncated-analysis soundness (FAIL inside explored part always trustworthy; never a false pass-complete) |
| `training/<cat>/snippets/**` | regenerated | via `cargo extract-training-snippets`; new snippets bucketed by parse+validate (see counts below) |
| `all_rfs_training.jsonl` | regenerated | via `cargo build-training`; 224 -> 231 lines |

## Changed functionality

None - no tool behavior changed; training data only.

## Removed functionality

None.

## New functionality

- SFT coverage of the Phase 4 language + MC surface (7 new examples across 2 categories):
  - `eventually (p)` / `p until q` syntax, mandatory parens, precedence (`!` tighter than `until`; `&&`/`||` looser), left-associativity
  - WEAK-until semantics (q-at-some-step-with-p-before OR q-never-and-p-forever) + strong-until encoding `p until q && eventually q`
  - invalid variants with verbatim diagnostics (parse: "trailing input after RFS source"; validator: "formula expression must be bool, found Bit(8)")
  - D-P4-9 liveness scope + deeper-nesting codegen diagnostic + monitor-subset boundary
  - exact-MC three-status taxonomy + exit codes + machine summary-line format + statistics block
  - input abstraction rule (D-P4-3) with a wide-payload spec
  - liveness lasso counterexample shape and field-by-field reading guide

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no test-code changes; existing `training_test` (structure/freshness/aggregate/snippet-parse) now covers the 7 new lines and their snippets |

## Tests run status

| Name | Status |
|---|---|
| `cargo extract-training-snippets` | OK (130 valid / 127 invalid snippets) |
| `cargo build-training` | OK (231 examples aggregated) |
| `cargo test-training` (4) | OK |
| `cargo test` (227) | OK (unchanged from pre-task baseline - training data adds no Rust tests; snippet suites green on the new snippets) |
| `cargo clippy --all-targets` | OK (baseline only: 1 pre-existing codegen_test warning, zero new) |
| `cargo fmt --check` | OK |

## Old -> new counts (record for task 14)

| Metric | Old | New |
|---|---|---|
| `all_rfs_training.jsonl` lines | 224 | 231 (+7) |
| snippets valid | 125 | 130 (+5) |
| snippets invalid | 125 | 127 (+2) |
| `properties_temporal_logic` jsonl lines | 15 | 18 (+3) |
| `properties_temporal_logic` snippets | 3 valid / 16 invalid | 5 valid / 18 invalid |
| `model_checking_properties` jsonl lines | 39 | 43 (+4) |
| `model_checking_properties` snippets | 38 valid / 50 invalid | 41 valid / 50 invalid |

New snippet bucketing: valid = liv_pass (ARQ-lite with eventually+until), nested (validates; skip is codegen-level), safe_pass, liv_fail, deliver; invalid = ev_noparen (parse fail), ev_nonbool (validate fail).

## Found problems/issues/mismatches
- None blocking. Observation for task 13/future parser work: the missing-parens case `eventually state == Done` surfaces as the GENERIC parse error "trailing input after RFS source" (the `until_formula`/primary ladder falls through to a non-boolean expression and leaves the rest unparsed) rather than a dedicated "eventually requires parentheses" diagnostic. The training entry teaches the real message; a targeted parser error would be friendlier.

## Other important findings/information
- Every output block quoted in the new entries was captured from REAL tool runs on 2026-10-19 (Rust backend, `rustc --edition 2021 -O`; parity guarantees byte-identical C output): liv_pass pass-complete 8 states; liv_fail liveness FAIL (states=1, step=0, cycle length 1); safe_pass pass-complete 2 states; deliver pass-complete 6 states (abstraction active); 44_token_ring capped at `--max-states 1000` -> unknown-state-limit (2964 transitions, 741 unexpected terminations).
- Snippet classification rule re-confirmed (`src/training.rs`): a snippet lands in `snippets/valid/` only if it PARSES AND VALIDATES; parse or validation failure -> `snippets/invalid/`. Dedup is per-category.
- All new example specs are small and self-contained (per task file "Other information"); shapes reuse examples 43-45 patterns (Tick-driven ARQ-lite, unused wide payload) without copying them wholesale.
- Scratch ground-truth specs used during development live in `/tmp/rfs_t12/` (outside the repo; not committed).
