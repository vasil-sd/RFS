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

TBD (task 5.3).

<a id="dev:decisions-p6-p7"></a>
## Phase 6 (D-P6-1..11) + Phase 7 (D-P7-1..13)

TBD (task 5.4).
