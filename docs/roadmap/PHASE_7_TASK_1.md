# Phase 7, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Introduce the internal/external renderer split for `rfs diagram` (D-P7-1/D-P7-2):
a new `--renderer internal|external` flag (default `internal`) and an in-process
Mermaid SVG path, so that `rfs diagram --format mmd --render svg` works with zero
external binaries. The existing mmdc-based path is preserved byte-for-byte as
`--renderer external`. This task proves the whole mechanism end-to-end before
PlantUML (task 2) and DOT (task 3) follow the same shape.

## Context

- Phase 7 plan: `PHASE_7_PLAN.md`, planned-tasks table rows 1, 1.1-1.5; decisions
  D-P7-1 (internal default), D-P7-2 (renderer split on `DiagramPlugin`),
  D-P7-4 (in-process crates, SVG-only features), D-P7-5 (`--print-script`
  internal variant).
- Before this task, `rfs diagram --format mmd` always shelled out to `mmdc`
  (node/Chrome) - the only external-tool dependency in the frontend.
- `mermaid-rs-renderer` was already a dev-dependency; dev-deps do not link into
  the binary, so a production renderer must be a regular dependency.
- Existing mmd e2e tests in `tests/frontend_test.rs` implicitly pinned the
  external behaviour and had to be switched to explicit `--renderer external`.

## Scope

### In scope
- `Renderer { Internal, External }` option + CLI flag + default (internal).
- `DiagramPlugin::internal_render_svg` / `internal_renderer_name` hooks.
- In-process Mermaid generator (`src/diagrams/mermaid.rs`) rendering the emitted
  stateDiagram-v2 subset via `render_strict`.
- Test migration + new internal-mode tests; BUILD.md / TOOLCHAIN.md updates.

### Out of scope
- PlantUML/DOT in-process generators (tasks 2-3), cross-renderer parity suite
  (task 4), format documentation entries (tasks 5-14).

## Planned actions

1. **Promote mermaid-rs-renderer to a regular dependency** (`Cargo.toml`,
   `Cargo.lock`): move from `[dev-dependencies]` to `[dependencies]` with
   `default-features = false` (SVG only, D-P7-4); the docs gate keeps compiling
   (`render_strict` is core API).
2. **Internal-render hook + `--renderer` flag** (`src/diagram.rs`, `src/main.rs`):
   add `internal_render_svg: Option<fn(&str) -> Result<String, Vec<String>>>` to
   `DiagramPlugin` (D-P7-2); add the `renderer: internal(default) | external`
   option (D-P7-1). Internal path in `run_diagram`: write source next to `--out`,
   render in-process, write SVG. External path stays byte-for-byte today's code.
   `--print-script` internal variant per D-P7-5: heredoc writes the source, then
   a comment step names the in-process crate - no tool invocation. Usage text
   updated.
3. **Mermaid internal renderer** (`src/diagrams/mermaid.rs`): `render_strict`
   over the emitted `stateDiagram-v2` subset (strict diagnostics double as
   negative-control material). Verify multi-line edge labels parse; if a subset
   element is rejected, adjust emission minimally and record in plan corrections.
4. **Tests** (`tests/frontend_test.rs`): existing real-tool mmd test switched to
   `--renderer external`; new: internal e2e pin (SVG contains event/guard/emits
   labels), 50-example internal sweep, unresolvable-`RFS_MERMAID` still exit 0 +
   SVG written (zero-external-dep proof), library-level negative control
   (corrupted mermaid text rejected with diagnostics).
5. **Docs** (`BUILD.md`, `TOOLCHAIN.md`): section 3 diagram row: `--renderer`
   flag, internal default; section 6: `mmdc` now optional (parity checks only).

#### Affected files

| File | Operation |
|---|---|
| `Cargo.toml` / `Cargo.lock` | changed - mermaid-rs-renderer dev-dependency -> regular dependency, `default-features = false` (SVG only) |
| `src/diagram.rs` | changed - `Renderer` option; `DiagramPlugin` gains `internal_render_svg` + `internal_renderer_name` |
| `src/main.rs` | changed - `--renderer internal\|external` flag wiring (default internal) |
| `src/diagrams/mermaid.rs` | changed - in-process Mermaid generator + `render_strict` SVG path |
| `tests/frontend_test.rs` | changed - existing mmd e2e tests -> `--renderer external`; 5 new internal-mode tests |
| `BUILD.md` | changed - diagram row (internal default), baseline re-pin |
| `TOOLCHAIN.md` | changed - section 6: renderer split, mmdc now optional |

## Task deliverables

- `rfs diagram --format mmd --render svg` works with zero external binaries
  (default internal).
- `--renderer external` reproduces the pre-Phase 7 mmdc behaviour byte-for-byte
  (strict errors, no fallback - CI/CD requirement).
- BUILD.md / TOOLCHAIN.md describe the split; full test matrix green.

## Task exit criteria

- [x] `--renderer internal|external` flag exists, default internal, unknown value rejected (DONE: `e2e_diagram_renderer_flag_rejects_unknown_value`)
- [x] In-process mmd SVG rendering works for all valid examples (DONE: `diagram_internal_mmd_sweep_valid_examples`, 50-example sweep)
- [x] External path unchanged byte-for-byte (DONE: existing e2e pins re-run with explicit `--renderer external`)
- [x] `--print-script` internal variant emits heredoc + crate comment, no tool line (DONE: `e2e_diagram_print_script_mmd_internal_pinned`)
- [x] Negative control: garbage Mermaid text rejected in-process with diagnostics (DONE: `diagram_mmd_internal_negative_control_rejects_garbage`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 458 / workspace 482, frontend_test 106)

## Non-goals

- No changes to the emitted stateDiagram-v2 text itself.
- No PlantUML/DOT in-process paths yet (tasks 2-3).
- No visual-output comparison anywhere (D-P7-10: syntax correctness only).

## Task corrections

- **1.4** (2026-10-26): switching the dot/puml real-tool + stub + print-script
  tests to explicit `--renderer external` was pulled FORWARD into task 1 (all at
  once) instead of per-renderer tasks 2-3 as originally sketched - with
  `internal` as the default from task 1 on, every test invoking an external
  renderer breaks immediately; the suite must stay green after each task. The
  two negative-control tests call the external fns directly and needed no change.
- **1.3/1.4** (2026-10-26): the negative control targets a missing diagram
  HEADER ("this is not mermaid at all"), not a malformed stateDiagram body -
  verified empirically: mermaid-rs-renderer (like mmdc) ACCEPTS dangling arrows
  and unclosed composites; only the header level rejects strictly. Parity tests
  (task 4) compare parse-success, not diagnostics text.

## Other information

- The internal/external split lives in the private `SvgMode` enum in
  `src/diagram.rs`; tasks 2-3 only needed to fill in their plugin's
  `internal_render_svg` / `internal_renderer_name` fields - no further CLI or
  dispatch changes.
- `e2e_diagram_list_pinned` pins the mmd description mentioning in-process
  rendering; tasks 2-3 changed the dot/puml descriptions and updated that pin
  again.
- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (the original per-task file was not written under the protocol name;
  work notes existed as `docs/roadmap/tasks/task_1.md`, since removed). Sources:
  commit `696339c` message, TASKS.md entry for task 1, `PHASE_7_PLAN.md`
  corrections table, BUILD.md baseline re-pins.
- Committed bundled with tasks 2-4 in `696339c` (single commit for the four
  diagram-rendering tasks; see `PHASE_7_TASK_4_STATUS.md`).
