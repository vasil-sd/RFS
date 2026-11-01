# Phase 8 Task 14 - Pointer/doc sync (START.md, KB.md, BUILD.md, docs/user/README.md)

Plan row 14 (single-line task, no subtasks): sync the repo-root pointer docs
and the external user-doc cross-link with the new documentation layout.

## Goal

Make the four pointer surfaces aware of the Phase 8 documentation sets:
`START.md` (section 2 documentation map + section 6 task->read table rows for
`docs/dev/` and `docs/llm/`), `KB.md` (documentation map + section 8 roadmap
row), `BUILD.md` (new suite row in the test-suite table + expected counts +
section 5 docs-sync checklist update), and a cross-link from
`docs/user/README.md` ("Where things live") to the developer set.

## Context

- **Doc state (measured 2026-11-01)**: START.md section 2 listed every doc
  except docs/dev/ + docs/llm/; its section 6 table had 9 task rows with no
  row pointing at the new sets. KB.md section 1 (the project-state table that
  doubles as the documentation map) had a Roadmap row saying "PHASE_0..7.md"
  and no rows for the new sets; its section 8 "Phase 3 and later" list ended
  at Phase 7. BUILD.md section 3's baseline was the Phase 7 close-out numbers
  (526/550/528) with no dev_docs_test row; section 5's checklist had no
  docs-sync bullet. docs/user/README.md "Where things live" linked examples,
  SEMANTICS, ide/, BUILD/TOOLCHAIN - not the developer sets.
- **Counts re-measured for this task** (2026-11-01): cargo test = 535/535
  root; --workspace = 559/559; --features smt = 537 (z3 env per TOOLCHAIN.md
  section 5.2); cargo test-generated-code = 1683/1683 (192 modules, unchanged
  - Phase 8 touched no specs/snippets). Examples 50 valid + 13 invalid and
  training aggregate 264 (snippets 138+128) unchanged (task 15 updates the
  training set).
- **Scope discipline**: the plan row names exactly these four files. KB.md's
  section 1 "Tests" row still carries the Phase 6 re-measured counts
  (453/477/455) - already stale before Phase 8 (BUILD.md is the authoritative
  count source and says so in that row); updating it is NOT part of this task
  and is left to task 16's close-out review.
- **Gate interaction**: docs/user/README.md is machine-parsed by
  tests/docs_md_test.rs (mdr stack) - the added rows are plain table cells,
  no new fenced blocks; START/KB/BUILD edits are prose/table only, so no
  source //! Docs: lines change and dev_docs_test is unaffected (re-run
  anyway).

## Planned actions

1. **START.md**: section 2 - add `docs/dev/` + `docs/llm/` rows (content,
   read-when, how-to-read incl. the never-fully rule + TASK_MAP locator);
   section 6 - add three task rows (locate a feature -> TASK_MAP; before
   editing source -> CHANGE_IMPACT + dev_docs_test verify; deep-dive a
   subsystem -> docs/dev fragments).
2. **KB.md**: section 1 - refresh the Roadmap row (PHASE_0..8, status reports
   0-7, per-task files 3-8) + add Developer-docs (Class A) and LLM-assistant
   docs (Class B) rows; section 8 - add the Phase 8 IN PROGRESS bullet
   (tasks 1-14 done as of 2026-11-01, close-out is task 16).
3. **BUILD.md**: section 3 - root row 526 -> 535 (+ dev_docs_test breakdown
   appended to the cell), workspace row 550 -> 559 ("the 535 above"), new
   "Doc sync gate" suite row; section 5 - new checklist bullet for docs/dev/
   docs/llm/ + //! Docs: line changes, baseline paragraph re-dated with the
   re-measured counts (535/559/537; generated 1683 unchanged).
4. **docs/user/README.md**: "Where things live" - add docs/dev/ + docs/llm/
   rows (maintainer-docs framing, point at each set's README index).
5. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5 (all
   edited .md parse), full verification (fmt/clippy/build/test root +
   workspace; counts unchanged: 535/535 root, 559/559 workspace); plan row
   14 -> DONE; TASKS.md entry; commit per action.

## Exit criteria (task 14)

- [x] START.md section 2: docs/dev/ + docs/llm/ rows added; section 6: three
      new task->read rows (TASK_MAP / CHANGE_IMPACT / docs/dev fragments).
- [x] KB.md section 1: Roadmap row refreshed to PHASE_0..8 + two new doc-set
      rows; section 8: Phase 8 IN PROGRESS bullet added.
- [x] BUILD.md section 3: counts updated (535 root / 559 workspace) with the
      dev_docs_test breakdown, new Doc-sync-gate suite row; section 5:
      docs-sync checklist bullet + baseline re-dated 2026-11-01 with
      re-measured counts (535/559/537 smt; 1683 generated unchanged).
- [x] docs/user/README.md "Where things live": cross-links to docs/dev/ +
      docs/llm/ added.
- [x] No source edits at all (docs-only task); all cargo counts unchanged
      from the task-13.3 tree (535/535 root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      fmt --check clean, clippy --all-targets exit 0, cargo test (+
      --workspace).
- [x] Plan row 14 marked DONE; TASKS.md entry appended.

## Non-goals (task 14)

- No KB.md section 1 "Tests" row count refresh (pre-existing staleness;
  BUILD.md is the authoritative count source - left to task 16's close-out
  review).
- No START.md section 1 snapshot refresh (it is a dated snapshot, not a live
  doc; task 16 owns the phase-state updates it does own).
- No DESIGN.md edits; no training-set changes (task 15); no PHASE_8_STATUS.md
  (task 16).
- No source or test changes of any kind; no new dependencies.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-11-01) for plan row 14 | Single-line plan task (no subtasks); written at execution following the one-file-per-plan-unit pattern. BUILD.md edit note: the root-package suite cell and the info_format_test cell end in byte-identical text (both describe the task-14 artifact-header sync test), so the count/breakdown update to the root row was applied line-specifically; all other edits were unique-string replacements. |
