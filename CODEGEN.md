# RFS Code Generation Subsystem

This document describes the code generation (codegen) subsystem of RFS in enough
detail that an LLM assistant can extend it without reading the whole codebase.
Companion documents: `DESIGN.md` (semantics and design rationale), `KB.md`
(knowledge base / project state), `src/backends/RUST.md` (Rust backend
architecture).

## 1. Purpose

Codegen translates a validated RFS module (IR) into target-language source code.
The subsystem is organized so that:

- one **common, target-language-agnostic core** defines the plugin API, the CLI
  surface, and the shared analysis helpers;
- each **target language is a self-contained plugin** under `src/backends/`
  (currently: Rust and C);
- adding a new backend never requires changing the core beyond appending one
  entry to the registry table.

## 2. File map and responsibilities

| File | Responsibility |
|---|---|
| `src/codegen.rs` | Plugin API (`Backend`, `BACKENDS`, `get_backend`, ...), shared option/output types, CLI request parsing for `rfs codegen`, help rendering. **This is the only file a new backend must touch (registry table).** |
| `src/codegen_fsm.rs` | Language-agnostic helpers for *production FSM* generation: variant flattening (`build_variants`, `VariantInfo`, `FsmVariants`), representative field values; legacy Phase 1 formula-subset helpers kept for tests (monitor-compilability now comes from `src/monitor.rs`). |
| `src/codegen_test.rs` | Language-agnostic *unit-test planning*: `TestPlan`, `build_test_plan`, static unexpected-input detection (`has_definite_unexpected`, `is_deadlocked_initial`). Decides **what** to test; backends render **how**. |
| `src/codegen_mc.rs` | Language-agnostic *exact-mode* concepts (Phase 4): status taxonomy (`ExactStatus`: pass-complete / fail / unknown-state-limit) + its wire strings, the pinned summary-line format, codegen-time flag defaults (`EXACT_DEFAULT_MAX_STATES/THREADS/TIMEOUT_SEC`, `SMT_DEFAULT_BOUND`) and their CLI wire format; legacy fuzz-mode types retained for `--mode fuzz`. |
| `src/mc_plan.rs` | Phase 4 *shared MC plan* (D-P4-1): computed once in the Rust tool - input-abstraction analysis, state-encoding layout, canonical traversal spec, liveness obligation decomposition - and embedded as constants/tables by BOTH backends. |
| `src/smt_plan.rs` / `src/smt_emit.rs` | Phase 4 task 11 (optional SMT mode): backend-neutral `SmtSpec` + IR->spec conversion with shared v1-scope rejections; shared emitter rendering one spec into BOTH backends' z3 constraint sections. |
| `src/monitor.rs` | Phase 3 *shared monitor compiler*: formula -> per-label `MonitorPlan` (language-agnostic); restrictions R1-R4; reference interpreter used only by tests. Both backends render the same plan. See section 5.4. |
| `src/backends/mod.rs` | Module root for backend plugins. |
| `src/backends/rust.rs` | Rust backend plugin entry: the `RUST_BACKEND` registration value plus shared Rust-specific helpers (identifier mangling, type mapping). |
| `src/backends/rust_fsm.rs` | Rust backend: production FSM source generation. |
| `src/backends/rust_test.rs` | Rust backend: unit-test source generation + aggregate test-crate helper for tooling. |
| `src/backends/rust_mc.rs` | Rust backend: model-checker executable - production FSM core + exact BFS engine (default, from the embedded plan) + legacy fuzz runtime + SMT-mode branch; all dispatched by the generated binary's `--mode`. |
| `src/backends/c.rs` | C backend plugin entry: the `C_BACKEND` registration value plus shared C helpers (identifier mangling, fixed-width type mapping). See `src/backends/C.md`. |
| `src/backends/c_fsm.rs` | C backend: production FSM source generation (tagged-union inputs/outputs, priority-ordered arms, on_error fallback, simultaneous updates, online monitors rendered from the shared MonitorPlan - parity with Rust). |
| `src/backends/c_test.rs` | C backend: plain-assert unit-test executable generation. |
| `src/backends/c_mc.rs` | C backend: mirror of the Rust exact engine (pthreads + C11 atomics for `--threads`, same summary line/counterexample format, gcc -Wall clean) + fuzz runtime + SMT-mode branch linking libz3. |

