# Phase 5, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.
>
> **Extended 2026-10-21 (user requirement)**: plugin architecture for diagram
> formats + Graphviz DOT / PlantUML / Mermaid plugins, SVG rendering via a CLI
> flag, unified external-tool parameterization, BUILD.md/TOOLCHAIN.md docs.

## Goal
Add the `rfs diagram` command built on a **plugin registry** (mirroring the
codegen backend pattern): every output format is a registered plugin -
`dot` (Graphviz), `puml` (PlantUML) and `mmd` (Mermaid) state diagrams, plus
`table`/`md` transition tables and variable/event summaries. PRIMARY behaviour
is pure-Rust text generation with **no external tool calls**; the flag
`--render svg` additionally invokes the format's external tool (`dot` /
`plantuml` / `mmdc`) to produce an SVG. External-tool parameterization is
unified with task 3's resolver: `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID` (+
`*_ARGS` env vars), overriding CLI flags, and `--print-script` mode.

## Context

### Plugin architecture (user requirement 1)
- Mirror `src/codegen.rs` exactly: a `DiagramPlugin` struct of static data +
  function pointers, a `const ALL_DIAGRAMS: &[&DiagramPlugin]` registry,
  `list_diagrams()` / `get_diagram(name)` lookups. One module per format under
  a new `src/diagrams/` dir (`mod.rs`, `dot.rs`, `plantuml.rs`, `mermaid.rs`,
  `tables.rs`) - same layout as `src/backends/`. Rationale (record in the
  module doc + DESIGN.md at task 15): adding a new diagram format in the
  future means adding one module + one registry line, no changes to other
  code.
- Suggested shape (adapt to house style):
  ```rust
  pub struct DiagramPlugin {
      pub name: &'static str,        // CLI: dot|puml|mmd|table|md
      pub description: &'static str, // for --list
      pub ext: &'static str,         // source-file extension: dot|puml|mmd|txt|md
      /// Pure-Rust text generation (deterministic, no I/O, no tools).
      pub generate: fn(&Module) -> String,
      /// External tool used by `--render svg` (None = format has no renderer;
      /// `table`/`md` are None). Preset from task 3's ext_tool module.
      pub svg_tool: Option<&'static ExtPreset>,
      /// Per-format render invocation (builds argv, handles format-specific
      /// extras like mmdc's puppeteer config file).
      pub render_svg: Option<fn(&ResolvedTool, &Path, &Path) -> Result<(), Vec<String>>>,
  }
  ```
- `src/diagram.rs` (new): registry + shared CLI plumbing (arg parsing,
  generate-to-stdout/file, `--render` orchestration, fallback warning,
  `--print-script` assembly). No format-specific logic lives here.

### Data source and determinism
- Input: parsed + validated IR `Module` (same path as `rfs info`). All
  generators use declaration order; every format's output for a pinned spec is
  byte-pinned in tests.
