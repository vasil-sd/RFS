# Phase 3, Task 21 - Implementation status

> Written once when all exit criteria of `PHASE_3_TASK_21.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (docs-only task).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|
| `DESIGN.md` | changed | section 5.10: `always[N]` added to the formula list, fixed stale `be[5] input_is(Tick)` example (missing parens), new "Monitor semantics (Phase 3, D-P3-3)" block (sampling model, obligation algorithm, worked example, window/top-level-be/next rules); section 6: formula EBNF rewritten for the task-23 grammar (formula-level `\|\|`/`->`/`&&`/`!`, `primary_formula` fallthrough to `non_boolean_expr = equality_expr`, explicit `expression` ladder, `atom_call` moved to formula level); section 7: additive `bounded_always` IR note (D-P3-1); section 12: v0 subset marked implemented + R1-R4 pointer; D8 table cell and section 19 bullet: "liveness skipped" claims replaced with Phase 3 update pointers; NEW section 21 "Phase 3 property/monitor decisions" with D-P3-1..D-P3-6 comparison tables (section-20 style) + R1-R4 text. |
| `KB.md` | changed | section 1 state table: new Phase 3 row, valid examples 40->42, invalid 11->12, tests 125->170 (full breakdown), generated aggregate 492->513, training aggregate 219->224; section 2.6: `always[N]` operator, Phase 3 monitor note, task-23 grammar fix documented (formula-level `&&`/`||`, non-boolean fallthrough, mixed atom kinds); section 3.9: `always[4]` example; section 4.2: new `src/monitor.rs` row, `codegen_fsm.rs` legacy-formula-helpers note; section 7: "Resolved during Phases 1-3" incl. online monitoring of all in-subset formulas; section 13.2/13.5/13.6: Phase 3 update notes replacing post-hoc/liveness-skipped claims, count 492->513. |
| `BUILD.md` | changed | section 3: main suite 125->170 (breakdown incl. 4 monitor integration + 3 parity), generated aggregate 492->513, example counts 40/11->42/12, "Other suites" paragraph now covers `monitor_integration_test.rs` + `backend_parity_test.rs`; section 4: `[--with-property-monitors]` on both codegen rows + dedicated flag row; section 5: known-good baseline updated to post-Phase 3 (2026-10-17, 170/170, 513/513, 12 pre-existing clippy warnings). |
| `CODEGEN.md` | changed | section 2 file map: `src/monitor.rs` row, `codegen_fsm.rs` legacy note, `c_fsm.rs` monitor-parity note; section 3.1: `BackendOptions.with_property_monitors` field; section 4.1: `CodegenRequest::Generate` field + parse rule (production only); section 4.3: flag example; section 5.1: legacy formula-subset note; NEW section 5.4 `monitor.rs` (shared monitor compiler, R1-R4, identical warnings); section 8: Phase 3 testing conventions (flag sensitivity both backends, monitor rendering, cross-backend parity suite, monitor integration); section 9: invariant 6 (shared MonitorPlan, identical semantics/warnings). |
| `src/backends/RUST.md` | changed | section 2.1 anatomy: Sample-based evaluation, StepError::PropertyViolation (debug-gated in production), post-hoc check functions marked GONE (D-P3-4), Fsm monitor fields + new()/commit_step; NEW section 2.3 "Online property/assumption monitors (Phase 3)" (Mon_* structs, PropMonitors/AssumeMonitors, gating, D-P3-6 flag behavior); section 4: MC online detection + enhanced counterexample format; section 5 decision 6 and section 6 limitations: R1-R4 wording replacing "liveness skipped". |
| `src/backends/C.md` | changed | sections 4-5 (correction 11a): "Translated safety subset" bullet replaced with full monitor-parity description (mon_* structs, rfs_sample_t, #ifdef RFS_PROPERTY_MONITORS gating, RFS_PROPERTY_VIOLATION + prop_label/prop_step/prop_trigger_step, identical warning text); section 5: online violation detection + same property-specific counterexample fields as Rust MC + parity-suite pointer. |
| `START.md` | changed | section 1 snapshot (2026-10-17): Phase 3 IMPLEMENTED (close-out report + roadmap table remain, task 22), Phase 3 feature summary + D-P3 pointer, next work -> Phase 4, baseline 170/170 + 513/513 + 42/12 examples + 224 SFT; section 7: generated-test count 492->513. |

## Other file changes

None.

## Changed functionality

None (docs-only).

## Removed functionality

None.

## New functionality

None (documentation of existing Phase 3 functionality).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|
| (none) | - | no test code changes; docs verified against actual generated output (example 41 in both backends) and parser source. |

## Tests run status

| Name | Status |
|---|---|
| (no code changed - main suite state unchanged from task 20: 170/170, 513/513 generated, fmt clean) | not re-run (docs-only change) |

## Found problems/issues/mismatches

- Stale claims found and fixed beyond the planned list: DESIGN.md D8 table cell + section 19 bullet ("liveness skipped with a warning"), KB.md section 13.2/13.6 (post-hoc assumption checks, liveness skipped), BUILD.md section 3 example counts (40/11 -> 42/12) and "part of the 95" line, START.md section 7 count.
- DESIGN.md 5.10 contained a non-parsing example (`bounded_eventually[5] input_is(Tick)` without parentheses) - fixed to the real grammar.
- One edit-tool incident: a long KB.md insertion was written as literal placeholder text (argument elided in transit); detected by re-reading the file, replaced with the intended content, and a repo-wide scan confirmed no other placeholder leaks.

## Other important findings/information

- Grep audit (exit criterion 6) passed: remaining "post-hoc"/"liveness skipped" mentions are only (a) intentional decision-record references explaining what was replaced (DESIGN section 21, RUST.md 2.3, C.md 5, KB 7/13.6 update notes) and (b) frozen historical documents (TASKS.md, PHASE_1_STATUS.md, task files) which the protocol forbids rewriting.
- All new doc content was verified against actual artifacts before writing: generated example-41 output in both backends (Mon_* structs, gating, step_result_t fields), parser source (formula grammar after task 23), and `src/codegen.rs` (BackendOptions/CodegenRequest shapes).
- Remaining Phase 3 close-out work is task 22 only: `PHASE_3_STATUS.md`, roadmap phase-state table update, full verification checklist, TASKS.md entry, commit.