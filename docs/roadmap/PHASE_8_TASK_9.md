# Phase 8, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/codegen_core.md` (Class A deep dive): full content on the
language-agnostic codegen core - the backend plugin API + registry + CLI
parsing (`src/codegen.rs`), the shared production-FSM helpers
(`src/codegen_fsm.rs`), the static unit-test planner (`src/codegen_test.rs`),
and the model-checker concepts (`src/codegen_mc.rs`) - the parity principle:
one plan, rendered identically in Rust and C. Also fill in the matching Class
B card `docs/llm/codegen_core.md` (within its 150-line budget) - per the
confirmed scope decision (plan Corrections entry 3).

## Context

- **Skeleton state (from task 1.5)**: `docs/dev/codegen_core.md` exists with
  one anchored fragment `dev:codegen-core-overview` (owner: ALL FOUR files -
  each file's `//! Docs:` line already names it) followed by a TBD line;
  `docs/llm/codegen_core.md` exists with budget header `<!-- llm-budget: 150 -->`,
  fragment `llm:codegen-core` (same four owners), and a TBD line. Existing
  anchor + heading stay byte-stable.
- **Fragment structure**: keep `dev:codegen-core-overview` for Overview; add
  four new sub-fragments, one per source file:
  - `dev:codegen-plugin-api` - Backend plugin API + registry + CLI (`src/codegen.rs`)
  - `dev:codegen-fsm-helpers` - Production FSM shared helpers (`src/codegen_fsm.rs`)
  - `dev:codegen-test-plan` - Unit-test planning (`src/codegen_test.rs`)
  - `dev:codegen-mc-concepts` - Model-checker concepts (`src/codegen_mc.rs`)
  ORPHAN RULE: the new IDs go on the owner file's `//! Docs:` line in the same
  action. Exact target lines:
  - `src/codegen.rs` line 17 becomes:
    `//! Docs: dev:codegen-core-overview, dev:codegen-plugin-api, dev:algorithms-ir-codegen, llm:codegen-core, dev:code-map-codegen, dev:deps-fn-codegen-rust`
  - `src/codegen_fsm.rs` line 16 becomes:
    `//! Docs: dev:codegen-core-overview, dev:codegen-fsm-helpers, dev:algorithms-ir-codegen, llm:codegen-core`
  - `src/codegen_test.rs` line 31 becomes:
    `//! Docs: dev:codegen-core-overview, dev:codegen-test-plan, dev:algorithms-ir-codegen, llm:codegen-core`
  - `src/codegen_mc.rs` line 20 becomes:
    `//! Docs: dev:codegen-core-overview, dev:codegen-mc-concepts, dev:algorithms-ir-codegen, llm:codegen-core`
- **No duplication with existing fragments**: CODEGEN.md (repo root, 523
  lines) is the EXTENSION guide - plugin API walkthrough, CLI surface, shared-
  module summaries incl. backend rendering details, adding-a-backend checklist,
  testing conventions, invariants; ALGORITHMS.md `dev:algorithms-ir-codegen`
  carries L1/L2/L3 of the pipeline (plugin + registry, options bag, planning
  invariants); DEPENDENCIES.md `dev:deps-fn-codegen-rust` has the core call
  graph; CODE_MAP.md `dev:code-map-codegen` has the file entries. The deep dive
  goes DEEPER: per-function detail (flattening naming scheme, the exact
  expect_progress computation, sibling/firing-order analysis, the capped static
  search and its candidate values, not-taken observability, monitor-violation
  search incl. the C5-1 sibling skip, LCG constants, the static evaluator's
  width semantics), pinned formats (summary line, JSON report key ordering,
  wire formats, test-name scheme), invariants, and change guidance.
  Cross-link; never restate.
- **Verified source material (measured 2026-10-30; line numbers for
  navigation only - doc references use item names)**:
  - `src/codegen.rs` (657 lines, no unit tests): `AssumptionSelection` (29:
    All/None/Labels + FromStr), `McMode` (38: Exact default / Fuzz legacy /
    SmtBounded z3-based + FromStr with pinned unknown-mode error),
    `NO_STD_PRODUCTION_ONLY_ERROR` (85) + `PROTOCOL_FSM_MC_ERROR` (92) - pinned
    rejection strings shared by CLI parser and backend entry points,
    `BackendOptions` (99: assumptions, steps, no_std, with_property_monitors,
    mc_mode, mc_max_states/nondet/timeout_sec/threads/smt_bound,
    counterexample_format, with_protocol_fsm, spec_hash; Default at 168),
    `PropertyMonitorMode` (185: Off/Gated/Always - backends map Gated to their
    compile-time debug switch), `ExtraFile` (200) + `CodegenOutput` (209:
    code/warnings/extra_files), `BackendCommandInfo` (225), `Backend` (242:
    name/description/commands + three fn pointers), registry
    `ALL_BACKENDS`/`BACKENDS` (259/261: C + Rust), `list_backends` (264) /
    `get_backend` (269) / `common_help` (274) / `backend_help` (340 - rendered
    from registry data only), `CodegenCommandKind` (362: Production/UnitTests/
    ModelChecker; CLI names production/test/mc; `from_str` 373, `info` 383,
    `run` 401 = standard dispatch: stamp spec_hash16 over the WHOLE file's
    canonical IR JSON into a cloned options set, generate for the FIRST module;
    `run_module` 420 = per-module library dispatch, caller owns spec_hash),
    `CodegenRequest` (436), `parse_codegen_args` (465: all-errors collection,
    command-scoped rejections of --with-property-monitors/--no-std on
    non-production and --with-protocol-fsm on mc) + `finish` (651).
  - `src/codegen_fsm.rs` (231 lines; production 1-174, `mod tests` 176-231 with
    3 tests): `VariantInfo` (22: group/variant/base_name/unique_name/fields),
    `FsmVariants` (40 + input_variant/output_variant lookups),
    `build_variants` (63: naming scheme - single-variant group whose variant
    name equals the group name keeps the bare name, otherwise group_variant;
    duplicates get numeric suffixes Name, Name2, ...; fields in declaration
    order; NOT yet target-language mangled), `RepValue` (109: Bool/Int/
    EnumVariant/EnumEmpty/EnumUnknown) + `representative_values` (125: bool ->
    false, bit<N> -> 0, enum -> first declared variant as full path),
    `contains_liveness` (151: Phase 1 subset - bounded_eventually only) +
    `safety_formula_inner` (167: always(phi) with liveness-free phi; None
    otherwise). Legacy Phase 1 helpers kept for tests - monitor compilability
    now comes from src/monitor.rs.
  - `src/codegen_test.rs` (2320 lines; production 1-1473, `mod tests`
    1475-2320 with 27 tests): plan data types - `TestPlan` (45: steps,
    expect_progress, definite_unexpected, has_on_error, transitions,
    invariants, monitor_violations, random_trace, fuzz_decode, notes),
    `VarValue` (81: Bool/Int/Enum full path), `InputCase` (92), `OutputCase`
    (102: args Vec<Option<VarValue>> - None where not statically evaluable),
    `TransitionExpectation` (114: Taken{next_state, outputs} | NotTaken{
    only_candidate, on_error, signature}), `OnCase` (138),
    `TransitionTestCase` (145: transition_id/from_state/input/vars COMPLETE
    configuration/expectation), `InvariantCase` (161: label/bound/signal),
    `MonitorViolationCase` (173: label/bound/atom/state/vars/step_input None
    when initial config already violates/post_state/post_vars),
    `RandomTraceCase` (197: seed/steps/input_indices precomputed),
    `FuzzDecodePlan` (210) / `FuzzVariantDecode` (219) / `FuzzFieldStrategy`
    (230: ByteNonZero/PayloadBytes{width}/Representative), `PlannerNote` (242),
    `RANDOM_TRACE_SEED` (250, = 12345). Planner: `expect_progress_from_initial`
    (266: EXACT computation - per-variant representative candidates; field
    literals must accept the representative value (int == 0, bool == false,
    enum == first variant); guard evaluated under initial vars + binding
    representatives; the old !transitions.is_empty() heuristic over-
    approximated - Phase 6 task 6 finding, spec 47), `build_test_plan` (349),
    `has_definite_unexpected` (369: a variant no pattern matches; bare group
    marks ALL its variants used), `is_deadlocked_initial` (401). Per-
    transition: `MAX_STATIC_ASSIGNMENTS` (412, = 4096), `ConcreteInput` (416),
    `plan_transitions` (427: siblings = same state + pattern can match this
    input; firing order = descending effective priority (unwrap_or(0)) then
    source order, stable sort - mirrors generated fsm_step; unguarded earlier
    sibling SHADOWS t (note, no taken case); static search over guard-relevant
    vars for taken (t guard true + all earlier siblings disabled) and not-
    taken (t guard false + observable)), `earlier_sibling_fires` (611),
    `pattern_can_match_variant` (644), `concrete_input_for_pattern` (658: named
    variant or group's first for whole-group; literal fields kept,
    wildcard/bind at representatives; binding_vals keyed by BINDING name),
    `not_taken_observable` (721: no matching sibling fires with identical
    signature (next_state + emits); on_error landing must differ too). Invariants:
    `select_invariants` (753: always/always[N] of a liveness-free per-sample
    signal via monitor::formula_to_signal; temporal inside -> skip),
    `has_liveness_op` (780: bounded_eventually/eventually/until - superset of
    codegen_fsm::contains_liveness, mirrors mc_plan). Monitor violations:
    `plan_monitor_violations` (804: only `always [N] (atom)` where the atom is
    a state/vars VarExpr - input_is/output_has not planned in v1; case 1
    initial config
    already violates (step_input None); case 2 search state x assignment x
    outgoing transition for a one-step drive whose POST-STEP config violates,
    skipping transitions an earlier sibling would take first - task 5
    correction C5-1). Random trace: `Lcg` (950: state = state * 6364136223846793005
    + 1442695040888963407 wrapping; top-53-bit double mapping; pick_index -
    SAME constants as the fuzz wire format), `build_random_trace` (976). Fuzz:
    `build_fuzz_decode` (996, pub(crate)). Static evaluator (mirrors backend
    lowering EXACTLY): `init_var_values` (1025: initializers may reference
    earlier vars), `EvalEnv` (1050) + `eval_env` (1069: builds a validator Env
    with bindings for width typing), `representative_var_value` (1105),
    `complete_configuration` (1123: unassigned vars at representatives - must
    match eval_env's assumption), `candidate_values` (1145: bool [false,true],
    bit<N> [0,1,2^N-1] deduped, enum all variants), `enumerate_assignments`
    (1180: lexicographic product, last var fastest counter, capped at 4096),
    `expr_refs_var` (1217), `eval_expr` (1256: u128 wrapping arithmetic masked
    to the expression's type width; shifts saturate to 0 at width; bool bitwise
    ops are logical), `width_of` (1381: validator Bit(w) or 8 for literals),
    `eval_bool` (1397), `mask` (1405), `apply_assignments` (1416: SIMULTANEOUS
    - all values from pre-step vars, like the generated temp-next-variable
    lowering), `eval_outputs` (1455).
  - `src/codegen_mc.rs` (863 lines; production 1-599, `mod tests` 601-863 with
    12 tests): legacy fuzz mode - `ViolationSpec` (26: label/prob/percent +
    parse wire format label:prob:percent% with defaults 1.0/100.0 + to_wire),
    `McPhase` (73: ForcedAssumptionViolation -> UnexpectedInjection ->
    AllowedRandomInput, ALL const), `McStatus` (90: Completed/Stuck/
    UnexpectedEvent/AssumptionViolated), `McConfig` (103: bound 5/runs 20/seed
    12345/assumes/violations/unexpected_prob defaults) +
    `parse_assume_selection` (133). Exact mode - `ExactStatus` (151:
    PassComplete/Fail/UnknownStateLimit; wire() pass-complete/fail/unknown-
    state-limit, human() PASS (complete)/FAIL/UNKNOWN (state limit reached),
    exit_code() 0/1/2), `FailReason` (163: PropertyViolation/LivenessViolation/
    Deadlock + wire()), `exact_summary_line` (221: pinned format "model-check
    summary: status=<s> states=N transitions=M max_depth=D" + on fail "
    reason=<r> label=<L> step=S", deadlock uses label=- and the dead config's
    depth as step). Counterexample format - `CounterexampleFormat` (254:
    Text default/Json + wire/parse) + `COUNTEREXAMPLE_FORMAT_ERROR` (282,
    pinned). JSON report model - key constants JSON_KEY_* (323-367; KEY
    ORDERING IS PART OF THE BYTE-PARITY CONTRACT, documented in the module
    comment at 300-319), `JsonIo` (369: variant + fields ordered pairs),
    `JsonStep` (377: input/outputs/from_state/to_state/vars),
    `JsonCounterexample` (387: Trace/Lasso{prefix, cycle, cycle_length}/
    DeadlockConfig{depth, state, vars}; ce_steps() = steps.len() / prefix+cycle
    / depth - the quality metric, ALWAYS LAST key), `McJsonReport` (415:
    status/metrics/[reason,label,step]/statistics/[counterexample]; to_json +
    to_json_string compact). Defaults - `EXACT_DEFAULT_MAX_STATES` (586, =
    1_000_000), `EXACT_DEFAULT_TIMEOUT_SEC` (589, = 0; wall-clock only, NOT
    parity-guaranteed), `EXACT_DEFAULT_THREADS` (593, = 1; N > 1 byte-identical
    via canonical commit, --fast non-parity), `SMT_DEFAULT_BOUND` (597, = 10).
  - **Consumers / pins (verified)**: backends render the plans (rust_test.rs /
    c_test.rs test suites incl. stable test names + random-trace line; rust_mc.rs
    / c_mc.rs embed copies of the plan tables + parsing helpers + minimal JSON
    writer); `src/mc_plan.rs` computes the shared exact-mode plan (D-P4-1) -
    outside this task's scope (task 11.1); tests/codegen_test.rs (registry,
    generation over all valid examples, option sensitivity, CLI parsing),
    tests/backend_parity_test.rs (cross-backend differential parity),
    tests/liveness_mc_test.rs, tests/benchmark_mc_test.rs, CODEGEN.md sections
    5.2/5.3 document the rendering side.
