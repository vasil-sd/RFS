# Phase 10, Task 11 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_11.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | 41 function rows (diagram.rs 6, diagrams/mod.rs 5, dot.rs 7, plantuml.rs 6, mermaid.rs 9, tables.rs 8) + 12 type rows (diagram.rs 10: PlanRenderFn/RenderSvgFn/InternalRenderSvgFn/Renderer/SvgMode/DiagramPlugin/RenderPlan/ToolOverride/DiagramOptions/DiagramResult; tables.rs 2: CellEntry/TableData) appended to `## All functions` / `## All types`; Component/Functionality still blank |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The diagram cluster is now dependency-annotated: diagram.rs's plugin registry mirrors the codegen backend pattern (static data + function pointers, one module per format; adding a format = one module + one registry line), with `--render svg` dispatching between the in-process renderers (Phase 7 default, D-P7-1) and the resolved external tools under strict error semantics; diagrams/mod.rs's four label helpers are the shared vocabulary that reuses info.rs's compact renderers (render_pattern_variant_group/render_expr/render_output), so diagram labels read identically to `rfs info` output; dot/plantuml/mermaid each pair a pure-Rust generator with an external argv builder + plan_render/render_svg trio and a Phase 7 in-process renderer (layout-rs / plantuml-core / mermaid-rs-renderer, D-P7-2); tables.rs's single build() data layer feeds both the plain-text and Markdown renderings so the two can never disagree.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green (one escaped pipe in render_text's description, one in the DiagramOptions type row) |

## Found problems/issues/mismatches

- None vs the plan: correction 2a's "T11 41/12" held exactly per file (diagram.rs 6/10, mod.rs 5/0, dot.rs 7/0, plantuml.rs 6/0, mermaid.rs 9/0, tables.rs 8/2); none of the six files has a `#[cfg(test)]` module, so raw and test-aware greps agree.

## Other important findings/information

- Task 9's forward edges into this chunk are now resolved: main.rs::cmd_diagram (DiagramOptions::default, list_diagrams, run_diagram, Renderer, DiagramOptions) and main.rs::tool_override_mut (DiagramOptions, ToolOverride) all reference live rows.
- The diagram cluster is the last consumer of task 9's forward-edge set that lands before task 12: remaining unresolved references in the inventory point only at ref_mc.rs (4), spec_hash.rs (1), and specgen.rs (9) - all task 12 scope.
- Registry dispatch through DiagramPlugin's raw function pointers (generate/plan_render/render_svg/internal_render_svg) is annotated with the plugin type + the static callees, following task 9's cmd_codegen precedent of naming the dispatch mechanism rather than enumerating every possible callee.
- mermaid.rs's two pub consts (DEFAULT_PUPPETEER_ARGS, PUPPETEER_ARGS_VAR) are constants, not types - correctly outside the inventory's type table.
