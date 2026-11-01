# Phase 7 - Implementation status

> Written once when all exit criteria of `PHASE_7.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `src/info_format.rs` | `rfs info format` sub-mode: static `FORMATS` registry (10 entries), dispatch, md/schema output rendering; single source of truth for the self-describing format docs (D-P7-6) |
| `src/ebnf_grammar.rs` | EBNF grammar model mirroring the actual parser (61 rules incl. lexical rules); renderers to markdown, PlantUML EBNF and a standalone SVG railroad diagram; source of truth for the DESIGN.md section 6 ebnf fence (D-P7-7) |
| `src/json_schema.rs` | Shared draft-07 JSON Schema emitter from a compact field model (`Str/Int/Bool/Null/Obj/Arr` + `EnumStr/Ref/TaggedUnion/Nullable/FreeObj/Union`) and the D-P7-9 walk checker; used by all five JSON format entries (D-P7-8) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/diagram.rs` | changed | `Renderer { Internal, External }` option + dispatch; plugin gains `internal_render_svg` / `internal_renderer_name` hooks (D-P7-1/D-P7-2) |
| `src/main.rs` | changed | `--renderer internal\|external` flag for `rfs diagram`; `info format` sub-mode dispatch in `cmd_info`; usage text updated to the 10-format registry (task 15 fix) |
| `src/lib.rs` | changed | registers the new modules `ebnf_grammar`, `info_format`, `json_schema` |
| `src/diagrams/mermaid.rs` | changed | internal SVG renderer via `mermaid_rs_renderer::render_strict`; negative control targets a missing diagram header (the renderer rejects strictly only at header level) |
| `src/diagrams/plantuml.rs` | changed | internal SVG renderer via `plantuml_core::render`; emitted puml changed to bare `note on link` with one line per output (plantuml-core rejects the position word); note-on-link text gap documented in-test |
| `src/diagrams/dot.rs` | changed | internal SVG renderer via layout-rs following the upstream binary's code path (`gv::DotParser` -> GraphBuilder -> `VisualGraph::do_it` -> SVGWriter) |
| `src/diagrams/tables.rs` | changed | `TABLE_PLUGIN` / `MD_PLUGIN` set the new plugin fields to `None` (no SVG renderer, error unchanged) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | changed | new section 25 recording D-P7-1..D-P7-13 as shipped + execution notes; section 6 ebnf fence replaced with the canonical text of `to_ebnf_text()` (was stale: missing version/on_error_decl/priority_action/output_call_payload/bitwise ladder/lexical rules) |
| `KB.md` | changed | phase-table Phase 7 row + lint/format line updated to the close-out baseline |
| `START.md` | changed | Phase 7 completion summary + known-good baseline re-measured at close-out (2026-10-28) |
| `docs/roadmap/INTRO.md` | changed | phase-state table row 7 -> COMPLETE (close-out 2026-10-28) |
| `docs/user/README.md` | changed | command overview: SVG rendered in-process by default; `info format` mode mentioned |
| `docs/user/INTEGRATION.md` | changed | external-tool section: `diagram --render` resolves external tools ONLY with `--renderer external` (default internal needs none) |
| `docs/user/MIGRATION.md` | changed | new "v1 -> v1.1" section: `--renderer` flag (default internal, external preserved byte-for-byte), puml source normalization, `info format` sub-mode |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | changed | mermaid-rs-renderer promoted from dev-dependency to regular dependency (`default-features = false`, SVG only); plantuml-core 0.2.0 added as rev-pinned git dependency (rev `8066f6124a86ed5da59c6d7d570f0568891c378e` - NOT on crates.io); layout-rs + railroad added (SVG only) |
| `training/frontend_cli/rfs_training.jsonl` | changed | 3 new two-message SFT entries with verbatim real tool output (`--renderer internal\|external` incl. the zero-external-deps proof; `info format` usage; CI/CD schema validation via python jsonschema); aggregate re-pinned 261 -> 264 (D-P7-12) |

## Changed functionality
- `rfs diagram --render svg` now renders IN-PROCESS by default (previously required external dot/plantuml/mmdc): zero external binaries needed for production/CI SVG output.
- Emitted PlantUML source: `note right on link` -> bare `note on link`; multiple outputs emit ONE `note on link : emits X` line each instead of a single multi-line note block (parses in BOTH plantuml-core and real PlantUML; external path re-verified byte-for-byte).
- DESIGN.md section 6 ebnf fence: replaced with the canonical text of the in-binary grammar model (was stale vs the actual parser).

## Removed functionality
- None. The external renderer path is preserved byte-for-byte (`--renderer external`); table/md "no SVG renderer" error unchanged.

## New functionality
- `rfs diagram --renderer internal|external` flag (default `internal`; unknown values rejected with a clear error).
  - In-process SVG rendering: mmd via mermaid-rs-renderer, puml via plantuml-core, dot via layout-rs (all linked into the rfs binary).
  - Internal-mode `--print-script`: heredoc + comment naming the in-process crate/version, no tool invocation.
