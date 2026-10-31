# RFS Rust Backend (Class A)

The Rust target-language backend: plugin registration plus the three
generators that render the shared codegen plans into standalone Rust source.

<a id="dev:backend-rust-overview"></a>
## Overview

The Rust backend is one of the two target-language backends in
`src/backends/` (the other is C, deep dive task 10.x). It renders the
language-agnostic codegen plans into standalone Rust source - one self-contained
`.rs` file per command. The governing constraint is PARITY: for the same spec
and options, the Rust and C artifacts must agree in every pinned format and in
observable behavior; cross-backend differential tests enforce this
(tests/backend_parity_test.rs).

| File | Lines | Role |
|---|---|---|
| `src/backends/mod.rs` | 16 | module declarations only (c*, rust*); no code |
| `src/backends/rust.rs` | 215 (3 unit tests) | plugin entry `RUST_BACKEND` + shared Rust-specific helpers (identifier mangling, type mapping, representative-input rendering) |
| `src/backends/rust_fsm.rs` | 1896 (no unit tests) | production FSM source generation; the core generator reused by test and mc; shared expression/signal renderers |
| `src/backends/rust_test.rs` | 977 (no unit tests) | unit-test module rendering, the separate fuzz-target skeleton, aggregate test-crate tooling |
| `src/backends/rust_mc.rs` | 2034 (no unit tests) | model-checker executable generation: exact engine + liveness + JSON report + fuzz runtime + SMT binary |

Position in the pipeline: `src/codegen.rs::CodegenCommandKind::run` dispatches
to the backend's function pointers - `src/backends/rust.rs::RUST_BACKEND`
wires `production` to `src/backends/rust_fsm.rs::generate_production`,
`test` to `src/backends/rust_test.rs::generate_unit_tests`, and
`mc` to `src/backends/rust_mc.rs::generate_model_checker`. All three entry
points funnel into `src/backends/rust_fsm.rs::generate_core` (the shared core
emitter); each command then appends its own runtime. The plugin API, the
options bag, the dispatch rules, and the shared planning data these generators
render are documented in docs/dev/codegen_core.md (`dev:codegen-plugin-api`,
`dev:codegen-test-plan`, `dev:codegen-mc-concepts`) - this doc covers only
what is Rust-specific.

Where to go next:

- src/backends/RUST.md - architecture + design decisions of the generated
  artifacts (anatomy, key decisions, known limitations);
- CODEGEN.md (repo root) - the extension guide: plugin API, CLI surface,
  adding-a-backend checklist, invariants;
- function-level call graph: `dev:deps-fn-codegen-rust` in DEPENDENCIES.md;
- file entries: `dev:code-map-codegen` in CODE_MAP.md;
- decision rationale: D-P3-4/D-P3-5/D-P3-6 (monitors), D-P4-* (MC engine),
  D-P5-10 (JSON report), D-P5-11 (ProtocolFsm), D-P6-3 (artifact header) in
  DECISIONS.md.

Code: `src/backends/rust.rs::RUST_BACKEND`,
`src/backends/rust_fsm.rs::generate_production`,
`src/backends/rust_test.rs::generate_unit_tests`,
`src/backends/rust_mc.rs::generate_model_checker`,
`src/backends/rust_fsm.rs::generate_core`

<a id="dev:backend-rust-plugin"></a>
## Plugin entry + shared Rust helpers (`src/backends/rust.rs`)

**Registration.** `src/backends/rust.rs::RUST_BACKEND` is the static
`Backend` value appended to the registry in `src/codegen.rs`: name "rust",
one-line description, three command descriptors (PRODUCTION_INFO,
TEST_INFO, MC_INFO - each carrying the CLI usage line and per-option help that
backend_help renders verbatim), and the three function pointers into the
generator modules. The descriptor text is user-visible: TEST_INFO documents
`--out` as REQUIRED (the fuzz skeleton is written next to it; stdout mode is
refused), and MC_INFO documents both the codegen-time flags and the generated
binary's runtime flags (including `--fast`, which is non-parity).

**Identifier mangling.** Generated code must compile even when spec names
collide with Rust syntax:

- RUST_KEYWORDS - the 36 keywords that require raw identifiers;
- rust_ident - keyword to `r#keyword`, everything else unchanged; applied to
  every emitted identifier (states, variants, fields, vars, enum paths);
- rust_binding_ident(name, reserved, used) - for PATTERN BINDINGS only: in
  addition to keyword mangling, a binding that collides with an identifier the
  generated transition body uses unqualified is renamed to `<name>__rfsb`
  (further suffixed until unique among `used`). Without this, e.g. a field
  named `out` would shadow mc_step_all's effect vector and every `out.push`
  would hit the binding - silently wrong code from a fully valid spec (Phase 6
  task 6 finding; the C backend has the same bug class via its effect counter
  `n`). The caller passes its own reserved set: the Fsm impl uses
  `{input, handler}`, mc_step_all uses `{out, state, vars}`.

