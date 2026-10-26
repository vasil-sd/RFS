# Task 3 - dot renderer via layout-rs

## Subtasks

### 3.1 Add layout-rs dependency
- [x] `layout-rs = "0.1"` in `Cargo.toml` (resolves to 0.1.3, the latest 0.1.x; MSRV 1.65 - fine for toolchain 1.99); its only optional dependency `log` is NOT enabled
- [x] `cargo fetch` + `cargo build` green (registry crate; cached under `$CARGO_HOME/registry/src/*/layout-rs-0.1.3`, offline rebuilds work)

### 3.2 Dot internal renderer (`src/diagrams/dot.rs`)
- [x] Inspect the layout-rs API from the fetched source: no documented one-shot dot->SVG function - the code path is `gv::DotParser::new(src).process() -> Result<ast::Graph, String>` (strict parse; errors are diagnostic strings), then `gv::GraphBuilder::visit_graph` + `.get() -> VisualGraph`, then `VisualGraph::do_it(false, false, false, &mut svg)`, then `backends::svg::SVGWriter::finalize() -> String` (the upstream binary's wiring)
- [x] `pub fn render_svg_internal(source: &str) -> Result<String, Vec<String>>` in `src/diagrams/dot.rs`; parse errors surfaced as `Err(vec!["internal dot renderer (layout-rs): <diag>"])` - strict error semantics (D-P7-2)
- [x] Wire into `DOT_PLUGIN.internal_render_svg` / `internal_renderer_name` ("layout-rs 0.1.3"); description line updated to the tasks 1-2 style
- [x] Verified the emitted subset parses AND lays out: `rankdir=LR` (builder maps it to LeftToRight), `node [shape=box]` default, `style=filled` + `fillcolor="#e0e0e0"` initial state, edge labels with `[guard]` / `-> next`, dashed on_error edges (`stroke-dasharray="5,5"` in the SVG - 3/3 on the pin spec), self-loops (spec 04 `SynSent -> SynSent` renders). Quoted node ids lex as identifiers; no emission change needed

### 3.3 Tests (`tests/frontend_test.rs`)
- [x] Existing real-tool dot test already on explicit `--renderer external` (pulled forward in task 1) - now actually RUNS: `dot` 2.43.0 was installed this session per TOOLCHAIN.md 6.1
- [x] New: `e2e_diagram_render_dot_internal_default` - unresolvable `RFS_DOT` + default renderer -> exit 0, source next to SVG, `<svg`, node names (Idle/Active/Error) + event/guard edge labels (`Tick -&gt; Active`, `Cmd(Go(lvl)) [(lvl &gt; 2)] -&gt; Active`) + `on_error -&gt; Error` all present (layout-rs HTML-escapes `>` in tspan text)
- [x] New: `diagram_internal_dot_sweep_valid_examples` - all 45 valid examples render in-process via layout-rs
- [x] New: `diagram_dot_internal_negative_control_rejects_garbage` - library-level: corrupted DOT text ("this is not dot at all") rejected with non-empty diagnostics (`Expected (graph|digraph)`)
- [x] New: `e2e_diagram_print_script_dot_internal_pinned` - byte-pinned heredoc + in-process comment naming layout-rs 0.1.3, no `dot -Tsvg` line
- [x] Update `e2e_diagram_list_pinned` for the new dot description

### 3.4 Docs
- [x] `BUILD.md`: diagram row names all three in-process renderers (tasks 1-3); root **466 pass** / workspace **490 pass** re-pinned to the measured numbers; frontend CLI logic 110 -> 114
- [x] `TOOLCHAIN.md` section 6: intro now names mmd + puml + dot as in-process (zero-dependency command covers all three formats); new 6.5 bullet documents layout-rs (version, code path, subset coverage, parser-error handling)

## Status

COMPLETE (2026-10-26).

**Dependency:** `layout-rs = "0.1"` -> locked **0.1.3** (crates.io; registry cache at
`$CARGO_HOME/registry/src/*/layout-rs-0.1.3`; offline rebuilds work). No features
enabled (the optional `log` dep stays out).

Verification: `cargo build` green; full root suite **466 pass / 0 fail**
(frontend_test now 114 incl. the 4 new task-3 tests: internal default e2e with
unresolvable-tool proof, 45-example sweep, negative control, print-script byte
pin); `cargo test --workspace` **490 pass / 0 fail**; `cargo clippy --all-targets`
zero warnings in every file this task touched (remaining warnings are the pre-task
baseline in tests/common/prod_driver.rs, tests/common/mc_json.rs, src/test_cmd.rs);
`cargo fmt --check` clean. External tools re-installed this session per
TOOLCHAIN.md 6.1 (dot 2.43.0, plantuml 1.2020.02, node v18.19.1 / npm 9.2.0,
mmdc via @mermaid-js/mermaid-cli) - the availability-gated real-tool tests
(dot/puml/mmd) now run against the real binaries and pass.

Notes for later tasks:
- All three in-process renderers are in place (tasks 1-3). Task 4's parity suite
  can assume `--renderer internal` works for dot/puml/mmd without any external
tool; the external side needs dot/plantuml/mmdc on PATH (availability-gated).
- layout-rs renders labels into `<tspan>` elements with HTML escaping (`>` ->
  `&gt;`) - parity tests (task 4) must compare parse/render SUCCESS only, never
  bytes or label text across renderers (D-P7-10).
- The emitted DOT subset needed NO generator change for layout-rs (unlike task 2's
  puml note-on-link adjustment) - the builder covers everything `generate()` emits.