### Placement rule (where does new code go?)

1. If the logic depends on a target language's syntax, identifiers, or types →
   the backend module (`src/backends/<lang>*.rs`).
2. If it is pure IR analysis that any backend would reuse (variant flattening,
   coverage/unexpected-input analysis, MC configuration semantics) → the
   matching `codegen_*.rs` core module.
3. If it concerns CLI parsing, help text, or registry mechanics →
   `src/codegen.rs`.

## 3. Plugin API (`src/codegen.rs`)

### 3.1 Core types

```rust
/// One registered code-generation backend (plugin). Static data only.
pub struct Backend {
    pub name: &'static str,              // CLI name, e.g. "rust"
    pub description: &'static str,       // one-line, shown by `rfs codegen --list`
    pub commands: &'static [BackendCommandInfo],
    pub generate_production:  fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>,
    pub generate_unit_tests:  fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>,
    pub generate_model_checker: fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>,
}

/// Help descriptor for one command of a backend.
pub struct BackendCommandInfo {
    pub name: &'static str,      // CLI name: "production" | "test" | "mc"
    pub summary: &'static str,   // one-line summary
    pub usage: &'static str,     // full usage line
    pub options: &'static str,   // free-form option documentation (may be multiline)
}

/// Options shared by all backend commands. Backends may ignore fields that do
/// not apply to a specific command (`steps` only matters for `test`).
pub struct BackendOptions {
    pub assumptions: AssumptionSelection,  // All | None | Labels(Vec<String>)
    pub steps: usize,                      // example-run length K (default 4)
    pub no_std: bool,                      // production only (task 9): Rust #![no_std]+alloc; C no-op
    pub with_property_monitors: bool,      // production only (D-P3-6); test/mc always include monitors
    pub with_protocol_fsm: bool,           // production+test only (task 11): emit `impl ProtocolFsm for Fsm`
}

/// Result of one generation command.
pub struct CodegenOutput {
    pub code: String,            // generated source text
    pub warnings: Vec<String>,   // non-fatal notes (printed to stderr by the CLI)
    pub extra_files: Vec<ExtraFile>, // additional files written next to --out
}

/// An additional file emitted alongside the primary artifact (Phase 5 task 5):
/// e.g. the Rust `test` command's `fuzz_target.rs`, which depends on
/// `libfuzzer_sys` and must not be inlined into the dependency-free artifact.
pub struct ExtraFile { pub name: String, pub content: String }
```

`AssumptionSelection` implements `FromStr` (`all`, `none`, or a comma-separated
label list), `Default = All`, and derives `PartialEq, Eq`. `NO_STD_PRODUCTION_ONLY_ERROR` is the pinned rejection message for `--no-std` on a non-production command; both the CLI parser and the four test/mc backend entry points return it, so one string covers every path. `PROTOCOL_FSM_MC_ERROR` is the analogous pinned rejection for `--with-protocol-fsm` on `mc` (the trait impl belongs to the production/test surface; the model checker keeps its own runtime) - enforced by the CLI parser and by both mc entry points at library level.

**`--with-protocol-fsm` (Phase 5 task 11)**: opt-in emission of
`impl ProtocolFsm for Fsm` in the Rust production/test artifacts, where
`ProtocolFsm` is the trait from the `rfs-adapters` workspace crate
(`crates/rfs-adapters`). This is the ONE documented opt-in that links a
generated artifact to an external crate: the emitted block starts with
`use rfs_adapters::ProtocolFsm;`, so a consuming crate must depend on
`rfs-adapters`. OFF by default - the plain surface (and every byte of default
output) is unchanged. Rust backend only: the C backend accepts and ignores
the flag (the ProtocolFsm trait is a Rust-ecosystem surface; the C equivalent
is the plain step API). The emitted impl forwards to the inherent `Fsm`
methods explicitly (`Fsm::new()` / `Fsm::step(self, input)`) so it does not
recurse into itself. Emission point: after the inherent `impl Fsm`, before
the `#[cfg(test)]` module (test artifacts). Byte-pinned in
`tests/codegen_test.rs` (`PF_IMPL_BLOCK`; flag-on output = default output +
the block exactly).

