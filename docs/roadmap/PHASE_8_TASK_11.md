# Phase 8 Task 11 - Model-checking engine deep dive (`docs/dev/mc_engine.md`)

Plan rows: 11.1 (exact + liveness paths, `mc_plan.rs` + `ref_mc.rs`) and
11.2 (SMT path, `smt_plan.rs` + `smt_emit.rs`, + diagnostics). One task file
per plan group; subtask sections are appended at execution.

## Subtask 11.1 - Exact + liveness paths deep dive

### Goal

Fill in `docs/dev/mc_engine.md` (Class A deep dive) for the exact + liveness
paths: the codegen-time `McPlan` (`src/mc_plan.rs`) and the brute-force
reference checker (`src/ref_mc.rs`) - plus the Class B card
`docs/llm/mc_engine.md` (per the confirmed scope decision: each deep-dive
task fills both files; the card's TBD names all four engine files, so 11.1
writes it in full with the SMT files at one-line-role level - their detail is
11.2's fragment). The dev doc gains new anchored sections for 11.1's scope;
the SMT section is added by 11.2.

### Context

- **Doc state**: `docs/dev/mc_engine.md` is a 15-line skeleton - one Overview
  section (`dev:mc-engine-overview`) whose TBD (task 11.1) describes the whole
  family at a high level; no other sections exist yet. `docs/llm/mc_engine.md`
  is a 14-line card skeleton with budget 150 and a single TBD (task 11.1).
- **Fragment structure** (house pattern: one fragment per major `##` section):
  keep `dev:mc-engine-overview` for the Overview; add four new sub-fragments:
  `dev:mc-engine-plan` (codegen-time plan, mc_plan.rs),
  `dev:mc-engine-exact` (exact bounded checker, ref_mc.rs),
  `dev:mc-engine-liveness` (liveness decomposition + analysis),
  `dev:mc-engine-verify` (counterexample verification + minimal labels).
  11.2 will add the SMT section after `dev:mc-engine-verify`.
- **No duplication with existing fragments**: docs/dev/codegen_core.md owns
  the shared plan DATA types consumed here only as inputs (MonitorPlan,
  build_variants, RepValue, AssumptionSelection); docs/dev/backend_rust.md +
  docs/dev/backend_c.md own the GENERATED engines that mirror ref_mc's
  algorithms (their deep dives already describe EXACT_ENGINE_C / LIV_ENGINE_C
  etc. - this doc describes the REFERENCE implementation and the codegen-time
  plan, and links to those for the generated twins); SEMANTICS.md owns the
  semantics (8.3 unexpected, 8.4 deadlock, 9.2 config encoding widths, 9.5
  stop rule, 6.4/6.5/6.6 liveness) - cite, never restate; docs/dev/monitor_
  compiler.md (task 11.3) owns the shared MonitorPlan compiler - ref_mc's OWN
  monitor compiler (Mon/MonSt) is described here as an independent
ground-truth implementation.
- **Verified source material (measured 2026-10-31; line numbers for
  navigation only - doc references use item names)**:
  - `src/mc_plan.rs` (1130 lines, 13 unit tests): ValueDomain (41: Bool /
    Int{max} / Enum{name, variants} - canonical ascending enumeration domain),
    FieldSpec (54: concrete flag + domain XOR representative), VariantInputSpec
    (67), InputAbstraction (80), VarSlot (90), MonitorNodeState (109: node_id
    preorder + pending/remaining/antecedent bit slots; sticky failed/
    trigger_step deliberately NOT part of the configuration - invariant on all
    live configs), MonitorStateLayout (134), ConfigEncoding (150: state_bits +
    var_slots + monitor blocks, total_bytes = ceil(total_bits/8) - the buffer
    both engines hash for the visited set), StateTransitionOrder (179),
    TraversalSpec (187), McPlan (208) + McPlan::compute (218: pure function of
    module + assumption selection; abstraction via referenced_fields, encoding
    via monitor_layouts + config_encoding, traversal via traversal_spec),
    ceil_log2 (262), type_bits (275), value_domain (292), referenced_fields
    (322: a field is concrete iff referenced by a transition guard / emit arg /
    assignment value binding OR an input_is pattern with CONSTRAINING field
    patterns - wildcards/binds constrain nothing), used_identifiers (368),
    collect_paths (384), collect_input_is_refs (399: walks all formula shapes
    incl. liveness bodies), monitor_layouts (451: properties then SELECTED
    assumptions = aggregate stepping order; out-of-subset formulas skipped,
    backends emit the identical warning), layout_for (471), walk_node (487:
    preorder node ids matching the backends' n{id} naming; Obligation bound>=1
    -> 1 pending bit + ceil_log2(bound+1) remaining bits, DelayedCheck -> 1
    antecedent bit, everything else stateless), config_encoding (533),
    traversal_spec (574: value_domains mirror the abstraction; state_orders =
    effective priority descending then declaration order via stable sort;
    assumption_labels for --assumes selection D-P4-6); LivenessObligation
    (621: Eventually{body} / Until{left, right}), SkeletonLeaf (628: Safety /
    Obligation(index)), Skeleton (639), LivenessPropertySpec (649),
    LivenessDecomposition (658: per-property Option + diagnostics),
    boolean_over_atoms (673: Expr/InputIs/OutputHas atoms under !/&&/||/->
    only), LivenessDecomposition::compute (690), contains_liveness (716),
    decompose_top (737: obligations may appear anywhere in the boolean
    skeleton; temporal operators inside a liveness property are rejected with
    a diagnostic - deeper nesting vs split-the-safety-part).
  - `src/ref_mc.rs` (4971 lines, own test module): Val (56) / RType (76) /
    eval_expr (268: u128 wrapping arithmetic + width masks, the reference
    evaluator), Sig (411) / InputVal (424) / OutputVal (433) / Sample (441) /
    eval_sig (471), FlatVariant (496) / flatten (505); Range (619: anchor
    window + shifted()), Mon (643: Check/Delayed/Obligation/And/Or - own type,
    same shapes the documented semantics require), compile_formula (671:
    Err = outside the monitor subset, identical classification to the
    generated engines' compiler), compile_signal (675), compile_check (697),
    MonSt (824: per-node runtime state incl. sticky failed + trigger_step),
    mon_init (861), mon_step (890: returns CURRENT-SAMPLE failure status -
    per-sample not sticky, essential for Or/And composition), min_trigger
    (1002), mon_key / mon_key_node (1016/1022: persistent projection - the
    CfgKey monitor part; remaining packed only while pending), mon_bits (1048);
    referenced_fields (1078, own copy of the mc_plan analysis over FlatVariant)
    + field_domain (1199) + representative_value (1221) + all_inputs (1237:
    canonical enumeration - variants in declaration order, per-field value
    lists (domain or single representative), Cartesian product outermost
    first); Effect (1285: tid usize::MAX = on_error), pattern_bindings (1295),
    match_effect (1322: deterministic FIRST-MATCH in effective-priority stable
    order + on_error fallback with no bindings; internal eval failure = no
    match for validated specs), input_matches_variant (1431); Leaf (1499) /
    Skel (1508) / LivProp (1518: label + formula_text + obligations + skeleton
    + leaves) + decompose_top (1559, own copy of the mc_plan decomposition)
    + decompose_properties (1611); own formula/expr printer (1642-1882:
    print_formula_ctx / print_expr_min with precedence table - byte-identical
    to the generated engines' embedded formula strings); RefConfig (1884:
    max_states / timeout_sec / assume_arg; defaults from codegen_mc), IoRec
    (1907) / StepRec (1916) / CeData (1928: Trace{trigger_step, steps} /
    Lasso{formula_text, cycle_start, note, steps} / Deadlock{depth, steps}),
    RefResult (1951: every field both report renderings need; deadlock_mons =
    persistent monitor projection of the deadlocked config so an external
    verifier can re-derive SEMANTICS 8.4), BNode (1991), CfgKey (2001:
    STRUCTURED key, no bit-packing - deliberate; behavior must not depend on
    the container) + val_key (2011) + bnode_key (2019: enums as variant index,
    bools 0/1, bits raw; monitors via mon_key), run (2055: own monitor
    compilation + liveness decomposition; config widths from the documented
    SEMANTICS 9.2 formulas for statistics only; initial config with constant-
    initializer check; sample-0 stepping - assumption violation = clean error
    (generated binaries panic), property violation = FAIL with empty trace;
    level-synchronous BFS - per-config timeout check (one level can exceed the
    budget on its own), match_effect per input, monitors stepped at d+1 with
    assumptions FIRST (a pruned successor is never reported as a property
    violation), first-violation-wins stop AFTER the per-level loop (SEMANTICS
    9.5), visited at ENQUEUE time + max_states cap, adj edges recorded for
    every success incl. to visited configs, complete flag, deadlock = no
    successor at all with deadlock_mons captured; liveness analysis only when
    no safety failure/deadlock - a witness inside a truncated subgraph is
    SOUND; status taxonomy Running/Pass/StateLimit/Timeout/Fail mapped to the
    shared ExactStatus), path_to (2467), trace_steps (2511); LivWitness (2548:
    lasso edges + cycle_start), LeafSig (2555) / leaf_signals (2561) /
    skel_eval (2587), liv_analyze (2599: position graph - position 0 = initial
    sample, one position per explored edge, EDGE-based labels SEMANTICS 6.5;
    per-position label bits via leaf_bits; product reachability BFS with parent
    pointers over (position, automaton state) pairs; iterative Tarjan SCCs;
    on-cycle = non-trivial SCC member or self-loop singleton; falsifying limit
    vector search q ascending in discovery order - first witness pair on a
    cycle with that q; lasso = product path to the witness pair + wpid's own
    edge first + DFS closing path back to wpid with GLOBAL visited pruning -
    path-only pruning re-explores diamond subgraphs exponentially, Phase 6
    task 4 finding), prod_delta (2884: mixed-radix per-leaf progress automata
    - safety 0=ok/1=bad, eventually 0=wait/1=done, until 0=ok/1=sat/2=bad,
    all one-way so every state sequence is eventually constant = limit
    vector), leaf_good (2932), leaf_bits (2942: bit 2k = atom A, bit 2k+1 =
    until-right), liv_note (2958: human explanation of the falsified leaves);
    render_val / render_vars / render_input / render_output(s) (3064-3131:
    Rust Debug form - State:: prefix added by renderers, Vars { ... },
    Input::Variant { ... }, [Output::X, ...]), render_text (3133) /
    render_text_impl (3139: byte-identical FORMAT to the generated exact-mode
    binaries' default report - status line, shared exact_summary_line,
    statistics block, per-reason counterexample header, "counterexample
    trace:" body), to_json_report (3279: canonical JSON document, same model +
    key order as the generated engines' --counterexample-format json; bit<64>
    values above i64::MAX documented unreachable for byte-parity-relevant
    specs); minimal_violation_labels (3416: phase 1 canonical run finds the
    first violation in traversal order, phase 2 re-runs BFS keeping levels
    until first_step-1 is fully expanded collecting every label confirmed at
    the minimal step; liveness witnesses from phase-1's graph included when
    their cycle start <= global min; sorted output), collect_min_labels
    (3447); initial_config (3620), json_val (3648), io_to_input (3664) /
    io_to_output (3698), step_trace (3737: replay inputs on a fresh module,
    producing the Sample sequence), check_initial (3773), check_step_config
    (3811: one replayed step must match the CE's claimed from/to state, vars,
    outputs), property_confirms_at (3882: step the label's monitor over the
    replayed samples - the FIRST confirmed failure must be exactly at the
    claimed step), verify_lasso (3925: configuration closure + product-
    automaton closure + falsifying limit vector - the same acceptance
    condition liv_analyze uses for end components), mon_from_key (4003:
    reconstruct a MonSt from its persistent projection), verify_deadlock
    (4077: domain checks + no-surviving-successor judgment re-derived from the
    carried monitor projection), verify_counterexample (4220: dispatch on
    (reason, ce kind) - replay + per-kind verification).
  - Shared types consumed here: `src/codegen_mc.rs::ExactStatus` (151:
    PassComplete/Fail/UnknownStateLimit + wire/human/exit_code),
    `src/codegen_mc.rs::FailReason` (163: PropertyViolation/LivenessViolation/
    Deadlock + wire names), `src/codegen_mc.rs::exact_summary_line` (221: the
    machine-readable summary line, parity-checked across backends),
    `src/codegen_mc.rs::parse_assume_selection`,
    `src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES` /
    `src/codegen_mc.rs::EXACT_DEFAULT_TIMEOUT_SEC`,
    `src/codegen_mc.rs::CounterexampleFormat` + McJsonReport model.
  - CLI surface: `src/mc_ref_cmd.rs` (rfs mc-ref - in-process ref_mc, no
    compilation; flags --max-states/--timeout-sec/--assumes/
    --counterexample-format; exit codes mirror the generated binaries 0/1/2;
    fuzz-mode + threading flags documented NOT applicable) - deep dive is
    task 12.1's CLI doc; this doc links only.
  - Consumers / pins (verified): tests/ref_mc_diff_test.rs (4 tests -
    ref-MC vs BOTH generated engines: status/states/max_depth agreement,
    label in the minimal-violation set, CE replay through
    verify_counterexample; transitions deliberately NOT asserted cross-engine),
    tests/ref_mc_sweep_test.rs (byte-parity sweep vs the generated Rust MC -
    exit code + exact stdout in text AND json, fail-fast parallel),
    tests/ref_mc_diag_test.rs (non-fail-fast mismatch enumerator over all
    tractable examples), tests/backend_parity_test.rs (cross-backend,
    includes liveness parity + JSON report parity), tests/benchmark_mc_test.rs.
- **Entry format / gate rules / bookkeeping**: same as subtask 8.1 in
  PHASE_8_TASK_8.md (house style with per-section Code: lines; doc-to-code
  reference rules - verify every reference against a declaration line; run
  dev_docs_test after each action, docs_md_test at task end; plan row ->
  DONE, boxes checked, one TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-31): all items listed above plus `src/codegen_fsm.rs::
  build_variants`, `src/codegen_fsm.rs::RepValue`, `src/codegen_fsm.rs::
  representative_values`, `src/codegen.rs::AssumptionSelection`,
  `src/monitor.rs::compile_formula`, `src/monitor.rs::MonitorPlan`,
  `src/mc_ref_cmd.rs::run_mc_ref`, file-level `tests/ref_mc_diff_test.rs`,
  `tests/ref_mc_sweep_test.rs`, `tests/ref_mc_diag_test.rs`,
  `tests/backend_parity_test.rs`, `tests/benchmark_mc_test.rs`. Non-reference
  backticks (safe): SEMANTICS.md, CODEGEN.md, DESIGN.md, shell commands, bare
  identifiers, generated-code item names without a file-path prefix.

### Planned actions

1. **Class A deep dive - `docs/dev/mc_engine.md`**: replace the Overview TBD
   in place (anchor stays byte-stable) and add four new anchored sections:
   - **Codegen-time plan** (`dev:mc-engine-plan`): McPlan::compute as a pure
     function of module + assumption selection; input abstraction (concrete
     iff referenced - the five reference kinds, constraining-pattern rule);
     configuration encoding (state bits + var slots + monitor blocks,
     total_bytes = the visited-set buffer, sticky flags deliberately excluded);
     traversal spec (canonical input order, effective-priority firing order,
     assumption labels); liveness decomposition (obligations anywhere in the
     boolean skeleton, diagnostics for uncheckable shapes).
   - **Exact bounded checker** (`dev:mc-engine-exact`): the reference value /
     expression evaluator; the OWN monitor compiler (Mon/MonSt, per-sample
     failure status, persistent projection); canonical all_inputs enumeration;
     match_effect first-match + on_error; run() level-synchronous BFS (sample-
     0 handling, assumptions-first pruning, SEMANTICS 9.5 stop rule, deadlock
     + deadlock_mons, structured CfgKey container-independence);
     render_text / to_json_report pinned formats.
   - **Liveness analysis** (`dev:mc-engine-liveness`): position graph with
     edge-based labels; product automaton reachability; iterative Tarjan SCCs;
     falsifying limit vector search (q ascending, discovery order); lasso
     witness construction with global visited pruning; liv_note; truncated-
     graph soundness.
   - **Counterexample verification + minimal labels** (`dev:mc-engine-verify`):
     verify_counterexample dispatch (replay through step_trace + per-kind
     checks), property_confirms_at first-confirmation rule, verify_lasso
     acceptance condition, verify_deadlock monitor-projection re-derivation,
     minimal_violation_labels two-phase collection; the differential-testing
     role (ref_mc_diff_test / ref_mc_sweep_test / ref_mc_diag_test).
   Run `cargo test --test dev_docs_test` after writing.
2. **Class B card - `docs/llm/mc_engine.md`**: fill the TBD in place (budget
   150): responsibility, key-items table covering all four engine files (SMT
   at one-line-role level - detail is 11.2), invariants that must not break
   (parity by construction, container-independence, pinned formats, soundness
   rules), change impact + deep-dive pointers.
3. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (build/test/clippy/fmt + workspace; counts unchanged: 535/535
   root, 559/559 workspace); plan row 11.1 -> DONE; check off this subtask's
   boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 11.1)

- [x] `docs/dev/mc_engine.md`: Overview TBD replaced in place + four new
      anchored sections (plan / exact / liveness / verify) with full content;
      no TBD remains for 11.1's scope; every technical statement carries a
      resolving doc-to-code reference (gate check 1); the existing anchor is
      byte-stable.
- [x] `docs/llm/mc_engine.md`: card filled in place within the 150-line budget
      (all four engine files at key-item level; SMT detail deferred to 11.2).
- [x] Source edits limited to the two `//! Docs:` owner lines (mc_plan.rs,
      ref_mc.rs - doc comments only); all cargo counts unchanged from the
      task-10.2 tree (535/535 root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 11.1 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 11.1)

- No backend behavior changes of any kind; no new dependencies.
- No SMT path content in the dev doc (that is 11.2's fragment) - the Overview
  may map it at one-line level with a forward pointer.
- No restatement of SEMANTICS.md, codegen_core, backend_rust, or backend_c
  content (link only); no edits to other docs (START.md/KB.md/BUILD.md pointer
  sync = task 14).
- No monitor_compiler deep dive (task 11.3) - ref_mc's OWN compiler is
described here as ground truth; the shared MonitorPlan compiler is linked.
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Subtask 11.2 - SMT path deep dive (added at execution, see Task corrections)

### Goal

Fill in `docs/dev/mc_engine.md` (Class A deep dive) for the SMT bounded mode:
the backend-neutral spec builder (`src/smt_plan.rs`) and the shared emitter
(`src/smt_emit.rs`) - plus the v1 scope rejection diagnostics. The dev doc gains
one new anchored section after `dev:mc-engine-verify`; the Class B card
`docs/llm/mc_engine.md` was already filled in full by 11.1 with the SMT files at
one-line-role level pointing to this fragment, so no card change is needed here.

### Context

- **Doc state**: `docs/dev/mc_engine.md` after 11.1 = Overview + four sections
  (plan / exact / liveness / verify); the Overview already carries the forward
  pointer "details in dev:mc-engine-smt (task 11.2)". `docs/llm/mc_engine.md`
  is complete (56 lines, budget 150) - its two SMT key-item rows and the
  Deep-dive paragraph already point at dev:mc-engine-smt; per the house
  convention the "(task 11.2)" markers stay (same as "deep dive task 10.2" in
  backend_c.md after 10.2 completed).
- **Fragment structure**: add ONE new sub-fragment `dev:mc-engine-smt` after
  `dev:mc-engine-verify`. ORPHAN RULE: the new ID goes on the owner files'
  `//! Docs:` lines in the same action. Exact target lines:
  - `src/smt_plan.rs` line 13 becomes:
    `//! Docs: dev:mc-engine-overview, dev:mc-engine-smt, dev:algorithms-mc-engine, llm:mc-engine`
  - `src/smt_emit.rs` line 19 becomes:
    `//! Docs: dev:mc-engine-overview, dev:mc-engine-smt, dev:algorithms-mc-engine, llm:mc-engine`
- **No duplication with existing fragments**: ALGORITHMS.md
  `dev:algorithms-mc-engine` owns the L1/L2/L3 algorithm views incl. the SMT
  path (link only); docs/dev/backend_rust.md owns SMT_ENGINE_RUST (the
  spec-independent k-step unrolling engine, verdicts, witness format) and
  docs/dev/backend_c.md owns SMT_ENGINE_C - this doc describes the SHARED spec
  + emitter and links to those for the engines; DESIGN.md sections 11/22
  D-P4-12 own the decision record; TOOLCHAIN.md section 5 owns the z3 5.3.1
  install facts (cite, never restate).
- **Verified source material (measured 2026-10-31; line numbers for navigation
  only - doc references use item names)**:
  - `src/smt_plan.rs` (691 lines, no unit tests): Se (21: Lit{value,is_bool} /
    State / Var(idx) / Field{idx,width,is_bool class-relative} / Not / BitNot /
    Neg / Bin(Sb,l,r)), Sb (49: 16-operator subset of the IR BinOp, same
    names), Sf (70: Se/Not/And/Or/Implies boolean layer), SmtVar (79: name,
    width 1 for bool, is_bool witness-display only, literal init), SmtClass
    (94: display name = plan unique_name; concrete fields only (name,width,
    is_bool)), SmtTrans (103: state/next_state/class indices, guard Option<Se>,
    assignments - vars absent unchanged), SmtPropKind (117: Always/Eventually),
    SmtProp (123), SmtSpec (131: states declaration order, initial_state,
    state_bits = max(1, ceil_log2(n)), vars, classes, transitions in FIRING
    order, properties), ExprCtx (148: var_idx + field name -> Some(concrete
    idx)/None abstracted + state_variant_idx; resolution order state -> payload
    bindings -> vars -> state variants, exactly the validator's), ceil_log2_pub
    (157) / ceil_log2 (164), build_smt_spec (171: module + McPlan -> SmtSpec or
    shared error string - v1 scope rejections: assumptions, on_error,
    enum-typed vars, non-literal inits, init overflow, concrete fields > 64
    bits (saturated u64::MAX domain), non-power-of-two domains, enum-typed
    payload fields, wildcard patterns over multi-variant groups, unknown
    state/next/var refs, top-level always/eventually only, formula
    expr/not/and/or/implies only, no enum paths), variant_name (419),
    formula_tag (427: diagnostic tag for rejected formula shapes), fi_concrete
    (443), convert_expr (451: IR Expr -> Se; abstracted fields substituted as
    representative literals via codegen_fsm::RepValue), convert_formula (564:
    properties get NO payload context - empty variant view), SeType (601:
    Bool/Bit(w)/IntLit, width() IntLit->8), se_type (625: bottom-up with
    EXACTLY the validator's rules), bit_op_type (679: (Bit(a),Bit(b)) requires
    a==b debug_assert; IntLit lifts; (IntLit,IntLit) -> Bit(8)).
  - `src/smt_emit.rs` (1849 lines; mod tests 1286-1849 with 5 tests):
    global_field_index (25: all classes in order, fields within a class in
    order - the fld array index), se_uses_step (39) / sf_uses_step (49: builder
    i parameter used or _i - warning-free code), RustRender (61: lines +
    counter; push returns eN binding name) / CRender (628), render_se_rust
    (92: sequential let-bindings; expected width hint lifts literals; class
    resolves fields; bool_as_bv at assignment top level), finish_bool_rust
    (172: ite(b, 1, 0) BV(1) wrap - structurally identical in both backends),
    produces_bool (182: dispatch to bool vs bit path), cmp_widths_full (198:
    exact-MC full-value comparison semantics - literals lift to the sibling's
    width; if a literal does not fit BOTH sides widen and the non-literal side
    is zero-extended; arithmetic needs no widening - ring homomorphism),
    widen_rust (243) / widen_c (252: concat with zeros), render_bool_bin_rust
    (261: logical ops via b_and/b_or wrappers; comparisons bvult/bvule/bvugt/
    bvuge or bool-ordering encodings Lt = not-a and b, Le = not(a and not-b),
    Gt = not-b and a, Ge = not(b and not-a)), render_bit_bin_rust (340:
    bvadd/bvsub/bvmul/bvand/bvor/bvxor at se_type width), render_sf_rust
    (377), rust_str_list (402), emit_rust_spec (408: constants STATE_BITS/
    INITIAL_STATE/STATES/VAR_NAMES/VAR_WIDTHS/VAR_INITS/VAR_IS_BOOL/CLASS_
    NAMES/N_CLASSES/CLS_BITS/FIELD_NAMES/FIELD_WIDTHS/FIELD_IS_BOOL/FIELD_POS/
    TRANS (state,next,class) firing order/PROP_NAMES/PROP_KINDS 0=always/
    1=eventually + use z3::ast::{BV, Bool} + expr_t{t}_guard(i) per transition
    (None guard -> true) + asgn_dummy only when needed + expr_t{t}_asgn_{j}(i)
    per assignment + expr_p{p}(i) per property + ASGN_MAP[t][j] +
    guard_fns()/asgn_fns()/prop_fns() function-pointer tables), shift_amt_width
    (652: BV keeps its own width; IntLit lifts to at least wl and enough bits
    to hold the value - no truncation, oversized amounts stay detectable),
    render_shift_rust (671) / render_shift_c (718: zero-extend amount to max(
    amount width, wl), guard bvult(amt, wl), truncate amount to wl bits
    lossless under the guard, ite(shift, 0)), render_se_c (761: C mirror;
    leaves are s / v[j] / fld[g] parameters), finish_bool_c (837),
    render_bool_bin_c (850: two-arg Z3_mk_and/Z3_mk_or over 2-element arrays -
    counterpart of b_and/b_or), render_bit_bin_c (938), render_sf_c (975),
    c_str_list (1006), emit_c_spec (1012: includes first; #define STATE_BITS/
    INITIAL_STATE/N_VARS/N_CLASSES/CLS_BITS/N_FIELDS/N_TRANS/N_PROPS + static
    arrays with dummy slots for empty VAR_WIDTHS/FIELD_NAMES/FIELD_WIDTHS/
    FIELD_IS_BOOL (zero-size arrays invalid in C - engine loops over N_VARS/
    N_FIELDS, never touches the dummy) + z3_num helper + expr_t{t}_guard(c,s,
    v,fld) builders with (void) casts + asgn_dummy + expr_p{p} + ASGN_MAP +
    GUARD_FNS/ASGN_FNS/PROP_FNS tables).
  - Tests (1286): test_spec (1292: 2 states, counter bit<8>/flag bool/big
    bit<16>, classes Go_F(x,y,ok)/Noop(), 4 transitions covering guards/
    assignments/shifts/oversized literals, 2 properties), bin_ops (1501),
    cmp_widths_full_fits_and_widens (1509), shift_amt_width_rules (1572),
    normalize_rust (1604) / normalize_c (1689: backend syntax stripped -> z3 op
    token sequence; leaves map to the same s/v_j/fld_g tokens), fn_body
    (1759), rust_and_c_render_identical_op_sequences (1770: D-P4-12 core
    guarantee - every builder renders to the SAME z3 op sequence in Rust and
    C), oversized_literal_comparison_widens (1796), shift_rendering_paths
    (1819).
  - Consumers / pins (verified): src/backends/rust_mc.rs line 44 build_smt_spec
    .map_err(|e| vec![e])? (shared error string = codegen error, exit 1) +
    line 53 emit_rust_spec; src/backends/c_mc.rs line 47 / line 56 the C
    mirror. SMT_ENGINE_RUST / SMT_ENGINE_C own the engine (backend docs). z3
    5.3.1 one build: TOOLCHAIN.md section 5 (z3 crate 0.21 / z3-sys 0.13,
    feature smt).
- **Entry format / gate rules / bookkeeping**: same as subtask 11.1 in this
  file (house style with per-section Code: lines; doc-to-code reference rules -
  verify every reference against a declaration line; run dev_docs_test after
  each action, docs_md_test at task end; plan row -> DONE, boxes checked, one
  TASKS.md entry, commit per action).
- **Verified doc-to-code references for this subtask** (declaration lines
  checked 2026-10-31): all items listed above plus `src/mc_plan.rs::McPlan`,
  `src/codegen_fsm.rs::RepValue`. Non-reference backticks (safe): SEMANTICS.md,
  DESIGN.md, TOOLCHAIN.md, shell commands, bare identifiers, generated-code
  item names without a file-path prefix (STATE_BITS, TRANS, ASGN_MAP,
  GUARD_FNS, expr_t0_guard, s_<i>, v_<i>_<j>, ev_<i>, fld_<i>_<g>, z3_num,
  b_and, b_or, SMT_ENGINE_RUST/C, test names).

### Planned actions

1. **Class A deep dive - `docs/dev/mc_engine.md`**: add one new anchored
   section after `dev:mc-engine-verify`:
   - **SMT bounded mode** (`dev:mc-engine-smt`): shared spec (build_smt_spec as
     module + McPlan -> SmtSpec; the five spec tables; firing order;
     abstraction reuse with representative-literal substitution); expression
     model (Se/Sb/Sf, validator-order name resolution, no payload context in
     properties); typing (SeType/se_type with exactly the validator's rules);
     v1 scope diagnostics (the shared error strings, identical in both
     backends, codegen exit 1); shared emitter (spec-section constants +
     builder functions + tables); leaf-variable convention (fixed z3 constant
     names, hash-consing parity); rendering (width semantics: full-value
     comparisons with zero-extension, shift guard/truncate, bool-ordering
     encodings, BV(1) wrap at assignment top level); warning-free code (_i
     params, asgn_dummy, C dummy slots); engine + verdicts (link to backend
     docs, k-liveness, incomplete PASS labeling, one libz3 build,
     --smt-bound); parity pins (the emitter's own 5 tests incl.
     rust_and_c_render_identical_op_sequences).
   Run `cargo test --test dev_docs_test` after writing.
2. **Fragment owners**: add `dev:mc-engine-smt` to the `//! Docs:` lines of
   `src/smt_plan.rs` (line 13) and `src/smt_emit.rs` (line 19) in the same
   action (ORPHAN RULE). Doc comments only.
3. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (build/test/clippy/fmt + workspace; counts unchanged: 535/535
   root, 559/559 workspace); plan row 11.2 -> DONE; check off this subtask's
   boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 11.2)

- [x] `docs/dev/mc_engine.md`: new anchored section dev:mc-engine-smt after
      dev:mc-engine-verify with full content (shared spec, expression model,
      typing, v1 scope diagnostics, shared emitter, leaf-variable convention,
      rendering width/shift semantics, warning-free code, engine + verdicts
      links, parity pins); no TBD remains; every technical statement carries a
      resolving doc-to-code reference (gate check 1); all existing anchors
      byte-stable.
- [x] Fragment owners: `dev:mc-engine-smt` on the `//! Docs:` lines of
      src/smt_plan.rs + src/smt_emit.rs in the same action as the section; no
      orphaned fragment (gate check 4).
- [x] Source edits limited to the two `//! Docs:` owner lines (doc comments
      only); all cargo counts unchanged from the task-11.1 tree (535/535 root,
      559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      build/test/clippy/fmt (+ --workspace).
- [x] Plan row 11.2 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 11.2)

- No backend behavior changes of any kind; no new dependencies.
- No SMT engine internals (k-step unrolling, verdicts, witness format) - that
  is SMT_ENGINE_RUST / SMT_ENGINE_C in docs/dev/backend_rust.md +
  docs/dev/backend_c.md (link only).
- No restatement of ALGORITHMS.md dev:algorithms-mc-engine L1-L3 views,
  DESIGN.md D-P4-12, or TOOLCHAIN.md z3 facts (link/cite only); no edits to
  other docs (START.md/KB.md/BUILD.md pointer sync = task 14).
- No Class B card change - docs/llm/mc_engine.md was filled in full by 11.1
  with the SMT rows pointing at this fragment; per house convention the
  "(task 11.2)" markers stay.
- No monitor_compiler deep dive (task 11.3); no CLI doc (task 12.1).
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Task file created at execution (2026-10-31) for plan group 11 | User model: one task file per plan group ("execute 11.1" = subtask of task file 11). Written for subtask 11.1 only; subtask 11.2 (SMT path + diagnostics) will be appended at execution, following the PHASE_8_TASK_9.md pattern (9.2 appended after 9.1 completed). |
| Subtask 11.2 appended at execution (2026-10-31) | Per the correction above: subtask sections are written when executed, following the PHASE_8_TASK_9.md pattern (9.2 appended after 9.1 completed). Written with verified source material measured 2026-10-31 (item maps of both SMT files: smt_plan.rs 691 lines no unit tests; smt_emit.rs 1849 lines, mod tests at 1286 with 5 tests), fragment plan with exact owner-line targets (smt_plan.rs line 13, smt_emit.rs line 19), no-duplication rule vs ALGORITHMS.md dev:algorithms-mc-engine + the backend SMT engine sections. |