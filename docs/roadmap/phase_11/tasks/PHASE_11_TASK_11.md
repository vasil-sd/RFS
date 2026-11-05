# Phase 11, Task 11 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Actualize SRC_STRUCTURE.md: rewrite every row identifier and Depends entry to
the post-refactor paths, convert the "Proposed" sections into the actual state,
swap in the LCG-consolidation rows, and prove exactness with a fresh string-
masked grep audit.

## Context
- Identifier convention after the refactor: a file that became a directory's
  mod.rs is identified by its REAL path - src/codegen/mod.rs::<item>,
  src/codegen/backends/rust/mod.rs::<item>, etc. The five merged diagram label
  helpers become src/cli/commands/diagram/mod.rs::<item>.
- LCG row swap: DELETE the specgen Rng type row + its method rows and the
  codegen_test Lcg type row + its method rows (task 1 deleted those structs);
  ADD utils/rng.rs rows for the consolidated generator (type + method rows,
  Depends per the actual source); rewrite every Depends entry that named the old
  Rng/Lcg identifiers to the utils/rng.rs ones. The two pub consts are not
  functions/types - they get no rows.
- Section conversions: "## Proposed naming scheme" -> "## Naming scheme" (drop
  the "Proposal only" preamble; update the target-tree text block to show
  codegen/ident.rs, which the proposal flagged but did not draw); "## Proposed
  layout" -> "## Layout"; the file-mapping table's "moved to ..." annotations
  become the actual locations (the Source file name column shows the NEW path).
- Audit method: the Phase 10 close-out string-masked grep (character-level
  masker: comments before strings, raw-string hash counting, char-literal
  lookahead; test-module-aware) must return ZERO per-file diffs against the
  refactored tree - the same standard that closed Phase 10.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-9. | all source moves | the tree is final; identifiers have stable targets |
| 10. | Docs re-sync | same old->new mapping, already proven against docs/ |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Inventories to docs/llm/ + protocol rules | moves the actualized document |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | change | identifiers + Depends rewritten; LCG rows swapped; proposal sections converted; file-mapping actualized |

## Planned actions

### Rewrite all row identifiers + Depends entries
Script the plan's old->new mapping over every row: the Function/Type identifier
column AND every Depends cell (both contain full paths). Apply the mod.rs
convention for absorbed files and the diagram-merge convention for the five
label helpers. Rationale: stable row identity is what future grep-based tooling
and the refactoring consumers rely on - a missed Depends entry breaks impact
calculation silently.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` (`## All functions`, `## All types`) | change | scripted path rewrite of identifiers + Depends, one pass |

### Swap the LCG rows
Delete the specgen Rng + codegen_test Lcg rows (type + methods); add the
utils/rng.rs rows read from the actual source; fix every Depends cell that named
the deleted identifiers. Rationale: the inventory must describe the code that
exists after task 1.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` (`## All functions`, `## All types`) | change | row deletion + addition per the actual source |

### Convert proposal sections to actual
Rename the two "Proposed" headings; rewrite their preambles from proposal voice
to actual-state voice; update the target-tree text block (add codegen/ident.rs);
rewrite the file-mapping table rows to show actual locations. Rationale: after
this task the document is the tree's description, not a suggestion for it.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` (`## Naming scheme`, `## Layout`, `## File mapping`) | change | section conversion + tree/mapping actualization |

### Audit
Run the fresh string-masked grep over the refactored non-test tree; require zero
per-file diffs (fns and types). Any diff is a task-correction loop, not a
hand-wave.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- An actualized SRC_STRUCTURE.md that describes the refactored tree exactly.

## Task exit criteria
- [ ] Fresh string-masked grep audit: zero per-file diffs (fns and types) against the refactored tree.
- [ ] No old path string remains anywhere in the document (grep for every mapping entry's old side).
- [ ] "Proposed" headings gone; naming scheme + layout + file mapping read as actual state; ident.rs present in the tree block.
- [ ] LCG rows swapped: no specgen Rng / codegen_test Lcg rows remain; utils/rng.rs rows present and matching source.
- [ ] docs_md_test 5/5 (the document still parses + its mermaid renders).

## Non-goals
- No re-classification of Component/Functionality cells (categories are path-
  independent); no change to the Conventions preamble beyond identifier examples.