- Transition data per edge: event name (+ variant pattern where the event has
  variants), guard expression, emitted outputs (`emits Out(var = value)` from
  the transition's action), target state, and whether it is an `on_error`
  fallback edge.

### Environment facts (verified in THIS container, 2026-10-21 - do not re-derive)
- Ubuntu 24.04, running as **root**. Installed: graphviz `dot` 2.43.0,
  plantuml 1.2020.02 (OpenJDK 21), node v18.19.1, npm 9.2.0,
  `@mermaid-js/mermaid-cli` (mmdc) installed globally via npm; it bundles
  mermaid **12.2.0** (ESM-only). Install commands: `apt install graphviz
  plantuml`, `npm i -g @mermaid-js/mermaid-cli`.
- **PlantUML verified syntax** (all accepted by the installed version):
  - `@startuml` / `@enduml`; start/end states `[*]`; transition
    `A --> B : label`.
  - Multi-line labels via LITERAL `\n` two-char sequences in the file:
    `A --> B : ev1 \n guard(x)` renders as two lines. (User requirement: edge
    label = `event \n guard`.)
  - Notes on links: single-line `note right on link : text` AND multi-line
    `note right on link\n line1\n line2\nend note`, placed directly AFTER the
    transition they annotate. Both verified; emitted data goes here.
  - Dashed edges (for on_error): `A -[dashed]-> B : label` works; the form
    `A -dashed-> B` is INVALID (exit 200). Use the bracketed form.
  - Long state names: `state "long name" as alias`. RFS state names are IR
    identifiers, so plain names are used directly (document this assumption).
- **PlantUML failure signature**: syntax error -> exit code **200**, stderr
  `Error line N in file: <path>` + `Some diagram description contains errors`.
  GOTCHA: plantuml STILL WRITES AN SVG FILE on error (an error-rendering
  picture) - test gates must check the EXIT CODE (and stderr), never
  "file exists".
- **Mermaid verified syntax** (`stateDiagram-v2`, v12.2.0):
  - `[*] --> A`; transitions `A --> B: label` (NOTE: no space before `:` in
    the doc examples; both parse, but pin ONE style - use `A --> B: label`).
  - Multi-line labels via literal `\n` work and render as separate lines.
  - **NO notes on links** (state-only: `note right of X ... end note`).
    Decision: emitted data goes INTO the edge label as extra line(s), so a
    mermaid transition renders as
    `A --> B: ev\n guard(x)\n emits Out(v = 42)` - lines in order event, guard,
    then each emitted output. (PlantUML keeps the user-specified split: label
    = event+guard, note-on-link = emitted data.)
  - **NO dotted/dashed edges**: `A -.-> B` is a parse error. Decision:
    on_error fallbacks use a normal arrow with the FIRST label line prefixed
    `[on_error]`.
  - Comments `%% ...`; composite states `state X { ... }` (not needed for v1
    flat FSMs).
- **mmdc failure signature**: genuine parse error -> exit code **1**, stderr
  `Error: Parse error on line N:` (+ jison grammar dump), NO output file.
  GOTCHA: the stateDiagram grammar is LOOSE - some malformed lines (e.g. a
  bare `this is broken @@@`) are accepted and rendered as text. Positive tests
  must therefore ALSO assert expected strings inside the produced SVG, not
  just exit code; and a negative control (unclosed `state {` block) proves the
  harness catches real errors.
- **mmdc invocation constraints**: `-o` must be an `.svg`/`.png`/`.pdf`/`.md`
  path (NOT `/dev/null`). Running as root REQUIRES a puppeteer config file:
  `mmdc -i in.mmd -o out.svg -p pup.json` with
  `pup.json = {"args":["--no-sandbox","--disable-dev-shm-usage"]}`; without it
  Chrome fails to launch. Decision: the mermaid render function writes this
  JSON to a temp file automatically (args overridable via env var
  `RFS_MERMAID_PUPPETEER_ARGS`, default the two flags above); in
  `--print-script` mode the script emits the JSON write as a step.

### Per-format content mapping
- **dot** (plugin `dot`, ext `.dot`): digraph; one node per state (initial
  state marked, e.g. ` [style=filled]`), one edge per transition labeled
  `Event[pattern] [guard] -> next`; on_error edges `style=dashed`. Pure Rust,
  no dependency.
- **puml** (plugin `puml`, ext `.puml`): `@startuml`/`@enduml`; `[*] --> S0`
  for the initial state; per transition
  `S1 --> S2 : ev \n guard` (guard line omitted when absent) followed, when
  the transition emits outputs, by a multi-line `note right on link` block
  listing each `emits Out(var = value)`; on_error transitions use
  `S1 -[dashed]-> S2 : on_error(ev) \n guard`. Deterministic declaration order.
- **mmd** (plugin `mmd`, ext `.mmd`): `stateDiagram-v2`; `[*] --> S0`; per
  transition a single multi-line label: event, then guard (if any), then one
  line per emitted output; on_error transitions prefix the first label line
  with `[on_error]`.
- **table** (plugin `table`, ext `.txt`) and **md** (plugin `md`, ext `.md`):
  state x event-group matrix (cells = guard(s)/action summary or `-`; on_error
  rows/section marked) + typed variable/event summaries; `md` wraps the same
  data layer in Markdown. Shared builder in `tables.rs`. No renderer.

### Rendering, fallback, script mode (user requirement 5)
- `--render svg` requires `--out FILE` ending in `.svg`; the generated source
  is written NEXT TO it with the plugin's extension (same stem:
  `--format puml --out build/diagram.svg` -> writes `build/diagram.puml` and
  renders to `build/diagram.svg`). Both files persist (reviewable).