**Name + type mapping.**

- snake - lowercase, non-alphanumerics to `_`, a leading digit prefixed with
  `_`, empty to `_`; used for generated function/variable names (monitor
  structs, test names, group-match helpers);
- bit_rust_type - bit<N> width to u8 (0..=8) / u16 (9..=16) / u32 (17..=32) /
  u64 (33..=64) / u128 (else);
- rust_type - Bool to bool, Bit{w} to bit_rust_type(w), Enum{name} to
  rust_ident(name);
- mask_expr - the width mask: `u128::MAX` for w >= 128, else
  `((1u128 << w) - 1u128)`; wraps every arithmetic/bitwise result so generated
  code has the DSL's wraparound semantics.

**Representative inputs.** representative_input_expr renders a variant's
representative input as Rust source from
`src/codegen_fsm.rs::representative_values` (the value-selection rules live in
the core, task 9.1): unit variant to `Input::Name`, struct to
`Input::Name { f: v, ... }`; unrepresentable enum cases render as
`Enum::__Empty__` / `Enum::__Unknown__`. Used for the ALL_INPUTS constant and
the test module's candidate_inputs().

Unit tests (3): identifier_mangling, type_mapping,
backend_registration_shape.

Code: `src/backends/rust.rs::RUST_BACKEND`,
`src/backends/rust.rs::PRODUCTION_INFO`, `src/backends/rust.rs::TEST_INFO`,
`src/backends/rust.rs::MC_INFO`, `src/backends/rust.rs::RUST_KEYWORDS`,
`src/backends/rust.rs::rust_ident`,
`src/backends/rust.rs::rust_binding_ident`, `src/backends/rust.rs::snake`,
`src/backends/rust.rs::bit_rust_type`, `src/backends/rust.rs::rust_type`,
`src/backends/rust.rs::mask_expr`,
`src/backends/rust.rs::representative_input_expr`

<a id="dev:backend-rust-fsm"></a>
## Production FSM generation (`src/backends/rust_fsm.rs`)

**Entry points.** generate_production maps `--with-property-monitors` to
PropertyMonitorMode Gated/Off and calls generate_core(module, options, false,
mode). generate_core is the core generator shared by all three commands: it
re-validates the module (`src/validator.rs::validate_module` - ALL errors
returned, never partial output), builds the Gen emitter, and returns
CodegenOutput with the rendered code + warnings. The test entry passes
with_tests = true plus Always mode; the mc entry passes with_tests = false
plus Always mode (D-P3-6: test/mc always include property monitors).

**The Gen pipeline.** Gen::new builds the variant table
(`src/codegen_fsm.rs::build_variants`) and compiles the monitors, then runs
emission in a fixed order - prelude, enums, Vars, Input/Output, report
renderers, trace/handler types, monitors, Fsm impl, [ProtocolFsm impl],
[test module]. The uses_vec_macro flag (any transition or on_error emits an
output) decides whether the no_std header imports the `vec!` macro, so the
file stays warning-free without widening the allow list.

**Prelude + artifact header.** Line 1 is the v1.0 unified artifact header
(D-P6-3): `// Generated by RFS v{CARGO_PKG_VERSION} | spec sha256:{spec_hash}
| module {name}` - identical text in both backends so artifacts are
recognizable and drift is detectable. Then the `#![allow(dead_code,
unused_variables, unused_mut, unused_parens, irrefutable_let_patterns,
non_camel_case_types, clippy::all)]` header. With `--no_std`: `#![no_std]`
+ `extern crate alloc;` + `use alloc::string::{String, ToString};` +
`use alloc::vec::Vec;` + `use alloc::format;` (+ `use alloc::vec;` only when
uses_vec_macro). The BODY is byte-identical to the std variant - pinned by
no_std_compile_sweep_and_body_parity_all_valid_examples (tests/codegen_test.rs:
all 50 valid examples x both monitor modes compile under
`rustc --edition 2021 --crate-type lib` and their bodies diff empty).

**Type declarations.** Module enums become `#[derive(Debug, Clone, Copy,
PartialEq, Eq)] pub enum` with rust_ident'd variants; Vars is a struct (unit
when there are no vars) with fields in declaration order; Input/Output are
FLATTENED enums - each variant of every group becomes a top-level variant
named by VariantInfo::unique_name carrying the constructor's fields (flat
enums keep transition matching exhaustive and simple for downstream users).

**Report renderers.** emit_renderers emits value renderers byte-identical to
ref_mc's text/JSON value forms: rfs_render_state (fixed name - State is a
language-reserved type) / rfs_render_e<i> per other enum (`Name::{v:?}`),
rfs_render_input / rfs_render_output (variant + payload), rfs_render_vars.
Declaration order throughout - deterministic artifact bytes. The enum-to-
renderer map comes from `src/backends/rust_fsm.rs::enum_renderer_names`,
shared with the MC JSON renderer so text and JSON carry identical data.