- **Entry format / gate rules / bookkeeping**: same as subtask 8.1 in
  PHASE_8_TASK_8.md (house style with per-section Code: lines; doc-to-code
  reference rules - verify every reference against a declaration line; run
  dev_docs_test after each action, docs_md_test at task end; plan row -> DONE,
  boxes checked, one TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-30): all items listed above plus `src/codegen.rs::finish`, file-
  level `tests/codegen_test.rs`, `tests/backend_parity_test.rs`,
  `src/mc_plan.rs`. Non-reference backticks (safe): CODEGEN.md, DESIGN.md,
  shell commands, bare identifiers, JSON key names.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.5 | codegen-core skeletons (`docs/dev/codegen_core.md`, `docs/llm/codegen_core.md`) with the fragment anchors + pointers in the four core files |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 9.2 | backend_rust deep dive cross-links into these fragments (plugin API, planning data it renders) |
| 2. | 10.1/10.2 | backend_c deep dives cross-link the same way (parity notes) |
| 3. | 13.3 | final Class B verification/budget pass over all 11 cards |

### Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/codegen_core.md` | change | replace the TBD with the full deep dive: Overview + 4 new anchored sub-fragments + unanchored "Changing the codegen core" checklist |
| `docs/llm/codegen_core.md` | change | replace the TBD with the compact card within the 150-line budget |
| `src/codegen.rs`, `src/codegen_fsm.rs`, `src/codegen_test.rs`, `src/codegen_mc.rs` | change | `//! Docs:` line extended with the new fragment ID (doc comment only, no code) |

