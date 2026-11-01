# Phase 7, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Add the in-process DOT renderer via `layout-rs` (crates.io), same shape as tasks
1-2: `rfs diagram --format dot --render svg` works with zero external binaries by
default; the real-Graphviz path stays byte-for-byte behind `--renderer external`.
This completes the in-process rendering for all three diagram formats.

## Context

- Phase 7 plan: rows 3, 3.1-3.4; decisions D-P7-2/D-P7-4.
- Tasks 1-2 landed the renderer split + internal Mermaid/PlantUML paths and the
  one-time switch of ALL real-tool tests to `--renderer external` (correction
  1.4).
- Risk note from the plan: layout-rs has no documented one-shot dot->SVG
  function - follow the crate's own binary wiring.

## Scope

### In scope
- layout-rs dependency (crates.io, version-pinned by cargo lock).
- Internal DOT renderer in `src/diagrams/dot.rs`.
- Tests: new internal e2e pin, sweep, negative control, print-script pin.
- BUILD.md / TOOLCHAIN.md updates.

### Out of scope
- Parity suite (task 4), format documentation entries (tasks 5-14).

## Planned actions

1. **Add layout-rs dependency** (`Cargo.toml`, `Cargo.lock`): `layout-rs =
   "0.1"` (MSRV 1.65, no features needed; skip its optional `log`).
2. **Dot internal renderer** (`src/diagrams/dot.rs`): wire dot text -> SVG
   following the layout-rs binary's code path: `gv::DotParser` -> VisualGraph ->
   layout pass -> `SVGWriter`. Verify our emitted subset (`rankdir=LR`, node
   fillcolor/filled, edge labels with `[guard]` / `-> next`, dashed on_error
   edges) parses and lays out.
3. **Tests** (`tests/frontend_test.rs`): new internal e2e pin, 50-example sweep,
   unresolvable-`RFS_DOT` exit 0 + SVG, library-level negative control (corrupted
   dot text rejected).
4. **Docs** (`BUILD.md`, `TOOLCHAIN.md`): section 3 row updated; section 6:
   `dot` now optional.

#### Affected files

| File | Operation |
|---|---|
| `Cargo.toml` / `Cargo.lock` | changed - `layout-rs = "0.1"` (locked 0.1.3; optional `log` not enabled) |
| `src/diagrams/dot.rs` | changed - internal renderer: `gv::DotParser` -> GraphBuilder -> `VisualGraph::do_it` -> `backends::svg::SVGWriter` |
| `tests/frontend_test.rs` | changed - 4 new internal-mode tests; list pin updated |
| `BUILD.md` | changed - dot row (internal default), baseline re-pin |
| `TOOLCHAIN.md` | changed - section 6: `dot` now optional |

## Task deliverables

- `rfs diagram --format dot --render svg` works with zero external binaries.
- All three formats now render in-process by default (external tools optional).
- Full test matrix green.

## Task exit criteria

- [x] layout-rs dependency added, workspace resolves (DONE: locked 0.1.3; optional `log` not enabled)
- [x] In-process dot SVG rendering works for all valid examples (DONE: `diagram_internal_dot_sweep_valid_examples`, sweep over all collected valid examples, asserted >= 45 - 50 at the time)
- [x] External path unchanged byte-for-byte (DONE: real-tool test + byte-pinned print-script pins re-verified)
- [x] `--print-script` internal pin (DONE: `e2e_diagram_print_script_dot_internal_pinned`)
- [x] Negative control: corrupted dot text rejected in-process (DONE: `diagram_dot_internal_negative_control_rejects_garbage`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 466 / workspace 490, frontend_test 114)

## Non-goals

- No parity suite yet (task 4).
- No emission changes were needed - the emitted DOT subset is fully covered by
  the layout-rs builder.

## Task corrections

None recorded.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (work notes existed as `docs/roadmap/tasks/task_3.md`, since removed).
  Sources: commit `696339c` message, TASKS.md entry for task 3,
  `PHASE_7_PLAN.md` corrections table, BUILD.md baseline re-pins.
- Committed bundled with tasks 1, 2, 4 in `696339c`.
- External tools (dot 2.43.0, plantuml 1.2020.02, mmdc 12.0.0 bundling mermaid
  12.2.0) were installed in the execution session per TOOLCHAIN.md 6.1 so the
  real-tool (external) tests could run.
