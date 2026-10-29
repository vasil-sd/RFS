# RFS Design Decisions - Rationale Index

Every decision entry in `DESIGN.md` (section 9 subsections 9.1-9.11, section
19 bullets, sections 20-25 D-numbered decisions) with a 2-4 line "why"
summary + link to the full table and to the implementing code.

<a id="dev:decisions-early"></a>
## Phase 0-2 decisions (section 9 + section 19)

The eleven major design choices of `DESIGN.md` section 9 (each a comparison
table with a marked Decision column) and the eight Phase 1 code-generation
bullets of section 19. Each entry states the chosen option, summarizes the
rationale in 2-4 lines, and links to the full table and the implementing code.

### 9.1 DSL hosting - external `.rfs` files (primary)

External Rust-like DSL in `.rfs` files is primary; IR JSON is internal-only;
an embedded proc-macro DSL is optional later; YAML/TOML rejected.
**Why:** a stable, versionable syntax with its own parser gives good
diagnostics and lets one IR feed multiple targets; embedding the DSL in Rust
macros would constrain the syntax to the macro system and degrade error
quality, while data formats are too weak for guarded transitions and temporal
properties.
Table: DESIGN.md section 9.1 - Code: `src/parser.rs::parse_source`,
`src/ir.rs::Ir`.

### 9.2 FSM authoring style - Rust-like pattern-matching transition blocks

Pattern-matching transition blocks are primary; explicit transition tables
rejected as primary; diagrams are generated only; statecharts not in v0.
**Why:** the style is familiar to Rust users, maps directly onto `match` in
generated code, and supports bindings/guards/actions in one construct; it
requires clear priority semantics, which 9.4 provides.
Table: DESIGN.md section 9.2 - Code: `src/backends/rust_fsm.rs::transition_block`,
`src/backends/c_fsm.rs::transition_block_c`.

### 9.3 Data model - fixed-width finite types (`bool`, enums, `bit<N>`)

Fixed-width finite types are the core; arbitrary Rust types excluded from the
core; SMT-style unbounded integers later; opaque payload fields allowed as
don't-care later.
**Why:** a finite state space is what makes exact model checking sound and
complete for small systems, and fixed-width types map directly onto primitive
integers in both backends; unbounded or side-effecting types would make MC
impossible or unsound.
Table: DESIGN.md section 9.3 - Code: `src/validator.rs::type_expr`,
`src/backends/rust.rs::rust_type`, `src/backends/c.rs::c_type`.

### 9.4 Determinism and transition selection - explicit priorities + disjointness validation

Explicit priorities plus disjointness validation are primary;
declaration-order determinism rejected; pure nondeterminism is an MC-only mode
later; a hybrid (deterministic production + optional nondeterministic MC) is
recommended later.
**Why:** production protocol handlers need deterministic, explainable
behavior, and hidden source-order semantics would be fragile; explicit
priorities give precise control with good diagnostics for duplicates and
overlapping guards.
Table: DESIGN.md section 9.4 - Code: `src/validator.rs::validate_module`.

### 9.5 Time and timeouts - explicit `Tick` event plus timer variables

An explicit `Tick` event plus timer variables is the core mechanism; no
real-time clocks in v0; implicit `after N` delays are sugar maybe later.
**Why:** discrete ticks keep the state space finite and make MC/codegen
trivially sound, at the cost of the user modeling ticks explicitly -
acceptable for protocol timeouts. `Tick` is a plain declared event name (no
parser special-casing), as in examples/valid/03_timeout_retry.rfs.
Table: DESIGN.md section 9.5 - Code: `src/parser.rs::parse_source`.

### 9.6 Property language - PSL-like subset

A PSL-like subset is primary; pure LTL stays the internal foundation; SVA-like
naming avoided; Rust closures rejected for core properties.
**Why:** the requested style is practical for bounded protocol event sequences
and compiles to online monitors; pure LTL is less practical for bounded
patterns, and arbitrary closures are not automatically model-checkable.
Table: DESIGN.md section 9.6 - Code: `src/parser.rs::primary_formula`,
`src/monitor.rs::compile_formula`.

### 9.7 Model-checking engine strategy - exhaustive reachable-state search (exact mode)

Exhaustive reachable-state BFS with state caching is the core exact mode;
brute-force input enumeration was the Phase 1 MVP only; symbolic BDD/bitset,
SMT-based bounded MC, and k-liveness are later optional.
The shipped exact engine reports the three-status taxonomy of DESIGN.md
section 11 (D-P4-4): `pass-complete` / `fail` / `unknown-state-limit`, exit
codes 0/1/2 - so bounded runs are never presented as full proof.
**Why:** complete for small finite systems and yields counterexamples, which
is the right core guarantee; symbolic approaches handle larger state spaces
but add significant implementation complexity, so they stay optional.
Table: DESIGN.md section 9.7 - Code: `src/mc_plan.rs::McPlan`,
`src/backends/rust_mc.rs::exact_codegen`, `src/ref_mc.rs::run`.

### 9.8 Production integration style - pure `step` function + trait adapters

A generated standalone pure `step` function is the core; trait-based adapters
shipped in Phase 5 (the `ProtocolFsm` trait + generic adapters, wired via the
opt-in `--with-protocol-fsm` codegen flag); runtime-specific generated code
e.g. tokio-only rejected.
**Why:** a pure step function is simple, testable, and has no runtime
coupling; the adapter layer adds sync/async composability without tying the
toolchain to any executor - the async adapter works on ANY executor.
Table: DESIGN.md section 9.8 - Code: `src/backends/rust_fsm.rs::generate_production`,
`crates/rfs-adapters/src/lib.rs::ProtocolFsm`.

### 9.9 Error handling in production code - `Result`-based, recoverable

Returning a `Result`/error type from the step function is the default;
panicking on an unexpected event rejected as default; an explicit error state
or default transition is supported later.
**Why:** recoverable, testable, and explicit errors are what production
protocol handlers need; panicking on an unexpected event is unsafe for
long-running services. The generated fallback path (declared `on_error` or the
default `Err`) is emitted per backend.
Table: DESIGN.md section 9.9 - Code: `src/backends/rust_fsm.rs::on_error_block`,
`src/backends/c_fsm.rs::emit_fsm_impl`.

### 9.10 Assumptions about the environment - PSL-like `assume` properties

PSL-like `assume` properties are core; a fully nondeterministic environment is
kept as an option (stress mode); a separate environment automaton is a later
extension.
**Why:** assumptions in the same property language are concise and integrate
directly into MC (they prune exploration), while staying optional so
stronger, assumption-free verification remains available.
Table: DESIGN.md section 9.10 - Code: `src/codegen.rs::AssumptionSelection`,
`src/mc_plan.rs::McPlan`.

### 9.11 Composition - single endpoint FSM (v0/v1)

