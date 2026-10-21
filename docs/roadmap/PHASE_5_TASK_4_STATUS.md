# Phase 5, Task 4 - Implementation status

> Written once when all exit criteria of `docs/roadmap/PHASE_5_TASK_4.md` are
> reached. Not rewritten afterwards; later fixes go into TASKS.md entries and
> follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | The task is a pure extension of `src/codegen_test.rs` (planner only - NO backend changes, per the exit criteria). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen_test.rs` | changed | Extended shared test planner: existing module-level fields kept (`steps`, `expect_progress`, `definite_unexpected`, `has_on_error`) + five new plan components (all plain data, language-agnostic): (1) `transitions: Vec<TransitionTestCase>` - per-transition cases driving from `t.state` with a concrete representative input matching `t.pattern` (pattern literals flow into the input; bindings get their literal/representative values) and an installed variable configuration; guarded transitions get statically computed TAKEN and NOT-TAKEN cases via candidate-value search (bool `[false,true]`, bit<N> `[0,1,2^N-1]`, enum = all declared variants; lexicographic enumeration, `MAX_STATIC_ASSIGNMENTS = 4096` cap); a taken case requires every higher-firing sibling (priority desc, source order - mirrors generated fsm_step) to be disabled by the SAME assignment; an unguarded earlier sibling SHADOWS the transition (noted, no cases); a not-taken case is planned only when statically observable (no matching sibling can fire with an identical next_state+emits signature, on_error included); (2) `invariants: Vec<InvariantCase>` - properties of shape `always [N] (atom)` where the atom is liveness-free (bounded_eventually / eventually / until all excluded, per mc_plan semantics) and per-sample (`monitor::formula_to_signal` succeeds); (3) `monitor_violations: Vec<MonitorViolationCase>` - for each in-subset property `always [N] (p)` where p is a boolean atom over state/vars (`SignalAtom::VarExpr`), the planner constructs the violating configuration: case 1 = initial config already violates (violation at sample 0 of Fsm::new, no step needed); case 2 = search states x candidate assignments x outgoing transitions for a one-step drive whose POST-STEP config (simultaneous assignment semantics) violates the atom - monitors evaluate post-step samples; other shapes skipped with `PlannerNote`; (4) `random_trace: RandomTraceCase` - deterministic LCG with the SAME constants as the fuzz wire format (6364136223846793005 / 1442695040888963407, top-53-bit double mapping, `pick_index` semantics), fixed seed `RANDOM_TRACE_SEED = 12345`, input sequence precomputed at codegen time; (5) `fuzz_decode: FuzzDecodePlan` - per input variant the field decode strategy for the libfuzzer/libFuzzer skeletons (bool = 1 byte non-zero, bit<N> payload = ceil(N/8) masked bytes, enum = representative value). Plus `notes: Vec<PlannerNote>` for every documented fallback. Private static evaluator (`eval_expr`/`eval_bool`) mirrors the Rust backend lowering EXACTLY: u128 wrapping arithmetic masked to the validator-typed width (IntLit -> 8), shift-by->=width -> 0, bool-typed bitwise ops as logical ops; widths come from `validator::type_expr` with pattern bindings inserted into a proper `Env`. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | changed | Section 5.2 rewritten: full TestPlan component list, candidate-value scheme, static-analysis rules (shadowing, sibling disabling, observability), LCG constants + seed, note semantics. |
| `BUILD.md` | changed | Section 3 counts actualized (326 root / 334 workspace; lib 158 -> 178 incl. the new planner suite). |

## Other file changes

- `TASKS.md`: one new entry (2026-10-21).
- `docs/roadmap/INTRO.md`: phase-state row 5 -> "tasks 1-4 + 8-9 of 16 done".

## Changed functionality

- None to existing commands. `build_test_plan` keeps its signature; the four backend call sites (rust_test/c_test) only read the pre-existing fields, so all rendered output is byte-identical (generated-code suite 537/537 unchanged proves it).

## Removed functionality

- None.

## New functionality

- The shared planner now emits complete data for five additional test families (per-transition cases with guard true/false static analysis, safety-invariant re-checks, monitor-violating injected traces, seeded random-trace input sequence, fuzz decode plan) - consumed by the Rust/C test renderers in tasks 5/6.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `lcg_pinned_sequences_match_fuzz_wire_format` | new | LCG byte-pins: seed 12345, len 2 x 8 -> [0,0,1,1,0,1,1,0]; len 5 x 10 -> [0,1,4,4,1,2,3,1,4,0] (cross-checked against an independent implementation before pinning) |
| `random_trace_case_is_deterministic_and_pinned` / `..._empty_inputs` | new | plan-level random-trace pins incl. the empty-inputs edge case |
| `unguarded_transition_gets_single_taken_case` / `guarded_transition_pins_both_cases_with_concrete_values` | new | unguarded -> single taken case (representative vars); guarded x<8 -> taken pin x=0 + not-taken pin x=15 with only_candidate + on_error=None |
| `unsatisfiable_guard_by_candidates_gets_note_and_not_taken_only` | new | x==8 unreachable by candidates [0,1,15] -> documented note + not-taken-only (the task's fallback) |
| `shadowed_transition_is_noted_and_gets_no_cases` / `higher_priority_sibling_must_be_disabled_for_taken_case` | new | unguarded earlier sibling shadows (note, zero cases); priority-2 sibling must be disabled by the taken-case assignment (x=15 pin) |
| `pattern_literals_flow_into_input_and_guard_evaluation` | new | `Recv(P(n: 5)) where n == 5`: literal 5 in the input case AND in guard evaluation (binding values from the pattern) |
| `not_taken_with_on_error_pins_fallback_case` / `..._without_on_error_is_unexpected_event` | new | on_error fallback OnCase pin vs pure UnexpectedEvent expectation |
| `invariants_select_safety_only_on_example_43` / `invariants_cover_bounded_always_and_skip_liveness` | new | example 43: exactly window_bound selected (eventually/until excluded); bounded always[5] bound pin; bounded_eventually-under-always excluded |
| `monitor_violation_at_initial_configuration_needs_no_step` / `..._one_step_drive_pins_post_configuration` | new | sample-0 violation (step_input None) and one-step drive with pinned post-state/post-vars |
| `example_43_monitor_violation_uses_bit_wraparound` | new | real-world case: next_seq=0, acked=1 -> 0-1 wraps to 15 > 4 in bit<4> - the planned violating config is exactly that |
| `monitor_violation_non_atom_shapes_are_skipped_with_notes` / `planned_violating_traces_fire_the_reference_monitor` | new | input_is atom + obligation shapes noted; EXIT-CRITERION CHECK: the planned traces stepped through the Phase 3 `ReferenceMonitor` fire EXACTLY the target property at the expected sample (0 or 1) and never earlier |
| `fuzz_decode_covers_every_variant_with_field_strategies` / `example_43_fuzz_decode_pins_payload_widths` | new | strategy pins per field kind; example 43: Tick (no fields) + Ack_Seq (PayloadBytes 4/32) |
| `evaluator_mirrors_backend_width_semantics` | new | static evaluator vs backend semantics: bit<4> wraparound, shift-by->=width -> 0, u128-masked arithmetic |
| `planner_runs_on_all_valid_examples` | new | corpus robustness: all 45 valid examples plan without error; structural sanity (real transition ids, in-bounds random indices, fuzz decode covers all variants) |

The three pre-existing planner tests are unchanged and still pass.

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root package) | OK (**326/326**; lib 158 -> 178 incl. the 20 new planner tests; was 306) |
| `cargo test --workspace` | OK (**334/334**; was 314) |
| `cargo test-generated-code` | OK (537/537 - NO backend changes in this task, rendered output byte-identical) |
| `cargo clippy --all-targets` | OK (zero new warnings; same 12 pre-existing baseline) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- C4-1: pattern LITERAL fields (`Recv(P(5))`) create NO binding in the parser/validator (only named fields do) - a guard referencing such a field is an unknown identifier. The planner's input-case builder handles both shapes; test specs use named literals (`P(n: 5)`).
- C4-2: event declarations do not accept typed-pattern syntax (`event E2(x: bit<4>)` is a parse error - that syntax is pattern-only); struct variants need a name (`E2(X(f: bit<4>))`). Caught while writing test specs; no tooling change needed.
- C4-3: `contains_liveness` in `codegen_fsm.rs` (Phase 1) does NOT treat Phase 4's `eventually`/`until` as liveness - the planner therefore uses its own local `has_liveness_op` (mc_plan semantics) instead of reusing it. Flagged for task 15 (docs sync) / a possible cleanup; no behaviour change made here.

## Other important findings/information

- For tasks 5/6 (renderers): the plan is fully self-contained data - `InputCase`/`VarValue` render through each backend's existing identifier/type rules (same path as `representative_input_expr`), `SignalExpr` renders through the existing monitor signal machinery, and `MonitorViolationCase` installs state/vars directly (both are pub fields of the generated Fsm) before stepping. The reference-interpreter test (`planned_violating_traces_fire_the_reference_monitor`) is the template for how the rendered tests should assert.
- For task 10 (MC counterexample JSON): `VarValue`/`InputCase`/plan shapes are plain structs (serializable-friendly) as required; nothing serializes them yet.
