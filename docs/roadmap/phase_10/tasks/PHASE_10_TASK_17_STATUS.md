# Phase 10, Task 17 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_17.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | new `## Proposed naming scheme` section (inserted between `## Functionality` and `## File mapping`, so task 18's mapping table can reference the proposed paths): (a) six naming-convention rules, (b) the target directory tree in text form with all 63 non-test source files mapped exactly once (27 moves + 1 merge + 3 new mod.rs + 32 kept), (c) per-group rationale naming the task-14 components each group realizes, (d) risk/effort notes per non-trivial move. The proposal: new directories `lang/` (Parser, IR model, Printer, Validator, Spec hashing), `codegen/` (Codegen core, Shared FSM concepts, Shared test planning, Shared MC concepts, Monitor compiler), `mc/` (MC plan, SMT plan + emitter, Reference MC), `commands/` (+ nested `commands/diagram/`) (the five command components); kept at root: main/cli_log/lib/json_rw/frontend/ext_tool/specgen/training; `backends/`, `bin/`, and all of `crates/` stay in place |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase (proposal only, nothing moved or renamed).

## Removed functionality

None.

## New functionality

- The proposed naming scheme for the refactoring phase to execute:
  - Command modules drop the `_cmd` suffix under `commands/` (`test_cmd.rs` -> `commands/test.rs`, `gen_spec_cmd.rs` -> `commands/gen_spec.rs`, `mc_ref_cmd.rs` -> `commands/mc_ref.rs`; `info.rs` and `diagram.rs` already follow the rule and simply move).
  - Shared-plan files drop the redundant `codegen_` prefix under `codegen/` (`codegen_fsm.rs` -> `codegen/fsm.rs`, `codegen_test.rs` -> `codegen/test_plan.rs` - not `test.rs`, which belongs to the rfs-test command, `codegen_mc.rs` -> `codegen/mc_concepts.rs`; `monitor.rs` moves in unrenamed).
  - Backend files KEEP the `<lang>_<artifact>.rs` split (the three artifacts are genuinely separate responsibilities; merging would destroy the split the parity invariant is checked against).
  - Two zero-churn `mod.rs` absorptions: `codegen.rs` -> `codegen/mod.rs` and `info_format.rs` -> `info_format/mod.rs` keep their module paths, so every existing importer (including the detached fuzz workspace's `rfs::codegen::*`) keeps compiling untouched.
  - `specgen.rs` deliberately stays at the root because `fuzz/fuzz_targets/common.rs` imports `rfs::specgen::{Bounds, SpecGen}` by module path - moving it would force a lockstep edit inside the detached fuzz workspace.
  - Bins are never renamed: all four are cargo alias targets (`.cargo/config.toml`: build-training, extract-training-snippets, generate-code-tests, test-generated-code) plus BUILD.md/START.md citations - recorded per bin for the refactoring phase.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green after the section was written (the large fenced text tree parses cleanly under comrak GFM) |

## Found problems/issues/mismatches

- The constraint audit caught two gaps in the first draft, both fixed before completion: (1) `crates/examples/fuzz_rust/src/lib.rs` exists on disk (23 lines wrapping the generated artifact via include!; 0 own fns/types per task 13, so it has no inventory rows but IS a real source file) and was missing from the tree - added as kept; (2) `src/diagrams/mod.rs` had no entry of its own (its content merges into `commands/diagram/mod.rs`) - now an explicit merge annotation line. After both fixes, a mechanical context-aware walk verifies all 63 actual non-test .rs files are named exactly once in the tree (no duplicates, no phantoms).
- The effort summary's first draft said "29 file moves"; the correct count is 27 moves + 1 merge - corrected in place.

## Other important findings/information

- External import-site counts measured at proposal time (files under tests/, src/bin/, fuzz/ importing each moved module by path): validator 8, codegen_mc 5, ref_mc 5, ir 2, parser 2, spec_hash 2, info 2, diagram 2, diagrams 3, gen_spec_cmd 1, mc_ref_cmd 1, ebnf_grammar 1, json_schema 1; all other moved modules have 0 external importers. The internal churn is larger but purely mechanical (ir has 576 incoming edges - the biggest single rename).
- `//! Docs:` re-sync scope for the refactoring phase: exactly 28 of the 71 manifest paths change (the 27 moves + the diagrams/mod.rs merge - verified: all 28 are in tests/dev_docs_manifest.txt), plus every backticked src/... reference in docs/dev/ + docs/llm/ pointing at a moved file. Fragment IDs and ownership are unchanged; dev_docs_test's output after the move IS the update list.
- The one deliberate cross-backend coupling (c_* emitters borrowing snake/rust_ident from rust.rs) is flagged in the backends rationale for resolution by moving those utilities to a neutral module (e.g. codegen/ident.rs) during the refactoring phase.
