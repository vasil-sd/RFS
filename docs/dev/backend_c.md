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

**Entry point.** generate_model_checker rejects no_std and
--with-protocol-fsm with the pinned constants (`src/codegen.rs::NO_STD_
PRODUCTION_ONLY_ERROR`, `src/codegen.rs::PROTOCOL_FSM_MC_ERROR`). Three shapes:

- SmtBounded mode: a standalone libz3 binary - v1.0 header + the shared spec
  section rendered by `src/smt_emit.rs::emit_c_spec` from
  `src/smt_plan.rs::build_smt_spec` over `src/mc_plan.rs::McPlan` +
  SMT_ENGINE_C with {BOUND} <- mc_smt_bound; no FSM core, no exact/fuzz
  runtimes. Both backends build identical constraints from the shared plan
  (parity by construction, D-P4-12);
- exact / fuzz mode: the FSM core via `src/backends/c_fsm.rs::generate_core`
  (Always - D-P3-6) + shared_printers + exact_codegen_c + fuzz_runtime +
  MAIN_DISPATCH_C. Warnings = production warnings + liveness decomposition
  diagnostics (D-P4-9).

**Shared printers.** shared_printers generates C functions that reproduce
byte-for-byte the Rust MC binary's Debug-based counterexample format (D-P4-4):
per-enum name lookup functions ONLY for enums actually referenced by vars/
input/output fields (an unused static would trip gcc -Wall); state_name_c
returns ref_mc's prefixed form "State::Variant" (Rust Debug variant names),
shared by the text trace and the JSON report; print_vars_debug renders `Vars {
 f: v, ... }` in declaration order with no trailing newline; print_input_debug
renders `Input::Variant { ... }`; print_outputs_debug renders `[Output::X,
...]`; print_trace emits the pinned "counterexample trace:" format. All names
go through `src/backends/rust.rs::rust_ident` - the same name source as the
JSON renderers, so text and JSON carry identical data.

**Plan tables (exact_codegen_c).** Codegen-time defaults embedded as runtime-
overridable constants (D-P4-7): DEFAULT_MODE / MAX_STATES / NONDET /
TIMEOUT_SEC / THREADS / COUNTEREXAMPLE_FORMAT + CONFIG_TOTAL_BYTES /
MONITORS_BITS / STATE_BITS from plan.encoding; N_TOTAL_INPUTS = product of the
concrete field domain sizes; MC_MAX_EFFECTS = max transitions per state +
on_error. all_inputs() is the canonical enumeration (D-P4-2): variants in
declaration order, concrete fields nested in declaration order over their
domains (bool 0/1, int 0..=max, enum variant index via _dom_k tables),
abstracted fields at representatives (`src/codegen_fsm.rs::RepValue` -
EnumEmpty/EnumUnknown have no C rendering and are an error). put_bits +
config_bytes: state index bits, var slots (bool 1/0, bit as u64, enum variant
index), monitor node bits with remaining packed ONLY while pending = ref_mc's
CfgKey byte-for-byte. Then render_mc_step_all_c, EXACT_ENGINE_C,
JSON_WRITER_C + json_renderers_c, LIV_ENGINE_C + the per-property leaf
evaluators (liv_p<i>_l<j>[b] via
`src/backends/c_fsm.rs::render_liv_sample_evaluator_c`; Safety kind 0 /
Eventually 1 / Until 2 with eval_b) + flatten_skeleton_c first-appearance re-
indexing to liv_p<i>_skel + the LIV_PROPS table (a dummy entry when there are
no liveness properties), EXACT_RUN_C.

**EXACT_ENGINE_C.** mc_node_t {state, vars, prop, assume}; mc_step_successor at depth+1 - 0 success / 1 pruned by an active assumption / 2 property violation. Two-phase parallel expansion (task 10B): phase A workers over disjoint config ranges compute per-(input, effect) outcomes into per-config lists (pure with respect to the shared engine state); phase B commits them SEQUENTIALLY in canonical order - visited insert at enqueue, parent/depth/adj pushes, first violation, deadlock check - so results are a pure function of the spec: byte-identical to --threads 1 for any N. The visited set is a sorted vector of config byte strings + binary search with a parallel node-index array (needed by the liveness graph edge recording) - behavior must not depend on this container (D-P4-2). mc_fill_path walks the parent pointers; mc_replay rebuilds the Phase 3 trace format on a fresh Fsm (nondet picks the effect with the recorded transition_id else the first; -1 = on_error); mc_report_property emits the pinned property report.