**Trace + assumption runtime.** TraceStep/Trace keep the complete execution
record; Sample { input: Option<Input>, outputs, from_state, to_state, vars }
is the unit the online monitors evaluate (sample 0 = the initial
configuration, no input/outputs, so input_is/output_has are false there).
AssumptionPolicy { Continue, Stop, Panic } + the AssumptionHandler trait +
the three built-in handlers. StepError: UnexpectedEvent,
AssumptionViolation{label}, and - gated in production output by
`#[cfg(debug_assertions)]` (D-P3-6) - PropertyViolation{label, step}; always
present in test/mc output.

**Online monitors.** compile_monitors compiles every property formula via
`src/monitor.rs::compile_formula` (assumptions are filtered by the --assumes
selection first); formulas outside the monitor subset are skipped with a
warning whose text is identical in every backend (D-P3-5). emit_monitors
renders one Mon_<snake(label)> struct per plan (assumptions first, then
properties; declaration order within each group), plus the PropertyViolation
record struct, the PropMonitors/AssumeMonitors aggregates, and the label
constants PROPERTY_LABELS / ASSUME_LABELS (+ SPEC_VERSION from the version
header, ALL_INPUTS = one representative input per variant).

render_monitor numbers the plan's nodes in DFS pre-order; the generated step
body evaluates them in post-order (children before parents), mirroring the
reference interpreter in src/monitor.rs. Per node kind (render_node):

- RangeCheck: n<i>_failed / n<i>_trigger_step / n<i>_now - `now` is
  recomputed every sample (the child may recover), `failed` is sticky for
  reporting;
- Obligation bound 0 (be[0]): the body must hold at the trigger sample
  itself;
- Obligation bound >= 1: the D-P3-3 pending model - n<i>_pending +
  n<i>_remaining countdown; failure is terminal (once confirmed, failing at
  every later sample);
- DelayedCheck: an n<i>_prev_antecedent register (sample 0 vacuous); failure
  is terminal, no trigger sample;
- And/Or: per-sample composition over the children's CURRENT failure status -
  a conjunction fails while ANY child fails, a disjunction only while ALL
  fail; trigger_step = min of the children's (i32::MAX to -1).

Each node's per-sample signal is rendered by gen_signal to
`src/backends/rust_fsm.rs::render_signal_expr` (pub(crate), shared with the
test module's invariant re-checks): VarExpr via gen_expr under a sample
context (state = s.to_state, vars from the sample); InputIs/OutputHas via
input_is_sample_expr / output_has_sample_expr - struct patterns bind each
field by its real name and guard on the pattern constraints (a matches! +
external guard would not compile: pattern bindings do not escape the match).
Anchor ranges become range_cond (`idx as u64 >= from [&& idx as u64 <= to]`,
"true" when unbounded from 0).

The aggregates step in declaration order and report the FIRST newly failed
monitor: AssumeMonitors.step returns the label of a newly failed ACTIVE
assumption (checked against the runtime active_assumes list);
PropMonitors.step returns the first PropertyViolation { label, step: idx,
trigger_step }.

**The Fsm impl.** emit_fsm_impl emits Fsm { state, vars, trace,
active_assumes, assume_monitors, [cfg] prop_monitors, property_violation }:

- new() initializes state/vars (initial_vars_expr lowers the var
  initializers, which may reference earlier vars), records the initial Trace,
  and steps sample 0 through the monitors so countdowns and one-shot
  obligations start correctly - an assumption violated at sample 0 panics
  ("RFS assumption violated: {label}"), a property violation is recorded in
  property_violation;
- step delegates to step_with_handler with PanicAssumptionHandler;
- transition blocks are grouped by state (first-appearance order) and, within
  a state, sorted by DESCENDING effective priority (priority.unwrap_or(0),
  stable sort) - the same firing order the planner uses
  (`dev:codegen-test-plan`), so generated code and planned test cases agree
  by construction;
- commit_step pushes the TraceStep, builds the Sample, steps the assumption
  monitors (handler policy: Continue / Stop to Err(AssumptionViolation) /
  Panic), then checks property monitors (the first violation wins to
  Err(PropertyViolation)), then updates state/vars.

**Transition blocks.** transition_block emits `if self.state == State::X {
<pattern scope> if <guard> { let outputs = vec![...]; let next_state = ...;
let new_vars = {...}; return self.commit_step(...); } }`. Variable updates
are SIMULTANEOUS: every assignment is first computed into a tmp_<var> from
pre-step values, then applied to a clone of self.vars - the same lowering the
planner's static evaluator mirrors (`dev:codegen-test-plan`). on_error_block
is the fallback when no transition matched: no state check, no pattern
bindings, vars carried over unchanged.

**Pattern scopes.** pattern_block_open / pattern_block_close emit the
input-matching scope and return the validator Env with the pattern bindings
populated:

- bare group pattern: single-variant group to a direct `if let Input::V`;
  multi-variant to a generated group-match helper
  input_matches_group_<snake(group)> (emitted per multi-variant group);
- unit variant to `if let Input::V` (error "unit pattern for struct" if the
  variant has fields);
- struct variant: field count checked ("field count mismatch"); named
  bindings go through rust_binding_ident with the caller's reserved set and
  are recorded in env.binding_idents so the expression context maps RFS name
  to mangled ident; unnamed fields bind as _f<i>; literal fields (Int/Bool/
  EnumVariant) become an additional `if b == value` guard inside the scope.

**Expression lowering.** gen_expr lowers the IR expression language to Rust,
typing every subexpression via `src/validator.rs::type_expr`:

- literals as-is; single-segment paths resolve through the ExprCtx name map
  (state to self.state / s.to_state depending on context; vars to
  self.vars.<name> / s.vars.<name>; pattern bindings to mangled idents) or to
  State variants; longer paths render as rust_ident'd qualified paths;
- BitNot/Neg and Plus/Minus/Star: u128 wrapping arithmetic masked to the
  expression's type width (mask_expr + bit_rust_type);
- comparisons cast both sides to u128 (unsigned semantics);
- shifts saturate to 0 when the shift amount >= width;
- BitAnd/BitOr/BitXor on bool operands are LOGICAL (& | ^), else u128 + mask.

width_from_type: an expected Bit(w) from the context wins; IntLit adapts to
8; anything else is an error ("expected bitvector"). This width semantics
must stay in lockstep with the planner's static evaluator (u128 wrapping +
mask, shift saturation, bool bitwise = logical) - drift makes planned test
cases disagree with generated runtime behavior.

