# RFS Codegen Core (Class A)

The language-agnostic core of the code generator: the backend plugin API and
the shared planning/concept modules that all backends render from.

<a id="dev:codegen-core-overview"></a>
## Overview

RFS code generation is a language-agnostic CORE plus per-language backends.
The core (four files) defines the plugin API, parses the `rfs codegen` CLI,
and computes everything that does not depend on target syntax - variant
flattening, the unit-test plan, and the model-checker concepts. Backends
(`src/backends/`, one module tree per language) render that data into their
own syntax. The governing principle is PARITY: one plan, rendered identically
in Rust and C - cross-backend differential tests assert byte-identical
artifacts and identical observable behavior for the same spec.

| File | Lines | Role |
|---|---|---|
| `src/codegen.rs` | 657 | plugin API (`Backend`, registry), shared option/output types, CLI request parsing, help rendering |
| `src/codegen_fsm.rs` | 231 (3 unit tests) | production-FSM analysis: variant flattening, representative values, legacy safety-subset helpers |
| `src/codegen_test.rs` | 2320 (27 unit tests) | the static unit-test planner: decides WHAT to test, emits data never code |
| `src/codegen_mc.rs` | 863 (12 unit tests) | model-checker concepts: run config, status taxonomy, pinned summary line, JSON report model |

Position in the pipeline: a validated `Ir` document goes through
`src/codegen.rs::CodegenCommandKind::run`, which stamps the spec hash and
dispatches to the backend's function pointer for the command kind; the
backend renders the shared plans into one self-contained artifact per command
(production FSM source, unit-test executable source, model-checker executable
source), each carrying the line-1 provenance header (see `dev:spec-hash`).

Where to go next:

- CODEGEN.md (repo root) - the extension guide: plugin API walkthrough, CLI
  surface, shared-module summaries incl. backend rendering details, the
  adding-a-backend checklist, testing conventions, invariants;
- algorithm levels (L1/L2/L3): `dev:algorithms-ir-codegen` in ALGORITHMS.md;
- core call graph: `dev:deps-fn-codegen-rust` in DEPENDENCIES.md;
- file entries: `dev:code-map-codegen` in CODE_MAP.md;
- decision rationale: D-P4-7 (MC modes), D-P6-3 (artifact header) in
  DECISIONS.md.

Code: `src/codegen.rs::Backend`, `src/codegen.rs::BACKENDS`,
`src/codegen.rs::CodegenCommandKind`, `src/codegen_fsm.rs::build_variants`,
`src/codegen_test.rs::build_test_plan`, `src/codegen_mc.rs::ExactStatus`

<a id="dev:codegen-plugin-api"></a>
## Backend plugin API + CLI (`src/codegen.rs`)

**The plugin value.** `src/codegen.rs::Backend` is STATIC DATA - no state,
no construction: a CLI name, a one-line description, a slice of
`src/codegen.rs::BackendCommandInfo` descriptors (name, summary, usage line,
option docs per command), and one function pointer per command kind:
`generate_production`, `generate_unit_tests`, `generate_model_checker`, each
with the exact signature `fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>`.
Adding a target language means adding a module under `crate::backends`
that defines such a value and appending it to the registry - that is the ONLY
core change (the full checklist is CODEGEN.md section 6).

**The registry.** `src/codegen.rs::BACKENDS` currently holds the C and Rust
backends (`ALL_BACKENDS`). `src/codegen.rs::list_backends` feeds
`rfs codegen --list`; `src/codegen.rs::get_backend` resolves a CLI name;
`src/codegen.rs::common_help` and `src/codegen.rs::backend_help` render help
from registry data only - no generation happens in the help paths.

**Command kinds + dispatch.** `src/codegen.rs::CodegenCommandKind` has three
variants (Production / UnitTests / ModelChecker) with CLI names
`production`, `test`, `mc` (`from_str`). Two dispatch entry points:
- `src/codegen.rs::run` - the STANDARD dispatch (the CLI rule): compute the
  v1.0 spec hash over the canonical IR JSON of the WHOLE input file
  (`spec_hash16` of `Ir::to_json()` text), stamp it into a CLONED options set,
  and generate for the FIRST module of the file. Multi-module files share the
  file's hash; one hash per artifact.
- `src/codegen.rs::run_module` - per-module dispatch for library consumers
  that generate each module individually (the CLI first-module rule is NOT
  applied); the caller owns `options.spec_hash` and must stamp the same
  whole-file hash when parity with CLI artifacts matters.

