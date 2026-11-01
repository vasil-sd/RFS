# Phase 3 - Implementation plan

> Written for a future LLM session: after reading `START.md`, `docs/roadmap/INTRO.md`
> and this file, the assistant must be able to start implementation immediately
> without re-deriving scope. Planning stage completed 2026-10-16; revised
> 2026-10-16 per user roadmap change - backend parity principle (all backends:
> same functionality, identical behavior, cross-tested); awaiting user review.

## Context (state found during planning)

- The Phase 0 parser **already parses** the whole property subset except `always[N]`:
  `!`, `&&`, `||`, `->`, `next`, `always`, `bounded_eventually[N]`, atoms
  (`input_is`, `output_has`, boolean expressions incl. `state == S`), and `assume`
  (src/parser.rs `primary_formula`, lines ~1198-1260). IR already carries all formula
  variants except bounded always (src/ir.rs `Formula`).
- Current codegen translates only `always(phi)` with liveness-free phi into **post-hoc
  trace checks** (`property_check(label, trace)`, `assume_check(label, trace)` in
  src/backends/rust_fsm.rs); everything else is skipped with warnings. The C backend
  mirrors this (src/backends/c_fsm.rs).
- Generated `Fsm { state, vars, trace, active_assumes }` checks assumptions post-hoc on
  the whole candidate trace inside `commit_step` via the `AssumptionHandler` mechanism;
  the MC runtime (src/backends/rust_mc.rs) does random bounded runs and checks property
  labels only after a completed run.
- CLI options are parsed in src/codegen.rs (`parse_codegen_args`: `--out`, `--assumes`,
  `--steps`) and flow through `BackendOptions` into the backend fns; main.rs dispatches.
- No duplicate-name validation exists for `prop`/`assume`; no example uses `always[` or
  `bounded_eventually[0]`; rust_test.rs and tests/integration_example_test.rs do not use
  the post-hoc check functions (verified by grep during planning).
- Training categories `properties_temporal_logic` and `model_checking_properties`
  already exist - training updates go there, no new category needed.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | IR: add `Formula::BoundedAlways` for `always[N]` | `src/ir.rs` (change) | New variant `BoundedAlways { bound: u64, inner: Box<Formula> }`, serde tag `bounded_always`. Additive - no `IR_VERSION` bump (decision D-P3-1); old JSON still deserializes. |
