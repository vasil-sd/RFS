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

## Subtask 10.2 - C backend MC + test generation deep dive (added at execution, see Task corrections)

### Goal

Fill in the two TBD sections of `docs/dev/backend_c.md` (Class A deep dive):
model-checker executable generation (`src/backends/c_mc.rs`, fragment
dev:backend-c-mc) and unit-test + fuzz-target generation
(`src/backends/c_test.rs`, fragment dev:backend-c-test), plus an unanchored
"Parity notes vs the Rust backend" section - backend parity 2/2b. The Class B
card `docs/llm/backend_c.md` was already filled in full by subtask 10.1 (all
four files, per the confirmed scope decision); correct it here only if an
inaccuracy is found.

### Context

- **Doc state after 10.1**: `docs/dev/backend_c.md` is 412 lines with all five
  anchored sections in place; `dev:backend-c-mc` and `dev:backend-c-test`
carry single TBD lines naming task 10.2 (anchors + ownership already done in
  10.1 - no source edits needed for the fragments). The unanchored "Changing
  the C backend" checklist sits at the end; the new parity-notes section goes
  between the test section and that checklist.
- **No duplication with existing fragments**: src/backends/C.md section 5 is
  the MC MIRROR SUMMARY (wire format, visited-set choice, pthreads mirror,
  legacy fuzz wire format, SMT libz3 note, counterexample report format,
  zero-warning bar); CODEGEN.md has the plugin/CLI surface; DEPENDENCIES.md
  `dev:deps-fn-backend-c` has the c.rs/c_fsm.rs call graph (c_mc/c_test are
  covered by the file-level graph); docs/dev/codegen_core.md owns the shared
  plan data (McPlan, TestPlan, McConfig, Lcg) and docs/dev/backend_rust.md is
  the Rust twin. The deep dive goes DEEPER: per-function detail of the emitters
  and the exact generated-code shapes that are pinned formats (plan-table
  constants, config_bytes layout, engine structures, JSON key order + escape
  contract, SMT variable naming + constraint/query order, test names +
  random-trace line, fuzz decode wire format), invariants (parity,
  container-independence D-P4-2, truncated-analysis soundness, gcc -Wall zero
  warnings, deterministic bytes), and change guidance. Cross-link; never
  restate.
