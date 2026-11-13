# Phase 14, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none - the split re-partitions the existing emission in `fsm.rs`; no new file.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen/backends/c/fsm.rs` | changed | The single-string `Gen` emission is re-partitioned into a header buffer (include guard + type std-includes + the `RFS_SPEC_VERSION`/`RFS_PROPERTY_MONITORS` defines + ALL typedefs/enums/unions + public prototypes) and a source buffer (own-header include + impl std-includes + the `-Wunused-function` pragma + every static helper and function body), routed by a `Piece::{Header,Source}` enum; each piece still lands in the legacy single `code` buffer (the byte-identity anchor for `generate_core`). `generate_production` assembles BOTH slots: header = line-1 + `#ifndef <MANGLED>_FSM_H`/`#define`/body/`#endif`, source = line-1 + `#include "<resolved header name>"` + body. Removed the dead `Piece::{Both,CodeOnly}` variants and the unused `emit_header_only` helper left by the groundwork (2 rustc dead-code warnings). |
| `src/codegen/mod.rs` | changed | `BackendOptions` gains `fsm_header_name: Option<String>` / `fsm_source_name: Option<String>` (the resolved C fsm reference names; None = standalone default `rfs_fsm.h`). Default impl sets both to None. |
| `src/cli/commands/codegen.rs` | changed | Stamps the resolved C fsm-header/source names into `BackendOptions` for C production only (explicit `--out-fsm-*` flag > BASE-derived default); every other backend/command leaves them None. |
| `src/cli/commands/test.rs` | changed | The fuzz pipeline's `FuzzContext` carries the production header + resolved header name; the C fuzz run writes the header next to the source so its self-include resolves, and the `--print-script` output gains a third heredoc (the header) with renumbered build/run/clean steps. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-2 complete, tasks 3-11 pending. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/c_sync_handler/Makefile` | changed | Generation now emits the header/source PAIR (`--out-fsm-header build/arq_fsm.h --out-fsm-code build/arq_fsm.c`); the link rule adds `-I.` so the source's self-include of `build/arq_fsm.h` resolves from the example dir. handler.c still includes the whole `arq_fsm.c` (the deeper `.o`-linking restructure is task 9). |
| `crates/examples/fuzz_c/Makefile` | changed | Same pair generation for `rfs_generated_fsm.{h,c}`; the clang build adds `-I.`; the `test` invocation now references the explicit generated pair via `--fsm-file-header/--fsm-file-source` (so it never does a CWD BASE-derived take-or-emit); `clean` defensively removes any stray BASE-derived fsm file. |

## Changed functionality
- C `production` codegen: the single self-contained `.c` is now an idiomatic header/source pair. The header carries the include guard, its own line-1 artifact header, the type std-includes (`stdint/stdbool/string`), the `RFS_SPEC_VERSION`/`RFS_PROPERTY_MONITORS` defines, ALL type definitions and the public API prototypes; the source includes its own header (by the resolved name) and carries its own line-1 header, the impl std-includes (`stdio/stdlib`), the `-Wunused-function` pragma, and every static helper + function body.
- The `rfs test --backend c --suite fuzz` pipeline (and its `--print-script` output) now also emits the production header: the script writes three heredocs (skeleton, source, header) and cleans all three up.

## Removed functionality
- The dead `Piece::{Both,CodeOnly}` enum variants and the unused `emit_header_only` method in the C generator (groundwork scaffolding never wired to a call site).

