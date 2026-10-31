# RFS Model-Checking Engine (Class A)

The model-checking engine family: the exact bounded checker plan, the shared
SMT encoding + emitter for the z3-based mode, and the brute-force reference
checker used as ground truth.

<a id="dev:mc-engine-overview"></a>
## Overview

The model-checking engine family has three parts, plus a shared outcome
taxonomy:

- **Codegen-time plan** (`src/mc_plan.rs`): `src/mc_plan.rs::McPlan` is
  computed ONCE in the Rust tool from the validated IR and embedded as
  constants/tables in both generated MC binaries, so the Rust and C exact
  engines are identical by construction (mirrors the Phase 3 MonitorPlan
  pattern). Three tables: input abstraction (D-P4-3), configuration encoding
  (D-P4-2), traversal spec (D-P4-2); plus `src/mc_plan.rs::LivenessDecomposition`
  for liveness properties (D-P4-9).
- **Exact + liveness runtime**: the brute-force reference checker
  (`src/ref_mc.rs::run`) is an independent implementation of SEMANTICS.md -
  ground truth behind `rfs mc-ref` (`src/mc_ref_cmd.rs::run_mc_ref`, in-process,
  no compilation) and differential testing. Both generated backends mirror its
  algorithms (Rust: docs/dev/backend_rust.md; C: docs/dev/backend_c.md).
- **SMT bounded mode**: the backend-neutral spec (`src/smt_plan.rs`) rendered
  by the shared emitter (`src/smt_emit.rs::emit_rust_spec`,
  `src/smt_emit.rs::emit_c_spec`) for parity by construction - details in
  dev:mc-engine-smt (task 11.2).

Shared outcome taxonomy (`src/codegen_mc.rs`): `src/codegen_mc.rs::ExactStatus`
(PassComplete / Fail / UnknownStateLimit, with wire names, human text, and exit
codes 0/1/2), `src/codegen_mc.rs::FailReason` (PropertyViolation /
LivenessViolation / Deadlock), and `src/codegen_mc.rs::exact_summary_line` -
the machine-readable summary line, parity-checked across backends.

Code: `src/mc_plan.rs::McPlan`, `src/mc_plan.rs::LivenessDecomposition`,
`src/ref_mc.rs::run`, `src/mc_ref_cmd.rs::run_mc_ref`,
`src/codegen_mc.rs::ExactStatus`, `src/codegen_mc.rs::FailReason`,
`src/codegen_mc.rs::exact_summary_line`, `src/smt_plan.rs`,
`src/smt_emit.rs::emit_rust_spec`, `src/smt_emit.rs::emit_c_spec`


<a id="dev:mc-engine-plan"></a>
## Codegen-time plan (`src/mc_plan.rs`)

**Entry point.** `src/mc_plan.rs::McPlan::compute`(module, assumptions) is a
pure function of the validated module + assumption selection - no environment
reads, no randomness. It produces three tables: `abstraction` (one
`src/mc_plan.rs::VariantInputSpec` per event variant, same declaration order as
`src/codegen_fsm.rs::build_variants`), `encoding` (`src/mc_plan.rs::ConfigEncoding`
from monitor layouts), and `traversal` (`src/mc_plan.rs::TraversalSpec`).

**Input abstraction (D-P4-3).** A payload field is CONCRETE iff it is
referenced by any of: a transition guard, an emit argument, or an assignment
value - via pattern binding names (`src/mc_plan.rs::used_identifiers` +
`src/mc_plan.rs::collect_paths` collect the single-segment path identifiers in
those contexts); or a property/assumption `input_is` pattern with CONSTRAINING
field patterns (literals / enum-variant matches - wildcards and binds constrain
nothing). `src/mc_plan.rs::collect_input_is_refs` walks every formula shape,
including liveness bodies (their boolean atoms may constrain input fields too).
Concrete fields carry their canonical `src/mc_plan.rs::ValueDomain` (Bool:
false < true; Int: 0..=max ascending, max saturated to u64::MAX for width >= 64
which exact MC cannot enumerate anyway; Enum: declaration order); abstracted
fields collapse to a single representative value
(`src/codegen_fsm.rs::representative_values`).

