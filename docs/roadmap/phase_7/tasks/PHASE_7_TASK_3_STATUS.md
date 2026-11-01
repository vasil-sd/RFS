# Phase 7, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_7_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes to existing files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` / `Cargo.lock` | changed | `layout-rs = "0.1"` added (locked 0.1.3; its optional `log` dependency is not enabled) |
| `src/diagrams/dot.rs` | changed | internal renderer following the upstream binary's code path: `gv::DotParser` -> GraphBuilder -> `VisualGraph::do_it` -> `backends::svg::SVGWriter` (no documented one-shot dot->SVG function exists) |

## Added/changed/extended documentation files

| Name | Operation | Description |
|---|---|---|
| `BUILD.md` | changed | section 3 dot row: internal default; baseline re-pin (root 466 / workspace 490) |
| `TOOLCHAIN.md` | changed | section 6: `dot` now optional (parity checks only) |

## Other file changes

None.

## Changed functionality

- `rfs diagram --format dot` now renders SVG IN-PROCESS by default (zero
  external binaries).
- All three diagram formats (mmd/puml/dot) now render in-process by default;
  external tools are optional.

## Removed functionality

None (external behaviour fully preserved; append-only CLI).

## New functionality

- In-process DOT SVG rendering via layout-rs (crates.io, locked 0.1.3).

## Tests add/change/remove

- Added (4): `e2e_diagram_render_dot_internal_default`,
  `diagram_internal_dot_sweep_valid_examples` (sweep over all collected valid
  examples, asserted >= 45 - 50 at the time),
  `diagram_dot_internal_negative_control_rejects_garbage`,
  `e2e_diagram_print_script_dot_internal_pinned`.
- Changed: none beyond task 1's one-time switch of all real-tool tests to
  `--renderer external`; the dot list pin was updated for the new internal
  default.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root) | OK - 466 pass / 0 fail |
| `cargo test --workspace` | OK - 490 pass / 0 fail |
| `tests/frontend_test.rs` | OK - 114 tests incl. the 4 new internal-mode tests |
| `cargo clippy --all-targets` | OK - zero warnings in touched files (pre-existing warnings only elsewhere) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches

- layout-rs has no documented one-shot dot->SVG function - the renderer follows
  the upstream binary's code path instead (`gv::DotParser` -> GraphBuilder ->
  `VisualGraph::do_it` -> `backends::svg::SVGWriter`). No emission change was
  needed: the emitted subset (rankdir=LR, box nodes, style=filled + fillcolor
  initial state, edge labels, dashed on_error edges) is fully covered by the
  builder.

## Other important findings/information

- **Dependency:** `layout-rs = "0.1"` -> locked **0.1.3** (crates.io; registry
  cache at `$CARGO_HOME/registry/src/*/layout-rs-0.1.3`; offline rebuilds work).
  No features enabled (the optional `log` dep stays out).
- External tools re-installed this session per TOOLCHAIN.md 6.1: dot 2.43.0,
  plantuml 1.2020.02, node v18.19.1 / npm 9.2.0, mmdc via
  @mermaid-js/mermaid-cli - the availability-gated real-tool tests
  (dot/puml/mmd) ran against the real binaries and passed.
- layout-rs renders labels into `<tspan>` elements with HTML escaping (`>` ->
  `&gt;`) - parity tests (task 4) compare parse/render SUCCESS only, never
  bytes or label text across renderers (D-P7-10).
- This status file was reconstructed retroactively on 2026-10-27 (protocol
  violation: no per-task status file was written at execution time). Sources:
  commit `696339c` message, TASKS.md entry for task 3, BUILD.md baseline re-pins.
- Committed bundled with tasks 1, 2, 4 in `696339c` (2026-10-26).