**The shared options bag.** `src/codegen.rs::BackendOptions` carries
everything a command may need; backends ignore fields that do not apply:

| Field | Used by | Notes |
|---|---|---|
| `assumptions` (`AssumptionSelection`: All default / None / Labels) | production, mc | which assumptions to activate |
| `steps` (default 4) | test | example-run length K |
| `no_std` | production only | Rust: `#![no_std]` + `extern crate alloc`, identical public API; C: documented no-op; rejected elsewhere with the pinned `NO_STD_PRODUCTION_ONLY_ERROR` |
| `with_property_monitors` | production only | test/mc always include monitors; rejected for them |
| `mc_mode` (`McMode`: Exact default / Fuzz legacy / SmtBounded z3) | mc | embedded as the generated binary's runtime default |
| `mc_max_states` / `mc_nondet` / `mc_timeout_sec` / `mc_threads` / `mc_smt_bound` | mc (exact / smt-bounded) | codegen-time defaults, runtime-overridable; see the MC concepts section |
| `counterexample_format` (`CounterexampleFormat`: Text default / Json) | mc exact mode only | fuzz/smt keep their own output |
| `with_protocol_fsm` | production + test (Rust backend only) | opt-in `impl ProtocolFsm for Fsm` linking the artifact to `rfs-adapters`; `mc` rejects it with the pinned `PROTOCOL_FSM_MC_ERROR`; C ignores it |
| `spec_hash` | all commands | stamped by `run`; empty only when a library caller did not stamp it |

The two pinned rejection constants (`src/codegen.rs::NO_STD_PRODUCTION_ONLY_ERROR`,
`src/codegen.rs::PROTOCOL_FSM_MC_ERROR`) are shared by the CLI parser AND the
backend entry points, so tests pin one string for every path.
`src/codegen.rs::PropertyMonitorMode` (Off / Gated / Always) is how backends
map monitor activation to their compile-time debug switch: Rust renders
Gated as `#[cfg(debug_assertions)]`, C as `#ifdef RFS_PROPERTY_MONITORS`;
test and mc always use Always.

**Output.** `src/codegen.rs::CodegenOutput` = generated source text +
non-fatal warnings (e.g. properties outside the translated subset) +
`src/codegen.rs::ExtraFile` entries written next to `--out`. The Rust/C
`test` commands emit their fuzz skeleton (`fuzz_target.rs` / `fuzz_target.c`) as an
EXTRA file because it depends on libFuzzer and must not be inlined into the
dependency-free test artifact; the CLI refuses to run a command with extras
in stdout mode.

**CLI parsing.** `src/codegen.rs::parse_codegen_args` parses
`rfs codegen [--list | --help | <backend> ...]` into a
`src/codegen.rs::CodegenRequest`, collecting ALL errors before failing
(through the `src/codegen.rs::finish` helper). It enforces command-scoped
rejections (`--with-property-monitors` and `--no-std` only for production;
`--with-protocol-fsm` not for mc), validates numeric options, and rejects
unknown options/extra arguments. The full option table with defaults is in
CODEGEN.md section 4.

Code: `src/codegen.rs::Backend`, `src/codegen.rs::BACKENDS`,
`src/codegen.rs::get_backend`, `src/codegen.rs::CodegenCommandKind`,
`src/codegen.rs::run_module`, `src/codegen.rs::BackendOptions`,
`src/codegen.rs::PropertyMonitorMode`, `src/codegen.rs::CodegenOutput`,
`src/codegen.rs::parse_codegen_args`

<a id="dev:codegen-fsm-helpers"></a>
## Production FSM shared helpers (`src/codegen_fsm.rs`)

**Variant flattening.** Event/output constructor GROUPS become flat variant
lists for code generation. `src/codegen_fsm.rs::build_variants` produces a
`src/codegen_fsm.rs::FsmVariants` (inputs + outputs) of
`src/codegen_fsm.rs::VariantInfo` entries: the owning group, the variant name,
the pre-mangling base name, the collision-free unique name, and the field
names/types in declaration order (raw IR names - backends apply their own
identifier rules when emitting). The naming scheme, shared by all backends:
- a group with a SINGLE variant whose name equals the group name keeps that
  bare name (`event Tick` -> `Tick`);
- otherwise the base name is `group_variant` (`event Recv(Syn)` ->
  `Recv_Syn`);