**Configuration encoding (D-P4-2).** config = (state index, packed variables,
monitor transition states), bit-packed little-endian into total_bytes =
ceil(total_bits/8) - the buffer both engines hash for the visited set.
state_bits = ceil_log2(state_count); `src/mc_plan.rs::VarSlot`s in declaration
order with type-bit widths (bool 1, bit<N> N, enum ceil_log2(variants));
monitor blocks in aggregate stepping order - properties in declaration order,
then SELECTED assumptions in declaration order. Only nodes with persistent
state appear in a `src/mc_plan.rs::MonitorStateLayout`: an Obligation with
bound >= 1 gets 1 pending bit + ceil_log2(bound+1) remaining bits; a DelayedCheck
gets 1 antecedent bit (`src/mc_plan.rs::walk_node`, preorder node ids matching
the backends' n{id} naming). The sticky failed/trigger_step flags are
deliberately NOT part of the configuration: every configuration enqueued by the
exact engine has no confirmed failure yet (a safety failure terminates the
search before enqueue), so the flags are invariant on all live configurations
and carry no information.

**Traversal spec (D-P4-2).** value_domains mirror the abstraction;
`src/mc_plan.rs::StateTransitionOrder` per state = effective priority
(`priority N` descending, declaration order for ties via stable sort) - the
canonical successor order used by det mode (first match fires) and nondet mode
(all matches are successors, in this order); assumption_labels = all assumption
labels in declaration order (the engine's known-labels list for --assumes
selection, D-P4-6).

**Liveness decomposition (D-P4-9).** `src/mc_plan.rs::LivenessDecomposition::compute`:
per property, None iff pure safety or uncheckable; Some(spec) otherwise.
Obligations (`src/mc_plan.rs::LivenessObligation`) are `eventually p` /
`p until q` with BOOLEAN-over-atoms bodies (`src/mc_plan.rs::boolean_over_atoms`:
Expr/InputIs/OutputHas atoms under !/&&/||/-> only - no temporal operators
inside). `src/mc_plan.rs::decompose_top`: obligations may appear anywhere in the
boolean skeleton (`src/mc_plan.rs::Skeleton` of And/Or/Implies/Not over
`src/mc_plan.rs::SkeletonLeaf`); every other leaf must be a safety atom;
temporal operators inside a liveness property are rejected with a diagnostic -
"deeper nesting" when they contain liveness, otherwise "split the safety part
into a separate property". Diagnostics are codegen warnings (never parse
errors): ref_mc surfaces them as skip_warnings, both backends emit them as
codegen warnings.

Code: `src/mc_plan.rs::McPlan`, `src/mc_plan.rs::ValueDomain`,
`src/mc_plan.rs::FieldSpec`, `src/mc_plan.rs::VariantInputSpec`,
`src/mc_plan.rs::InputAbstraction`, `src/mc_plan.rs::VarSlot`,
`src/mc_plan.rs::MonitorNodeState`, `src/mc_plan.rs::MonitorStateLayout`,
`src/mc_plan.rs::ConfigEncoding`, `src/mc_plan.rs::StateTransitionOrder`,
`src/mc_plan.rs::TraversalSpec`, `src/mc_plan.rs::referenced_fields`,
`src/mc_plan.rs::used_identifiers`, `src/mc_plan.rs::collect_paths`,
`src/mc_plan.rs::collect_input_is_refs`, `src/mc_plan.rs::monitor_layouts`,
`src/mc_plan.rs::walk_node`, `src/mc_plan.rs::config_encoding`,
`src/mc_plan.rs::traversal_spec`, `src/mc_plan.rs::LivenessObligation`,
`src/mc_plan.rs::SkeletonLeaf`, `src/mc_plan.rs::Skeleton`,
`src/mc_plan.rs::LivenessPropertySpec`, `src/mc_plan.rs::LivenessDecomposition`,
`src/mc_plan.rs::boolean_over_atoms`, `src/mc_plan.rs::decompose_top`,
`src/codegen_fsm.rs::build_variants`, `src/codegen_fsm.rs::representative_values`,
`src/monitor.rs::compile_formula`


<a id="dev:mc-engine-exact"></a>
## Exact bounded checker (`src/ref_mc.rs`)

**Value + expression evaluation.** `src/ref_mc.rs::Val` (Bool / Bit u128 / Enum
name) + `src/ref_mc.rs::RType`; `src/ref_mc.rs::eval_expr` is the reference
evaluator - u128 wrapping arithmetic with width masks, the same modular
semantics the generated backends lower to (Rust u128 directly; C uint64
intermediates + masks). `src/ref_mc.rs::EvalCtx` carries module + state + vars
+ pattern binds.

