# Phase 8, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/backend_c.md` (Class A deep dive): full content on the C
target-language backend's core translation + FSM codegen - plugin registration
+ shared C-specific helpers (`src/backends/c.rs`) and production FSM source
generation (`src/backends/c_fsm.rs`) - the parity principle: one plan,
rendered identically in Rust and C. Also fill in the matching Class B card
`docs/llm/backend_c.md` (within its 150-line budget) covering ALL FOUR C
backend files - per the confirmed scope decision (plan Corrections entry 3;
the card's TBD text assigns it to task 10.1). The model-checker and unit-test
sections of the Class A doc are added as anchored sections with single-TBD
content for subtask 10.2.

## Context

- **Skeleton state (from task 1.7)**: `docs/dev/backend_c.md` exists with one
  anchored fragment `dev:backend-c-overview` (owner: ALL FOUR C files - each
  file's `//! Docs:` line already names it) followed by a TBD line;
  `docs/llm/backend_c.md` exists with budget header `<!-- llm-budget: 150 -->`,
  fragment `llm:backend-c` (same four owners), and a TBD line. Existing anchor
  + heading stay byte-stable.
- **Fragment structure**: keep `dev:backend-c-overview` for Overview; add
  four new sub-fragments, one per source file (`src/backends/mod.rs` is module
  declarations only - covered in the Overview):
  - `dev:backend-c-plugin` - Plugin entry + shared C helpers (`src/backends/c.rs`)
  - `dev:backend-c-fsm` - Production FSM generation (`src/backends/c_fsm.rs`)
  - `dev:backend-c-mc` - Model-checker executable generation (`src/backends/c_mc.rs`)
  - `dev:backend-c-test` - Unit-test + fuzz-target generation (`src/backends/c_test.rs`)
  STRUCTURAL DECISION (recorded here): unlike the single-subtask deep dives of
  tasks 8.x/9.x, this doc is split across subtasks 10.1/10.2. ALL FOUR
  sub-fragment anchors are added in 10.1 so the doc's final structure is
  stable; `dev:backend-c-mc` and `dev:backend-c-test` carry a single TBD line
  (task 10.2) until then, and 10.2 fills them in place. ORPHAN RULE: all four
  new IDs go on the owner file's `//! Docs:` line in the same action. Exact
  target lines:
  - `src/backends/c.rs` line 13 becomes:
    `//! Docs: dev:backend-c-overview, dev:backend-c-plugin, llm:backend-c, dev:deps-fn-backend-c`
  - `src/backends/c_fsm.rs` line 10 becomes:
    `//! Docs: dev:backend-c-overview, dev:backend-c-fsm, llm:backend-c`
  - `src/backends/c_mc.rs` line 18 becomes:
    `//! Docs: dev:backend-c-overview, dev:backend-c-mc, llm:backend-c`
  - `src/backends/c_test.rs` line 25 becomes:
    `//! Docs: dev:backend-c-overview, dev:backend-c-test, llm:backend-c`
  - `src/backends/mod.rs` line 7 UNCHANGED (owns only the Rust overview + card,
    per the task 9.2 decision).
- **No duplication with existing fragments**: src/backends/C.md is the
  architecture/design doc of the generated artifacts (file layout, artifact
  design + type mapping D8, tagged-union value representation, semantics-parity
  summary, MC mirror summary, known limitations); CODEGEN.md (repo root) is
  the EXTENSION guide - plugin API walkthrough, CLI surface, shared-module
  summaries incl. backend rendering details, adding-a-backend checklist,
  testing conventions, invariants; DEPENDENCIES.md `dev:deps-fn-backend-c` has
  the function-level call graph of c.rs + c_fsm.rs; CODE_MAP.md
  `dev:code-map-codegen` has the file entries; docs/dev/codegen_core.md (task
  9.1) owns the language-agnostic core - plugin API, planning data, MC
  concepts; docs/dev/backend_rust.md (task 9.2) is the Rust twin. The deep dive
  goes DEEPER: per-function detail of the emitters (Gen pipeline order incl.
  the up-front width rejection, tagged-union emission, monitor node rendering
  with exact field sets and step semantics, pattern-scope binding mangling +
  reserved sets, simultaneous assignment lowering, gen_expr_c width semantics),
  the exact generated-code shapes that are pinned formats (artifact header,
  MAX_OUTPUTS, step_result_t layout, mon_* struct shape, firing order),
  invariants (parity, D8 widths, gcc -Wall zero warnings, deterministic bytes,
  no_std no-op), and change guidance. Cross-link; never restate.
- **Verified source material (measured 2026-10-31; line numbers for
  navigation only - doc references use item names)**:
  - `src/backends/mod.rs` (16 lines): module declarations c, c_fsm, c_mc,
    c_test, rust, rust_fsm, rust_mc, rust_test; no code.
  - `src/backends/c.rs` (177 lines; production 1-135, `mod tests` 138-177 with
    4 tests): C_KEYWORDS (19: 38 entries - the 35 C keywords + bool/true/false
    from stdbool.h + main), c_ident (32: keyword to `c_` prefix, everything
    else unchanged - RFS identifiers are already snake_case-friendly),
    c_binding_ident (46: keyword mangling + rename of bindings colliding with
    the caller's reserved set to `<name>__rfsb`, further suffixed until unique
    among `used`; Phase 6 task 6 finding - a field named `n` would shadow
    mc_step_all's effect counter `int n`: effects written to `out[payload
    value]` (an out-of-bounds stack write for wide fields) and the counter
    never incremented, so the C model checker silently missed every successor
    through such a transition - a backend-parity violation reachable from fully
    valid specs; the caller passes its reserved set: {f, in, handler, user} in
    transition_block_c, {state, vars, out, n} in render_mc_step_all_body),
    bit_c_type (66: 0..=8 uint8_t / 16 uint16_t / 32 uint32_t / 64 uint64_t /
    else Err - decision D8; the Rust backend instead falls back to u128),
    c_type (80: Bool to bool, Bit{w} to bit_c_type(w), Enum{name} to
    `<Name>_t`), int_c_literal (90: width < 64 to `({value & mask}u)` keeping
    the literal in-range for the target type, else `({value}ULL)`),
    mask_c_expr (98: w >= 64 to `0xFFFFFFFFFFFFFFFFULL` else
    `((1ULL << w) - 1)`), PRODUCTION_INFO (106) / TEST_INFO (113: --out
    REQUIRED, fuzz skeleton written next to it, stdout refused) / MC_INFO
    (120: codegen-time + runtime flags incl. --fast non-parity and the
    wall-clock-only timeout caveat), C_BACKEND (128). Tests:
    identifier_mangling (142), type_mapping (149), backend_registration_shape
    (164), literals_and_masks (171).
  - `src/backends/c_fsm.rs` (2160 lines, no unit tests):
    generate_production (23: --with-property-monitors to Gated else Off),
    generate_core (37: shared by all three commands; test and mc always pass
    Always; returns CodegenOutput with empty extra_files - the commands append
    their own), Gen (51: variants, prop_mode, assume_plans/prop_plans in
    declaration order, active_assumes, property_names, code),
    render_signal_expr_c (69, pub(crate): per-sample signal expression against
    `s: const rfs_sample_t*` referencing pattern helpers named
    `{prefix}_in_<i>` / `{prefix}_out_<i>`; VarExpr via gen_expr_c with vars
    mapped to s->vars.<name> and state to s->to_state; shared by the monitor
    renderer (prefix `mon_<m>`), the liveness sample evaluator and the
    generated invariant tests - Phase 5 task 6), input_is_sample_helper_c
    (127) / output_has_sample_helper_c (141: static inline bool helpers; NULL
    input - sample 0 - never matches), input_match_cond_c (154) /
    output_match_cond_c (230: kind check + literal field guards - Int via
    int_c_literal, Bool true/false, EnumVariant to the `ENUM_VARIANT` constant;
    wildcard/bind unconstrained; None when unresolvable - unit pattern on a
    struct variant, field-count mismatch, empty group), Gen::new (309: rejects
    unsupported bit widths UP FRONT collecting every bad width into one error,
    D8; build_variants; compile_monitors; fixed emission order emit_header ->
    emit_types -> emit_monitors -> emit_fsm_impl), has_prop/prop_wrap/
    prop_emit (364/372/385: Gated wraps in `#ifdef RFS_PROPERTY_MONITORS` with
    the directive lines standing alone - call sites may be mid-line; Off
    omits; Always bare), emit_header (393: line 1 = v1.0 unified artifact
    header D-P6-3, identical text to the Rust backend (`//` is a valid C
    comment); includes stdint.h/stdbool.h/string.h/stdio.h/stdlib.h only;
    --no-std accepted as a documented no-op - one comment line; RFS_SPEC_
    VERSION define when module.version is set; Gated adds `#define
    RFS_PROPERTY_MONITORS 1`), emit_types (423: non-State enums first -
    `<NAME>_<VARIANT>` constants + `<name>_t` typedefs, vars/fields may
    reference them; State enum to STATE_* / state_t (`typedef int state_t`
    when no State enum); vars_t struct (empty module to `{ int _unused; }`);
    tagged unions via emit_tagged; `#define MAX_OUTPUTS n` = max emit count
    over transitions + on_error, minimum 1; trace_step_t / rfs_trace_t with a
    dynamic realloc buffer (cap 0 -> 8 -> doubling); rfs_status_t - RFS_OK /
    RFS_UNEXPECTED_EVENT / RFS_ASSUMPTION_VIOLATION / RFS_PROPERTY_VIOLATION;
    step_result_t - fixed outputs[MAX_OUTPUTS] + n_outputs + assume_label[64]
    + prop_label[64] + prop_step + prop_trigger_step, zeroed by
    rfs_make_result; rfs_policy_t Continue/Stop/Panic + the
    rfs_assumption_handler function pointer (the C analogue of the Rust trait)
    + rfs_panic_handler (fprintf + abort); trace_state/trace_vars - position 0
    is the initial configuration, else steps[pos-1]; trace_push), emit_tagged
    (513: one payload struct per struct variant `<kw>_<variant>_payload_t`,
    kind enum `<KW>_<VARIANT>`, wrapper `int kind` + union payload; empty group
    to a `_NONE` kind + `int _none`), Ctx (586: names map + state_variants
    set), c_contains_ident (592: whole-token identifier containment - drives
    the `(void)` casts for unused bindings), gen_expr_c (608: types every
    subexpression via validator::type_expr; Int to int_c_literal at the
    expression's width; Path single-segment to ctx.names or a STATE_ variant,
    multi-segment to the `ENUM_VARIANT` constant; BitNot/Neg cast to the target
    type after `~` / `0ULL - x`; Star computes in a uint64_t-wide intermediate
    (Phase 6 task 2a - bit<16> products above INT_MAX must not be signed
    overflow UB) while + and - stay as-is; every bit<N> arithmetic/bitwise
    result masked to the type width; comparisons render bare `({l} {sym} {r})`
    relying on C integer promotion (both sides non-negative within width);
    LogicAnd/Or to &&/||; ShiftLeft/Right widen BOTH operands to uint64_t with
    a runtime guard `amount >= w -> 0` (Phase 6 task 2a - the old
    promotion-based rendering was UNDEFINED when a variable amount reached the
    promoted LHS width, e.g. gcc wraps the count mod 32); one uniform form for
    literal and variable amounts, matching the Rust backend's saturation),
    compile_monitors (787: monitor::compile_formula per property / selected
    assumption; out-of-subset skip warning with IDENTICAL text in every
    backend, D-P3-5), emit_monitors (819: rfs_sample_t - the C analogue of
    Rust's Sample, input NULL at sample 0; per-label mon_<snake(label)>_t
    structs in declaration order; the property machinery gated TOGETHER -
    rfs_property_violation_t + per-label structs + prop_monitors aggregate;
    assume_monitors aggregate always present (an empty stub keeps the fsm_t
    shape and call sites uniform); PROPERTY_LABELS/ASSUME_LABELS/N_* constants
    with an RFS_UNUSED attribute so a production TU without them referenced
    stays -Wall-clean, D-P3-4), render_monitor (907: nodes numbered in DFS
    pre-order over the plan tree, step body evaluates them post-order;
    mon_<m>_step returns m->n0_failed && !before = newly confirmed at this
    sample; mon_<m>_trigger_step from n0_trigger_step; one helper function per
    distinct pattern mon_<m>_in_<i>/mon_<m>_out_<i> + signal expressions
    mon_<m>_sig_<i>), render_node (981: RangeCheck - now recomputed every
    sample (the child may recover), failed sticky, trigger_step stays -1;
    Obligation bound 0 = be[0] body at the trigger sample; bound >= 1 = D-P3-3
    pending model with a `long long` remaining countdown, failure terminal;
    DelayedCheck prev_antecedent register, terminal, no trigger sample (stays
    -1); And/Or per-sample over the children's _now - conjunction fails while
    ANY child fails, disjunction only while ALL fail, trigger_step = min of
    the children's), range_cond_c (1124: AnchorRange to an idx condition;
    "true" when unbounded from 0), gen_signal_c (1137) /
    input_is_sample_helper_c method (1155) / output_has_sample_helper_c method
    (1165: prefix `mon_<m>` wrappers over the free functions),
    render_assume_monitors (1175: aggregate struct + new + step - first newly
    failed ACTIVE assumption in declaration order, label looked up in the
    active_assumes array, or NULL), render_prop_monitors (1208: aggregate;
    first new violation fills out {label, step: idx, trigger_step}; empty stub
    when no properties), emit_fsm_impl (1246: fsm_t struct with gated prop
    fields; initial vars as a designated initializer - vars mapped to
    `vars.<name>`, a render failure falls back to `.name = 0`; fsm_new steps
    sample 0 - an assumption violated at sample 0 aborts, a property violation
    is recorded sticky; fsm_free (Phase 5 task 16 close-out fix) frees the two
    per-fsm heap allocations - trace buffer + active_assumes array - so a
    long-lived consumer (the libFuzzer target calls it once per input) does not
    accumulate RSS; rfs_commit is static RFS_UNUSED - push the trace step,
    idx = trace.len, build the sample, assume monitors -> handler policy (Stop
    to RFS_ASSUMPTION_VIOLATION with the label, Panic to abort), prop check
    first-violation-wins (prop_label[0] == 0 guard) to RFS_PROPERTY_VIOLATION,
    then state/vars update; fsm_step - NULL handler defaults to
    rfs_panic_handler, transitions grouped by state in first-appearance order,
    within a state sorted by DESCENDING effective priority (stable sort_by_key
    Reverse(priority.unwrap_or(0))) - mirrors the planner's firing order;
    on_error fallback appended (no state check, no bindings, vars carried over)
    or RFS_UNEXPECTED_EVENT), transition_block_c (1409: nested if-blocks -
    pattern match (kind + literal guards) -> bindings as locals in declared
    field order via c_binding_ident with reserved set {f, in, handler, user} ->
    guard -> actions; a binding no rendered guard/output/assignment references
    gets a `(void)` cast (c_contains_ident); SIMULTANEOUS assignment lowering -
    all tmp_<var> evaluated from pre-step values then applied to a copy of
    f->vars; unit pattern on a struct variant is an error), output_expr_c
    (1645: bare output to the group's single variant else "bare output
    ambiguous"; per-arg field count check; designated initializer `(output_t){
    .kind = OUTPUT_*, .payload.output_<v>.<f> = ... }`), collect_signal_
    patterns (1699: distinct InputIs/OutputHas patterns in first-seen DFS
    order over the plan tree), render_mc_step_all_c (1762, pub(crate): builds a
    Gen in Off mode and renders mc_step_all - every enabled transition in
    firing order; deterministic mode uses only the first effect, nondet
    explores all; on_error fallback appended with transition_id = -1; rendered
    from the same IR with the same helpers as fsm_step so both paths agree by
    construction), collect_liv_signal_patterns (1772, pub(crate): distinct
    patterns of one signal in first-appearance order), render_liv_sample_
    evaluator_c (1812, pub(crate): per-sample boolean evaluator for a
    temporal-operator-free formula - Phase 4 task 7 liveness leaves, C mirror
    of rust_fsm's render_sample_formula_evaluator; emits unique pattern helpers
    + `static int {name}(const rfs_sample_t*)` returning 0/1), mc_pattern_cond
    (1847: mirrors the pattern part of transition_block_c),
    render_mc_step_all_body (1935: mc_effect_t struct; per-state firing order
    identical to emit_fsm_impl; bindings with reserved set {state, vars, out,
    n}; effect body collects into out[n] WITHOUT commit - no trace, no monitor
    stepping; on_error fires only when n == 0 with transition_id = -1).
  - `src/backends/c_mc.rs` (2836 lines, no unit tests) and
    `src/backends/c_test.rs` (1035 lines, no unit tests): deep dive in subtask
    10.2; for the Class B card only the entry points + template constants were
    verified: c_mc - generate_model_checker (27: rejects no_std +
    with_protocol_fsm with the pinned constants; SmtBounded to a standalone
    libz3 binary = v1.0 header + smt_emit::emit_c_spec(build_smt_spec(...)) +
    SMT_ENGINE_C with {BOUND}; exact/fuzz to the FSM core via generate_core
    (Always) + shared_printers + exact_codegen_c + EXACT_ENGINE_C /
    LIV_ENGINE_C / JSON_WRITER_C / EXACT_RUN_C / MAIN_DISPATCH_C); c_test -
    generate_unit_tests (788: rejects no_std; generate_core with Always +
    build_test_plan; table-driven main() with one PASS/FAIL line per test,
    failures counted not aborting; cont_handler emitted only when a test uses
    it) + emit_fuzz_target_c (546: separate libFuzzer skeleton, single-exit
    decode, fsm_free per input).
- **Consumers / pins (verified)**: tests/codegen_test.rs (C generation over
  all valid examples, flag toggles, the no_std comment pin, gcc -Wall zero-
  warning compile sweep); tests/backend_parity_test.rs (11 tests: cross-backend
  differential parity); tests/prod_parity_test.rs (production-FSM cross-backend
  random-input parity D-P6-6 - identical seeded LCG input scripts, seed 43210,
  200 steps, byte-identical stdout through a raw-value driver independent of
  either backend's renderers); tests/wide_ops_parity_test.rs (2 tests:
  cross-backend differential pins for the bitvector arithmetic lowering,
  Phase 6 task 2a); src/backends/C.md (artifact design + key decisions + known
  limitations).
- **Entry format / gate rules / bookkeeping**: same as subtask 8.1 in
  PHASE_8_TASK_8.md (house style with per-section Code: lines; doc-to-code
  reference rules - verify every reference against a declaration line; run
  dev_docs_test after each action, docs_md_test at task end; plan row ->
  DONE, boxes checked, one TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-31): all items listed above plus `src/codegen.rs::NO_STD_
  PRODUCTION_ONLY_ERROR`, `src/codegen.rs::PROTOCOL_FSM_MC_ERROR`,
  `src/mc_plan.rs::McPlan`, `src/smt_plan.rs::build_smt_spec`,
  `src/smt_emit.rs::emit_c_spec`, `src/monitor.rs::compile_formula`,
  `src/monitor.rs::formula_to_signal`, `src/codegen_fsm.rs::build_variants`,
  `src/codegen_test.rs::build_test_plan`, `src/validator.rs::type_expr`,
  `src/validator.rs::Env`, file-level `tests/codegen_test.rs`,
  `tests/backend_parity_test.rs`, `tests/prod_parity_test.rs`,
  `tests/wide_ops_parity_test.rs`. Non-reference backticks (safe): C.md,
  CODEGEN.md, DESIGN.md, SEMANTICS.md, shell commands, bare identifiers,
  generated-code item names without a file-path prefix.

## Scope

### Depends on tasks

- 1.7 (skeleton + pointers + manifest entries) - DONE
- 9.2 (Rust twin deep dive; the parity notes reference it) - DONE

### Dependent tasks

- 10.2 (fills the two TBD sections added here + parity notes vs the Rust backend)
- 13.2 (CHANGE_IMPACT table rows for c*.rs), 14 (pointer sync), 16 (close-out)

### Affected files

- change: `docs/dev/backend_c.md` (deep dive + two anchored TBD sections)
- change: `docs/llm/backend_c.md` (Class B card, all four files)
- change: `src/backends/c.rs`, `src/backends/c_fsm.rs`, `src/backends/c_mc.rs`,
  `src/backends/c_test.rs` (`//! Docs:` lines only - doc comments, no code)

## Planned actions

### Action 1: Class A deep dive - `docs/dev/backend_c.md`

Replace the TBD and add the four new sections (anchor line immediately above
each heading):

- **Overview** (`dev:backend-c-overview`, existing anchor): the parity
  principle; file map with line/test counts; position in the pipeline via
  C_BACKEND's fn pointers into generate_core; key differences from the Rust
  backend (D8 widths, tagged unions instead of enums, no_std no-op,
  --with-protocol-fsm ignored, no aggregate test crate); cross-links to C.md,
  CODEGEN.md, `dev:deps-fn-backend-c`, `dev:code-map-codegen`, DECISIONS
  D-numbers.
- **Plugin entry + shared C helpers** (`dev:backend-c-plugin`): registration +
  descriptor text; C_KEYWORDS + c_ident + c_binding_ident with the two caller
  reserved sets and the Phase 6 task 6 finding; bit_c_type D8 rejection vs the
  Rust u128 fallback; c_type/int_c_literal/mask_c_expr; the note that monitor
  label naming reuses `crate::backends::rust::snake` so mon_<label> names match
  across backends.
- **Production FSM generation** (`dev:backend-c-fsm`): entry points + Gen
  pipeline order incl. the up-front width rejection; artifact header + no_std
  comment pin; emit_types (tagged unions, MAX_OUTPUTS, trace/status/policy
  types); gen_expr_c width semantics (uint64 intermediate for *, shift guard
  amount >= w -> 0) in lockstep with the Rust backend and the planner's static
  evaluator; monitor rendering (DFS pre-order / post-order, per-node-kind field
  sets and step semantics incl. be[0] and the D-P3-3 pending model); the shared
  signal machinery (free functions, prefix-parameterized so monitor rendering
  stays byte-identical); Fsm impl (fsm_new sample-0 stepping, fsm_free,
  rfs_commit policy chain, priority-desc stable firing order); transition
  blocks with simultaneous tmp_* lowering + binding mangling + (void) unused
  casts; output_expr_c designated initializers; render_mc_step_all_c /
  render_liv_sample_evaluator_c shared renderers.
- **Model-checker executable generation** (`dev:backend-c-mc`): single TBD line
  (task 10.2).
- **Unit-test + fuzz-target generation** (`dev:backend-c-test`): single TBD
  line (task 10.2).
- **Changing the C backend** (unanchored closing section): developer checklist
  - parity first + differential suites; pinned formats list; firing order
  mirrors fsm_step AND the planner; gen_expr_c width semantics lockstep; D8
  widths; gcc -Wall zero-warning bar; deterministic artifacts; no_std /
  protocol-fsm no-ops; verify commands + docs sync.

Run `cargo test --test dev_docs_test` after writing; extend the four `//!
Docs:` lines in the same action (mod.rs unchanged).

### Action 2: Class B card - `docs/llm/backend_c.md`

Replace the TBD (keep budget header, title, intro, anchor byte-stable):
responsibility; key-items table covering all four files (C_BACKEND / c_ident +
c_binding_ident reserved sets / bit_c_type D8 / int_c_literal + mask_c_expr /
generate_core pipeline / emit_types tagged unions + MAX_OUTPUTS / gen_expr_c
width semantics / render_monitor + render_node / shared signal machinery /
fsm_step firing order + rfs_commit policy chain / transition_block_c tmp_*
lowering / render_mc_step_all_c / c_test five test classes + fuzz_target.c
single-exit decode / c_mc exact engine mirror + liveness + JSON writer + SMT
via emit_c_spec); invariants that must not break (parity, D8, firing order,
width-semantics lockstep, gcc -Wall zero warnings, deterministic bytes,
no_std/protocol-fsm no-ops); change-impact pointers (placement rule; verify
with codegen_test + backend_parity_test + prod_parity_test +
wide_ops_parity_test; full table in llm:change-impact at task 13.2); deep-dive
links to the five Class A fragments + C.md + CODEGEN.md + codegen_core +
backend_rust. Total <= 150 lines. Run the gate.

### Action 3: verification + bookkeeping

dev_docs_test 9/9, docs_md_test 5/5, full verification (build/test/clippy/fmt
+ workspace; counts unchanged: 535/535 root, 559/559 workspace); plan row 10.1
-> DONE; check off this subtask's boxes; TASKS.md entry; commit per action.

## Task deliverables

- `docs/dev/backend_c.md`: Overview + plugin + FSM sections filled; mc/test
  sections anchored with single TBD lines (task 10.2); "Changing the C backend"
  checklist.
- `docs/llm/backend_c.md`: card filled (all four files), <= 150 lines.
- Four `//! Docs:` lines extended (doc comments only, no code).

## Task exit criteria

- [x] `docs/dev/backend_c.md`: Overview + plugin + FSM sections filled, no TBD
      in them; mc/test sections anchored with a single TBD line each naming
      task 10.2; every technical statement carries a resolving doc-to-code
      reference (gate check 1); the existing `dev:backend-c-overview` anchor +
      heading unchanged; all four new fragments anchored AND owned in the same
      action.
- [x] `docs/llm/backend_c.md`: card filled, no TBD, total line count <= 150
      (58), links to Class A without restating it.
- [x] Source edits limited to the four `//! Docs:` lines (no code changes); all
      cargo counts unchanged from the task-9.2 tree (535/535 root, 559/559
      workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 10.1 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

## Non-goals

- No backend behavior changes of any kind; no new dependencies.
- No MC/test generation content (subtask 10.2 fills the two TBD sections +
  parity notes vs the Rust backend); no restatement of C.md, CODEGEN.md,
  DEPENDENCIES graph, codegen_core, or backend_rust content (link only).
- No edits to other docs (START.md/KB.md/BUILD.md pointer sync = task 14).
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-10-31) for plan group 10 | User model: one task file per plan group ("execute 10.1" = subtask of task file 10). Written for subtask 10.1 only; subtask 10.2 (c_mc + c_test deep dive + parity notes) will be appended at execution, following the PHASE_8_TASK_9.md pattern (9.2 appended after 9.1 completed). |
