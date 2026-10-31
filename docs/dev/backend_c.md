# RFS C Backend (Class A)

The C target-language backend: plugin registration plus the three generators
that render the shared codegen plans into self-contained C source.

<a id="dev:backend-c-overview"></a>
## Overview

The C backend is one of the two target-language backends in
`src/backends/` (the other is Rust, deep dive docs/dev/backend_rust.md). It
renders the language-agnostic codegen plans into self-contained C source -
one translation unit per command. The governing constraint is PARITY: for the
same spec and options, the Rust and C artifacts must agree in every pinned
format and in observable behavior; cross-backend differential tests enforce
this (tests/backend_parity_test.rs, tests/prod_parity_test.rs,
tests/wide_ops_parity_test.rs).

| File | Lines | Role |
|---|---|---|
| `src/backends/mod.rs` | 16 | module declarations only (c*, rust*); no code |
| `src/backends/c.rs` | 177 (4 unit tests) | plugin entry `C_BACKEND` + shared C-specific helpers (identifier mangling, type mapping, literals and masks) |
| `src/backends/c_fsm.rs` | 2160 (no unit tests) | production FSM source generation; the core generator reused by test and mc; shared expression/signal renderers |
| `src/backends/c_mc.rs` | 2836 (no unit tests) | model-checker executable generation: exact engine + liveness + JSON report + fuzz runtime + SMT binary (deep dive task 10.2) |
| `src/backends/c_test.rs` | 1035 (no unit tests) | plain-assert unit-test harness + the separate fuzz-target skeleton (deep dive task 10.2) |

Position in the pipeline: `src/codegen.rs::CodegenCommandKind::run` dispatches
to the backend's function pointers - `src/backends/c.rs::C_BACKEND` wires
`production` to `src/backends/c_fsm.rs::generate_production`, `test` to
`src/backends/c_test.rs::generate_unit_tests`, and `mc` to
`src/backends/c_mc.rs::generate_model_checker`. All three entry points funnel
into `src/backends/c_fsm.rs::generate_core` (the shared core emitter); each
command then appends its own runtime. The plugin API, the options bag, the
dispatch rules, and the shared planning data these generators render are
documented in docs/dev/codegen_core.md (`dev:codegen-plugin-api`,
`dev:codegen-test-plan`, `dev:codegen-mc-concepts`) - this doc covers only
what is C-specific.

Key differences from the Rust backend (everything else is parity):

- bit<N> widths outside {8, 16, 32, 64} are a codegen error (decision D8) -
  the Rust backend accepts any width up to 128 via u128 intermediates;
- inputs/outputs are tagged unions (`kind` enum + payload union), not enums;
- `--no-std` is accepted as a documented no-op (plain C is already
  dependency-free) - one comment line in the header, nothing else;
- `--with-protocol-fsm`: ignored by production/test (the ProtocolFsm surface
  is Rust-ecosystem-only, D-P5-11), rejected by mc with the pinned error
  `src/codegen.rs::PROTOCOL_FSM_MC_ERROR`;
- no aggregate test crate: each module's tests are a standalone executable.

Where to go next:

- src/backends/C.md - architecture + design decisions of the generated
  artifacts (anatomy, type mapping, value representation, known limitations);
- CODEGEN.md (repo root) - the extension guide: plugin API, CLI surface,
  adding-a-backend checklist, invariants;
- function-level call graph: `dev:deps-fn-backend-c` in DEPENDENCIES.md;
- file entries: `dev:code-map-codegen` in CODE_MAP.md;
- decision rationale: D8 (type mapping), D-P3-4/D-P3-5/D-P3-6 (monitors),
  D-P4-* (MC engine), D-P5-10 (JSON report), D-P5-11 (ProtocolFsm), D-P6-3
  (artifact header) in DECISIONS.md.

Code: `src/backends/c.rs::C_BACKEND`,
`src/backends/c_fsm.rs::generate_production`,
`src/backends/c_test.rs::generate_unit_tests`,
`src/backends/c_mc.rs::generate_model_checker`,
`src/backends/c_fsm.rs::generate_core`,
`src/codegen.rs::CodegenCommandKind::run`,
`src/codegen.rs::PROTOCOL_FSM_MC_ERROR`