- duplicates get numeric suffixes: `Name`, `Name2`, `Name3`, ...
  (the first duplicate keeps the bare base).

**Representative values.** `src/codegen_fsm.rs::representative_values` gives
one default value per field of a variant, used to synthesize candidate inputs
for tests and model checking: bool -> `false`, `bit<N>` -> `0`, enum -> the
FIRST declared variant as a full path. `src/codegen_fsm.rs::RepValue` also has
two unrepresentable cases - `EnumEmpty` (the enum declares no variants) and
`EnumUnknown` (the type is not declared in the module) - so callers can tell
"no value exists" apart from a concrete one.

**Legacy safety-subset helpers.** `src/codegen_fsm.rs::contains_liveness`
(Phase 1 subset: only `bounded_eventually` counts as liveness) and
`src/codegen_fsm.rs::safety_formula_inner` (the inner formula of a
translatable safety property - `always(phi)` where phi is liveness-free;
None otherwise) are kept for tests. Monitor compilability is now decided by
the shared compiler in `src/monitor.rs`, so both backends warn identically
for out-of-subset formulas; the test planner uses its own superset
`src/codegen_test.rs::has_liveness_op` (see below).

Code: `src/codegen_fsm.rs::build_variants`,
`src/codegen_fsm.rs::VariantInfo`, `src/codegen_fsm.rs::FsmVariants`,
`src/codegen_fsm.rs::representative_values`, `src/codegen_fsm.rs::RepValue`,
`src/codegen_fsm.rs::safety_formula_inner`

<a id="dev:codegen-test-plan"></a>
## Unit-test planning (`src/codegen_test.rs`)

The planner decides WHAT to test; backends render the actual test code from
the plan data (the parity principle). It is STATIC and language-agnostic:
it emits data - cases, expected outcomes, labels - never code. Where a case
cannot be decided statically, `src/codegen_test.rs::PlannerNote` records the
documented fallback (renderers may surface it as a test comment).

**The plan.** `src/codegen_test.rs::build_test_plan(variants, module, steps)`
returns a `src/codegen_test.rs::TestPlan`:
- `steps` - the example-run length K;
- `expect_progress` - whether the positive drive is expected to make at least
  one step from the initial configuration;
- `definite_unexpected` / `has_on_error` - negative-test selection (see below);
- `transitions: Vec<TransitionTestCase>` - per-transition cases (declaration
  order; a guarded transition may contribute up to two);
- `invariants: Vec<InvariantCase>` - safety properties re-checked at every
  sample of the positive drive trace;
- `monitor_violations: Vec<MonitorViolationCase>` - injected violating traces
  for property monitors;
- `random_trace: RandomTraceCase` - deterministic LCG input sequence,
  precomputed at plan time;
- `fuzz_decode: FuzzDecodePlan` - per-variant field decode strategies for the
  fuzz skeletons;
- `notes` - the documented fallbacks.

**Expecting progress (exact, not heuristic).**
`src/codegen_test.rs::expect_progress_from_initial` computes EXACTLY as the
generated driver runs: its candidates are the per-variant representative
inputs, and a candidate progresses iff some transition OUT OF THE INITIAL
STATE matches it and its guard passes under the initial variable values plus
the candidate's binding values. Field literals must accept the representative
value (an int literal must be 0, a bool literal false, an enum literal the
first variant) or that arm can never fire under the candidates. The old
heuristic (`!transitions.is_empty()`) over-approximated: a spec whose initial-
state arms all reject the representative inputs (e.g. `w > 0` guards with the
w = 0 representative) would generate a failing positive drive (Phase 6 task 6
finding, spec 47).

**Negative-test selection.** `src/codegen_test.rs::has_definite_unexpected`
is true when at least one input variant is never matched by any transition
pattern - its representative value is guaranteed to produce an
`UnexpectedEvent` (or the `on_error` fallback landing) somewhere reachable;
a BARE group pattern marks ALL of the group's variants as used. When true and
the module declares `on_error`, steps never fail with `UnexpectedEvent`, so
the negative test asserts the fallback state is reached instead.
`src/codegen_test.rs::is_deadlocked_initial` covers the extreme case: inputs
exist but there are no transitions at all.

**Per-transition cases.** For each transition (declaration order),
`src/codegen_test.rs::plan_transitions`:
1. builds the representative concrete input matching its pattern
   (`src/codegen_test.rs::concrete_input_for_pattern`): the named variant, or
   the group's FIRST variant for whole-group patterns; literal pattern fields
   keep their literals, wildcard/bound fields take representative values;
   binding values are keyed by BINDING name (guards reference bindings, not
   declared field names);
