# Phase 4, Task 13 - Implementation status

> Written once when all exit criteria of `PHASE_4_TASK_13.md` are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | No new files in `src/`; the task is documentation sync plus one parser-diagnostic fix. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/parser.rs` | changed | USER-REQUESTED addition beyond the planned (docs-only) scope: dedicated parse diagnostic for `eventually` without parens. On ANY parse failure `parse_source` now first runs a targeted scan (`find_eventually_missing_paren`: whole-word match excluding `bounded_eventually`, string literals skipped, whitespace before the body position) and reports `expected '(' after 'eventually' (the body must be a parenthesized formula)` at the keyword's exact position - previously the top-level `many0(module)` recovery swallowed the failure into the generic "trailing input after RFS source". 3 new unit tests (dedicated message+span; generic fallback for other failures; identifier named `eventually` outside formulas still parses). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | changed | §5.10 property list (+ `eventually (f)` / `p until q`, MC-only note, weak_until superseded); §6 EBNF formula ladder (new `until_formula`/`neg_operand` levels between `&&` and primary; `eventually` primary; precedence comment); §7 additive Phase 4 IR variants (`eventually`/`until`, no version bump); §8.3 dedicated diagnostic note; §9.7 final status taxonomy pointer; §11 REWRITTEN around the exact engine (D-P4-1 architecture + shared plan, exact BFS + canonical order, statuses/summary line D-P4-4, bounded/truncated checking, SMT mode D-P4-12, flags/parallelism incl. task 10B) - finalizes task 6's liveness formalization; §12 subset table (eventually/until implemented MC-only; planned subset trimmed); NEW §22 "Phase 4 model-checking decisions (as shipped)" D-P4-1..D-P4-14 + plan/task corrections. |
| `KB.md` | changed | §1 state table (new Phase 4 row, 45 valid examples, 230-test breakdown / 522 generated, 231 training examples, codegen row); §2.6 liveness operators + weak-until/precedence/scope notes; §3.9 liveness example lines; §4.5 MC codegen flags (codegen-time defaults + runtime overrides + SMT note); §6.5 dedicated-diagnostic corner case; §12 next-work line current; §13 superseded note + 522 count fix. |
| `BUILD.md` | changed | §3 suite table to final Phase 4 state: 230 pass with per-suite breakdown (+2 with `--features smt`), 45 valid examples, 522 generated tests; §4 command rows: 179 modules, 130/127 snippets, 231 aggregate. |
| `CODEGEN.md` | changed | §2 file map (codegen_mc.rs exact-mode concepts; new `mc_plan.rs` and `smt_plan.rs`/`smt_emit.rs` rows; rust_mc/c_mc descriptions); §4.1 Generate variant fields + MC flag defaults + parse-error list; §5.3 rewritten for exact mode (ExactStatus, summary line, defaults, fuzz legacy types); §8 parity description (seedless exact mode, no `--violate`, benchmark suite). |
| `src/backends/RUST.md` | changed | §1 rust_mc row; §4 REWRITTEN: two generate_model_checker shapes (SMT standalone vs core+exact+fuzz+dispatch), exact engine details (packed configs, enqueue-visited, canonical order, deadlock/unexpected semantics, assumption pruning, liveness engine, two-phase `--threads` + runtime `--fast`), embedded plan tables note. |
| `src/backends/C.md` | changed | §1 c_mc row; §5 REWRITTEN: mirror of the Rust exact engine (sorted-vector containers, gcc -Wall zero-warning bar, byte-identical summary/counterexamples), pthreads + C11 atomics for `--threads`, SMT branch linking the same libz3 5.3.1 (API deltas in generated code). |
| `START.md` | changed | §1 snapshot -> Phase 4 complete at implementation level (tasks 1-13; close-out task 14 pending) + final baseline numbers; "Next work" -> task 14; §3 source tree adds mc_plan/monitor/smt_plan+smt_emit rows, codegen_mc description updated. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `training/properties_temporal_logic/rfs_training.jsonl` | changed | Task 12's invalid-variants entry now quotes the NEW dedicated diagnostic verbatim (was "trailing input after RFS source") so training data matches shipped behavior. |
| `training/*/snippets/**`, `all_rfs_training.jsonl` | regenerated | No count change (message text only): 130 valid / 127 invalid snippets, 231 aggregate lines. |

## Changed functionality

- Parse diagnostics: `eventually` without a parenthesized body now reports the dedicated message at the keyword position instead of "trailing input after RFS source" (user-requested; every previously VALID spec parses identically - the scan only runs on parse failure).

## Removed functionality

- None.

## New functionality

- Documentation: first complete record of the shipped Phase 4 design in the design docs - DESIGN.md §22 (D-P4-1..D-P4-14 as shipped, incl. plan corrections) and the rewritten §11 exact-engine section; all other design-relevant docs synchronized (stale sweep clean).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `parser::tests::eventually_missing_paren_dedicated_diagnostic` | new | full-module spec with parens-less `eventually` -> exact dedicated message + span at the keyword offset |
| `parser::tests::other_parse_failures_keep_generic_message` | new | missing closing brace (no eventually pattern) -> generic "trailing input after RFS source" unchanged |
| `parser::tests::identifier_named_eventually_outside_formulas_still_parses` | new | `var eventually: bool` + guard usage parses fine (no false positive on the success path) |

## Tests run status

| Name | Status |
|---|---|
| `cargo extract-training-snippets` + `cargo build-training` | OK (130 valid / 127 invalid snippets; 231 aggregate - unchanged) |
| `cargo test-training` | OK (4/4) |
| `cargo test` | OK (**230/230**; lib 125 incl. the 3 new parser tests) |
| `cargo test --features smt` (z3 env per TOOLCHAIN.md 5.2) | OK (**232/232**) |
| `cargo clippy --all-targets` | OK (baseline only: 11 lib + 1 codegen_test pre-existing warnings, zero new) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- Stale sweep found exactly ONE live stale description: KB §13.4 presented the fuzz runtime options (`--bound/--runs/--seed/--violate/...`) as the current MC CLI - fixed with a superseded note pointing at DESIGN.md §11 + CODEGEN.md §5.3. All other fuzz/same-seed/`--violate` mentions live in FROZEN historical phase files (`docs/roadmap/PHASE_3*`, `PHASE_4.md` planning text) which are not rewritten per protocol.
- No "z3 not installed" claims anywhere; TOOLCHAIN.md §5 (z3 5.3.1 installed + verified) and BUILD.md SMT notes already agree.

## Other important findings/information

- For task 14 (close-out): final verification numbers confirmed by this task's runs - `cargo test` **230/230** default, **232/232** with `--features smt`; generated-code tests **522/522** (recorded at task 10B; no codegen-template changes since); 45 valid + 12 invalid examples; snippets 130 valid / 127 invalid; aggregate 231 lines.
- The parser diagnostic fix is a user-requested addition beyond task 13's planned scope (docs sync) - recorded here per protocol; it changes no pinned design decision, so no plan correction entry is required.
