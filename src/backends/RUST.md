# Rust Codegen Backend

Architecture and design decisions of the Rust target-language backend
(`src/backends/rust*.rs`). For the generic plugin API see `CODEGEN.md` at the
repository root; for semantics see `DESIGN.md`.

## 1. File layout

| File | Role |
|---|---|
| `rust.rs` | Plugin entry: `RUST_BACKEND` static (name, description, command descriptors, function pointers) plus shared Rust-specific helpers: `RUST_KEYWORDS`, `rust_ident` (keyword-safe identifier), `snake` (case conversion), `bit_rust_type` / `mask_expr` (fixed-width integer mapping), `rust_type` (IR type → Rust type), `representative_input_expr`. |
| `rust_fsm.rs` | `production` command: emits the standalone FSM source. Also the core reused by `test` and `mc`. |
| `rust_test.rs` | `test` command: appends a unit-test module (module-level + per-transition/invariant/monitor-violation/random-trace classes) to the production code and emits the separate `fuzz_target.rs` skeleton as an extra file; plus the aggregate test-crate helpers used by the cargo tooling (`generate_aggregate_test_crate`, `write_generated_crate`, `GeneratedCrate`). |
| `rust_mc.rs` | `mc` command: (default exact / legacy fuzz) production FSM core + exact engine rendered from the embedded plan + fuzz runtime + a generated `main` dispatching on `--mode`; (SMT mode) a standalone z3-based binary from the shared `smt_plan`/`smt_emit` spec section. |

Dependency direction inside the backend: `rust_mc → rust_fsm (+ MC_RUNTIME)`,
`rust_test → rust_fsm (via generate_with_tests)`, all three → shared helpers in
`rust.rs`, and all of them → the language-agnostic core modules
(`codegen_fsm`, `codegen_test`, `codegen_mc`).

## 2. Generated artifact design

Every command emits **one self-contained `.rs` file**:

- no dependency on the `rfs` crate or any external crate; compiles with stable
  Rust, edition 2021;
- header `#![allow(dead_code, unused_variables, ...)]` so generated code is
  warning-clean even for degenerate modules (no variables, no outputs);
- deterministic: same IR + options ⇒ byte-identical output.

**no_std production variant (Phase 5 task 9)**: with `--no-std` the file header
becomes `#![no_std]` + `extern crate alloc;` + `use alloc::string::{String,
ToString};` + `use alloc::vec::Vec;` (+ `use alloc::vec;` for the `vec!` macro
only when some transition emits outputs). The BODY is byte-identical to the std
variant - the public API shape is unchanged (alloc provides Vec/String/ToString;
`panic!` comes from core). test/mc reject the flag (`NO_STD_PRODUCTION_ONLY_ERROR`).
Verified by `no_std_compile_sweep_and_body_parity_all_valid_examples`
(tests/codegen_test.rs): all 45 valid examples x both monitor modes compile
under `rustc --edition 2021 --crate-type lib` and their bodies diff empty.

### 2.1 Production FSM anatomy (`rust_fsm.rs`)

The generated file contains, in order:

1. `pub enum State { ... }` — one variant per RFS state.
2. `pub struct Vars { ... }` — module variables (unit struct when there are
   none).
3. `pub enum Input { ... }` / `pub enum Output { ... }` — **flattened**
   constructors: each variant of every event/output group becomes a top-level
   variant named by `VariantInfo::unique_name` (mangled through
   `rust_ident`), carrying the constructor's fields. Flat enums keep the
   transition `match` exhaustive and simple for downstream users.
4. Sample + trace runtime: `Sample { input: Option<Input>, outputs,
   from_state, to_state, vars }` — the per-sample record the monitors evaluate
   (sample 0 has no input/outputs); `TraceStep`/`Trace` keep the complete
   execution record for counterexample printing.
5. Assumption runtime: `AssumptionPolicy { Continue, Stop, Panic }`,
   `trait AssumptionHandler`, and the three built-in handlers
   (`Panic/Stop/Continue`). `Fsm::step` is the plain entry point (panic on
   violation); `Fsm::step_with_handler` delegates policy decisions to a caller
   handler.