A single endpoint FSM is primary for v0/v1; modules with ports and product
composition are a later extension; a shared global event space is a possible
intermediate step.
**Why:** a single endpoint keeps MC/codegen simple and the v1 scope
manageable; multi-party composition brings state explosion and codegen
complexity that are deliberately deferred.
Table: DESIGN.md section 9.11 - Code: `src/ir.rs::Module`.

### Section 19 - Phase 1 code-generation decisions

The eight bullets of DESIGN.md section 19 (full text there; recorded in detail
in KB.md section 13):

- **Code generation consumes the validated IR, not parser text.** Why: one
  normalized form guarantees every target sees identical semantics and lets
  validation errors stop generation early. Code: `src/codegen.rs::run`.
- **`bit<N>` maps to the smallest suitable primitive integer; arithmetic uses
  wrapping `u128` intermediates masked to the declared width.** Why: exact
  fixed-width semantics without overflow UB - wider intermediates keep
  multi-operand expressions correct, and the final mask enforces the declared
  width. Code: `src/backends/rust.rs::bit_rust_type`,
  `src/backends/rust_fsm.rs::gen_expr`.
- **Variable updates are simultaneous** (all assignment right-hand sides are
  evaluated before any variable is written). Why: matches the spec's
  mathematical semantics and avoids order-dependent surprises in
  multi-assignment transitions. Code: `src/backends/rust_fsm.rs::transition_block`.
- **Assumption violations use a trait-based handler passed to
  `step_with_handler`; the default `step` panics.** Why: production callers
  choose the policy (panic/stop/continue) explicitly while the simple API
  stays panic-by-default. Code:
  `src/backends/rust_fsm.rs::step_with_handler`.
- **Assumption-selection option: `all`, `none`, or comma-separated labels;
  default `all`.** Why: lets MC and codegen target specific assumptions (e.g.
  stress runs with none) without editing the spec. Code:
  `src/codegen.rs::AssumptionSelection`.
- **Only safety formulas in the translated subset become runtime checks;
  `bounded_eventually` skipped with a warning - Phase 3 update
  (D-P3-4/D-P3-5): all in-subset formulas, including `bounded_eventually`, are
  now online-monitored in both backends; only out-of-subset (R1-R4) formulas
  are skipped.** Why: monitors must be decidable online; the subset restriction
  keeps that guarantee, and Phase 3 widened it to the full in-subset language.
  Code: `src/monitor.rs::compile_formula`.
- **Generated unit tests include initial-state, positive bounded-sequence, and
  negative unexpected-input tests; sequence length configurable, default 4.**
  Why: a minimal deterministic test battery per spec that exercises the happy
  path and the error path without hand-written tests. Code:
  `src/codegen_test.rs::build_test_plan`.
- **The generated bounded model checker uses representative input constructors
  by default, can inject unexpected events, and can force selected safety
  assumptions to violate with per-run probabilities.** Why: representative
  values keep the input space finite while still covering each variant;
  injection/forcing give directed bug hunting on top of exhaustive search.
  Code: `src/codegen_fsm.rs::representative_values`,
  `src/codegen_mc.rs::ViolationSpec`.

<a id="dev:decisions-p2-p3"></a>
## Phase 2 core DSL (D1-D10) + Phase 3 (D-P3-1..6)

Phase 2 makes the DSL practical for real endpoint protocol FSMs (the IR already
carried `priority`, pattern bindings, and output payloads since Phase 0; Phase
2 wires them through validation and code generation, adds two syntax forms,
and adds the C backend). Phase 3 implements the PSL-like property subset with
ONLINE monitors in every backend: each `prop`/`assume` compiles in the shared,
language-agnostic monitor compiler to a per-label plan that both backends
render with identical semantics.

### D1. Priority direction - larger number wins (checked first)

Larger priority numbers win; smaller-wins (POSIX nice / interrupt style)
rejected.
**Why:** matches the "priority 10 beats priority 5" intuition and is easy to
explain in specs; the inverted convention would leave authors wondering whether
1 is top or bottom.
Table: DESIGN.md section 20, D1 - Code: `src/backends/rust_fsm.rs::emit_fsm_impl`,
`src/backends/c_fsm.rs::emit_fsm_impl`.

### D2. Ambiguity and duplicate detection (same state, identical event pattern)

Two transitions in one state can conflict only when their patterns are
identical. Severity ladder: identical pattern + identical guard = **error**
("duplicate transitions" - the second is provably shadowed); two *different*
explicit priorities = allowed (author pinned intent); duplicate explicit
priority value = **error** (a tie cannot be ordered deterministically); no
priorities with different (or one missing) guard = **warning**
("order-dependent" - first match wins in source order). Guards are compared
structurally, no value analysis.
**Why:** the check is precise where conflicts are possible and lenient where
they are not; provably dead transitions must fail, unorderable ties must fail,
and legal if/else chains stay valid but get told to pin intent - the "at least
syntactically" level required by the roadmap, with bounded checking left to
Phase 4.
Table: DESIGN.md section 20, D2 - Code: `src/validator.rs::validate_module`.

### D3. Duplicate-priority scope - unique per (state, pattern)

Priorities must be unique per (state, pattern); per-state uniqueness rejected.
**Why:** numbers can be reused across unrelated patterns in the same state with
the minimal constraint; per-state uniqueness forces a global numbering scheme
even for patterns that cannot conflict - it was rejected after it broke a
natural spec during implementation.
Table: DESIGN.md section 20, D3 - Code: `src/validator.rs::validate_module`.

### D4. Severity of reachability/usage diagnostics - warnings only

Unreachable state, dead event (constructor never in any pattern), unused
variable, and impossible guard (literal `false`; same variable compared equal
to two different integer literals in one top-level conjunction) are all
**warnings**; they never block code generation or validation exit codes.
**Why:** these checks over-approximate (guards ignored), so legitimate specs
with defensive states must not fail, and the existing valid examples had to
keep passing; warnings are printed by `rfs validate` and surfaced through
codegen output warnings.
Table: DESIGN.md section 20, D4 - Code: `src/validator.rs::module_warnings`,
`src/frontend.rs::run_validate`.

### D5. Effective priority default - 0 (lowest)

A transition without `priority N;` has effective priority 0; within a state,
arms are checked in descending effective priority (stable for ties; ties are
validator errors); cross-state ordering is irrelevant because only the active
state can match. Generated code groups transitions by state and sorts each
group.
**Why:** specs without priorities generate behaviorally identical code to
Phase 1 (source order preserved within a state), so the default is invisible
unless the author opts in.
Table: DESIGN.md section 20, D5 - Code: `src/backends/rust_fsm.rs::emit_fsm_impl`,
`src/backends/c_fsm.rs::emit_fsm_impl`.

### D6. Error handling for unexpected inputs - module-level `on_error` block