2. finds SIBLINGS: other transitions in the same state whose pattern can match
   THIS concrete input (`src/codegen_fsm.rs`-style group + variant check via
   `src/codegen_test.rs::pattern_can_match_variant`);
3. orders them by FIRING ORDER - descending effective priority
   (`priority.unwrap_or(0)`), then source order, stable sort - which MIRRORS
   the generated `fsm_step`; an UNGUARDED sibling that fires earlier SHADOWS
   the transition for this input (a note is emitted; no taken case is
   possible);
4. runs a STATIC SEARCH over concrete assignments to the guard-relevant
   variables (`src/codegen_test.rs::enumerate_assignments`: candidate values
   bool [false, true], bit<N> [0, 1, 2^N-1] deduplicated, enum = all declared
   variants; lexicographic product with the last variable as the fastest
   counter; capped at `src/codegen_test.rs::MAX_STATIC_ASSIGNMENTS` = 4096,
   which only trims the tail of a deterministic enumeration):
   - a TAKEN case needs the transition's guard true AND every earlier sibling
     disabled by the same assignment;
   - a NOT-TAKEN case needs the guard false AND the decision statically
     OBSERVABLE (`src/codegen_test.rs::not_taken_observable`): no matching
     sibling fires with an outcome identical to the transition's signature
     (next state + emits), and the `on_error` landing differs from it too -
     otherwise "not taken" is indistinguishable from "taken";
5. installs a COMPLETE configuration (`src/codegen_test.rs::complete_configuration`):
   every module variable in declaration order, searched values for the
   guard-relevant ones and representatives for the rest - EXACTLY what the
   static evaluator assumes for unassigned variables, so the installed runtime
   configuration matches the analysis.

**Invariants.** `src/codegen_test.rs::select_invariants` picks the in-subset
SAFETY properties: `always(p)` / `always[N](p)` where p is a liveness-free
per-sample signal (via `monitor::formula_to_signal`); temporal operators
inside (e.g. `next`) are not per-sample invariants and are skipped.
Liveness-bearing properties (`bounded_eventually`, `eventually`, `until` -
`src/codegen_test.rs::has_liveness_op`, the superset of
`src/codegen_fsm.rs::contains_liveness`, mirroring the Phase 4 MC
classification) belong to the model checker and are excluded.

**Monitor-violating traces.** `src/codegen_test.rs::plan_monitor_violations`
plans, for properties of the shape `always[N](p)` where p is a boolean atom
over state/vars (the monitor compiles to a stateless range check, so directly
installing a violating configuration is sound):
- case 1: the INITIAL configuration already violates - no step needed; the
  monitor records it at sample 0 of `Fsm::new()` (`step_input` None);
- case 2: search (state x assignment x outgoing transition) for a one-step
  drive whose POST-STEP configuration violates the atom. The search only
  accepts a transition the generated `fsm_step` would REALLY take - a
  higher-firing sibling (priority desc, source order) that matches the input
  and whose guard holds is skipped (`src/codegen_test.rs::earlier_sibling_fires`,
  task 5 correction C5-1); assignments apply SIMULTANEOUSLY from pre-step
  values (`src/codegen_test.rs::apply_assignments`, like the generated temp-
  next-variable lowering).
Other shapes (temporal bodies, `input_is`/`output_has` atoms) are skipped with
a note - they are covered by the MC suites.

**Random trace + fuzz decode.** The random-trace test drives a deterministic
LCG input sequence: `src/codegen_test.rs::Lcg` uses the SAME constants and
top-53-bit double mapping as the generated fuzz runtimes (multiplier
6364136223846793005, increment 1442695040888963407, wrapping), with the fixed
seed `src/codegen_test.rs::RANDOM_TRACE_SEED` (12345); the index sequence is
precomputed at plan time and byte-pinned by unit tests.
`src/codegen_test.rs::build_fuzz_decode` emits per-variant field strategies
(`src/codegen_test.rs::FuzzFieldStrategy`): bool -> one nonzero byte,
bit<N> -> ceil(N/8) little-endian bytes masked to N bits, enum -> constant
representative value; the renderer selects the variant as `first_byte %
variants.len()`.