## Planned actions

### Action 1: Class A deep dive - `docs/dev/codegen_core.md`

Replace the TBD under `dev:codegen-core-overview` and add the four new sections
(anchor line immediately above each heading):

1. **Overview** (`dev:codegen-core-overview`, existing anchor): what the core
   is (language-agnostic plugin layer + shared planning/concept modules; the
   parity principle - one plan, rendered identically in Rust and C); file map
   with line/test counts; position in the pipeline (validated Ir ->
   CodegenCommandKind::run -> backend fn pointer -> artifact with spec-hash
   header); cross-links to CODEGEN.md (extension guide),
   `dev:algorithms-ir-codegen` (L1/L2/L3), `dev:deps-fn-codegen-rust` (call
   graph), `dev:code-map-codegen` (file entries), DESIGN.md D-P4-7/D-P6-3.
2. **Backend plugin API + CLI** (`dev:codegen-plugin-api`, in
   `src/codegen.rs`): the Backend struct as static data (three fn pointers,
   command descriptors); the registry (C + Rust; adding a backend = append one
   entry - CODEGEN.md section 6 checklist); CodegenCommandKind + the standard
   dispatch run (whole-file spec hash, first module) vs run_module (library,
   caller owns the hash); BackendOptions field-by-field applicability table
   (which command uses which field; defaults); the two pinned rejection
   constants; PropertyMonitorMode and its backend mapping; CodegenOutput +
   ExtraFile (why fuzz_target.rs/.c is a separate file - libFuzzer dependency,
   CLI refuses stdout mode with extras); parse_codegen_args all-errors
   collection + command-scoped rejections; help rendered from registry data.