<a id="dev:backend-c-plugin"></a>
## Plugin entry + shared C helpers (`src/backends/c.rs`)

**Registration.** `src/backends/c.rs::C_BACKEND` is the static `Backend` value
appended to the registry in `src/codegen.rs`: name "c", one-line description,
three command descriptors (PRODUCTION_INFO, TEST_INFO, MC_INFO - each carrying
the CLI usage line and per-option help that backend_help renders verbatim),
and the three function pointers into the generator modules. The descriptor
text is user-visible: TEST_INFO documents `--out` as REQUIRED (the fuzz
skeleton is written next to it; stdout mode is refused), and MC_INFO documents
both the codegen-time flags and the generated binary's runtime flags
(including `--fast`, which is non-parity, and the wall-clock-only timeout
caveat).

**Identifier mangling.** Generated code must compile even when spec names
collide with C syntax:

- C_KEYWORDS - the 38 reserved names (the 35 C keywords + bool/true/false
  from stdbool.h + main);
- c_ident - keyword to `c_` prefix, everything else unchanged (RFS
  identifiers are already snake_case-friendly); applied to every emitted
  identifier (states, variants, fields, vars, enum paths);
- c_binding_ident(name, reserved, used) - for PATTERN BINDINGS only: in
  addition to keyword mangling, a binding that collides with an identifier the
  generated transition body uses unqualified is renamed to `<name>__rfsb`
  (further suffixed until unique among `used`). Without this, e.g. a field
  named `n` would shadow mc_step_all's effect counter `int n`: effects were
  written to `out[payload value]` (an out-of-bounds stack write for wide
  fields) and the counter never incremented, so the C model checker silently
  missed every successor through such a transition - a backend-parity
  violation reachable from fully valid specs (Phase 6 task 6 finding; the Rust
  backend has the same bug class via its effect vector `out`). The caller
  passes its own reserved set: {f, in, handler, user} in
  `src/backends/c_fsm.rs::transition_block_c`, {state, vars, out, n} in
  `src/backends/c_fsm.rs::render_mc_step_all_body`.

**Type + value mapping.**

- bit_c_type - bit<N> width to uint8_t (0..=8) / uint16_t (16) / uint32_t
  (32) / uint64_t (64); any other width is a codegen error (decision D8) -
  the Rust backend instead falls back to u128;
- c_type - Bool to bool, Bit{w} to bit_c_type(w), Enum{name} to `<Name>_t`
  (the typedef emitted by the FSM generator);
- int_c_literal(value, width) - keeps an integer literal in-range for the
  target type: width < 64 renders `({value & mask}u)` (e.g. 300 at bit<8> to
  `(44u)`), else `({value}ULL)`;
- mask_c_expr(width) - the width mask: `0xFFFFFFFFFFFFFFFFULL` for w >= 64,
  else `((1ULL << w) - 1)`; wraps every arithmetic/bitwise result so generated
  code has the DSL's wraparound semantics.

Note: there is no C-side snake helper in c.rs - monitor label naming reuses
`src/backends/rust.rs::snake` (from c_fsm.rs and c_test.rs) so `mon_<label>`
names match across backends; representative-input rendering lives per
generator in c_test.rs / c_mc.rs (task 10.2).

Unit tests (4): identifier_mangling, type_mapping,
backend_registration_shape, literals_and_masks.

Code: `src/backends/c.rs::C_BACKEND`,
`src/backends/c.rs::PRODUCTION_INFO`, `src/backends/c.rs::TEST_INFO`,
`src/backends/c.rs::MC_INFO`, `src/backends/c.rs::C_KEYWORDS`,
`src/backends/c.rs::c_ident`, `src/backends/c.rs::c_binding_ident`,
`src/backends/c.rs::bit_c_type`, `src/backends/c.rs::c_type`,
`src/backends/c.rs::int_c_literal`, `src/backends/c.rs::mask_c_expr`,
`src/backends/rust.rs::snake`

<a id="dev:backend-c-fsm"></a>
## Production FSM generation (`src/backends/c_fsm.rs`)

**Entry points.** generate_production maps `--with-property-monitors` to
Gated (else Off); generate_core is the shared core emitter - `test` and `mc`
always pass Always. It returns a CodegenOutput with empty extra_files; each
command appends its own extras.

