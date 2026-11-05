# Phase 10, Task 32 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_32.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | Component + Functionality cells set on ALL 974 inventory rows (940 function + 34 type) from the task-31 vocabulary; zero blank cells remain. Component is file-level (the suite's test-side component PLUS the tested-side source component(s) per the reuse rule; inline-test rows carry Inline unit tests + their host file's source component; fixture rows carry Doc quality gates). Functionality is the file's verification axis with row-level additions where a row verifies something extra. Only the two cells changed on any row - identifier, Depends, description and Documented-in are byte-identical (git diff: exactly 974 lines touched, one per row). |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The classified test inventory is complete: task 33 can now build the
  per-test-file mapping table and justify the proposed test-tree layout
  against a row-level component/functionality picture instead of an empty one.
- Design decisions applied while filling (mirroring tasks 15/16's file-level
  Component / row-level Functionality split):
  - Suite files carry their category's tested-side source components, refined
    per file where a file clearly does not exercise one: wide_ops_parity and
    prod_parity omit Shared MC concepts (production-FSM parity, no MC wire
    format); liveness_mc_test omits C backend (verified Rust-only - the file
    compiles with rustc only, no gcc path exists); benchmark/liveness/
    smt_bounded each carry exactly the source components their codegen + run
    paths touch.
  - liveness_mc_test's axis is behavior pinning, not parity pinning: its rows
    verify pinned lasso formula/step/cycle/note and lasso structure on a
    single backend (exact expected output), so the cross-implementation axis
    does not apply even though the task-31 suite entry groups it with the
    byte-identical-across-backends cluster.
  - Task 31's helper-axis sketch was refined row-precisely: prod_driver rows
    get parity pinning + determinism pinning (seeds + byte-identity) (the LCG
    lineage input script feeds identical bytes to both drivers); fail_variants
    rows get behavior pinning (pinned FAIL-variant specs with the anchor
    drift-guard - exact expected artifacts); mc_json's zero_peak_rss gets
    parity pinning (it exists so Rust/C MC reports compare byte-for-byte).
  - Row-level functionality additions (50 rows) follow what the row verifies:
    frontend_test's example-corpus walks + file-driven corpus coverage, its
    `rfs test --suite fuzz` lines + fuzzing (generated code), its gen-spec
    seed rows + determinism pinning, its mc-ref agreement row + differential
    soundness; codegen_test's both-backends rows + parity pinning, its header
    pins + exit-code/CLI-contract pinning, its fuzz-skeleton rows + fuzzing
    (generated code); benchmark's threads gate + determinism pinning;
    ref_mc_diff's CLI row + exit-code/CLI-contract pinning; toolchain_fuzz's
    ground-truth rows + differential soundness; psl_translation's all-examples
    walk + file-driven corpus coverage; info_format_test's two all-valid-
    example walks + file-driven corpus coverage; the inline LCG/seed/byte-pin
    rows (codegen_test 2, specgen 4, ref_mc 4, gen_spec_cmd 1) + determinism
    pinning; smt_emit's two both-backends op-sequence rows + parity pinning;
    codegen_test's two all-50-examples planner rows + file-driven corpus
    coverage.
  - dev_docs_test splits row-by-row between doc-quality gating (fragment
    grammar, budgets, manifest validation, the aggregate gate) and code<->doc
    traceability (the four link checks + their negative controls + the item-
    declaration matching helpers that serve them), per the task-31 axis
    definitions.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the classification was written (the filled cells are plain category names, no backticked paths or mermaid) |

## Found problems/issues/mismatches

- No plan correction needed: the Goal's "~957 function rows + ~14 type rows"
  is the pre-correction estimate; the authoritative totals are 940 function /
  34 type rows per plan corrections 20a-30c, which already document them (the
  same situation task 31's status recorded for the same figures).
- The Goal's "~957" also predates correction 30c's +12 rfs-build inline rows
  and +4 fuzz macro-entry rows; nothing in this task's scope changed because
  of it - every row that exists was classified.

## Other important findings/information

- Completeness sweep (exit criterion): zero rows with an empty Component or
  Functionality cell across both tables; every value verified to come from the
  task-31 category tables (50 components / 11 axes) - no free-text values.
- Column-preservation proof: git diff shows exactly 974 changed lines (one per
  inventory row, nothing else in the file); a per-row cell comparison confirms
  cells 1/2/5/6 (identifier, Depends, description, Documented-in) are
  byte-identical and only cells 3/4 differ. Rows remain one-per-line.
- Spot-check (exit criterion): 10 random rows verified against their source -
  frontend_test::example_root_dir, benchmark_mc_test::BenchRun,
  info_format_test::diagram_dot_md_pins_sections,
  frontend_test::e2e_fuzz_print_script_pinned_and_no_execution,
  backend_parity_test::root, src/monitor.rs::tests::run,
  fuzz/fuzz_targets/common.rs::split_head_rest,
  docs_md_test::non_rendering_mermaid_is_detected,
  psl_translation_test::bits_of, common/fail_variants::http2_rst_fail_src -
  all classifications match what the source verifies.
- Component-cell coverage: all 50 vocabulary components (18 test-side + 32
  reused source) appear in at least one row's Component cell, and all 11
  functionality axes appear in at least one row - including the two
  end-to-end-only components task 31 flagged (rfs-test command via
  frontend_test's file-level set; Example crates via the example-projects and
  integration-example suites).