- `rfs info format [--list] <name?> [--output md|plantuml|svg|schema]` sub-mode with a 10-format registry:
  - `spec-rfs` (md/plantuml/svg): EBNF grammar mirroring the parser (61 rules).
  - `spec-json`, `ce-trace`, `mc-report`, `validate-json`, `info-json` (md/schema): document description + draft-07 JSON Schema for CI validation.
  - `diagram-dot`, `diagram-puml`, `diagram-mmd` (md): documented generated-subset line grammars.
  - `artifact-header` (md): the exact line-1 stamp of every generated artifact.
- Sync tests pinning documentation to code (D-P7-9): KEYWORDS/grammar parity, two-directional key walks over ALL real JSON documents, per-line coverage over all 50 valid examples x every module for the three diagram formats, byte-equal artifact-header check over 50 examples x both backends x all three codegen commands.
- Cross-renderer syntax parity suite: internal half covers ALL 50 valid examples x 3 formats; external half (availability-gated) covers a pinned subset x dot/plantuml/mmdc; compares parse/render SUCCESS only, never visual output.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/diagram_render_parity_test.rs` | new | cross-renderer syntax parity (internal: 50 examples x 3 formats; external: pinned subset x 3 tools, availability-gated, serialized) + unresolvable-tools e2e + print-script pins + 3 garbage-rejection negative controls |
| `tests/info_format_test.rs` | new | `rfs info format` family: registry pin, per-format md/schema pins, D-P7-9 walk/coverage sync tests (38 tests at close-out) |
| `tests/common/fail_variants.rs` | new | shared pinned FAIL-variant specs (moved out of benchmark_mc_test to avoid ~220 lines of duplication); consumed by the benchmark suite + ce-trace walk |
| `tests/frontend_test.rs` | changed | real-tool diagram tests switched to explicit `--renderer external` (internal is now the default) |
| `tests/benchmark_mc_test.rs` | changed | pinned FAIL variants now consumed from `tests/common/fail_variants.rs` |
| `tests/common/mod.rs` | changed | registers the `fail_variants` module |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (526) | OK |
| `cargo test --workspace` (550) | OK |
| `cargo test --features smt` (528) | OK |
| `cargo test-generated-code` (1683, 192 modules) | OK |
| `cargo test --test examples_test` (3; 50 valid + 13 invalid) | OK |
| training pipeline: extract + build + `cargo test-training` (5; aggregate 264 lines) | OK |
| `cargo clippy --all-targets` (no errors; 46 warning lines / 35 unique - dead-code in shared test modules or pre-existing patterns, zero in Phase 7's new source files and the two new test suites) | OK |
| `cargo fmt --check` | OK |
| `cargo test --test diagram_render_parity_test` (13; internal + external halves both ran - dot/plantuml/mmdc installed) | OK |

## Found problems/issues/mismatches
- DESIGN.md section 6 ebnf fence was STALE vs the actual parser (missing `version` in module, `on_error_decl`, `priority_action`, `output_call_payload`, the bitwise ladder levels, literal alternatives in pattern rules; lexical rules referenced but never defined) - replaced with the canonical model text; the sync test now enforces rule-name parity.
- The WIP `GRAMMAR_RULES` from an interrupted earlier attempt was MISSING `add_expr`/`mul_expr` (the parser's expression ladder contains BOTH the bitwise and the arithmetic levels between shift and unary); rule count 59 -> 61. Caught by cross-checking the model against the parser BEFORE writing the sync test.
- plantuml-core's parser REJECTS the position word in `note right on link` - emitted puml changed to bare `note on link` (real PlantUML accepts both; external path re-verified).
- plantuml-core does NOT draw note-on-link TEXT into the SVG (syntax accepted, layout skips it) - the e2e pin asserts states/edge labels only and documents the gap in-test.
- mermaid-rs-renderer (like mmdc) ACCEPTS dangling arrows and unclosed composites; only the diagram header level rejects strictly - the negative control targets a missing header.
- Plan name drift: `mc_report_json.rs` -> the constants actually live in `src/codegen_mc.rs`; `ir.rs render_json` -> lives in `src/info.rs`.

## Other important findings/information
- plantuml-core is NOT on crates.io; pinned git rev `8066f6124a86ed5da59c6d7d570f0568891c378e` (plantuml-core 0.2.0, the repo's latest commit at task 2 implementation time, 2026-10-26). First build on a fresh machine needs network; offline rebuilds work via the cargo git cache (recorded in TOOLCHAIN.md section 6.5).
- Training aggregate re-pinned 261 -> 264 lines (3 new frontend_cli entries); snippets unchanged at 138 valid + 128 invalid (D-P7-11: no new `.rfs` files).
- The installed apt plantuml (1.2020.02) predates PlantUML EBNF support (`@startebnf` -> "No diagram found", exit 100) - the `spec-rfs --output plantuml` text targets real PlantUML with EBNF support; internal SVG rendering does not use it.
- Phase 7 shipped as incremental commits (tasks 1-4 squashed, one per task 5-15, plus one fixup for missed files) - the phase is documented as one unit by this file + the TASKS.md close-out entry.