**MC/test shared renderers.** `src/backends/rust_fsm.rs::render_mc_step_all`
(pub(crate)) renders mc_step_all(state, vars, input) -> Vec<McEffect> for the
exact MC runtime (D-P4-11): for (state, input), the effects of EVERY enabled
transition in firing order (deterministic mode uses only the first effect;
nondet mode explores all), with the on_error fallback appended when nothing
matched (transition_id usize::MAX). It reuses the same pattern_block_open /
gen_expr / output_expr helpers as Fsm::step - both paths agree by
construction. render_sample_formula_evaluator renders
`fn <name>(s: &Sample) -> bool` for liveness leaves via
`src/monitor.rs::formula_to_signal` + gen_signal (error when the subformula
contains temporal operators).

Code: `src/backends/rust_fsm.rs::generate_production`,
`src/backends/rust_fsm.rs::generate_core`, `src/backends/rust_fsm.rs::Gen`,
`src/backends/rust_fsm.rs::emit_prelude`,
`src/backends/rust_fsm.rs::emit_renderers`,
`src/backends/rust_fsm.rs::compile_monitors`,
`src/backends/rust_fsm.rs::render_monitor`,
`src/backends/rust_fsm.rs::render_node`,
`src/backends/rust_fsm.rs::render_signal_expr`,
`src/backends/rust_fsm.rs::emit_fsm_impl`,
`src/backends/rust_fsm.rs::transition_block`,
`src/backends/rust_fsm.rs::pattern_block_open`,
`src/backends/rust_fsm.rs::gen_expr`,
`src/backends/rust_fsm.rs::render_mc_step_all`,
`src/backends/rust_fsm.rs::render_sample_formula_evaluator`,
`src/backends/rust_fsm.rs::enum_renderer_names`

<a id="dev:backend-rust-test"></a>
## Unit-test, fuzz and aggregate-crate generation (`src/backends/rust_test.rs`)

**Entry point.** generate_unit_tests rejects `--no_std` with the pinned
`src/codegen.rs::NO_STD_PRODUCTION_ONLY_ERROR`, runs generate_core(module,
options, true, Always), and appends one ExtraFile: the fuzz_target.rs
skeleton (below). The test artifact itself stays dependency-free.

**The test module.** emit_test_module renders a `#[cfg(test)] mod tests`
appended to the production source, from the shared plan
`src/codegen_test.rs::build_test_plan(variants, module, steps)` - one plan,
rendered per backend (the planning semantics are `dev:codegen-test-plan`; this
section covers only the rendering). candidate_inputs() = one representative
input per variant. Test classes (names are STABLE and mirrored by the C
backend):

- initial_state_is_correct - Fsm::new() starts in the declared initial state;
- positive_sequence_runs - greedy drive with representative inputs for up to
  K steps (trial clone + StopAssumptionHandler); asserts progress when the
  plan's expect_progress holds (the exact computation, not a heuristic);
- negative_unexpected_input_search - probes for an input that fails with
  UnexpectedEvent (ContinueAssumptionHandler); without on_error it asserts
  the failure is found when definite_unexpected; with on_error and a
  statically unhandled input it instead asserts the fallback state is reached
  ("on_error fallback was never reached");
