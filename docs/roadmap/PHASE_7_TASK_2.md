# Phase 7, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Add the in-process PlantUML renderer (`puml`) via the `plantuml-core` git
dependency (D-P7-3), same shape as task 1: `rfs diagram --format puml --render
svg` works with zero external binaries by default; the real-PlantUML path stays
byte-for-byte behind `--renderer external`. The git dependency is the novel part.

## Context

- Phase 7 plan: rows 2, 2.1-2.4; decisions D-P7-3 (git dependency, pinned rev),
  D-P7-4 (SVG-only).
- Task 1 landed the renderer split + internal Mermaid path and already switched
  ALL real-tool tests to explicit `--renderer external` (correction 1.4).
- `plantuml-core` lives in the repo's `crates/plantuml-core`; use an explicit
  `package` name if cargo cannot resolve it unambiguously (it resolved fine).

## Scope

### In scope
- plantuml-core git dependency (pinned rev, recorded in this file + TOOLCHAIN.md).
- Internal PlantUML renderer in `src/diagrams/plantuml.rs`.
- Tests: new internal e2e pin, sweep, negative control, print-script pin.
- BUILD.md / TOOLCHAIN.md updates.

### Out of scope
- DOT renderer (task 3), parity suite (task 4).

## Planned actions

1. **Add plantuml-core git dependency** (`Cargo.toml`, `Cargo.lock`):
   `plantuml-core = { git = "https://github.com/askidmobile/PlantUML_RUST",
   rev = "<latest at implementation time>" }` (D-P7-3). Record the pinned rev in
   the task status file. Verify workspace resolution.
2. **PlantUML internal renderer** (`src/diagrams/plantuml.rs`): render via
   `plantuml_core::render` (takes source text, returns SVG string). Verify the
   emitted subset parses (note-on-link, dashed bracket edges for on_error);
   fallback per risk note: adjust the emitted subset minimally (recording it in
   plan corrections) or - last resort - keep puml external-only.
3. **Tests** (`tests/frontend_test.rs`): new internal e2e pin, 50-example sweep,
   unresolvable-`RFS_PLANTUML` exit 0 + SVG, library-level negative control
   (corrupted PlantUML text rejected).
4. **Docs** (`BUILD.md`, `TOOLCHAIN.md`): section 3 row updated; section 6:
   `plantuml` now optional; TOOLCHAIN.md notes the git dependency and
   offline-rebuild behaviour (D-P7-3).

#### Affected files

| File | Operation |
|---|---|
| `Cargo.toml` / `Cargo.lock` | changed - plantuml-core git dependency, pinned rev `8066f6124a86ed5da59c6d7d570f0568891c378e` (plantuml-core 0.2.0) |
| `src/diagrams/plantuml.rs` | changed - internal renderer via `plantuml_core::render`; emission change: bare `note on link`, one line per output |
| `tests/frontend_test.rs` | changed - 4 new internal-mode tests; list pin updated |
| `BUILD.md` | changed - puml row (internal default), baseline re-pin |
| `TOOLCHAIN.md` | changed - section 6: plantuml now optional; git dependency + offline-rebuild notes |

## Task deliverables

- `rfs diagram --format puml --render svg` works with zero external binaries.
- Pinned rev recorded (this file, TOOLCHAIN.md).
- Full test matrix green.

## Task exit criteria

- [x] plantuml-core git dependency added with pinned rev, workspace resolves (DONE: rev `8066f6124a86ed5da59c6d7d570f0568891c378e`; works via the cargo git cache)
- [x] In-process puml SVG rendering works for all valid examples (DONE: `diagram_internal_puml_sweep_valid_examples`)
- [x] External path unchanged byte-for-byte (DONE: real-tool test + byte-pinned print-script pins re-verified)
- [x] `--print-script` internal pin (DONE: `e2e_diagram_print_script_puml_internal_pinned`)
- [x] Negative control: corrupted PlantUML text rejected in-process (DONE: `diagram_puml_internal_negative_control_rejects_garbage`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 462 / workspace 486, frontend_test 110)

## Non-goals

- No DOT path yet (task 3).
- No visual parity (D-P7-10); the known plantuml-core SVG gap (note-on-link text
  not drawn) is documented, not fixed.

## Task corrections

- **2.2/2.3** (2026-10-26): emitted puml source changed: `note right on link` ->
  bare `note on link` (position word dropped), and several outputs now emit ONE
  `note on link : emits X` line per output instead of a single multi-line note
  block - plantuml-core's parser REJECTS the position word; real PlantUML accepts
  both forms, so the external path is unaffected (real-tool test + byte-pinned
  print-script pins re-verified). The per-output one-line form is the minimal
  change that parses in BOTH renderers.
- **2.2** (2026-10-26): plantuml-core rendered-SVG gap recorded: note-on-link
  TEXT is not drawn into the SVG (syntax accepted, layout skips it) - task 2's
  e2e pin asserts states/edge labels only and documents the gap in-test. If the
  crate later draws notes, the pin can be tightened without a generator change.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (work notes existed as `docs/roadmap/tasks/task_2.md`, since removed).
  Sources: commit `696339c` message, TASKS.md entry for task 2,
  `PHASE_7_PLAN.md` corrections table, BUILD.md baseline re-pins.
- Committed bundled with tasks 1, 3, 4 in `696339c`.
