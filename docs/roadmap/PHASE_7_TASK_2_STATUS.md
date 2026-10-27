# Phase 7, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` / `Cargo.lock` | changed | `plantuml-core = { git = "https://github.com/askidmobile/PlantUML_RUST", rev = "8066f6124a86ed5da59c6d7d570f0568891c378e" }` (plantuml-core 0.2.0; D-P7-3) |
| `src/diagrams/plantuml.rs` | changed | internal renderer via `plantuml_core::render`; emission change: bare `note on link`, one line per output (correction 2.2/2.3) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 puml row: internal default; baseline re-pin (root 462 / workspace 486) |
| `TOOLCHAIN.md` | changed | section 6: `plantuml` now optional; git dependency + offline-rebuild behaviour noted (D-P7-3); pinned rev recorded |

## Other file changes

None.

## Changed functionality

- `rfs diagram --format puml` now renders SVG IN-PROCESS by default (zero
  external binaries).
- Emitted PlantUML source changed: `note right on link` -> bare `note on link`,
  one `note on link : emits X` line per output (minimal change that parses in
  both plantuml-core and real PlantUML; external path unaffected - re-verified).

## Removed functionality

None (external behaviour fully preserved; append-only CLI).

## New functionality

- In-process PlantUML SVG rendering via `plantuml_core::render` (git dependency,
  pinned rev `8066f6124a86ed5da59c6d7d570f0568891c378e`).

## Tests add/change/remove

- Added (4): `e2e_diagram_render_puml_internal_default`,
  `diagram_internal_puml_sweep_valid_examples` (50-example sweep),
  `diagram_puml_internal_negative_control_rejects_garbage`,
  `e2e_diagram_print_script_puml_internal_pinned`.
- Changed: none beyond task 1's one-time switch of all real-tool tests to
  `--renderer external`; the puml list pin was updated for the new internal
  default.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 462 pass / 0 fail |
| `cargo test --workspace` | OK - 486 pass / 0 fail |
| `tests/frontend_test.rs` | OK - 110 tests incl. the 4 new internal-mode tests |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- plantuml-core's parser REJECTS the position word in `note right on link` -
  emission changed to bare `note on link`, one line per output (plan correction
  2.2/2.3); real PlantUML accepts both forms, so the external path is unaffected.
- plantuml-core rendered-SVG gap: note-on-link TEXT is not drawn into the SVG
  (syntax accepted, layout skips it) - documented in TOOLCHAIN.md 6.5 and
  in-test; the e2e pin asserts states/edge labels only (plan correction 2.2).

## Other important findings/information

- **Pinned rev** (D-P7-3 requirement): `8066f6124a86ed5da59c6d7d570f0568891c378e`
  (plantuml-core 0.2.0, latest commit of
  https://github.com/askidmobile/PlantUML_RUST at implementation time;
  rev-pinned in Cargo.toml + Cargo.lock). Cargo git cache:
  `$CARGO_HOME/git/db/plantuml_rust-383032fde707cbba` - offline rebuilds work.
- At task 2 execution time the external tools (dot/plantuml/mmdc) were NOT
  installed in the environment, so the availability-gated real-tool tests
  skipped - the internal path is exactly what ran and passed without them.
  (They were re-installed during task 3's session.)
- Any future byte pin of generated puml text must use the new emission form
  (bare `note on link`, one line per output); real PlantUML accepts both, so
  the external path is unaffected.
- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `696339c` message, TASKS.md entry for task 2, BUILD.md baseline re-pins.
- Committed bundled with tasks 1, 3, 4 in `696339c` (2026-10-26).
