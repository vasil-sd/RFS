# Phase 7 - Implementation plan

> Written for future LLM sessions: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

Crate verification (checked 2026-10-26 against crates.io API and local cargo cache at
`/workspace/host/.cargo`):

- `mermaid-rs-renderer` **0.3.4** is already a dev-dependency (`default-features = false`,
  used by the docs gate `docs_md_test.rs` via `render_strict`). Moving it to regular
  dependencies is safe; `render_strict` is core API (not behind a feature). The `cli` /
  `png` features are optional extras we do not need.
- `plantuml-core` is **NOT on crates.io**. It is a workspace crate in
  https://github.com/askidmobile/PlantUML_RUST (`crates/plantuml-core`). Add as a git
  dependency, rev-pinned to the repo's latest commit at implementation time (record the
  rev in the task 2 status file). Cargo caches the checkout under `/workspace/host/.cargo/git`,
  so rebuilds work offline after the first fetch.
- `layout-rs` **0.1.0** exists (MSRV 1.65, MIT/Apache-2.0). It is a graphviz port:
  `gv::DotParser -> VisualGraph -> layout pass -> SVGWriter`. The crate's own binary
  (`dot.rs`) shows the wiring; there is no documented one-shot dot->SVG function, so task 3
  follows the binary's code path (risk noted below).
- `railroad` **0.3.11** exists (MSRV 1.85 - fine, toolchain is 1.99; edition-2024 crate).
  Use `default-features = false` (the default `resvg` feature only matters for PNG output;
  we emit SVG strings only).

Current code facts:

- `src/diagram.rs`: `DiagramPlugin { name, ext, render_svg: Option<fn(&Path)->Result<(),Vec<String>>>, generate }`;
  `run_diagram` generates source text, writes it next to `--out`, and when `render_svg` is
  requested calls the external renderer resolved through `crate::ext_tool` (strict errors,
  no fallback - user decision 2026-10-21). `--print-script` emits a POSIX script that runs
  the resolved tool.
- `src/diagrams/{mermaid.rs, plantuml.rs, dot.rs}`: pure text generators over IR v1;
  `table.rs` / `md.rs` have no SVG renderer (correct - they are table formats).
- `tests/frontend_test.rs` already exercises real tools (`dot`, `plantuml`, `mmdc`) with
  availability gating + a static mutex for serialization. These tests must be switched to
  the explicit external path (decision D-P7-1) so they keep testing what their names say.
- `src/main.rs`: `cmd_info(args)` at line ~436 handles the existing spec-introspection mode
  (`rfs info <spec> --json`). The new `format` mode is a sibling dispatch inside it.
- `src/ir.rs`: `KEYWORDS: &[&str]` (28 entries) is the single source of truth for reserved
  words; IR v1 JSON key order is fixed and pinned by tests.
- `src/mc_report_json.rs`: `JSON_KEY_*` constants are the single source of truth for the MC
  report / counterexample-trace JSON keys - the ce-trace and mc-report format models must be
  built against these constants, not string literals.

Decisions (recorded here; mirrored into DESIGN.md section 25 at closeout):

- **D-P7-1** `rfs diagram` gains `--renderer internal|external`, default **internal**.
  External mode keeps today's behaviour byte-for-byte (resolved tool, strict errors,
  `--print-script` with the tool). Rationale: production/CI needs zero external binaries by
  default; users who want official renderers opt in explicitly. This is a v1.1 CLI addition,
  not a break - documented in MIGRATION.md.
- **D-P7-2** `DiagramPlugin` gains `internal_render_svg: Option<fn(&str) -> Result<String, Vec<String>>>`
  (source text -> SVG string). `table` / `md` keep `None` ("no SVG renderer" error unchanged).
- **D-P7-3** plantuml-core is a rev-pinned git dependency (not on crates.io); first build
  needs network, offline rebuilds work via the cargo git cache. Recorded in TOOLCHAIN.md.
- **D-P7-4** mermaid-rs-renderer moves dev-dependency -> regular dependency,
  `default-features = false` (SVG only; no `resvg`/`png`/`cli`).
- **D-P7-5** `--print-script` in internal mode: heredoc writes the source file, then a
  comment line states which in-process crate rendered the SVG and its version - no tool
  invocation. External mode unchanged.
