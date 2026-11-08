# Phase 12, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Write the phase's design deliverable: `dev_tools/README.md` section
`## Proposed tools` - the table `| Tool name | CLI | Purpose | Detailed
description and rationale |` covering every operation class the frozen analysis
shows to be mechanical, repeated and token-expensive. Rationale: this table is
the contract task 5 implements against - each row must be traceable to an
analysis category with counts, and its CLI must match the dispatch stubs fixed
in task 1.

## Context
- Inputs (read-only): the FROZEN `## Analysis of tool calls patterns` section
  (task 3.5) + `PHASE_12.md` scope item 3 (the five minimum categories) +
  task 1's pinned subcommand list. The analysis section is never edited here.
- Tool-criteria (from dev_tools/PROTOCOL.md, the standing rule): an operation
  class becomes a tool row only if it is (a) mechanical - no design judgment in
  the individual call; (b) repeated - large or growing category count; (c)
  token-expensive or error-prone by hand - chunked reads, pattern-fragile
  rewrites, reconstructed session scripts. Categories failing a criterion are
  recorded in the section as "no tool, reason: ..." - absence is a decision.
- Minimum rows (PHASE_12.md scope item 3), mapped to task 1's stubs:
  - `sync-check` / `sync-fix` - docs<->sources sync: `//! Docs:` pointer lines,
    `tests/dev_docs_manifest.txt`, every backticked source-path reference in
    `docs/**/*.md` vs the actual tree; compact fix-list output (replaces the
    Phase 11 task-10 scripted path mapping over 1,794 stale references).
  - `context <name> [--file F] [--depth N]` - inventory row + Depends closure
    (depth-bounded) + Documented-in fragment content + related DECISIONS.md
    entries, as a fixed markdown template (operationalizes the PROTOCOL.md
    section-4 impact-calculation rule; replaces the manual grep+sed+read
    sequence behind the lookup-sources category).
  - `edit-rename <old> <new>`, `edit-move <from> <to>`,
    `edit-replace-item <file> <item>` - syn-parsed structured edits immune to
    strings/comments containing the pattern (replaces the 231 python
    bulk-rewrite + 51 shell-edit calls - Phase 10's template-phantom failure
    class).
  - `doc-section <file|all> <heading...>` - section content by heading path,
    optionally across all docs (replaces the ~4,480 lookup-docs chunked reads;
    pulldown-cmark-backed).
  - `inventory add|update|remove|audit` - SRC/TEST_STRUCTURE.md row
    maintenance keeping tables mechanically valid + the string-masked
    completeness audit built in (replaces the Phase 10 close-out audit that
    had to be RECONSTRUCTED in Phase 11 because its script was session-local).
  - `calls extract|classify` - productizes tasks 2-3 for PROTOCOL.md re-runs
    (one command instead of a committed python script + manual runs).
- CLI conventions for every row: compact structured stdout (tables/JSON, never
  raw file dumps); exit codes 0 success / 1 operational failure / 2 usage error
  (rfs CLI convention); `--root` defaults to the repo root; mutating tools
  (`sync-fix`, `edit-*`, `inventory add/update/remove`) take `--dry-run`
  defaulting to OFF but printing the exact edit plan first.
- OPEN DECISION owned by this task: whether `DOCS_STRUCTURE.md` (section-level
  docs inventory) is built. Decide from the frozen data: if lookup-docs calls
  show frequent cross-file section hunting (same heading sought in several
  files), build it as a task-5 subtask; if lookups are single-file range reads,
  on-the-fly heading walks suffice and the inventory is declined with that
  reason recorded. The decision + data rationale go into the section text.
- Consistency obligation: the subcommand names in this table MUST equal task 1's
  pinned stub list. If elaboration reveals a better name/shape, do NOT edit
  task 1's stubs here - record a plan correction naming both the README change
  and the stub change; task 5 implements the corrected surface.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3.5 (inside task 3) | Consolidate the analysis | the frozen `## Analysis of tool calls patterns` section is the sole evidence input - every row cites it |
| 1. | Scaffold the dev_tools crate | the pinned stub list is the CLI surface this table must match |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | Implement the proposed tools + tests | implements exactly this table - one subtask per tool group; the DOCS_STRUCTURE.md decision determines whether 5.4 grows a subtask |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/README.md` | change | fill `## Proposed tools` (placeholder from task 1) - the only edit of this task; the analysis section above it is untouched |

## Planned actions

### Action 1: derive the tool rows from the frozen evidence
Re-read the frozen analysis section + PHASE_12.md scope item 3. For each
operation class, apply the three criteria (mechanical / repeated /
token-expensive-or-error-prone) and draft its table row: Tool name (must match
a task-1 stub), CLI (full flag shape per the Context conventions), Purpose
(one line), Detailed description and rationale (what it does, how - syn /
pulldown-cmark/audit mechanics where relevant - and WHICH recorded operations
it replaces, citing category names + counts from the frozen section). Write
the "no tool, reason: ..." lines for classes that fail a criterion (expected:
conversation-recall, web-docs-lookup, git, env-check). Rationale: row-by-row
evidence citation is what makes the proposal auditable - a row citing no
category is incomplete and gets rejected at review.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/README.md` | change (`## Proposed tools` section only) | the full table + no-tool decisions written - the phase's design deliverable, frozen for task 5 to implement against |

### Action 2: decide DOCS_STRUCTURE.md and check CLI consistency
From the lookup-docs subtable data, take the DOCS_STRUCTURE.md decision (build
as a task-5 subtask / decline with reason) and write it into the section. Then
diff the table's subcommand names against task 1's pinned stub list; any
mismatch becomes a plan correction entry (naming the README row and the stub),
never a silent rename. Rationale: the decision is data-owned (not taste-owned)
and the consistency check closes the loop between the fixed CLI surface and
the elaborated proposal.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/README.md` | change (`## Proposed tools` section only) | DOCS_STRUCTURE.md decision paragraph appended - completes the section |

## Task deliverables
- `dev_tools/README.md` section `## Proposed tools` filled: the full table
  (>=6 tool rows covering the five scope categories + calls), no-tool decisions
  with reasons, the DOCS_STRUCTURE.md decision with data rationale.
- Plan-correction entries for any CLI-surface mismatch (if found).

## Task exit criteria
- [ ] The table covers all five PHASE_12.md scope categories + `calls`
      (or a documented reason for omitting one).
- [ ] Every row's rationale cites >=1 frozen-analysis category with its count;
      every no-tool decision states the failed criterion.
- [ ] Subcommand names match task 1's stub list exactly (or plan corrections
      recorded for each delta).
- [ ] The DOCS_STRUCTURE.md decision is written with the deciding data cited.
- [ ] The `## Analysis of tool calls patterns` section is byte-identical to its
      frozen state (no edits leaked into it).

## Non-goals
- No implementation - task 5 builds what this table specifies.
- No edits to the frozen analysis section, to task 1's stubs, or to any other
  file.
- No new tool categories beyond the evidence: a row without a cited category
  does not get written.