### 3.2 Registry

```rust
pub static BACKENDS: &[Backend] = std::slice::from_ref(&rust::RUST_BACKEND);
pub fn list_backends() -> Vec<(&'static str, &'static str)>;   // (name, description)
pub fn get_backend(name: &str) -> Option<&'static Backend>;
pub fn common_help() -> String;      // `rfs codegen --help` text
pub fn backend_help(backend: &Backend) -> String;  // `rfs codegen <b> --help` text
```

Registration is **compile-time and static**: each backend defines a
`pub static <NAME>_BACKEND: Backend` in its module, and `BACKENDS` lists the
statics. There is no dynamic loading; "plugin" means modular, self-contained
code behind a fixed function-pointer interface.

### 3.3 Command dispatch

```rust
pub enum CodegenCommandKind { Production, UnitTests, ModelChecker }

impl CodegenCommandKind {
    pub fn from_str(s: &str) -> Option<Self>;        // "production" | "test" | "mc"
    pub fn info(&self, backend: &Backend) -> &'static BackendCommandInfo;
    /// Dispatch to the backend's registered function pointer.
    pub fn run(self, backend: &Backend, module: &Module,
               options: &BackendOptions) -> Result<CodegenOutput, Vec<String>>;
}
```

Every backend registers all three command kinds; `info()` panics if a backend
omits one (a registration bug, not a runtime condition).

### 3.4 Error and warning conventions

- Functions return `Result<_, Vec<String>>`: **all** errors are collected in the
  vector (no fail-fast), mirroring the rest of the toolchain.
- `CodegenOutput.warnings` carries non-fatal notes; the CLI prints each as
  `warning: <text>` to stderr and still exits 0.
- Generated code must be **self-contained**: it must compile without depending
  on the `rfs` crate or any external dependency.

## 4. CLI surface (`rfs codegen`)

### 4.1 Request parsing

```rust
pub enum CodegenRequest {
    List,                                // rfs codegen --list
    Help,                                // rfs codegen --help | -h
    BackendHelp(String),                 // rfs codegen <backend> --help
    Generate { backend_name: String, command: CodegenCommandKind, input: String,
               out: Option<String>, assumptions: AssumptionSelection, steps: usize,
               with_property_monitors: bool,
               mc_mode: McMode, mc_max_states: usize, mc_nondet: bool,
               mc_timeout_sec: u64, mc_threads: usize, mc_smt_bound: usize,
               counterexample_format: CounterexampleFormat,
               with_protocol_fsm: bool },
}

pub fn parse_codegen_args(args: &[String]) -> Result<CodegenRequest, Vec<String>>;
```

Defaults: `--assumes all`, `--steps 4`, no `--out` (stdout), monitors off,
no `--no-std`, no `--with-protocol-fsm`; MC flags default to `--mode exact`,
`--max-states 1000000`,
no `--nondet`, `--timeout-sec 0` (off), `--threads 1`, `--smt-bound 10`,
`--counterexample-format text`. Parse errors: missing subcommand, unknown
command name, `--out`/`--assumes` without value, non-numeric `--steps`,
`--with-property-monitors` on a non-production command (test/mc always include
property monitors), `--no-std` on a non-production command (pinned
`NO_STD_PRODUCTION_ONLY_ERROR`; the test/mc backend entry points enforce the
same rule at library level), bad `--mode` value (exact|fuzz|smt-bounded),
non-positive `--threads`/`--smt-bound`, bad `--counterexample-format` value
(pinned `COUNTEREXAMPLE_FORMAT_ERROR`; accepts exactly `json`|`text`),
`--with-protocol-fsm` on `mc` (pinned `PROTOCOL_FSM_MC_ERROR`; the mc backend
entry points enforce the same rule at library level), unknown options, stray
positional arguments. The parser does **not** check the input
extension or the backend name — those are done in the CLI layer (below) so
tests can exercise parsing in isolation.

