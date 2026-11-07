# Phase 11, Task 13 - Implementation status

> Written once when all exit criteria of `PHASE_11_TASK_13.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | no new source files |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/dev_docs_test.rs` | changed | `check_doc_code_refs` now skips exactly `docs/llm/SRC_STRUCTURE.md` + `docs/llm/TEST_STRUCTURE.md` (documented in its doc comment): the move brought them under reference resolution for the first time and 27 of their backticked spans are identifier-format examples / rename-rule prose (some naming OLD paths that no longer exist), not navigational references - their freshness is enforced by the string-masked grep audit (PROTOCOL.md section 5), strictly stronger. No fn/type/source file added, removed or renamed - no inventory row changes required under the correspondence rule |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed (moved) | git mv from repo root + gate-forced `<!-- llm-budget: 1710 -->` header line; no content change |
| `docs/llm/TEST_STRUCTURE.md` | changed (moved) | git mv from repo root + gate-forced `<!-- llm-budget: 1287 -->` header line; no content change |
| `docs/roadmap/PROTOCOL.md` | changed | new section 4 (impact calculation - mandatory Depends-column consultation of the two inventories before scoping any source change, linked by their new paths) + section 5 (docs<->source correspondence - same-commit inventory row update obligation, string-masked audit as the check) |
| `docs/roadmap/PRINCIPLES.md` | changed | one new bullet in section 2 naming the two protocol sections as the doc-set's impact/correspondence obligations |
| `docs/llm/README.md` | changed | two index-table rows for the moved inventories (user-directed answer to the open question; within the 70-line budget) |
| `docs/roadmap/phase_11/tasks/PHASE_11_TASK_13.md` | changed | Task corrections table: the three deviations above |
| `docs/roadmap/phase_11/PHASE_11_PLAN.md` | changed | Plan corrections row 13a (task 13 scope: the move breaks dev_docs_test in two unforeseen ways + user-directed README rows) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | task 13 entry appended |

## Changed functionality
- The doc-sync gate's reference-resolution scope now explicitly excludes the
  two inventory tables (they were never in scope before - at repo root they
  were not scanned at all; the move would have put them in scope by accident).

## Removed functionality
- None.

## New functionality
- PROTOCOL.md section 4: impact calculation rule - any source-change scoping
  must consult the affected rows' Depends columns in docs/llm/SRC_STRUCTURE.md
  (non-test) or docs/llm/TEST_STRUCTURE.md (test); a plan naming no inventory
  rows is incomplete.
- PROTOCOL.md section 5: docs<->source correspondence rule - every commit that
  adds/removes/renames a function, type or source file updates the affected
  inventory rows in the same commit; the string-masked grep audit is the check.
- PRINCIPLES.md section 2 names both obligations via the protocol sections.
- The inventories live at their docs/llm/ home with budget headers and README
  index entries - first links into them from the doc set.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/dev_docs_test.rs` (`check_doc_code_refs`) | changed | inventory exclusion per above; all 9 tests still pass, negative controls untouched |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (535) | OK - exact documented baseline, all 31 test binaries green |
| `docs_md_test` (5) | OK - both inventories parse at their new location, mermaid renders, PROTOCOL.md's new relative links resolve |
| `dev_docs_test` (9) | OK - budget headers accepted, exclusion in place, sync gate clean |
| `cargo fmt --check` | OK |
| `cargo clippy --all-targets` | OK - no new warnings (pre-existing 46-line baseline unchanged; zero in dev_docs_test.rs) |

## Found problems/issues/mismatches
- The move broke dev_docs_test in two ways the plan did not foresee (plan
  correction 13a): the Class B budget rule (every docs/llm/*.md needs a header)
  and reference resolution (27 non-navigational backticked spans). Both fixed
  minimally: one header line per inventory + a documented two-path exclusion.
- Pre-existing environment failure, unrelated to this task: `mmdc` lost its
  chrome-headless-shell (puppeteer cache) and `parity_external_mmd_pinned_subset`
  failed on the first full-suite run. Restored the browser cache (the same fix
  as Phase 11 task 6's); the suite then passed 13/13 and the full run 535/535.

## Other important findings/information
- The two missing status files of tasks 11-12 were backfilled in a separate
  commit (cbb23be) before this task executed - the PROTOCOL.md section 3 step m
  gap is closed and noted in each backfilled file.
- Task 14 (close-out) can verify the moved documents at their new home; the
  phase exit criterion "the inventories live at docs/llm/..." is met by this
  task.
