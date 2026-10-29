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

TBD (task 5.2).

<a id="dev:decisions-p4-p5"></a>
## Phase 4 (D-P4-1..14) + Phase 5 (D-P5-1..14)

TBD (task 5.3).

<a id="dev:decisions-p6-p7"></a>
## Phase 6 (D-P6-1..11) + Phase 7 (D-P7-1..13)

TBD (task 5.4).