### 4.2 CLI layer (`cmd_codegen` in `src/main.rs`)

For a `Generate` request the CLI, in order:

1. resolves the backend via `get_backend` (unknown → error + exit 1);
2. selects the input format from the **file extension** (`.rfs` / `.json`;
   anything else → error + exit 1);
3. converts the file to IR (`frontend::convert_file_to_ir`); parse errors →
   stderr + exit 1;
4. takes `ir.modules[0]` (a spec with no modules is an error);
5. dispatches via `CodegenCommandKind::run`; generation errors → stderr + exit 1;
6. prints warnings to stderr, then writes the code to `--out` (creating parent
   directories) or stdout; with `--out`, every `extra_files` entry is written
   next to it (`<dir(--out)>/<name>`) and a `wrote <path>` line is printed per
   file; success → exit 0.

Strict multi-file rule (Phase 5 task 5): a command whose output carries extra
files CANNOT run in stdout mode - the CLI exits 1 with a diagnostic naming the
extra files and pointing at `--out` (no graceful degradation: the extra files
would be silently lost).

Exit codes: 0 on success, 1 on any error. All diagnostics go to stderr except
the generated source (stdout).

### 4.3 Examples

```bash
rfs codegen --list
rfs codegen --help
rfs codegen rust --help
rfs codegen rust production spec.rfs --out fsm.rs
rfs codegen rust production spec.rfs --with-property-monitors --out fsm.rs
rfs codegen rust production spec.rfs --no-std --out fsm.rs   # #![no_std] + extern crate alloc (identical API)
rfs codegen rust production spec.rfs --with-protocol-fsm --out fsm.rs  # + impl ProtocolFsm (links to rfs-adapters)
rfs codegen rust test spec.json --steps 6 --out fsm_test.rs
rfs codegen rust mc spec.rfs --assumes a1,a2 --out fsm_mc.rs
```

## 5. Shared language-agnostic modules

### 5.1 `codegen_fsm.rs` (production FSM generation)

- `build_variants(module) -> FsmVariants`: flattens grouped event/output
  declarations into per-constructor `VariantInfo { group, variant, base_name,
  unique_name, fields }`. `unique_name` is collision-free (numeric suffixes on
  duplicates) but **not yet target-language mangled** — backends apply their own
  identifier rules.
- `representative_values(...)`: default field values used to synthesize
  candidate inputs for tests/model checking.
- Legacy Phase 1 formula-subset helpers (`contains_liveness`, `safety_formula_inner`)
  kept for tests; monitor-compilability is decided by the shared compiler in
  `src/monitor.rs` (section 5.4), so both backends warn identically.

### 5.2 `codegen_test.rs` (unit-test planning)