**Own monitor compiler.** ref_mc compiles formulas into its OWN
`src/ref_mc.rs::Mon` type (Check / Delayed / Obligation / And / Or) - an
independent implementation of the documented semantics (SEMANTICS 5.5), not the
shared MonitorPlan (deep dive: docs/dev/monitor_compiler.md, task 11.3).
`src/ref_mc.rs::compile_formula` returns Err for out-of-subset formulas -
identical classification to the generated engines' compiler;
`src/ref_mc.rs::mon_step` returns the node's CURRENT-SAMPLE failure status
(per-sample, not sticky - essential for Or/And composition, where disjuncts may
fail and recover on different samples); `src/ref_mc.rs::MonSt` carries the
sticky failed + trigger_step; `src/ref_mc.rs::mon_key` /
`src/ref_mc.rs::mon_key_node` produce the persistent projection (remaining
packed only while pending) that forms the monitor part of the configuration key.

**Canonical inputs.** `src/ref_mc.rs::all_inputs`: variants in declaration
order; per-field value lists = full domain for referenced fields, single
representative otherwise; Cartesian product outermost-first - the same
enumeration McPlan's abstraction prescribes for the generated engines.

**Transition matching.** `src/ref_mc.rs::match_effect`: deterministic FIRST-MATCH
over the state's transitions in effective-priority stable order (own computation
of SEMANTICS 3.4); pattern group + variant + field-pattern check, guard
evaluation (an internal failure means "no match" - validated specs always
evaluate), emit args, SIMULTANEOUS var updates from old values; on_error is the
final catch-all with tid = usize::MAX and no bindings.

**run() - level-synchronous BFS.** `src/ref_mc.rs::run` (config:
`src/ref_mc.rs::RefConfig` - max_states / timeout_sec / assume_arg, defaults
from `src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES` /
`src/codegen_mc.rs::EXACT_DEFAULT_TIMEOUT_SEC`). Own monitor compilation
(liveness properties skipped - checked post-BFS) + liveness decomposition;
config widths recomputed from the documented SEMANTICS 9.2 formulas for
statistics only (monitor size, memory estimate). Initial config: constant-
initializer check (non-constant = Err - such specs do not produce working
generated binaries either); sample-0 stepping with assumptions FIRST - an
assumption violated at sample 0 is a clean error (the generated binaries panic
there; a CLI tool reports exit 1 instead), a property violation is FAIL with the
initial trace only. BFS: per-config timeout check (ONE level can exceed the
wall-clock budget on its own, so checking only between levels would let a run
blow past --timeout-sec); for each input in canonical order, match_effect ->
unexpected = path terminates (SEMANTICS 8.3), counted in statistics; else step
the monitors at d+1 (assumptions first - a pruned successor is never reported as
a property violation), first-violation-wins stop AFTER the per-level loop
(SEMANTICS 9.5); visited checked at ENQUEUE time with the max_states cap; adj
edges recorded for every success including to already-visited configs; complete
flag; deadlock = no successor at all (SEMANTICS 8.4) with deadlock_mons
captured - the persistent monitor projection of the deadlocked configuration,
which an external verifier needs to re-derive the "no surviving successor"
judgment (it depends on assumption-pruning state not printed in the report).
`src/ref_mc.rs::CfgKey` is a STRUCTURED key (state + typed var encodings - enums
as variant index, bools 0/1, bits raw - + per-monitor mon_key projections), no
bit-packing: deliberate, behavior must not depend on the container. Liveness
analysis runs only when no safety failure/deadlock was found; a witness inside
a truncated subgraph is SOUND (all its edges are real transitions). Status
taxonomy Running/Pass/StateLimit/Timeout/Fail maps to the shared
`src/codegen_mc.rs::ExactStatus`.

**Report rendering.** `src/ref_mc.rs::render_text`: byte-identical FORMAT to the
generated exact-mode binaries' default report - status line (PASS (complete) /
FAIL / UNKNOWN (state limit reached[, timeout after Ns])), the shared
`src/codegen_mc.rs::exact_summary_line`, statistics block (unexpected
terminations + first unexpected, monitor size bits, memory estimate), per-reason
counterexample header (property violation with trigger/violation step + vars at
each; liveness with formula + cycle length + note; deadlock line), then
"counterexample trace:" with initial state/vars + per-step lines in Rust Debug
form (State:: prefix added by the renderers). `src/ref_mc.rs::to_json_report`:
the canonical JSON document - same model + key order as the generated engines'
--counterexample-format json output (ce kinds trace / lasso{prefix, cycle,
cycle_length} / deadlock-config; ce_steps last); bit<64> values above i64::MAX
are documented unreachable for byte-parity-relevant specs (exact MC does not
enumerate such input fields).

