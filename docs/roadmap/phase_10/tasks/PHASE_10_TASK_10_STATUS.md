# Phase 10, Task 10 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_10.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 75 function rows (info.rs 20, info_format.rs 20, ebnf_grammar.rs 8, json_schema.rs 27) + 7 type rows (InfoOptions, FormatInfo, RuleNode, Field, FieldType, TaggedVariant, SchemaDoc) appended to `## All functions` / `## All types`; Component/Functionality still blank |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_10.md` | changed | stale "~85" figures in Planned actions + Scope corrected to ~75 (the Goal's ~75 and the cluster notes were already right - no plan correction needed) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The introspection + self-describing-format cluster is now dependency-annotated: info.rs's renderers are the shared compact text forms (render_output is the diagram-label vocabulary, monitor_status is the R1-R4 verdict shared with the codegen backends); info_format.rs's registry entries each derive their field model from the code's own writers/key constants (the D-P7-9 single-source-of-truth chain: spec-json <- ir.rs to_json, ce-trace/mc-report <- codegen_mc JSON_KEY_* + counterexample_model shared between the two, validate-json <- frontend::validate_json_doc, info-json <- info.rs render_json); ebnf_grammar.rs's const RuleNode model is the canonical text of DESIGN.md section 6; json_schema.rs is the shared draft-07 emitter + md renderer + walk_check sync walker behind all five JSON format entries.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green after fixing two dep references (Json::Str/Json::Null are enum variants, not functions - the Json type row covers them; to_svg's nested-fn reference corrected to the to_svg::to_railroad identifier) |

## Found problems/issues/mismatches

- None vs the plan: correction 2a's "T10 75/7" held exactly (info.rs 20/1, info_format.rs 20/1, ebnf_grammar.rs 8/1 - to_railroad included per the nested-fn convention, json_schema.rs 27/4). The task file's own Planned-actions "~85" was stale relative to its Goal and cluster notes; corrected in place.

## Other important findings/information

- Task 9's forward edges into this chunk are now resolved: main.rs::cmd_info (info.rs::run_info, InfoOptions), main.rs::run_info_format (info_format.rs::list_markdown/find/render) all reference live rows.
- ebnf_grammar.rs::to_svg::to_railroad is the second use of the nested-fn identifier convention introduced in task 8's preamble note - the convention generalizes as expected.
- json_schema.rs's FieldType::r#ref row keeps the raw-identifier spelling from the source (the function name is `ref`, a reserved word).
- Remaining forward edges in the inventory now point only at the diagram cluster (task 11) and ref_mc/specgen (task 12).