- `build_test_plan(variants, module, steps) -> TestPlan`.
  - Module-level fields (pre-existing): `steps`, `expect_progress` (true when
    the module has transitions), `definite_unexpected` (some input variant is
    matched by no transition — the negative test *asserts* the failure),
    `has_on_error`.
  - Extended components (Phase 5 task 4; rendered by backends in tasks 5/6):
    - `transitions: Vec<TransitionTestCase>` — per-transition cases. Each case
      drives from `t.state` with a concrete representative input matching
      `t.pattern` and an installed variable configuration. Guarded transitions
      get statically computed taken / not-taken cases (candidate values:
      bool `[false,true]`, bit<N> `[0,1,2^N-1]`, enum = all variants; capped
      search). A taken case requires every higher-priority matching sibling to
      be disabled by the same assignment (shadowed transitions are noted and
      skipped); a not-taken case is planned only when statically observable.
    - `invariants: Vec<InvariantCase>` — safety properties of the shape
      `always [N] (atom)` with a liveness-free per-sample atom; re-checked at
      every step of the positive drive trace. Liveness properties are excluded
      (they belong to the model checker).
    - `monitor_violations: Vec<MonitorViolationCase>` — for each in-subset
      property `always [N] (p)` where p is a boolean atom over state/vars, an
      injected violating configuration (+ one-step drive when needed) that
      must fire the monitor. The search only accepts a transition whose post-
      step configuration actually violates the atom AND which the generated
      `fsm_step` would really take (a higher-firing sibling - priority desc,
      source order - matching the input and whose guard holds is skipped;
      task 5 correction C5-1). Other shapes are skipped with a planner note.
    - `random_trace: RandomTraceCase` — deterministic LCG input sequence
      (SAME constants as the fuzz wire format: 6364136223846793005 /
      1442695040888963407, top-53-bit double mapping; fixed seed 12345),
      precomputed at codegen time.
    - `fuzz_decode: FuzzDecodePlan` — per input variant, the field decode
      strategy for the fuzz skeletons (bool = byte non-zero, bit<N> payload =
      ceil(N/8) masked bytes, enum = representative value).
    - `notes: Vec<PlannerNote>` — documented static-analysis fallbacks.
  The planner is language-agnostic and emits DATA ONLY (cases, expected
  outcomes, labels), never code. A private static expression evaluator
  mirrors the backend width semantics (u128 wrapping + mask to the declared
  bit width; shifts saturate at width) so guard/atom decisions agree with the
  generated runtime.
- `is_deadlocked_initial(module, variants)`: inputs exist but there are no
  transitions at all (deadlock-style negative case).
- Rendering (Phase 5 tasks 5/6: Rust backend in `rust_test.rs`, C backend in
  `c_test.rs`) - stable test names, asserted by the parity suite:
  - `t<id>_<state>_on_<event>` / `..._not_taken` — per-transition tests. The
    case's configuration is installed (`fsm.state`/`fsm.vars`) and the input
    driven with `ContinueAssumptionHandler`; the outcome is asserted via the
    RECORDED trace step (`last.to_state`, `last.outputs`), which stays correct
    even when a property violation halts the commit. Not-taken asserts either
    `UnexpectedEvent` + no recorded step, or a landing observably different
    from the transition's own signature.
  - `invariant_<label>` — re-checks the property's per-sample atom (rendered
    against `&Sample` by the shared `rust_fsm::render_signal_expr`) at every
    sample of the positive drive trace; fails with the step index. Bounded
    invariants check samples `< N` only.
  - `monitor_<label>_fires_on_violation` — drives the planned violating trace
    and asserts the sticky `property_violation` record holds the target label
    (the planner guarantee above makes this sound).
  - `random_trace_seed<seed>_steps<K>` — drives the precomputed LCG index
    sequence (`ALL_INPUTS[i]`) with `StopAssumptionHandler` and prints one
    deterministic line: `random_trace seed=… steps=… ok=… unexpected=…
    assumption=… property=… states_visited=…` (distinct visited states).
  The Rust `test` command ALSO emits `fuzz_target.rs` as a separate extra
  file (`CodegenOutput.extra_files`): `#![no_main]` + `libfuzzer_sys::
  fuzz_target!`, decoding the first byte as variant selector and each field
  per `FuzzDecodePlan`, looping decoded inputs over `Fsm::step`. It is kept
  OUT of the main artifact so that one stays dependency-free (the aggregate
  generated-code crate compiles without external deps); the CLI writes it
  next to `--out` and refuses stdout mode. Building/running it is task 12.