**Gen pipeline.** Gen::new rejects unsupported bit widths UP FRONT (collecting
every bad width into one error, D8), builds the flattened variants via
`src/codegen_fsm.rs::build_variants`, compiles the monitors, then emits in a
fixed order: emit_header -> emit_types -> emit_monitors -> emit_fsm_impl.
has_prop/prop_wrap/prop_emit implement the property-monitor gating (D-P3-6):
Gated wraps chunks in `#ifdef RFS_PROPERTY_MONITORS` with the directive lines
standing alone (call sites may be mid-line), Off omits them, Always emits
them bare.

**Header.** emit_header writes line 1 = the v1.0 unified artifact header
(D-P6-3) - identical text to the Rust backend (`//` is a valid C comment) -
then the five standard includes (stdint.h, stdbool.h, string.h, stdio.h,
stdlib.h; no project or third-party dependencies). `--no-std` adds one
comment line documenting that it is a no-op on this backend. A module version
becomes `#define RFS_SPEC_VERSION "..."`; Gated mode adds `#define
RFS_PROPERTY_MONITORS 1`.

**Types.** emit_types emits, in order: non-State enums first (vars/fields may
reference them) as `<NAME>_<VARIANT>` constants + `<name>_t` typedefs; the
State enum as STATE_* / state_t (`typedef int state_t` when the module
declares no State enum); the vars_t struct (an empty module gets `{ int
_unused; }`); the input/output tagged unions via emit_tagged - one payload
struct per struct variant `<kw>_<variant>_payload_t`, a kind enum
`<KW>_<VARIANT>`, and a wrapper with `int kind` + union payload (an empty
group degenerates to a `_NONE` kind + `int _none`); `#define MAX_OUTPUTS n`
where n is the maximum emit count over all transitions and the on_error block
(minimum 1); trace_step_t / rfs_trace_t with a dynamic realloc buffer (cap 0
-> 8 -> doubling); rfs_status_t (RFS_OK / RFS_UNEXPECTED_EVENT /
RFS_ASSUMPTION_VIOLATION / RFS_PROPERTY_VIOLATION); step_result_t - a fixed
outputs[MAX_OUTPUTS] array + n_outputs + assume_label[64] + prop_label[64] +
prop_step + prop_trigger_step, zeroed by rfs_make_result; rfs_policy_t
(Continue/Stop/Panic) + the rfs_assumption_handler function pointer (the C
analogue of the Rust trait) + rfs_panic_handler (fprintf + abort); and the
trace helpers trace_state/trace_vars (position 0 is the initial configuration,
else steps[pos-1]) + trace_push.

**Expression lowering.** gen_expr_c types every subexpression via
`src/validator.rs::type_expr` and renders:

- Int to int_c_literal at the expression's width; Bool to true/false;
- Path single-segment to the Ctx name map (vars, bindings, state) or a STATE_
  variant constant; multi-segment to the `ENUM_VARIANT` constant;
- BitNot/Neg cast to the target type after `~` / `0ULL - x`;
- Plus/Minus as-is, Star in a uint64_t-wide intermediate (Phase 6 task 2a - a
  bit<16> product above INT_MAX must not be signed overflow UB); every bit<N>
  arithmetic/bitwise result is then masked to the type width;
- comparisons render bare `({l} {sym} {r})` relying on C integer promotion
  (both sides are non-negative within their widths, so the result matches the
  Rust backend's u128-cast comparison); LogicAnd/Or to &&/||;
- ShiftLeft/Right widen BOTH operands to uint64_t with a runtime guard
  `amount >= w -> 0` (Phase 6 task 2a - the old promotion-based rendering was
  UNDEFINED when a variable amount reached the promoted LHS width, e.g. gcc
  wraps the count mod 32); one uniform form for literal and variable amounts,
  matching the Rust backend's saturation.

The width semantics must stay in lockstep with the Rust backend AND the
planner's static evaluator - drift makes planned test cases disagree with
runtime behavior; cross-backend differential pins live in
tests/wide_ops_parity_test.rs. c_contains_ident is the whole-token identifier
containment check that drives the `(void)` casts for unused bindings.

**Online monitors.** compile_monitors compiles every property and every
selected assumption via `src/monitor.rs::compile_formula`; formulas outside
the monitor subset are skipped with a warning whose text is IDENTICAL in
every backend (D-P3-5). emit_monitors emits, in order: rfs_sample_t - the C
analogue of Rust's Sample (`input` is NULL at sample 0, so `input_is` is false
there); per-label assumption monitor structs in declaration order; the
property machinery gated TOGETHER (rfs_property_violation_t + per-label
structs + the prop_monitors aggregate) so a build without the debug switch
carries nothing (D-P3-6); the assume_monitors aggregate - always present, with
an empty stub when no assumptions are active so the fsm_t shape and call sites
stay uniform; and the PROPERTY_LABELS/ASSUME_LABELS/N_* label constants (kept
for MC/test runtimes and user code, D-P3-4) under an RFS_UNUSED attribute so a
production TU that never references them stays -Wall-clean.