A single module-level `on_error { next S; [emit O(...);] }` block is chosen;
wildcard transitions and policy keywords rejected. Semantics: when no
transition matches, `on_error` executes in the current state - it moves to its
declared state and emits its outputs, with **no pattern bindings** and no
variable assignments (values carry over unchanged). Without `on_error`,
behavior is unchanged from Phase 1 (`StepError::UnexpectedEvent` /
`RFS_UNEXPECTED_EVENT`).
**Why:** explicit, one per module, no new pattern grammar, works in every
state, and MC-compatible (it is just a fallback arm); the wildcard form would
clash with the existing `_` field wildcard and be easy to misuse, and an
"ignore" policy silently drops protocol errors (bad for model checking).
Table: DESIGN.md section 20, D6 - Code: `src/ir.rs::Module`,
`src/backends/rust_fsm.rs::on_error_block`, `src/backends/c_fsm.rs::emit_fsm_impl`.

### D7. Module version header syntax - `module Name version "x.y.z" {`

An optional version token on the module line is chosen; a
`meta version = "..."` item inside the module rejected (revisit if more
metadata keys appear). The version is stored in the IR (`Module.version`,
optional, skipped when absent), printed by the RFS printer, and emitted as
`pub const SPEC_VERSION` (Rust) / `#define RFS_SPEC_VERSION` (C).
**Why:** visible at a glance, one optional token, trivially round-trips; a new
item kind for what is currently a single value was overkill while no second
metadata key exists.
Table: DESIGN.md section 20, D7 - Code: `src/ir.rs::Module`,
`src/printer.rs::print_ir`, `src/backends/rust_fsm.rs::emit_monitors`,
`src/backends/c_fsm.rs::emit_monitors`.

### D8. C backend type mapping and scope

`bool` -> `bool`; `bit<N>` -> `uintN_t` for N in {8, 16, 32, 64} (other widths
are a **codegen error** - C has no wider fixed-width types; Rust uses u128
intermediates, C relies on native unsigned wrap-around, identical results mod
2^N); enums -> `typedef enum` with the `STATE_` prefix on state variants;
inputs/outputs -> payload structs + `kind` int + anonymous union
(`input_t` / `output_t`); simultaneous updates via temp variables exactly like
Rust; assumption handling via a function pointer with CONTINUE/STOP/PANIC
policies (default handler aborts), mirroring the Rust `AssumptionHandler`
trait; unit tests are a plain-assert harness in `main()` with failure counting
(same TestPlan semantics as Rust, including the on_error landing check); the
model checker uses the **same runtime wire format** as the Rust MC (same flags,
same LCG constants 6364136223846793005 / 1442695040888963407, same three-phase
step strategy, same summary line); property translation is the same subset -
**Phase 3 update (D-P3-4/D-P3-5):** all in-subset formulas, including bounded
liveness, are now online-monitored in BOTH backends from the shared monitor
plan; only out-of-subset formulas (R1-R4) are skipped, with identical warning
text.
**Why:** backend parity - the C backend must produce exactly the same
observable behavior as Rust for the same spec and inputs; native unsigned
wrap-around gives bit-identical fixed-width arithmetic without wider types,
and sharing the MC wire format makes cross-backend differential testing
meaningful.
Table: DESIGN.md section 20, D8 - Code: `src/backends/c.rs::c_type`,
`src/backends/c.rs::bit_c_type`, `src/backends/c_fsm.rs::emit_monitors`,
`src/backends/c_mc.rs::generate_model_checker`.

### D9. Feature flags (std/no_std, property-monitors, logging hooks) - deferred, resolved in Phase 5

Deferred beyond Phase 2: the roadmap marked `no_std` as "later" and no
consumer needed the other two yet; recorded so they are not silently dropped.
**Phase 5 resolution (D-P5-9):** `--no-std` production codegen shipped
(`#![no_std]` + `extern crate alloc`, identical public API shape; C accepts the
flag as a documented no-op); property monitors shipped in Phase 3 (debug-gated,
D-P3-6); no logging hook (the CLI has `--log-level` instead).
**Why:** generated code is already dependency-free single files, which keeps
the door open; shipping flags only when a consumer needs them avoids maintaining
dead options.
Table: DESIGN.md section 20, D9 - Code: `src/codegen.rs::BackendOptions`.

### D10. IR evolution rule - additive optional fields do not bump IR_VERSION

Additive optional fields (default + skip-when-none) do **not** bump
`IR_VERSION`; existing IR JSON keeps deserializing. Breaking changes to the IR
shape must bump `IR_VERSION` and update round-trip tests. Phase 2 added
`Module.version` and `Module.on_error` under this rule (IR stayed v0).
**Why:** purely additive changes need no version boundary - old JSON
round-trips unchanged; reserving bumps for breaking changes keeps the version
meaningful. (Phase 5's hand-rolled JSON reader-writer, D-P5-2, preserves this
contract with pinned key order.)
Table: DESIGN.md section 20, D10 - Code: `src/ir.rs::IR_VERSION`,
`src/json_rw.rs`.

### D-P3-1. IR representation of `always[N]` - additive `BoundedAlways` variant

An additive `BoundedAlways { bound, inner }` formula variant (wire tag
`bounded_always`) with no `IR_VERSION` bump; a v1 bump rejected.
**Why:** old IR JSON round-trips unchanged - purely additive per the D10 rule;
bumping for an additive change would break all existing IR JSON for no benefit.
Table: DESIGN.md section 21, D-P3-1 - Code: `src/ir.rs::Formula`.

### D-P3-2. Zero-bound rules - `always[N]` requires N >= 1; `be[0]` stays legal

`always[N]` requires N >= 1 (validator error); `bounded_eventually[0]` stays
legal and means "the body must hold at the trigger sample itself"; uniform
rejection of both zero bounds rejected.
**Why:** the new syntax has no compatibility concern, while `be[0]` keeps a
meaningful degenerate "hold now" form; no existing example used either form
(verified), so the asymmetric rule costs nothing.
Table: DESIGN.md section 21, D-P3-2 - Code:
`src/validator.rs::bounded_always_error`.

### D-P3-3. Exact monitor semantics - violation confirmed at the deadline sample

Sampling follows DESIGN.md section 11: per-sample values are post-step;
`input_is`/`output_has` refer to the transition just taken; sample 0 is the
initial configuration (no input/outputs, so both atoms are false there). For an
obligation triggered at sample i with bound N, the violation is confirmed at
the deadline sample i + N - 1 (countdown reaches 0); confirming one sample
after the deadline (i + N) rejected.
**Why:** tighter counterexamples - the violation step is the last allowed
sample, matching the countdown implementation; the alternative points past the
last allowed sample and is off-by-one vs the countdown. Pinned by
monitor-compiler tests before any codegen rendered it (PHASE_3.md's conceptual
pseudocode had to be reconciled).
Table: DESIGN.md section 21, D-P3-3 - Code: `src/monitor.rs::compile_check`,
`src/monitor.rs::ReferenceMonitor`.

