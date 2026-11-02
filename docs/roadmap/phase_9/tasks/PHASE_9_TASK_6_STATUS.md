# Phase 9, Task 6 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_6.md` are reached. Not
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
| `docs/dev/DECISIONS.md` | changed | all 117 cross-document `.md` mentions outside code fences converted to clickable relative links: DESIGN.md x91 -> `[DESIGN.md](../../DESIGN.md)`, KB.md x3, TOOLCHAIN.md x3, BUILD.md x1, CODEGEN.md x1 (root docs, two levels up), SEMANTICS.md x3 (`../SEMANTICS.md`), MIGRATION.md x3 (`../user/MIGRATION.md`), roadmap files x10 (phase descriptions/plans/status/task files under `../roadmap/...`), PRINCIPLES.md x1; "section N" prose kept outside the brackets; every `.rs/.c/.h` doc-to-code span, fragment anchor, and fenced text byte-identical |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- None - plain-text document references replaced by equivalent clickable links.

## New functionality

- One-click navigation from the rationale index to every cited document:
  developers clicking a "DESIGN.md section N" reference land in DESIGN.md;
  roadmap history references resolve to the exact phase plan/status/task file.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK (doc-to-code spans + fragment ownership intact) |
| `cargo test --test docs_md_test` (5) | OK after the root-doc depth fix (first run FAILED - see below) |

## Found problems/issues/mismatches
- **Root-doc link depth wrong in the planned path rules**: the Context said
  root docs link as `../DESIGN.md`, but docs/dev/ is two levels below the repo
  root - the correct form is `../../DESIGN.md`. docs_md_test caught all 99
  root-doc links on the first run; a single re-depthing pass fixed them and
  the gate went green. Recorded in the plan's corrections table (task 7 must
  use the corrected depth).
- **Backticked mentions**: 15 of the 117 mentions were originally wrapped in
  backticks (e.g. `` `docs/roadmap/phase_4/PHASE_4_PLAN.md` ``). Converting
  them inside the backticks would have rendered literal `[...](...)` text,
  not a link - a second pass unwrapped every `` `[...](...)` `` to a real
  link.

## Other important findings/information
- Conversion was done with a single-pass regex (longest-alternative-first)
  over the 18 distinct mention tokens, so no token could match twice and no
  already-inserted link could be re-matched; a post-conversion scan confirms
  zero bare cross-doc `.md` mentions remain outside fences (self-references
  excepted - there are none in this file).
- The ~line-143 citation links to the Phase 1 status report target fixed by
  task 5 (`../roadmap/phase_1/PHASE_1_STATUS.md`), not to a removed KB section.