**The static evaluator.** Guard/atom decisions must agree with the generated
runtime, so the private evaluator mirrors the backend lowering EXACTLY:
u128 wrapping arithmetic masked to the expression's type width
(`src/codegen_test.rs::width_of`: the validator's Bit(w), or 8 for integer
literals; `src/codegen_test.rs::mask`), shifts saturate to 0 when the amount
reaches the width, and bool bitwise ops are logical. `src/codegen_test.rs::eval_env`
builds a validator `Env` with the pattern bindings registered (for width
typing); `src/codegen_test.rs::init_var_values` evaluates initializers in
declaration order (they may reference earlier variables).

Code: `src/codegen_test.rs::build_test_plan`,
`src/codegen_test.rs::TestPlan`,
`src/codegen_test.rs::expect_progress_from_initial`,
`src/codegen_test.rs::has_definite_unexpected`,
`src/codegen_test.rs::plan_transitions`,
`src/codegen_test.rs::not_taken_observable`,
`src/codegen_test.rs::select_invariants`,
`src/codegen_test.rs::plan_monitor_violations`,
`src/codegen_test.rs::Lcg`, `src/codegen_test.rs::RANDOM_TRACE_SEED`,
`src/codegen_test.rs::MAX_STATIC_ASSIGNMENTS`

<a id="dev:codegen-mc-concepts"></a>
## Model-checker concepts (`src/codegen_mc.rs`)

The `mc` command generates a bounded model checker: an executable that drives
the generated FSM and reports property violations, unexpected events, and
assumption violations as counterexample traces. This module defines the
language-independent concepts; backends render them into executable target-
language code. The generated binary must NOT depend on this crate, so each
backend embeds its own copies of the plan tables and parsing helpers - the
formats defined here are the source of truth, and the embedded copies must be
kept in sync when they change.

**Legacy fuzz mode (`--mode fuzz`).** `src/codegen_mc.rs::McConfig` (bound 5,
runs 20, seed 12345, assumption selection, violation specs, unexpected-event
probability) + `src/codegen_mc.rs::parse_assume_selection`; forced-violation
directives in the wire format `label:prob:percent%`
(`src/codegen_mc.rs::ViolationSpec::parse`, missing fields default to prob 1.0
/ percent 100.0); the ordered per-step sequence-generation strategy
`src/codegen_mc.rs::McPhase::ALL` - forced assumption violation, then
unexpected injection (with probability `unexpected_prob`), then allowed random
input; and the per-run outcome taxonomy `src/codegen_mc.rs::McStatus`
(Completed / Stuck / UnexpectedEvent / AssumptionViolated).

**Exact mode (default).** The three-status taxonomy
`src/codegen_mc.rs::ExactStatus` - PassComplete / Fail / UnknownStateLimit -
carries its wire strings (`pass-complete` / `fail` /
`unknown-state-limit`), human text (`PASS (complete)` / `FAIL` / `UNKNOWN
(state limit reached)`), and process exit codes 0/1/2. `UnknownStateLimit`
is never a completeness claim. `src/codegen_mc.rs::FailReason` distinguishes
PropertyViolation / LivenessViolation / Deadlock (wire names
`property-violation` / `liveness-violation` / `deadlock`).
`src/codegen_mc.rs::exact_summary_line` renders the PINNED machine-readable
report line - `model-check summary: status=<s> states=N transitions=M
max_depth=D`, plus on fail ` reason=<r> label=<L> step=S` (deadlock uses
`label=-` and the depth of the dead configuration as `step`) - unit-test-
pinned and parity-checked across backends.

**Codegen-time defaults** (embedded as runtime-overridable defaults in the
generated binary): `src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES` = 1,000,000;
`src/codegen_mc.rs::EXACT_DEFAULT_TIMEOUT_SEC` = 0 (off; wall-clock ONLY - NOT
parity-guaranteed, never used by parity or benchmark assertions);
`src/codegen_mc.rs::EXACT_DEFAULT_THREADS` = 1 (N > 1 keeps results byte-
identical to 1 via canonical commit; `--fast` is runtime-only and non-parity);
`src/codegen_mc.rs::SMT_DEFAULT_BOUND` = 10 (small by design - SMT mode is bug
hunting on larger bitvectors, not a completeness tool).

**Counterexample report format.** `src/codegen_mc.rs::CounterexampleFormat`
(Text default - the parity-pinned human-readable report; Json - one compact
machine-readable document); invalid values are rejected with the pinned
`src/codegen_mc.rs::COUNTEREXAMPLE_FORMAT_ERROR`. Exact mode only - fuzz and
smt-bounded modes keep their own output.