### D-P3-4. Online monitors replace post-hoc checks

Monitors step online in the generated commit-step function; the generated
post-hoc `property_check`/`assume_check` trace functions were removed in BOTH
backends; keeping both paths rejected. Assumption enforcement stays on the
existing `AssumptionHandler` path (a monitor-driven assumption violation is
reported through it), so all MC/test call sites - trial-step forced violations,
`--assumes`, integration tests - keep working unchanged; the
`PROPERTY_LABELS`/`ASSUME_LABELS` constants stay.
**Why:** a single evaluation path - bounded-liveness obligations become
monitorable mid-run and assumption pruning works online in MC (incl. liveness
assumptions); two paths could diverge and would duplicate monitor state.
Table: DESIGN.md section 21, D-P3-4 - Code:
`src/backends/rust_fsm.rs::step_with_handler`, `src/monitor.rs::compile_formula`.

### D-P3-5. C backend: full monitor parity

Full monitor parity in C: the same monitor plan rendered in C, C MC gets online
detection + enhanced counterexamples, and cross-backend differential tests
verify identical behavior; Rust-only monitors rejected. Monitor-compilability
restrictions R1-R4 (no `bounded_eventually` under negation; a
`bounded_eventually[N]` body must be temporal-operator-free with placement
rules; `always[N]` only at top level or under boolean connectives; `next` not
inside a bounded-eventually body) are enforced by the shared monitor compiler,
identical in every backend. Formulas outside R1-R4 parse + validate succeed;
codegen in EVERY backend emits the same skip warning and generates no monitor
for that label.
**Why:** the backend parity principle (INTRO.md) - same functionality and
exactly the same observable behavior in ALL backends, with differential
testing meaningful; Rust-only monitors would leave C MC silently skipping
liveness assumptions and property detection.
Table: DESIGN.md section 21, D-P3-5 - Code: `src/monitor.rs::compile_formula`,
`src/backends/c_fsm.rs::emit_monitors`, `tests/backend_parity_test.rs`.

### D-P3-6. CLI flag and per-language gating - `--with-property-monitors` on production only

`--with-property-monitors` applies to the PRODUCTION command of every backend;
mc and test always include monitors; gating is per-language with
zero-cost-when-disabled semantics (Rust `#[cfg(debug_assertions)]`, C
`#ifdef RFS_PROPERTY_MONITORS`). New generated error surface: Rust
`StepError::PropertyViolation { label, step }`; C `RFS_PROPERTY_VIOLATION`
status in `step_result_t` with `prop_label`/`prop_step`/`prop_trigger_step`
fields.
**Why:** release production builds pay zero monitor cost; MC/tests never need
an opt-in flag (violations are the point - MC must not be able to disable
them); one flag across backends keeps the CLI uniform.
Table: DESIGN.md section 21, D-P3-6 - Code: `src/codegen.rs::PropertyMonitorMode`,
`src/backends/rust_fsm.rs::emit_monitors`,
`src/backends/c_fsm.rs::render_prop_monitors`.

<a id="dev:decisions-p4-p5"></a>
## Phase 4 (D-P4-1..14) + Phase 5 (D-P5-1..14)

Phase 4 shipped scalable model checking as implemented (plan:
`docs/roadmap/PHASE_4_PLAN.md`; full semantics: DESIGN.md section 11). Phase 5
shipped the full toolchain and production integration (plan:
`docs/roadmap/PHASE_5_PLAN.md`; full command behavior: KB.md section 4.5).
Each entry below summarizes the shipped decision with its rationale and links
to the implementing code.

### D-P4-1. Architecture: shared codegen-time plan, per-backend generated engines

`src/mc_plan.rs` computes the plan once (abstraction, encoding, traversal spec,
obligation decomposition); both backends embed it as constants/tables, so
behavior is identical by construction. The SMT mode reuses the pattern via
`src/smt_plan.rs` + `src/smt_emit.rs`.
**Why:** one plan rendered twice makes cross-backend parity a construction
guarantee rather than a test hope - the differential suites then only need to
pin rendering, not semantics.
Table: DESIGN.md section 22, D-P4-1 - Code: `src/mc_plan.rs::McPlan`,
`src/smt_plan.rs::build_smt_spec`, `src/smt_emit.rs::emit_rust_spec`.

### D-P4-2. Exact engine: level-synchronous BFS over configurations

Configurations = (state index, packed vars, monitor states); visited at
enqueue; parent pointers for counterexamples; canonical successor order
(events/fields in declaration order, values ascending). Determinism is
independent of container choice - Rust and C report identical
status/counterexample (parity suites pin this).
**Why:** level-synchronous BFS with a visited set is the standard complete
reachable-state search: it terminates on finite state spaces, finds the
SHORTEST counterexample by construction, and canonical ordering makes results
reproducible across backends and runs.
Table: DESIGN.md section 22, D-P4-2 - Code: `src/backends/rust_mc.rs::exact_codegen`,
`src/ref_mc.rs::run`, `src/mc_plan.rs::TraversalSpec`.

### D-P4-3. Input abstraction

A payload field is abstracted to a single representative value iff no
transition guard, output payload expression, variable update expression, or
property/assumption formula references it (`input_is` patterns with field
values count as references). Soundness arguments for safety and liveness:
DESIGN.md section 11.
**Why:** unreferenced fields cannot influence behavior, so collapsing them to
one representative value shrinks the state space without losing any reachable
behavioral distinction - the main scalability win for payload-heavy protocols.
Table: DESIGN.md section 22, D-P4-3 - Code: `src/mc_plan.rs::InputAbstraction`,
`src/mc_plan.rs::collect_input_is_refs`.

### D-P4-4. Statuses and summary line

`pass-complete` / `fail` (reason: `property-violation` | `liveness-violation`
| `deadlock`) / `unknown-state-limit`; exit codes 0/1/2; machine line
`model-check summary: status=<s> states=N transitions=M max_depth=D` plus
` reason=<r> label=<L> step=S` on fail; human lines `PASS (complete)` /
`FAIL` / `UNKNOWN (state limit reached)`.
**Why:** a three-status taxonomy with distinct exit codes lets CI distinguish
"proven", "bug found", and "incomplete" - presenting a bounded run as full
proof would be unsound, so incompleteness is a first-class status.
Table: DESIGN.md section 22, D-P4-4 - Code: `src/codegen_mc.rs::ExactStatus`,
`src/codegen_mc.rs::FailReason`.

### D-P4-5. Deadlock and unexpected-input semantics