6. `pub enum StepError { UnexpectedEvent, AssumptionViolation { label },
   #[cfg(debug_assertions)] PropertyViolation { label, step } }` — the
   property-violation variant is debug-gated in production output (D-P3-6);
   it is always present in test/mc output.
7. Online property/assumption monitors (Phase 3, section 2.3) — rendered from
   the shared `MonitorPlan`; the pre-Phase-3 post-hoc `property_check`/
   `assume_check` trace functions are GONE (D-P3-4).
8. `pub struct Fsm { state, vars, trace, active_assumes, assume_monitors,
   #[cfg(debug_assertions)] prop_monitors, #[cfg(debug_assertions)]
   property_violation: Option<PropertyViolation> }` with `new()` and
   `step(input) -> Result<Vec<Output>, StepError>`: the transition block is a
   `match` on `(state, input)`; guards are lowered expressions over old
   values; variable updates are **simultaneous** (all assignments read pre-step
   values); outputs are collected per step. `new()` steps one initial sample
   (position 0) so monitor countdowns start correctly; `commit_step` steps the
   monitors online after every successful transition.

**Optional ProtocolFsm impl (Phase 5 task 11, D-P5-11):** with
`--with-protocol-fsm` (OFF by default - every byte of default output unchanged)
the production and test commands append one block AFTER the inherent `impl Fsm`
(before `#[cfg(test)]` in test artifacts):
`use rfs_adapters::ProtocolFsm;` + `impl ProtocolFsm for Fsm { type Input =
Input; type Output = Output; type Error = StepError; fn new() -> Self {
Fsm::new()} fn step(&mut self, input) -> Result<Vec<Output>, StepError> {
Fsm::step(self, input)} }` - forwarding explicitly so it cannot recurse. This
is the ONE documented opt-in that links a generated artifact to an external
crate (the consumer must then depend on `rfs-adapters`). `mc` rejects the flag
(pinned `PROTOCOL_FSM_MC_ERROR`, CLI + library level); the C backend accepts
and ignores it byte-identically. Byte-pinned: flag-on output = default output
+ the block exactly once.

#### 2.3 Online property/assumption monitors (Phase 3)

Every in-subset `prop`/`assume` compiles (in the shared, language-agnostic
`src/monitor.rs`) to a per-label monitor plan; this backend renders it as:

- one `pub struct Mon_<label>` per formula with `new()` and
  `step(s: &Sample, idx: usize) -> Option<PropertyViolation>`; fields are the
  plan's state - per-sample signal results (`n<i>_now`), failure flags
  (`n<i>_failed`), trigger steps (`n<i>_trigger_step`), obligation countdowns
  (`n<i>_pending`, `n<i>_remaining`), and delay registers for `next`
  (`n<i>_prev_antecedent`, initialized so sample 0 is vacuous);
- aggregate structs `PropMonitors` (properties) and `AssumeMonitors`
  (assumptions); the `PROPERTY_LABELS` / `ASSUME_LABELS` constants stay;
- sticky violation record: a confirmed property violation sets
  `fsm.property_violation = Some(PropertyViolation { label, step,
  trigger_step })` and is never cleared; assumption-monitor violations go
  through the existing `AssumptionHandler` path (so all MC/test call sites keep
  working unchanged - D-P3-4).

Gating (D-P3-6): in PRODUCTION output the property monitors, their `Fsm`
fields, and the `StepError::PropertyViolation` variant are wrapped in
`#[cfg(debug_assertions)]` - debug builds report violations as errors, release
builds have zero monitor cost. ASSUMPTION monitors are never gated (they are
part of step semantics). In TEST and MC output everything is always on (no
gating). Formulas outside the monitor subset (R1-R4) generate no monitor and
warn with the same text as every other backend.

### 2.2 Expression lowering

`gen_expr(e: &Expr, env: &Env, ctx: &ExprCtx) -> Result<String, String>` lowers
the IR expression language to Rust expressions using the validator's typing
environment (`crate::validator::{Env, ExprType}`). Fixed-width integers are
mapped to `u1..=u64` via `bit_rust_type`, with masking helpers (`mask_expr`) so
wraparound semantics match the DSL. Width inference for literals is
context-sensitive (`width_from_type`).

## 3. Unit-test generation (`rust_test.rs`)