render_monitor renders one monitor struct from a compiled plan: nodes are
numbered in DFS pre-order over the plan tree, the step body evaluates them in
post-order (children before parents), mirroring the reference interpreter in
`src/monitor.rs`. Each distinct InputIs/OutputHas pattern gets one helper
function (`mon_<m>_in_<i>` / `mon_<m>_out_<i>`, collected by
collect_signal_patterns in first-seen DFS order) plus signal expressions
(`mon_<m>_sig_<i>`). mon_<m>_step returns `m->n0_failed && !before` - the
failure newly confirmed at this sample; mon_<m>_trigger_step reads
n0_trigger_step.

render_node per node kind (field sets + step semantics, identical to the Rust
rendering):

- RangeCheck: n<id>_failed / n<id>_trigger_step / n<id>_now - `now` is
  recomputed every sample (the child may recover), `failed` is sticky for
  reporting; trigger_step stays -1 (a plain per-sample check);
- Obligation: bound 0 = be[0] - the body must hold at the trigger sample
  itself; bound >= 1 = the D-P3-3 pending model (n<id>_pending + a `long long`
  n<id>_remaining countdown, failure terminal);
- DelayedCheck: an n<id>_prev_antecedent register - the antecedent held at the
  PREVIOUS sample so the consequent must hold now; failure is terminal and
  there is no trigger sample to report (stays -1);
- And/Or: per-sample composition over the children's CURRENT failure status -
  a conjunction fails while ANY child fails, a disjunction only while ALL
  fail; trigger_step = min of the children's.

range_cond_c renders an AnchorRange as an idx condition ("true" when unbounded
from 0).

**Shared signal machinery.** The free functions render_signal_expr_c,
input_is_sample_helper_c, output_has_sample_helper_c, input_match_cond_c and
output_match_cond_c are prefix-parameterized so the monitor renderer (prefix
`mon_<m>`) stays byte-identical while the same machinery serves the generated
invariant tests (c_test.rs) and the liveness sample evaluator (c_mc.rs) -
Phase 5 task 6. input_match_cond_c/output_match_cond_c build a kind check plus
literal field guards (Int via int_c_literal, Bool true/false, EnumVariant to
the `ENUM_VARIANT` constant); wildcards/bindings add no constraint; they
return None when the pattern is unresolvable (unit pattern on a struct
variant, field-count mismatch, empty group).

**Fsm impl.** emit_fsm_impl emits:

- fsm_t - state, vars, trace, the active_assumes array + n_active_assumes,
  assume_monitors, and (gated) prop_monitors + prop_label[64] + prop_step +
  prop_trigger_step;
- initial vars as a designated initializer `(vars_t){ .a = ..., .b = ... }` -
  var references map to `vars.<name>` and a render failure falls back to
  `.name = 0`;
- fsm_new - sets the initial state, records the initial trace configuration,
  callocs the active_assumes array from ASSUME_LABELS, then steps sample 0 so
  countdowns and one-shot obligations start correctly: an assumption violated
  at sample 0 aborts (as with the previous post-hoc check), a property
  violation is recorded sticky;
- fsm_free (Phase 5 task 16 close-out fix) - frees the two per-fsm heap
  allocations (trace buffer + active_assumes array) so a long-lived consumer,
  notably the libFuzzer target which calls it once per input, does not
  accumulate RSS until libFuzzer's out-of-memory limit trips; short-lived
  consumers (unit-test and MC executables) may simply exit without calling it;
