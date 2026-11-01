# Phase 7: Documentation and self-describing formats

## Goal

Make RFS documentation first-class, machine-accessible from the toolchain itself,
and free of external rendering dependencies in production environments:
`rfs diagram --render svg` switches to pure-Rust renderers (no `dot` / `plantuml`
/ `mmdc` required), and a new `rfs info format` subcommand family exposes
authoritative descriptions of every input/output format the toolchain speaks
(EBNF of the RFS language as markdown / PlantUML / railroad SVG; JSON Schema for
spec and counterexample-trace JSON) - generated from data embedded in the binary
so it stays in sync with the code. This phase is documentation-only: no changes
to parser, validator, codegen, or model-checking algorithms.

## Scope

- internal SVG rendering for `rfs diagram --render svg`:
  - mermaid (`mmd`) via the `mermaid-rs-renderer` crate (promoted from dev-dep to a regular dependency);
  - plantuml (`puml`) via the `plantuml-core` library from askidmobile/PlantUML_RUST (git dependency - not published on crates.io, rev-pinned);
  - dot via the `layout-rs` crate;
  - external tools (`dot`, `plantuml`, `mmdc`) remain available for parity/syntax cross-check tests and as an explicit opt-in renderer path (keep-vs-test-only decision made in the plan);
- cross-renderer syntax tests: diagram sources generated from the example corpus must parse WITHOUT SYNTAX ERRORS both in the internal Rust renderers and (when installed) in the external tools; comparison is syntax correctness only, never visual output;
- `rfs info format <name> [--output ...]` subcommand family (sibling of the existing spec-introspection mode): machine-readable format descriptions embedded in the binary - EBNF of the RFS input language (`spec-rfs`, outputs md / plantuml / svg via the `railroad` crate), JSON format descriptions with JSON Schema output (`spec-json`, `ce-trace`, plus the other formats enumerated in "Format inventory" below);
- documentation updates: `BUILD.md`, `TOOLCHAIN.md` (new dependencies/tools, per user requirement), `KB.md`, `DESIGN.md` (Phase 7 decisions section), the `docs/user/` set (+ `MIGRATION.md` note if the CLI surface changes);
- training-set update for the new commands (category-local additions, snippet + aggregate regeneration).

## Internal SVG rendering (item 1)

| Format | Crate | Entry API (verified 2026-10-26) | Notes |
|---|---|---|---|
| `mmd` | `mermaid-rs-renderer` 0.3.x | `render(&str) -> Result<String>` (SVG); supports `stateDiagram-v2` | already a dev-dependency (docs gate, pinned 0.3.1 in cache); promote to `[dependencies]` with `default-features = false`; must verify the emitted subset parses: `stateDiagram-v2`, `[*]` start state, `A --> B: label` with multi-line edge labels (literal two-char `\n`) |
| `puml` | `plantuml-core` 0.2.x - git dependency `https://github.com/askidmobile/PlantUML_RUST` | `render(source, &RenderOptions) -> Result<String>` (SVG); state diagrams among the 16 supported types | NOT on crates.io (roadmap item unchecked) => git dependency pinned by rev in Cargo.lock; MSRV 1.83 (toolchain is 1.99 - fine); must verify the emitted subset parses: `@startuml`/`@enduml`, `[*]`, `A --> B : label`, `note right on link` blocks, `- [dashed]->` edges; visual identity with Java PlantUML is explicitly OUT of scope |
| `dot` | `layout-rs` 0.1.x (crates.io) | `gv::DotParser` (DOT -> AST) + `topo` layout (`VisualGraph`) + `backends::svg::SVGWriter` | pure Rust; must verify the emitted subset parses and renders: `digraph`, `rankdir=LR`, node `fillcolor`, quoted labels with `\\`/`\"` escapes, dashed edges |

- `--render svg` no longer requires any external tool; strict error semantics are
  preserved (internal renderer parse/render failure = exit 1, no graceful
  degradation).
- External tools: `dot` 2.43.0, `plantuml` 1.2020.02, `mmdc` (mermaid 12.2.0)
  are ALREADY installed in this environment (`TOOLCHAIN.md` section 6) - no new
  installation is expected; tests that invoke them stay availability-gated
  (established pattern: skip when absent, never a hard failure) and mmdc calls
  remain serialized (two concurrent mmdc processes fail nondeterministically in
  this container); plantuml acceptance is gated on exit status (exit 200 =
  syntax error), not file presence.
