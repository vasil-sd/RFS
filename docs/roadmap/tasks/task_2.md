# Task 2 - plantuml renderer (puml) via plantuml-core git dependency

## Subtasks

### 2.1 Add plantuml-core git dependency
- [x] Identify the latest commit of https://github.com/askidmobile/PlantUML_RUST and add `plantuml-core = { git = "...", rev = "<rev>" }` to `Cargo.toml` (D-P7-3; explicit `package` name if workspace resolution is ambiguous) - resolved unambiguously, no `package` rename needed
- [x] Record the pinned rev in this status section (see Status below)
- [x] `cargo build --release` green (first fetch needs network; offline rebuilds via the cargo git cache afterwards - git db present at `$CARGO_HOME/git/db/plantuml_rust-383032fde707cbba`)

### 2.2 PlantUML internal renderer
- [x] Inspect the plantuml-core API (render entry point, options, error type) from the fetched source - `plantuml_core::render(source: &str, opts: &RenderOptions) -> Result<String, String>`; errors carried as a single diagnostic string
- [x] `pub fn render_svg_internal(source: &str) -> Result<String, Vec<String>>` in `src/diagrams/plantuml.rs`
- [x] Wire into `PUML_PLUGIN.internal_render_svg` / `internal_renderer_name`; verify the emitted subset parses (note-on-link, dashed bracket edges for on_error, multi-line labels); plantuml-core REJECTED `note right on link` (position word) - emission adjusted to bare `note on link`, one line per output; recorded in plan corrections (2026-10-26, 2.2/2.3). Known crate gap: note text not drawn into the SVG (syntax accepted) - documented in-test and in TOOLCHAIN.md 6.5

### 2.3 Tests (`tests/frontend_test.rs`)
- [x] New: `e2e_diagram_render_puml_internal_default` - unresolvable `RFS_PLANTUML` + default renderer -> exit 0, source next to SVG, labels present in the SVG (guards / on_error forms; note text NOT asserted - plantuml-core crate gap, documented in-test)
- [x] New: in-process 50-example internal puml sweep
- [x] New: library-level negative control - corrupted PlantUML text rejected with non-empty diagnostics
- [x] New: `e2e_diagram_print_script_puml_internal_pinned` - byte-pinned heredoc + in-process comment naming plantuml-core, no plantuml line
- [x] Update `e2e_diagram_list_pinned` for the new puml description

### 2.4 Docs
- [x] `BUILD.md`: diagram row notes both in-process renderers (tasks 1-2); test counts re-pinned to the measured numbers after this task's tests
- [x] `TOOLCHAIN.md` section 6: intro now names mmd + puml as in-process; new 6.5 documents the plantuml-core git dependency (pinned rev, offline-rebuild behaviour, repoint procedure) and the note-on-link rendering gap

## Status

COMPLETE (2026-10-26).

**Pinned plantuml-core rev (D-P7-3):** `8066f6124a86ed5da59c6d7d570f0568891c378e`
(plantuml-core 0.2.0, latest commit of https://github.com/askidmobile/PlantUML_RUST
at implementation time; rev-pinned in Cargo.toml + Cargo.lock). Cargo git cache:
`$CARGO_HOME/git/db/plantuml_rust-383032fde707cbba` - offline rebuilds work.

Verification: `cargo build --release` green; full `cargo test --release` root suite
**462 pass / 0 fail** (frontend_test now 110 incl. the 4 new task-2 tests: internal
default e2e, 50-example sweep, negative control, print-script byte pin); `cargo test
--release --workspace` **486 pass / 0 fail**; `cargo clippy --release --all-targets`
zero warnings in every file this task touched (remaining warnings are the pre-task
baseline in tests/common/prod_driver.rs, tests/common/mc_json.rs, src/test_cmd.rs);
`cargo fmt --check` clean. External tools (dot/plantuml/mmdc) are NOT installed in
the current environment, so the availability-gated real-tool tests skip - the
internal path is exactly what runs and passes without them.

Notes for later tasks:
- Emitted puml source changed (`note right on link` -> `note on link`, one line per
  output) - any future byte pin of generated puml text must use the new form; real
  PlantUML accepts both, so the external path is unaffected.
- plantuml-core does not yet draw note-on-link text into the SVG (syntax accepted).
  If it gains that later, `e2e_diagram_render_puml_internal_default` can tighten its
  label assertions without a generator change.
- Task 3 (dot via layout-rs) follows the same shape: fill in `DOT_PLUGIN.internal_render_svg`
  / `internal_renderer_name` only - no further CLI or dispatch changes needed.