- **D-P7-6** `rfs info format [--list] <name?> [--output md|plantuml|svg|schema]` is a new
  sub-mode of the existing `info` command (dispatch on first arg == "format" in `cmd_info`).
  Registry pattern mirrors `DIAGRAMS` / `BACKENDS`: static `FORMATS: &[FormatInfo]` in a new
  module `src/info_format.rs`; each format task registers exactly one entry. A file literally
  named `format` cannot collide because dispatch matches the bare token before path handling.
- **D-P7-7** EBNF model: new module `src/ebnf_grammar.rs` with
  `enum RuleNode { Terminal(String), NonTerminal(String), Sequence(Vec<RuleNode>), Choice(Vec<RuleNode>), Optional(Box<RuleNode>), ZeroOrMore(Box<RuleNode>), OneOrMore(Box<RuleNode>) }`
  and `pub static GRAMMAR_RULES: &[(&str, RuleNode)]` mirroring DESIGN.md section 6 exactly
  (including the lexical rules `identifier`, `letter`, `digit`, `unsigned_integer_literal`).
  Renderers: `to_md()` (code block + rule table), `to_plantuml_ebnf()` (@startebnf/@endebnf,
  quoted terminals, `[0..n]` / `[1..n]` suffixes per plantuml.com/en/ebnf "All EBNF
  Elements"), and a railroad mapping for SVG (Sequence->Seq, Choice->Choice+Alt, Optional->Opt,
  ZeroOrMore->Repeat, OneOrMore->OneOrMore; `Diagram::new_with_stylesheet`, standalone SVG).
- **D-P7-8** JSON Schema: draft-07 (widest CI-validator support), emitted by a small shared
  emitter `src/json_schema.rs` from a compact field model
  `enum JsonDocField { Str, Int, Bool, Null, Obj(Vec<JsonDocField>), Arr(Box<JsonDocField>) }`
  with required/optional marking and enum-constraint notes. Schema output is pretty JSON
  produced through `json_rw` (project rule: all JSON goes through json_rw). The mc-report
  model `$ref`s the ce-trace model via `definitions`.
- **D-P7-9** Sync tests (the "stays in sync with the code" guarantee):
  - spec-rfs: every `KEYWORDS` entry appears as a terminal in GRAMMAR_RULES; rule names equal
    the names parsed out of the DESIGN.md section 6 ebnf fence.
  - spec-json / validate-json / info-json: walk every generated JSON document (all 50 valid
    examples via `rfs parse` output, all 13 invalid examples for validate-json) and verify
    (a) every required key in the model is present, (b) every key actually present at each
    object level is documented - catches drift in both directions without touching `ir.rs`.
  - ce-trace / mc-report: pinned FAIL-variant benchmark runs produce real reports; same walk
    check against models built from the `JSON_KEY_*` constants.
- **D-P7-10** Cross-renderer test corpus: internal half covers ALL 50 valid examples x 3
  formats (fast, pure Rust); external half covers a pinned representative subset (~4 specs
  exercising guards / emits / on_error / payloads) x 3 tools, availability-gated and
  serialized under a static mutex, exactly like the existing real-tool tests. Comparison is
  syntax correctness only - never visual output (user requirement).
- **D-P7-11** No new example `.rfs` files are needed: no language or algorithm changes in this
  phase, so the valid/invalid example sets stay untouched.
- **D-P7-12** Training set: category-local additions to `training/frontend_cli/rfs_training.jsonl`
  (diagram `--renderer` + `rfs info format` usage), then snippet + aggregate regeneration per
  protocol. No `.rfs` snippets added (no new language surface).
- **D-P7-13** table/md diagram formats and the `.err` artifact get no `info format` entries
  (self-describing plain text / standard error stream); they are covered by docs/user only.

Risks to re-verify at implementation time:

- plantuml-core is a young library: its subset verification may reject parts of our emitted
  PlantUML (note-on-link, dashed bracket edges). Fallback if needed: adjust the emitted
  subset minimally (recording it in plan corrections) or - last resort - keep puml
  external-only and record the decision. Same risk class for mermaid-rs-renderer with
  multi-line edge labels containing literal `\n`.
