# Phase 14, Task 9 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_9.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `crates/examples/c_sync_handler/src/handler.c` | changed (include + comment) | `#include "arq_fsm.c"` -> `#include "arq_fsm.h"`; the header comment no longer claims a single translation unit - the FSM implementation lives in arq_fsm.o and is linked by the Makefile. No `.c` include remains. |
| `crates/examples/c_sync_handler/Makefile` | changed (build rules) | Explicit object-file rules: `$(CC) -Wall -I. -c build/arq_fsm.c -o build/arq_fsm.o` (the generated source self-includes `"build/arq_fsm.h"`, hence `-I.`) and `$(CC) -Wall -I$(BUILD_DIR) -I. -c src/handler.c -o build/handler.o`; a link rule `$(CC) -o c_sync_handler build/handler.o build/arq_fsm.o` (main's object first). The zero-warning bar now holds for BOTH compiles. `clean` already removed the objects via `rm -rf $(BUILD_DIR)` (commented). The generation rule was already on the new flags from task 2 (`--out-fsm-header/--out-fsm-code`). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/c_sync_handler/README.md` | changed | Layout section: handler.c includes ONLY the generated header (separate translation unit); the Makefile bullet describes the explicit object compiles + link and the two-compile zero-warning bar. |
| `crates/examples/fuzz_c/Makefile` | changed (generate rule comment + delete-dance removal) | The `rm -f build/spec_test.c` line is GONE - under the new CLI the skeleton is a named slot, so every artifact of the `test` command lands in build/ and `make clean` removes them all; comments updated to the three-slot shape (test-executable source kept at build/spec_test.c, unused by this example). The both-source clang build rule was already in place from task 4. |
| `crates/examples/fuzz_c/README.md` | changed | Layout: build/ now lists the header/source pair + skeleton + co-emitted spec_test.c; the CLANG-requirement recipe shows the both-source compile (`clang -O1 -fsanitize=fuzzer,address -I. -o fuzz_target build/rfs_generated_fsm.c build/fuzz_target.c`). |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-9 complete, tasks 10-11 pending. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

(none.)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | unchanged | Action 9.3 required NO migration: task 4 had already moved the fuzz_c pins to the header-include assertion (`#include "build/rfs_generated_fsm.h"`) and the new generated-file names, and the c_sync_handler leg's zero-gcc-diagnostic bar greps `handler.c:` / `arq_fsm.c:` - which now covers arq_fsm.o's compile exactly as intended. The suite passes unmodified against the rewritten Makefiles. |

## Tests run status

| Name | Status |
|---|---|
| `make clean && make` in c_sync_handler (manual, RFS_BIN pinned) | OK - generation + both object compiles with ZERO gcc -Wall diagnostics (the only stderr lines are rfs codegen's expected monitor-subset warnings, which the test's diagnostic grep does not match), link, run exit 0 |
| c_sync_handler scenario output | byte-identical to the pinned expectation: 19 ticks / frames [0..8] / acks [0..8] / 5 dropped out-of-band acks / max window 4 (bound 4) |
| `make clean && make` in fuzz_c (manual, clang 21.1.8 present) | OK - all four artifacts generated into build/ (no delete-dance), both-source clang build clean |
| `./fuzz_target -max_total_time=10` (bounded smoke run) | OK - exit 0 after 1,127,031 runs in 11 s, no crashes |
| `cargo test --test example_projects_test` | OK - 3 passed / 0 failed |
| `cargo test --workspace` | OK - 676 passed / 0 failed (exact task-8 baseline) |
| `cargo fmt --check` | OK - clean |
| clippy | no NEW warnings by construction: this task changed zero `.rs` files (.c / Makefile / README only); the warning set is byte-identical to HEAD's (task 8's run on this same tree state: rfs 12, rfs-adapters 3, async_handler 1, sync_handler 1) |

## Found problems/issues/mismatches
- None. The task file's description of fuzz_c's current state ("generates via `--out $(BUILD_DIR)/spec_test.c` then deletes") was already partially outdated when this task started: tasks 2 and 4 had migrated both Makefiles' generation rules to the new flags and the test pins to the header-include assertion. The remaining gaps were exactly the delete-dance (fuzz_c) and the single-translation-unit build (c_sync_handler), both closed here.

## Other important findings/information
- The generated fsm source's self-include carries the RESOLVED header name (`"build/arq_fsm.h"` / `"build/rfs_generated_fsm.h"` - relative, as stamped by the Makefile's flags), which is why arq_fsm.o's compile needs `-I.` while handler.o's needs `-I$(BUILD_DIR)` for its bare `#include "arq_fsm.h"`. Both flags are pinned in the Makefile rules.
- fuzz_c's build/ now retains spec_test.c (the test-executable source) instead of deleting it - documented in the Makefile comment and README as an unused co-emitted artifact; `make clean` removes it with everything else.
