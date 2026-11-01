# Phase 7, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` / `Cargo.lock` | changed | `mermaid-rs-renderer` moved from `[dev-dependencies]` to `[dependencies]` with `default-features = false` (SVG only, D-P7-4) |
| `src/diagram.rs` | changed | `Renderer { Internal, External }` option; `DiagramPlugin` gains `internal_render_svg` + `internal_renderer_name` (D-P7-2) |
| `src/main.rs` | changed | `--renderer internal\|external` flag (default internal); unknown value rejected with a clear error |
| `src/diagrams/mermaid.rs` | changed | in-process Mermaid generator: `render_strict` over the emitted stateDiagram-v2 subset; strict diagnostics double as negative-control material |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 diagram row: `--renderer` flag, internal default; baseline re-pin (root 458 / workspace 482) |
| `TOOLCHAIN.md` | changed | section 6: renderer split documented; `mmdc` now optional (parity checks only) |

## Other file changes

None.

## Changed functionality

- `rfs diagram --format mmd` now renders SVG IN-PROCESS by default (zero
  external binaries); the mmdc path is preserved byte-for-byte behind
  `--renderer external` (strict errors, no fallback - CI/CD requirement).
- `--print-script` in internal mode: heredoc writes the source, then a comment
  step names the in-process crate - no tool invocation (D-P7-5).

## Removed functionality

None (external behaviour fully preserved; append-only CLI).

## New functionality

- `--renderer internal|external` flag on `rfs diagram` (default internal).
- In-process Mermaid SVG rendering via `mermaid-rs-renderer::render_strict`
  (regular dependency, SVG-only features).

## Tests add/change/remove

- Added (5): `e2e_diagram_render_mmd_internal_default`,
  `diagram_internal_mmd_sweep_valid_examples` (50-example sweep),
  `diagram_mmd_internal_negative_control_rejects_garbage`,
  `e2e_diagram_print_script_mmd_internal_pinned`,
  `e2e_diagram_renderer_flag_rejects_unknown_value`.
- Changed: existing mmd real-tool e2e tests switched to explicit
  `--renderer external` (per correction 1.4, ALL dot/puml/mmd switches were done
  at once in this task).

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 458 pass / 0 fail (lib 238, frontend 106 incl. the 5 new tests, all other suites unchanged) |
| `cargo test --workspace` | OK - 482 pass / 0 fail |
| `tests/frontend_test.rs` | OK - 106 tests incl. the 5 new internal-mode tests |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- mermaid-rs-renderer (like mmdc) ACCEPTS dangling arrows and unclosed
  composites; only the missing-header level rejects strictly - the negative
  control was re-targeted to a missing diagram header (plan correction 1.3/1.4).
- With `internal` becoming the default, every existing external-renderer test
  broke immediately - the switch to explicit `--renderer external` had to be
  pulled forward into this task (plan correction 1.4).

## Other important findings/information

- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `696339c` message, TASKS.md entry for task 1, BUILD.md baseline re-pins.
- Committed bundled with tasks 2-4 in `696339c` (2026-10-26); the four
  diagram-rendering tasks ship as one commit by design of the execution session.