- layout-rs public API path from dot text to SVG is not a one-liner; follow the crate's own
  `dot.rs` binary wiring (parse -> build VisualGraph -> layout -> SVGWriter).
- railroad `default-features = false`: confirm SVG string emission does not require the
  `resvg` feature before committing the Cargo.toml change.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|-------------------------|
| 1. | Internal rendering infrastructure + mermaid renderer (`mmd`) | `Cargo.toml` (change), `Cargo.lock` (change), `src/diagram.rs` (change), `src/main.rs` (change), `src/diagrams/mermaid.rs` (change), `tests/frontend_test.rs` (change), `BUILD.md` (change), `TOOLCHAIN.md` (change) | First renderer end-to-end, proving the whole mechanism before the other two follow the same shape. |
| 1.1 | Promote mermaid-rs-renderer to a regular dependency | `Cargo.toml`, `Cargo.lock` | Move from `[dev-dependencies]` to `[dependencies]` with `default-features = false` (D-P7-4). Docs gate keeps compiling (`render_strict` is core API). Dev-deps do not link into the binary, so a production renderer must be a regular dependency. |
| 1.2 | Internal-render hook + `--renderer` flag | `src/diagram.rs`, `src/main.rs` | Add `internal_render_svg: Option<fn(&str) -> Result<String, Vec<String>>>` to `DiagramPlugin` (D-P7-2); add `renderer: internal(default) \| external` option (D-P7-1). Internal path in `run_diagram`: write source next to `--out`, render in-process, write SVG. External path stays byte-for-byte today's code. `--print-script` internal variant per D-P7-5. Usage text updated. |
| 1.3 | Mermaid internal renderer | `src/diagrams/mermaid.rs` | `render_strict` over the emitted `stateDiagram-v2` subset (strict diagnostics double as negative-control material). Verify multi-line edge labels parse; if a subset element is rejected, adjust emission minimally and record in plan corrections. |
| 1.4 | Tests | `tests/frontend_test.rs` | Existing real-tool mmd test switched to `--renderer external`; new: internal e2e pin (SVG contains event/guard/emits labels), 50-example internal sweep, unresolvable-`RFS_MERMAID` still exit 0 + SVG written (zero-external-dep proof), library-level negative control (corrupted mermaid text rejected with diagnostics). |
| 1.5 | Docs | `BUILD.md`, `TOOLCHAIN.md` | Section 3 diagram row: `--renderer` flag, internal default; section 6: `mmdc` now optional (parity checks only). |
| 2. | plantuml renderer (`puml`) via plantuml-core git dependency | `Cargo.toml` (change), `Cargo.lock` (change), `src/diagrams/plantuml.rs` (change), `tests/frontend_test.rs` (change), `BUILD.md` (change), `TOOLCHAIN.md` (change) | Same shape as task 1; the git dependency is the novel part. |
| 2.1 | Add plantuml-core git dependency | `Cargo.toml`, `Cargo.lock` | `plantuml-core = { git = "https://github.com/askidmobile/PlantUML_RUST", rev = "<latest at implementation time>" }` (D-P7-3). Record the pinned rev in the task status file. Verify workspace resolution (the crate lives in the repo's `crates/plantuml-core`; use an explicit `package` name if cargo cannot resolve it unambiguously). |
| 2.2 | PlantUML internal renderer | `src/diagrams/plantuml.rs` | Render via `plantuml_core::render` (API verified in context: takes source text, returns SVG string). Verify the emitted subset parses (note-on-link, dashed bracket edges for on_error); fallback per risk note above. |
| 2.3 | Tests | `tests/frontend_test.rs` | Existing real-tool puml test switched to `--renderer external`; new internal e2e pin, 50-example sweep, unresolvable-`RFS_PLANTUML` exit 0 + SVG, library-level negative control (corrupted PlantUML text rejected). |
| 2.4 | Docs | `BUILD.md`, `TOOLCHAIN.md` | Section 3 row updated; section 6: `plantuml` now optional; TOOLCHAIN.md notes the git dependency and offline-rebuild behaviour (D-P7-3). |
| 3. | dot renderer via layout-rs | `Cargo.toml` (change), `Cargo.lock` (change), `src/diagrams/dot.rs` (change), `tests/frontend_test.rs` (change), `BUILD.md` (change), `TOOLCHAIN.md` (change) | Same shape as tasks 1-2. |
| 3.1 | Add layout-rs dependency | `Cargo.toml`, `Cargo.lock` | `layout-rs = "0.1"` (MSRV 1.65, no features needed; skip its optional `log`). |
| 3.2 | Dot internal renderer | `src/diagrams/dot.rs` | Wire dot text -> SVG following the layout-rs binary's code path: `gv::DotParser` -> VisualGraph -> layout pass -> `SVGWriter` (see risk note). Verify our emitted subset (`rankdir=LR`, node fillcolor/filled, edge labels with `[guard]` / `-> next`, dashed on_error edges) parses and lays out. |
| 3.3 | Tests | `tests/frontend_test.rs` | Existing real-tool dot test switched to `--renderer external`; new internal e2e pin, 50-example sweep, unresolvable-`RFS_DOT` exit 0 + SVG, library-level negative control (corrupted dot text rejected). |
| 3.4 | Docs | `BUILD.md`, `TOOLCHAIN.md` | Section 3 row updated; section 6: `dot` now optional. |
| 4. | Cross-renderer syntax suite (internal vs external tools) | `tests/diagram_render_parity_test.rs` (new), `BUILD.md` (change) | The user-required tests against external tools, consolidated in a dedicated file mirroring the `backend_parity_test.rs` naming pattern. |
| 4.1 | Parity test matrix | `tests/diagram_render_parity_test.rs` | Per format x {internal: all 50 valid examples; external: pinned ~4-spec subset exercising guards/emits/on_error/payloads, availability-gated, serialized under a static mutex} - assert parse/render success only, never visual comparison (D-P7-10). Plus: unresolvable-tools e2e for all three formats in one place, `--print-script` internal-mode pin (heredoc + comment, no tool line), and the three library-level negative controls. |
| 4.2 | Docs | `BUILD.md` | Section 3 new row for the parity suite; section 5 checklist note that the real-tool tests are now availability-gated opt-in checks. |
| 5. | `rfs info format` infrastructure + spec-rfs (EBNF) | `src/info_format.rs` (new), `src/ebnf_grammar.rs` (new), `Cargo.toml` (change), `Cargo.lock` (change), `src/main.rs` (change), `tests/info_format_test.rs` (new), `BUILD.md` (change) | Registry + CLI plumbing + the flagship format (RFS language EBNF in md / plantuml / svg). |
| 5.1 | Format registry + CLI dispatch | `src/info_format.rs`, `src/main.rs` | `FormatInfo { name, summary, outputs: &[&str], render: fn(&str /*output*/, &mut String) }` + static `FORMATS`; `run_info_format`: `--list` (markdown table), name lookup with clear error on unknown name, `--output` validated against the entry's supported list (default md). `cmd_info` dispatches bare first arg `"format"` to it; usage text updated (D-P7-6). |
| 5.2 | EBNF grammar model + renderers | `src/ebnf_grammar.rs`, `Cargo.toml`, `Cargo.lock` | `RuleNode` + `GRAMMAR_RULES` mirroring DESIGN.md section 6 exactly, including lexical rules (D-P7-7). `to_md()`, `to_plantuml_ebnf()` per plantuml.com/en/ebnf "All EBNF Elements" (quoted terminals, `[0..n]`/`[1..n]`). Add `railroad = { version = "0.3", default-features = false }`; SVG via RuleNode->railroad mapping on a stylesheet diagram. |
| 5.3 | Tests | `tests/info_format_test.rs` | `--list` pin (all registered names); spec-rfs md pin (every rule present, KEYWORDS coverage test); plantuml pin (@startebnf/@endebnf, all rule names, quoted terminals); svg pin (`<svg`, every rule name as a label); DESIGN.md section 6 rule-name parity test (parse the ebnf fence, compare name sets). |
| 5.4 | Docs | `BUILD.md` | Section 3 row for `rfs info format`. |
| 6. | spec-json format entry (md + schema) | `src/info_format.rs` (change), `src/json_schema.rs` (new), `tests/info_format_test.rs` (change), `BUILD.md` (change) | Machine-readable description of the IR v1 JSON document (`rfs parse` / `--json` output). |
| 6.1 | Shared JSON-Schema emitter | `src/json_schema.rs` | `JsonDocField` model + draft-07 emitter (pretty JSON via json_rw, required/optional, enum notes) per D-P7-8; also the md table renderer shared by all JSON format entries. |
| 6.2 | spec-json field model | `src/info_format.rs` | Full IR v1 document model derived from `ir.rs` `to_json` at implementation time (top level: `ir_version`, `modules[]`; per module: `name`, `version`, `initial_state`, `enums`, `events`/`outputs` groups + variants + fields, `vars` name/type/init, `transitions` with every sub-field incl. guard/next/emits/sets/priority, `on_error`, `properties`/`assumptions`). Register the entry (outputs: md, schema). |
| 6.3 | Tests | `tests/info_format_test.rs` | Schema document pin (structure); walk check of every one of the 50 valid examples' parse JSON against the model - required keys present + no undocumented keys at any object level (D-P7-9). |
| 7. | ce-trace format entry (md + schema) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change), `BUILD.md` (change) | Counterexample trace: the JSON document embedded in MC reports plus the text trace format. |
| 7.1 | Field model | `src/info_format.rs` | Model built against `JSON_KEY_*` constants from `mc_report_json.rs`: `kind` enum (`trace` \| `lasso` \| `deadlock-config`), `initial_state` / `initial_vars` / `steps` (trace), `prefix` / `cycle` / `cycle_length` (lasso), `depth` / `state` / `vars` (deadlock); step shape: `input{variant,fields}`, `outputs[]`, `from_state`, `to_state`, `vars`; plus a md section describing the default text trace format (`step N` lines as printed by MC). Register entry (outputs: md, schema). |
| 7.2 | Tests | `tests/info_format_test.rs` | Pinned FAIL-variant benchmark runs (reuse the specs from `benchmark_mc_test.rs`) produce real reports on both backends; walk check against the model; pin. |
| 8. | mc-report format entry (md + schema) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change), `BUILD.md` (change) | Full MC report document. |
| 8.1 | Field model | `src/info_format.rs` | Model from `JSON_KEY_*`: `status` enum (`pass-complete` \| `fail` \| `unknown-state-limit`), `metrics{states,transitions,max_depth}`, fail fields (`reason`, `label`, `step`), `statistics{unexpected_terminations, monitor_size_bits, memory_estimate_bytes, peak_rss_kb}`, `counterexample` via `$ref` to the ce-trace model (D-P7-8). Register entry (outputs: md, schema). |
| 8.2 | Tests | `tests/info_format_test.rs` | All 6 benchmark specs x both backends (pass + fail variants) produce real reports; walk check against the model; pin. |
| 9. | validate-json format entry (md + schema) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change), `BUILD.md` (change) | `rfs validate --json` output. |
| 9.1 | Field model | `src/info_format.rs` | File mode: `{file, ok, errors:[{kind: parse\|semantic, line, col, message}]}`; directory mode: array of the same objects. Register entry (outputs: md, schema). |
| 9.2 | Tests | `tests/info_format_test.rs` | Run in-process over all 13 invalid examples (both error kinds must be observed) + valid files (`ok=true`); walk check; pin. |
| 10. | info-json format entry (md + schema) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change), `BUILD.md` (change) | `rfs info <spec> --json` output. |
| 10.1 | Field model | `src/info_format.rs` | Model from `ir.rs` `render_json`: `name`, `version`, `initial_state`, `enums`, `events`, `outputs`, `vars`, `transitions` (an INT count - document explicitly, it is not an array), `on_error`, `properties`, `assumptions` with `monitor_status`. Register entry (outputs: md, schema). |
| 10.2 | Tests | `tests/info_format_test.rs` | Run in-process over all 50 valid examples; walk check; pin. |
| 11. | diagram-dot format entry (md) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change) | Human-readable description of the generated DOT subset: digraph/rankdir, node attributes (fillcolor/filled/shape per state kind), edge label grammar (`Event [guard] -> next`, dashed on_error edges), string escaping rules. Sync test: assert the documented attribute/construct set covers everything `generate()` can emit for all 50 examples (no undocumented token appears in generated output). |
| 12. | diagram-puml format entry (md) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change) | Same pattern for the PlantUML subset: state declarations per kind, note-on-link forms, dashed bracket edges for on_error, `\n` label convention, escaping. Same coverage sync test against generated output. |
| 13. | diagram-mmd format entry (md) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change) | Same pattern for the mermaid subset: `stateDiagram-v2`, `[*]` edges, edge label line grammar incl. `[on_error]` prefix and multi-line labels, escaping. Same coverage sync test. Runs after tasks 1-3 since generator adjustments there could change the documented subset. |
| 14. | artifact-header format entry (md) | `src/info_format.rs` (change), `tests/info_format_test.rs` (change) | Line 1 of every generated artifact: exact stamp format derived from `src/codegen.rs` at implementation time (toolchain version + spec hash; record the precise syntax in the entry). Sync test: compare the documented format against actually generated artifacts for all 50 examples. |
| 15. | Training-set update | `training/frontend_cli/rfs_training.jsonl` (change), `all_rfs_training.jsonl` (change - regenerated), `training/*/snippets/*` (regenerated) | Category-local entries in the two-message format per KB.md: `rfs diagram --renderer internal\|external` usage incl. the zero-external-deps rationale, and `rfs info format` usage (`--list`, per-format `--output` choices, CI/CD use of JSON schemas). No `.rfs` snippets added (D-P7-11/D-P7-12); regenerate snippets + aggregate anyway per protocol; `cargo test-training` green. |
| 16. | Phase 7 closeout: docs sync + status | `DESIGN.md` (change), `KB.md` (change), `START.md` (change), `docs/user/README.md` (change), `docs/user/INTEGRATION.md` (change), `docs/user/MIGRATION.md` (change), `docs/roadmap/INTRO.md` (change), `docs/roadmap/PHASE_7_STATUS.md` (new), `TASKS.md` (change) | Full verification matrix from BUILD.md section 5 (build/test/clippy/fmt + workspace, test-generated-code, docs gate, benchmark suite). DESIGN.md new section 25 with D-P7-1..D-P7-13. KB.md sync: section 1 state, section 4.2 source map (+ `info_format.rs`, `ebnf_grammar.rs`, `json_schema.rs`), section 4.5 CLI behaviour (`diagram --renderer`, `info format`). docs/user: README (internal rendering in the pipeline picture), INTEGRATION.md line ~205 (tool-invoking commands no longer include diagram by default), MIGRATION.md v1.1 note (`--render svg` now renders in-process; `--renderer external` restores old behaviour). INTRO.md table row -> COMPLETE with summary; PHASE_7_STATUS.md per template 2.3 (incl. the plantuml-core pinned rev); TASKS.md entry; single commit. |

