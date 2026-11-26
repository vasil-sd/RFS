# Phase 18, Task 3J - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3J.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (documentation-only task per the Non-goals).

## Changed/removed source files

None - no source-code change. One stale code comment was FOUND but not fixed
(see Found problems): `src/mc/smt_plan.rs::SmtVar.init`'s doc comment still
says "literal inits only" (stale since task 3H).

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/user/MODEL_CHECKING.md` | changed | the centerpiece: new section 10 "smt-bounded vs exact mode: residual differences" - intro naming 3A-3D closed (D1/D2/D3/D4/D8) + 3F-3H lifted (V17/S3/S4/E4-S1), then five subsections (10.1 D5 completeness of PASS, 10.2 D6 counterexample form, 10.3 flag-gated FULL-property omission, 10.4 flag-gated expensive-assumption omission, 10.5 sample-0 assumption vacuous-PASS divergence) each with exact-mode snippet / smt-bounded alternative / rationale citing SEMANTICS.md / conversion instructions, plus the closing construct-by-construct summary table; TOC gains row 10; the stale pre-3G top-form list in the existing smt-bounded description replaced (bare/next desugaring + constant-expression inits noted); S4/E4-S1 appear ONLY in the intro's closed/lifted list |
| `docs/user/USAGE.md` | changed | the error-details bullet no longer claims "any flag but --bound/--full-properties under smt-bounded" or "an assumption-bearing spec under smt-bounded exits 1" - both stale since 3C; now names all four accepted flags and the two smt-only flags' cross-mode usage errors |
| `docs/user/PROPERTIES.md` | changed | section 6 gains the "Under smt-bounded" bullet: every temporal form checkable (default class weak at trace death, FULL class behind --full-properties) with a pointer to MODEL_CHECKING.md section 10; site 4's binds sentence verified accurate as-is (no change) |
| `docs/user/FAQ.md` | changed | section 2 gains the "In smt-bounded MC" assumption bullet (per-step base constraints, cheap default / expensive --full-assumptions, the sample-0 vacuous-PASS difference); section 5's SMT bullet expanded with the no-bound-makes-it-complete statement + pointer to MODEL_CHECKING.md section 10 |
| `docs/user/LANGUAGE.md` | changed (verify-only) | confirmed: no "smt-bounded rejects enum/wildcard/pattern" claim exists - the only smt mention is the MODEL_CHECKING.md pointer; left unchanged |
| `docs/SEMANTICS.md` | changed | section 9.9: the property-support paragraph now names tasks 1A+3G (bare boolean + top-level next desugared to always(...) before encoding) and adds the constant-expression var-init sentence (tasks 3H/3L, rule-4 fit check at validation, identical both-mode rejection of var-referencing inits); the section's closing line gains the pointer to MODEL_CHECKING.md section 10 for the residual differences (D5, D6, declared omissions, sample-0 divergence) |
| `docs/dev/mc_engine.md` | changed | three stale spots fixed: the SmtVar bullet (bit<N>/bool only + "literal inits only (v1)" -> bit<N>/bool/enum with ceil_log2 width + folded constant-expression inits); the SmtProp bullet ("top-level always / eventually only" -> kinds after the task-3G desugar, no new kind needed); the entire "v1 scope diagnostics" paragraph rewritten as "Remaining rejections" - only the var-referencing-init message (identical to exact mode), the >64-bit concrete payload rejection (S2), and defensive/internal checks survive; everything 3C/3D/3F/3G/3H lifted is listed as supported |
| `docs/dev/ALGORITHMS.md` | changed | the SMT-bounded paragraph + mermaid node gain the alive[] liveness gate (task 3A), on_error fallback in the next-state ITE (task 3C), per-step assumption base constraints (task 3C), enum index encoding (task 3D) |
| `docs/dev/cli_commands.md` | changed | McRefOptions field list gains full_properties/full_assumptions; the smt-bounded dispatch sentence now names all four accepted flags and the two smt-only flags' cross-mode usage errors |
| `docs/dev/DECISIONS.md` | changed | D-P4-12's "v1 scope rejections" sentence annotated SUPERSEDED by the Phase 18 tasks 3A-3H corrections (historical text kept per the task file) |
| `docs/dev/validator.md` | changed | the "Formula typing" fragment's Phase-1-stub sentence (site 1) replaced: check_pattern_group/check_output_pattern_group are documented no-ops (task 3F, audit row V17) - a named field is a labeled wildcard that constrains nothing, unlike transition-pattern binds which feed guards |
| `docs/dev/DEPENDENCIES.md` | changed | OUT-OF-SCOPE FINDING fixed per the task's own rule: the prose "both are Phase 1 stubs (formula patterns do not bind variables yet)" + the two mermaid node labels "(Phase 1 stub)" - the exit-criterion grep would have failed without this |
| `DESIGN.md` | changed | section 5.8: the pattern-field table gains the anonymous `Enum::Variant` row (task 3D) and the corner-case note rewritten - `on E(V(Mode::Normal))` documented as SUPPORTED (constrains without binding), with the finding that the constructor name is required (bare `E(Mode::Normal)` does not parse); section 5.10: one paragraph after the atomic-proposition list - named fields in formula patterns are legal and constrain nothing (task 3F), `input_is(Recv(Ack(x)))` == `input_is(Recv(Ack(_)))` |
| `KB.md` | changed (verify-only) | section 7 confirmed to cite no lifted restriction (Spans/Anonymous payloads/Ambiguity/C widths/Feature flags/Error recovery rows only) - left unchanged as expected |
| `docs/llm/SRC_STRUCTURE.md` | changed | the check_pattern_group row rewritten from "rejects named bindings ... (Phase 1)" to the post-3F no-op truth; the check_output_pattern_group row's "always Ok in Phase 1" reworded the same way; two legitimate task-3C description rows rephrased "top-level always/always[N] dropped" -> "outer always / always[N] wrappers dropped" so the exit-criterion grep passes verbatim (meaning unchanged) |
| `docs/llm/mc_engine.md` | changed | the build_smt_spec card row gains the capability note (assumptions + on_error + enum index encoding + bare/next desugaring + constant-expression inits, tasks 3C-3H) |
| `docs/llm/cli_commands.md` | changed | the exit_code_for card row gains the flag-scope note (--full-properties/--full-assumptions SMT-BOUNDED-only, --assumes all three modes) |
| `docs/llm/TASK_MAP.md` | changed | the SMT-bounded-mode row gains the Phase 18 arc map (task -> files for 1A/3A/3C/3D/3F/3G/3H/3L) so a future session navigates from task number to code |
| `docs/llm/CHANGE_IMPACT.md` | changed | the SMT row's affected-files column now includes ref_mc_smt.rs + the generated runtimes in backends/{rust,c}/mc.rs (parity by construction) and states that assumption/on_error/enum specs are affected, not just bit/bool ones; the verify column adds ref_mc_smt_test |
| `V1_R_R_AUDIT.md` | changed | the Implementation-tasks section gains the closure note: 3F (2026-11-24), 3G (2026-11-24), 3H (2026-11-25), 3I (2026-11-25), 3J (2026-11-26) implemented - the audit reads as a closed loop from finding to removal to documentation |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/mc_walkthrough/captures/fail-rust.json` | changed (regenerated by the test run) | only `peak_rss_kb` moved (2660 -> 2444, a run-dependent memory metric); every semantic field identical, all pins still matched - the same behavior recorded in tasks 3F/3G's statuses |

