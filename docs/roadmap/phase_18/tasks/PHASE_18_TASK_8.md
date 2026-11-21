# Phase 18, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Add the "Other examples" section to `docs/user/EXAMPLES.md` - a 50-row table (title | short
FSM description + rationale | README link) that lets users discover the whole Phase 18
real-world-FSM catalogue from the user docs. Rationale: the examples are only useful if users
can find them; this is the discovery surface.

## Context
- Depends on tasks 1-7: every one of the 50 READMEs must exist so each link resolves (the
  `docs_md_test` gate checks that all cross-links in repo `.md` files resolve and that mermaid
  parses).
- The table format is specified in PHASE_18.md ("The `docs/user/EXAMPLES.md` 'Other examples'
  table"): one row per FSM - `<title>` | <one or two sentences: what the machine is, the
  industrial area / standard it comes from, and which RFS functionality the example showcases>
  | `[README](../../crates/examples/<fsm_title>/README.md)` (link pattern; `<fsm_title>` is replaced per row).
- The section intro states these are real-world FSMs modelling the full RFS workflow
  (specify -> analyze -> model-check positive/negative -> generate production code in both
  backends), complementing the language-feature specs in `examples/valid/` and the Phase 17
  integration-workflow examples.
- Relative link depth: from `docs/user/EXAMPLES.md` to `crates/examples/<name>/README.md` is
  `../../crates/examples/<name>/README.md`.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-7. | all example tasks | every README must exist so the 50 links resolve |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Close-out | verifies the section is present and links resolve |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/EXAMPLES.md` | change | new "Other examples" section + 50-row table |
| `docs/user/README.md` | change | a pointer to the new section if README indexes EXAMPLES.md content |

## Planned actions

### 8.1 - Write the "Other examples" section
Append a new `## Other examples` section to `docs/user/EXAMPLES.md` after the existing
walkthroughs. Write the intro paragraph, then the 50-row table - one row per FSM in PHASE_18.md
Groups A/B/C, each with a concise description + rationale (why selected, what it demonstrates)
and a README link. Rationale: this is the user-facing index of the catalogue.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/EXAMPLES.md` | change | the new section + 50 rows - the discovery surface for the catalogue |

### 8.2 - Verify links resolve
Run `cargo test --test docs_md_test`: it collects every repo `.md`, so the 50 new README links
must resolve and any mermaid in EXAMPLES.md must parse. Fix any broken link before finishing.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/README.md` | change | pointer to the new section (only if README indexes EXAMPLES.md) |

## Task deliverables
- The "Other examples" section in `docs/user/EXAMPLES.md` with all 50 rows, each linking to a
  resolvable README.

## Task exit criteria
- [ ] The "Other examples" section exists with exactly 50 rows (one per FSM).
- [ ] Every README link resolves (`cargo test --test docs_md_test` green).
- [ ] Each row carries a short description + rationale (why selected, what it demonstrates).
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No changes to the example projects themselves (tasks 1-7).
- No restructuring of the existing EXAMPLES.md walkthroughs - the new section is additive.