No-successor configuration = FAIL(deadlock); inputs matching no transition
while others are enabled terminate that path and are counted as
`unexpected_terminations` without changing the status.
**Why:** a state with no enabled successor is a genuine protocol deadlock and
must fail; an unmatched input on a path where other inputs work is an
environment misbehavior, not a spec bug - counting it keeps the signal visible
without false failures.
Table: DESIGN.md section 22, D-P4-5 - Code: `src/backends/rust_mc.rs::exact_codegen`,
`src/codegen_mc.rs`.

### D-P4-6. Assumptions prune exploration

Active assumptions remove violating inputs from the search; `--assumes none`
explores everything.
**Why:** assumptions declare environment guarantees; pruning on them verifies
the protocol under its contract, while the escape hatch keeps
assumption-free (stress) verification available without spec edits.
Table: DESIGN.md section 22, D-P4-6 - Code: `src/mc_plan.rs::McPlan`,
`src/codegen.rs::AssumptionSelection`.

### D-P4-7. Modes and flags

`--mode exact|fuzz`, `--max-states N` (default 1,000,000), `--nondet`,
`--timeout-sec N` (default 0 = off; wall-clock only, NOT parity-guaranteed),
`--threads N` (default 1; byte-identical results for any N), runtime-only
`--fast` (non-parity early abort); codegen flag `--smt-bound N` (default 10)
for the SMT mode. Plan deviation: the `--timeout-sec` help lives in
`src/codegen.rs` (generic codegen help) and the per-backend command
descriptions (`rust.rs`/`c.rs`) - NOT in `main.rs` usage text, which does not
document MC flags.
**Why:** exact is the parity-guaranteed default; fuzz/bounded variants are
explicit opt-ins with documented non-parity caveats, so a flag can never
silently change what "pass" means.
Table: DESIGN.md section 22, D-P4-7 - Code: `src/codegen.rs::McMode`,
`src/codegen.rs::common_help`.

### D-P4-8. Liveness language: `eventually (p)` + infix `until` (WEAK until)

The `until` precedence level sits between `&&` and `!` such that `!` binds
TIGHTER than `until` (`!a until b` = `(!a) until b`) and `&&`/`||` bind looser
(`a && b until c` = `a && (b until c)`). The plan's EBNF sketch contradicted
its own pinned consequence; the implementation follows the pinned consequence
(standard PSL/CTL), recorded in `PHASE_4_TASK_5.md` "Task corrections". IR:
additive `Formula::Eventually` / `Formula::Until`, no version bump.
**Why:** standard PSL/CTL precedence keeps the language predictable for anyone
who knows temporal logic; pinning the consequence over the sketch resolved the
internal contradiction in favor of the conventional reading.
Table: DESIGN.md section 22, D-P4-8 - Code: `src/parser.rs::primary_formula`,
`src/ir.rs::Formula`.

### D-P4-9. Liveness scope

Top-level obligations `eventually p` / `p until q` where p, q are BOOLEAN
formulas over atomic propositions (no temporal operators inside); obligations
combinable at top level with `&&`/`||`/`!`/`->`. Deeper nesting -> codegen
diagnostic + property skipped (binary still compiles/runs). Any formula
containing eventually/until is outside the monitor subset (identical skip
warning, MC-only).
**Why:** restricting temporal operators to top-level obligations over boolean
atoms keeps the liveness algorithm (per-leaf progress automata) tractable and
the monitor/MC split clean - monitors stay online-decidable, liveness stays an
MC feature.
Table: DESIGN.md section 22, D-P4-9 - Code: `src/mc_plan.rs::LivenessDecomposition`.

### D-P4-10. Liveness algorithm: per-leaf progress automata on the completed graph

As implemented WITH THE CORRECTED WEAK-UNTIL EQUATION (plan correction in
DESIGN.md section 11): neg(p until q) = backward reachability, staying in
{pos: ¬q}, to {pos: ¬p}. Verified against brute-force lasso enumeration (1200
equation + 2000 full-algorithm checks, 0 mismatches). Bounded mode: truncated
analysis - sound FAIL if the witness is inside the explored part, else
`unknown-state-limit`; never a false pass-complete.
**Why:** analyzing liveness on the COMPLETED graph (after safety exploration)
with per-leaf progress automata avoids product-construction blowup; the
corrected weak-until equation was validated against brute force before being
trusted, and the bounded mode preserves soundness by never claiming
pass-complete on a truncated analysis.
Table: DESIGN.md section 22, D-P4-10 - Code:
`src/mc_plan.rs::LivenessDecomposition`, `src/backends/rust_mc.rs::exact_codegen`,
`src/ref_mc.rs::run`.

### D-P4-11. Fairness policy

Deterministic mode: environment fairness by construction, no filter.
`--nondet`: strong transition fairness - a bad cycle counts only if closed
(every enabled successor of each cycle state is taken on the cycle);
implemented by restricting stay-edges to fully expanded configs. Discriminator
test-pinned (det FAIL vs nondet pass-complete).
**Why:** in deterministic mode the environment is fixed, so fairness is
automatic; under `--nondet` an open cycle that merely avoids progress is not a
liveness violation - strong transition fairness filters exactly those, and the
discriminator test pins the det/nondet difference.
Table: DESIGN.md section 22, D-P4-11 - Code:
`src/backends/rust_mc.rs::exact_codegen`, `src/codegen.rs::McMode`.

### D-P4-12. SMT bounded mode (optional)

Gate PASSED, IMPLEMENTED in task 11. z3 5.3.1 at `/workspace/host/z3`
(TOOLCHAIN.md section 5); feature-gated `smt` cargo feature (z3 crate 0.21 /
z3-sys 0.13.1) - default builds unaffected. `--mode smt-bounded`: k-step
unrolling over bitvectors from the shared plan/emitter (parity by
construction), k-liveness, exit codes 0/1/2, PASS always labeled incomplete.
v1 scope rejections: assumptions, on_error blocks, enum-typed vars, property
formulas beyond top-level always/eventually over boolean combinations of
expressions. Details: DESIGN.md section 11 + `PHASE_4_TASK_11_STATUS.md`
(incl. semantic bugs found/fixed: shift-amount width mismatch,
oversized-literal comparison wrap, z3 5.3.1 C API deltas).
**Why:** SMT unrolling finds bugs beyond the reachable-state budget with wider
bitvectors, and deriving it from the SAME shared plan keeps Rust/C parity by
construction; feature-gating keeps the default toolchain dependency-free, and
labeling every PASS incomplete preserves the soundness discipline of D-P4-4.
Table: DESIGN.md section 22, D-P4-12 - Code: `src/smt_plan.rs::build_smt_spec`,
`src/smt_emit.rs::emit_c_spec`.

### D-P4-13. Benchmarks

