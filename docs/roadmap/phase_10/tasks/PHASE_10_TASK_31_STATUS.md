# Phase 10, Task 31 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_31.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | `## Components` filled with 50 rows (18 test-specific + 32 source components reused from SRC_STRUCTURE.md, spelled identically, depends-on lists copied verbatim) and `## Functionality` filled with 11 test-methodology axes. Every row carries a depends-on list + description/rationale. The Context's start lists were refined against the rows as the task prescribes: four new components forced by files with no other home (Parser comparison suite, Toolchain fuzz CI suite, Integration example suite, and the Example projects / Example corpus split) and one new functionality axis (behavior pinning - the home for the 295 inline unit-test rows + workspace-crate rows). The determinism-pinning axis name was comma-free'd ((seeds + byte-identity)) so multi-valued cells stay parseable. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The test-side classification vocabulary exists: task 32 can now fill the
  Component/Functionality cells of all 940 function rows + 34 type rows from
  these two tables, and task 33 can justify the proposed layout against them.
- Design decisions recorded in the tables themselves:
  - Reuse rule applied: a test row carries its suite's test-side component
    PLUS the tested-side source component(s) - e.g. a parity-suite row gets
    Parity suites + Rust backend + C backend. All 32 SRC components are
    referenced by at least one test row (30 via Depends edges; rfs-test
    command and Example crates only end-to-end, through the binary/Makefile,
    with no identifier edges - noted in their rows).
  - Functionality is methodology-only (how a row verifies), never
    source-functionality names - the tested side stays in the Component cell.
  - Inline unit tests is an umbrella component with depends-on '-' (each row
    additionally carries its host file's source component per the reuse rule).
  - Doc quality gates keeps the Context's grouping of docs_md + dev_docs +
    info_format; the three fixtures are the gate's own negative controls.
  - Shared test helpers rows get their CONSUMING suites' axes (prod_driver ->
    parity pinning / corpus coverage; fail_variants -> benchmarking /
    exit-code contracts; mc_json -> differential soundness) - helpers have no
    standalone verification methodology.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the tables were written (plain-text table cells, no backticked paths or mermaid) |

## Found problems/issues/mismatches

- No plan correction needed: the Context's start lists were a seed to refine
  against the rows (the task's own design), and the refinements are recorded in
  the tables' rationales rather than as corrections. The Context's "~957
  function rows" is the pre-correction figure - the authoritative total is 940
  per plan correction 30c, which already documents it.
- Two of the 32 reused SRC components (rfs-test command, Example crates) have
  ZERO direct Depends references from any test row - their suites drive them
  end-to-end through the binary/Makefile/cargo. They are kept in the vocabulary
  because the reuse rule is semantic (a suite that tests component X gets X),
  and task 32 will assign those rows accordingly; both rows say so explicitly.
- The first draft of the determinism-pinning axis name contained a comma
  ((seeds, byte-identity)), which would have made multi-valued Functionality
  cells ambiguous under the preamble's comma-separated rule - renamed to
  (seeds + byte-identity) before the gate run.

## Other important findings/information

- Assignability walk (exit criterion): all 62 test files - 27 tests/ suites +
  helpers + 3 fixtures, 25 inline-test hosts, 3 crate-test files, 5 fuzz
  targets - are assignable to >= 1 component and >= 1 functionality. The only
  files needing a new component were library_comparison.rs (Parser comparison
  suite), toolchain_fuzz_test.rs (Toolchain fuzz CI suite) and
  integration_example_test.rs (Integration example suite); example_projects
  was split from the example-corpus concept because it drives real build
  systems rather than the .rfs corpus.
- Edge analysis input: 30 of 32 SRC components are referenced by test Depends
  edges (Parser 30 files, IR model 25, Codegen core 19, JSON I/O 17, Shared
  test planning 13, Validator 11 - the rest fewer); the two unreferenced ones
  are the end-to-end-only components above.
- Task 32's fill order should follow task 20-30 chunk order (the preamble's
  binding rule), using these tables verbatim; multi-valued cells are allowed
  and expected for cross-cutting rows (parity suites, frontend_test).