Code: `src/ref_mc.rs::Val`, `src/ref_mc.rs::RType`, `src/ref_mc.rs::EvalCtx`,
`src/ref_mc.rs::eval_expr`, `src/ref_mc.rs::Sig`, `src/ref_mc.rs::InputVal`,
`src/ref_mc.rs::OutputVal`, `src/ref_mc.rs::Sample`, `src/ref_mc.rs::eval_sig`,
`src/ref_mc.rs::FlatVariant`, `src/ref_mc.rs::flatten`, `src/ref_mc.rs::Mon`,
`src/ref_mc.rs::compile_formula`, `src/ref_mc.rs::compile_signal`,
`src/ref_mc.rs::compile_check`, `src/ref_mc.rs::MonSt`, `src/ref_mc.rs::mon_init`,
`src/ref_mc.rs::mon_step`, `src/ref_mc.rs::min_trigger`, `src/ref_mc.rs::mon_key`,
`src/ref_mc.rs::mon_key_node`, `src/ref_mc.rs::mon_bits`,
`src/ref_mc.rs::all_inputs`, `src/ref_mc.rs::field_domain`,
`src/ref_mc.rs::representative_value`, `src/ref_mc.rs::Effect`,
`src/ref_mc.rs::pattern_bindings`, `src/ref_mc.rs::match_effect`,
`src/ref_mc.rs::input_matches_variant`, `src/ref_mc.rs::RefConfig`,
`src/ref_mc.rs::IoRec`, `src/ref_mc.rs::StepRec`, `src/ref_mc.rs::CeData`,
`src/ref_mc.rs::RefResult`, `src/ref_mc.rs::BNode`, `src/ref_mc.rs::CfgKey`,
`src/ref_mc.rs::val_key`, `src/ref_mc.rs::bnode_key`, `src/ref_mc.rs::run`,
`src/ref_mc.rs::path_to`, `src/ref_mc.rs::trace_steps`,
`src/ref_mc.rs::render_text`, `src/ref_mc.rs::to_json_report`,
`src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES`,
`src/codegen_mc.rs::EXACT_DEFAULT_TIMEOUT_SEC`,
`src/codegen_mc.rs::parse_assume_selection`


<a id="dev:mc-engine-liveness"></a>
## Liveness analysis (`src/ref_mc.rs`)

