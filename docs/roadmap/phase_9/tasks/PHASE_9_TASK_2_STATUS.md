# Phase 9, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_9_TASK_2.md` are reached. Not
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
| `DESIGN.md` | changed | section 1 rewritten: merged KB 2.1's target list (Rust AND C parity-pinned, tests + fuzz skeletons, diagram, IDE), the "one constrained specification -> one versioned IR -> multiple targets" goal line, and the three-use framing on top of DESIGN's existing use-case list; section 2's architecture diagram replaced with KB section 1's current high-level state diagram (adds `rfs info format`, `rfs mc-ref`, `rfs gen-spec`, the in-process SVG note, and the standalone semantic-validator branch) |
| `KB.md` | changed | sections 2.1/2.2/2.3 bodies replaced by short reference stubs pointing at DESIGN 1/3/4 + docs/SEMANTICS.md (headings kept for cross-reference stability); section 1's text diagram + workspace-crate paragraph replaced by a one-line pointer to DESIGN section 2 |
| `docs/SEMANTICS.md` | changed | the "Design documents" citation re-pointed from "KB.md section 2.2, 2.6, 3" to "DESIGN.md section 3 (core semantics), 12 (property language plan), 5 (RFS v0 language subset)" - the final targets after tasks 3-4 |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None (no toolchain behavior changed).

## Removed functionality

- The duplicated purpose/semantics/constraint text in KB sections 2.1-2.3 and
  the stale Phase-5-era pipeline diagram in DESIGN section 2 - both now have a
  single home in DESIGN.md.

## New functionality

- DESIGN section 1 as the complete purpose statement (merged from both files).
- DESIGN section 2 carrying the CURRENT pipeline diagram (all eight commands
  incl. Phase 6/7 additions).
- KB sections 2.1-2.3 as stable reference stubs routing to the source of truth.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test --test docs_md_test` (5) | OK |

## Found problems/issues/mismatches
- None recorded. The execution matched the plan exactly; action 2's line-by-
  line comparison confirmed DESIGN sections 3-4 already cover KB 2.2-2.3 with
  no KB-unique content (DESIGN 4's constraint table even carries an extra
  "Production target" row absent from KB).

## Other important findings/information
- When moving the diagram into DESIGN section 2, the KB-style status markers
  (`[DONE, Phase N]`) were normalized to DESIGN's existing `[Phase N]` style -
  a stylistic adaptation only; every branch and note of the newer diagram is
  preserved. DESIGN is a design document and should not carry project-status
  markers.
- The KB section 1 workspace-crate paragraph was dropped as planned (it
  duplicates DESIGN section 2's equivalent paragraph); the pointer line notes
  that the crate layout lives with the diagram.