The `test` command runs the shared planner from `codegen_test::build_test_plan`
and renders (all names are stable; the C backend mirrors them in task 6):

- **positive test**: drive the FSM with representative inputs up to K steps
  (`--steps`, default 4); when `expect_progress` is set, assert at least one
  step succeeded;
- **negative test (unexpected-input search)**: probe input variants for an
  unexpected event. When `definite_unexpected` holds (a variant no transition
  matches), the test *asserts* the failure is found; otherwise it records the
  attempt without asserting, so fully-covered modules still get a passing test;
- deadlock-style modules (`is_deadlocked_initial`) are covered by the same
  negative path;
- **per-transition tests** `t<id>_<state>_on_<event>` (and `..._not_taken`):
  install the case's state + full variable configuration, drive the concrete
  input with `ContinueAssumptionHandler`, and assert on the RECORDED trace step
  (`to_state` + outputs) - robust even when a property violation halts the
  commit. Not-taken asserts `UnexpectedEvent` + no recorded step, or a landing
  observably different from the transition's own signature;
- **invariant tests** `invariant_<label>`: re-check the property's per-sample
  atom (rendered against `&Sample` by `rust_fsm::render_signal_expr`, shared
  with the monitor rendering) at every sample of the positive drive trace,
  failing with the step index; bounded invariants check samples `< N` only;
- **monitor-violation tests** `monitor_<label>_fires_on_violation`: drive the
  planner's violating trace and assert the sticky `property_violation` record
  holds the target label (the planner guarantees no other monitored property
  records first - see `CODEGEN.md` 5.2);
- **random-trace test** `random_trace_seed<seed>_steps<K>`: drive the
  precomputed LCG index sequence and print one deterministic coverage line
  (`ok`/`unexpected`/`assumption`/`property` counts + distinct visited
  states).

Test code is appended to the production source as `mod tests { ... }`, so the
`test` artifact always contains the full production API plus its tests.

**Fuzz skeleton (separate file).** The command ALSO emits `fuzz_target.rs`
via `CodegenOutput.extra_files`: a `#![no_main]` + `libfuzzer_sys::
fuzz_target!` target that decodes the first byte as variant selector and each
field per the planner's `FuzzDecodePlan`, then loops decoded inputs over
`Fsm::step`. It is deliberately NOT inlined: the main artifact (and the
aggregate generated-code crate) must stay dependency-free. The CLI writes it
next to `--out` and refuses stdout mode for the `test` command (no graceful
degradation). Building/running it with cargo-fuzz is wired by Phase 5 task 12.

### Aggregate tooling

`generate_aggregate_test_crate(files, options) -> GeneratedCrate` renders one
crate with a `pub mod` per module (name-mangled to be collision-free across
files); `write_generated_crate(root, crate_)` writes it to disk. This powers
`cargo generate-code-tests` / `cargo test-generated-code`, which compile and run
all generated tests for every valid example and training snippet.

## 4. Model-checker generation (`rust_mc.rs`, Phase 4)

`generate_model_checker` has two shapes:

**SMT bounded mode** (`--mode smt-bounded` at codegen time): a standalone
z3-based binary - no FSM core, no exact/fuzz runtimes. The spec section is
rendered from the SHARED plan (`smt_plan::build_smt_spec` +
`smt_emit::emit_rust_spec`) so both backends build structurally identical
constraints; `SMT_ENGINE_RUST` supplies the runtime (k-step unrolling over
bitvectors, k-liveness cycle query, exit codes 0 pass-within-bound / 1 fail /
2 unknown, PASS always labeled incomplete). The codegen-time `--smt-bound N`
default is embedded via `{BOUND}`.

**Exact / fuzz modes** (default): the production FSM core (`generate_core`, no
unit tests, property monitors ALWAYS included per D-P3-6) plus three appended
parts dispatched by the generated `main` on the runtime `--mode` flag:

- `EXACT_ENGINE` (default; rendered from the plan by `exact_codegen`):
  level-synchronous BFS over configurations = (state index, packed vars,
  monitor states). Visited check at ENQUEUE time; parent pointers for
  counterexample reconstruction; canonical successor order (events in
  declaration order, payload fields in declaration order, values ascending) so
  results are deterministic and identical to the C backend (D-P4-2). Reports
  the three-status taxonomy + pinned summary line from `codegen_mc`;
  deadlock = FAIL when a reachable configuration has no successor at all;
  unexpected inputs terminate their path and are counted in statistics without
  changing the status; active assumptions prune inputs. Liveness: per-leaf
  progress automata + product limit-vector analysis on the completed graph,
  lasso counterexamples (prefix + one cycle iteration), `--nondet` strong-
  fairness filter, truncated-mode soundness when `--max-states` is hit
  (never a false pass-complete). `--threads N > 1` runs a two-phase per-level
  expansion (`std::thread::scope` workers over disjoint frontier ranges, then
  a sequential canonical commit) - byte-identical results to `--threads 1`;
  runtime-only `--fast` adds a non-deterministic early abort (sound: never a
  false pass).
- `FUZZ_RUNTIME` (`--mode fuzz`, legacy Phase 1/3 engine): embedded RNG
  (deterministic given `--seed`), `codegen_mc::McConfig` CLI wire format
  (`--bound/--runs/--seed/--assumes/--violate/--unexpected-prob`), phase-
  ordered input selection, online property-violation detection.
- liveness diagnostics: formulas outside the D-P4-9 scope (deeper temporal
  nesting) produce codegen WARNINGS and are skipped; any formula containing
  eventually/until is outside the monitor subset (identical skip warning).

**Counterexample report format (Phase 5 task 10, D-P5-10):** the exact engine
renders its report in `text` mode (default - the parity-pinned human-readable
report, byte-for-byte unchanged) or `json` mode (`--counterexample-format
json|text` at codegen time or runtime): one compact machine-readable document
(status, metrics, reason/label/step on fail, statistics, and the counterexample
as structured data - `trace` / lasso with prefix+cycle+cycle_length /
deadlock-config). The JSON is rendered by an EMBEDDED copy of the hand-rolled
writer (D-P5-2) so the generated binary stays dependency-free; key ordering is
part of the byte-parity contract and the document is byte-identical across
backends for the same spec/mode (pinned over all property-bearing examples + a
golden full-document pin). Exact mode only - fuzz and smt-bounded keep their
own output.

Because the generated binary cannot depend on this crate, `rust_mc.rs`
contains embedded copies of the plan tables and parsing helpers. **The formats
in `src/codegen_mc.rs` / `src/mc_plan.rs` are the source of truth** — when a
wire format changes, update both the core definition and the embedded copy,
and extend the tests.

## 5. Key design decisions

1. **Static registry, function pointers.** Backends are compile-time statics in
   `codegen::BACKENDS`; dispatch is a plain function-pointer call through
   `CodegenCommandKind::run`. No trait objects, no dynamic loading — simplest
   possible plugin model for a single-binary toolchain.
2. **Flat Input/Output enums** instead of nested per-group types: exhaustiveness
   and pattern matching stay trivial in user code; the flattening logic is
   shared with all backends via `codegen_fsm::build_variants`.
3. **Trace recording built into production code**, not only into tests/MC:
   property checks, unit tests, and the model checker all consume the same
   trace type, guaranteeing they observe identical behavior.
4. **Assumption handling as a pluggable policy** (`AssumptionHandler` trait) so
   production users can choose panic/stop/continue while the model checker uses
   stop semantics to keep exploring.
5. **MC = production core + runtime**, not a separate translation: the checked
   system and the shipped system are literally the same code, which is what
   makes model-checking results meaningful for production.
6. **Online monitors from one shared plan** (Phase 3, D-P3-4/D-P3-5): every
   in-subset prop/assume - including bounded liveness - is monitored online via
   the `MonitorPlan` compiled once in `src/monitor.rs`; only out-of-subset
   formulas (R1-R4) are skipped, with a warning that tells the user exactly what
   was skipped (identical text in every backend).

## 6. Known limitations

- Only the first module of a multi-module file is generated by the CLI;
  aggregate tooling handles multi-file crates instead.
- Only formulas outside the monitor-compilability restrictions R1-R4 are
  skipped (with a warning); everything in-subset - including bounded liveness -
  is monitored online (see `DESIGN.md` section 21).
- Identifier mangling escapes Rust keywords but does not perform full name
  resolution against user code that includes the generated file.
