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

## Subtask 13.2 - CHANGE_IMPACT.md + README budget rules (added at execution, see Task corrections)

### Goal

Fill in `docs/llm/CHANGE_IMPACT.md` (Class B): change type -> affected source
files + tests + doc fragments at file granularity, mirroring and extending the
BUILD.md section 5 checklist - and finalize `docs/llm/README.md`'s context-
budget rules (complete file/budget index for all 14 Class B files + the full
machine-checked rule set).

### Context

- **Doc state**: `docs/llm/CHANGE_IMPACT.md` was an 11-line skeleton with
  budget 250 - intro + `llm:change-impact` anchor + a `## Impact table`
  section whose TBD (task 13.2) named the change-type rows (grammar, IR
  schema, codegen, backends, MC engine, monitor, CLI, diagrams, tooling).
  `docs/llm/README.md` was a 29-line file with budget 70: a 3-row index that
  said "per-module cards (11 files)" without naming them, the 4-step workflow,
  and a 3-item rule list.
- **Source of truth for the rows** (verified 2026-11-01): BUILD.md section 5
  (the conditional-check checklist + known-good baseline), START.md section 6
  (task -> read/do/verify conventions: examples-first for grammar/validator
  changes, alias commands), the 87 dev + 12 llm fragment inventory (every
  "fragments to update" pointer checked against it), and the per-module deep
  dives completed in tasks 2-12 (parity-by-construction facts: MC engine
  twins, monitor plan rendering, backend wire formats).
- **Class B file/budget census** (measured): README 70, TASK_MAP 400,
  CHANGE_IMPACT 250, and the 11 module cards - parser 120, validator 120,
  ir_json_rw 150, codegen_core 150, backend_rust 150, backend_c 150,
  mc_engine 150, monitor_compiler 120, cli_commands 200, workspace_crates
  200, toolchain_fuzz 200.
