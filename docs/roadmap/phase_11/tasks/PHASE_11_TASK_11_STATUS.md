# Phase 11, Task 11 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_11.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | documentation-only task - no source added |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | no source touched |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | actualized: all row identifiers + Depends rewritten to post-refactor paths (mod.rs convention for absorbed files, diagram-merge convention for the five label helpers); LCG rows swapped (12 specgen Rng / codegen_test Lcg rows deleted, 9 canonical utils/rng.rs rows inserted per actual source); ident.rs rows re-pointed with corrected Documented-in; "Proposed naming scheme" -> "Naming scheme" (tree block now draws codegen/ident.rs), "Proposed layout" -> "Layout", file-mapping table carries actual locations (49 rows) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | task 11 entry appended |

## Changed functionality
- None - documentation-only change; the inventory now describes the ACTUAL
  refactored tree instead of proposing it.

## Removed functionality
- None.

## New functionality
- None (behavioral). The document's role changed from proposal to living
  inventory of the source tree.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no committed test changes; the string-masked grep audit ran as a session-local script (reconstructed from the Phase 10 close-out method), not a repo test |

## Tests run status

| Name | Status |
|---|---|
| `docs_md_test` (5) | OK - document parses, fences closed, mermaid incl. the new ident.rs node renders, links resolve |
| `dev_docs_test` (9) | OK |
| string-masked grep audit (session script) | EXACT - 936 fns / 196 types, zero per-file diffs against the refactored tree; pre-check validated the extractor + mapping before any edit (clean except the five expected special cases: diagram merge x2, LCG x3) |
| old-path grep (all 42 mapping entries' old sides) | ZERO hits anywhere in the document |

## Found problems/issues/mismatches
- The Phase 10 close-out audit script was session-local (never committed), so it
  had to be reconstructed from the recorded method before this task could run.
  Reconstruction findings encoded in the script: (1) trait-impl methods use the
  `Type(Trait)::method` marked form only where Phase 10 tasks marked them - the
  audit derives the marked set from the document itself; (2) nested-type naming
  is per-file as written in Phase 10 (ref_mc `run::St` keeps its parent-fn
  prefix, test_cmd `Guard` does not); (3) associated-type bindings inside impls
  are phantoms, not inventory types; (4) raw identifiers and where-clauses in
  impl headers need explicit masker handling.

## Other important findings/information
- This status file was BACKFILLED: the task executed and committed (b1e5687)
  without its `PHASE_11_TASK_11_STATUS.md` - a PROTOCOL.md section 3 step m gap.
  Content reconstructed from the commit record, the TASKS.md entry of
  2026-11-06 20:45, and the task file's exit criteria (all met at commit time).
- The audit script's conventions (marked-form derivation, mod-scope handling)
  were reused unchanged by task 12.
