# Phase 10, Task 25 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_25.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TEST_STRUCTURE.md` | changed | 49 function rows + 6 type rows appended to `## All functions` / `## All types` for the model-checking test cluster: 24 fns + 5 types for `tests/benchmark_mc_test.rs` (six benchmark specs pass-complete with pinned deterministic metrics, five pinned FAIL variants with counterexample parity + ce_steps pins, JSON report + timing tables, --threads byte-identity gate, --fast smoke), 17 fns + 1 type for `tests/liveness_mc_test.rs` (eventually/WEAK-until/skeleton/input_is violations with lasso structure checks, --nondet strong-fairness filter, truncated-mode soundness, deeper-nesting diagnostic, safety precedence), 8 fns for `tests/smt_bounded_test.rs` (feature-gated SMT bounded mode: both backends x FAIL/liveness/PASS byte-identical with pinned summary lines, runtime --bound override). Every row carries bidirectional project Depends (mapped to real identifiers across src/{parser,codegen,json_rw}.rs and the task-20 tests/common/fail_variants.rs helpers - spec_source/http2_rst_fail_src/can_passive_fail_src), a one-line description, and blank Component/Functionality/Documented-in cells. Also restored the `## All types` and `## Components` section headings that this task's insertion had glued onto the last rows (see Found problems). |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | Plan correction 25a: the type figures were grep counts inflated by RFS spec enums inside inline raw strings - see Found problems. |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_25.md` | changed | Task corrections section: type rows ~23 -> 6 with the phantom analysis (mirrors plan correction 25a). |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The model-checking test cluster is now in the test inventory:
  - `tests/benchmark_mc_test.rs` (24 fns / 5 types): the six property-bearing benchmark specs (43/44/45/47/48/49) through both backends in exact mode - pass-complete statuses, pinned deterministic metrics (incl. monitor size), five pinned FAIL variants (safety, liveness, assumption relaxation) with counterexample parity + ce_steps pins from the JSON report, `target/mc_benchmarks.json` + per-backend summary/timing tables (asserted on parsed fields only), the --threads 1/2/4 byte-identity gate over all eight workloads, and the --fast soundness smoke.
  - `tests/liveness_mc_test.rs` (17 fns / 1 type): liveness engine integration - eventually / WEAK until / skeleton / input_is violations with pinned formula/step/cycle/note + the lasso structure check (prefix + one cycle iteration, closing configuration), --nondet strong-fairness filter, truncated-mode soundness (FAIL inside the explored part, UNKNOWN cut off), deeper-nesting codegen diagnostic, safety-over-liveness precedence.
  - `tests/smt_bounded_test.rs` (8 fns): the feature-gated SMT bounded mode (`--features smt`, project-local z3 5.3.1) - both backends x three inline specs byte-identical with pinned status/label/step summary lines and incomplete labeling on the PASS spec, plus the runtime --bound override.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written and the two glued section headings were restored (the new rows are plain-text table cells, no backticked paths or mermaid, so they do not trip the gate's link/mermaid checks) |

## Found problems/issues/mismatches

- PLAN CORRECTION 25a: the Goal's per-file type figures ("10 types" liveness_mc_test.rs, "3 types" smt_bounded_test.rs) and the Context's "~23" total were grep counts inflated by RFS spec enums inside inline raw strings - the phantom class of corrections 7a/8a/20a/21a/23a/24a. liveness_mc_test.rs: nine of the ten type hits are `enum State` declarations inside its nine inline spec constants; only LivOut is a real Rust type. smt_bounded_test.rs: all three hits are `enum State` inside SMT_FAIL_SRC / SMT_LIV_SRC / SMT_PASS_SRC - zero real types. benchmark_mc_test.rs is clean at 5 (Pin, BenchRun, FailPin, CodegenTimes, SpecRuns). The fn counts were all clean (24 + 17 + 8 = 49). Authoritative total: 49 fns / 6 types.
- Pre-existing layout quirk surfaced by this task's insertion: the committed file had NO blank line between the last All-functions row and the `## All types` heading, nor between the last All-types row and `## Components` (introduced by task 23's splice, which noted the same adjacency but left the headings directly attached to the table). This task's after-the-row insertion therefore glued both headings onto the last inserted rows (`...|  |## All types`, `...|  |## Components`) - comrak parses a heading inside a table cell as cell text, so docs_md_test would not have caught it. Both headings were split back onto their own lines with a separating blank line; all six section headers verified present in order.

## Other important findings/information

- Future chunk tasks (26-30) must insert rows BEFORE the `## All types` / `## Components` headings with an explicit blank-line separator, not blindly after the last row - the heading adjacency makes naive after-the-row splices glue the heading onto the new last row.
- All 55 rows have blank Component/Functionality (task 32) and Documented-in (none of the three files carries a `//! Docs:` line; all three DO have docs/llm/TASK_MAP.md suite entries, noted per the preamble's rule rather than repeated row by row - same practice as tasks 21-24).
