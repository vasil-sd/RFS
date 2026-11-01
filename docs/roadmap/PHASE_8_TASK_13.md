# Phase 8 Task 13 - Class B consolidation (TASK_MAP, CHANGE_IMPACT, cards)

Plan rows: 13.1 (full docs/llm/TASK_MAP.md), 13.2 (docs/llm/CHANGE_IMPACT.md +
docs/llm/README.md budget rules), 13.3 (final verification/budget pass over all
11 per-module cards). One task file per plan group; subtask sections are
appended at execution.

## Subtask 13.1 - Full functionality-to-source map (docs/llm/TASK_MAP.md)

### Goal

Rewrite `docs/llm/TASK_MAP.md` from its growing stub into the full Class B
task map: functionality -> files/functions for EVERY CLI command, subsystem,
and test suite, as the "read this fragment for this task" index extending
`START.md` section 6. The sync-gate coverage check (tests/dev_docs_test.rs,
check_task_map_coverage) activates on the file's existence and requires every
manifest-listed source file to appear as a doc->code reference - the existing
flat Coverage list already satisfied that; this task restructures it into the
functionality map while keeping full coverage.

### Context

- **Doc state**: `docs/llm/TASK_MAP.md` was a 95-line skeleton with budget
  400: an intro, the `llm:task-map` anchor + a `## Map` section whose TBD
  (task 13.1) promised "the full functionality-to-source map over all 72 hand-
  written files + every test suite", and a `## Coverage (growing stub)`
  section holding the flat one-line-role list of all manifest files appended
  by chunks 1.3-1.10.
- **Gate mechanics (verified in tests/dev_docs_test.rs)**:
  check_task_map_coverage scans the file for doc->code references and fails if
  any path from tests/dev_docs_manifest.txt is missing; the manifest holds
  EXACTLY 71 files (the skeleton's "72" was an approximation - corrected in
  the new Coverage header). A doc->code reference is a backticked span whose
  path part contains a slash and ends in .rs/.c/.h, optionally with ::item;
  bare filenames without a directory component are prose, not references.
  The Class B budget check requires the `<!-- llm-budget: N -->` header within
  the first 10 lines and the file to stay within it (400 here).
- **Fragment inventory (measured 2026-11-01)**: 87 dev fragments across 19
  Class A docs + 12 llm fragments; every "read fragment" pointer in the new
  map was checked against this inventory. llm:change-impact already exists as
  an anchor in docs/llm/CHANGE_IMPACT.md (its content is task 13.2) - the map
  points there for pre-edit impact checks.
- **START.md section 6** is the Task -> read/do/verify table (9 rows); the new
  map extends it as a locator (the plan row says "extending START.md section
  6") and explicitly defers workflow authority to that table. No START.md
  edits in this subtask (pointer sync = task 14).
- **Test-suite inventory**: 24 root test files; each got a pins/fragments row
  (verified against module docs: integration_example_test.rs = Phase 2 exit-
  criterion driver; library_comparison.rs = the nom-vs-chumsky decision
  record).

### Planned actions

1. **Rewrite `docs/llm/TASK_MAP.md`** in place (budget header + anchor stay
   byte-stable): new "How to use" intro; `## Map` with six functionality
   tables - CLI commands (8 subcommands -> dispatch fn + logic module + read
   fragment), frontend core (parser/validator/IR/JSON/printer/errors/spec
   hash), codegen (registry + shared plans + both backends), model-checking
   engines + monitor compiler, workspace crates, fuzz + training + code-test
   harness; `## Test suites` table (24 rows: what each pins + related
   fragments); `## Coverage` header rewritten ("all 71 manifest files", stub
   language removed) keeping the existing 71 one-line-role bullets as the
   gate's coverage list.
2. **Verification + bookkeeping**: dev_docs_test 9/9 (coverage check now
   active over the full map), docs_md_test 5/5, full verification (fmt/
   clippy/build/test root + workspace; counts unchanged: 535/535 root,
   559/559 workspace); plan row 13.1 -> DONE; check off this subtask's boxes;
   TASKS.md entry; commit per action.

### Exit criteria (subtask 13.1)

- [x] `docs/llm/TASK_MAP.md`: Map TBD replaced with the full functionality-to-
      source map (every CLI command, subsystem, and test suite at file/
      function granularity with read-fragment pointers); Coverage section
      rewritten as the complete 71-file manifest list; no TBD remains; within
      the 400-line budget (final: 192 lines); anchor + budget header byte-
      stable.
- [x] Sync-gate coverage check passes with the file active: all 71 manifest
      files appear as doc->code references; every reference resolves (gate
      check 1).
- [x] No source edits at all (docs-only subtask); all cargo counts unchanged
      from the task-12.3 tree (535/535 root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      fmt --check clean, clippy --all-targets exit 0, cargo test (+
      --workspace).
- [x] Plan row 13.1 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 13.1)

- No CHANGE_IMPACT.md content (task 13.2) - the map points to the existing
  llm:change-impact anchor only.
- No per-module card rewrites (task 13.3's budget pass).
- No START.md / KB.md / BUILD.md edits (pointer sync = task 14); no DESIGN.md
  edits.
- No source or test changes of any kind; no new dependencies.
- No line-number-based anchors anywhere; stable item-name references only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-11-01) for plan group 13 | User model: one task file per plan group ("execute 13.1" = subtask of task file 13). Written for subtask 13.1 only; subtasks 13.2 (CHANGE_IMPACT + README budget rules) and 13.3 (final card budget pass) will be appended at execution, following the PHASE_8_TASK_9.md / PHASE_8_TASK_11.md pattern. |
| Skeleton's "72 hand-written files" corrected to 71 | tests/dev_docs_manifest.txt holds exactly 71 entries (verified by count); the new Coverage header states 71. The skeleton's number was an approximation from task 1.x. |