- `BUILD.md` + `TOOLCHAIN.md` document the new dependencies and the fact that
  SVG rendering no longer needs external tools (explicit user requirement).

## rfs info format (item 2)

CLI shape (takes NO spec file - it describes the formats themselves):

```sh
rfs info format --list                                # all formats + supported outputs
rfs info format spec-rfs                              # EBNF as markdown (default output: md)
rfs info format spec-rfs --output plantuml            # @startebnf diagram source
rfs info format spec-rfs --output svg                 # railroad SVG (railroad crate)
rfs info format spec-json                             # IR JSON description (md)
rfs info format spec-json --output schema             # JSON Schema document
rfs info format ce-trace [--output md|schema]         # counterexample trace format
```

- Single source of truth inside the binary: an embedded machine-readable model
  - an EBNF rule tree for `spec-rfs` (mirrors `DESIGN.md` section 6, which stays
    the human-facing document; a test keeps them in sync),
  - key/field models for the JSON formats derived from the same data that drives
    `src/json_rw.rs` key ordering and the MC report writer (`JSON_KEY_*`
    constants), so descriptions cannot drift from actual output.
- Output renderers per format:
  - `md` (default everywhere): human-readable description (rule/field tables);
  - `plantuml`: PlantUML EBNF diagram source using the "All EBNF Elements"
    syntax (`=` definition, quoted terminals, `|` alternative, `[ ]` optional,
    `{ }` zero-or-more, `( )` grouping, `(* notes *)`, `? ... ?` special nodes
    for lexical classes like digits/letters);
  - `svg`: railroad diagram built with the `railroad` crate (0.3.x; its resvg
    re-export is already in the dependency tree via mermaid-rs-renderer);
  - `schema`: JSON Schema document on stdout (machine-parsed, for CI/CD and
    other automation pipelines).
- Rationale (user requirement): users get actual information about internals
  from the rfs binary itself - kept in sync with the code by tests - so it can
  be used (especially the JSON Schemas) without reading other documents.

### Format inventory (every input/output format of the rfs binary)

| # | Format name | Direction | What it is | Proposed command / outputs |
|---|---|---|---|---|
| 1 | `spec-rfs` | input | RFS source text (`.rfs`) | `rfs info format spec-rfs` - md (default), plantuml, svg (railroad) |
| 2 | `spec-json` | input + output | IR v1 JSON document (hand-rolled writer, ordered keys) | `rfs info format spec-json` - md (default), schema |
| 3 | `ce-trace` | output | MC counterexample trace: text CE lines and the JSON counterexample document (both backends + `mc-ref`) | `rfs info format ce-trace` - md (default), schema |
| 4 | `mc-report` | output | full MC JSON report wrapper (status, statistics, counterexample) | `rfs info format mc-report` - md (default), schema |
| 5 | `validate-json` | output | structured diagnostics of `rfs validate --json` (one object per file / directory array) | `rfs info format validate-json` - md (default), schema |
| 6 | `info-json` | output | introspection document of `rfs info --json` (stable key order, all keys present) | `rfs info format info-json` - md (default), schema |
| 7 | `diagram-dot` | output | the DOT subset rfs generates (node/edge attributes, label escaping) | `rfs info format diagram-dot` - md |
| 8 | `diagram-puml` | output | the PlantUML state-diagram subset rfs generates (notes on link, dashed edges) | `rfs info format diagram-puml` - md |
| 9 | `diagram-mmd` | output | the Mermaid `stateDiagram-v2` subset rfs generates (multi-line edge labels) | `rfs info format diagram-mmd` - md |
| 10 | `diagram-table` / `diagram-md` | output | transition tables (plain text / markdown) | self-describing plain text - no dedicated entry; documented in user docs (plan may add entries if deemed useful) |
| 11 | `artifact-header` | output | line-1 provenance header of every generated artifact (toolchain version + spec hash, D-P6-3) | candidate entry `rfs info format artifact-header` - md (plan decides) |
| 12 | `.rfs-validation.err` | output | plain-text sidecar error report next to failing sources | self-evident plain text - no entry |

