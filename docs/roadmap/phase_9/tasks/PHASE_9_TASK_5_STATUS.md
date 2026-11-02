# Phase 9, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_5.md` are reached. Not
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
| `KB.md` | changed | header rewritten (status 2026-11-02, summary list matching the new 1-10 structure, read order starting from START.md); section 8 "Updated roadmap" -> "Roadmap pointer" (short pointer to docs/roadmap/ + PRINCIPLES.md section 3 for post-v1 candidates - the stale per-phase status text, incl. "Phase 8 IN PROGRESS", is gone); sections 9 (Historical R&D plan), 12 (What to work on next), 13 (Phase 1 codegen + bounded MC) removed in full; sections 10/11 renumbered to 9/10. File shrank 1056 -> 674 lines |
| `docs/dev/DECISIONS.md` | changed | the ~line-143 citation "recorded in detail in KB.md section 13" now points at the Phase 1 status report (`docs/roadmap/phase_1/PHASE_1_STATUS.md`) - the actual detailed record |
| `docs/dev/toolchain_fuzz.md` | changed | three wrong training-count pointers "KB.md section 3" -> "KB.md section 1" (the counts live in KB section 1's state table, not the syntax reference) |
| `docs/llm/toolchain_fuzz.md` | changed | one same-line "section 3" -> "section 1" replacement; file stays 78 lines - Class B line budget untouched (dev_docs_test budget check green) |
| `START.md` | changed | line ~255: "condensed from KB.md section 11" -> "section 10" (the renumbered target) |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- KB section 9 "Historical R&D plan (Phase 0 -> Phase 1)" - bannered
  superseded; substance in docs/roadmap/phase_0..1 status files + DESIGN 19.
- KB section 12 "What to work on next" - bannered obsolete as of Phase 6 task
  9; pointer target covered by the new section 8 body.
- KB section 13 "Phase 1 code generation and bounded model checking" -
  bannered superseded (Phase 4); substance in DESIGN 19-22 + CODEGEN.md.
- The stale per-phase status view in old KB section 8 (duplicated
  docs/roadmap/CURRENT_STATE.md and said Phase 8 was IN PROGRESS).

## New functionality

- KB.md with a current header, a roadmap-pointer section 8, no sections
  9/12/13, and contiguous numbering 1-10.
- All four wrong training-count pointers re-targeted to KB section 1; the
  DECISIONS.md Phase-1-decisions citation re-targeted to the status report.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK (incl. the Class B line-budget check over the edited llm card) |
| `cargo test-training` (5: 1 codegen-aggregate + 4 dedicated) | OK |
| `cargo test --test docs_md_test` (5) | OK |

## Found problems/issues/mismatches
- The first boundary-based removal script kept the "## 9. Historical R&D plan"
  heading + body (it deleted only the blank line before the next heading),
  creating a duplicate section 9 alongside the renumbered Quick-start. Caught
  immediately by the post-edit structure check (`grep -n "^## "`); fixed with
  a second precise removal of the 237-line block. Final structure verified:
  sections 1-10 contiguous, no gaps, no duplicates.

## Other important findings/information
- Reference audit (re-run at execution, matching planning): exactly two live
  references to the removed/renumbered sections existed - DECISIONS.md:143
  (KB 13) and START.md:255 (KB 11) - both fixed by this task. No training-set
  file quotes any removed KB section (grep clean).
- The post-v1 candidates list that lived in old section 8's last bullet is not
  lost: it is the same list as `docs/roadmap/PRINCIPLES.md` section 3, which
  the new pointer names explicitly.
