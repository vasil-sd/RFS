# Phase 8 Task 15 - Training-set update (documentation layout + Class B workflow)

Plan row: 15 (single-line task, no subtasks). One task file per plan group;
task files are created at execution.

## Task 15 - Training-set update

### Goal

Add `training/project_tooling/rfs_training.jsonl` entries on the Phase 8
documentation layout and how an LLM session uses the Class B docs (TASK_MAP
fragment -> locate code -> CHANGE_IMPACT -> doc updates; sync-gate behavior),
then regenerate snippets (`cargo extract-training-snippets`) and the aggregate
(`cargo build-training` -> `all_rfs_training.jsonl`).

### Context

- **Category state (measured 2026-11-01)**: project_tooling had 6 entries,
  none with ```rfs fences, and an EMPTY snippets dir (0 valid + 0 invalid) -
  so new fence-free entries leave the snippet counts untouched (138 valid +
  128 invalid overall) and only grow the aggregate.
- **Format rules** (KB.md section 4.6 + src/training.rs::validate_training_line):
  exactly 2 messages (user then assistant), non-empty user content, assistant
  in `< think> ... < /think>` chain-of-thought format (spaced tags as pinned).
- **Content sources** (all verified against the tree): docs/llm/README.md
  (the workflow + rules), docs/llm/TASK_MAP.md (structure),
  docs/llm/CHANGE_IMPACT.md (17-row table), tests/dev_docs_test.rs (the five
  gate checks + negative controls), tests/dev_docs_manifest.txt (71 files).
- **Count-sync precedent**: Phase 7 task 15 updated the KB.md aggregate count
  when it changed; this task does the same for 264 -> 267 in KB.md section 1
  and BUILD.md sections 4 + 5 (three references).

### Planned actions

1. **Three new project_tooling entries** (appended, fence-free):
   - the two documentation sets (docs/dev Class A: 18 files, stable fragment
     anchors, never-fully; docs/llm Class B: TASK_MAP / CHANGE_IMPACT /
     README + 11 budgeted cards; link-only relationship);
   - the LLM-session workflow over Class B (TASK_MAP -> read exactly the named
     fragments via grep anchor + sed range -> CHANGE_IMPACT before editing ->
     listed tests + dev_docs_test after doc-affecting edits; budget discipline);
   - the sync gate (five checks: code->doc pointers over the 71-file manifest,
     doc->code reference resolution, fragment definition/uniqueness/ownership
     with no orphans, TASK_MAP coverage, Class B line budgets + negative-
     control fixtures; failure output IS the update list; the three rules of
     thumb for new fragments / new tracked files / reference shape).
2. **Regenerate**: `cargo extract-training-snippets` (138/128 unchanged) +
   `cargo build-training` (aggregate 264 -> 267).
3. **Count sync**: KB.md section 1 LLM-training-set row (267 + the Phase 8
   note, preserving the Phase 7/6 history chain) + BUILD.md section 4 command
   row and alias re-verification note + section 5 baseline (267 lines).
4. **Verification + bookkeeping**: training tests + both doc gates + full
   suite; plan row 15 -> DONE; this task file; TASKS.md entry; commits per
   action.

### Exit criteria (task 15)

- [x] `training/project_tooling/rfs_training.jsonl`: 6 -> 9 entries; all three
      new entries pass the pinned format (2 messages, roles, `< think> ... < /think>`
      assistant) - proven by `cargo test-training` (validate_training_line runs
      over every line during aggregation).
- [x] Snippets regenerated: 138 valid + 128 invalid (unchanged - no new rfs
      fences); project_tooling snippets dir still empty by construction.
- [x] Aggregate rebuilt: all_rfs_training.jsonl 264 -> 267 lines; freshness
      test passes (aggregate in sync with the category files).
- [x] Documented counts synced: KB.md section 1 + BUILD.md sections 4/5 now
      say 267 (three references); no other stale count introduced.
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5,
      cargo test-training 5 pass, fmt clean, clippy exit 0, root 535/535,
      workspace 559/559 (unchanged from the task-14 tree).
- [x] Plan row 15 marked DONE; this task file written; TASKS.md entry
      appended.

### Non-goals (task 15)

- No changes to other training categories, training.py, or the GPU pipeline.
- No new ```rfs fences in the new entries (they are about documentation and
  workflow, not RFS syntax - keeping snippet counts stable was deliberate).
- No DESIGN.md/START.md edits (task 14's scope); no code changes of any kind.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-11-01) for plan row 15 | Single-line plan task (no subtasks): one section per the house pattern; file created when executed, like PHASE_8_TASK_14.md. |
| Count syncs in KB.md + BUILD.md added to scope | The plan row lists only the training files as changes, but the aggregate count (264) is documented in three places and Phase 7 task 15 set the precedent of updating it with the training change; leaving it stale would contradict the re-measured baseline written by task 14 hours earlier. Recorded here for traceability. |