- C rendering (Phase 5 task 6, `c_test.rs`) mirrors the Rust renderer with
  IDENTICAL coverage: each test is a plain-assert function run from a table
  in `main()`, printing one line per test (`PASS <name>` / `FAIL <name>`) so
  the parity suite can compare per-test results across backends (the Rust
  side is parsed from libtest output). Details:
  - Per-transition tests install `f.state`/`f.vars`, drive with a
    `RFS_CONTINUE` assumption handler and assert on the RECORDED trace step
    (`last->to`, `last->outputs`) - same robustness against property-violation
    halts as Rust. Not-taken compares the landing against the transition's own
    signature (state + full outputs) computed in C.
  - Invariant atoms are rendered against `const rfs_sample_t*` by the shared
    free function `c_fsm::render_signal_expr_c` (extracted from the monitor
    renderer; prefix-parameterized helper names, so monitor output stays
    byte-identical), with per-invariant `rfs_inv_<label>_in/out_<i>` pattern
    helpers emitted on demand.
  - Monitor tests assert the sticky `f.prop_label` record holds the target
    label (initial-config case: right after `fsm_new`; one-step case: after
    installing state/vars and driving with `RFS_CONTINUE`).
  - The random-trace test drives the precomputed LCG index table over the
    harness `ALL_INPUTS` with a `RFS_STOP` handler and prints the SAME
    deterministic coverage line as Rust (byte-identical, parity-checked).
  - The C `test` command ALSO emits `fuzz_target.c` as a separate extra file:
    an `LLVMFuzzerTestOneInput` decoding per `FuzzDecodePlan` (first byte =
    variant selector, bool = non-zero byte, bit<W> = ceil(W/8) little-endian
    masked bytes, enum = constant representative), looping over `fsm_step`.
    The header documents the clang + libFuzzer build recipe; generation needs
    no clang (the main artifact stays plain `gcc -Wall`; building/running the
    target is task 12).

### 5.3 `codegen_mc.rs` (model-checker concepts, Phase 4)

Exact mode (default):

- `ExactStatus`: the three-status taxonomy - `pass-complete` / `fail` /
  `unknown-state-limit` - with `wire()` (summary-line string), `human()`
  (`PASS (complete)` / `FAIL` / `UNKNOWN (state limit reached)`) and
  `exit_code()` (0/1/2); `FailReason`: `property-violation` /
  `liveness-violation` / `deadlock`.
- `exact_summary_line(...)`: the pinned machine-readable report line
  `model-check summary: status=<s> states=N transitions=M max_depth=D`
  (+ ` reason=<r> label=<L> step=S` on fail). Unit-test-pinned; parity-checked
  across backends.
- Codegen-time flag defaults embedded as runtime-overridable defaults:
  `EXACT_DEFAULT_MAX_STATES = 1_000_000`, `EXACT_DEFAULT_TIMEOUT_SEC = 0` (off;
  wall-clock only, NOT parity-guaranteed), `EXACT_DEFAULT_THREADS = 1`,
  `SMT_DEFAULT_BOUND = 10`.
- `CounterexampleFormat` (Phase 5 task 10): the exact-mode report format -
  `Text` (default; the pinned human-readable report) or `Json` (one compact
  machine-readable document). Codegen flag `--counterexample-format json|text`
  is embedded as the runtime default (`DEFAULT_COUNTEREXAMPLE_FORMAT`),
  overridable at runtime by the generated binary. Exact mode only - fuzz and
  smt-bounded modes keep their own output.
- The JSON report model (task 10): `McJsonReport` (status, metrics,
  reason/label/step on fail, statistics, counterexample) +
  `JsonCounterexample` (`trace` / `lasso` with prefix+cycle+cycle_length /
  `deadlock-config`) + `JsonStep`/`JsonIo`. **Key ordering is part of the
  byte-parity contract** (documented next to the `JSON_KEY_*` constants):
  top level `status, metrics, [reason, label, step], statistics,
  [counterexample]`; `metrics` = `states, transitions, max_depth`;
  `statistics` = `unexpected_terminations, monitor_size_bits,
  memory_estimate_bytes, peak_rss_kb`; step objects = `input, outputs,
  from_state, to_state, vars`; the counterexample object carries
  `ce_steps` (counterexample length in steps) on every FAIL document. input/output objects = `variant, fields` (declaration
  order). Strings use the `json_rw.rs` escaping contract; integers are
  unsigned decimal. Each backend embeds a minimal copy of the shared writer
  (Rust: value-tree writer with u64 integers - every document value is
  non-negative and bit<64> fields reach 2^64-1; C: direct-append port of the
  same contract) so generated binaries stay dependency-free. Parity extends
  to byte-identical JSON across backends (asserted over all property-bearing
  examples + a golden full-document pin), EXCEPT `statistics.peak_rss_kb` -
  each generated binary self-reports its own peak RSS at exit (informational,
  machine-dependent; the parity suites zero it before byte comparison, and
  `rfs mc-ref` reports 0 since it runs in-process).