## Changed functionality

None - documentation only. No command behaviour, engine, planner, validator,
or codegen change.

## Removed functionality

None.

## New functionality

- `docs/user/MODEL_CHECKING.md` section 10: the user-facing supported/unsupported/conversion picture for smt-bounded vs exact mode - five residual-difference subsections (D5, D6, the two flag-gated omissions, the sample-0 assumption divergence) each with a verified RFS snippet, plus the closing construct-by-construct summary table. Every snippet was executed against the built toolchain before being documented (see Other important findings).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no test added, changed, or removed - the doc gates (dev_docs_test, docs_md_test) exercise the edited markdown itself |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test dev_docs_test` (9) | OK - pointers, fragment ownership, TASK_MAP coverage, Class B budgets all hold after the edits |
| `cargo test --test docs_md_test` (5) | OK - every edited markdown parses/renders clean, all links resolve (run twice: after 3J.1 and in the full matrix) |
| `rfs-dev sync-check` | OK - clean (after fixing three root-relative path references in the new TASK_MAP arc-map row) |
| `rfs-dev inventory audit` | OK - EXACT: SRC 1037 fns / 217 types, TEST 1146 fns / 35 types, zero per-file diffs (unchanged from the 3L baseline - docs-only task) |
| `cargo build && cargo test` (root, 639) | OK - exactly the task-3L baseline count; all 34 suites green incl. example_projects_test 42/42 (2190 s), backend_parity 11/11, ref_mc_diff 6/6 |
| `cargo clippy --all-targets` | OK - no errors; warning output = the documented pre-existing baseline (dead-code in tests/common/*, type_complexity etc.), zero in any file this task touched (none are source files) |
| `cargo fmt --check` | OK - clean |

## Found problems/issues/mismatches
- Stale code comment (NOT fixed - docs-only task): `src/mc/smt_plan.rs::SmtVar.init`'s doc comment still reads "Initial value (literal inits only; ...)" - stale since task 3H folded constant-expression inits. A future source task should update the comment.
- `docs/dev/DEPENDENCIES.md` carried three stale "Phase 1 stub" sites (prose + two mermaid node labels) that were NOT in the task file's site list - the exit-criterion grep `grep -rn "do not bind variables\|Phase 1 stub" docs/ DESIGN.md KB.md` would have failed without fixing them. Fixed and recorded here per the task's own out-of-scope-sync-fix rule.
- Two legitimate task-3C inventory descriptions in `docs/llm/SRC_STRUCTURE.md` contained the literal string "top-level always/always" that the exit-criterion grep targets (they describe the NEW behavior - outer wrappers dropped - not the old S4 rejection). Rephrased to "outer always / always[N] wrappers" so the verbatim grep passes; meaning unchanged.
- The task file's site-2 verification snippet `transition A on E(V(Mode::Normal)) { next B; }` was re-verified during execution - it validates. Additionally found: the OLD doc line's bare form `on E(Mode::Normal)` does NOT parse ("trailing input after RFS source") because the constructor name is required - the corrected DESIGN.md note states this explicitly.

## Other important findings/information
- Every RFS snippet in the new MODEL_CHECKING.md section 10 was executed against the built toolchain before being documented: D5 counter spec FAILs at step 13 under exact mode, PASSes WITHIN BOUND 10 and FAILs at bound 14 under smt-bounded (the verdict-flip illustration); D6 witness shows `event=Go_G(v=255)` (solver-chosen) vs exact mode's concrete `Go(G(1))` (canonical smallest satisfying the guard) - the snippet needed a guard-referenced payload field, since unreferenced fields are abstracted away entirely and single-class modules render `event=-`; D7/D4 omission warnings + `properties_omitted=1`/`assumptions_omitted=1` suffixes observed verbatim; the sample-0 assumption is rejected by exact mode with exit 1 ("assumption 'force_tick' violated at initial configuration (sample 0)") while smt-bounded returns a vacuous PASS WITHIN BOUND, and the documented workaround `always (next (input_is(Tick)))` PASSes in both modes.
- The sample-0 subsection (10.5) goes beyond the four subsections planned in the task file: task 3C's status explicitly nominated this divergence for the residual-differences section ("candidate for the MODEL_CHECKING.md residual-differences section (task 3E)"), and the merged 3J carries 3E's scope - recorded as a task correction.
- Large-group wildcards are NOT flag-gated: task 3D's status records that profiling a 16-variant group at bound 5 solved in milliseconds, so subsection 10.4 states they are supported by default (per the task file's instruction to check the STATUS files before documenting a flag).
- The mc_walkthrough capture's `peak_rss_kb` fluctuation (2660 -> 2444) is the known run-dependent metric - same pattern as tasks 3F/3G; all semantic fields and pins unchanged.