## Plan corrections (during execution)

| Date | Task | What changed | Why |
|------|------|--------------|-----|
| 2026-10-26 | 1.4 | Switching the dot/puml real-tool + stub + print-script tests to explicit `--renderer external` was pulled FORWARD into task 1 (all at once) instead of per-renderer tasks 2-3 as originally sketched | With `internal` as the default from task 1 on, every test invoking an external renderer breaks immediately - the suite must stay green after each task. The two negative-control tests call the external fns directly and needed no change |
| 2026-10-26 | 1.3/1.4 | Negative control targets a missing diagram HEADER ("this is not mermaid at all"), not a malformed stateDiagram body | Verified empirically: mermaid-rs-renderer (like mmdc) ACCEPTS dangling arrows and unclosed composites; only the header level rejects strictly. Parity tests (task 4) compare parse-success, not diagnostics text |
| 2026-10-26 | 2.2/2.3 | Emitted puml source changed: `note right on link` -> bare `note on link` (position word dropped), and several outputs now emit ONE `note on link : emits X` line per output instead of a single multi-line note block | plantuml-core's parser REJECTS the position word in `note right on link`; real PlantUML accepts both forms, so the external path is unaffected (real-tool test + byte-pinned print-script pins re-verified). The per-output one-line form is the minimal change that parses in BOTH renderers |
| 2026-10-26 | 2.2 | plantuml-core rendered-SVG gap recorded: note-on-link TEXT is not drawn into the SVG (syntax accepted, layout skips it) - task 2's e2e pin asserts states/edge labels only and documents the gap in-test | Syntax correctness is the Phase 7 requirement (visual parity explicitly out of scope); if the crate later draws notes, the pin can be tightened without a generator change |
| 2026-10-26 | 5.2 | DESIGN.md section 6 ebnf fence found STALE vs the actual parser: missing `version` in module, `on_error_decl`, `priority_action`, `output_call_payload`, the bitwise ladder levels, literal alternatives in `pattern_field`/`pattern_expr`, and the lexical rules (`letter`/`digit`/`unsigned_integer_literal`/`string_literal` were referenced but never defined); it still carried the old arithmetic-only placement. The fence was REPLACED with the canonical text of `to_ebnf_text()` (one rule per line, no in-fence comments - the two precedence notes moved to prose after the fence), and the sync test now enforces rule-name parity | Phase 7's whole rationale is docs staying in sync with the code; the plan assumed section 6 was accurate. The in-binary model (verified rule-by-rule against `parser.rs`) is the source of truth; DESIGN.md follows it |
| 2026-10-26 | 5.2 | The WIP `GRAMMAR_RULES` from an interrupted earlier attempt was MISSING `add_expr`/`mul_expr`: the parser's expression ladder contains BOTH the bitwise levels (`\| ^ &`) AND the arithmetic levels (`+ - *`) between shift and unary (verified in `parser.rs` `shift`->`additive`->`multiplicative`). Added both rules + notes; rule count 59 -> 61 | Caught by cross-checking the model against the parser BEFORE writing the sync test - exactly the drift class D-P7-9 exists to catch. The old DESIGN.md fence had add/mul but no bitwise levels; neither document was right on its own |
| 2026-10-26 | 6.1 | D-P7-8's compact field model sketch (`Str/Int/Bool/Null/Obj/Arr`) could not express what IR v1 JSON actually needs: named definitions + `$ref` (recursion in `expr`/`formula`, reuse of `type`/`output_expr`/...), internally-tagged unions (`kind` discriminator -> `oneOf` per variant), and nullable fields (`string|null`, `expr|null`). The shipped model adds `EnumStr`, `Ref`, `TaggedUnion`, `Nullable` to the sketch; draft-07 constraints: `additionalProperties:false` everywhere, nullable refs as `anyOf` ($ref siblings are ignored in draft-07) | The sketch was a starting point, not a contract - D-P7-8's own requirements (required/optional marking, enum constraints) plus IR v1's recursive tagged unions forced the extension. Emitter + md renderer + walk_check stay shared for tasks 7-10 |