**The JSON report model.** `src/codegen_mc.rs::McJsonReport` (status, metrics,
reason/label/step on fail, statistics, counterexample) +
`src/codegen_mc.rs::JsonCounterexample` (`trace` / `lasso` with prefix + cycle
+ cycle_length / `deadlock-config`) + `src/codegen_mc.rs::JsonStep` /
`src/codegen_mc.rs::JsonIo`. KEY ORDERING IS PART OF THE BYTE-PARITY CONTRACT:
top level `status, metrics, [reason, label, step], statistics,
[counterexample]`; `metrics` = `states, transitions, max_depth`;
`statistics` = `unexpected_terminations, monitor_size_bits,
memory_estimate_bytes, peak_rss_kb`; step objects = `input, outputs,
from_state, to_state, vars`; input/output objects = `variant, fields`
declaration order. The counterexample object carries `ce_steps` LAST on every
FAIL document - the trace-length quality metric (steps for a trace, prefix +
cycle for a lasso, depth for a deadlock). Strings use the json_rw escaping
contract; integers are unsigned decimal. Each backend embeds a minimal copy of
the shared writer so generated binaries stay dependency-free, and parity
extends to byte-identical JSON across backends EXCEPT `peak_rss_kb` -
informational, machine-dependent (self-reported by the binary); the parity
suites normalize it to 0 before byte comparison, and `rfs mc-ref` reports 0
since it runs in-process. The key names live in the `JSON_KEY_*` constants
next to the contract documentation.

**Where exact mode actually runs.** This module holds the CONCEPTS; the shared
exact-mode plan (input abstraction, state encoding, canonical traversal,
liveness decomposition - D-P4-1) is computed once in `src/mc_plan.rs` and
embedded as constants/tables by both backends (deep dive: task 11.1); the SMT
path (`smt_plan.rs` / `smt_emit.rs`) is task 11.2.

Code: `src/codegen_mc.rs::ExactStatus`, `src/codegen_mc.rs::FailReason`,
`src/codegen_mc.rs::exact_summary_line`, `src/codegen_mc.rs::McConfig`,
`src/codegen_mc.rs::ViolationSpec`, `src/codegen_mc.rs::McPhase`,
`src/codegen_mc.rs::CounterexampleFormat`,
`src/codegen_mc.rs::McJsonReport`, `src/codegen_mc.rs::JsonCounterexample`,
`src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES`

## Changing the codegen core

Checklist for developers changing the core (the backends are tasks 9.2/10.x;
this area is where they share behavior):

1. **Placement rule** (CODEGEN.md section 2): target-language syntax goes in
   the backend module; pure IR analysis any backend would reuse goes in the
   matching `codegen_*.rs` core module; CLI parsing/help/registry mechanics go
   in `src/codegen.rs`. A new backend touches ONLY the registry table.
2. **Parity principle**: a new feature lands in EVERY backend in the same
   change and is covered by cross-backend differential tests
   (`tests/backend_parity_test.rs` asserts field-identical MC runs and
   per-test agreement of the generated test executables over all examples;
   `tests/liveness_mc_test.rs` pins lasso counterexamples; the benchmark suite
   pins deterministic metrics + `--threads` byte-identity).
3. **Pinned formats break on drift** - update them together with their tests:
   the exact summary line, the JSON report key ordering (incl. `ce_steps`
   last), the LCG constants + seed, the stable test-name scheme and the
   random-trace coverage line, the `ViolationSpec` wire format, and the two
   pinned option-rejection strings.
4. **Planner semantics must mirror the generated `fsm_step`**: firing order is
   descending effective priority then source order; the static evaluator's
   width semantics (u128 wrapping + mask, shift saturation, bool bitwise =
   logical) must stay in lockstep with the backend lowering, or planned cases
   will disagree with runtime behavior.
5. **Embedded copies**: generated binaries embed their own copies of the MC
   parsing helpers and JSON writer - when a format here changes, update the
   embedded-copy emitters in both backends in the same change.
6. **Verify**: `cargo test --test codegen_test` (registry, generation over all
   valid examples, option sensitivity, CLI parsing),
   `cargo test --test backend_parity_test`, then the full BUILD.md section 5
   checklist.
7. **Docs sync**: CODEGEN.md (the extension guide), KB.md; training snippets
   if the change is user-visible.