- t<id>_<state>_on_<event> (+ _not_taken) - per-transition tests
  (render_transition_test): install the case's state + COMPLETE variable
  configuration, drive the concrete input with ContinueAssumptionHandler, and
  assert on the RECORDED trace step (to_state + outputs) - robust even when a
  property violation halts the commit. Not-taken asserts UnexpectedEvent + no
  recorded step, or a landing observably different from the transition's own
  signature; STRICT: when the signature's output args are not statically
  evaluable the emitter refuses to emit a toothless test (codegen error);
- invariant_<label> - re-checks the property's per-sample atom
  (render_signal_expr, shared with monitor rendering) at every sample of the
  positive drive trace, failing with the step index; bounded invariants check
  samples < N only;
- monitor_<label>_fires_on_violation - drives the planner's violating trace
  and asserts the sticky property_violation record holds the target label
  (case 1: the initial configuration already violates - Fsm::new() records it
  at sample 0; case 2: one planned drive step). The planner guarantees no
  other monitored property records first;
- random_trace_seed<seed>_steps<K> - drives the precomputed LCG index table
  over ALL_INPUTS (same constants as the fuzz wire format, seed
  `src/codegen_test.rs::RANDOM_TRACE_SEED` = 12345) and prints ONE
  deterministic coverage line: `random_trace seed={} steps={} ok={} unexpected={}
  assumption={} property={} states_visited={}`.

**Literal rendering.** var_value_literal / vars_initializer (panics "planner
must cover variable" if the planner missed a var - a contract violation, not a
runtime condition) / input_case_expr / resolve_output_variant (bare output to
the group's single variant) / output_case_full_literal (None when some arg is
not statically evaluable). render_outputs_assert: full assert_eq! when every
arg is known, otherwise per-output partial match assertions (variant + known
fields only - the documented planner fallback for None args).

**Fuzz skeleton (separate file).** emit_fuzz_target renders fuzz_target.rs:
line 1 = the same v1.0 artifact header; `#![no_main]` +
libfuzzer_sys::fuzz_target!. Decoding per the planner's
`src/codegen_test.rs::FuzzDecodePlan`: the first byte selects the input
variant (byte % n_variants); each field follows - bool = one non-zero byte,
bit<W> = ceil(W/8) little-endian bytes masked to W bits, enum = constant
representative (first declared variant, else __Unknown__); a budget check
breaks when the remaining data cannot cover the selected variant's fields.
The loop steps decoded inputs over Fsm::step with StopAssumptionHandler until
the input is exhausted. It is deliberately a SEPARATE file: it depends on
libfuzzer_sys, which the dependency-free test artifact (and the aggregate
crate) must not pull in; the CLI writes it next to --out and refuses stdout
mode for the test command.

**Aggregate tooling.** generate_aggregate_test_crate renders one crate with a
`pub mod` per module - name m{:03}_<snake(file_stem)>_<snake(module.name)>
(collision-free across files) - from every RFS file, stamping the WHOLE
file's spec hash onto a per-file options clone (D-P6-3). The fuzz skeleton is
NOT part of the aggregate. The manifest carries an empty [workspace] table so
the generated crate is its OWN workspace root (without it, cargo walking up
from target/ discovers the repository's workspace and refuses to build a
non-member manifest - Phase 5 task 1). write_generated_crate writes it to
<root>/target/rfs_codegen_tests. This powers cargo generate-code-tests /
cargo test-generated-code.

Code: `src/backends/rust_test.rs::generate_unit_tests`,
`src/backends/rust_test.rs::emit_test_module`,
`src/backends/rust_test.rs::render_transition_test`,
`src/backends/rust_test.rs::render_invariant_test`,
`src/backends/rust_test.rs::render_monitor_test`,
`src/backends/rust_test.rs::render_random_trace_test`,
`src/backends/rust_test.rs::emit_fuzz_target`,
`src/backends/rust_test.rs::generate_aggregate_test_crate`,
`src/backends/rust_test.rs::write_generated_crate`

<a id="dev:backend-rust-mc"></a>
## Model-checker executable generation (`src/backends/rust_mc.rs`)

**Entry point.** generate_model_checker rejects `--no_std`
(`src/codegen.rs::NO_STD_PRODUCTION_ONLY_ERROR`) and `--with-protocol-fsm`
(`src/codegen.rs::PROTOCOL_FSM_MC_ERROR` - the trait impl belongs to the
production/test surface; the model checker keeps its own runtime). Three
shapes:

1. SmtBounded mode: a standalone z3-based binary - no FSM core, no exact/fuzz
   runtimes. The file = v1.0 header + the shared spec section
   (`src/smt_emit.rs::emit_rust_spec` over
   `src/smt_plan.rs::build_smt_spec(module, McPlan::compute(...))`, so both
   backends build structurally identical constraints) + SMT_ENGINE_RUST with
   {BOUND} replaced by options.mc_smt_bound.