Three protocol-like specs in `examples/valid`: 43 sliding-window ARQ (task 6;
liveness + abstraction showcase), 44 token ring and 45 elevator (task 8;
sizing 129,985 / 11,098 states). Benchmark suite
`tests/benchmark_mc_test.rs` (task 9/10B): pass-complete metric parity in both
backends, inline FAIL variants with counterexample parity, `--threads`/
`--fast` gates, JSON report (`target/mc_benchmarks.json`, development
artifact).
**Why:** real protocol shapes (not toy FSMs) are the only honest sizing data
for the exact engine, and pinning deterministic metrics as equality bounds
keeps CI stable while still catching regressions.
Table: DESIGN.md section 22, D-P4-13 - Code: `tests/benchmark_mc_test.rs`.

### D-P4-14. Test migration to seedless exact mode

`backend_parity_test.rs` + `monitor_integration_test.rs` migrated from
same-seed fuzz to seedless exact mode; values re-pinned from the deterministic
traversal - including 8 NEW deadlock findings (terminal states the fuzz walk
never reported) and 1 new property violation.
**Why:** exact mode makes parity tests deterministic and complete - the
migration immediately found real spec deadlocks that seeded random walks had
missed, proving the stronger mode pays for itself.
Table: DESIGN.md section 22, D-P4-14 - Code: `tests/backend_parity_test.rs`,
`tests/monitor_integration_test.rs`.

### D-P5-1. Workspace layout (task 1)

The single `rfs` package became a cargo workspace with an EXPLICIT member list
(no globs - `crates/examples` has no manifest): root package `rfs` +
`crates/rfs-build` + `crates/rfs-adapters` + example crates (`sync_handler`,
`async_handler`, `fuzz_rust`). The C example projects (`c_sync_handler`,
`fuzz_c`) are Makefile-driven and NOT cargo members. Plain
`cargo build/test/clippy` at the root still select only the root package;
`--workspace` covers everything. All `.cargo/config.toml` aliases re-verified
unchanged after the conversion.
**Why:** an explicit member list keeps the workspace stable against new
directories, and keeping C projects Makefile-driven avoids forcing a cargo
manifest on plain-C code; root-package-only default commands preserve every
existing alias's behavior.
Table: DESIGN.md section 23, D-P5-1 - Code: `crates/rfs-build/src/lib.rs`,
`crates/rfs-adapters/src/lib.rs`.

### D-P5-2. Hand-rolled JSON reader-writer, ALL sources (task 2)

A full hand-rolled JSON READER + WRITER (`src/json_rw.rs`) replaced
serde/serde_json EVERYWHERE: IR serialization/round-trip, all `--json` CLI
outputs (info/validate/MC counterexample), and the generated MC JSON via an
embedded copy of the writer. Rationale: guarantee KEY ORDERING (declaration
order, pinned) and ALWAYS byte-identical text output across platforms, Rust
versions, and dependency updates - external crates give no such contract; it
also keeps the toolchain dependency-light and consistent with the
generated-code constraint. Correction C3: the sketch's infallible `to_json`
became FALLIBLE `Ir::to_json(&self) -> Result<Json, String>` - unsigned IR
fields can hold values up to `u64::MAX` (the parser accepts them), which the
i64 value model cannot represent; failing with a clear error beats silent
clamping. Output proved byte-identical to the pre-migration format on all 45
valid examples, so no test re-pinning was needed.
**Why:** byte-identical JSON is part of the cross-backend parity contract and
of artifact reproducibility; only owning the serializer guarantees pinned key
order forever, and a fallible writer turns an unrepresentable value into a
diagnostic instead of corrupted output.
Table: DESIGN.md section 23, D-P5-2 - Code: `src/json_rw.rs::parse`,
`src/ir.rs::to_json`.

### D-P5-3. rfs-build: build.rs helper (task 1)

`crates/rfs-build` is a dependency-free Rust API for build scripts: locates
the toolchain (`RFS_BIN` env var first - empty treated as unset, nonexistent
path an error - then `rfs` on PATH with a printed note), runs `codegen
<backend> production|test`, writes the artifact to `OUT_DIR/<stem>.rs|c`, and
emits the cargo rerun/include directives. Builder options: `.backend()`,
`.production()` (default) / `.test_command()`, `.with_property_monitors(bool)`,
`.no_std(bool)`, `.with_protocol_fsm(bool)`. Consumer pattern (task 11
correction C11-1): `include!` rejects inner attributes, so build.rs strips the
`#!` lines into a body file in OUT_DIR and lib.rs uses `include!` under a
mirroring outer `#[allow]` - generated code stays out of the source tree (cargo
fmt never sees it).
**Why:** a dependency-free helper with explicit rerun directives makes
generate-at-build-time correct under cargo's incremental model, and keeping
generated artifacts in OUT_DIR keeps the source tree (and fmt/clippy) clean.
Table: DESIGN.md section 23, D-P5-3 - Code:
`crates/rfs-build/src/lib.rs::RfsBuild`, `crates/rfs-build/src/lib.rs::resolve_rfs_bin`.

### D-P5-4. rfs info: static spec introspection (task 3)

New stable command: per module, states, events/outputs with variants + field
types, variables with types + init, transition count, and properties/
assumptions each with monitor-subset status (`monitored` / `skipped:
reason`). File input only (`.rfs`/`.json` by extension); NO model checking.
`--json` emits a machine-readable document (stable key order, ALL keys present
- empty arrays and null instead of omitted fields) for IDE/tooling reuse;
`--log-level` filters stderr progress. Exit 0 valid / 1 parse-validation error.
**Why:** cheap static introspection answers "what is in this spec" without
running anything, and the all-keys-present JSON shape gives tooling a stable
schema to code against.
Table: DESIGN.md section 23, D-P5-4 - Code: `src/info.rs::run_info`.

### D-P5-5. rfs test: generate + compile + run (tasks 7/17)