- rfs_commit (static RFS_UNUSED - a transition-less module must not warn) -
  pushes the trace step, takes idx = trace.len after the push, builds the
  sample, steps the assumption monitors and applies the handler policy (Stop
  to an RFS_ASSUMPTION_VIOLATION result carrying the label, Panic to abort),
  then checks the property monitors first-violation-wins (the prop_label[0] ==
  0 guard) to an RFS_PROPERTY_VIOLATION result, and only then updates
  state/vars;
- fsm_step - a NULL handler defaults to rfs_panic_handler; transitions are
  grouped by state in first-appearance order and, within a state, sorted by
  DESCENDING effective priority (stable sort_by_key Reverse(priority.unwrap_or
  (0))) - mirroring the planner's firing order; the on_error fallback is the
  final catch-all (no state check, no bindings, vars carried over) or, when
  the module declares none, RFS_UNEXPECTED_EVENT.

**Transition blocks.** transition_block_c renders one transition as nested
if-blocks: pattern match (group arms joined by ||, a unit kind check, or a
struct kind + literal guards) -> bindings as locals in declared field order
via c_binding_ident with reserved set {f, in, handler, user} -> guard ->
actions. A binding that no rendered guard/output/assignment references gets a
`(void)` cast (c_contains_ident over the rendered text). Actions: the outputs
into a local array, then SIMULTANEOUS assignment lowering - `vars_t tmp_vars
= f->vars;`, every assignment RHS evaluated into `tmp_<var>` from pre-step
values only, then applied to tmp_vars and committed. A unit pattern on a
struct variant is an error.

**Outputs.** output_expr_c renders an output as a designated initializer: a
bare output resolves to the group's single variant (else "bare output
ambiguous"), the field count is checked against the args, and the result is
`(output_t){ .kind = OUTPUT_*, .payload.output_<v>.<f> = ... }`.