- **Verified source material (measured 2026-10-31; line numbers for
  navigation only - doc references use item names)**:
  - `src/backends/c_mc.rs` (2836 lines, no unit tests):
    generate_model_checker (27: rejects no_std + with_protocol_fsm with the
    pinned constants; SmtBounded -> standalone libz3 binary = v1.0 header +
    smt_emit::emit_c_spec(build_smt_spec(module, McPlan::compute(...))) +
    SMT_ENGINE_C with {BOUND} <- mc_smt_bound - no FSM core, no exact/fuzz
    runtimes; exact/fuzz -> FSM core via generate_core (Always) +
    shared_printers + exact_codegen_c + fuzz_runtime + MAIN_DISPATCH_C;
    warnings = prod warnings + liveness decomposition diagnostics D-P4-9),
    shared_printers (91: C functions reproducing byte-for-byte the Rust MC
    binary's Debug-based counterexample format D-P4-4 - per-enum name lookups
    ONLY for enums referenced by vars/input/output fields (an unused static
    would trip gcc -Wall); state_name_c returns ref_mc's prefixed form
    "State::Variant" shared by text trace and JSON; print_vars_debug
    `Vars { f: v, ... }` declaration order no trailing newline;
    print_input_debug `Input::Variant { ... }`; print_outputs_debug
    `[Output::X, ...]`; print_trace pinned "counterexample trace:" format;
    names via rust_ident - same name source as the JSON renderers),
    print_value_c (306: bool true/false, bit unsigned long long decimal,
    enum via the shared lookup), domain_loop (317: canonical iterator bounds
    per ValueDomain D-P4-2), rep_value_c (332: representative value C
    rendering; EnumEmpty/EnumUnknown have no C rendering - error),
    domain_size (353), rep_inputs (362) / rep_input_c (371: same defaults as
    the C tests - bool false, zero integers, first declared enum variant),
    exact_codegen_c (404: codegen-time defaults embedded as runtime-
    overridable constants D-P4-7 - DEFAULT_MODE/MAX_STATES/NONDET/
    TIMEOUT_SEC/THREADS/COUNTEREXAMPLE_FORMAT + CONFIG_TOTAL_BYTES /
    MONITORS_BITS/STATE_BITS from plan.encoding; N_TOTAL_INPUTS = product of
    concrete field domain sizes; MC_MAX_EFFECTS = max transitions per state +
    on_error; all_inputs() canonical enumeration D-P4-2 - variants declaration
    order, concrete fields nested in declaration order over their domains
    (bool 0/1, int 0..=max, enum variant index via _dom_k tables), abstracted
    at representatives; put_bits + config_bytes layout - state index bits,
    var slots (bool 1/0, bit as u64, enum variant index), monitor node bits
    with remaining packed ONLY while pending = ref_mc CfgKey byte-for-byte;
    render_mc_step_all_c; EXACT_ENGINE_C; JSON_WRITER_C + json_renderers_c;
    liveness - LivenessDecomposition::compute diagnostics to codegen warnings
    D-P4-9, LIV_ENGINE_C, leaf evaluators liv_p<i>_l<j>[b] (Safety 0 /
    Eventually 1 / Until 2 with eval_b) via render_liv_sample_evaluator_c,
    flatten_skeleton_c first-appearance re-indexing to liv_p<i>_skel,
    LIV_PROPS table with a dummy entry when no liveness properties),
    json_renderers_c (742: module-specific mc_json_input/mc_json_output/
    mc_json_vars - names via rust_ident, bool true/false, bit unsigned long
    long decimal, enum variant name via the shared lookup), json_value_c
    (835), flatten_skeleton_c (846: C mirror of rust_mc's flatten_skeleton),
    c_string_literal (876), var_value_c (894); EXACT_ENGINE_C (908: mc_node_t
    {state, vars, prop, assume}; mc_step_successor at depth+1 - 0 success / 1
    pruned by an active assumption / 2 property violation; two-phase parallel
    expansion task 10B - phase A workers over disjoint config ranges compute
    per-(input, effect) outcomes into per-config lists (pure w.r.t. shared
    state), phase B sequential canonical commit = byte-identical to --threads
    1 for any N; visited set = sorted vector of config byte strings + binary
    search with a parallel node-index array - behavior must not depend on the
    container D-P4-2; mc_fill_path parent pointers; mc_replay rebuilds the
    Phase 3 trace format on a fresh Fsm - nondet picks the effect with the
    recorded transition_id else first, -1 = on_error; mc_report_property
    pinned property report), LIV_ENGINE_C (1121: per-leaf progress automata -
    safety 0=ok/1=bad, eventually 0=wait/1=done, until 0=ok/1=sat/2=bad, all
    one-way so every state sequence is eventually constant = limit vector;
    EDGE-based labels - position 0 initial config sample, position k sample of
    step k; liv_bits_at cached per edge position (PER-PROPERTY cache - a
    shared cache mislabels edges from the second property on, found and fixed
    in Phase 4 task 7); liv_delta mixed-radix; liv_good; liv_analyze product
    BFS from (start_q, config 0), per-q stay-edge subgraph, nondet STRONGLY
    FAIR filter D-P4-11 - each cycle state exactly one explored successor and
    fully expanded when truncated, iterative Kosaraju SCCs over ascending
    config ids (the SCC partition is independent of DFS start order), cyclic =
    comp size > 1 or self-loop, smallest id = canonical anchor; limit vector
    falsifies the skeleton -> liv_build_lasso: product path to (q, cstar) +
    stay-edge cycle through cstar = lasso witness (prefix + one cycle
    iteration); liv_note human explanation of the falsified leaves),
    JSON_WRITER_C (1708: embedded port of the toolchain compact writer
    src/json_rw.rs - mjb_* buffer functions; escape contract " \\ b t n f r +
    other control chars as \\u00XX lowercase hex else raw UTF-8; integers
    unsigned long long - all values non-negative, bit<64> reaches 2^64-1;
    mc_peak_rss_kb from getrusage(RUSAGE_SELF) ru_maxrss - INFORMATIONAL,
    normalized to 0 in cross-backend byte comparisons, 0 on failure;
    mc_json_doc_head key order = byte-parity contract: status, metrics
    {states, transitions, max_depth}, [reason, label, step], statistics
    {unexpected_terminations, monitor_size_bits, memory_estimate_bytes,
    peak_rss_kb}; ce kinds trace / lasso{prefix, cycle, cycle_length} /
    deadlock-config{depth, state, vars}; ce_steps ALWAYS the last key),
    EXACT_RUN_C (1880: run_exact() -> exit code; runtime flags over
    codegen-time defaults D-P4-7 - --max-states/--nondet/--timeout-sec/
    --threads/--fast/--counterexample-format/--assumes; sample 0 via fsm_new -
    assumption violated aborts, property violation reported immediately (JSON
    or text with the pinned summary line + dedicated sample-0 report);
    level-synchronous BFS D-P4-2 - visited at ENQUEUE time, parent pointers,
    depth, adj edges for every Success incl. to already-visited configs,
    complete flag; status 0 running / 1 pass-complete / 2 state limit / 3
    timeout / 4 fail, exit codes 0/1/2; parallel level expansion task 10B -
    phase A pthread workers (up to 64) over disjoint frontier chunks calling
    pure mc_expand_range, phase B SEQUENTIAL canonical commit = byte-identical
    to --threads 1 for any N, inline sequential fallback if pthread_create
    fails; --fast non-parity early abort - race-dependent which configs were
    expanded, never a false pass; Unexpected = path terminates D-P4-5 counted
    in statistics, transitions untouched; Pruned counts as a transition;
    deadlock = no successor at all -> FAIL reason=deadlock label=- step=depth;
    SEMANTICS 9.5 stop at first violation/state limit - check AFTER the
    per-level loop, matches the Rust engine's status == 0 guard; liveness only
    when BFS ended without safety failure/deadlock; truncated analysis sound -
    witness inside explored part or UNKNOWN stands, never a false pass-
    complete; JSON one compact document or text PASS (complete) / FAIL /
    UNKNOWN (state limit reached[, timeout after Ns]) + pinned summary line +
    statistics block), MAIN_DISPATCH_C (2315: --mode fuzz -> run_fuzz else
    run_exact), fuzz_runtime (2330: LCG state = state *
    6364136223846793005 + 1442695040888963407 wrapping, top-53-bit double
    mapping - SAME constants as codegen_test::Lcg and the Rust MC;
    ViolationSpec wire format label:prob:percent% defaults 1.0/100.0 - label
    truncated at the first ':' to match the Rust parser; assume selection
    all/none/labels filtered against ASSUME_LABELS; run_fuzz flags --bound 5 /
    --runs 20 / --seed 12345 / --assumes / --violate / --unexpected-prob =
    McConfig defaults; per step: phase 1 forced violation injection (trial-
    step ALL_INPUTS for the target label), phase 2 optional unexpected
    injection, phase 3 allowed representative inputs - property-violating
    inputs allowed on purpose; property violations terminate immediately exit
    1 so the property_violations counter is always 0 (field kept for a stable
    summary format); summary line "model-check summary: runs={} bound={}
    completed={} unexpected={} assumption_stops={} property_violations={}"),
    SMT_ENGINE_C (2517: C mirror of SMT_ENGINE_RUST - DEFAULT_BOUND <-
    {BOUND}; parity-critical: step variables z3 constants with FIXED names in
    fixed order s_i/v_i_j/ev_i/fld_i_g, constraints asserted in fixed order,
    queries in fixed order safety-then-liveness declaration order; two-arg
    Z3_mk_and/Z3_mk_or over 2-element arrays (the C counterpart of the Rust
    b_and/b_or wrappers - identical constraint trees); unrolling: enabled_t =
    state match and class match and guard (firing order), fire_t = enabled
    and no earlier enabled (first match, prefix-OR chain), alive_i =
    alive_{i-1} and any_fire, next state/vars via ITE chains; safety query
    viol_p = exists i<k: alive_i and not phi_p(i) -> FAIL + pinned summary
    line exit 1; liveness query never_p and cycle (exists a<b identical
    (state, vars)) -> FAIL reason=liveness-violation + cycle length exit 1;
    Unknown -> UNKNOWN (solver inconclusive within bound {k}) + status=unknown
    exit 2; all Unsat -> PASS WITHIN BOUND {k} (incomplete - no completeness
    claim) + status=pass-within-bound exit 0; smt_print_trace pinned witness
    format; targets the z3 5.3.1 API surface - Z3_L_UNDEF, no Z3_del_solver,
    Z3_model_eval bool + out-ast - one solver build shared with the Rust
    backend).
  - `src/backends/c_test.rs` (1035 lines, no unit tests):
    rep_input_c (40: representative input_t initializer - bool false, zero
    integers, first declared enum variant), var_value_literal_c (73: bool
    true/false, int decimal, enum ENUM_VARIANT constant), vars_initializer_c
    (97: panics "planner must cover variable" - contract violation),
    input_case_expr_c (117: field name + count checks), resolve_output_
    variant_c (147: bare emit to the group's single variant - multi-variant
    groups are rejected by production codegen so this cannot fail for a
    generated spec), render_outputs_assert_c (169: full assert or per-output
    partial match - variant + statically known fields only, the documented
    planner fallback when an arg is not evaluable), render_signature_compare_
    c (211: same_sig = state + full outputs; STRICT refusal to emit a
    toothless test when the signature's outputs are not fully evaluable),
    render_transition_test_c (263: name t<id>_<state>_on_<event> (+ _not_
    taken) - same stable names as the Rust renderer; taken asserts on the
    RECORDED trace step - robust against property-violation halts; not-taken
    CHECK(!same_sig)), render_invariant_test_c (349: name invariant_<label>;
    reuses the shared signal machinery with prefix rfs_inv_<label> ->
    rfs_inv_check_<label>(s); drives the positive trace (same greedy policy as
    positive_sequence_runs) and re-evaluates the per-sample atom at every
    recorded sample, failing with the step index; bounded checks samples < N
    only), render_monitor_test_c (434: name monitor_<label>_fires_on_
    violation; case 1 initial config already violates - fsm_new records at
    sample 0; case 2 one planned drive step; asserts the sticky
    property_violation label recorded first), render_random_trace_test_c
    (475: name random_trace_seed<seed>_steps<K>; precomputed LCG index table
    RT_IDX[] over ALL_INPUTS - same constants as the fuzz wire format, seed +
    indices fixed at plan time; stop_handler; prints ONE deterministic
    coverage line random_trace seed={} steps={} ok={} unexpected={}
    assumption={} property={} states_visited={} - same line as Rust),
    payload_byte_count (538: ceil(W/8)), emit_fuzz_target_c (546, pub:
    separate file - needs a libFuzzer-enabled clang which the plain gcc -Wall
    test artifact must not require; same v1.0 header + wiring recipe in its
    header comment; LLVMFuzzerTestOneInput decodes per build_fuzz_decode -
    first byte variant selector (byte % n_variants), bool = one non-zero
    byte, bit<W> = ceil(W/8) little-endian bytes masked to W bits, enum =
    constant representative; SINGLE-EXIT decode (Phase 5 task 16 close-out
    fix) - truncated field run or unknown variant sets done and breaks so the
    trailing fsm_free always runs, mirroring the Rust skeleton's break + scope-
    drop; loops over fsm_step with fuzz_stop_handler until exhausted),
    generate_unit_tests (788, pub: rejects no_std with NO_STD_PRODUCTION_
    ONLY_ERROR; generate_core with Always - D-P3-6, a spec that violates its
    own property fails its generated tests + build_test_plan(variants,
    module, steps); plain-assert harness - rfs_test_failures counter + CHECK
    macro (failures counted not aborting so one run reports every broken
    check), stop_handler/cont_handler (cont_handler emitted only when a test
    uses it - modules without transitions/monitor drives must stay -Wall-
    clean), clone_fsm deep-copies the trace buffer for trial steps, ALL_INPUTS
    = representative per variant + N_ALL_INPUTS, EXPECT_PROGRESS from
    plan.expect_progress; module-level tests initial_state_is_correct /
    positive_sequence_runs (greedy trial-clone drive up to K steps with
    stop_handler) / negative_unexpected_input_search via render_negative_c;
    extended classes appended after the module-level ones; table-driven main()
    - one function per test, one line per test PASS <name> / FAIL <name> keeps
    the wire format comparable to the Rust harness (the parity suite
    normalizes both), exit 0 + "all generated C tests passed" or exit 1 +
    failure count; appends ExtraFile fuzz_target.c), render_negative_c (968:
    three shapes - has_on_error: search for the on_error landing state, assert
    found when definite_unexpected else a comment; definite_unexpected without
    on_error: assert RFS_UNEXPECTED_EVENT found; else an empty test with a
    comment).
- **Consumers / pins (verified)**: tests/backend_parity_test.rs (11 tests:
  cross-backend differential parity - exact runs, RE-pinned values, generated
  test executables, liveness parity incl. truncated-mode soundness, JSON
  report parity incl. five status/reason cases + golden document pin, forced-
  violation parity); tests/prod_parity_test.rs (production random-input stdout
  parity over all valid examples, D-P6-6); tests/wide_ops_parity_test.rs;
  tests/liveness_mc_test.rs (9 tests); tests/benchmark_mc_test.rs (5 tests);
  src/backends/C.md section 5 (MC mirror summary).
- **Entry format / gate rules / bookkeeping**: same as subtask 8.1 in
  PHASE_8_TASK_8.md (house style with per-section Code: lines; doc-to-code
  reference rules - verify every reference against a declaration line; run
  dev_docs_test after each action, docs_md_test at task end; plan row ->
  DONE, boxes checked, one TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-31): all items listed above plus `src/codegen.rs::NO_STD_
  PRODUCTION_ONLY_ERROR`, `src/codegen.rs::PROTOCOL_FSM_MC_ERROR`,
  `src/mc_plan.rs::McPlan`, `src/mc_plan.rs::ValueDomain`,
  `src/mc_plan.rs::LivenessDecomposition`, `src/mc_plan.rs::SkeletonLeaf`,
  `src/smt_plan.rs::build_smt_spec`, `src/smt_emit.rs::emit_c_spec`,
  `src/codegen_fsm.rs::RepValue`, `src/codegen_test.rs::build_test_plan`,
  `src/codegen_test.rs::Lcg`, `src/codegen_mc.rs::McConfig`,
  `src/codegen_mc.rs::CounterexampleFormat`, `src/printer.rs::print_formula`,
  file-level `src/json_rw.rs`, `tests/backend_parity_test.rs`,
  `tests/prod_parity_test.rs`, `tests/wide_ops_parity_test.rs`,
  `tests/liveness_mc_test.rs`, `tests/benchmark_mc_test.rs`. Non-reference
  backticks (safe): C.md, CODEGEN.md, DESIGN.md, SEMANTICS.md, shell commands,
  bare identifiers, JSON key names, generated-code item names without a file-
  path prefix.

### Planned actions

1. **Class A deep dive - `docs/dev/backend_c.md`**: replace the two TBD lines
   in place (anchors + headings stay byte-stable) and insert the unanchored
   "Parity notes vs the Rust backend" section between the test section and
   "Changing the C backend":
   - **Model-checker executable generation** (`dev:backend-c-mc`): three entry
     shapes incl. the standalone SMT binary; shared_printers byte-for-byte
     Debug format + rust_ident name source; exact_codegen_c plan tables
     (canonical all_inputs, config_bytes layout matching ref_mc's CfgKey);
     EXACT_ENGINE_C (two-phase parallel expansion byte-identical to --threads
     1, sorted-vector visited set D-P4-2); LIV_ENGINE_C (limit vectors, strong
     fairness, per-property label cache, lasso witness); JSON_WRITER_C (key-
     ordering byte-parity contract, escape contract, peak_rss_kb
     normalization) + json_renderers_c; EXACT_RUN_C (SEMANTICS 9.5 stop rule,
     truncated liveness soundness, pinned summary line + statistics block);
     MAIN_DISPATCH_C + fuzz_runtime (LCG constants, phase-ordered input
     selection); SMT_ENGINE_C (fixed names / constraint order / query order,
     z3 5.3.1 API surface, pinned summary lines); embedded-copy rule.
   - **Unit-test + fuzz-target generation** (`dev:backend-c-test`): entry
     point + plain-assert harness (CHECK macro, conditional cont_handler,
     clone_fsm, ALL_INPUTS, EXPECT_PROGRESS); the module-level tests incl. the
     three negative-search shapes; per-transition tests with the STRICT not-
     taken refusal; literal rendering + partial-assert fallback; invariant /
     monitor / random-trace tests (stable names, deterministic coverage line);
     table-driven main() wire format; the separate fuzz_target.c single-exit
     decode skeleton.
   - **Parity notes vs the Rust backend** (unanchored): one plan two
     renderings + placement rule; per-area differences (type mapping D8 vs
     u128, tagged unions vs enums, expression lowering uint64 intermediates vs
     u128 wrapping, monitors identical rendering + shared snake naming, firing
     order, MC engine container-independence + two-phase expansion, JSON
     embedded copies, SMT shared spec + one libz3 build, fuzz LCG constants,
     test names + coverage line); the pinning suites.
   Run `cargo test --test dev_docs_test` after writing. No source edits
   (fragments already owned in 10.1).
2. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (build/test/clippy/fmt + workspace; counts unchanged: 535/535
   root, 559/559 workspace); plan row 10.2 -> DONE; check off this subtask's
   boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 10.2)

- [x] `docs/dev/backend_c.md`: both TBD sections filled in place, no TBD
      remains anywhere in the doc; the new "Parity notes vs the Rust backend"
      section inserted before "Changing the C backend"; every technical
      statement carries a resolving doc-to-code reference (gate check 1);
      anchors + headings byte-stable.
- [x] `docs/llm/backend_c.md`: unchanged unless an inaccuracy is found (it was
      filled in full by 10.1).
- [x] No source edits at all (fragments were owned in 10.1); all cargo counts
      unchanged from the task-10.1 tree (535/535 root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 10.2 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 10.2)

- No backend behavior changes of any kind; no new dependencies.
- No restatement of C.md, CODEGEN.md, DEPENDENCIES graph, codegen_core, or
  backend_rust content (link only); no edits to other docs (START.md/KB.md/
  BUILD.md pointer sync = task 14).
- No mc_plan.rs / smt_* deep dives (task 11.1/11.2) - the doc references them,
  never describes them.
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-10-31) for plan group 10 | User model: one task file per plan group ("execute 10.1" = subtask of task file 10). Written for subtask 10.1 only; subtask 10.2 (c_mc + c_test deep dive + parity notes) will be appended at execution, following the PHASE_8_TASK_9.md pattern (9.2 appended after 9.1 completed). |
| Subtask 10.2 section appended at execution (2026-10-31) | Following the PHASE_8_TASK_9.md pattern (9.2 appended after 9.1 completed): Goal / Context with verified source material - item maps of c_mc.rs (2836 lines, no unit tests) and c_test.rs (1035 lines, no unit tests) measured 2026-10-31, no-duplication rule vs C.md section 5 + codegen_core + backend_rust, consumers/pins inventory / Planned actions / Exit criteria / Non-goals. The Class B card was already filled in full by 10.1 (all four files), so 10.2 is doc-only with no source edits. |