**Decomposition.** ref_mc carries its own copy of the mc_plan decomposition
(`src/ref_mc.rs::Leaf` / `src/ref_mc.rs::Skel` / `src/ref_mc.rs::LivProp` +
`src/ref_mc.rs::decompose_top` / `src/ref_mc.rs::decompose_properties`) -
LivProp additionally carries formula_text from the own formula printer
(byte-identical to the generated engines' embedded formula strings) and leaves
in first-appearance order.

**liv_analyze.** `src/ref_mc.rs::liv_analyze` on the explored graph:

- **Position graph**: position 0 = initial sample; one position per explored
  edge; labels are EDGE-based (SEMANTICS 6.5) - position k's label is the
  sample of step k. Per-position label bits via `src/ref_mc.rs::leaf_bits`
  (bit 2k = leaf k atom A, bit 2k+1 = the until-right operand).
- **Product reachability**: BFS with parent pointers over (position, automaton
  state) pairs from (0, start_q); `src/ref_mc.rs::prod_delta` is the mixed-radix
  transition of the per-leaf progress automata - safety 0=ok/1=bad, eventually
  0=wait/1=done, until 0=ok/1=sat/2=bad; all transitions are one-way, so every
  state sequence on an infinite path is eventually constant (the limit vector).
- **SCCs**: iterative Tarjan over the reachable product graph; on-cycle =
  non-trivial SCC member or self-loop singleton.
- **Falsifying limit vector search**: q ascending in discovery order - t_k =
  `src/ref_mc.rs::leaf_good`(kind, digit); skip if the skeleton evaluates true
  under t (`src/ref_mc.rs::skel_eval`); else the first witness pair on a cycle
  with automaton state q (discovery order).
- **Lasso construction**: product path from the start pair to the witness pair
  via parent pointers (prefix edges = positions of P1..P_{a-1}); the witness
  pair's own edge starts the cycle (pushed FIRST); DFS for the closing path back
  to the witness pair with GLOBAL visited pruning - not just the current path:
  path-only pruning re-explores diamond subgraphs exponentially and spins forever
  on small graphs (Phase 6 task 4 finding, specgen seed 900_006); edges to the
  witness pair are checked before the visited test, so any cycle through it is
  still found. Returns `src/ref_mc.rs::LivWitness`{lasso, cycle_start} where
  cycle_start = prefix length = the reported violation step.

**Truncated-graph soundness.** liv_analyze runs on the completed OR truncated
graph (only when no safety failure/deadlock was found): a witness found inside
the explored part is sound - all its edges are real transitions; otherwise the
UNKNOWN status stands. Never a false pass-complete.

**liv_note.** `src/ref_mc.rs::liv_note`: human-facing explanation of which leaves
are falsified on the witness - for each falsified leaf, the first falsifying
position (safety: bad is absorbing and constant on the cycle; until: left fails
without right holding) - "safety subformula false at step N" / "eventually body
never holds on the path" / "until left fails at step N without right holding",
joined with "; ".

Code: `src/ref_mc.rs::Leaf`, `src/ref_mc.rs::Skel`, `src/ref_mc.rs::LivProp`,
`src/ref_mc.rs::decompose_top`, `src/ref_mc.rs::decompose_properties`,
`src/ref_mc.rs::LivWitness`, `src/ref_mc.rs::leaf_signals`,
`src/ref_mc.rs::skel_eval`, `src/ref_mc.rs::liv_analyze`,
`src/ref_mc.rs::prod_delta`, `src/ref_mc.rs::leaf_good`,
`src/ref_mc.rs::leaf_bits`, `src/ref_mc.rs::liv_note`


<a id="dev:mc-engine-verify"></a>
## Counterexample verification + minimal labels (`src/ref_mc.rs`)

**verify_counterexample.** `src/ref_mc.rs::verify_counterexample` is the ground-
truth verifier for counterexamples from ANY engine (used by differential
testing): dispatch on (reason, ce kind) - a mismatch is an error.

- PropertyViolation + Trace: `src/ref_mc.rs::check_initial` (the CE's claimed
  initial configuration must equal the module's), replay the inputs through
  `src/ref_mc.rs::step_trace` (fresh-module stepping producing the Sample
  sequence), `src/ref_mc.rs::check_step_config` per step (claimed from/to state,
  vars, and outputs must match the replay), then
  `src/ref_mc.rs::property_confirms_at` - step the label's monitor over the
  replayed samples: the FIRST confirmed failure must be exactly at the claimed
  step (liveness labels are rejected as not monitorable).
- LivenessViolation + Lasso: same replay over prefix+cycle, then
  `src/ref_mc.rs::verify_lasso` - configuration closure, product-automaton
  closure, and a falsifying limit vector (the same acceptance condition
  liv_analyze uses for end components); the summary step must equal the prefix
  length.
- Deadlock + DeadlockConfig: `src/ref_mc.rs::verify_deadlock` - domain checks
  against declared types + the no-surviving-successor judgment re-derived from
  the carried monitor projection (`src/ref_mc.rs::mon_from_key` reconstructs a
  MonSt from its persistent projection).

**minimal_violation_labels.** `src/ref_mc.rs::minimal_violation_labels`: two
phases - phase 1 canonical run finds the first violation in traversal order;
phase 2 re-runs BFS but keeps processing levels until level first_step-1 is
fully expanded, collecting every label confirmed at the minimal step
(`src/ref_mc.rs::collect_min_labels`); liveness witnesses from phase-1's graph
are included when their cycle start <= the global min. Sorted output.

**Differential-testing role.** tests/ref_mc_diff_test.rs (ref-MC vs BOTH
generated engines: status/states/max_depth agreement, the engine's label is a
member of the minimal-violation set - for specs with a UNIQUE minimal label,
label+step equality is additionally asserted - and CE replay through
verify_counterexample; transitions deliberately NOT asserted cross-engine, since
traversal accounting may differ - it IS asserted cross-backend by
tests/backend_parity_test.rs); tests/ref_mc_sweep_test.rs (byte-parity sweep vs
the generated Rust MC - exit code + exact stdout in text AND json, fail-fast
parallel over all tractable valid examples); tests/ref_mc_diag_test.rs (non-
fail-fast mismatch enumerator over all tractable examples). The negative control
(the "no false PASS" invariant of Phase 6) is part of the diff suite.

Code: `src/ref_mc.rs::verify_counterexample`, `src/ref_mc.rs::check_initial`,
`src/ref_mc.rs::check_step_config`, `src/ref_mc.rs::property_confirms_at`,
`src/ref_mc.rs::verify_lasso`, `src/ref_mc.rs::verify_deadlock`,
`src/ref_mc.rs::mon_from_key`, `src/ref_mc.rs::step_trace`,
`src/ref_mc.rs::initial_config`, `src/ref_mc.rs::io_to_input`,
`src/ref_mc.rs::io_to_output`, `src/ref_mc.rs::json_val`,
`src/ref_mc.rs::minimal_violation_labels`, `src/ref_mc.rs::collect_min_labels`,
`tests/ref_mc_diff_test.rs`, `tests/ref_mc_sweep_test.rs`,
`tests/ref_mc_diag_test.rs`, `tests/backend_parity_test.rs`


<a id="dev:mc-engine-smt"></a>
## SMT bounded mode (`src/smt_plan.rs` + `src/smt_emit.rs`)

**Shared spec (D-P4-12).** `src/smt_plan.rs::build_smt_spec`(module, plan) converts a
validated module + its exact-mode `src/mc_plan.rs::McPlan` into the backend-neutral
`src/smt_plan.rs::SmtSpec` for k-step unrolling over z3 bitvectors. Both backends
render this SAME structure with the SAME traversal order and the SAME z3 operations,
so constraint construction is identical in the Rust and C generated binaries (parity
by construction). The spec carries:

- states in declaration order + initial_state index + state_bits = max(1, ceil_log2(n))
  (`src/smt_plan.rs::ceil_log2_pub`);
- `src/smt_plan.rs::SmtVar`s - bit<N>/bool only (width 1 for bool; is_bool is witness
  display only: true/false vs 0/1), literal inits only (v1);
- `src/smt_plan.rs::SmtClass`es from the plan's abstraction table - one canonical input
  class per event variant, CONCRETE fields only (name, bit width, is_bool); abstracted
  fields are substituted as their representative literal during expression conversion;
- `src/smt_plan.rs::SmtTrans`itions in FIRING order (priority desc, declaration order
  for ties via stable sort - the same order the exact engine uses for first-match
  selection): state/next_state/class indices, guard Option<Se>, assignments (var index,
  expr) - vars absent are unchanged;
- `src/smt_plan.rs::SmtProp`s: top-level always / eventually only
  (`src/smt_plan.rs::SmtPropKind`), formula in the boolean layer `src/smt_plan.rs::Sf`
  (Se/Not/And/Or/Implies) over configuration expressions.

**Expression model.** `src/smt_plan.rs::Se` mirrors the IR expression language with
references already resolved to indices: Lit{value, is_bool} (a bool literal is a z3
Bool in boolean contexts; an integer literal is lifted by its parent context), State,
Var(idx), Field{idx, width, is_bool} - class-relative, both captured at conversion
time so they cannot be recovered from the spec alone -, Not/BitNot/Neg, and Bin over
the 16-operator `src/smt_plan.rs::Sb` subset of the IR BinOp. `src/smt_plan.rs::convert_expr`
resolves names in EXACTLY the validator's order (state -> payload bindings -> vars ->
state variants; `src/smt_plan.rs::ExprCtx`); an abstracted field becomes its
representative literal (`src/codegen_fsm.rs::RepValue`). `src/smt_plan.rs::convert_formula`
walks property formulas with NO payload context (an empty variant view).

**Typing.** `src/smt_plan.rs::SeType` (Bool / Bit(w) / IntLit) +
`src/smt_plan.rs::se_type`: bottom-up typing computed with EXACTLY the validator's
rules - integer literals default to Bit(8) and are lifted by their parent per the
binary-op rules (`src/smt_plan.rs::bit_op_type`: (Bit(a), Bit(b)) requires a == b,
IntLit lifts to the sibling's width, (IntLit, IntLit) -> Bit(8)). The spec is already
validated, so this never fails; both emitters use it to pick z3 constructors (Bool vs
bitvector) and to mask literals to the right width.

**v1 scope diagnostics.** `src/smt_plan.rs::build_smt_spec` rejects unsupported
constructs with a shared error string - IDENTICAL in both backends (codegen behavior
is parity-safe), surfaced as a codegen error (exit 1): assumptions present; on_error
blocks; enum-typed variables; non-literal initial values; an init that does not fit
its width; concrete payload fields wider than 64 bits (the plan's saturated u64::MAX
domain); non-power-of-two domains (not a bit<N> type); enum-typed payload fields;
wildcard event patterns over multi-variant groups (a bare group pattern names exactly
one variant only for single-variant groups); unknown state/next-state/var references;
properties that are not top-level always/eventually and property formulas beyond
expr/not/and/or/implies (`src/smt_plan.rs::formula_tag` names the rejected shape);
enum paths in expressions.

**Shared emitter.** `src/smt_emit.rs::emit_rust_spec` / `src/smt_emit.rs::emit_c_spec`
render the spec section: constants (STATE_BITS, INITIAL_STATE, STATES,
VAR_NAMES/WIDTHS/INITS/IS_BOOL, CLASS_NAMES, N_CLASSES, CLS_BITS,
FIELD_NAMES/WIDTHS/IS_BOOL, FIELD_POS, TRANS = (state, next_state, class) in firing
order, PROP_NAMES, PROP_KINDS 0=always/1=eventually), one expression-builder function
per transition guard / assignment / property formula, and function-pointer tables +
an ASGN_MAP so the spec-independent engine (in the backend files) can loop over them.

**Leaf-variable convention (parity-critical).** Step variables are z3 CONSTANTS with
fixed names - s_<i> state, v_<i>_<j> vars, ev_<i> input class, fld_<i>_<g> fields; the
spec's builders reference the s/v/fld leaves. The C builders receive them as
parameters; the Rust builders recreate them by name - z3 hash-consing returns the
SAME node, so both backends build structurally identical formulas.
`src/smt_emit.rs::global_field_index` fixes the global concrete-field ordering (all
classes in order, fields within a class in order) for the fld arrays.

**Rendering.** Both emitters lower an Se/Sf tree to sequential bindings e0..eN
(`src/smt_emit.rs::RustRender` / `src/smt_emit.rs::CRender`; `src/smt_emit.rs::render_se_rust`
/ `src/smt_emit.rs::render_se_c` mirrors; `src/smt_emit.rs::produces_bool` dispatches
each binary op to the bool or bit path):

- arithmetic/bitwise at the se_type width (bvadd/bvsub/bvmul/bvand/bvor/bvxor;
  `src/smt_emit.rs::render_bit_bin_rust` / `src/smt_emit.rs::render_bit_bin_c`);
- comparisons with EXACT-MC FULL-VALUE semantics (`src/smt_emit.rs::cmp_widths_full`):
  integer literals lift to the sibling's width; if a literal does not fit, BOTH sides
  widen to hold it and the non-literal side is zero-extended (`src/smt_emit.rs::widen_rust`
  / `src/smt_emit.rs::widen_c`, concat with zeros) - counter < 256 on a bit<8> is always
  true; arithmetic ops need no such widening (truncating the literal is equivalent
  there - ring homomorphism); unsigned bvult/bvule/bvugt/bvuge, plus bool-ordering
  encodings for comparisons on bools (RFS allows them: Lt = not-a and b, Le =
  not(a and not-b), Gt = not-b and a, Ge = not(b and not-a);
  `src/smt_emit.rs::render_bool_bin_rust` / `src/smt_emit.rs::render_bool_bin_c` - the
  C side uses two-arg Z3_mk_and/Z3_mk_or over 2-element arrays, the counterpart of the
  Rust b_and/b_or wrappers);
- shifts with exact-MC semantics amount >= width -> 0 (`src/smt_emit.rs::shift_amt_width`
  + `src/smt_emit.rs::render_shift_rust` / `src/smt_emit.rs::render_shift_c`):
  zero-extend the amount to max(amount width, wl) - integer literals lift to at least
  wl and to enough bits to hold the value (no truncation, so an oversized amount stays
  detectable), guard bvult(amt, wl), truncate the amount to wl bits (lossless under
  the guard), ite(shift, 0);
- Bool results wrapped as BV(1) only where the context requires it - assignment top
  level, where the target var is a BV (`src/smt_emit.rs::finish_bool_rust` /
  `src/smt_emit.rs::finish_bool_c`, an ite structurally identical in both backends).

**Warning-free generated code.** `src/smt_emit.rs::se_uses_step` /
`src/smt_emit.rs::sf_uses_step` decide whether a builder's step-index parameter is used
(_i otherwise); asgn_dummy fills unused ASGN_MAP slots (never called - the engine
checks ASGN_MAP first) and is emitted only when some slot needs it. C specifics:
includes come first; zero-size arrays are invalid in C, so empty VAR_WIDTHS/
FIELD_NAMES/FIELD_WIDTHS/FIELD_IS_BOOL carry a dummy slot the engine never touches (it
loops over N_VARS/N_FIELDS).

**Engine + verdicts (link, not restate).** The spec-independent k-step unrolling
engine lives in the backend files - SMT_ENGINE_RUST (docs/dev/backend_rust.md) and
SMT_ENGINE_C (docs/dev/backend_c.md): first-match firing as a prefix-OR chain over
firing order, alive tracking, safety query (exists i < k: alive_i and not phi_p(i))
and liveness via k-liveness (never + cycle query). Verdicts: SAT -> sound FAIL within
bound k; all UNSAT -> PASS WITHIN BOUND k (always labeled incomplete - no completeness
claim); inconclusive -> UNKNOWN; exit codes 0/1/2. One libz3 build (5.3.1,
TOOLCHAIN.md section 5) is shared by both backends; the mode is feature-gated (smt
cargo feature), codegen flag --smt-bound N (default 10) overridable at runtime.
Algorithm views L1-L3: dev:algorithms-mc-engine; external references:
[model checking, SMT/SAT solving](REFERENCES.md#model-checking-smt-sat-solving).

**Parity pins.** The emitter's own test module pins the width/shift semantics
(cmp_widths_full_fits_and_widens, shift_amt_width_rules,
oversized_literal_comparison_widens, shift_rendering_paths) and - the D-P4-12 core
guarantee - that EVERY builder function renders to the SAME sequence of z3 operations
in Rust and C: rust_and_c_render_identical_op_sequences normalizes both backends'
bodies to op tokens (backend syntax stripped; leaves map to the same s/v_j/fld_g
tokens) and asserts equality.

Code: `src/smt_plan.rs::Se`, `src/smt_plan.rs::Sb`, `src/smt_plan.rs::Sf`,
`src/smt_plan.rs::SmtVar`, `src/smt_plan.rs::SmtClass`, `src/smt_plan.rs::SmtTrans`,
`src/smt_plan.rs::SmtPropKind`, `src/smt_plan.rs::SmtProp`, `src/smt_plan.rs::SmtSpec`,
`src/smt_plan.rs::ExprCtx`, `src/smt_plan.rs::ceil_log2_pub`,
`src/smt_plan.rs::build_smt_spec`, `src/smt_plan.rs::formula_tag`,
`src/smt_plan.rs::convert_expr`, `src/smt_plan.rs::convert_formula`,
`src/smt_plan.rs::SeType`, `src/smt_plan.rs::se_type`, `src/smt_plan.rs::bit_op_type`,
`src/smt_emit.rs::global_field_index`, `src/smt_emit.rs::se_uses_step`,
`src/smt_emit.rs::sf_uses_step`, `src/smt_emit.rs::RustRender`,
`src/smt_emit.rs::CRender`, `src/smt_emit.rs::render_se_rust`,
`src/smt_emit.rs::render_se_c`, `src/smt_emit.rs::produces_bool`,
`src/smt_emit.rs::finish_bool_rust`, `src/smt_emit.rs::finish_bool_c`,
`src/smt_emit.rs::cmp_widths_full`, `src/smt_emit.rs::widen_rust`,
`src/smt_emit.rs::widen_c`, `src/smt_emit.rs::render_bool_bin_rust`,
`src/smt_emit.rs::render_bool_bin_c`, `src/smt_emit.rs::render_bit_bin_rust`,
`src/smt_emit.rs::render_bit_bin_c`, `src/smt_emit.rs::shift_amt_width`,
`src/smt_emit.rs::render_shift_rust`, `src/smt_emit.rs::render_shift_c`,
`src/smt_emit.rs::emit_rust_spec`, `src/smt_emit.rs::emit_c_spec`,
`src/mc_plan.rs::McPlan`, `src/codegen_fsm.rs::RepValue`