Legacy fuzz mode (`--mode fuzz`) retains its types: `McConfig` (bound, runs,
seed, assumption selection, violation specs, unexpected-event probability + CLI
wire format), `ViolationSpec::parse("label:prob:percent%")`, `McPhase` (the
ordered per-step strategy), `McStatus` (per-run outcome taxonomy).

Important: the generated model-checker binary must not depend on this crate, so
each backend **embeds its own copy** of the plan tables and parsing helpers in
the generated source. The formats defined here are the source of truth; keep
the embedded copies in sync when changing them.

### 5.4 `monitor.rs` (shared monitor compiler, Phase 3)

- `compile_formula(label, is_assumption, formula) -> Result<MonitorPlan,
  String>`: compiles a property/assumption formula into a language-agnostic
  per-label monitor plan - per-sample boolean signals (temporal-operator-free
  subformulas), delay registers for `next`, obligation monitors with
  trigger/deadline countdown for `bounded_eventually[N]`, countdown monitors
  for `always[N]`, and top-level violation wiring.
- Restrictions R1-R4 decide what is compilable; out-of-subset formulas return
  Err, and EVERY backend emits the same warning (`property '<name>' is outside
  the monitor subset; skipped`) and generates no monitor for that label
  (D-P3-5).
- Both backends render the SAME `MonitorPlan` (Rust in `rust_fsm.rs`, C in
  `c_fsm.rs`); exact deadline semantics (D-P3-3) are pinned by unit tests,
  including a small reference interpreter used only by tests. See DESIGN.md
  section 21.

## 6. Adding a new backend (checklist)

1. Create `src/backends/<lang>.rs` with shared helpers and one module per
   command (`<lang>_fsm.rs`, `<lang>_test.rs`, `<lang>_mc.rs`), mirroring the
   Rust backend's layout.
2. Implement the three functions with the exact signature
   `fn(&Module, &BackendOptions) -> Result<CodegenOutput, Vec<String>>`.
   Reuse `codegen_fsm` / `codegen_test` / `codegen_mc` for analysis; only the
   rendering is new.
3. Define `pub static <LANG>_BACKEND: Backend` with name, description, and a
   `commands` slice containing one `BackendCommandInfo` per command (name must
   be exactly `production`, `test`, or `mc`).
4. Append the static to `BACKENDS` in `src/codegen.rs`. That is the only core
   change; `--list`, `--help`, and dispatch pick the backend up automatically.
5. Add tests: registry visibility, per-command generation over
   `examples/valid/*` with output markers, and (recommended) compile-and-run a
   generated artifact for at least one example, as done for Rust.
6. Document the backend in `src/backends/<LANG>.md` and link it from this file
   and `KB.md`.

## 7. Pre-plugin API (removed)

The monolithic pre-plugin API (`CodegenOptions`, `CodegenResult`,
`generate_module_code`, `generate_model_checker_code`) was removed during Phase
2 preparation. All callers now use the registry API or the backend functions
directly:

- CLI dispatch: `get_backend(name)` + `CodegenCommandKind::run(backend, module, &BackendOptions)`.
- Aggregate test-crate tooling (`src/bin/generate_code_tests.rs`,
  `src/bin/test_generated_code.rs`, `tests/codegen_test.rs`):
  `rfs::backends::rust_test::generate_aggregate_test_crate(&files, &BackendOptions)`
  and `write_generated_crate(&root, &crate_)`.

New code must go through the registry API; do not reintroduce wrapper surfaces.

## 8. Testing conventions

`tests/codegen_test.rs` covers:

- registry: `rust` is listed, all three commands registered, unknown names
  rejected; help texts mention every command;
- generation through the registry for **every** valid example and valid
  training snippet, with per-command output markers (`pub struct Fsm`,
  `#[test]`, `fn main()`);
- option sensitivity: different `--steps` produce different test code;
  `--with-property-monitors` on/off changes PRODUCTION output in BOTH backends,
  and the flag is rejected for mc/test (they always include monitors);
- monitor rendering: monitor structs present in generated code for
  property-bearing examples (e.g. example 41) in Rust AND C; no skip warnings
  for in-subset properties, identical warning wording for out-of-subset formulas;
- CLI parsing: list/help/backend-help, full generate with options, defaults,
  assumption label lists, and every documented parse-error case;
- the aggregate test-crate helper and model-checker generation.

Cross-backend differential parity (`tests/backend_parity_test.rs`, Phase 4 task
4: migrated to seedless EXACT mode): MC runs over ALL property-bearing examples
must be field-identical (exit codes, summary-line fields incl.
states/transitions/max_depth, violation label + trigger/violation steps,
counterexample traces byte-for-byte), and both backends' generated test
executables must agree on every example. Liveness parity
(`tests/liveness_mc_test.rs`, Phase 4 task 6): lasso counterexamples with pinned
cycle fields in both backends. Benchmark parity (`tests/benchmark_mc_test.rs`,
Phase 4 tasks 9/10B): specs 43/44/45 pass-complete with identical deterministic
metrics, FAIL-variant counterexample parity, `--threads 1/2/4` byte-identity,
SMT-mode byte-identity under the `smt` feature (`tests/smt_bounded_test.rs`).
Monitor integration (`tests/monitor_integration_test.rs`): an injected bug is
catch by the MC in BOTH backends with the same property-specific counterexample
info, and debug-gated production monitors report violations
(`StepError::PropertyViolation` / `RFS_PROPERTY_VIOLATION`).

End-to-end behavior of the binary (exit codes, extension enforcement, unknown
backend) is exercised by running `target/debug/rfs codegen ...` in CI-style
checks; keep any such checks aligned with section 4.2 above.

## 9. Invariants

1. Generated source is a single self-contained file per command; no external
   dependencies; compiles with stable Rust, edition 2021. The ONE documented
   exception (Phase 5 task 11): `--with-protocol-fsm` (off by default) emits
   an `impl ProtocolFsm for Fsm` block that references the `rfs-adapters`
   crate - the opt-in that links a generated artifact to an external crate.
2. Codegen is deterministic: same IR + same options ⇒ byte-identical output.
3. Production code and model checker consume the **same IR module**; they must
   not diverge in transition semantics.
4. The core (`codegen*.rs`) contains no target-language syntax; the backends
   contain no IR analysis that belongs to the core (see placement rule).
5. Backend parity (roadmap principle, `docs/roadmap/INTRO.md`): all registered
   backends provide the same commands and deliverables with identical observable
   behavior for the same spec and inputs; new backend features land in every
   backend in the same change and are covered by cross-backend differential
   tests.
6. Property/assumption monitors are compiled ONCE by the shared `src/monitor.rs`
   and rendered by every backend: monitor semantics (D-P3-3) and out-of-subset
   warning text must be identical across backends (D-P3-5).
7. Artifact provenance header (Phase 6 task 2.3, D-P6-3): EVERY generated
   artifact of every command of every backend (incl. fuzz skeletons) starts
   with a line-1 comment carrying three FIELDS - the toolchain version that
   generated it (`env!("CARGO_PKG_VERSION")`), the spec hash (SHA-256 of the
   canonical IR JSON of the whole input file, first 16 hex chars, computed in
   `src/codegen.rs` from `Ir::to_json()` via the hand-rolled `src/spec_hash.rs`),
   and the module name. The stable contract is that the fields are present and
   correct; the exact line wording may change in a MINOR bump (pin the fields,
   not the text - `docs/user/MIGRATION.md` section 2.5).
