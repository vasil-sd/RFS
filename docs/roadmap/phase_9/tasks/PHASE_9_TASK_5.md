# Phase 9, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Finish the KB.md cleanup: update its stale header, replace the outdated
"Updated roadmap" section with a pointer to docs/roadmap/, remove the three
explicitly superseded historical sections (9, 12, 13), renumber the two
surviving tail sections, and fix the four stale cross-references in other
documents that point at moved/removed KB content. Rationale: these sections
are outdated information that duplicates the roadmap status files and DESIGN
sections 19-22; wrong section pointers mislead every future session.

## Context
Verified at planning:
- **KB header (lines 1-28)** is stale: "Status: **2026-10-13**"; the summary
  list describes the OLD structure (items 3 "updated roadmap" and 7 "detailed
  next R&D plan" no longer match); the read order starts with KB.md itself,
  but `START.md` is the session entry point now. Rewrite: current status date,
  a summary list matching the NEW structure (1 project state; 2 design
  decisions - references to DESIGN.md; 3 syntax reference - reference to
  DESIGN.md section 5; 4 source code map; 5 build/test/run pointers; 6 agent
  workflow experience; 7 current limitations; 8 roadmap pointer; 9 quick-start
  pointer; 10 rules for future work), and a read order starting from START.md.
- **KB section 8 "Updated roadmap"** duplicates
  `docs/roadmap/CURRENT_STATE.md` and is stale (it says Phase 8 "IN PROGRESS
  (tasks 1-14 done as of 2026-11-01; close-out is task 16)" - Phase 8 closed
  2026-11-01 per CURRENT_STATE.md). Replace the whole section body with a short
  pointer: the authoritative roadmap is `docs/roadmap/` (INTRO.md index +
  CURRENT_STATE.md phase-state table + per-phase directories); post-v1
  candidates are listed in `docs/roadmap/PRINCIPLES.md` section 3. Keep the
  heading "## 8. Updated roadmap" or rename it to "## 8. Roadmap pointer" -
  either is fine, no live doc cites KB section 8 by number (re-verify with
  grep).
- **KB section 9 "Historical R&D plan (Phase 0 -> Phase 1)"** carries its own
  "Superseded" banner; the actual outcomes are in
  `docs/roadmap/phase_1/PHASE_1_STATUS.md` and DESIGN sections 19-22. REMOVE.
- **KB section 12 "What to work on next"** is bannered "Obsolete as of Phase 6
  task 9". REMOVE (its pointer target, docs/roadmap/CURRENT_STATE.md, is already
  covered by the new section 8 body).
- **KB section 13 "Phase 1 code generation and bounded model checking"** is
  bannered "Superseded (Phase 4)"; the real decisions are DESIGN sections
  19-22 + CODEGEN.md. REMOVE.
- **Renumbering**: after removing 9/12/13, the surviving tail sections are
  "## 10. Quick-start for a new agent session" and "## 11. Rules for future
  LLM-assisted work" -> renumber to 9 and 10. Reference audit done at planning:
  exactly ONE live reference exists - `START.md:255` "## 8. Workflow rules
  (condensed from KB.md section 11)" -> update it to "section 10" as part of
  this task (task 8 does the larger START.md map updates). No references to
  KB sections 9/12/13 exist outside TASKS.md history and the roadmap status
  files (append-only / frozen - never edited); re-verify with grep at
  execution time.
- **Stale cross-references to fix** (exact texts located at planning):
  - `docs/dev/DECISIONS.md` ~line 143: "The eight bullets of DESIGN.md section
    19 (full text there; recorded in detail in KB.md section 13):" -> replace
    "recorded in detail in KB.md section 13" with "recorded in detail in the
    Phase 1 status report, `docs/roadmap/phase_1/PHASE_1_STATUS.md`" (KB 13 is
    gone; the detailed record it pointed at is the status file). Note: task 6
    will later linkify this line's document names - fix the TARGET now so the
    link points at the right place.
  - `docs/dev/toolchain_fuzz.md` ~lines 39, 164, 199 (three places): "Current
    training-set counts ... live in KB.md section 3" / "Current counts: KB.md
    section 3." / "(current counts: KB.md section 3)" - WRONG section (training
    counts are in KB section 1's state table and BUILD.md). Replace "section
    3" with "section 1" in all three.
  - `docs/llm/toolchain_fuzz.md` ~line 77: "current training counts: KB.md
    section 3." -> "section 1". THIS FILE IS CLASS B WITH A MACHINE-CHECKED
    LINE BUDGET (`tests/dev_docs_test.rs`): use a same-line word replacement
    only - never rewrap or add/remove lines.
- **Training-set check**: grep `training/` for quotes of the removed KB
  sections ("section 13", "Historical R&D plan"); none were found at planning -
  if a hit appears, record it as a task correction instead of editing training
  data silently.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | KB->DESIGN dedup: purpose/semantics/constraints/diagram | section 2 stubs must exist before the header's summary list can describe the final structure |
| 3. | KB->DESIGN dedup: syntax reference | same - section 3 stub first |
| 4. | KB->DESIGN dedup: IR/parser/property/codegen | same - all of section 2 stubbed before renumbering/removals |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | docs/dev clickable links: DECISIONS.md | this task fixes the "KB.md section 13" target in DECISIONS.md first; task 6 then linkifies the corrected line |
| 7. | docs/dev clickable links: remaining files | same for toolchain_fuzz.md (dev) - its "section 1" text is fixed here, linkified there |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `KB.md` | change | header rewritten; section 8 -> roadmap pointer; sections 9/12/13 removed; sections 10/11 renumbered to 9/10 |
| `docs/dev/DECISIONS.md` | change | one citation fixed: "KB.md section 13" -> Phase 1 status report |
| `docs/dev/toolchain_fuzz.md` | change | three "KB.md section 3" training-count pointers -> "section 1" |
| `docs/llm/toolchain_fuzz.md` | change | one same-line "section 3" -> "section 1" (Class B budget preserved) |
| `START.md` | change | line ~255: "condensed from KB.md section 11" -> "section 10" |