3. **Production FSM shared helpers** (`dev:codegen-fsm-helpers`, in
   `src/codegen_fsm.rs`): VariantInfo/FsmVariants; the build_variants naming
   scheme with worked example (single-variant bare name, group_variant,
   numeric suffixes); representative_values + RepValue incl. the unrepresentable
   cases; contains_liveness/safety_formula_inner as legacy Phase 1 helpers and
   their relationship to codegen_test::has_liveness_op and monitor.rs.
4. **Unit-test planning** (`dev:codegen-test-plan`, in `src/codegen_test.rs`):
   the parity principle (data, never code); TestPlan field tour; the exact
   expect_progress computation (and why the old heuristic was wrong - spec 47);
   has_definite_unexpected + is_deadlocked_initial; per-transition planning in
   detail (concrete input construction, sibling analysis, firing order =
   priority desc then source order mirroring fsm_step, shadowing, the capped
   static search with candidate values, taken/not-taken conditions, not-taken
   observability, complete_configuration); invariant selection; monitor-
   violation planning (the two cases + the C5-1 sibling skip + simultaneous
   assignments); the LCG random trace (constants, seed, precomputation); fuzz
   decode strategies; the static evaluator's width semantics (u128 wrapping +
   mask, shift saturation, bool bitwise = logical) and why it must mirror the
   backend lowering exactly.