One invocation per selected suite (`--suite unit|fuzz`, repeatable or
comma-separated, default `unit`): codegen the `test` artifact into a temp dir
(cleaned on every exit path), compile it (`rustc --edition 2021 --test` /
`gcc -Wall`), run it; per-test results on stdout, progress on stderr. The fuzz
suite (task 17) builds + bounded-runs the libFuzzer target from the EMITTED
skeleton extra file: C = `clang -O1 -fsanitize=fuzzer,address` over
`fuzz_target.c` ONLY (compiling the production source too is a
multiple-definition link error - the skeleton `#include`s it under its
hardcoded name), bounded run with `ASAN_OPTIONS=detect_leaks=0` merged (the
generated C FSM has no free API; ASan reads env only); Rust = a temp
cargo-fuzz scaffold (lib root = generated production source, adapted skeleton,
`[package.metadata] cargo-fuzz = true`, `libfuzzer-sys = "=0.4.13"` pinned to
the local registry cache for offline builds, `rust-toolchain.toml` pinning
nightly because cargo-fuzz needs `-Zsanitizer=address`, with a non-nightly
inherited `RUSTUP_TOOLCHAIN` stripped from the subprocess) run via `cargo fuzz
build` + `cargo fuzz run fuzz_target -- -max_total_time=N`. Missing
clang/cargo-fuzz is an error ONLY when the fuzz suite is selected - the
default unit path stays tool-free. Exit 0 = every selected suite passes;
1 = test failure, fuzzer crash, compile error, missing tool for a selected
suite, or usage/parse/validation error.
**Task 16 close-out correction to C17-2:** the generated C core now carries an
explicit `fsm_free` lifecycle end and the fuzz skeleton calls it once per input
(single-exit decode) - per-input allocations no longer accumulate, so long runs
stay within libFuzzer's RSS limit (regression: spec 01 OOM'd at ~200k runs
before the fix). The `detect_leaks=0` merge stays pinned as defense in depth.
**Why:** one command that generates, compiles, and runs closes the loop from
spec to executed test with no manual steps; strict tool availability (error
only for the selected suite) keeps the default path dependency-free while CI
can opt into fuzzing.
Table: DESIGN.md section 23, D-P5-5 - Code: `src/test_cmd.rs::run_test`.

### D-P5-6. rfs diagram: plugin registry, strict rendering (task 8)

State diagrams + tables as plugins mirroring the codegen backend pattern
(`src/diagram.rs` + `src/diagrams/`): v1 formats `dot` (Graphviz), `puml`
(PlantUML state diagram - edge label `event \n guard`, emitted data via
`note on link`), `mmd` (Mermaid stateDiagram-v2 - all info in multi-line edge
labels), plus `table`/`md` transition tables + variable/event summaries.
PRIMARY behaviour is pure text generation with NO tool calls. `--render svg`
(requires `--out FILE.svg`; source written next to it) invokes the resolved
renderer (`dot -Tsvg` / `plantuml -tsvg` / `mmdc` + auto temp puppeteer config
for root). STRICT error semantics (user decision 2026-10-21): missing/failing
render tool = exit 1 + diagnostic incl. captured tool stderr, NO graceful
degradation - RFS runs in CI/CD pipelines, a command either does what was
requested or throws; users compose their own fallbacks (e.g. via
`--print-script`). Runs on the first module of the spec.
**Why:** pure-text generation as the primary behaviour keeps the command usable
with zero external tools, and strict rendering semantics are a CI/CD
requirement - silent fallback to a stale or missing diagram would hide broken
pipelines.
Table: DESIGN.md section 23, D-P5-6 - Code: `src/diagram.rs::DIAGRAMS`.

### D-P5-7. External-tool parameterization (task 3, extended task 17)