## Planned actions

### Action 1: Rewrite the KB header
Update the status date to the execution date, replace the summary list with
the new-structure list (per Context), and make the read order start from
`START.md` (then this file's section 4 source map for "how is X implemented",
`DESIGN.md` for core design, `docs/roadmap/INTRO.md` for phase work). Keep the
"Audience" line. Rationale: the header is the file's table of contents - it
must describe the post-dedup structure.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (lines 1-28) | status date, summary list, and read order actualized to the new structure with START.md as entry point |

### Action 2: Replace section 8 with a roadmap pointer
Delete section 8's per-phase status text (stale - duplicates CURRENT_STATE.md)
and write the short pointer body from Context. Rationale: one authoritative
phase-state table, in docs/roadmap/.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 8, ~lines 1169-1225) | stale duplicate of CURRENT_STATE.md -> pointer to docs/roadmap/ + PRINCIPLES.md section 3 for post-v1 candidates |

### Action 3: Remove sections 9, 12, 13 and renumber 10/11
Delete the three bannered-superseded sections in full. Renumber "## 10.
Quick-start..." -> "## 9." and "## 11. Rules for future LLM-assisted work" ->
"## 10.". Before deleting, grep the repo (excluding TASKS.md and
`docs/roadmap/phase_*/` frozen files) for references to KB sections 9/12/13
and to "KB.md section 1[01]"; the only expected hit is START.md:255 (fixed in
action 4). Rationale: superseded history already lives in TASKS.md + roadmap
status files + DESIGN 19-22; renumbering keeps the file contiguous.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 9, ~lines 1226-1462) | removed - explicitly superseded historical R&D plan; substance in docs/roadmap/phase_0..1 status files + DESIGN 19 |
| `KB.md` | change (section 12, ~lines 1484-1498) | removed - bannered obsolete; pointer target covered by new section 8 body |
| `KB.md` | change (section 13, ~lines 1499-1580) | removed - explicitly superseded by Phase 4; substance in DESIGN 19-22 + CODEGEN.md |
| `KB.md` | change (sections 10/11 headings, ~lines 1463, 1469) | renumbered to 9 and 10 after the removals |

### Action 4: Fix the stale cross-references
Apply the four reference fixes from Context (DECISIONS.md citation;
toolchain_fuzz dev x3; toolchain_fuzz llm x1 same-line) plus the START.md:255
"section 11" -> "section 10" update. Rationale: every pointer must land on
content that still exists and says what the pointer claims.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/DECISIONS.md` | change (~line 143) | "recorded in detail in KB.md section 13" -> the Phase 1 status report (KB 13 removed by action 3) |
| `docs/dev/toolchain_fuzz.md` | change (~lines 39, 164, 199) | three training-count pointers re-targeted KB section 3 -> section 1 (where the counts actually live) |
| `docs/llm/toolchain_fuzz.md` | change (~line 77) | same-line "section 3" -> "section 1"; Class B line budget untouched |
| `START.md` | change (~line 255) | "condensed from KB.md section 11" -> "section 10" (renumbered by action 3) |

### Action 5: Training-set grep + gate run
Grep `training/` for quotes of removed KB sections (expected: none). Run
`cargo test --test docs_md_test --test dev_docs_test --test training` to prove
the llm-card budget and all doc gates still hold. Rationale: the Class B
budget is machine-checked; the training suites guard against quoted-doc drift.

#### Affected files

None (grep + test runs only).

## Task deliverables
- KB.md with a current header, a roadmap-pointer section 8, no sections
  9/12/13, and contiguous numbering 1-10.
- Four stale cross-references fixed (DECISIONS.md, toolchain_fuzz dev x3,
  toolchain_fuzz llm x1) + the START.md:255 renumber reference.

## Task exit criteria
- [ ] KB header: current date, summary list matching sections 1-10 as they
      exist after this task, read order starting from START.md.
- [ ] KB section 8 body is a pointer to docs/roadmap/ (no per-phase status
      text, no "Phase 8 IN PROGRESS").
- [ ] KB sections 9/12/13 are gone; the file's last section is "## 10. Rules
      for future LLM-assisted work"; no numbering gaps.
- [ ] Grep-verified: no live reference (outside TASKS.md + frozen roadmap
      files) to KB sections 9/12/13 or to old numbers 10/11 remains.
- [ ] DECISIONS.md cites the Phase 1 status report, not "KB.md section 13";
      all four toolchain_fuzz pointers say "KB.md section 1"; the llm card's
      line count is unchanged.
- [ ] `cargo test --test docs_md_test --test dev_docs_test --test training`
      passes.

## Non-goals
- No edits to TASKS.md (append-only) or to frozen `docs/roadmap/phase_*/`
  files, even where they cite old KB section numbers.
- No changes to KB sections 1-7 content beyond what tasks 2-4 already did.
- No source/test/training-data changes (a training hit would become a task
  correction, not a silent edit).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- If the grep in action 3 finds an UNEXPECTED live reference to a removed
  section, fix that reference too and list it in the status file - do not
  leave a dangling pointer.