## Other information

Per-task verification commands (run from `/workspace/host/RFS`):

```bash
cargo build --release && cargo test --release            # full suite incl. new tests
cargo clippy --release --all-targets && cargo fmt --check
cargo run --release --bin rfs diagram <spec> --format mmd --render svg -o /tmp/x.svg   # default = internal, no tools needed
cargo run --release --bin rfs info format --list
cargo run --release --bin rfs info format spec-rfs --output plantuml | head
```

Assumptions to re-verify before relying on them (all checked 2026-10-26, may drift):

- plantuml-core latest commit rev at implementation time; its `render` API signature.
- mermaid-rs-renderer `default-features = false` keeps `render_strict` compiling.
- layout-rs public path dot-text -> SVG (follow the crate's own binary wiring).
- railroad `default-features = false` still emits standalone SVG strings.

Examples update: explicitly none - no language or algorithm changes in this phase
(D-P7-11), so `examples/valid/` and `examples/invalid/` are untouched.

Tests update: embedded in every task (per-task test rows) plus the dedicated cross-renderer
suite (task 4) and the `tests/info_format_test.rs` family (tasks 5-14).

Training-sets update: explicit task 15.

Git: one commit at closeout (task 16), message summarizing Phase 7; tasks 1-15 leave a
working tree that stays test-green after each task (incremental commits are allowed if a
task is paused, but the phase ships as one documented unit).