Every CLI command that invokes external tools/compilers/build systems is
parameterizable by tool PATHS and additional tool PARAMETERS. Environment
variables are the PREFERRED mechanism - `RFS_RUSTC`/`RFS_GCC`/`RFS_DOT`/
`RFS_PLANTUML`/`RFS_MERMAID` (task 3) + `RFS_CLANG`/`RFS_CARGO_FUZZ` (task 17)
for paths and the matching `*_ARGS` vars (space-separated extra parameters);
rfs-build uses `RFS_BIN`. Per-command CLI flags (`--rustc PATH`,
`--rustc-args "..."`, ... incl. task 17's `--clang/--cargo-fuzz` pair)
override the env vars; without either, the bare tool name is looked up on
PATH. One shared resolver + invocation helper (`src/ext_tool.rs`) implements
the precedence (flag > env var > PATH) and renders/executes command lines
identically for all commands; task 17 extended `ExtTool` with a fixed
`subcommand` field (cargo-fuzz = resolved cargo path + `fuzz`, rendered before
extra args).
**Why:** one resolver with one documented precedence eliminates per-command
drift in tool handling, and env-var preference lets CI pin toolchains globally
while local overrides stay possible per invocation.
Table: DESIGN.md section 23, D-P5-7 - Code: `src/ext_tool.rs::ExtTool`.

### D-P5-8. --print-script: script-generation mode (task 3)

Commands that invoke external tools accept `--print-script`: instead of
executing, they print a deterministic POSIX sh script to stdout (shebang,
`set -e`, header comment with rfs version + spec + RESOLVED tool paths/args,
one commented step per invocation) and exit 0. Self-contained: generated
sources are embedded in quoted heredocs (`rfs test`: the full test artifact;
`rfs diagram --render svg`: the diagram source, plus a temp puppeteer-config
step for mermaid). Output is unit-test-pinned; the scripts were manually
executed with `sh` during tasks 7/8/17 (exit 0 + self-cleanup).
**Why:** a self-contained script makes every tool invocation reproducible and
auditable outside rfs (CI debugging, air-gapped machines, custom wrappers)
while the pinned output keeps the resolver's decisions visible and testable.
Table: DESIGN.md section 23, D-P5-8 - Code: `src/ext_tool.rs::Script`.

### D-P5-9. no_std production codegen (task 9)

`--no-std` applies to the PRODUCTION command only (`test`/`mc` reject it with
the pinned `NO_STD_PRODUCTION_ONLY_ERROR` at CLI and library level - they need
std's allocation-heavy machinery). Generated Rust becomes `#![no_std]` +
`extern crate alloc` with an IDENTICAL public API shape (Vec/String/ToString
from alloc; body byte-identical to the std variant - pinned by a 45-example x
2-modes compile sweep + body-parity check). The C backend accepts the flag as
a documented no-op (plain C is already dependency-free) adding one header
comment line.
**Why:** embedded targets need the same generated FSM without std, and an
identical API shape means porting a handler is a build-flag change, not a code
change; rejecting test/mc keeps the flag's contract precise.
Table: DESIGN.md section 23, D-P5-9 - Code:
`src/codegen.rs::NO_STD_PRODUCTION_ONLY_ERROR`, `src/codegen.rs::BackendOptions`.

### D-P5-10. MC counterexample JSON (task 10)

`--counterexample-format json|text` on `codegen <backend> mc` (and at runtime
on the generated binary); default `text` - the parity-pinned human-readable
report is byte-for-byte unchanged. `json` emits ONE machine-readable document
mirroring the text report: status, metrics (states/transitions/max_depth),
reason/label/step (fail only), statistics block, and the counterexample as
structured data (`trace` / lasso with prefix+cycle+cycle_length /
deadlock-config). Exact mode only - fuzz and smt-bounded keep their own
output. Parity extends to byte-identical JSON across backends for the same
spec/mode (pinned over ALL property-bearing examples + a golden full-document
pin). The generated binary carries an EMBEDDED copy of the hand-rolled writer
(D-P5-2) - generated code stays dependency-free, and key ordering is part of
the byte-parity contract.
**Why:** tooling (bug trackers, CI dashboards) needs machine-readable
counterexamples, and byte-identical JSON across backends extends the existing
parity contract to the structured form without a second source of truth.
Table: DESIGN.md section 23, D-P5-10 - Code:
`src/codegen_mc.rs::CounterexampleFormat`.

### D-P5-11. ProtocolFsm trait + adapters (task 11)

The Rust-ecosystem integration layer: `crates/rfs-adapters` provides the
`ProtocolFsm` trait (associated `Input`/`Output`/`Error` + `new()` +
`step(&mut self, input) -> Result<Vec<Output>, Error>` - the exact shape of
every generated Rust FSM) plus FOUR generic adapters, none depending on
generated code: `channel` (FSM on its own thread, std mpsc, FIFO; error
outcomes don't stop the thread), `async_adapter` (futures Stream; steps
synchronously in `poll_next` so ANY executor works - no tokio; Pending until
close()), `callback::drive(fsm, decide, max_steps)` (reactive loop seeing
previous outputs), `minimal` (core-only step-forwarding wrapper that is itself
a ProtocolFsm - composes). Because self-contained generated code is a design
invariant, the trait CANNOT be linked into generated code: codegen OPTIONALLY
emits `impl ProtocolFsm for Fsm` behind `--with-protocol-fsm` (OFF by default
- every byte of default output unchanged; the one documented exception to
invariant 1 in `CODEGEN.md` section 9), placed after the inherent `impl Fsm`,
before `#[cfg(test)]` in test artifacts, forwarding explicitly so it cannot
recurse. `mc` rejects the flag (pinned `PROTOCOL_FSM_MC_ERROR`, CLI + library
level, both backends); the C backend accepts and ignores it byte-identically.
Rust-ecosystem-only by design per the parity principle (no C trait exists).
The crate is `#![no_std]` with `std` (default) / `async` features; futures 0.3
is optional behind `async` - the only external dependency of the workspace
beyond the root package, never leaking into the toolchain or generated code.
**Why:** a trait matching the generated FSM shape exactly lets generic adapters
compose with ANY spec's FSM without the adapter crate depending on generated
code; opt-in emission keeps the self-contained-code invariant intact by
default.
Table: DESIGN.md section 23, D-P5-11 - Code:
`crates/rfs-adapters/src/lib.rs::ProtocolFsm`,
`crates/rfs-adapters/src/channel.rs::ChannelFsm`,
`crates/rfs-adapters/src/async_adapter.rs::AsyncFsm`,
`crates/rfs-adapters/src/callback.rs::drive`,
`crates/rfs-adapters/src/minimal.rs::Minimal`,
`src/codegen.rs::PROTOCOL_FSM_MC_ERROR`.

### D-P5-12. Extended test generation: one plan, both backends (tasks 4/5/6)

The shared planner (`src/codegen_test.rs`) was extended with language-agnostic
plans for per-transition unit tests (representative inputs + statically
computed guard taken/not-taken cases; sibling-priority/shadowing rules),
invariant tests (safety property atoms re-checked at every driven step),
property-monitor debug tests (injected violating trace must fire the monitor -
planner guarantee: no higher-firing sibling preempts, task 5 correction C5-1),
seeded random-trace tests (deterministic LCG, fixed seed 12345, K steps via
`--steps`, one byte-identical coverage line), and fuzz-target skeleton planning
(per-variant decode strategy). ONE plan rendered by BOTH backends with
identical coverage and stable test names - the parity principle made
executable (the cross-backend suite compares per-test result lists + the
random-trace line). Fuzz skeletons are SEPARATE extra files
(`fuzz_target.rs`/`fuzz_target.c`) so the main artifact (and the aggregate
generated-code crate) stay dependency-free; the CLI writes them next to
`--out` and refuses stdout mode (strict - no silent file loss).
**Why:** one language-agnostic plan rendered twice makes test coverage identical
across backends by construction, and separating fuzz skeletons keeps the main
artifact free of fuzzer dependencies.
Table: DESIGN.md section 23, D-P5-12 - Code:
`src/codegen_test.rs::build_test_plan`, `src/codegen_test.rs::build_fuzz_decode`.

### D-P5-13. IDE support + structured diagnostics (task 13)

In-repo editor support under `ide/` (TextMate grammar for `.rfs`, scope
`source.rfs`; snippet set; VS Code/Sublime install notes) - full LSP remains a
non-goal. The parser's keyword list became the single source of truth
(`parser::KEYWORDS`, 28 words) with a bidirectional drift-guard test (no
call-site refactor of the byte-sensitive parser, correction C13-2). CLI
diagnostics integration: `rfs validate --json` emits structured parse/semantic
errors on stdout - parse errors carry real 1-based line/col from
`ParseError::line_col` (which can read 1:1 when the first module fails
wholesale - the top-level `many0` matches zero modules), semantic errors carry
no span and are pinned to 0/0 (C13-1); file mode = one document, directory mode
= an array per failing file; JSON mode writes NO `.err` side files (editors
consume stdout; C13-3); pretty-printed like `info --json` (D13-1). Exit codes
unchanged.
**Why:** editors need machine-readable diagnostics on stdout with stable
shapes, and making the parser's keyword list the grammar's source of truth
prevents syntax highlighting from drifting from what the parser accepts.
Table: DESIGN.md section 23, D-P5-13 - Code: `src/parser.rs::KEYWORDS`,
`src/frontend.rs::run_validate_structured`.

### D-P5-14. Example-crate matrix (tasks 11/12)

Rust: `sync_handler` (spec-43 sliding-window ARQ over std mpsc channels via the
channel adapter) and `async_handler` (the same FSM as a futures Stream) - both
generate via rfs-build with `--with-protocol-fsm`, carry their session as a
test + demo binary, and observe identical pinned behavior (19 ticks, in-order
[0..8] frames/acks, 5 dropped out-of-band acks, max window exactly 4). C
mirror: `c_sync_handler` (Makefile-driven plain-C scripted peer; `gcc -Wall`
zero-warning bar; scenario data lines byte-identical to the Rust side).
Fuzzing harness examples for BOTH ecosystems wiring the emitted skeletons:
`fuzz_rust` (cargo-fuzz layout, detached `fuzz/` workspace) and `fuzz_c`
(clang + libFuzzer; generation works without clang, building needs it).
**Why:** end-to-end example crates prove the integration story (build.rs +
adapters + generated FSM) on a real protocol in both ecosystems, with pinned
scenario values making Rust/C behavioral parity checkable line by line.
Table: DESIGN.md section 23, D-P5-14 - Code:
`crates/examples/sync_handler/src/lib.rs`,
`crates/examples/async_handler/src/lib.rs`.

<a id="dev:decisions-p6-p7"></a>
## Phase 6 (D-P6-1..11) + Phase 7 (D-P7-1..13)

TBD (task 5.4).
