# Phase 10, Task 13 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_13.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 62 function rows (training.rs 10, src/bin 4, rfs-build 13, rfs-adapters 21, sync_handler 7, async_handler 6, fuzz_rust build 1) + 12 type rows (SnippetCounts, RfsBuildError, RfsBuild, ProtocolFsm, ChannelOutcome, ChannelFsm, AsyncOutcome, AsyncFsm, CallbackResult, Minimal, and the two example-crate ArqReport types) appended - FINAL non-test chunk; Component/Functionality still blank |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 13a recorded (missed demo bins + associated-type phantoms; authoritative final inventory total 938 fns / 197 types) |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_13.md` | changed | stale figures corrected in place: Goal ~55/~19 -> ~58/~12 before bins, chunk total ~59/~19 -> ~62/~12; adapters note 14 types -> 7 (seven associated-type bindings were counted as types); sync_handler note "3 types" -> 1 (Fsm/Input/Output come from the include!'d generated artifact) + main.rs added; async_handler note 3 fns -> 4 (the async step helper) + main.rs added; Planned actions ~73/~21 -> ~62/~12 |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The non-test inventory is COMPLETE: every non-test source file in the repo now has rows (verified by a full file-coverage sweep - only src/backends/mod.rs lacks rows, legitimately 0 fns / 0 types per correction 2a). training.rs is the training-set aggregation + JSONL validation + ```rfs snippet extraction pipeline behind the two training bins; the four src/bin wrappers are the thin CLI entries for training/test-generation tooling; rfs-build is the dependency-free build.rs helper (RFS_BIN -> PATH resolution, cargo rerun directives, builder options); rfs-adapters is the #![no_std] ProtocolFsm trait + four generic adapters (channel / async stream / callback driver / minimal wrapper) that never depend on generated code; the example crates drive the spec-43 sliding-window ARQ sender over each adapter with session-as-test + demo binary, and fuzz_rust wires the emitted libfuzzer_sys skeleton into the cargo-fuzz layout.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green (the ```rfs triple-backticks in two training.rs descriptions sit inside table cells, not fence position, so the fence checker is unaffected) |

## Found problems/issues/mismatches

- PLAN CORRECTION 13a recorded: the cluster notes' "~59 fns / ~19 types incl. bins" was wrong in both directions. Functions undercounted by 3 (the two demo main.rs binaries were never mentioned though the Goal's src/*.rs covers them; async_handler lib.rs holds 4 non-test fns, not 3 - the async step helper). Types overcounted by 7 (raw grep counted associated-type bindings as types: ProtocolFsm's own type Input/Output/Error; declarations, the Stream impl's type Item, minimal's three impl bindings - the same phantom class task 5's read-through established for codegen.rs and correction 9a for frontend.rs).
- Pre-existing discrepancy surfaced by the completeness sweep: correction 2a's "T5 66/40" was itself phantom-inflated by the two codegen.rs `type Err = String` bindings; task 5's status recorded the authoritative 38 but the corrections-table row was never updated. A-K's true type total is therefore 185, not the ~187 a naive sum of the corrections table would give.

## Other important findings/information

- COMPLETENESS NOTE (task deliverable): final inventory totals are 938 function rows + 197 type rows = 1,135 rows. Per-chunk authoritative sum: A 86/4, B 115/27, C 47/13, D 66/38 (task 5's read-through, not correction 2a's 40), E 57/29, F 71/3, G 73/2, H 89/19, I 75/7, J 41/12, K 156/31, L 62/12. Every Depends identifier in the inventory resolves (zero unresolved refs, full-table scan).
- Identifier-collision case handled with the trait-impl form: Minimal has BOTH inherent new/step and a hand-written ProtocolFsm impl of the same names - rows are Minimal::new / Minimal::step (inherent) vs Minimal(ProtocolFsm)::new / Minimal(ProtocolFsm)::step. The manual Debug/PartialEq impls on ChannelOutcome/AsyncOutcome/CallbackResult (bounded on the associated types, since the generated Fsm derives neither) and AsyncFsm's Stream impl likewise use the trait-impl form.
- The two example-crate ArqReport types are deliberately separate rows (different files) even though their fields are identical - row identity is per-file by construction.
- Remaining tasks: 14 (categories proposal - can start now that the full inventory exists), 15-16 (classification fills), 17-18 (file mapping + proposed layout), 29-30 (test inventory).
