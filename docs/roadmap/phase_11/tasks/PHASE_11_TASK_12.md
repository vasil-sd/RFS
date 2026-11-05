# Phase 11, Task 12 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Actualize TEST_STRUCTURE.md: rewrite identifiers for the three suite renames,
convert the "Proposed layout" into the actual layout, and prove exactness with a
fresh string-masked grep audit.

## Context
- Only three test paths changed (task 9): library_comparison.rs ->
  parser_comparison_test.rs, examples_test.rs -> example_corpus_test.rs,
  integration_example_test.rs -> endpoint_protocol_test.rs. Every other test
  identifier is untouched - the rewrite is small but must still be audit-exact.
- Section conversion: "## Proposed layout" -> "## Layout"; the file-mapping
  table's three renamed rows show actual names; the layout mermaid already drew
  the proposed names (it used the new names by design) - verify it matches the
  actual tree, adjust only if it does not.
- Audit method: same Phase 10 close-out string-masked grep, test-side scope
  (tests/**, crates/*/tests/**, fuzz/fuzz_targets/** + inline test modules of
  all host files + the four fuzz_target! macro entries). Must return zero
  per-file diffs.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 9. | Test-suite renames | the three new names exist |
| 10. | Docs re-sync | same mapping proven against docs/ |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Inventories to docs/llm/ + protocol rules | moves the actualized document |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TEST_STRUCTURE.md` | change | three-path identifier rewrite; proposal section converted; file-mapping actualized |

## Planned actions

### Rewrite the renamed identifiers
Script the three old->new test-path rewrites over every row identifier AND
Depends cell that names them. Rationale: same stable-row-identity rationale as
task 11.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` (`## All functions`, `## All types`) | change | scripted rewrite of the three renamed paths |

### Convert proposal section to actual
Rename "## Proposed layout" -> "## Layout"; rewrite its preamble from proposal
voice; update the file-mapping table's three rows to actual names. Rationale:
the document becomes the test tree's description.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `TEST_STRUCTURE.md` (`## Layout`, `## File mapping`) | change | section conversion + mapping actualization |

### Audit
Run the fresh string-masked grep over the refactored test tree (incl. inline
test modules + fuzz macro entries); require zero per-file diffs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- An actualized TEST_STRUCTURE.md that describes the refactored test tree exactly.

## Task exit criteria
- [ ] Fresh string-masked grep audit: zero per-file diffs against the refactored test tree.
- [ ] No old suite-name string remains in the document (grep for all three old names).
- [ ] "Proposed layout" heading gone; layout + file mapping read as actual state.
- [ ] docs_md_test 5/5.

## Non-goals
- No re-classification of Component/Functionality cells; no change to the
  Conventions preamble beyond identifier examples.
