# Phase 10, Task 22 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_22.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | the documentation-quality gate suites: 59 function rows for tests/info_format_test.rs + 13 function rows for tests/docs_md_test.rs appended to `## All functions`. Every row carries bidirectional project Depends (mapped to real identifiers across src/{info_format,ebnf_grammar,json_schema,json_rw,ir,diagram(s),codegen,codegen_mc,ref_mc,spec_hash,frontend,parser,lib}.rs plus the tests/common/fail_variants.rs helpers from task 20; consts like the JSON_KEY_*/JSON_CE_KIND_* keys, GRAMMAR_RULES, KEYWORDS, FORMATS get no Depends entries), a one-line description, and blank Component/Functionality/Documented-in cells. Three nested functions get `<file>::<parent_fn>::<child_fn>` identifiers (collect_terminals in keywords_appear_as_terminals_in_model; walk in check_markdown; walk_dir in collect_docs) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase (no source moved or renamed).

## Removed functionality

None.

## New functionality

- The documentation-quality gate suites are now enumerated in TEST_STRUCTURE.md:
  - info_format_test.rs: the Phase 7 `rfs info format` suite - registry --list pin, spec-rfs md/plantuml/svg pins with KEYWORDS coverage, DESIGN.md section 6 rule-name parity, the five draft-07 schema document pins (spec-json/ce-trace/mc-report/validate-json/info-json), the D-P7-9 sync walks of REAL tool output against the embedded models, the diagram-dot/puml/mmd generated-subset coverage syncs, and the artifact-header pin + hash-variation sanity.
  - docs_md_test.rs: the mdr-stack doc-quality gate - comrak GFM parse, fenced-block closing, mermaid-rs-renderer full-pipeline render, relative-link resolution, plus the four negative-control fixtures (one per failure class).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green; this chunk's own subject suite parses TEST_STRUCTURE.md + SRC_STRUCTURE.md, and the 72 new rows keep it passing (plain-text table cells, no backticked paths or mermaid, so no render/link checks are triggered) |

## Found problems/issues/mismatches

- None. The re-verified counts matched the cluster notes exactly: info_format_test.rs 59 fns / 0 types, docs_md_test.rs 13 fns / 0 types = 72 fns / 0 types. No plan correction was needed (unlike tasks 20/21, whose type estimates were template phantoms).

## Other important findings/information

- Three of the 72 fns are NESTED inside other function bodies and get the `<file>::<parent_fn>::<child_fn>` identifier form: info_format_test's collect_terminals (in keywords_appear_as_terminals_in_model), and docs_md_test's walk (in check_markdown) + walk_dir (in collect_docs). A col-0-only fn grep misses these (it counts 58 + 11 = 69); the authoritative count including nested fns is 59 + 13 = 72.
- docs_md_test.rs's own negative-control fixtures (valid_mermaid_diagrams_and_links_pass, unterminated_fence_is_detected, non_rendering_mermaid_is_detected, dead_relative_link_is_detected) are enumerated like any other test fn - they are the gate's self-tests, not project code, but they ARE real functions of this suite.
- The D-P7-9 "walk" helpers (walk_ce, walk_mc_report, walk_validate_json) and the check_*_documented_subset helpers are the sync machinery that compares REAL generated tool output against the embedded info-format models - their Depends capture which model/schema items they assert over.
