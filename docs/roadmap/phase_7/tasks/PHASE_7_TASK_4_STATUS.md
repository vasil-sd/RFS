# Phase 7, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `tests/diagram_render_parity_test.rs` | 13-test cross-renderer syntax parity suite: internal sweep (all 50 valid examples x dot/puml/mmd), external pinned subset (specs 04/19/40/48 x real tools, availability-gated + mutex-serialized, source file asserted byte-identical to in-process text), unresolvable-tools e2e for all three formats, `--print-script` internal-mode structural pins, three library-level negative controls |

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3: diagram row names all three in-process renderers + new parity-suite row; counts re-pinned (root 479 / workspace 503); section 5 checklist note that the real-tool tests are availability-gated opt-in checks |
| `TOOLCHAIN.md` | changed | section 6: in-process default for all three formats, tools now optional; new 6.5 crate-dependency notes incl. the plantuml-core pinned rev and offline-rebuild behaviour |

## Other file changes

None.

## Changed functionality

None (test + docs only).

## Removed functionality

None.

## New functionality

- Dedicated cross-renderer syntax parity suite (`tests/diagram_render_parity_test.rs`),
  mirroring the `backend_parity_test.rs` naming pattern: proves internal and
  external renderers parse the SAME generated text (syntax correctness only,
  never visual output - D-P7-10).

## Tests add/change/remove

- Added (13, all in the new suite): `parity_internal_dot_all_valid_examples`,
  `parity_internal_puml_all_valid_examples`, `parity_internal_mmd_all_valid_examples`,
  `parity_external_dot_pinned_subset`, `parity_external_puml_pinned_subset`,
  `parity_external_mmd_pinned_subset`,
  `parity_unresolvable_tools_internal_all_formats_exit_zero`,
  `parity_print_script_internal_dot_pinned`,
  `parity_print_script_internal_puml_pinned`,
  `parity_print_script_internal_mmd_pinned`,
  `parity_negative_control_dot_rejects_garbage`,
  `parity_negative_control_puml_rejects_garbage`,
  `parity_negative_control_mmd_rejects_garbage`.
- Changed: none (frontend_test's per-renderer tests stay as-is).

## Tests run status

| Name | Status |
|---|---|
| `cargo build --release` | OK - exit 0 |
| `cargo test` (root) | OK - 479 pass / 0 fail (frontend_test 114 incl. per-renderer internal e2e pins with unresolvable-tool proofs, 50-example internal sweeps, negative controls) |
| `cargo test --workspace` | OK - 503 pass / 0 fail |
| `tests/diagram_render_parity_test.rs` | OK - 13/13, external half running against the real tools (dot 2.43.0, plantuml 1.2020.02, mmdc 12.0.0 bundling mermaid 12.2.0) |
| `cargo clippy --all-targets` | OK - zero warnings in every file this commit touched |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

None recorded.

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `696339c` message, TASKS.md entry for task 4, BUILD.md baseline re-pins.
- Committed bundled with tasks 1-3 in `696339c` (2026-10-26) - the four
  diagram-rendering tasks ship as one commit by design of the execution session;
  this is the last of the four, so the commit-level verification numbers are
  recorded here.
- Deliberately NOT committed: `docs/TODO.txt` (pre-existing user notes, not a
  phase artifact) and `docs/.#TODO.txt` (broken vim swap symlink).
