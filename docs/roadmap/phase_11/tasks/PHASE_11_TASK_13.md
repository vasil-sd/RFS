# Phase 11, Task 13 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the two actualized inventories into docs/llm/ and codify in the protocol
the impact-calculation rule (linking both documents) and the docs<->source
correspondence rule (source change => inventory rows updated in the same commit).

## Context
- Move: git mv SRC_STRUCTURE.md docs/llm/SRC_STRUCTURE.md and TEST_STRUCTURE.md
  docs/llm/TEST_STRUCTURE.md. Only TASKS.md history references them at old paths
  - that stays by design (append-only record). No docs/dev or docs/llm file
  links to them yet (verified at planning) - this task creates the first links.
- PROTOCOL.md gains two rules (new sections after section 3, numbered 4 and 5):
  (a) Impact calculation: before scoping ANY source-code change, consult the
  affected rows' Depends columns in docs/llm/SRC_STRUCTURE.md (non-test source)
  or docs/llm/TEST_STRUCTURE.md (test source) - the bidirectional edges define
  the blast radius; a change plan that names no inventory rows is incomplete.
  (b) Docs<->source correspondence: every commit that adds/removes/renames a
  function, type or source file updates the affected inventory rows (identifier,
  Depends, description) in the SAME commit - the inventories are living
  documents, and the string-masked audit is the check.
- PRINCIPLES.md section 2 (general rules for the roadmap document set) gains one
  bullet pointing at the two new protocol sections, so the doc-set rules stay
  the single place that names the correspondence obligation.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 11.-12. | inventory actualization | the documents are in their final content before the move |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 14. | Full verification + close-out | verifies the moved documents parse at their new home |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `SRC_STRUCTURE.md` | remove (moved) | git mv to docs/llm/SRC_STRUCTURE.md |
| `TEST_STRUCTURE.md` | remove (moved) | git mv to docs/llm/TEST_STRUCTURE.md |
| `docs/roadmap/PROTOCOL.md` | change | new sections 4 (impact calculation) + 5 (correspondence) linking both inventories |
| `docs/roadmap/PRINCIPLES.md` | change | section-2 bullet pointing at the two protocol sections |

## Planned actions

### Move both inventories into docs/llm/
git mv both files; no content edits. Rationale: they are LLM-facing reference
documents - their home is the docs/llm/ set, not the repo root.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `SRC_STRUCTURE.md` | remove | git mv to docs/llm/SRC_STRUCTURE.md |
| `TEST_STRUCTURE.md` | remove | git mv to docs/llm/TEST_STRUCTURE.md |

### Add the two protocol rules
Write PROTOCOL.md section 4 (impact calculation - mandatory Depends-column
consultation, linking both documents by their new paths) and section 5
(correspondence - same-commit inventory update obligation + the audit as check);
add the PRINCIPLES.md section-2 pointer bullet. Rationale: the user's close-out
requirements - the protocol must name both how to USE the inventories and how
to KEEP them current.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/roadmap/PROTOCOL.md` | change | sections 4 + 5 added after section 3 |
| `docs/roadmap/PRINCIPLES.md` | change | one new bullet in section 2 |

### Gate
docs_md_test 5/5 (parses both documents at their new location, mermaid renders);
dev_docs_test 9/9.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- docs/llm/SRC_STRUCTURE.md + docs/llm/TEST_STRUCTURE.md; PROTOCOL.md sections
  4-5; the PRINCIPLES.md pointer.

## Task exit criteria
- [ ] Both inventories exist under docs/llm/ and nowhere else (repo root clean).
- [ ] PROTOCOL.md carries both rules with working links to the new paths.
- [ ] PRINCIPLES.md section 2 names the correspondence obligation via the protocol sections.
- [ ] docs_md_test 5/5 + dev_docs_test 9/9 green.

## Non-goals
- No inventory content change (that was tasks 11-12); no other doc restructured.
