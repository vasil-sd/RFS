# Phase 10, Task 15 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_15.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | Component + Functionality cells set on ALL 938 rows of the `## All functions` table (the types table's 197 rows stay blank - task 16). Base component per row = the file's category from the task-14 Components table; base functionality per row = what the function does, not where it sits. Cross-cutting additions: 8 rows carry two components (rust.rs `rust_ident`/`rust_binding_ident`/`snake` also C backend - the deliberate cross-backend coupling flagged in task 14; printer.rs `roundtrip_rfs`/`roundtrip_json` also Parser - the round-trip path; info.rs `render_expr`/`render_pattern_variant_group`/`render_output` also Diagram command + plugins - the shared label vocabulary diagrams/mod.rs calls). 301 rows carry multiple functionalities per the under-assignment-is-worse rule (e.g. codegen.rs `CodegenCommandKind::run` = codegen-production + codegen-tests + model-checking + provenance; ref_mc.rs rows split across model-checking / property monitoring / PSL translation / wire-format serialization / source rendering by what each function does) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The fully classified `## All functions` table: every one of the 938 function rows now carries Component + Functionality cells drawn exclusively from the task-14 vocabulary (32 components, 20 axes - zero out-of-vocabulary values, machine-checked). Coverage: all 32 components and all 20 axes are referenced by at least one row; component mentions total 946 (8 multi-valued rows), functionality mentions 1267 (301 multi-valued rows).
- Classification judgments recorded for the refactoring consumer:
  - Formula-tree handling is tagged PSL translation at every home the axis names: parser formula chain (11 rows), validator formula typing (5), codegen_fsm extraction (2), monitor compilation (5), SMT conversion (6), MC liveness decomposition (5), ref_mc reference evaluation (16), specgen formula generation/rewriting (13).
  - Monitor machinery inside the emitters is tagged property monitoring (rust_fsm 14 rows, c_fsm 21 rows, plus monitor_layouts in mc_plan and the test planner's invariant/monitor-violation classes).
  - Fuzz-runtime emission is tagged fuzzing at its three homes: both MC emitters' generate_model_checker + fuzz_runtime/fuzz-target emitters, the codegen_test LCG + decode plan, and the specgen Rng (shared constants with the fuzz wire format).
  - Provenance follows the D-P6-3 stamp: ir.rs `Ir::new` + version read policy, spec_hash.rs, codegen run's hash stamping, both emitters' artifact-header emission, rust_test's aggregate-crate stamp, and info_format's artifact-header document.
  - CLI surface covers option parsing/help/usage/dispatch/logger rows across codegen core, main.rs, cli_log.rs, the three thin command modules, and frontend's Format/file-planning helpers; exit-code mapping rows (codegen_mc `ExactStatus::exit_code`, mc-ref `exit_code_for`) carry both model-checking and CLI surface.
  - The wire-format serialization axis covers json_rw, ir.rs to/from_json pairs, codegen_mc report builders, json_schema output, ref_mc's JSON report + CE wire handling, frontend's convert/render paths, and training's JSONL validation/aggregation.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - 12 runs, one after each cluster save (chunks B, A, C, D, E, F, G, H, I, J, K, L), all green first try |

## Found problems/issues/mismatches

- None recorded. No plan correction was needed: the task-14 vocabulary classified every row without gaps or collisions, and no assignment required re-reading a source file beyond the 10-row spot-check (all ten confirmed against the code - e.g. `crate::backends::rust::snake` call sites in c_fsm.rs, `spec_hash16` stamping at codegen.rs:412, `crate::info::render_expr` in diagrams/mod.rs, the specgen validator-gate rollback at specgen.rs:1770).

## Other important findings/information

- Editing mechanics that kept the exit criteria mechanically verifiable: every row is canonical `| a | b | c | d | e | f |` with single-space cell padding (empty cells are exactly two spaces); 27 rows contain escaped pipes (`\|`) in Depends/description and 106 have an empty Depends cell, so the fill was done by unescaped-pipe split + positional cell replacement (cells 3-4 only) + rejoin - never a blind string replace. `git diff` proves exactly 938 line-replacements with Function/Depends/Short description/Documented in byte-identical on every pair.
- The table's physical row order is chunk B, A, C..L (chunk A was appended after B during enumeration) - tasks 16-18 should work the same physical ranges: B 46-160, A 161-246, C 247-293, D 294-359, E 360-416, F 417-487, G 488-560, H 561-649, I 650-724, J 725-765, K 766-921, L 922-983.
- docs_md_test runtime is ~185 s per run (it parses every repo .md through the mdr stack); it was run after every cluster save per the plan's command rule.