### Format sync (machine-checked, per user rationale)

- `spec-rfs`: every entry of the parser's `KEYWORDS` constant appears as a
  terminal in the embedded grammar; the rule set is name-equivalent to
  `DESIGN.md` section 6; the file-driven example suite (50 valid parse /
  13 invalid rejected) stays the behavioral backstop.
- JSON formats (`spec-json`, `ce-trace`, `mc-report`, `validate-json`,
  `info-json`): schemas are generated from the same key-ordering model that
  drives the writers; tests verify that documents actually emitted (all 50
  valid examples converted to JSON; pinned MC runs; validate/info runs) carry
  exactly the documented keys - structural check, existing byte-pins untouched.

## Cross-renderer syntax tests

- For each of `dot` / `puml` / `mmd`: generate diagram sources for the example
  corpus (all 50 valid examples or a pinned representative subset - plan decides)
  and assert BOTH: (a) the internal Rust renderer parses + renders to SVG
  without error (always run, hard requirement); (b) the external tool accepts
  the text without syntax error (`dot` exit 0; `plantuml` exit 0 and not 200;
  `mmdc` exit 0) - availability-gated, mmdc serialized.
- Negative controls: corrupted diagram text is rejected by the internal
  renderers (parse error -> `rfs` exit 1), so the switch cannot silently accept
  garbage.

## Deliverables

- `rfs diagram --render svg` with zero external-tool dependency (pure-Rust
  renderers for mmd/puml/dot) + the external-renderer path per the plan decision.
- New dependencies declared and pinned: `mermaid-rs-renderer` (regular dep),
  `plantuml-core` (git, rev-pinned), `layout-rs`, `railroad`.
- Cross-renderer syntax tests (internal vs external, availability-gated) +
  negative controls.
- `rfs info format` subcommand family (`--list`, per-format `--output`) with the
  embedded machine-readable grammar/format models and their sync tests.
- Documentation: `BUILD.md` (new commands, test expectations, dependencies),
  `TOOLCHAIN.md` (new crates/tools, git-dependency + offline notes), `KB.md`
  (source map + CLI behavior), `DESIGN.md` (Phase 7 decisions section),
  `docs/user/` updates (+ `MIGRATION.md` note if the CLI surface changes).
- Training-set additions (`frontend_cli` / `project_tooling`) + regenerated
  snippets and aggregate.

## Exit criteria

- `rfs diagram --format mmd|puml|dot ... --render svg` succeeds with
  dot/plantuml/mmdc UNRESOLVABLE (test forces nonexistent tool paths -> exit 0
  + non-empty SVG written).
- All generated diagram sources parse without syntax errors in the internal
  renderers; when the tools are installed they also pass the external tools
  (syntax only); negative controls reject corrupted text.
- `rfs info format --list` enumerates all inventory formats with their supported
  outputs; every entry's default md output renders; `spec-rfs --output plantuml`
  emits valid `@startebnf` source (accepted by plantuml when installed) and
  `--output svg` emits a railroad SVG; `spec-json` / `ce-trace` `--output
  schema` emit JSON Schema documents covering every key the toolchain actually
  emits for those formats.
- EBNF sync tests pass (KEYWORDS coverage + DESIGN.md section 6 equivalence).
- Full verification green: `cargo build && cargo test && cargo clippy
  --all-targets && cargo fmt --check` (+ `--workspace`), `cargo
  test-generated-code`, docs gate.
- `BUILD.md` / `TOOLCHAIN.md` / `KB.md` / `DESIGN.md` / `docs/user/` updated and
  synchronized; `TASKS.md` entry appended.

## Non-goals

- NO changes to parser, validator, codegen, or model-checking algorithms or core
  functionality (documentation-only phase by user requirement).
- Visual/pixel parity between internal renderers and external tools - syntax
  correctness only (user requirement); official-tool rendering quality is not
  matched.
- PNG output from `rfs diagram --render` (SVG only; the renderers support PNG
  but that is out of scope).
- No new RFS language features, no LSP.
- No changes to MC report semantics or status taxonomy (`ce-trace` / `mc-report`
  DOCUMENT the existing formats; they do not change them).
- Pure-text diagram generation (primary behavior) is unchanged and not
  deprecated.
