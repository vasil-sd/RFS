# Phase 10, Task 29 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_29.md` are reached. Not
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
| `TEST_STRUCTURE.md` | changed | 180 inline-test function rows appended - the frontend/IR cluster's `#[cfg(test)]` modules: parser.rs 32 fns, validator.rs 26, json_rw.rs 26, printer.rs 14, monitor.rs 28, codegen_test.rs 31 (30 top-level + 1 nested helper), ext_tool.rs 10, cli_log.rs 2, lib.rs 2, frontend.rs 3, spec_hash.rs 2, codegen_fsm.rs 4. Every row uses the `src/<file>.rs::tests::<fn>` identifier form (the one nested fn gets the preamble's `<parent_fn>::<child_fn>` form), carries project Depends mapped to real SRC_STRUCTURE.md identifiers (parse_source/validate_module/Json reader+writer/print_ir+roundtrips/compile_formula+ReferenceMonitor/build_test_plan+evaluator/ext_tool resolution/LogLevel/collect_rfs_files/Format/sha256/build_variants) plus same-module test helpers, a one-line description of the pinned behavior, and blank Component/Functionality/Documented-in cells per the binding rules. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase.

## Removed functionality

None.

## New functionality

- The inline `#[cfg(test)]` enumeration is half done (task 29 of the two-chunk
  split): all twelve frontend/IR host files are inventoried in TEST_STRUCTURE.md.
  Their tests pin, among the rest: the task-23 formula-level boolean grammar
  (mixed atom kinds, precedence, until/eventually liveness operators and their
  dedicated diagnostics), the validator's version-header/priority/shadowing/
  warning classes and liveness operand typing, the strict JSON reader/writer
  (escaping, surrogate pairs, depth limit, key-order preservation, round-trip
  property over an IR/CLI corpus), the printer's canonical minimal-parenthesis
  forms in both directions, the reference monitor's R1-R4 subset rejections and
  pinned obligation/invariant semantics (deadlines, delay registers, per-sample
  Or recovery), the test planner's taken/not-taken case construction with
  concrete candidate values (incl. the two correction-5a regression pins and
  the 50-example corpus guarantees), ext-tool resolution precedence + byte-pinned
  script rendering, log-level parsing/ordering, the crate-root API smoke tests,
  Format/normalize_abs/err_file_path, the NIST SHA-256 vectors + spec_hash16
  prefix property, and FSM variant flattening/representative values/safety-subset
  analysis.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green after the rows were written (plain-text table cells, no backticked paths or mermaid; literal pipes in descriptions escaped as \|) |

## Found problems/issues/mismatches

- No plan correction needed: the re-verified extraction matched the Context's
  per-file figures EXACTLY (parser 32, validator 26, json_rw 26, printer 14,
  monitor 28, codegen_test 31, ext_tool 10, cli_log 2, lib 2, frontend 3,
  spec_hash 2, codegen_fsm 4 = 180). The planning-time counts were already
  test-aware per correction 2a's rule.
- The extraction itself needed real string/comment masking to be trustworthy:
  a naive line-based brace-depth scan miscounted badly (parser 18 vs 32,
  codegen_test 5 vs 31) because multi-line string literals and raw strings
  (r#"..."# - json_rw alone has 19 in its test module) carry braces and `fn`
  text. The authoritative count comes from a character-level scanner that
  masks line/block comments, normal strings, char literals, and raw strings
  before depth tracking.
- codegen_test.rs's 31st fn is NESTED: `eval_sig` inside the test body of
  planned_monitor_violations_are_executable_and_record_target_first. Per the
  binding preamble convention it gets the identifier
  `src/codegen_test.rs::tests::planned_monitor_violations_are_executable_and_record_target_first::eval_sig`
  - the single row in this chunk that deviates from the task's pinned
  `src/<file>.rs::tests::<fn>` form, justified by the preamble (nested fns are
  `<file>::<parent_fn>::<child_fn>`; same precedent as task 28's three walk fns).
- No types in any of the twelve test modules - the printer.rs grep hits for
  `enum` are all RFS spec snippets inside string literals passed to parse_source,
  not Rust type declarations. The task's "if any appear, record a plan
  correction" clause does not trigger.

## Other important findings/information

- Seven descriptions contain the formula-level disjunction operator; the raw
  `||` would split the pipe-delimited table row into extra cells, so those
  occurrences are escaped as `\|\|` (the document's first escaped pipes - no
  earlier row needed them). docs_md_test does not check table column counts,
  so this was caught by a structural re-parse of every inserted row, not by the
  gate.
- All 180 rows have blank Component/Functionality (task 32) and Documented-in
  (inline tests have no manifest line of their own; the host files' fragments
  are already recorded on the hosts' SRC_STRUCTURE.md rows).
- Every Depends target was cross-checked against the union of both documents'
  identifier columns - all resolve (no dangling edges).