| 2. | Parser: accept `always[N]` | `src/parser.rs` (change) | In `primary_formula`, after the `always` soft keyword parse an optional `[N]` (`int_lit`) before `(`; absent N keeps `Formula::Always`. The only missing operator of the Phase 3 subset. |
| 3. | Printer: print + round-trip `always[N]` | `src/printer.rs` (change) | New arm in `print_formula_ctx` (`always[N] (...)`) plus precedence-helper entry; keeps rfs->json->rfs and json->rfs->rfs round-trips green (frontend tests cover both). |
| 4. | Validator: bound check + duplicate prop/assume names | `src/validator.rs` (change) | (a) Error when `always[N]` has N < 1 (new syntax, no compat concern); `bounded_eventually[0]` stays legal with documented meaning (D-P3-2). (b) New error on duplicate `prop`/`assume` names - prevents generated monitor struct name collisions. |
| 5. | Parser/validator unit tests for `always[N]` | `src/parser.rs`, `src/validator.rs` (change, `#[cfg(test)]`) | Parse + round-trip of `always[N]` at top level and inside boolean contexts; N = 0 rejected; duplicate names rejected. Pins grammar/precedence before codegen work starts. |
| 6. | Shared monitor compiler (new module) | `src/monitor.rs` (new); `src/lib.rs` (change: declare `pub mod monitor`) | Language-agnostic formula -> `MonitorPlan` compiler implementing restrictions R1-R4 (Other information): per-sample boolean signals (temporal-operator-free subformulas), delay registers for `next`, obligation monitors for `bounded_eventually[N]` (trigger = implication antecedent or true), countdown monitors for `always[N]`, top-level violation wiring. Includes a small reference interpreter used only by tests. Rationale: shared per CODEGEN.md architecture; Rust backend renders it (task 8), C backend renders it with identical semantics (task 11) - backend parity principle. |
| 7. | Monitor compiler unit test battery | `src/monitor.rs` (change, `#[cfg(test)]`) | Plan-structure assertions for a formula battery (accepted and rejected by R1-R4); behavioral tests stepping the reference interpreter over hand-crafted sample sequences: the Fin/Ack example from PHASE_3.md (violation confirmed at the exact deadline sample), periodic `always(be[N] p)`, `always[N]` window, `next` delay, same-step satisfaction. Pins semantics D-P3-3 before any codegen renders monitors. |
| 8. | Rust backend: generate online monitors | `src/backends/rust_fsm.rs` (change); `src/codegen_fsm.rs` (change) | Emit per-label monitor structs + aggregate property/assumption monitor state from `MonitorPlan`; new generated `Sample { input, outputs, from_state, to_state, vars }`; refactor the existing pattern-match emission so it works against a `Sample` instead of only a `Trace`. `Fsm` gains `monitors` + sticky `property_violation: Option<PropertyViolation { label, step, trigger_step }>`; `commit_step` steps monitors online (assumption-monitor violation goes through the existing handler path; property violations are recorded); `new()` steps one initial sample (position 0) so countdowns start correctly. Remove post-hoc `property_check`/`assume_check` functions, keep `PROPERTY_LABELS`/`ASSUME_LABELS` (D-P3-4). Update `codegen_fsm.rs` doc comments: formula-subset classification now comes from the shared monitor compiler so both backends warn identically; the Rust path renders monitors via `monitor::compile_formula` (C in task 11). |
| 9. | `StepError::PropertyViolation` + `--with-property-monitors` (shared CLI + Rust) | `src/backends/rust_fsm.rs` (change); `src/codegen.rs` (change); `src/main.rs` (change) | New generated `StepError::PropertyViolation { label: String, step: usize }` variant. New `BackendOptions.with_property_monitors: bool` parsed as `--with-property-monitors`, accepted for the production command of ANY backend (mc/test always include monitors - D-P3-6), threaded through `CodegenRequest::Generate` + main.rs dispatch + help text. Rust production codegen gates monitor emission and stepping under `#[cfg(debug_assertions)]`: debug builds return `Err(StepError::PropertyViolation)` on violation, release builds have zero monitor cost. C-side rendering/gating is task 11 (same option, `#ifdef RFS_PROPERTY_MONITORS`). |
| 10. | Rust MC runtime: online detection + enhanced counterexamples | `src/backends/rust_mc.rs` (change) | After each successful step check `fsm.property_violation`; on hit print the property-specific counterexample required by PHASE_3.md (which property failed, which step triggered the obligation, when the deadline expired, relevant variable values, full input/output sequence), set status `property-violated:<label>`, exit 1. Assumption pruning keeps working through the handler path (now monitor-driven, so liveness assumptions like `always(be[5] input_is(Tick))` prune too). Keep the summary line field names stable. No change to `src/codegen_mc.rs` concepts is required: the status string follows the existing free-form `assumption-violated:<label>` pattern already used by the runtime, and `McConfig`/`McStatus`/`ViolationSpec` stay valid as-is. C MC parity: task 12 (identical output/status). |
| 11. | C backend: online monitors with full parity | `src/backends/c_fsm.rs` (change); `src/backends/C.md` (change) | Render the same `MonitorPlan` in C (backend parity; D-P3-5 revised): per-label monitor structs + aggregate state mirroring the Rust rendering, sample-based evaluation reusing the existing C input/output match helpers, initial sample stepped in `fsm_new`, monitors stepped online in `rfs_commit`; sticky violation record (label/step/trigger_step) plus a new `RFS_PROPERTY_VIOLATION` status in `step_result_t`. Assumption monitors go through the existing handler path; remove post-hoc `rfs_property_check`/`rfs_assume_check` (parity with D-P3-4); keep `PROPERTY_LABELS`/`ASSUME_LABELS`. Production command: monitor emission/stepping behind `#ifdef RFS_PROPERTY_MONITORS`, enabled by `--with-property-monitors` (C analogue of the Rust debug gating, task 9). Out-of-subset formulas warn with the same text as Rust. Update C.md sections 4-5 to describe monitor parity. |
| 12. | C MC runtime: online detection + enhanced counterexamples (parity) | `src/backends/c_mc.rs` (change) | Mirror of task 10 in C: after each successful step check the sticky violation record; on hit print the same property-specific counterexample fields as the Rust MC, set status `property-violated:<label>`, exit 1; identical summary line format and field names. Same-seed runs of the Rust and C MC must be indistinguishable (verified by task 17). |
| 13. | New valid examples with monitorable properties | `examples/valid/41_property_monitors.rfs` (new); `examples/valid/42_bounded_liveness_protocol.rfs` (new) | 41: Fin/Ack-style endpoint protocol - safety prop, `next`, `always(a -> be[N] b)`, `always[N]`, assumption `always(be[5] input_is(Tick))`; every property must compile to a monitor with zero warnings. 42: retry/timeout protocol with periodic liveness and two concurrent obligations. File-driven suite picks them up without test-code changes. |
| 14. | New invalid example (zero bound) | `examples/invalid/invalid_always_zero_bound.rfs` (new) | `always[0] (...)` must fail validation (task 4a); filename follows the existing `invalid_` prefix convention. The suite accepts parse-OR-validation failure. |
| 15. | Codegen tests: option sensitivity + per-command coverage, both backends | `tests/codegen_test.rs` (change) | `--with-property-monitors` on/off changes production output in BOTH backends; flag not accepted/needed for mc/test; monitor structs present in generated code for example 41 in Rust AND C; no skip warnings for in-subset properties in either backend (skip warnings only for out-of-subset formulas, identical wording); per-command generation over all valid examples now includes the new ones automatically. |
| 16. | Integration test: MC catches an injected bug + debug monitors work, both backends | `tests/monitor_integration_test.rs` (new) | (a) Buggy variant of example 41 (Ack-sending transition dropped, written to a temp dir): generate MC in BOTH backends -> rustc / gcc -> run -> expect exit 1 and output containing the same property label plus trigger/deadline step info in both. (b) The correct spec passes within bound in both. (c) Production codegen with `--with-property-monitors`: Rust debug build driving the FSM to a violation returns `Err(StepError::PropertyViolation)`; C build compiled with `-DRFS_PROPERTY_MONITORS` reports the same violation via the step result. Mirrors the Phase 1/2 integration-test pattern. |
| 17. | Cross-backend differential parity test | `tests/backend_parity_test.rs` (new) | The backend parity principle made executable: for every valid example containing properties or assumptions, generate the MC in both backends, compile (rustc + gcc), run with identical `--bound --runs --seed`, and assert equal exit codes, equal summary-line fields (`runs= bound= completed= unexpected= assumption_stops= property_violations=`), and - on any violation - equal property label and trigger/deadline step numbers; also force violations via `--violate` in both backends and compare the counterexample information. Additionally run the generated test executables of both backends and require both to pass (identical coverage from the shared TestPlan). Rationale: backends are tested against each other for maximal confidence in correctness. |
| 18. | Regenerate + run generated test aggregate | `target/rfs_codegen_tests` (via cargo aliases, not checked in) | `cargo generate-code-tests && cargo test-generated-code`: new examples' monitor code must compile and pass; record the updated counts for task 20. |
| 19. | Training-set update | `training/properties_temporal_logic/rfs_training.jsonl` (change); `training/model_checking_properties/rfs_training.jsonl` (change) | Add 2-3 SFT examples on `always[N]` syntax/semantics, the monitor-compilation concept, and restrictions R1-R4; add 1-2 on online monitors in MC, assumption pruning, and enhanced counterexamples. Keep the 2-message format with `< think> ... < /think>` assistant content per KB section 4.6; rfs code blocks become snippets automatically. |
| 20. | Regenerate snippets + aggregate + training tests | `training/<cat>/snippets/**` (change); `all_rfs_training.jsonl` (change) | `cargo extract-training-snippets && cargo build-training && cargo test-training`; update expected snippet/aggregate counts in BUILD.md sections 3-4 if they changed. |
| 21. | Docs sync | `DESIGN.md` (change: section 5.10 add `always[N]` + monitor semantics, section 7 IR note for the additive variant, section 12 mark implemented subset, new section 21 "Phase 3 property/monitor decisions" with D-P3-1..D-P3-6 tables in the D1-D10 style); `KB.md` (change: sections 2.6, 3.9, 4.2 add src/monitor.rs, 1 state table, 7 limitations incl. replacing the post-hoc assumption-check note with online monitors in both backends); `BUILD.md` (change: `--with-property-monitors` row, updated test counts, parity suite row); `CODEGEN.md` (change: BackendOptions field, monitor conventions, testing conventions incl. cross-backend parity tests); `src/backends/RUST.md` (change: Sample, monitor structs, StepError variant, debug hooks, post-hoc check removal); `START.md` (change: section 1 snapshot counts + next work -> Phase 4) | Keeps docs synchronized with design-relevant changes (KB section 6.6 / START section 8). Decision tables follow the style of DESIGN section 20. (`src/backends/C.md` is updated in task 11, not here.) |
| 22. | Roadmap state update + full verification + phase close-out | `docs/roadmap/INTRO.md` (change: phase-state table row 3 -> COMPLETE with PHASE_3_STATUS.md listed); all suites | Update the phase-state table per INTRO section 4; run the BUILD.md section 5 checklist (`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` - includes the new parity + integration suites), `cargo generate-code-tests && cargo test-generated-code`, `cargo run -- validate --input rfs examples/valid` (exit 0), and a `gcc -Wall` spot check of the NEW C monitor output. Then write `docs/roadmap/PHASE_3_STATUS.md` per template 2.3, append the TASKS.md entry, and commit (protocol 3.2 g-i). |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 4a. | Validator: bound check + duplicate prop/assume names | change | Test sources revealed the pre-existing grammar constraint that bare expression children of `&&`/`\|\|` must be parenthesized (primary_formula falls through to the full expr grammar and absorbs following boolean operators) - documented in test comments; no grammar change. |
| 4b. | Validator: bound check + duplicate prop/assume names | change | Duplicate prop/assume name rejection ALREADY exists at parse time (`parser.rs build_module` rejects duplicates within and across the prop/assume lists) - no new parser behavior needed. The validator-level duplicate check is kept as defense-in-depth for IR loaded from JSON (bypasses the parser); tests assert the parse-time rejection plus the validator guard on a hand-built IR. |
| 6. | Shared monitor compiler (new module) | new | Semantics decision pinned during tasks 6-7 (extends D-P3-3): bare top-level signals and `next p` are INVARIANTS anchored at the full range [0, inf), not one-shot position-0 checks. Rationale: sample 0 has no input/outputs, so a one-shot reading would make I/O atoms vacuously false and assumptions like `assume a : input_is(Tick)` would prune every trace; the invariant reading matches the runtime-monitor purpose. Top-level `be[N]` stays one-shot (pending from sample 0) per D-P3-3; periodicity requires explicit `always`. |