**LIV_ENGINE_C.** Mirror of the Rust LIV_ENGINE: per-leaf progress automata (safety 0=ok/1=bad, eventually 0=wait/1=done, until 0=ok/1=sat/2=bad; all one-way, so every state sequence on an infinite path is eventually constant = limit vector); EDGE-based labels - position 0 is the initial configuration sample, position k the sample of step k; liv_bits_at cached per edge position in a PER-PROPERTY cache (a shared cache would mislabel edges from the second property on - found and fixed in Phase 4 task 7); liv_delta mixed-radix transition; liv_good good sets. liv_analyze: product BFS from (start_q, config 0) over the explored graph; per-q stay-edge subgraph (edges whose label keeps the limit vector); nondet STRONGLY FAIR filter (D-P4-11) - each cycle state has exactly one explored successor and was fully expanded when truncated; iterative Kosaraju SCCs over ascending config ids (the SCC partition is independent of DFS start order, so this reproduces the Rust engine's results); cyclic = component size > 1 or a stay self-loop; smallest id = canonical anchor. When the limit vector falsifies the skeleton, liv_build_lasso reconstructs the witness: product path from (start_q, 0) to (q, cstar) + stay-edge cycle through cstar = lasso (prefix + one cycle iteration); liv_note writes the human-facing explanation of which leaves are falsified.

**JSON_WRITER_C.** Embedded port of the toolchain's compact writer (src/json_rw.rs): mjb_* buffer functions; escape contract - `"` `\` `b` `t` `n` `f` `r` escapes, other control chars as \u00XX lowercase hex, everything else raw UTF-8; integers are unsigned long long (every value in this document is non-negative and bit<64> fields reach 2^64-1). mc_peak_rss_kb self-reports via getrusage(RUSAGE_SELF) ru_maxrss - INFORMATIONAL, machine-dependent, normalized to 0 in cross-backend byte comparisons, 0 on failure. mc_json_doc_head key order is the byte-parity contract: status, metrics{states, transitions, max_depth}, [reason, label, step on fail], statistics{unexpected_terminations, monitor_size_bits, memory_estimate_bytes, peak_rss_kb}; counterexample kinds trace / lasso{prefix, cycle, cycle_length} / deadlock-config{depth, state, vars}; ce_steps is ALWAYS the last key. json_renderers_c emits the module-specific mc_json_input / mc_json_output / mc_json_vars - names via rust_ident (the same name source as the text printers), bool true/false, bit unsigned long long decimal, enum variant name via the shared lookup.

**EXACT_RUN_C.** run_exact() -> exit code; runtime flags over codegen-time defaults (D-P4-7): --max-states / --nondet / --timeout-sec / --threads / --fast / --counterexample-format / --assumes. Sample 0 via fsm_new - an assumption violated at sample 0 aborts, a property violation is reported immediately (JSON or text with the pinned summary line + dedicated sample-0 report). Level-synchronous BFS (D-P4-2): visited checked at ENQUEUE time, parent pointers, depth, adj edges recorded for every Success including to already-visited configs, complete flag. Status 0 running / 1 pass-complete / 2 state limit / 3 timeout / 4 fail; exit codes 0/1/2. Parallel level expansion (task 10B): phase A pthread workers (up to 64) over disjoint frontier chunks call the pure mc_expand_range, phase B is a SEQUENTIAL canonical commit - byte-identical to --threads 1 for any N; if pthread_create fails, that range runs inline (results stay canonical). --fast is a non-parity early abort: which configs were expanded before the abort is race-dependent (states/transitions/label may vary between runs), never a false pass. Unexpected = path terminates (D-P4-5) counted in statistics, transitions untouched; Pruned counts as a transition; deadlock = no successor at all -> FAIL reason=deadlock label=- step=depth. SEMANTICS 9.5 stop-at-first-violation/state-limit: the check happens AFTER the per-level loop, matching the Rust engine's status == 0 guard. Liveness runs only when BFS ended without a safety failure or deadlock; on a truncated graph this is a TRUNCATED analysis - a witness found inside the explored part is sound (all its edges are real transitions), otherwise the UNKNOWN status stands: never a false pass-complete. Report: JSON one compact document, or text PASS (complete) / FAIL / UNKNOWN (state limit reached[, timeout after Ns]) + pinned summary line + statistics block.

**MAIN_DISPATCH_C + fuzz_runtime.** main dispatches --mode fuzz -> run_fuzz else run_exact. The fuzz runtime keeps the Phase 3 wire format: LCG state = state * 6364136223846793005 + 1442695040888963407 wrapping, top-53-bit double mapping - SAME constants as `src/codegen_test.rs::Lcg` and the Rust MC; ViolationSpec wire format label:prob:percent% defaults 1.0/100.0 (the label is truncated at the first ':' to match the Rust parser); assume selection all/none/labels filtered against ASSUME_LABELS. run_fuzz flags --bound 5 / --runs 20 / --seed 12345 / --assumes / --violate / --unexpected-prob = `src/codegen_mc.rs::McConfig` defaults. Per step: phase 1 forced violation injection (trial-step ALL_INPUTS for the target label), phase 2 optional unexpected injection, phase 3 allowed representative inputs - property-violating inputs are allowed on purpose (the model checker wants to find violations). Property violations terminate immediately with exit 1, so the property_violations counter is always 0 (the field stays for a stable summary format); summary line "model-check summary: runs={} bound={} completed={} unexpected={} assumption_stops={} property_violations={}".

**SMT_ENGINE_C.** C mirror of SMT_ENGINE_RUST: DEFAULT_BOUND <- {BOUND}. Parity-critical: step variables are z3 constants with FIXED names in fixed order s_i / v_i_j / ev_i / fld_i_g, constraints asserted in fixed order, queries in fixed order safety-then-liveness declaration order; two-arg Z3_mk_and/Z3_mk_or over 2-element arrays (the C counterpart of the Rust b_and/b_or wrappers - identical constraint trees). Unrolling: enabled_t = state match and class match and guard (firing order); fire_t = enabled and no earlier transition enabled (first match, prefix-OR chain); alive_i = alive_{i-1} and any_fire; next state/vars via ITE chains. Safety query viol_p = exists i<k: alive_i and not phi_p(i) -> FAIL + pinned summary line exit 1. Liveness query never_p (phi holds at no alive step) and cycle (exists a<b: identical (state, vars)) -> FAIL reason=liveness-violation + cycle length exit 1. Unknown -> UNKNOWN (solver inconclusive within bound {k}) + status=unknown exit 2. All Unsat -> PASS WITHIN BOUND {k} (incomplete - no completeness claim) + status=pass-within-bound exit 0. smt_print_trace emits the pinned witness format. Targets the z3 5.3.1 API surface (Z3_L_UNDEF, no Z3_del_solver, Z3_model_eval bool + out-ast) - one solver build shared with the Rust backend.

**Embedded-copy rule.** Because the generated binary cannot depend on this crate, c_mc.rs embeds its own copies of the plan tables, parsing helpers, and JSON writer. The formats in `src/codegen_mc.rs` / `src/mc_plan.rs` are the SOURCE OF TRUTH - when a wire format changes, update both backends' emitters in the same change and extend the tests (CODEGEN.md section 9).

Code: `src/backends/c_mc.rs::generate_model_checker`,
`src/backends/c_mc.rs::shared_printers`,
`src/backends/c_mc.rs::print_value_c`,
`src/backends/c_mc.rs::domain_loop`,
`src/backends/c_mc.rs::rep_value_c`,
`src/backends/c_mc.rs::domain_size`,
`src/backends/c_mc.rs::rep_inputs`,
`src/backends/c_mc.rs::rep_input_c`,
`src/backends/c_mc.rs::exact_codegen_c`,
`src/backends/c_mc.rs::json_renderers_c`,
`src/backends/c_mc.rs::json_value_c`,
`src/backends/c_mc.rs::flatten_skeleton_c`,
`src/backends/c_mc.rs::c_string_literal`,
`src/backends/c_mc.rs::var_value_c`,
`src/backends/c_mc.rs::EXACT_ENGINE_C`,
`src/backends/c_mc.rs::LIV_ENGINE_C`,
`src/backends/c_mc.rs::JSON_WRITER_C`,
`src/backends/c_mc.rs::EXACT_RUN_C`,
`src/backends/c_mc.rs::MAIN_DISPATCH_C`,
`src/backends/c_mc.rs::fuzz_runtime`,
`src/backends/c_mc.rs::SMT_ENGINE_C`,
`src/codegen.rs::NO_STD_PRODUCTION_ONLY_ERROR`,
`src/codegen.rs::PROTOCOL_FSM_MC_ERROR`,
`src/mc_plan.rs::McPlan`, `src/mc_plan.rs::LivenessDecomposition`,
`src/smt_plan.rs::build_smt_spec`, `src/smt_emit.rs::emit_c_spec`,
`src/codegen_fsm.rs::RepValue`, `src/codegen_test.rs::Lcg`,
`src/codegen_mc.rs::McConfig`, `src/printer.rs::print_formula`,
`src/backends/rust.rs::rust_ident`, `src/json_rw.rs`

<a id="dev:backend-c-test"></a>
## Unit-test + fuzz-target generation (`src/backends/c_test.rs`)

**Entry point.** generate_unit_tests rejects no_std with the pinned `src/codegen.rs::NO_STD_PRODUCTION_ONLY_ERROR`; it renders the production core via `src/backends/c_fsm.rs::generate_core` with Always (D-P3-6 - a spec that violates its own property fails its generated tests) plus `src/codegen_test.rs::build_test_plan`(variants, module, steps), and appends the separate fuzz_target.c as an ExtraFile.

**Harness.** Plain-assert harness (no framework): rfs_test_failures counter + CHECK macro - failures are counted, not aborting, so one run reports every broken check; stop_handler / cont_handler (cont_handler is emitted only when at least one test uses it - modules without transitions/monitor drives must stay warning-free under -Wall); clone_fsm deep-copies the trace buffer for trial steps; ALL_INPUTS = one representative per variant (rep_input_c: bool false, zero integers, first declared enum variant) + N_ALL_INPUTS; EXPECT_PROGRESS from plan.expect_progress.

**Module-level tests.** initial_state_is_correct; positive_sequence_runs - greedy trial-clone drive up to K steps with stop_handler, CHECK(steps > 0 || !EXPECT_PROGRESS || N_ALL_INPUTS == 0); negative_unexpected_input_search (render_negative_c) in three shapes: has_on_error -> search for the on_error landing state (asserts found when definite_unexpected, else a comment), definite_unexpected without on_error -> asserts an RFS_UNEXPECTED_EVENT is found, else an empty test with a comment.

**Per-transition tests.** render_transition_test_c: name t<id>_<state>_on_<event> (+ _not_taken) - the same stable names as the Rust renderer. Taken asserts on the RECORDED trace step (last->to + outputs) - robust against property-violation halts. Not-taken is STRICT: render_signature_compare_c computes same_sig (state + full outputs) and CHECK(!same_sig); it refuses to emit a toothless test when the signature's outputs are not fully evaluable (error "not-taken signature has un-evaluable output args").

**Literal rendering.** var_value_literal_c (bool true/false, int decimal, enum ENUM_VARIANT constant); vars_initializer_c panics "planner must cover variable" - a contract violation; input_case_expr_c checks field names + counts; resolve_output_variant_c resolves a bare emit to the group's single variant (multi-variant groups are rejected by production codegen, so this cannot fail for a generated spec); render_outputs_assert_c renders a full assert or a per-output partial match - variant + statically known fields only, the documented planner fallback when an arg is not evaluable.

**Invariant tests.** render_invariant_test_c: name invariant_<label>; reuses the shared signal machinery with prefix rfs_inv_<label> (`src/backends/c_fsm.rs::collect_liv_signal_patterns` + input/output helpers + `src/backends/c_fsm.rs::render_signal_expr_c` -> rfs_inv_check_<label>(s)); drives the positive trace (same greedy policy as positive_sequence_runs) and re-evaluates the property's per-sample atom at every recorded sample, failing with the step index; bounded checks cover samples < N only.

**Monitor tests.** render_monitor_test_c: name monitor_<label>_fires_on_violation; case 1 - the initial configuration already violates, fsm_new records it at sample 0; case 2 - one planned drive step from the planned state/vars/input; asserts the sticky property-violation record holds the target label (the planner guarantees no other monitored property records first).

**Random-trace test.** render_random_trace_test_c: name random_trace_seed<seed>_steps<K>; precomputed LCG index table RT_IDX[] over ALL_INPUTS - same constants as the fuzz wire format, seed + indices fixed at plan time; stop_handler; prints ONE deterministic coverage line "random_trace seed={} steps={} ok={} unexpected={} assumption={} property={} states_visited={}" - the same line as Rust.

**main().** Table-driven: one function per test, one line per test (PASS <name> / FAIL <name>) keeps the wire format comparable to the Rust harness (the parity suite normalizes both); exit 0 + "all generated C tests passed" or exit 1 + failure count.

**Fuzz skeleton.** emit_fuzz_target_c: a SEPARATE file - building it needs a libFuzzer-enabled clang, which the plain gcc -Wall test artifact must not require (the CLI writes it next to --out; stdout mode is refused). Same v1.0 header + wiring recipe in its header comment. LLVMFuzzerTestOneInput decodes per `src/codegen_test.rs::build_fuzz_decode`: first byte variant selector (byte % n_variants), bool = one non-zero byte, bit<W> = ceil(W/8) little-endian bytes masked to W bits (payload_byte_count), enum = constant representative. SINGLE-EXIT decode (Phase 5 task 16 close-out fix): a truncated field run or an unknown variant sets done and breaks out of the loop so the trailing fsm_free always runs - mirroring the Rust skeleton's break + scope-drop, which keeps per-input allocations from accumulating. The loop steps the FSM with fuzz_stop_handler until the input is exhausted.

Code: `src/backends/c_test.rs::generate_unit_tests`,
`src/backends/c_test.rs::rep_input_c`,
`src/backends/c_test.rs::var_value_literal_c`,
`src/backends/c_test.rs::vars_initializer_c`,
`src/backends/c_test.rs::input_case_expr_c`,
`src/backends/c_test.rs::resolve_output_variant_c`,
`src/backends/c_test.rs::render_outputs_assert_c`,
`src/backends/c_test.rs::render_signature_compare_c`,
`src/backends/c_test.rs::render_transition_test_c`,
`src/backends/c_test.rs::render_invariant_test_c`,
`src/backends/c_test.rs::render_monitor_test_c`,
`src/backends/c_test.rs::render_random_trace_test_c`,
`src/backends/c_test.rs::payload_byte_count`,
`src/backends/c_test.rs::emit_fuzz_target_c`,
`src/backends/c_test.rs::render_negative_c`,
`src/codegen_fsm.rs::build_variants`,
`src/codegen_test.rs::build_test_plan`,
`src/codegen_test.rs::build_fuzz_decode`

## Parity notes vs the Rust backend

One plan, two renderings: both backends consume the same codegen plans
(`src/codegen_fsm.rs::build_variants`, `src/codegen_test.rs::build_test_plan`,
`src/mc_plan.rs::McPlan`, `src/mc_plan.rs::LivenessDecomposition`, the shared
SMT spec) - differences are target syntax only (placement rule: CODEGEN.md
section 2). The Rust twin of each file is src/backends/rust*.rs (deep dive:
docs/dev/backend_rust.md).

- Type mapping: D8 widths {8, 16, 32, 64} vs Rust u8..u128 (any width up to
  128); enum E_t typedef vs Rust enum; tagged unions vs algebraic enums.
- Expression lowering: C uint64 intermediates + masks vs Rust u128 wrapping +
  mask - the same modular result for every C-legal width; shifts: C runtime
  guard vs Rust if-else - the same saturation (amount >= width -> 0). Pinned by
  tests/wide_ops_parity_test.rs.
- Monitors: identical MonitorPlan rendering (DFS pre-order / post-order, same
  field sets and step semantics); label naming via the shared
  `src/backends/rust.rs::snake` so mon_<label> names match; out-of-subset
  warning text identical in every backend (D-P3-5).
- Firing order: both render descending effective priority then source order -
  it mirrors generated fsm_step AND the planner's static analysis.
- MC engine: same embedded plan tables; C sorted-vector visited set vs Rust -
  behavior must not depend on the container (D-P4-2); two-phase parallel
  expansion byte-identical to --threads 1 in both; pinned summary line +
  statistics block identical; JSON key ordering + escape contract identical
  (embedded copies of the same writer); lasso / deadlock / trace
  counterexample fields byte-identical.
- SMT: same shared spec section (`src/smt_plan.rs::build_smt_spec` /
  `src/smt_emit.rs::emit_c_spec`), fixed variable names / constraint order /
  query order, one libz3 build (5.3.1).
- Fuzz runtime: same LCG constants + seed defaults + phase-ordered input
  selection; same summary line.
- Tests: same stable test names + random-trace coverage line; C plain-assert
  table-driven main() vs Rust #[test] module - the parity suite normalizes
  both outputs.

Pinned by: tests/backend_parity_test.rs (cross-backend differential parity),
tests/prod_parity_test.rs (production random-input stdout parity over all
valid examples, D-P6-6), tests/wide_ops_parity_test.rs (wide-op lowering),
tests/liveness_mc_test.rs (lasso fields + truncated-mode soundness),
tests/benchmark_mc_test.rs (performance).

Code: `src/codegen_fsm.rs::build_variants`,
`src/codegen_test.rs::build_test_plan`, `src/mc_plan.rs::McPlan`,
`src/mc_plan.rs::LivenessDecomposition`,
`src/smt_plan.rs::build_smt_spec`, `src/smt_emit.rs::emit_c_spec`,
`src/backends/rust.rs::snake`, `tests/backend_parity_test.rs`,
`tests/prod_parity_test.rs`, `tests/wide_ops_parity_test.rs`,
`tests/liveness_mc_test.rs`, `tests/benchmark_mc_test.rs`

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
