# dev_tools maintenance protocol

This file governs the FUTURE analysis and update of the `dev_tools/` tool set.
It is a companion to (not part of) Phase 12: Phase 12 built the tools from the
first `commands.md` analysis; this protocol repeats that cycle whenever a new
accumulation of recorded assistant tool calls appears. Written 2026-11-07,
after the Phase 12 planning stage, per the user's instruction.

## Update dev_tools

**Trigger.** The user asks to analyze a new version of `commands.md` (the
repo-root log of recorded chat tool calls) and propose new dev_tools or changes
to existing ones - e.g. "a lot of new sessions have been recorded, update the
tool set". The assistant then runs the two subsections below IN ORDER, and on
their basis defines a NEW PHASE (the next free number, like Phase 12 was) with
a corresponding task set, following the two-stage protocol in
`docs/roadmap/PROTOCOL.md` (planning stage -> user review stop ->
implementation stage with per-task stops).

**Input.** The current `commands.md`. If it is a REPLACEMENT of the old file
(the old analysis is superseded), the analysis below is absolute; if it is an
APPEND/extension (new sessions added on top of what Phase 12 already analyzed),
the analysis is DIFFERENTIAL: extract the new inventory, diff it against the
Phase 12 baseline recorded in `dev_tools/README.md` section
`## Analysis of tool calls patterns`, and work from the delta. Ask the user
which case applies if it is not obvious from the file (line count vs the
12,596-call / 161,362-line Phase 12 baseline).

**Standing rules for every update cycle.**
- Read first, in order: `START.md`, `BUILD.md`, `TOOLCHAIN.md` (check
  availability of all tools, including optional ones like z3 - install missed),
  then `docs/roadmap/`: `INTRO.md`, `PRINCIPLES.md`, `FORMATTING.md`,
  `PROTOCOL.md`, `CURRENT_STATE.md`, then the current phase description if one
  exists. (This is step p1 of the original cycle.)
- Read several dozens of lines of the new `commands.md` BEFORE any extraction,
  to re-verify its entry format: `**tool call:** \`<kind>\`` entries with
  kind-specific fields (`**command:**` / `**path:**`+`**edits:**` / `**code:**`
  / `**query:**`), payloads either inline-backticked or fenced. The format may
  have EVOLVED (new tool kinds, new field names, new payload forms) - if it
  has, the extractor (`dev_tools/analysis/extract_calls.py` or its productized
  successor `rfs-dev calls extract`) must be extended FIRST, and the extension
  recorded as a plan correction. Never assume the Phase 12 format still holds.
  (Step p2.)
- The category vocabulary of `dev_tools/README.md` section
  `## Analysis of tool calls patterns` is STABLE: new data is classified into
  the existing categories first; a NEW category may be introduced only when no
  existing one fits, and must then be added to the vocabulary with its abstract
  pattern. Categories never get silently renamed or merged.
- Every update cycle ends with `dev_tools/README.md` re-synced (both sections),
  `docs/roadmap/CURRENT_STATE.md` updated for the new phase, and a `TASKS.md`
  entry per its first-paragraph template.

### Analysis of tool calls patterns

The analysis half of the cycle (original steps p3, split into smaller tasks
because `commands.md` is huge - keep that split):
1. **Extract** the structured call inventory: one row per call
   (`ordinal, kind, payload, paths[]` with docs/sources/scratch/other class
   labels). Use `rfs-dev calls extract` if Phase 12 shipped it, else the
   committed `dev_tools/analysis/extract_calls.py`. The extractor's per-kind
   count summary must be printed and sanity-checked against the raw file
   (`grep -c '^\*\*tool call:\*\*' commands.md`) before anything else.
2. **Classify** per call kind, one task per kind (terminal / edit_file /
   python / misc), each emitting its TSV (`category, count, abstract_pattern,
   examples`) under `dev_tools/analysis/`. Work on the inventory JSONL, never
   on raw-file chunks (spot-checks against the raw file are the validation).
3. **Diff** (differential case only): per category - old count vs new count vs
   delta; new categories; vanished categories; new abstract patterns inside an
   existing category. The diff, not the absolute table, is the decision input
   for the proposal half.
4. **Consolidate**: update `dev_tools/README.md` section
   `## Analysis of tool calls patterns` - master category table (counts must
   sum to the total call count), per-kind subtables, and the "what this means
   for the tool set" paragraph rewritten against the new data. Freeze the
   section before the proposal half starts.

### Proposal of new dev_tools

The proposal half of the cycle (original steps p4-p5):
1. **Decide per operation class** whether the evidence demands action, using
   the Phase 12 criteria: an operation class becomes a tool candidate only if
   it is (a) mechanical (no design judgment in the individual call),
   (b) repeated (the category count is large or growing), and (c)
   token-expensive or error-prone when done by hand (chunked reads, pattern-
   fragile rewrites, reconstructed session scripts). One-off or judgment-heavy
   categories are recorded as "no tool, reason: ..." - absence of a tool is a
   decision, not an oversight.
2. **Propose**: for each candidate - a NEW tool (new row in `## Proposed
   tools`) or a CHANGE to an existing one (extend its CLI / purpose / rationale
   in place; the table row is updated, never duplicated). Table shape is fixed:
   `| Tool name | CLI | Purpose | Detailed description and rationale |`.
   Each rationale names the concrete recorded operations it replaces (category
   + counts from the frozen analysis section) - a proposal row that cites no
   analysis category is incomplete.
3. **Define the new phase**: write `docs/roadmap/phase_N/PHASE_N.md`
   (description: goal, scope = the proposed delta, deliverables, exit criteria,
   non-goals - per FORMATTING.md section 2), then run the PLANNING STAGE of
   `docs/roadmap/PROTOCOL.md` section 2 to produce `PHASE_N_PLAN.md` with the
   task set: analysis tasks only if the differential data still needs
   classification work, proposal elaboration if step 2 above was not finished
   in this session, and one implementation task per proposed tool/change plus
   a close-out task (verification + docs sync + status report). STOP for user
   review after the plan (protocol step i); write the per-task files only after
   confirmation (step j) and STOP again (step k).
4. **Implement** per the implementation stage (`docs/roadmap/PROTOCOL.md`
   section 3): one task at a time, each starting only after explicit user
   confirmation; every implemented tool gets its demonstration on a real
   maintenance operation as acceptance test, exactly as Phase 12 required.