| 10a. | Example 33 (german_transfer_tla): give_up_bounded bound | change | Under exact D-P3-3 deadline semantics the GiveUp output fires at trigger+4 (timer = 3 counts down over three ticks), so the old `be[4]` window was off by one; corrected to `be[5]` with an explanatory comment. Discovered while running online-monitor MC over existing examples during tasks 10-12 verification (previous session, before task files existed). |
| 11a. | C backend: online monitors with full parity | change | The C.md sections 4-5 update assigned to task 11 was not made in the previous session (`src/backends/C.md` has no monitor content); folded into task 21 (docs sync) scope, together with `src/backends/RUST.md`. |
| 13a. | New valid examples with monitorable properties | change | Example 42 as written FAILS TO PARSE: the `retry_responds` antecedent `state == Sending && output_has(Sent)` hits a parser constraint - `primary_formula` falls through to the full expr grammar, whose `primary_expr` accepts bare identifier paths, so `output_has` is consumed as a variable reference and `(Sent)` becomes trailing input ("trailing input after RFS source"). Fix (no grammar change, consistent with 4a): reorder the antecedent - conjunction is commutative, semantics unchanged; verified to parse. Accepted/rejected shapes pinned by new parser unit tests; constraint documented in KB.md (task 21). |

| 23. | Grammar fix: formula-level boolean operators with mixed atom kinds (+ tests) | new | Added per user request (2026-10-16); SUPERSEDES the workaround in 13a - example 42 keeps its original text. Root cause: `primary_formula`'s expr fallthrough uses the FULL expression grammar (logic_or/logic_and include `&&`/`||`; primary_expr accepts bare identifier paths), so `state == S && output_has(O)` absorbs `output_has` as a variable reference and leaves `(O)` as trailing input. Fix: restrict the fallthrough to the NON-BOOLEAN expression level (equality - the level below logic_and); formula-level `&&`/`||` then build Formula::And/Or with each operand parsed via primary_formula; printer round-trip verified; parser + monitor + round-trip tests added. Pure-expression conjunctions in properties now parse as Formula::And(Expr, Expr) instead of Expr(Binary(LogicAnd)) - additive for JSON (old IR still deserializes), no IR_VERSION bump; transition-guard grammar unaffected (guards call expr() directly). Executes BEFORE task 13. |
| 24. | Example 13 self-violation: generated test executable fails in BOTH backends (by design) | new | Discovered during task 17 execution (2026-10-17): `examples/valid/13_properties_safety.rfs` fails its OWN generated `test` executable identically in both backends - the shared TestPlan's positive-sequence walk takes ToBad -> Bad at step 1, violating the example's own `no_bad` property, and `test` codegen always includes monitors (D-P3-6); the run then aborts on the no-progress assertion. NOT a backend divergence (parity holds - pinned by task 17's parity suite); it is a spec-level mismatch (the property forbids a state the representative walk deterministically reaches, with no assumption preventing it). Consequence: `cargo test-generated-code` (task 18) and the close-out full verification (task 22) fail on example 13 until resolved. Follow-up: resolve the example/test-plan mismatch (candidate fixes in PHASE_3_TASK_24.md; preferred = minimal spec change to example 13, fallback = assumption, last resort = shared TestPlan rule) and update task 17's pinned failing set to empty. Executes AFTER task 17, BEFORE tasks 18/22. |

(corrections are appended here during implementation; the Planned tasks table above is frozen)

## Other information

Plan revision 2026-10-16 (pre-approval, so the Planned tasks table was edited
directly instead of via corrections): user changed the roadmap - all backends must
have identical functionality and exactly the same behavior, kept in sync and tested
against each other. Consequences: D-P3-5 rewritten (C monitor parity is now IN
scope), new C tasks 11-12, new cross-backend differential test task 17, tasks
renumbered (old 16-20 -> 18-22), exit-criteria mapping updated.

### Decisions made during planning (record here; move to DESIGN.md section 21 in task 21)

- **D-P3-1 (IR):** `BoundedAlways` is an additive serde variant (tag `bounded_always`);
  no `IR_VERSION` bump. Existing JSON round-trips unchanged.
- **D-P3-2 (bounds):** `always[N]` requires N >= 1 (validator error otherwise, since it is
  new syntax). `bounded_eventually[0]` remains legal and means "the body must hold at the
  trigger sample itself". No existing example uses either form (verified by grep).
- **D-P3-3 (monitor semantics - exact, pinned by task 7 tests before codegen):**
  Sampling follows DESIGN section 11: per-sample values are post-step; `input_is`/
  `output_has` refer to the transition just taken; sample 0 is the initial configuration
  (no input/outputs, so both atoms are false there). An obligation triggered at sample i
  requires its body to hold at some sample j with i <= j <= i + N - 1. The violation is
  confirmed at the last allowed sample (i + N - 1) when the body still fails.
  Obligation monitor algorithm per sample s:
  ```text
  a = eval(trigger signal at s)   # implication antecedent, or true for bare obligations
  b = eval(body signal at s)
  if !pending and a: pending = true; remaining = N; trigger_step = Some(s)
  if pending:
      if b: pending = false; trigger_step = None
      else: remaining -= 1; if remaining == 0: VIOLATION confirmed at sample s
  ```
  Worked check (N = 3, trigger at i, body never holds): s=i rem 3->2, s=i+1 rem 2->1,
  s=i+2 rem 1->0 -> violation confirmed at i+2; if the body holds at any of i..i+2 the
  obligation clears. (PHASE_3.md's conceptual pseudocode reports one sample later;
  confirming at the deadline sample itself is chosen for tighter counterexamples.)
  `always[N] p`: p must hold at samples 0..N-1 (countdown; violation at the first failing
  sample). Top-level `be[N] p`: obligation pending from sample 0. `next p` in a monitor:
  delay register initialized to true (vacuous at sample 0), matching current post-hoc
  semantics.
- **D-P3-4 (online replaces post-hoc):** the generated `property_check`/`assume_check`
  trace functions are removed in BOTH backends (C: task 11); monitors step online in `commit_step`/`rfs_commit`. Assumption
  enforcement stays on the existing `AssumptionHandler` path so all MC/test call sites
  (trial-step forced violations, `--assumes`, integration tests) keep working unchanged.
  `PROPERTY_LABELS`/`ASSUME_LABELS` constants stay.
- **D-P3-5 (C backend - REVISED 2026-10-16 per user roadmap change):** full monitor
  parity in Phase 3. The backend parity principle (`INTRO.md`) requires all backends
  to have the same functionality and exactly the same behavior: the C backend renders
  the same `MonitorPlan` (task 11), its MC runtime gets online detection + enhanced
  counterexamples (task 12), and cross-backend differential tests (task 17) verify
  Rust and C behave identically. Only out-of-subset formulas are skipped, with the
  same warning text in both backends.
- **D-P3-6 (CLI):** `--with-property-monitors` applies to the production command of
  every backend; mc and test always include monitors. Gating is per-language with the
  same zero-cost-when-disabled semantics: Rust via `#[cfg(debug_assertions)]`, C via a
  compile-time define (`RFS_PROPERTY_MONITORS`). New generated
  `StepError::PropertyViolation { label, step }` variant (Rust) /
  `RFS_PROPERTY_VIOLATION` status in `step_result_t` (C).

### Monitor-compilability restrictions (R1-R4) - enforced by `monitor::compile_formula`

- **R1:** no `bounded_eventually` under `not` (directly or indirectly).
- **R2:** a `bounded_eventually[N] b` node requires `b` to be temporal-operator-free and
  may appear (a) at top level, or (b) inside an `always`/`always[N]` body, except under
  another `bounded_eventually` and not under `next`. Its trigger signal is the
  implication antecedent when it is directly the consequent of `->`, else true.
- **R3:** `always[N]` only at top level or under boolean connectives - never under
  `next`, `always`, `always[N]`, or `bounded_eventually`.
- **R4:** `next` allowed anywhere safety is allowed, except inside a bounded-eventually
  body (see R2).
- Formulas outside R1-R4: parse + validate succeed; codegen in EVERY backend emits
  the same warning ("outside monitor subset; skipped") and generates no monitor.

### Exit criteria mapping (PHASE_3.md)

| Exit criterion | Covered by tasks |
|---|---|
| PSL-like properties parse and validate | 1-5, 13-14 |
| monitors are generated in ALL backends with identical behavior (no skip warnings for in-subset formulas) | 6-9, 11-12, 15, 17 |
| cross-backend differential tests pass (same specs/seeds -> same results, labels, counterexample info) | 10, 12, 16-17 |
| model checker can verify safety and bounded liveness properties in all backends | 8, 10, 12, 16-18 |
| assumptions restrict input sequences correctly in all backends | 8, 10, 11, 12, 16-17 |
| counterexamples include property-specific information in all backends | 10, 12, 16, 17 |
| optional runtime property monitors compile and work in debug mode in all backends | 9, 11, 16 |

### Verification commands (task 22)

```sh
export RUSTUP_HOME=/workspace/host/rustup && export CARGO_HOME=/workspace/host/.cargo \
  && export PATH="$CARGO_HOME/bin:$PATH" && cd /workspace/host/RFS
cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check
cargo generate-code-tests && cargo test-generated-code
cargo run -- validate --input rfs examples/valid   # exit 0 expected
gcc -Wall <generated C output>                      # spot check, NEW monitor output compiles clean
# tests/backend_parity_test.rs runs inside `cargo test` (rustc + gcc, same seeds)
```

### Assumptions to re-check at implementation start (all verified during planning)

1. No example/training file uses `always[` or `bounded_eventually[0]`.
2. `rust_test.rs` and `tests/integration_example_test.rs` do not reference the post-hoc
   check functions being removed.
3. The MC summary line field names (`runs= bound= completed= unexpected=
   assumption_stops= property_violations=`) are referenced by docs/tests - keep stable.
4. Duplicate prop/assume name validation is currently absent (task 4 adds it).
5. The C side mirrors Rust exactly today (same `fsm_t` shape, same post-hoc
   `rfs_assume_check` in the shared `rfs_commit`), so tasks 11-12 are a direct port
   of tasks 8/10 to C; gcc is available at /usr/bin/gcc (used since Phase 2).

### Risks

- Off-by-one in obligation/`always[N]` semantics: mitigated by the reference interpreter
  + task 7 tests written BEFORE any backend renders monitors.
- Pattern-match emission refactor for `Sample`: reuse the existing match-arm generation
  from rust_fsm.rs (currently trace-based); keep Trace helpers for counterexample output.
- Generated-code size growth (monitor structs): small fixed-size structs per label;
  acceptable for finite specs.
- C rendering drift from Rust semantics (tagged-union sample matching, countdown
  state): mitigated by the shared `MonitorPlan` (single source of truth), task 7's
  reference interpreter, and the cross-backend differential suite (task 17) which
  fails on any behavioral divergence.
