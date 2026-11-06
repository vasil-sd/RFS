# Phase 11, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none new) | both absorbed mod.rs files keep their existing //! Docs: pointer lines (diagram/mod.rs names dev:cli-commands-diagram; info_format/mod.rs names dev:cli-commands-overview + llm:cli-commands) - fragment ownership unchanged |

## Changed/removed source files

| Name | Operation | Description |
|---|---|
| `src/diagram.rs` | removed (moved) | git mv to `src/cli/commands/diagram/mod.rs`; 80 changed lines = the MERGE (five label-helper items + their ir-type imports appended under a marked section, verbatim - their bodies were already absolute paths after task 6), four sibling `pub mod` declarations, registry refs rewritten from the bound name `diagrams::` to absolute `crate::cli::commands::diagram::<plugin>::`, module doc comment updated |
| `src/diagrams/mod.rs` | removed (merged) | the phase's one genuine merge: event_label / guard_label / emit_labels / states / render_check moved into diagram/mod.rs, file deleted, `src/diagrams/` directory gone |
| `src/diagrams/{dot,plantuml,mermaid,tables}.rs` | removed (moved) | git mv to `src/cli/commands/diagram/`; 6-8 changed lines each = import rewrites (crate::diagram -> cli::commands::diagram, crate::diagrams::{helpers,render_check} -> same module) + doc-comment path fixes + fmt import ordering |
| `src/info_format.rs` | removed (moved) | git mv to `src/cli/info_format/mod.rs`; 10 changed lines = NEW `pub mod ebnf_grammar;` child declaration (E0255 - the old grouped import became a self-referential re-import), use list reduced to json_schema, three doc-comment file mentions repointed |
| `src/ebnf_grammar.rs` | removed (moved) | git mv to `src/cli/info_format/ebnf_grammar.rs`, contents byte-identical (0 changed lines) |
| `src/lib.rs` | changed | four `pub mod` lines removed (diagram, diagrams, ebnf_grammar, info_format) - the crate root now declares 10 modules |
| `src/cli/mod.rs` | changed | gains `pub mod info_format;` |
| `src/cli/commands/mod.rs` | changed | gains `pub mod diagram;`; doc comment updated (diagram joined commands/, info_format sits at cli/) |
| `src/main.rs` | changed | 11 use-path rewrites: rfs::diagram:: -> rfs::cli::commands::diagram:: (8 sites), rfs::info_format:: -> rfs::cli::info_format:: (3 sites) |
| `tests/frontend_test.rs`, `tests/info_format_test.rs`, `tests/diagram_render_parity_test.rs` | changed | scripted rewrites: rfs::diagram:: / rfs::diagrams:: (helpers AND plugin paths) -> rfs::cli::commands::diagram::, rfs::info_format:: -> rfs::cli::info_format::, rfs::ebnf_grammar:: -> rfs::cli::info_format::ebnf_grammar:: (18 + 57 + 8 sites) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `tests/dev_docs_manifest.txt` | changed | eight old paths replaced by the seven new sorted entries (87 -> 86 entries; diagram/* before gen_spec, info_format/* between frontend and log) |
| `docs/dev/CODE_MAP.md` | changed | "Root area" renamed "Crate root (2 files)" (Parser subsection + ebnf_grammar bullet removed; lib.rs module count 14 -> 10); CLI surface section 9 -> 16 files restructured into four subsections (Shared infrastructure 4 / Commands 5 / Diagram cluster 5 / Info-format cluster 2) with the merged mod.rs bullet documenting the label-helper merge; "Diagrams + bins area (12)" reduced to "Bins + training area (5)" |
| `docs/llm/TASK_MAP.md` | changed | Map-section path rewrites (7 refs); Coverage: eight scattered bullets replaced by grouped entries inside the cli block (info_format pair after frontend, diagram quintet after commands/mod.rs); header 87 -> 86 |
| `docs/dev/DEPENDENCIES.md` | changed | file-level mermaid graph FULLY REBUILT (correction): ~35 bare node labels updated to current locations (stale since before Phase 11 - the path scripts never matched them), subgraphs restructured to the post-task-7 tree, merged dm node deleted with its seven edges resolved (dm-->inf -> dg-->inf; rest subsumed), intro count 71 -> 86; one prose directory mention fixed |
| `docs/dev/{cli_commands,ARCHITECTURE,DECISIONS,ALGORITHMS,ir_json_rw,parser,REFERENCES}.md`, `docs/llm/{cli_commands,CHANGE_IMPACT,ir_json_rw,parser}.md` | changed | 160 path refs rewritten (cli_commands.md alone 79) + directory-level `src/diagrams/` mentions fixed by hand (ARCHITECTURE x3 incl. two mermaid node labels, DECISIONS x1, cli_commands x2, CHANGE_IMPACT x1, llm/cli_commands x1) |
| `KB.md`, `DESIGN.md`, `START.md` | changed | root docs kept in sync per the standing user direction: KB 10 + 2 dir-form refs, DESIGN 4 + the line-wrapped `src/\ndiagram.rs` mention, START 3 + its source-tree block's two cluster lines restructured to directory form |

## Other file changes

| Name | Operation | Description |
|---|---|
| (none) | - | no examples/training/build-file changes |

## Changed functionality

None - pure moves/renames + the one merge; plugin output byte-identical (the 50-example diagram sweeps + cross-renderer parity suite green). No public API change: these modules were never re-exported at the crate root, all consumers use fully-qualified paths.

## Removed functionality

- The `src/diagrams/` directory and its module-root file (items merged, not dropped).

## New functionality

- `src/cli/commands/diagram/`: registry + shared label helpers + the four format plugins in one directory (the plugin pattern's home now matches the codegen backends' layout).
- `src/cli/info_format/`: the info-format registry absorbing its EBNF-grammar leaf.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| (none) | - | import-path edits only in three test files; no test logic touched, no suites added/removed |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK (warning-free after the E0255 fix) |
| `cargo test` (full) | OK - 535 passed / 0 failed, exactly the documented baseline (diagram byte-pin + cross-renderer parity suites prove plugin output identity) |
| `cargo test --test dev_docs_test` (9) | OK - green after the manifest + CODE_MAP + TASK_MAP re-sync |
| `cargo fmt --check` | OK - clean after import-order normalization in the four plugins |

## Found problems/issues/mismatches

- E0255 on the absorbed info_format/mod.rs: the old grouped import re-imported its own new child module. Fixed by declaring the child + trimming the use list (task-correction recorded).
- DEPENDENCIES.md's file-level mermaid graph had been silently stale since before Phase 11 (bare labels invisible to every path script) - rebuilt wholesale this commit; future move tasks should check mermaid node labels, not just backticked paths.

## Other important findings/information

- The merge was lower-risk than planned: task 6 had already rewritten the helpers' internal refs to absolute paths (crate::cli::commands::info::render_*, crate::cli::ext_tool), so all five items moved verbatim with zero body edits.
- diagram/mod.rs's registry used the BOUND name (`use crate::diagrams;` + `diagrams::dot::DOT_PLUGIN`) - the import was deleted and the five registry lines rewritten to absolute paths (task-4 unqualified-usage lesson, third occurrence).
- No fuzz/ or crates/ lockstep needed again: zero references to any of the seven modules there.
