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

## Subtask 9.2 - Rust backend deep dive (added at execution, see Task corrections)

### Goal

Fill in `docs/dev/backend_rust.md` (Class A deep dive): full content on the
Rust target-language backend - plugin registration + shared Rust-specific
helpers (`src/backends/rust.rs`), production FSM source generation
(`src/backends/rust_fsm.rs`), unit-test / fuzz-skeleton / aggregate-crate
generation (`src/backends/rust_test.rs`), and model-checker executable
generation (`src/backends/rust_mc.rs`) - the parity principle: one plan,
rendered identically in Rust and C. Also fill in the matching Class B card
`docs/llm/backend_rust.md` (within its 150-line budget) - per the confirmed
scope decision (plan Corrections entry 3).

### Context

- **Skeleton state (from task 1.6)**: `docs/dev/backend_rust.md` exists with
  one anchored fragment `dev:backend-rust-overview` (owner: ALL FIVE backend
  files - each file's `//! Docs:` line already names it) followed by a TBD
  line; `docs/llm/backend_rust.md` exists with budget header
  `<!-- llm-budget: 150 -->`, fragment `llm:backend-rust` (same five owners),
  and a TBD line. Existing anchor + heading stay byte-stable.
- **Fragment structure**: keep `dev:backend-rust-overview` for Overview;
  add four new sub-fragments, one per source file (`src/backends/mod.rs` is
  module declarations only - covered in the Overview):
  - `dev:backend-rust-plugin` - Plugin entry + shared Rust helpers (`src/backends/rust.rs`)
  - `dev:backend-rust-fsm` - Production FSM generation (`src/backends/rust_fsm.rs`)
  - `dev:backend-rust-test` - Unit-test, fuzz and aggregate-crate generation (`src/backends/rust_test.rs`)
  - `dev:backend-rust-mc` - Model-checker executable generation (`src/backends/rust_mc.rs`)
  ORPHAN RULE: the new IDs go on the owner file's `//! Docs:` line in the same
  action. Exact target lines:
  - `src/backends/rust.rs` line 12 becomes:
    `//! Docs: dev:backend-rust-overview, dev:backend-rust-plugin, llm:backend-rust`
  - `src/backends/rust_fsm.rs` line 9 becomes:
    `//! Docs: dev:backend-rust-overview, dev:backend-rust-fsm, llm:backend-rust`
  - `src/backends/rust_test.rs` line 37 becomes:
    `//! Docs: dev:backend-rust-overview, dev:backend-rust-test, llm:backend-rust`
  - `src/backends/rust_mc.rs` line 18 becomes:
    `//! Docs: dev:backend-rust-overview, dev:backend-rust-mc, llm:backend-rust`
  - `src/backends/mod.rs` line 7 UNCHANGED (owns overview + the card).
- **No duplication with existing fragments**: CODEGEN.md (repo root) is the
  EXTENSION guide - plugin API walkthrough, CLI surface, shared-module
  summaries incl. backend rendering details, adding-a-backend checklist,
  testing conventions, invariants; `src/backends/RUST.md` is the
  architecture/design doc of the generated artifacts (file layout, artifact
  design + anatomy, expression-lowering summary, test classes, MC generation
  shapes, key design decisions, known limitations); DEPENDENCIES.md
  `dev:deps-fn-codegen-rust` has the function-level call graph of core + Rust
  backend; CODE_MAP.md has the file entries; docs/dev/codegen_core.md (task
  9.1) owns the language-agnostic core - plugin API, planning data, MC
  concepts. The deep dive goes DEEPER: per-function detail of the emitters
  (the Gen pipeline order, monitor node rendering with exact field sets and
  step semantics, pattern-scope binding mangling + reserved sets, simultaneous
  assignment lowering, gen_expr width semantics), the exact generated-code
  shapes that are pinned formats (artifact header, test names + random-trace
  line, fuzz decode wire format, config_bytes layout, summary line, JSON key
  order + escape contract, SMT variable naming + constraint/query order,
  witness trace format), invariants (parity, no_std body byte-identity,
  deterministic bytes, embedded-copy sync, dependency-free artifacts), and
  change guidance. Cross-link; never restate.
- **Verified source material (measured 2026-10-31; line numbers for
  navigation only - doc references use item names)**:
  - `src/backends/mod.rs` (16 lines): module declarations c, c_fsm, c_mc,
    c_test, rust, rust_fsm, rust_mc, rust_test; no code.
  - `src/backends/rust.rs` (215 lines; production 1-181, `mod tests`
    183-215 with 3 tests): `RUST_KEYWORDS` (17: the 36 keywords requiring
    r# mangling), `rust_ident` (25), `rust_binding_ident` (41: keyword
    mangling + rename of bindings colliding with identifiers the generated
    body uses unqualified to `<name>__rfsb`, further suffixed until unique;
    Phase 6 task 6 finding - a field named `out` would shadow mc_step_all's
    effect vector; C has the same bug class via its effect counter `n`; the
    caller passes its reserved set: {input, handler} in the Fsm impl,
    {out, state, vars} in mc_step_all), `snake` (60), `bit_rust_type` (84:
    0..=8 u8 / 9..=16 u16 / 17..=32 u32 / 33..=64 u64 / else u128),
    `rust_type` (95), `mask_expr` (105: u128::MAX for w >= 128 else
    ((1u128 << w) - 1u128)), `representative_input_expr` (115: from
    codegen_fsm::representative_values; EnumEmpty/EnumUnknown to
    Name::__Empty__/::__Unknown__), `PRODUCTION_INFO` (148) /
    `TEST_INFO` (157: --out REQUIRED, fuzz skeleton next to it, stdout
    refused) / `MC_INFO` (165: codegen-time + runtime flags incl. --fast
    non-parity), `RUST_BACKEND` (173). Tests: identifier_mangling (187),
    type_mapping (195), backend_registration_shape (209).
  - `src/backends/rust_fsm.rs` (1896 lines, no unit tests):
    `generate_production` (26: --with-property-monitors to Gated/Off),
    `generate_core` (43: re-validate via validator::validate_module, ALL
    errors returned; shared by all three commands), `Gen` (62: incl.
    uses_vec_macro - whether any transition or on_error emits an output,
    gating the no_std vec! macro import), `Gen::new` (82: build_variants +
    compile_monitors, then fixed emission order prelude -> enums -> Vars ->
    Input/Output -> renderers -> trace/handlers -> monitors -> Fsm impl ->
    [ProtocolFsm] -> [tests]), `emit_protocol_fsm_impl` (133: opt-in
    ProtocolFsm forwarding to the inherent methods explicitly so it cannot
    recurse; the one documented opt-in linking an artifact to rfs-adapters),
    `has_prop`/`prop_cfg` (156/162: Gated to #[cfg(debug_assertions)]),
    `emit_prelude` (176: line 1 = v1.0 unified artifact header D-P6-3,
    identical text in both backends; #![allow(...)] list; no_std header
    variant - body BYTE-IDENTICAL to std), `emit_enums` (210), `emit_vars`
    (221), `emit_input_output` (238: flattened Input/Output enums from
    FsmVariants unique_names), `emit_renderers` (274: ref_mc-identical value
    forms; rfs_render_state fixed name / rfs_render_e<i> / rfs_render_input /
    rfs_render_output / rfs_render_vars; declaration order = deterministic
    bytes), `emit_trace_and_handlers` (357: TraceStep/Trace/Sample - sample 0
    = initial config, no input/outputs; AssumptionPolicy Continue/Stop/Panic;
    StepError incl. cfg-gated PropertyViolation{label, step}),
    `compile_monitors` (400: monitor::compile_formula per property / selected
    assumption; out-of-subset skip warning identical in every backend,
    D-P3-5), `emit_monitors` (432: Mon_* structs assumptions-first then
    properties, declaration order; PropertyViolation record struct;
    PROPERTY_LABELS/ASSUME_LABELS/SPEC_VERSION/ALL_INPUTS constants),
    `render_monitor` (497: nodes numbered DFS pre-order, step body evaluates
    post-order; step returns self.n0_failed && !before = newly confirmed at
    this sample; trigger_step from n0_trigger_step), `render_node` (533:
    RangeCheck now-recomputed/failed-sticky; Obligation bound 0 = be[0] body
    at trigger sample; bound >= 1 = D-P3-3 pending model, failure terminal;
    DelayedCheck prev_antecedent register, terminal, no trigger sample;
    And/Or per-sample over children's _now - conjunction fails while ANY
    child fails, disjunction only while ALL fail, trigger_step = min),
    `range_cond` (664), `gen_signal` (676) -> `render_signal_expr` (1323,
    pub(crate), shared with rust_test invariant re-checks; state = s.to_state,
    vars from sample), `input_is_sample_expr` (1359) /
    `output_has_sample_expr` (1441: struct patterns bind by real name +
    guard - matches! external guards would not compile), `emit_assume_monitors`
    (681: first newly failed ACTIVE assumption, declaration order),
    `emit_prop_monitors` (737: first new PropertyViolation{label, step: idx,
    trigger_step}), `initial_vars_expr` (802: initializers may reference
    earlier vars), `emit_fsm_impl` (819: Fsm struct; new() steps sample 0 -
    assumption violated at sample 0 panics, property violation recorded;
    step -> step_with_handler(PanicAssumptionHandler); transitions grouped by
    state first-appearance order, within a state sorted by DESCENDING
    effective priority (stable sort_by_key Reverse(priority.unwrap_or(0))) -
    mirrors the planner's firing order; commit_step: TraceStep push, Sample,
    idx = trace.steps.len(), assume monitors -> handler policy, prop check
    first-violation-wins, state/vars update; group-match helpers
    input_matches_group_<name> for multi-variant groups), `transition_block`
    (967: SIMULTANEOUS assignment lowering - all tmp_<var> from pre-step
    values then applied to a clone of self.vars), `on_error_block` (1055: no
    state check, no bindings, vars carried over), `pattern_block_open` (1099)
    / `pattern_block_close` (1245: bare group single-variant direct if let /
    multi-variant helper; unit variant - error "unit pattern for struct";
    struct variant - field count check, named bindings via
    rust_binding_ident recorded in env.binding_idents, unnamed _f<i>, literal
    fields to additional `if b == value` guards), `output_expr` (1262: bare
    output -> group's single variant; per-arg expected type for width
    inference), `ExprCtx` (1482), `gen_expr` (1512: types every subexpression
    via validator::type_expr; BitNot/Neg + Plus/Minus/Star = u128 wrapping +
    mask to type width; comparisons cast both sides to u128; shifts saturate
    to 0 at >= width; bool bitwise ops are LOGICAL), `width_from_type` (1632:
    expected Bit(w) wins, IntLit -> 8), `render_mc_step_all` (1649,
    pub(crate): every enabled transition in firing order - deterministic uses
    first effect only, nondet explores all; on_error fallback appended with
    transition_id usize::MAX; same helpers as Fsm::step so both paths agree
    by construction), `render_sample_formula_evaluator` (1835: via
    monitor::formula_to_signal + gen_signal; error on temporal operators),
    `enum_renderer_names` (1868: State -> rfs_render_state, others
    rfs_render_e<i>; shared with the MC JSON renderer), `renderer_arg_rust`
    (1886).
  - `src/backends/rust_test.rs` (977 lines, no unit tests - the `mod tests`
    at line 104 is inside a generated template string): `generate_unit_tests`
    (52: rejects no_std with NO_STD_PRODUCTION_ONLY_ERROR; generate_core
    with_tests=true + Always; appends ExtraFile fuzz_target.rs),
    `emit_test_module` (79: build_test_plan(variants, module, steps);
    candidate_inputs() = representative per variant; initial_state_is_correct;
    positive_sequence_runs - greedy trial-clone drive up to K steps with
    StopAssumptionHandler, EXPECT_PROGRESS from plan.expect_progress;
    negative_unexpected_input_search - ContinueAssumptionHandler, asserts
    found when definite_unexpected && !has_on_error, or on_error landing when
    has_on_error && has_definite_unexpected), `var_value_literal` (228) /
    `vars_initializer` (242: panics "planner must cover variable" - contract
    violation) / `input_case_expr` (259) / `resolve_output_variant` (285) /
    `output_case_full_literal` (307: None when an arg is not statically
    evaluable) / `render_outputs_assert` (337: full assert_eq! or per-output
    partial match - variant + known fields only, the documented planner
    fallback), `render_transition_test` (411: name t<id>_<state>_on_<event>
    (+ _not_taken); taken asserts on the RECORDED trace step - robust against
    property-violation halts; not-taken STRICT - refuses to emit a toothless
    test when the signature's outputs are not fully evaluable),
    `render_invariant_test` (522: per-sample atom via render_signal_expr at
    every sample of the positive drive trace; bounded checks samples < N
    only), `render_monitor_test` (573: case 1 initial config already violates
    - Fsm::new() records at sample 0; case 2 one planned drive step; asserts
    the sticky property_violation label recorded first),
    `render_random_trace_test` (618: precomputed LCG index table over
    ALL_INPUTS, StopAssumptionHandler; prints ONE deterministic coverage line
    random_trace seed={} steps={} ok={} unexpected={} assumption={} property={}
    states_visited={}), `payload_byte_count` (666: ceil(W/8)),
    `emit_fuzz_target` (674: same v1.0 header; #![no_main] + libfuzzer_sys;
    decode - first byte variant selector (byte % n_variants), bool = one
    non-zero byte, bit<W> = ceil(W/8) LE bytes masked to W bits, enum =
    constant representative (first declared variant else __Unknown__); budget
    check breaks on short data; loops over Fsm::step until exhausted),
    `GeneratedCrate` (904), `generate_aggregate_test_crate` (915: one pub mod
    per module, name m{:03}_<snake(file_stem)>_<snake(module.name)>; whole-
    file spec hash on a per-file options clone; fuzz skeleton NOT included;
    manifest carries an empty [workspace] table = own workspace root,
    Phase 5 task 1), `write_generated_crate` (968: <root>/target/
    rfs_codegen_tests).
  - `src/backends/rust_mc.rs` (2034 lines, no unit tests):
    `generate_model_checker` (25: rejects no_std + with_protocol_fsm with the
    pinned constants; SmtBounded -> standalone z3 binary = v1.0 header +
    smt_emit::emit_rust_spec(build_smt_spec(module, McPlan::compute(...))) +
    SMT_ENGINE_RUST with {BOUND} <- mc_smt_bound; exact/fuzz -> FSM core via
    generate_core (steps 0, monitors Always, no tests) + exact_codegen +
    FUZZ_RUNTIME + MAIN_DISPATCH), `exact_codegen` (91: codegen-time defaults
    embedded as runtime-overridable constants D-P4-7 - DEFAULT_MODE/MAX_STATES/
    NONDET/TIMEOUT_SEC/THREADS/COUNTEREXAMPLE_FORMAT + CONFIG_TOTAL_BYTES /
    MONITORS_BITS from plan.encoding; all_inputs() canonical enumeration
    D-P4-2 - variants declaration order, concrete fields over their domain
    nested in declaration order, abstracted at representatives; put_bits +
    config_bytes layout - state index bits, var slots (bool 1/0, bit as u64,
    enum variant index), monitor node bits with remaining packed ONLY while
    pending = ref_mc CfgKey byte-for-byte; render_mc_step_all; EXACT_ENGINE;
    JSON_WRITER_RUST + json_renderers_rust; liveness -
    LivenessDecomposition::compute diagnostics to codegen warnings D-P4-9,
    LIV_ENGINE, leaf evaluators liv_p<i>_l<j>[b] (Safety 0 / Eventually 1 /
    Until 2 with eval_b), flatten_skeleton first-appearance re-indexing to
    liv_p<i>_skel, LIV_PROPS; EXACT_RUN), `json_renderers_rust` (351: same
    name source as the text renderers - JSON strings carry ref_mc's prefixed
    value forms identical to text), `flatten_skeleton` (444), `domain_iter`
    (474), `rep_expr` (491), `var_value_expr` (509); EXACT_ENGINE (548: Node
    {state, vars, prop, assume}; StepOutcome Success/Pruned/Violation(label,
    step, trigger); step_successor at depth+1; path_to parent pointers;
    replay rebuilds the Phase 3 trace format - nondet picks the effect with
    the recorded transition_id else first; exact_summary_line pinned D-P4-4
    "model-check summary: status={} states={} transitions={} max_depth={}" +
    on fail " reason={} label={} step={}", deadlock label=- depth as step),
    LIV_ENGINE (621: per-leaf progress automata - safety 0=ok/1=bad,
    eventually 0=wait/1=done, until 0=ok/1=sat/2=bad, all one-way so every
    state sequence is eventually constant = limit vector; EDGE-based labels -
    position 0 initial config sample, position k sample of step k;
    liv_bits_at cached per edge position; liv_delta mixed-radix; liv_good;
    liv_analyze product BFS from (start_q, config 0), per-q stay-edge
    subgraph, nondet STRONGLY FAIR filter D-P4-11 - each cycle state exactly
    one explored successor and fully expanded when truncated, iterative
    Kosaraju SCCs, cyclic = comp > 1 or self-loop, limit vector falsifies the
    skeleton; liv_witness lasso = product path to (q, cstar) + stay-edge
    cycle through cstar = smallest cyclic config; liv_note human explanation),
    JSON_WRITER_RUST (983: embedded minimal copy of the toolchain compact
    writer with u64 integers - all values non-negative, bit<64> reaches
    2^64-1; escape contract " \ b t n f r + other control chars as \u00XX
    lowercase hex else raw UTF-8; peak_rss_kb from /proc/self/status VmHWM,
    0 on failure - INFORMATIONAL, normalized to 0 in cross-backend byte
    comparisons; build_mc_json_doc key order = byte-parity contract: status,
    metrics{states, transitions, max_depth}, [reason, label, step],
    statistics{unexpected_terminations, monitor_size_bits,
    memory_estimate_bytes, peak_rss_kb}, [counterexample]; ce kinds trace /
    lasso{prefix, cycle, cycle_length} / deadlock-config{depth, state, vars};
    ce_steps ALWAYS the last key), EXACT_RUN (1170: run_exact() -> !; runtime
    flags over codegen-time defaults D-P4-7; Fsm::new sample 0 - assumption
    violated panics, property violation reported immediately; level-
    synchronous BFS D-P4-2 - visited at ENQUEUE time, parent pointers, depth,
    adj edges for every Success incl. to already-visited configs, complete
    flag; status 0 running / 1 pass-complete / 2 state limit / 3 timeout /
    4 fail, exit codes 0/1/2; parallel level expansion task 10B - phase A
    std::thread::scope workers over disjoint frontier chunks calling pure
    expand_config, phase B SEQUENTIAL canonical commit = byte-identical to
    --threads 1 for any N; --fast non-parity early abort, never a false pass;
    Unexpected = path terminates D-P4-5 counted in statistics, transitions
    untouched; Pruned counts as a transition; deadlock = no successor at all
    -> FAIL reason=deadlock label=- step=depth; SEMANTICS 9.5 stop at first
    violation/state limit - check AFTER the per-level loop, matches the C
    engine's status == 0 guard, Phase 6 task 4 finding correction 4b;
    liveness only when BFS ended without safety failure/deadlock; truncated
    analysis sound - witness inside explored part or UNKNOWN stands, never a
    false pass-complete; JSON one compact document or text PASS (complete) /
    FAIL / UNKNOWN (state limit reached[, timeout after Ns]) + pinned summary
    line + statistics block), MAIN_DISPATCH (1553: --mode fuzz -> run_fuzz
    else run_exact), FUZZ_RUNTIME (1570: Rng LCG state = state *
    6364136223846793005 + 1442695040888963407 wrapping, top-53-bit double
    mapping - SAME constants as codegen_test::Lcg; ViolationSpec wire format
    label:prob:percent% defaults 1.0/100.0; parse_assume_selection all/none/
    labels filtered against ASSUME_LABELS; print_trace + vars_at via the
    renderers; run_fuzz flags --bound 5 / --runs 20 / --seed 12345 /
    --assumes / --violate / --unexpected-prob = McConfig defaults; per step:
    forced violation injection first (trial-step ALL_INPUTS for the target
    label), then optional unexpected injection, else allowed representative
    inputs - property-violating inputs allowed on purpose; property
    violations terminate immediately exit 1 so property_violations counter is
    always 0 (field kept for stable format); summary line "model-check
    summary: runs={} bound={} completed={} unexpected={}
    assumption_stops={} property_violations={}"), SMT_ENGINE_RUST (1756:
    DEFAULT_BOUND <- {BOUND}; parity-critical - step variables z3 constants
    with FIXED names in fixed order s_i/v_i_j/ev_i/fld_i_g, constraints
    asserted in fixed order, queries in fixed order safety-then-liveness
    declaration order; b_and/b_or two-arg wrappers over z3 0.21 variadic
    associated functions - identical to C; unrolling: enabled_t = state match
    and class match and guard (firing order), fire_t = enabled and no earlier
    enabled (first match, prefix-OR chain), alive_i = alive_{i-1} and
    any_fire, next state/vars via ITE chains; safety query viol_p = exists
    i<k: alive_i and not phi_p(i) -> FAIL + pinned summary line exit 1;
    liveness query never_p and cycle (exists a<b identical (state, vars)) ->
    FAIL reason=liveness-violation + cycle length exit 1; Unknown -> UNKNOWN
    (solver inconclusive within bound {k}) + status=unknown exit 2; all Unsat
    -> PASS WITHIN BOUND {k} (incomplete - no completeness claim) +
    status=pass-within-bound exit 0; smt_print_trace pinned witness format).
  - **Consumers / pins (verified)**: tests/codegen_test.rs (51 tests:
    registry shape, generation over all valid examples, flag toggles,
    priority order in generated step, on_error fallback, SPEC_VERSION
    constant, no_std header pin + body-parity compile sweep over all 50 valid
    examples x both monitor modes, protocol-fsm emission pin + default
    unchanged, test-class emission + pinned name convention, fuzz skeleton as
    separate file + CLI writes next to --out, aggregate crate stays
    dependency-free); tests/backend_parity_test.rs (11 tests: cross-backend
    differential parity - exact runs, RE-pinned values, generated test
    executables, liveness parity incl. truncated-mode soundness, JSON report
    parity incl. five status/reason cases + golden document pin, forced-
    violation parity); tests/liveness_mc_test.rs (9 tests),
    tests/benchmark_mc_test.rs (5 tests); src/backends/RUST.md (artifact
    design + key decisions + known limitations).
- **Entry format / gate rules / bookkeeping**: same as subtask 8.1 in
  PHASE_8_TASK_8.md (house style with per-section Code: lines; doc-to-code
  reference rules - verify every reference against a declaration line; run
  dev_docs_test after each action, docs_md_test at task end; plan row ->
  DONE, boxes checked, one TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-31): all items listed above plus `src/codegen.rs::NO_STD_
  PRODUCTION_ONLY_ERROR`, `src/codegen.rs::PROTOCOL_FSM_MC_ERROR`,
  `src/mc_plan.rs::McPlan`, `src/mc_plan.rs::ValueDomain`,
  `src/mc_plan.rs::LivenessDecomposition`, `src/smt_plan.rs::build_smt_spec`,
  `src/smt_emit.rs::emit_rust_spec`, `src/monitor.rs::compile_formula`,
  `src/monitor.rs::formula_to_signal`, `src/spec_hash.rs::spec_hash16`,
  `src/codegen_fsm.rs::build_variants`,
  `src/codegen_fsm.rs::representative_values`,
  `src/codegen_test.rs::build_test_plan`,
  `src/codegen_test.rs::has_definite_unexpected`,
  `src/codegen_test.rs::Lcg`, `src/codegen_test.rs::RANDOM_TRACE_SEED`,
  `src/codegen_test.rs::FuzzDecodePlan`, `src/codegen_mc.rs::McConfig`,
  `src/validator.rs::validate_module`, `src/validator.rs::type_expr`, file-
  level `tests/codegen_test.rs`, `tests/backend_parity_test.rs`,
  `tests/liveness_mc_test.rs`, `tests/benchmark_mc_test.rs`. Non-reference
  backticks (safe): CODEGEN.md, RUST.md, DESIGN.md, shell commands, bare
  identifiers, JSON key names, generated-code item names without a file-path
  prefix.

### Planned actions

1. **Class A deep dive - `docs/dev/backend_rust.md`**: replace the TBD and
   add the four new sections (anchor line immediately above each heading):
   - **Overview** (`dev:backend-rust-overview`, existing anchor): the parity
     principle; file map with line/test counts; position in the pipeline via
     RUST_BACKEND's fn pointers into generate_core; cross-links to RUST.md,
     CODEGEN.md, `dev:deps-fn-codegen-rust`, `dev:code-map-codegen`,
     DECISIONS D-numbers.
   - **Plugin entry + shared Rust helpers** (`dev:backend-rust-plugin`):
     registration + descriptor text; the three mangling helpers incl.
     rust_binding_ident's reserved sets; snake/bit_rust_type/rust_type/
     mask_expr; representative_input_expr.
   - **Production FSM generation** (`dev:backend-rust-fsm`): entry points +
     Gen pipeline order; artifact header + no_std byte-identical body pin;
     flat Input/Output enums; ref_mc-identical renderers; trace/assumption
     runtime; monitor rendering (DFS pre-order / post-order, per-node-kind
     field sets and step semantics); Fsm impl (sample-0 stepping, priority-
     desc stable firing order, commit_step policy chain); transition blocks
     with simultaneous assignment lowering; pattern scopes with binding
     mangling + literal guards; gen_expr width semantics in lockstep with the
     planner's static evaluator; render_mc_step_all / render_sample_formula_
     evaluator shared renderers.
   - **Unit-test, fuzz and aggregate-crate generation** (`dev:backend-rust-
     test`): the seven stable test classes incl. the STRICT not-taken refusal;
     literal rendering + partial-assert fallback; the separate fuzz_target.rs
     decode wire format; aggregate crate with the empty [workspace] table.
   - **Model-checker executable generation** (`dev:backend-rust-mc`): three
     entry shapes incl. the standalone SMT binary; exact_codegen plan tables
     (canonical all_inputs, config_bytes layout matching ref_mc's CfgKey);
     EXACT_ENGINE/EXACT_RUN (enqueue-time visited, two-phase parallel
     expansion byte-identical to --threads 1, SEMANTICS 9.5 stop rule,
     truncated liveness soundness); LIV_ENGINE (limit vectors, strong
     fairness, lasso witness); JSON_WRITER_RUST (key-order byte-parity
     contract, escape contract, peak_rss_kb normalization); FUZZ_RUNTIME (LCG
     constants, phase-ordered input selection); SMT_ENGINE_RUST (fixed names /
     constraint order / query order, pinned summary lines); embedded-copy
     rule.
   - **Changing the Rust backend** (unanchored closing section): developer
     checklist - parity first + differential suites; pinned formats list;
     firing order mirrors fsm_step AND the planner; gen_expr width semantics
     lockstep; dependency-free deterministic artifacts; no_std body pin;
     verify commands + docs sync.
   Run `cargo test --test dev_docs_test` after writing; extend the four
   `//! Docs:` lines in the same action (mod.rs unchanged).
2. **Class B card - `docs/llm/backend_rust.md`**: replace the TBD (keep
   budget header, title, intro, anchor byte-stable): responsibility; key-
   items table (RUST_BACKEND / rust_ident + rust_binding_ident /
   bit_rust_type + mask_expr / generate_core / render_monitor + render_node /
   gen_expr / render_mc_step_all / emit_test_module seven classes /
   emit_fuzz_target decode wire format / generate_aggregate_test_crate /
   generate_model_checker three shapes / exact_codegen plan tables / EXACT_
   ENGINE + EXACT_RUN / LIV_ENGINE / JSON_WRITER_RUST / FUZZ_RUNTIME +
   SMT_ENGINE_RUST); invariants that must not break (parity, firing order,
   width-semantics lockstep, pinned formats, dependency-free artifacts, no_std
   body pin, embedded-copy sync); change-impact pointers (placement rule;
   verify with codegen_test + backend_parity_test + liveness_mc_test +
   benchmark_mc_test; full table in llm:change-impact at task 13.2); deep-
   dive links to the five Class A fragments + RUST.md + CODEGEN.md +
   codegen_core. Total <= 150 lines. Run the gate.
3. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5,
   full verification (build/test/clippy/fmt + workspace; counts unchanged:
   535/535 root, 559/559 workspace); plan row 9.2 -> DONE; check off this
   subtask's boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 9.2)

- [x] `docs/dev/backend_rust.md`: all five anchored sections + the unanchored
      "Changing the Rust backend" checklist filled, no TBD remains; every
      technical statement carries a resolving doc-to-code reference (gate
      check 1); the existing `dev:backend-rust-overview` anchor + heading
      unchanged; the four new fragments anchored AND owned in the same action.
- [x] `docs/llm/backend_rust.md`: card filled, no TBD, total line count <=
      150 (56), links to Class A without restating it.
- [x] Source edits limited to the four `//! Docs:` lines (no code changes);
      all cargo counts unchanged from the task-9.1 tree (535/535 root,
      559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 9.2 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 9.2)

- No backend behavior changes of any kind; no new dependencies.
- No restatement of CODEGEN.md, src/backends/RUST.md, DEPENDENCIES graph,
  or codegen_core content (link only); no edits to other docs (backend_c deep
  dives = tasks 10.1/10.2; START.md/KB.md/BUILD.md pointer sync = task 14).
- No mc_plan.rs / smt_* deep dives (task 11.1/11.2); no C backend internals
  (tasks 10.x) - the doc references them, never describes them.
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-10-30) for plan group 9 | User model: one task file per plan group ("execute 9.1" = subtask of task file 9). Written for subtask 9.1 only; subtask 9.2 (backend_rust deep dive) will be appended at execution, following the PHASE_8_TASK_8.md pattern (8.2/8.3 appended after 8.1 completed). |
| Subtask 9.2 appended at execution (2026-10-31) | Per the correction above: subtask sections are written when executed, following the PHASE_8_TASK_8.md pattern (8.2/8.3). Written with verified source material measured 2026-10-31 (item maps of all five backend files: mod.rs 16 lines no code; rust.rs 215/3 tests; rust_fsm.rs 1896 no unit tests; rust_test.rs 977 no unit tests - its `mod tests` at line 104 is inside a generated template string; rust_mc.rs 2034 no unit tests), fragment plan with exact owner-line targets (mod.rs unchanged), no-duplication rule vs CODEGEN.md + src/backends/RUST.md + DEPENDENCIES + codegen_core, consumers/pins inventory. |
| TASKS.md link fix during verification (2026-10-31) | The 9.1 completion entry (commit f2838b7) contained an unbackticked bracket-paren formula pattern in quotes, which the mdr-stack quality check parses as a relative markdown link - docs_md_test failed on the pristine tree BEFORE any subtask 9.2 change (the pattern was added after that session's last green gate run). Reworded the 9.1 verification line in TASKS.md to avoid the pattern (same fix class as the one recorded in the 9.1 entry itself); docs_md_test green again (5/5, 185.1s - baseline). |
