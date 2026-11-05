# Phase 11, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Re-sync every docs<->source reference after the refactor: the //! Docs:
manifest (42 moved paths) and all backticked source-path references in
docs/**/*.md, so no stale pointer survives.

## Context
- Manifest: ALL 42 relocated src files are listed in tests/dev_docs_manifest.txt
  (verified at planning - the PHASE_10_STATUS.md "28 of 71" figure is stale).
  After tasks 1-9, dev_docs_test FAILS and its output IS the authoritative
  update list: run it first, then rewrite the manifest paths old->new per the
  plan's mapping. The three test renames (task 9) are NOT in the manifest unless
  dev_docs_test says otherwise - verify, do not assume.
- Doc references: 109 distinct backticked source paths across docs/**/*.md at
  planning time (thousands of occurrences). Script the rewrite with the plan's
  old->new mapping applied to every `src/...`, `tests/...` backticked .rs
  reference. The mapping covers all 42 src moves + 3 test renames; anything not
  in the mapping must not be rewritten (kept files keep their references).
- Scope is docs/**/*.md ONLY - TASKS.md history keeps its pre-refactor paths by
  design (append-only record, per INTRO.md layout note); roadmap phase files
  likewise cite historical paths.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-9. | all moves + renames | the tree is final; references now have a stable target |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 11.-12. | inventory actualization | the inventories' own path rewrites follow the same mapping |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/dev_docs_manifest.txt` | change | 42 paths rewritten old->new per dev_docs_test's output |
| every docs/**/*.md carrying a backticked moved/renamed source path | change | scripted reference rewrite (109 distinct refs at planning) |

## Planned actions

### Re-sync the manifest
Run cargo test --test dev_docs_test; take its failure output as the update list;
rewrite tests/dev_docs_manifest.txt paths old->new; re-run until 9/9 green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/dev_docs_manifest.txt` | change | moved-file paths updated to their new locations |

### Rewrite all doc source references
Script the old->new mapping over every backticked .rs path in docs/**/*.md;
then verify with a fresh grep for EVERY old path in the mapping - each must
return zero hits under docs/. Rationale: exit criterion "no stale source-path
reference remains in any docs/**/*.md file".

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| docs/**/*.md (every file with a moved/renamed reference) | change | scripted rewrite, one pass per mapping entry |

### Gate
dev_docs_test 9/9; docs_md_test 5/5; fresh grep for all 45 old paths under
docs/ returns nothing.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- Updated manifest; zero stale source references in docs/**/*.md.

## Task exit criteria
- [ ] dev_docs_test 9/9 green on the updated manifest.
- [ ] A fresh grep for every old path (42 src + 3 test) under docs/ returns zero hits.
- [ ] docs_md_test 5/5 still green.
- [ ] TASKS.md and docs/roadmap/** untouched by this task.

## Non-goals
- No doc content rewording beyond path substitution; no restructuring of docs/.