5. **Model-checker concepts** (`dev:codegen-mc-concepts`, in
   `src/codegen_mc.rs`): legacy fuzz-mode types (McConfig defaults,
   ViolationSpec wire format, McPhase ordered strategy, McStatus); exact mode
   (ExactStatus taxonomy with wire/human/exit_code, FailReason, the pinned
   summary line); codegen-time defaults (max states, timeout wall-clock-only,
   threads canonical commit, SMT bound); CounterexampleFormat + pinned error;
   the JSON report model with the key-ordering byte-parity contract (ce_steps
   last, peak_rss_kb informational/normalized); why backends embed their own
   copies (generated binaries must not depend on this crate) and where exact
   mode actually runs (mc_plan.rs - task 11.1; SMT path - task 11.2).
6. **Changing the codegen core** (unanchored closing section): developer
   checklist - placement rule (CODEGEN.md section 2); parity principle (new
   features land in every backend in the same change + cross-backend
   differential tests); pinned formats that break on drift (summary line, JSON
   key ordering, LCG constants/seed, test-name scheme, wire formats); planner
   semantics must mirror the generated fsm_step (firing order!); verify with
   tests/codegen_test.rs + backend_parity_test + liveness/benchmark suites;
   docs sync (CODEGEN.md, KB.md; training snippets if user-visible).

