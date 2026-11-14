# Phase 14, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none - the change re-partitions how the existing test/fuzz emission references the FSM and how the C pipelines compile it; no new file.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/backends/c/test.rs` | changed | `generate_unit_tests` no longer inlines the FSM core: it calls the shared `generate_fsm_pair` (monitors Always) and fills BOTH fsm slots so the write loop applies take-or-emit; the test source is now line-1 + `#include "<fsm header>"` + stdio/stdlib + the harness/suites/main (the implementation includes moved here from the inlined core). `emit_fuzz_target_c` gains a `fsm_header_name` parameter and replaces `#include "rfs_generated_fsm.c"` with `#include "<fsm header>"`; its wiring comments are rewritten to the new CLI (`rfs codegen c test <spec> --out BASE` -> four files; build with clang over BOTH sources). |
| `src/cli/commands/test.rs` | changed | C unit pipeline: writes the fsm pair + test.c under their standalone default names, compiles EACH with `gcc -Wall -c` (zero-warning bar) and links into the test binary. C fuzz pipeline: `FuzzContext` now sources the fsm pair from the TEST codegen output (monitors Always - a separate monitors-Off production dispatch would emit a header lacking the prop fields the tests use), writes fsm source + header + skeleton, and clang-compiles BOTH sources together (the "compile ONLY the skeleton" multiple-definition constraint is gone). `--print-script` heredocs regenerated for the object+link unit build and the both-source fuzz build. |
| `crates/examples/fuzz_c/Makefile` | changed | The clang rule now compiles `$(GEN_FSM_C)` + the skeleton together (the skeleton no longer source-includes the fsm .c). A minimal piece of task 9's scope pulled forward - see plan correction 4b. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-4 complete, tasks 5-11 pending. |

(User-facing / dev architecture docs for the test/fuzz header-reference model are task 8's scope - the phase plan defers the doc re-sync there.)

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `tests/utils_test.rs` | changed | `--print-script` heredoc pins regenerated from the NEW script (object+link unit build; both-source fuzz build; the fsm source heredoc renamed `__RFS_FUZZ_FSM_C__`, the header `__RFS_FUZZ_FSM_H__`; the clang line now names both sources). |
| `tests/codegen_test.rs` | changed | C test legs re-pointed at the fsm-header slot (monitors-Always types live there, not in the primary source); the fuzz-skeleton compile leg writes the TEST codegen's own fsm pair (not a separate monitors-Off production header). Added two tests: `cli_c_test_writes_fsm_pair_and_references_header` (the four-file `--out arq` output + both .c include `arq-fsm.h`) and `no_generated_c_source_includes_a_c_file` (the grep bar over every C command x every valid example). |
| `tests/backend_parity_test.rs`, `tests/c_compiler_agnostic_test.rs`, `tests/toolchain_fuzz_test.rs`, `tests/info_format_test.rs`, `tests/example_projects_test.rs` | changed | C test/fuzz legs migrated to the header-reference + object/both-source build shape; example_projects_test's skeleton-include assertion re-pointed from `rfs_generated_fsm.c` to `build/rfs_generated_fsm.h`. |

## Changed functionality
- The C `test` artifact and the emitted `fuzz_target.c` skeleton now reference the FSM through its header (`#include "<fsm header>"`) instead of inlining / source-including the fsm `.c`. The FSM implementation exists exactly once, in the fsm source.
- `rfs test --backend c`: the unit pipeline compiles the fsm source and the test source to objects and links them; the fuzz pipeline clang-compiles the fsm source and the skeleton together. No generated C artifact source-includes another `.c`.

## Removed functionality
- The inlined FSM core inside the C test artifact (it now includes the fsm header).
- `#include "rfs_generated_fsm.c"` in the fuzz skeleton (replaced by the fsm-header include).
- The "compile ONLY the skeleton" constraint in the C fuzz pipeline and its print-script heredoc (both sources are now compiled together - no multiple-definition risk).

## New functionality
- `rfs codegen c test <spec> --out BASE` writes FOUR files: `BASE-fsm.h`, `BASE-fsm.c`, `BASE-test.c`, `BASE-fuzz-target.c`; both `.c` artifacts include `BASE-fsm.h`.
- The standing grep-bar regression test `no_generated_c_source_includes_a_c_file`: across every C codegen command (production / mc / test) and every valid example, no generated C artifact source-includes a `.c` file.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs::cli_c_test_writes_fsm_pair_and_references_header` | new | CLI: `rfs codegen c test --out arq` writes all four artifacts; both `.c` include `arq-fsm.h`; neither source-includes a `.c`. |
| `tests/codegen_test.rs::no_generated_c_source_includes_a_c_file` | new | The grep bar: every C command x every valid example, no generated C slot contains an `#include "...*.c"`. |
| `tests/codegen_test.rs` (C test/fuzz legs) | changed | Re-pointed at the fsm-header slot; the fuzz compile leg uses the TEST codegen's own monitors-Always fsm pair. |
| `tests/utils_test.rs` (`--print-script` pins) | changed | Regenerated from the new object+link / both-source script. |
| `tests/{backend_parity,c_compiler_agnostic,toolchain_fuzz,info_format}_test.rs` (C test/fuzz legs) | changed | Migrated to the header-reference + object/both-source build shape. |
| `tests/example_projects_test.rs` (`fuzz_c_generates_without_clang_and_builds_when_available`) | changed | Skeleton-include assertion re-pointed from `rfs_generated_fsm.c` to `build/rfs_generated_fsm.h`. |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, all 32 suites) | OK - 0 failed (codegen_test 55 passed incl. the two new tests) |
| `cargo fmt --check` | OK |
| `cargo clippy --lib` | OK - at baseline (13 lib warnings; task 4 adds ZERO new - the one warning in a touched file, `TestSuite::from_str` should_implement_trait at cli/commands/test.rs:43, is pre-existing in HEAD from an earlier task) |

## Found problems/issues/mismatches
- The C fuzz pipeline originally obtained its fsm source via a SEPARATE `Production.run()` dispatch. That emits a monitors-OFF header by default, which lacks the `prop_monitors_t` / `rfs_property_violation_t` fields the test artifact (monitors Always) uses - so the skeleton would not compile against it. The fix: source the fsm pair from the TEST codegen output (which is monitors-Always), so the fuzz target drives the IDENTICAL fsm the unit tests do. This is a refinement of action 4.2's "write the fsm pair + skeleton", not a change to its stated goal.
- The `fuzz_c` example's clang rule compiled ONLY the skeleton (relying on the source-include). Once the skeleton references the header instead, that rule no longer links the fsm - so the both-source build line had to be pulled forward from task 9 (plan correction 4b) to keep `example_projects_test` green.

## Other important findings/information
- Task 1 was completed and committed (in `961540b`, together with task 2) but never received a `TASKS.md` entry - a pre-existing bookkeeping gap, flagged here for backfill (it is NOT part of this task's scope).
- The grep bar is the standing regression for the whole-source-inclusion pattern this phase removes: any future generator that re-introduces an `#include "...*.c"` in generated C fails it across every command and example.