## New functionality
- `BackendOptions.fsm_header_name` / `fsm_source_name`: the resolved C fsm reference names as data inputs to the generator, so an explicitly named header produces a source that includes the CORRECT name (not a CLI-side string rewrite).
- The C production header/source split with a mangled, uppercased include-guard macro derived from the module name.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/codegen_test.rs::c_production_emits_header_source_pair` | new | Pins the pair: both slots filled; include guard (`#ifndef TCP_HANDSHAKE_LIKE_FSM_H`/`#define`/`#endif`); both line-1 headers; source includes its own header; no `static` in the header, no `typedef` in the source; header compiles ALONE (self-containedness) and source compiles with `-I`, both warning-free under `gcc -Wall -c`. |
| `tests/codegen_test.rs::all_c_commands_generate_for_valid_examples` | changed | C production leg re-pointed: asserts BOTH slots filled; `fsm_step` checked against the source slot, `#include <stdint.h>` against the header slot. |
| `tests/codegen_test.rs::c_production_flag_toggles_property_monitors` | changed | Re-pointed at the split: the gating define + violation TYPE land in the header; the `#ifdef` gate + monitor state in the source; off-mode asserts BOTH files are clean; the stable `rfs_status_t` ABI checked in the header. |
| `tests/codegen_test.rs::c_production_matches_rust_semantics_on_endpoint_protocol` | changed | The input enum variant + version define re-pointed to the header slot; the on_error fallback stays in the source. |
| `tests/codegen_test.rs::{c_test_emits_fuzz_skeleton_as_extra_file, c_fuzz_target_frees_per_input_and_bounded_run_stays_green}` | changed | Write BOTH the header (`rfs_fsm.h`) and the source, compile/build with `-I` so the source's self-include resolves. |
| `tests/info_format_test.rs::artifact_header_generated_output_matches_documented_format` | changed | C production now fills TWO slots (header + source); asserts the header carries its OWN line-1 stamp; slot-count expectation updated (C production = 2). |
| `tests/common/prod_driver.rs::production_artifacts` | changed | Writes the C production header (`rfs_fsm.h`) into the per-tag dir alongside the source so every driver's `#include "<tag>_prod.c"` -> `#include "rfs_fsm.h"` chain resolves (fixes all prod_driver-based suites). |
| `tests/toolchain_fuzz_test.rs` | changed | The C production leg writes the header alongside the source in the scratch dir. |
| `tests/monitor_integration_test.rs::c_production_monitors_surface_property_violation_parity` | changed | Inlines the codegen to write both the source and the header (the driver includes `fsm.c` -> `rfs_fsm.h`). |
| `tests/wide_ops_parity_test.rs::wide_ops_production_drivers_byte_identical_and_pinned` | changed | Inlines the C production codegen to write the header alongside the concatenated source+driver file. |
| `tests/utils_test.rs::e2e_fuzz_print_script_pinned_and_no_execution` | changed | The pinned wrapper now expects THREE heredocs (skeleton, source, header) with renumbered steps 4/5/6 and the updated cleanup line; cross-checks the header heredoc against actual codegen (generated with the standalone default header name so the include line matches byte-for-byte). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, all 32 suites) | OK - 0 failed |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets` | OK - at the pre-phase baseline (12 lib warnings, zero new; removed 2 dead-code warnings + 1 `if_same_then_else` introduced mid-task) |

## Found problems/issues/mismatches
- The groundwork (prior session) had implemented Actions 2.1-2.2 but left dead scaffolding (`Piece::{Both,CodeOnly}`, `emit_header_only`) and had NOT done Action 2.3 (test pinning) - the C production legs still used `primary_code()` (now the source only), so 5 codegen_test legs + the info_format slot-shape test failed until re-pointed.
- The split makes the generated source NON-self-contained (it includes its own header). Every consumer that `#include`s the whole `.c` - both C example Makefiles, the shared `prod_driver`, and four standalone test legs - broke with "header not found" until each was made to also generate/write the header.
- A stale `43_sliding_window-fsm.h` left in the example dirs by an intermediate run tripped the `test` command's CWD BASE-derived take-or-emit partial-pair error; fuzz_c's `test` invocation now references the explicit generated pair instead.

## Other important findings/information
- The source includes its header by the RESOLVED name (full path when `--out DIR/base` or `--out-fsm-header DIR/h.h` is used). A quoted include carrying a directory component does NOT resolve from the nested file's own dir - it needs `-I.` (CWD) or the files to share a directory. This is why the example Makefiles and the standalone test legs add `-I.` or co-locate header+source.
- The `--print-script` shape change (third header heredoc + renumbered steps) landed in task 2 via the fuzz-pipeline groundwork, not task 4 as the plan assumed - see the Plan corrections entry.