Run `cargo test --test dev_docs_test` after writing; extend the four
`//! Docs:` lines in the same action.

### Action 2: Class B card - `docs/llm/codegen_core.md`

Replace the TBD (keep budget header, title, intro, anchor byte-stable):
responsibility; key-items table (Backend / BACKENDS + get_backend /
CodegenCommandKind::run + run_module / BackendOptions / CodegenOutput +
ExtraFile / parse_codegen_args / build_variants / representative_values /
build_test_plan / expect_progress_from_initial / has_definite_unexpected /
plan_transitions firing order / Lcg + RANDOM_TRACE_SEED / ExactStatus +
exact_summary_line / McJsonReport key ordering / EXACT_DEFAULT_*); invariants
that must not break (parity principle, data-never-code planner, firing order
mirrors fsm_step, pinned formats, embedded-copy sync, spec hash stamping);
change-impact pointers (placement rule, parity tests; full table in
llm:change-impact at task 13.2); deep-dive links to the five Class A fragments
+ CODEGEN.md. Total <= 150 lines. Run the gate.

### Action 3: verification + bookkeeping

Run the full gate: `cargo test --test dev_docs_test` (9/9),
`cargo test --test docs_md_test` (5/5), then full verification: `cargo build &&
cargo test && cargo clippy --all-targets && cargo fmt --check` (+ `--workspace`).
Source edits are doc-comment lines only, so all cargo counts must be UNCHANGED
from the task-8.3 tree (535/535 root, 559/559 workspace). Mark plan row 9.1
DONE in `PHASE_8_PLAN.md`; check off this file's exit-criteria boxes; append ONE
`TASKS.md` entry; commit per action (Action 1: deep dive + owner lines;
Action 2+3: card + bookkeeping).

## Task deliverables

- `docs/dev/codegen_core.md` filled: Overview + plugin API/CLI + FSM helpers +
  test planning + MC concepts + unanchored change checklist; four new anchored
  fragments, all owned.
- `docs/llm/codegen_core.md` card filled within its 150-line budget.
- `//! Docs:` lines of the four core files extended (doc comments only).
- Plan row 9.1 DONE; TASKS.md entry.

## Task exit criteria

- [x] `docs/dev/codegen_core.md`: all five anchored sections + the unanchored
      "Changing the codegen core" checklist filled, no TBD remains; every
      technical statement carries a resolving doc-to-code reference (gate
      check 1); the existing `dev:codegen-core-overview` anchor + heading
      unchanged; the four new fragments anchored AND owned in the same action.
- [x] `docs/llm/codegen_core.md`: card filled, no TBD, total line count <= 150,
      links to Class A without restating it.
- [x] Source edits limited to the four `//! Docs:` lines (no code changes);
      all cargo counts unchanged from the task-8.3 tree.
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 9.1 marked DONE; this file's boxes checked; TASKS.md entry
      appended.

## Non-goals

- No codegen behavior changes of any kind; no new dependencies.
- No restatement of CODEGEN.md, ALGORITHMS L1/L2/L3, or DEPENDENCIES graph
  content (link only); no edits to other docs (backend_rust deep dive = task
  9.2; START.md/KB.md/BUILD.md pointer sync = task 14).
- No mc_plan.rs / smt_* deep dives (task 11.1/11.2); no backend internals
  (tasks 9.2/10.x) - the core doc references them, never describes them.
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-10-30) for plan group 9 | User model: one task file per plan group ("execute 9.1" = subtask of task file 9). Written for subtask 9.1 only; subtask 9.2 (backend_rust deep dive) will be appended at execution, following the PHASE_8_TASK_8.md pattern (8.2/8.3 appended after 8.1 completed). |
