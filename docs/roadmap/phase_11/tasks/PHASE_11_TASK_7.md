# Phase 11, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Execute the phase's one genuine MERGE (diagrams/mod.rs label helpers into
cli/commands/diagram/mod.rs), move the four diagram plugins, and relocate the
info_format cluster (registry + EBNF leaf) under cli/.

## Context
- diagram.rs -> cli/commands/diagram/mod.rs: the plugin registry becomes the
  module root (module path cli::commands::diagram). src/diagrams/mod.rs holds
  five label-helper items (event_label / guard_label / emit_labels / states /
  render_check) that are MERGED into diagram/mod.rs as items - this is the only
  hand edit of the phase (read both files, move the items, fix their internal
  use-paths); src/diagrams/ is then deleted.
- Plugins move unrenamed: dot.rs, plantuml.rs, mermaid.rs, tables.rs ->
  cli/commands/diagram/.
- info_format.rs -> cli/info_format/mod.rs (absorbed as mod.rs; module path
  becomes cli::info_format - 1 external test import site); ebnf_grammar.rs ->
  cli/info_format/ebnf_grammar.rs (leaf consumed only by the registry). The
  JSON-schema emitter already left for json/ in task 2.
- External import sites: diagram 2, diagrams 3, info_format 1, ebnf_grammar 1 -
  re-verify by grep at task start.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | cli/ root + commands | cli/commands/mod.rs exists and gets extended here |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | mc/ | last source move; its import lines are final after this task |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/diagram.rs` | remove (moved) | git mv to src/cli/commands/diagram/mod.rs |
| `src/diagrams/mod.rs` | remove | five label-helper items merged into diagram/mod.rs |
| `src/diagrams/{dot,plantuml,mermaid,tables}.rs` | remove (moved) | git mv to src/cli/commands/diagram/ |
| `src/info_format.rs` | remove (moved) | git mv to src/cli/info_format/mod.rs |
| `src/ebnf_grammar.rs` | remove (moved) | git mv to src/cli/info_format/ebnf_grammar.rs |
| `src/cli/commands/mod.rs` | change | gains the diagram module declaration |
| all import sites of the seven modules | change | scripted use-path rewrite + merge fixes |

## Planned actions

### Merge diagrams/mod.rs into diagram/mod.rs + move the plugins
git mv src/diagram.rs src/cli/commands/diagram/mod.rs; read src/diagrams/mod.rs,
move its five label-helper items verbatim into diagram/mod.rs (fixing their
use-paths for the new module location); git mv the four plugin files; delete
src/diagrams/. Rationale: the plugins exist only for the diagram command - they
nest under it, and the shared label vocabulary lives with the registry.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/diagram.rs` | remove | git mv to src/cli/commands/diagram/mod.rs |
| `src/diagrams/mod.rs` | remove | items merged into diagram/mod.rs, then deleted |
| `src/diagrams/{dot,plantuml,mermaid,tables}.rs` | remove | git mv to src/cli/commands/diagram/ |
| `src/cli/commands/diagram/mod.rs` | change | gains the five merged label-helper items |

### Move the info_format cluster
git mv src/info_format.rs src/cli/info_format/mod.rs and src/ebnf_grammar.rs
src/cli/info_format/ebnf_grammar.rs; script the import-site rewrites (registry
path -> cli::info_format, EBNF leaf -> cli::info_format::ebnf_grammar).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/info_format.rs` | remove | git mv to src/cli/info_format/mod.rs |
| `src/ebnf_grammar.rs` | remove | git mv to src/cli/info_format/ebnf_grammar.rs |
| import sites of both modules | change | scripted rewrite |

### Gate
cargo build && cargo test green; src/diagrams/ gone from the tree.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/cli/commands/diagram/` (registry+merged helpers+4 plugins);
  `src/cli/info_format/` (registry+EBNF leaf); src/diagrams/ deleted.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] src/diagrams/ no longer exists; the five label helpers are defined exactly once, in cli/commands/diagram/mod.rs.
- [ ] No file references the old paths for the seven modules (grep-verified).

## Non-goals
- No diagram/info-format behavior change; plugin output byte-identical.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Absorbed info_format/mod.rs needed its child declaration (E0255) | The original `use crate::{ebnf_grammar, json::json_schema};` grouped import became a self-referential re-import once the file became `cli/info_format/mod.rs` with a `pub mod ebnf_grammar;` child - the compiler rejected the duplicate definition. Fixed by declaring the child and dropping it from the use list (`use crate::json::json_schema;`). Same lesson as tasks 4/5 (absorbed mod.rs files need explicit sibling declarations), now also covering the import side. |
| DEPENDENCIES.md file-level mermaid graph fully rebuilt | The graph's node labels are BARE file names (no `src/` prefix), so none of the tasks 1-6 path scripts ever touched it - every moved file still carried its pre-Phase-11 label (root (6) with error.rs/cli_log.rs/frontend.rs, specgen inside the CLI subgraph, monitor as its own subgraph, ...). Rebuilt in this commit: all ~35 stale labels updated to current locations, subgraphs restructured to the post-task-7 tree (crate root 2 / lang 6 / json 2 / utils 1 / codegen core+monitor 5 / codegen/backends 9 / cli shared+info_format 5 / cli/commands 5 / diagram plugins 4 / MC engine 4 / bins 5), the merged `dm` node deleted with its seven edges resolved (dm-->inf became dg-->inf; the rest subsumed by existing dg edges), intro count 71 -> 86. Edge semantics otherwise preserved (174 -> 167 edges). |
| Directory-level `src/diagrams/` mentions fixed by hand | The scripted rewrite only matched full file paths; seven docs carried directory-form mentions (`src/diagrams/`, `src/diagrams/<fmt>.rs`, the line-wrapped `src/\ndiagram.rs` in DESIGN.md) that were fixed individually (KB x2, DESIGN x1, START tree block, ARCHITECTURE x3, DECISIONS x1, DEPENDENCIES x1, cli_commands x2, CHANGE_IMPACT x1, llm/cli_commands x1). |