2. Exact / fuzz (default): the production FSM core via generate_core (no unit
   tests, property monitors ALWAYS per D-P3-6; the stamped spec_hash carries
   into the core's line-1 header) + the exact_codegen output + FUZZ_RUNTIME +
   MAIN_DISPATCH (the generated main dispatches on the runtime --mode flag).

**exact_codegen - plan tables + engines.** Renders, in order:

- codegen-time defaults embedded as runtime-overridable constants (D-P4-7):
  DEFAULT_MODE (0 exact / 1 fuzz), DEFAULT_MAX_STATES, DEFAULT_NONDET,
  DEFAULT_TIMEOUT_SEC, DEFAULT_THREADS, DEFAULT_COUNTEREXAMPLE_FORMAT (0 text
  / 1 json), plus CONFIG_TOTAL_BYTES + MONITORS_BITS from the plan encoding;
- all_inputs() - the canonical enumeration (D-P4-2): variants in declaration
  order, concrete fields iterated over their domain (domain_iter: Bool
  [false, true], Int 0..=max, Enum qualified variant paths) nested in
  declaration order, abstracted fields at representative values (rep_expr);
- put_bits + config_bytes(state, vars, prop, assume) - the plan's
  configuration layout: state index bits, one slot per variable
  (var_value_expr: bool to 1/0, bit to as u64, enum to variant index), then
  monitor node bits (pending; remaining packed ONLY while pending - matching
  ref_mc's CfgKey byte-for-byte so config identity agrees with the reference
  engine; antecedent);
- render_mc_step_all (from rust_fsm - every enabled transition in firing
  order);
- EXACT_ENGINE - Node { state, vars, prop, assume }, StepOutcome { Success /
  Pruned / Violation(label, step, trigger) }, step_successor (steps the
  monitors at depth + 1; an active assumption prunes, a property violation
  reports), path_to (parent pointers), replay (rebuilds the Phase 3 trace
  format on a fresh Fsm - nondet picks the effect with the recorded
  transition_id, else the first), and exact_summary_line: the pinned D-P4-4
  machine line `model-check summary: status={} states={} transitions={}
  max_depth={}` + on fail ` reason={} label={} step={}` (deadlock uses
  label=- and the dead config's depth as step);
- JSON_WRITER_RUST + json_renderers_rust - see below;
- liveness (D-P4-9/10/11): `src/mc_plan.rs::LivenessDecomposition` compute
  (diagnostics become codegen WARNINGS), LIV_ENGINE, per-property leaf
  evaluators liv_p<i>_l<j>[b] via render_sample_formula_evaluator (Safety
  kind 0 / Eventually kind 1 / Until kind 2 with eval_b), flatten_skeleton
  re-indexing the leaves in first-appearance order into liv_p<i>_skel(t:
  &[bool]), and the LIV_PROPS table;
- EXACT_RUN - see below.

**The exact engine (EXACT_ENGINE + EXACT_RUN).** run_exact() -> ! parses the
runtime flags (--max-states / --nondet / --timeout-sec / --threads / --fast /
--counterexample-format / --assumes) over the codegen-time defaults, then:

- initial configuration via Fsm::new (sample 0; an assumption violated at
  sample 0 panics; a property violation at sample 0 is reported immediately -
  JSON or text);
- level-synchronous BFS (D-P4-2): canonical input order, successors in firing
  order; visited set checked at ENQUEUE time over config_bytes; parent
  pointers + depth for counterexample reconstruction; graph edges recorded for
  every Success (including edges to already-visited configs) for liveness; a
  complete flag per config;
- status codes: 0 running / 1 pass-complete / 2 state limit / 3 timeout /
  4 fail; exit codes 0 / 1 / 2;
- parallel level expansion (task 10B): with --threads N > 1, phase A computes
  every successor outcome of the current level in parallel
  (std::thread::scope workers over disjoint frontier chunks calling the pure
  expand_config), and phase B commits them SEQUENTIALLY in canonical order -
  visited insert at enqueue, parent/depth/adj pushes, first violation,
  deadlock check - exactly as the single-threaded engine does. Results are a
  pure function of the spec: byte-identical to --threads 1 for any N.
  --fast (non-parity) stops scheduling work once any worker finds a violation
  and reports the first found one - states/transitions/label may vary between
  runs, never a false pass;
- outcome semantics: Unexpected = an input matches no transition and there is
  no on_error - the path terminates (D-P4-5), counted in statistics without
  touching the transitions counter; Pruned counts as a transition; Violation
  to status 4 + fail_path; Success to enqueue under max_states, else status
  2. Deadlock (D-P4-5): a reachable configuration with no successor at all to
  FAIL reason=deadlock label=- step=depth. SEMANTICS 9.5: the search STOPS at
  the first violation or state limit - the check sits AFTER the per-level
  loop (a Violation outcome breaks out of it), matching the C engine's
  status == 0 loop guard (Phase 6 task 4 finding, correction 4b);
- liveness analysis runs only when BFS ended without a safety failure or
  deadlock. On a truncated graph (state limit / timeout) it is a TRUNCATED
  analysis: a witness found inside the explored part is sound (all its edges
  are real transitions), otherwise the UNKNOWN status stands - never a false
  pass-complete;
- report: JSON mode prints ONE compact document and exits; text mode prints
  PASS (complete) / FAIL / UNKNOWN (state limit reached[, timeout after Ns])
  + the pinned summary line + statistics (unexpected terminations, first
  unexpected, monitor size in bits, memory estimate). Counterexamples are
  reconstructed by replay into the Phase 3 trace format.

**Liveness engine (LIV_ENGINE).** Per property: a boolean skeleton over
per-leaf progress automata - safety 0=ok/1=bad, eventually 0=wait/1=done,
until 0=ok/1=sat/2=bad; all transitions are one-way, so every state sequence
on an infinite path is eventually constant (the limit vector). Labels are
EDGE-based: position 0 uses the initial-configuration sample, position k uses
the sample of step k - input_is/output_has atoms inside obligation bodies
depend on the edge taken. liv_analyze: product BFS from (start_q, config 0)
over reachable[q] sets; per limit vector q, a stay-edge subgraph on R(q); in
nondet mode a bad cycle must be STRONGLY FAIR (D-P4-11) - every enabled
successor of a cycle state is taken on the cycle, i.e. each cycle state has
exactly one explored successor and was fully expanded when truncated;
iterative Kosaraju SCCs; cyclic = component size > 1 or self-loop; the limit
vector t falsifies the skeleton to witness. liv_witness reconstructs the
lasso: a product path from (start_q, 0) to (q, cstar) plus a stay-edge cycle
through cstar (the smallest cyclic config); liv_note explains which leaves
are falsified ("safety subformula false at step N" / "eventually body never
holds on the path" / "until left fails at step N without right holding").

**JSON counterexample report (JSON_WRITER_RUST).** An embedded minimal copy
of the toolchain's compact JSON writer with u64 integers (every value in this
document is non-negative; bit<64> fields reach 2^64-1) - the generated binary
stays self-contained by construction. The escape contract matches the
toolchain writer exactly: `"` `\` `\b` `\t` `\n` `\f` `\r`, other control
chars U+0000..=U+001F as \u00XX (lowercase hex), everything else raw UTF-8.
build_mc_json_doc builds the canonical document - KEY ORDERING IS PART OF THE
BYTE-PARITY CONTRACT (the model is `dev:codegen-mc-concepts`): status,
metrics { states, transitions, max_depth }, [reason, label, step on fail],
statistics { unexpected_terminations, monitor_size_bits,
memory_estimate_bytes, peak_rss_kb }, [counterexample]. peak_rss_kb
self-reports VmHWM from /proc/self/status (0 on read failure) -
INFORMATIONAL and machine-dependent; cross-backend byte comparisons normalize
it to 0. Counterexample kinds: trace (ce_steps = steps.len()), lasso (prefix
/ cycle / cycle_length; ce_steps = total), deadlock-config (depth / state /
vars; ce_steps = depth) - ce_steps is the quality metric and ALWAYS the last
key. json_renderers_rust renders this module's input/output/vars values with
the same name source as the text renderers, so JSON strings carry ref_mc's
prefixed value forms identical to text.

**Fuzz runtime (FUZZ_RUNTIME).** The legacy Phase 3 engine behind --mode
fuzz: an embedded LCG Rng (state = state * 6364136223846793005 +
1442695040888963407 wrapping; top-53-bit double mapping - the SAME constants
as `src/codegen_test.rs::Lcg`), ViolationSpec parsing (the label:prob:percent%
wire format, defaults 1.0/100.0), parse_assume_selection (all / none / labels
filtered against ASSUME_LABELS), print_trace + vars_at via the renderers, and
run_fuzz with runtime flags --bound 5 / --runs 20 / --seed 12345 / --assumes /
--violate / --unexpected-prob (the `src/codegen_mc.rs::McConfig` defaults).
Per step: forced violation injection first (trial-stepping ALL_INPUTS for an
input that violates a target label), then optional unexpected injection, else
allowed representative inputs - inputs that confirm a property violation are
allowed on purpose (the checker wants to find violations, not avoid them).
Property violations terminate the checker immediately (exit 1), so the
summary's property_violations counter is always 0 (the field stays for a
stable format): `model-check summary: runs={} bound={} completed={} unexpected={}
assumption_stops={} property_violations={}`.

**SMT engine (SMT_ENGINE_RUST).** The standalone z3 runtime appended to the
shared spec section. Parity-critical conventions (mirrored exactly by the C
backend): step variables are z3 constants with FIXED names declared in a fixed
order (s_i, v_i_j, ev_i, fld_i_g); constraints are asserted in a fixed order;
queries run in a fixed order - safety properties in declaration order, then
liveness. b_and/b_or are two-argument wrappers over z3 0.21's variadic
associated functions (identical to the C backend). The unrolling:
enabled_t = state match and class match and guard (firing order); fire_t =
enabled_t and no earlier transition enabled (first match, prefix-OR chain);
alive_i = alive_{i-1} and any_fire; next state/vars via ITE chains. Safety
query viol_p = exists i<k: alive_i and not phi_p(i) to FAIL +
`model-check summary: status=fail mode=smt-bounded bound={k}
reason=property-violation label={} step={}` exit 1. Liveness query never_p
(phi holds at no alive step) and cycle (exists a<b: identical (state, vars))
to FAIL reason=liveness-violation + cycle length, exit 1. Unknown to
`UNKNOWN (solver inconclusive within bound {k})` + status=unknown, exit 2.
All Unsat to `PASS WITHIN BOUND {k} (incomplete - no completeness claim)` +
status=pass-within-bound, exit 0. smt_print_trace emits the pinned witness
format (`witness trace (solver model; bound k={}):` + per-step state/vars/
event lines).

**Embedded-copy rule.** Because the generated binary cannot depend on this
crate, rust_mc.rs embeds its own copies of the plan tables, parsing helpers,
and JSON writer. The formats in `src/codegen_mc.rs` / `src/mc_plan.rs` are
the SOURCE OF TRUTH - when a wire format changes, update both backends'
emitters in the same change and extend the tests (CODEGEN.md section 9).

Code: `src/backends/rust_mc.rs::generate_model_checker`,
`src/backends/rust_mc.rs::exact_codegen`,
`src/backends/rust_mc.rs::json_renderers_rust`,
`src/backends/rust_mc.rs::flatten_skeleton`,
`src/backends/rust_mc.rs::domain_iter`, `src/backends/rust_mc.rs::rep_expr`,
`src/backends/rust_mc.rs::var_value_expr`,
`src/backends/rust_mc.rs::EXACT_ENGINE`, `src/backends/rust_mc.rs::LIV_ENGINE`,
`src/backends/rust_mc.rs::JSON_WRITER_RUST`,
`src/backends/rust_mc.rs::EXACT_RUN`,
`src/backends/rust_mc.rs::MAIN_DISPATCH`,
`src/backends/rust_mc.rs::FUZZ_RUNTIME`,
`src/backends/rust_mc.rs::SMT_ENGINE_RUST`

## Changing the Rust backend

Developer checklist (the placement rule is CODEGEN.md section 2 - core-level
analysis goes to the language-agnostic modules, target syntax stays here):

1. Parity first: a new feature lands in EVERY backend in the same change,
   with cross-backend differential tests (tests/backend_parity_test.rs;
   liveness: tests/liveness_mc_test.rs; performance:
   tests/benchmark_mc_test.rs). The C twin of each file is src/backends/c*.rs
   (deep dive: task 10.x).
2. Pinned formats that break on drift - change them only with the parity
   suite updated in the same commit:
   - the line-1 artifact header (D-P6-3);
   - test names + the random-trace coverage line;
   - the fuzz decode wire format (variant selector byte, field encodings) and
     the LCG constants/seed;
   - config_bytes layout (must match ref_mc's CfgKey byte-for-byte);
   - the exact summary line + statistics block;
   - JSON key ordering + the escape contract (ce_steps last; peak_rss_kb
     normalized to 0 in comparisons);
   - SMT variable naming, constraint order, query order, and the witness
     trace format.
3. Firing order everywhere is descending effective priority then source
   order - it must mirror generated fsm_step AND the planner's static
   analysis (`dev:codegen-test-plan`). The pattern-scope reserved sets
   ({input, handler} in the Fsm impl; {out, state, vars} in mc_step_all)
   exist because of this same shadowing class of bug.
4. gen_expr width semantics (u128 wrapping + mask to type width, shift
   saturation at width, bool bitwise = logical) must stay in lockstep with
   the planner's static evaluator - drift makes planned test cases disagree
   with runtime behavior.
5. Generated artifacts stay dependency-free and deterministic: declaration
   order everywhere; the fuzz skeleton stays a separate file; the aggregate
   crate keeps its empty [workspace] table.
6. no_std: only the header may change - the body must stay byte-identical (the
   compile sweep pins this over all 50 valid examples x both monitor modes).
7. Verify: cargo test --test codegen_test, cargo test --test
   backend_parity_test, cargo test --test liveness_mc_test, cargo test
   --test benchmark_mc_test, then the full BUILD.md section 5 checklist.
   Docs sync: src/backends/RUST.md (artifact design), CODEGEN.md (if the
   plugin surface changes), KB.md; training snippets if user-visible.