**Shared MC renderers.** render_mc_step_all_c builds a Gen in Off mode and
renders mc_step_all - for (state, input), the effects of EVERY enabled
transition in firing order (deterministic mode uses only the first effect;
nondet explores all); the on_error fallback is appended when no transition
matched, with transition_id = -1. It is rendered from the same IR with the
same helpers as fsm_step, so both paths agree by construction.
render_liv_sample_evaluator_c renders a per-sample boolean evaluator for a
temporal-operator-free formula (Phase 4 task 7 liveness leaves; C mirror of
rust_fsm's render_sample_formula_evaluator) via `src/monitor.rs::formula_to_
signal`: unique pattern helpers plus `static int {name}(const rfs_sample_t*)`
returning 0/1.

Code: `src/backends/c_fsm.rs::generate_production`,
`src/backends/c_fsm.rs::generate_core`, `src/backends/c_fsm.rs::Gen`,
`src/backends/c_fsm.rs::emit_header`, `src/backends/c_fsm.rs::emit_types`,
`src/backends/c_fsm.rs::emit_tagged`, `src/backends/c_fsm.rs::gen_expr_c`,
`src/backends/c_fsm.rs::c_contains_ident`,
`src/backends/c_fsm.rs::compile_monitors`,
`src/backends/c_fsm.rs::emit_monitors`,
`src/backends/c_fsm.rs::render_monitor`,
`src/backends/c_fsm.rs::render_node`,
`src/backends/c_fsm.rs::range_cond_c`,
`src/backends/c_fsm.rs::render_signal_expr_c`,
`src/backends/c_fsm.rs::input_is_sample_helper_c`,
`src/backends/c_fsm.rs::output_has_sample_helper_c`,
`src/backends/c_fsm.rs::input_match_cond_c`,
`src/backends/c_fsm.rs::output_match_cond_c`,
`src/backends/c_fsm.rs::render_assume_monitors`,
`src/backends/c_fsm.rs::render_prop_monitors`,
`src/backends/c_fsm.rs::emit_fsm_impl`,
`src/backends/c_fsm.rs::transition_block_c`,
`src/backends/c_fsm.rs::output_expr_c`,
`src/backends/c_fsm.rs::collect_signal_patterns`,
`src/backends/c_fsm.rs::render_mc_step_all_c`,
`src/backends/c_fsm.rs::collect_liv_signal_patterns`,
`src/backends/c_fsm.rs::render_liv_sample_evaluator_c`,
`src/backends/c_fsm.rs::mc_pattern_cond`,
`src/backends/c_fsm.rs::render_mc_step_all_body`,
`src/codegen_fsm.rs::build_variants`, `src/monitor.rs::compile_formula`,
`src/monitor.rs::formula_to_signal`, `src/validator.rs::type_expr`,
`tests/wide_ops_parity_test.rs`

<a id="dev:backend-c-mc"></a>
## Model-checker executable generation (`src/backends/c_mc.rs`)

TBD (task 10.2): entry shapes (exact/fuzz core + standalone SMT binary), the
embedded plan tables, EXACT_ENGINE_C / LIV_ENGINE_C / JSON_WRITER_C /
EXACT_RUN_C / MAIN_DISPATCH_C / SMT_ENGINE_C, and the parity notes vs the Rust
checker.

<a id="dev:backend-c-test"></a>
## Unit-test + fuzz-target generation (`src/backends/c_test.rs`)

TBD (task 10.2): the five stable test classes rendered as a table-driven
main(), literal rendering + partial-assert fallback, and the separate
fuzz_target.c single-exit decode skeleton.

## Changing the C backend

Developer checklist (the placement rule is CODEGEN.md section 2 - core-level
analysis goes to the language-agnostic modules, target syntax stays here):

1. Parity first: a new feature lands in EVERY backend in the same change,
   with cross-backend differential tests (tests/backend_parity_test.rs;
   production random-input parity: tests/prod_parity_test.rs; wide-op
   lowering: tests/wide_ops_parity_test.rs; liveness:
   tests/liveness_mc_test.rs; performance: tests/benchmark_mc_test.rs). The
   Rust twin of each file is src/backends/rust*.rs (deep dive:
   docs/dev/backend_rust.md).
2. Pinned formats that break on drift - change them only with the parity
   suite updated in the same commit:
   - the line-1 artifact header (D-P6-3);
   - MAX_OUTPUTS and the step_result_t layout;
   - the mon_* struct shape + helper naming (mon_<m>_in_<i> / _out_<i> /
     _sig_<i>) - label naming must stay in lockstep with the Rust backend via
     `src/backends/rust.rs::snake`;
   - test names + the random-trace coverage line;
   - the fuzz decode wire format and the LCG constants/seed;
   - config_bytes layout (must match ref_mc's CfgKey byte-for-byte);
   - the exact summary line + statistics block;
   - JSON key ordering + the escape contract (ce_steps last; peak_rss_kb
     normalized to 0 in comparisons).
3. Firing order everywhere is descending effective priority then source
   order - it must mirror generated fsm_step AND the planner's static
   analysis (`dev:codegen-test-plan`). The pattern-scope reserved sets ({f,
   in, handler, user} in transition_block_c; {state, vars, out, n} in
   render_mc_step_all_body) exist because of this same shadowing class of bug.
4. gen_expr_c width semantics (uint64 intermediates for *, mask to type
   width, shift saturation at width via the runtime guard) must stay in
   lockstep with the Rust backend and the planner's static evaluator - drift
   makes planned test cases disagree with runtime behavior.
5. D8: bit widths outside {8, 16, 32, 64} stay a codegen error; the up-front
   rejection in Gen::new collects every bad width into one error.
6. Generated artifacts stay dependency-free (the five standard includes only)
   and deterministic: declaration order everywhere; gcc -Wall at ZERO warnings
   on every artifact over all valid examples (the RFS_UNUSED attribute and the
   conditional cont_handler emission exist for this bar); the fuzz skeleton
   stays a separate file.
7. no_std / protocol-fsm no-ops: `--no-std` may only add its one comment line;
   `--with-protocol-fsm` must stay ignored by production/test (and rejected by
   mc with the pinned error).
8. Verify: cargo test --test codegen_test, cargo test --test
   backend_parity_test, cargo test --test prod_parity_test, cargo test --test
   wide_ops_parity_test, then the full BUILD.md section 5 checklist. Docs
   sync: src/backends/C.md (artifact design), CODEGEN.md (if the plugin
   surface changes), KB.md; training snippets if user-visible.
