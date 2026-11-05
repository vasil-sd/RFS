# Phase 10, Task 23 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_23.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | 65 function rows appended to `## All functions`: 61 for `tests/codegen_test.rs` (backend registry behavior, per-command generation for both backends, option sensitivity incl. --with-property-monitors / --no-std / --with-protocol-fsm / --counterexample-format json, CLI argument parsing, the 50-example codegen sweep incl. the C-fuzz per-input free regression and the no_std compile sweep) + 4 for `tests/examples_test.rs` (all 50 valid examples parse+validate+round-trip, all 13 invalid rejected, file-driven so a new .rfs needs no test-code change). Every row carries bidirectional project Depends (mapped to real identifiers across src/{codegen,codegen_mc,backends/*,ir,json_rw,parser,spec_hash,validator,lib}.rs; consts like NO_STD_PRODUCTION_ONLY_ERROR / PROTOCOL_FSM_MC_ERROR / COUNTEREXAMPLE_FORMAT_ERROR get no Depends entries), a one-line description, and blank Component/Functionality/Documented-in cells. No type rows (see correction 23a). |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | Plan correction 23a: the cluster notes' "~67 fns / ~5 types" was wrong in both counts for codegen_test.rs - 2 of the grep's 63 fns are phantoms (`fn new` / `fn step` inside the PF_IMPL_BLOCK string template) and all 5 "types" are phantoms (2 RFS spec enums in the module m raw string + 3 associated-type bindings in PF_IMPL_BLOCK); authoritative 61 fns / 0 types for codegen_test.rs, 4 / 0 for examples_test.rs = 65 fns / 0 types total. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The codegen + examples test suites are now in the test inventory:
  - `tests/codegen_test.rs` (61 fns): backend registry, per-command generation for both backends, option sensitivity (--with-property-monitors, --no-std, --with-protocol-fsm, --counterexample-format json), CLI argument parsing, the 50-example codegen sweep incl. the C-fuzz per-input free regression and the no_std compile sweep.
  - `tests/examples_test.rs` (4 fns): the file-driven example suite - all 50 valid examples parse+validate+round-trip, all 13 invalid rejected; dropping a new .rfs into examples/valid|invalid works without test-code change.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written (the new rows are plain-text table cells, no backticked paths or mermaid, so they do not trip the gate's link/mermaid checks) |

## Found problems/issues/mismatches

- PLAN CORRECTION 23a: the cluster notes' "~67 fns / ~5 types" was wrong in both counts for codegen_test.rs. The grep's 63 fns include two phantoms (`fn new` / `fn step` inside the PF_IMPL_BLOCK backslash-continuation string template, lines 1449-1464), and all five "types" are phantoms (two RFS spec enums in the module m raw-string test input at line 1020 + three associated-type bindings in PF_IMPL_BLOCK - the correction-9a/13a phantom class). Authoritative: codegen_test.rs 61 fns / 0 types, examples_test.rs 4 fns / 0 types = 65 fns / 0 types total.
- A hand-rolled line-by-line string-state classifier over-triggered (it flagged 30 "phantom" fns including real top-level ones like `example_module`), so the phantom set was instead pinned down by mapping the file's two actual multi-line string templates (the module m raw string and PF_IMPL_BLOCK) - only those interiors hold phantom fn/type lines.
- An intermediate-file round-trip in the row generator turned deps lists into strings, so a `", ".join` mangled every non-empty Depends field char-by-char; caught on review, TEST_STRUCTURE.md reverted to the last commit and the rows regenerated in one in-memory pass (no round-trip).
- The first insertion glued the new `files` row onto the end of the previous row (the All functions table's last row had no blank line before `## All types`, so the anchor consumed the only separating newline); split back into two rows. A follow-up scan confirmed the other 14 "two |tests/ segments" lines are legitimate single rows whose Depends cell references a same-file helper, not glued rows.

## Other important findings/information

- The one real nested fn (`gen`, inside mc_exact_flags_embedded_as_runtime_defaults) gets the `<file>::<parent_fn>::<child_fn>` identifier `tests/codegen_test.rs::mc_exact_flags_embedded_as_runtime_defaults::gen`; a col-0-only grep misses it (60 vs the authoritative 61).
- codegen_test.rs references the rfs library both via `rfs::X` and via bare module paths (`codegen::X`, `backends::X`, `ir::X`) through a `use rfs::{codegen, ...}` alias - both forms map to the same src/ identifiers and were captured.
- All 65 rows have blank Component/Functionality (task 32) and Documented-in (neither file is a manifest file).