- **Gate mechanics**: check_llm_budgets enforces the header-within-first-10-
  lines + within-budget rules for every docs/llm/*.md; no other gate reads
  these two files' content (the llm:change-impact anchor already existed and
  is referenced by TASK_MAP.md + the module cards - it stays byte-stable).

### Planned actions

1. **`docs/llm/CHANGE_IMPACT.md`**: replace the Impact-table TBD in place
   (anchor stays byte-stable) with a "How to use" intro + a 17-row table:
   RFS syntax/grammar, IR schema/wire format, parser internals, validator
   checks, codegen core/plugin API, Rust backend, C backend, MC engine
   (exact/liveness), SMT bounded mode, monitor compiler, CLI command,
   diagram format, external-tool layer, workspace crates, fuzz/training/code-
   test harness, spec generation, and the UNIVERSAL row (full checklist +
   dev_docs_test + TASKS.md entry). Each row: affected source files at file
   granularity, concrete test commands (mirroring BUILD.md section 5), doc
   fragments to update (Class A + Class B + repo-root docs where content must
   move).
2. **`docs/llm/README.md`**: finalize the index - add a Budget column, name
   all 11 module cards with their budgets; complete the rule set - budget
   header format, fragment location via grep on the anchor ID, no Class A
   duplication, and the TASK_MAP manifest-coverage duty (extend
   tests/dev_docs_manifest.txt + the //! Docs: line when adding a tracked
   .rs file).
3. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (fmt/clippy/build/test root + workspace; counts unchanged:
   535/535 root, 559/559 workspace); plan row 13.2 -> DONE; check off this
   subtask's boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 13.2)

- [x] `docs/llm/CHANGE_IMPACT.md`: Impact-table TBD replaced in place with the
      full 17-row change-type table at file granularity (anchor byte-stable);
      no TBD remains; within the 250-line budget (final: 36 lines).
- [x] `docs/llm/README.md`: index finalized (all 14 Class B files with
      budgets) + rule set completed (budget header format, fragment location,
      no duplication, manifest-coverage duty); within the 70-line budget
      (final: 39 lines).
- [x] No source edits at all (docs-only subtask); all cargo counts unchanged
      from the task-13.1 tree (535/535 root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      fmt --check clean, clippy --all-targets exit 0, cargo test (+
      --workspace).
- [x] Plan row 13.2 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 13.2)

- No per-module card rewrites (task 13.3's final budget pass).
- No START.md / KB.md / BUILD.md edits (pointer sync = task 14); no DESIGN.md
  edits.
- No source or test changes of any kind; no new dependencies.
- No line-number-based anchors anywhere; stable item-name references only.

## Subtask 13.3 - Final verification/budget pass over the 11 cards (added at execution, see Task corrections)

### Goal

Per the plan correction (2026-10-30): the 11 per-module Class B cards were
filled by their matching deep-dive tasks (8.1-12.3), so 13.3 is the FINAL
verification/budget pass over all of them now that tasks 2-12 are complete -
confirm every card stays within its line budget, matches the final Class A
content (one-line roles, key items, invariants, deep-dive pointers), and
carries no stale forward-references to since-completed tasks.

### Context

- **Card census (measured 2026-11-01)**: parser.md 53/120, validator.md
  43/120, ir_json_rw.md 56/150, codegen_core.md 47/150, backend_rust.md
  56/150, backend_c.md 58/150, mc_engine.md 56/150, monitor_compiler.md
  54/120, cli_commands.md 77/200, workspace_crates.md 77/200,
  toolchain_fuzz.md 78/200 - all within budget (the gate enforces this;
  check_llm_budgets passed).
- **Pass design**: each card was written in the SAME task as its deep dive,
  so content drift can only come from (a) forward-references to tasks that
  were pending when the card was written but are now complete, and (b)
  pointers/counts that later tasks invalidated. The pass therefore: (1)
  audits every dev:/llm: fragment token in the whole Class B set against the
  defined-anchor inventory; (2) verifies each card's Deep-dive section names
  exactly its doc's fragments; (3) checks cited test suites + cargo aliases
  exist; (4) scans for stale numeric claims.

### Planned actions

1. **Fragment-pointer audit** over all 14 Class B files: every referenced
   dev:/llm: token must resolve to a defined anchor; cross-doc references
   must point at the right home doc.
2. **Deep-dive section completeness**: each card's Deep-dive paragraph must
   name exactly the fragments defined in its Class A doc (no omissions, no
   stale IDs).
3. **Stale forward-reference sweep**: remove "(task N.N)" annotations on
   pointers to now-completed tasks (11.2, 12.3, 13.2); keep none - all plan
   groups 8-13 are done except 14/15/16 which no card references.
4. **Cited-command audit**: every tests/*.rs suite and cargo alias named in
   the cards must exist (tests dir + .cargo/config.toml).
5. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (fmt/clippy/build/test root + workspace; counts unchanged:
   535/535 root, 559/559 workspace); plan row 13.3 -> DONE; check off this
   subtask's boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 13.3)

- [x] All 11 cards verified within their line budgets (gate: check_llm_budgets
      green; census recorded above).
- [x] Fragment-pointer audit clean: every referenced dev:/llm: token resolves
      to a defined anchor (82 distinct tokens across the Class B set; the one
      wildcard, dev:cli-commands-* in CHANGE_IMPACT.md, is intentional);
      17 cross-doc references all point at the correct home doc.
- [x] Every card's Deep-dive section names exactly its Class A doc's fragments
      (11/11 complete).
- [x] Stale forward-references removed: 14 annotations across 11 cards -
      "llm:change-impact (task 13.2)" x11, "dev:mc-engine-smt (task 11.2)"
      x2 (mc_engine.md), "dev:toolchain-fuzz-overview (task 12.3)" x1
      (workspace_crates.md). No other task annotations remain in the Class B
      set.
- [x] Cited-command audit clean: all named test suites exist (the three
      crate-level ones - adapters/integration/nostd_minimal - are cited with
      full paths); all cargo aliases exist in .cargo/config.toml; no stale
      numeric claims in any card.
- [x] Full verification green after the fixes: dev_docs_test 9/9, docs_md_test
      5/5, cargo fmt --check clean, clippy --all-targets exit 0, cargo test
      (+ --workspace) - 535/535 root, 559/559 workspace (unchanged from the
      13.2 tree).
- [x] Plan row 13.3 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 13.3)

- No card content rewrites beyond the stale-reference fixes - the cards match
  their final Class A deep dives by construction (written in the same task);
  the pass found no factual drift, only the forward-reference annotations.
- No START.md / KB.md / BUILD.md edits (pointer sync = task 14); no DESIGN.md
  edits.
- No source or test changes of any kind; no new dependencies.
- No budget changes to any Class B file.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Subtask 13.3 appended at execution (2026-11-01) | Per the correction above: subtask sections are written when executed, following the PHASE_8_TASK_9.md / PHASE_8_TASK_11.md pattern. Executed as the verification/budget pass per the 2026-10-30 plan correction (cards filled by their deep-dive tasks); found and fixed exactly one drift class - 14 stale "(task N.N)" forward-references to now-completed tasks across all 11 cards. |
| Subtask 13.2 appended at execution (2026-11-01) | Per the correction above: subtask sections are written when executed, following the PHASE_8_TASK_9.md / PHASE_8_TASK_11.md pattern. Written with verified source material measured 2026-11-01 (BUILD.md section 5 checklist rows, START.md section 6 conventions, the 87+12 fragment inventory, and the full Class B file/budget census), row plan covering the TBD's nine named change types plus seven more (parser internals, SMT, external-tool layer, workspace crates, fuzz/training/harness, spec generation, universal) to reach every subsystem in TASK_MAP.md. |
| Task file created at execution (2026-11-01) for plan group 13 | User model: one task file per plan group ("execute 13.1" = subtask of task file 13). Written for subtask 13.1 only; subtasks 13.2 (CHANGE_IMPACT + README budget rules) and 13.3 (final card budget pass) will be appended at execution, following the PHASE_8_TASK_9.md / PHASE_8_TASK_11.md pattern. |
| Skeleton's "72 hand-written files" corrected to 71 | tests/dev_docs_manifest.txt holds exactly 71 entries (verified by count); the new Coverage header states 71. The skeleton's number was an approximation from task 1.x. |