- Without `--render`: text goes to stdout or `--out FILE`; NO external tool is
  ever invoked (primary behaviour).
- Renderer tool missing at render time -> WARN on stderr, still write the
  source file, exit 0 (graceful degradation; documented optional dependency).
- `--print-script` (task 3's Script builder): for `--render`, a self-contained
  POSIX sh script - header (rfs version, spec, resolved tool paths/args),
  heredoc step writing the generated source, (mermaid only: step writing the
  puppeteer JSON), then the render invocation. Without `--render`: no-tool
  script (comment + echo of what would be emitted). Byte-pinned in tests.
- Tool resolution per task 3's ext_tool: FLAG > ENV > PATH precedence,
  env args first + CLI args appended, sh-quoted rendering. NEW presets to add
  to `src/ext_tool.rs`: `RFS_PLANTUML`/`RFS_PLANTUML_ARGS`,
  `RFS_MERMAID`/`RFS_MERMAID_ARGS` (path to `mmdc`); existing `RFS_DOT`/
  `RFS_DOT_ARGS` reused. Flags: `--plantuml PATH`, `--plantuml-args "..."`,
  `--mermaid PATH`, `--mermaid-args "..."` (+ existing `--dot`/`--dot-args`).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | `rfs info` command + shared CLI infrastructure | ext_tool resolver + presets (dot; plantuml/mermaid added here), cli_log, Script builder for --print-script, IR-walking precedent for summaries |

### Dependent tasks

(none directly; task 14 trains it, task 15 documents it - both must pick up
the plugin list and the new env vars)

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/diagram.rs` | new | DiagramPlugin API + registry (list/get), CLI arg parsing, generate/render orchestration, fallback warning, --print-script assembly |
| `src/diagrams/mod.rs` | new | plugin modules wiring (mirrors src/backends/mod.rs) |
| `src/diagrams/dot.rs` | new | Graphviz DOT generator + render_svg via resolved dot (-Tsvg) |
| `src/diagrams/plantuml.rs` | new | PlantUML state-diagram generator (label = event\n guard; note-on-link = emitted data; -[dashed]-> on_error) + render_svg via plantuml -tsvg |
| `src/diagrams/mermaid.rs` | new | Mermaid stateDiagram-v2 generator (all info in multi-line edge labels; [on_error] prefix) + render_svg via mmdc with auto temp puppeteer JSON |
| `src/diagrams/tables.rs` | new | shared table/summary data layer + `table` (txt) and `md` plugins (no renderer) |
| `src/ext_tool.rs` | change | add RFS_PLANTUML/RFS_MERMAID presets (+ *_ARGS); no behaviour change for existing presets |
| `src/main.rs` | change | dispatch for the new `diagram` subcommand + usage text |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/frontend_test.rs` | change | all diagram tests (convention: command e2e tests live here, as for `info`) - see Action 7 |

### Affected docs (user requirement 6)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change | section: installing dot/plantuml/mermaid (apt/npm commands, versions seen here), running each by hand on a generated file, RFS_* env vars + flags table row for diagram, --render/--print-script semantics |
| `TOOLCHAIN.md` | change | same install/run instructions from the toolchain perspective (optional system dependencies; root/container note: mmdc needs the puppeteer --no-sandbox config which rfs generates automatically) |

## Planned actions

### Action 1: plugin API + registry + table/md plugins
Define `DiagramPlugin` + registry in `src/diagram.rs`, `src/diagrams/mod.rs`;
implement the shared table/summary data layer and register `table` + `md`
plugins (no renderers). Pure text, no tools. Pin both outputs for a small
hand-written spec.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagram.rs` | new | plugin API + registry + CLI skeleton (generate path only) |
| `src/diagrams/mod.rs`, `src/diagrams/tables.rs` | new | table/md plugins; shared matrix builder (guards, actions, on_error, multi-variant events) |

### Action 2: dot plugin
DOT generator per the content mapping + `render_svg` via resolved `dot -Tsvg`
(reuses existing RFS_DOT preset). Pin full DOT output for the small spec.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagrams/dot.rs` | new | generator + renderer; pinned output test |

### Action 3: plantuml plugin
PUML generator per the content mapping (label = `event \n guard`, multi-line
`note right on link` for emitted data, `-[dashed]->` for on_error) +
`render_svg` via `plantuml -tsvg`. Pin output for the small spec; unit-test
the note/dash emission shapes.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagrams/plantuml.rs` | new | generator + renderer; pinned output test |

### Action 4: mermaid plugin
MMD generator per the content mapping (multi-line edge labels incl. emitted
data and `[on_error]` prefix) + `render_svg` via mmdc with auto temp
puppeteer JSON (`RFS_MERMAID_PUPPETEER_ARGS`). Pin output for the small spec.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagrams/mermaid.rs` | new | generator + renderer (puppeteer config handling); pinned output test |

### Action 5: ext_tool presets + render orchestration
Add RFS_PLANTUML/RFS_MERMAID presets to `src/ext_tool.rs`; wire `--render
svg` in `src/diagram.rs`: write source next to `--out *.svg`, invoke the
plugin's renderer via the resolved tool (capturing stderr for diagnostics),
missing tool -> warn + keep source file + exit 0. `--print-script` assembly
for all formats (self-contained script per Context).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/ext_tool.rs` | change | two new presets; existing unit tests unchanged/green |
| `src/diagram.rs` | change | render path, fallback, script emission |

### Action 6: command wiring
Dispatch in `src/main.rs`, options (`--format dot|puml|mmd|table|md` default
dot, `--out`, `--render svg`, `--list`, tool flags, `--log-level`,
`--print-script`), usage text; exit 1 on parse/validation error (same as
other commands).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/main.rs` | change | dispatch + usage text |

### Action 7: tests (tests/frontend_test.rs)
1. **Pinned generation**: byte-pinned output of ALL FIVE formats for one small
   hand-written spec (states, guards, multi-variant event, emitted outputs,
   on_error) - the pin IS the syntax contract.
2. **45-example sweep**: every valid example generates all five formats
   without error (text path only; no tools).
3. **Tool-gated render tests** (gate: resolved binary present in env - it IS
   present here; skip cleanly otherwise):
   - dot: real `dot` renders the pinned spec's DOT to SVG, exit 0, SVG
     contains state names.
   - plantuml: `plantuml -tsvg` on generated puml -> exit **0** + SVG contains
     event/guard/note strings. NEGATIVE CONTROL: known-bad puml -> exit
     **200** (proves the gate has teeth; documents the error-SVG gotcha).
   - mermaid: mmdc (+auto puppeteer config) on generated mmd -> exit **0** +
     SVG contains event/guard/emits strings. NEGATIVE CONTROL: unclosed
     `state {` block -> exit **1** + `Parse error` in stderr.
4. **Parameterization proof**: stub tools (as in task 3) for dot/plantuml/
   mmdc - argv incl. `--*-args` env+CLI values reaches the process;
   FLAG>ENV>PATH precedence spot-checked for one new tool.
5. **Fallback**: RFS_DOT pointed at a missing path + `--render svg` -> warning
   on stderr, source file written, exit 0.
6. **--print-script**: byte-pinned scripts - (a) puml+render (heredoc +
   plantuml line), (b) mmd+render (heredoc + puppeteer JSON step + mmdc
   line), (c) dot without render (no-tool script).
7. **CLI**: `--list` output pinned; unknown format rejected; `--render`
   without `--out *.svg` rejected with usage error; table/md + `--render`
   rejected.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/frontend_test.rs` | change | the tests above (e2e convention for commands) |

### Action 8: docs (BUILD.md + TOOLCHAIN.md)
Installation + manual-run instructions for dot/plantuml/mermaid per Scope's
docs table; env-var/flag reference rows; `--render`/`--print-script`
semantics; root/container puppeteer note. Keep both files consistent with the
existing external-tool documentation style.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change | install/run/env-var docs for diagram tools |
| `TOOLCHAIN.md` | change | toolchain-side install/run docs (optional deps, container note) |

## Task deliverables
- `rfs diagram` command on a plugin registry: 5 formats (`dot`, `puml`,
  `mmd`, `table`, `md`); adding a format = one module + one registry line.
- Primary behaviour = pure text generation, zero tool calls; `--render svg`
  invokes the format's tool (dot/plantuml/mmdc) with graceful fallback.
- Unified parameterization: `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID` +
  `*_ARGS`, overriding flags, `--print-script` (self-contained scripts).
- Tool-acceptance tests for generated puml/mmd (incl. negative controls) and
  pinned outputs for all formats; BUILD.md/TOOLCHAIN.md instructions.

## Task exit criteria
- [ ] All five formats byte-pinned for the small spec; 45-example sweep green (text path).
- [ ] Generated puml accepted by real plantuml (exit 0, expected strings in SVG); negative control fails with exit 200 through the same harness.
- [ ] Generated mmd accepted by real mmdc (exit 0, expected strings in SVG, auto puppeteer config works as root); negative control fails with exit 1 + `Parse error`.
- [ ] dot render path green with real dot; missing-tool fallback warns + keeps source + exit 0 (test).
- [ ] Stub-tool argv/args proof for all three tools incl. env+CLI precedence; --print-script outputs byte-pinned (puml, mmd-with-puppeteer-step, no-tool).
- [ ] BUILD.md + TOOLCHAIN.md contain install/run instructions for dot/plantuml/mermaid.
- [ ] Full suite green (`cargo test` root + `--workspace`), generated-code tests unchanged, clippy zero-new, fmt clean.

## Non-goals
- PNG output (v1 renders SVG only; `dot -Tpng` is a trivial later extension).
- Interactive/animated diagrams, layout engines in Rust.
- Composite/nested state rendering (RFS FSMs are flat; plugins stay simple).
- Diagrams of the MC-expanded state space including monitor states (MC
  internals stay out of user-facing diagrams).
- Mermaid/plantuml theming options beyond defaults (no skinparam/classDef in
  v1 output).

## Other information
- All tools ARE installed in this environment (versions in Context) - real-tool
  tests run here; keep the availability gate anyway for portability.
- PlantUML's error-SVG gotcha and mmdc's loose grammar are why the render
  tests assert on exit codes AND rendered text, with negative controls.
- `mmdc`'s `-o` rejects `/dev/null`; never use it in tests or scripts.
- The mermaid plugin writes a TEMP puppeteer JSON per invocation (cleaned up
  after); the script-mode variant emits it as an explicit step instead.
- Task 14 (training) and task 15 (docs sync/DESIGN section 23) must be updated
  to cover: the plugin registry, the three new env vars + flags, `--render`,
  and the format list - flag this in their status notes when they run.
