# Task 1 - Internal rendering infrastructure + mermaid renderer (mmd)

## Subtasks

### 1.1 Promote mermaid-rs-renderer to a regular dependency
- [x] Move `mermaid-rs-renderer = { version = "0.3", default-features = false }` from `[dev-dependencies]` to `[dependencies]` in `Cargo.toml` (D-P7-4)
- [x] `cargo build --release` green; docs gate (`docs_md_test`) still compiles against the moved dependency

### 1.2 Internal-render hook + `--renderer` flag
- [x] Add `pub type InternalRenderSvgFn = fn(&str) -> Result<String, Vec<String>>` and field `internal_render_svg: Option<InternalRenderSvgFn>` (+ `internal_renderer_name: Option<&'static str>` for --print-script comments) to `DiagramPlugin` in `src/diagram.rs` (D-P7-2); set on all five plugin literals (mermaid filled in 1.3; dot/puml/table/md = None for now)
- [x] Add `pub enum Renderer { Internal, External }` + `renderer: Renderer` option (default `Internal`) to `DiagramOptions` (D-P7-1)
- [x] `run_diagram`: common `--out FILE.svg` preconditions first (both modes), then per-mode renderer availability via the private `SvgMode` enum; internal path writes source next to `--out`, renders in-process, writes the SVG; external path byte-for-byte today's code
- [x] `--print-script`: internal variant = heredoc + comment line naming the in-process crate, no tool invocation (D-P7-5); external variant unchanged except desc gains `--renderer external` when that mode is explicit
- [x] `src/main.rs`: parse `--renderer internal|external` in `cmd_diagram` + usage text update (flag line + diagram paragraph)

### 1.3 Mermaid internal renderer
- [x] `pub fn render_svg_internal(source: &str) -> Result<String, Vec<String>>` in `src/diagrams/mermaid.rs` via `mermaid_rs_renderer::render_strict`
- [x] Wire into `MMD_PLUGIN.internal_render_svg`; verified the emitted stateDiagram-v2 subset (multi-line edge labels with literal `\n`, `[on_error]` prefix, guards, emits) parses and renders - full PIN spec checked by hand (all labels present in the SVG `<tspan>` text)

### 1.4 Tests (`tests/frontend_test.rs`)
- [x] Switch ALL existing external-renderer tests to explicit `--renderer external`: the three real-tool render tests, both stub argv tests, flag-beats-env, missing-dot-tool, failing-plantuml, failing-mmdc, and the two render print-script pins (pulled forward from tasks 2-3 because the internal default would break them otherwise - recorded in plan corrections). The two negative-control tests call the external `render_svg` fns directly and needed no change
- [x] Update `e2e_diagram_table_md_have_no_renderer` for the internal-mode message (`has no internal SVG renderer`) + `e2e_diagram_list_pinned` for the new mmd description
- [x] New: `e2e_diagram_render_mmd_internal_default` - unresolvable `RFS_MERMAID` + default renderer -> exit 0, source written next to SVG, SVG contains labels (zero-external-dep proof)
- [x] New: in-process 50-example internal mmd sweep (`run_diagram` with `Renderer::Internal` over all of `examples/valid`)
- [x] New: library-level negative control - garbage mermaid text rejected by `render_svg_internal` with a "diagram header" diagnostic (mermaid's grammar is loose about malformed stateDiagram BODIES - dangling arrows / unclosed composites are accepted, same as mmdc; documented in the test)
- [x] New: `--print-script` internal pin (heredoc + in-process comment, no mmdc line)
- [x] New: `--renderer bogus` -> exit 1 with usage hint

### 1.5 Docs
- [x] `BUILD.md` section 3 diagram row: `--renderer internal|external`, internal default; test counts updated to the actual numbers (root 458 / workspace 482, both verified by full runs)
- [x] `TOOLCHAIN.md` section 6: `--render svg` renders in-process by default (pure Rust); dot/plantuml/mmdc now optional, used only by `--renderer external` and parity checks

## Status

COMPLETE (2026-10-26).

Verification: `cargo build --release` + full `cargo test --release` green (root 458 pass: lib 238, frontend 106 incl. the 5 new tests, all other suites unchanged); `cargo test --workspace` 482 pass; `cargo clippy --release --all-targets` - zero warnings in any touched file (pre-existing warnings remain in untouched files: tests/common/prod_driver.rs, tests/common/mc_json.rs, src/test_cmd.rs - out of scope for this task); `cargo fmt --check` clean.

Notes for later tasks:
- The internal/external split lives in the private `SvgMode` enum in `src/diagram.rs`; tasks 2-3 only need to fill in their plugin's `internal_render_svg` / `internal_renderer_name` fields - no further CLI or dispatch changes.
- mermaid-rs-renderer is lenient about malformed stateDiagram bodies (accepts dangling arrows, unclosed composites); strict rejection happens at the diagram-header level. Task 4's external-vs-internal parity tests should compare parse-success, not diagnostics text.
- `e2e_diagram_list_pinned` now pins the mmd description mentioning in-process rendering; tasks 2-3 will change the dot/puml descriptions and must update that pin again.
