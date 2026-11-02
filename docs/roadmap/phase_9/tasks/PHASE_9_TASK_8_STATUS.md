# Phase 9, Task 8 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_8.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

(none - docs-only task)

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `START.md` | changed | section 2 KB.md row rewritten (content = project state + DESIGN.md pointer stubs + source-code map + CLI/training/workflow/limitations/roadmap-pointer; read-it-when routes language-semantics questions to DESIGN.md / docs/SEMANTICS.md first); section 5 KB.md section map rewritten to the final 1-10 structure (stubs for 2-3, roadmap pointer for 8, renumbered 9-10, removed 9/12/13 entries gone); section 6 rows re-checked: the syntax/grammar row now cites DESIGN 5-6 directly (KB 3 is a stub) and the phase-implementation row cites DESIGN 12 + 5.10 for properties (KB 2.6 is a stub); the KB 4.2/4.5/4.6/7 citations still resolve and were kept |
| `DESIGN.md` | changed | section 7 IR design rule 4: CJK artifact "should not污染 the core IR" -> "should not pollute the core IR" (the only non-English occurrence in any doc; assigned here by the plan's corrections table) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- The CJK artifact in DESIGN section 7 rule 4.

## New functionality

- `START.md` - the session entry point every future LLM session reads first -
  now describes the post-phase documentation set exactly: KB.md as state +
  pointers + layout/CLI/workflow, DESIGN.md as the single source of truth for
  core design.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (535) | OK - exactly the known-good baseline (docs_md_test 5, dev_docs_test 9 among them) |
| `cargo clippy --all-targets` | OK - no errors; the documented pre-existing warning baseline (46 lines / 35 unique) unchanged |
| `cargo fmt --check` | OK - clean |

## Found problems/issues/mismatches
- **Environmental, not project**: `diagram_render_parity_test::parity_external_mmd_pinned_subset`
  failed on the first full run because THIS sandbox session's puppeteer cache
  (`/opt/unsloth-studio/sandbox/__LOCALID_KLa3Bmk/.cache/puppeteer`) has no
  chrome-headless-shell, while mmdc is on PATH so the availability-gated
  external half runs. The exact browser version (156.0.8078.47) exists in
  other sessions' caches; pointing `PUPPETEER_CACHE_DIR` at one of them makes
  all 13 tests pass. Future sessions hitting this: export PUPPETEER_CACHE_DIR
  to an existing cache (or run `npx puppeteer browsers install
  chrome-headless-shell`) - it is not a code or docs regression.
- **My own status file tripped the doc gate**: PHASE_9_TASK_7_STATUS.md line 35
  carried an unwrapped illustrative link (`[KB.md](../../KB.md)`) that does not
  resolve from docs/roadmap/phase_9/tasks/. Same failure class as task 1's -
  wrapped in a code span; gate then green. Reinforces the standing rule:
  inside roadmap documents, show link syntax only inside backticks.

## Other important findings/information
- Final sweep results (all five greps from the task file): (1) no live
  reference to removed KB sections 9/12/13 - the only hits are Phase 9's own
  planning/task documents quoting the old text while describing the fix
  (allowed exception, same category as TASKS.md history); (2) zero bare
  cross-doc `.md` mentions in docs/dev outside fences; (3) root README link
  targets exist and resolve (docs_md_test proves it); (4) "C is planned next"
  gone from KB.md + DESIGN.md; (5) no CJK characters in any doc.
- Phase 9 is now at its exit criteria - the close-out itself (PHASE_9_STATUS.md
  per FORMATTING.md section 4, CURRENT_STATE.md row, TASKS.md entry, commit)
  follows per PROTOCOL.md section 3 steps r-t.
