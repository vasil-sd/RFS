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
  (currently: Rust; planned: C);
- adding a new backend never requires changing the core beyond appending one
  entry to the registry table.

## 2. File map and responsibilities

| File | Responsibility |
|---|---|
| `src/codegen.rs` | Plugin API (`Backend`, `BACKENDS`, `get_backend`, ...), shared option/output types, CLI request parsing for `rfs codegen`, help rendering. **This is the only file a new backend must touch (registry table).** |
| `src/codegen_fsm.rs` | Language-agnostic helpers for *production FSM* generation: variant flattening (`build_variants`, `VariantInfo`, `FsmVariants`), representative field values, safety/liveness formula subset analysis. |
| `src/codegen_test.rs` | Language-agnostic *unit-test planning*: `TestPlan`, `build_test_plan`, static unexpected-input detection (`has_definite_unexpected`, `is_deadlocked_initial`). Decides **what** to test; backends render **how**. |
| `src/codegen_mc.rs` | Language-agnostic *model-checker* concepts: run configuration (`McConfig`) and its CLI wire format, forced-violation specs (`ViolationSpec`, `label:prob:percent%`), per-step generation phases (`McPhase`), per-run outcome taxonomy (`McStatus`). |
| `src/backends/mod.rs` | Module root for backend plugins. |
| `src/backends/rust.rs` | Rust backend plugin entry: the `RUST_BACKEND` registration value plus shared Rust-specific helpers (identifier mangling, type mapping). |
| `src/backends/rust_fsm.rs` | Rust backend: production FSM source generation. |
| `src/backends/rust_test.rs` | Rust backend: unit-test source generation + aggregate test-crate helper for tooling. |
| `src/backends/rust_mc.rs` | Rust backend: bounded model-checker executable generation. |
| `src/backends/c.rs` | C backend plugin entry: the `C_BACKEND` registration value plus shared C helpers (identifier mangling, fixed-width type mapping). See `src/backends/C.md`. |
| `src/backends/c_fsm.rs` | C backend: production FSM source generation (tagged-union inputs/outputs, priority-ordered arms, on_error fallback, simultaneous updates, translated safety subset). |
| `src/backends/c_test.rs` | C backend: plain-assert unit-test executable generation. |
| `src/backends/c_mc.rs` | C backend: bounded model-checker executable (same CLI wire format and LCG as the Rust MC). |

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
}

/// Result of one generation command.
pub struct CodegenOutput {
    pub code: String,            // generated source text
    pub warnings: Vec<String>,   // non-fatal notes (printed to stderr by the CLI)
}
```

`AssumptionSelection` implements `FromStr` (`all`, `none`, or a comma-separated
label list), `Default = All`, and derives `PartialEq, Eq`.

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
               out: Option<String>, assumptions: AssumptionSelection, steps: usize },
}

pub fn parse_codegen_args(args: &[String]) -> Result<CodegenRequest, Vec<String>>;
```

Defaults: `--assumes all`, `--steps 4`, no `--out` (stdout). Parse errors:
missing subcommand, unknown command name, `--out`/`--assumes` without value,
non-numeric `--steps`, unknown options, stray positional arguments. The parser
does **not** check the input extension or the backend name — those are done in
the CLI layer (below) so tests can exercise parsing in isolation.

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
   directories) or stdout; success → exit 0.

Exit codes: 0 on success, 1 on any error. All diagnostics go to stderr except
the generated source (stdout).

### 4.3 Examples

```bash
rfs codegen --list
rfs codegen --help
rfs codegen rust --help
rfs codegen rust production spec.rfs --out fsm.rs
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
- Formula subset analysis: which properties are expressible in the translated
  (safety) subset; liveness constructs outside it become warnings, not errors.

### 5.2 `codegen_test.rs` (unit-test planning)

- `build_test_plan(variants, module, steps) -> TestPlan { steps,
  expect_progress, definite_unexpected }`.
  - `expect_progress`: true when the module has transitions (a positive run
    should make at least one step).
  - `definite_unexpected`: true when some input variant is matched by no
    transition — the generated negative test then *asserts* that an
    unexpected-event failure is found; otherwise it only records the attempt.
- `is_deadlocked_initial(module, variants)`: inputs exist but there are no
  transitions at all (deadlock-style negative case).

### 5.3 `codegen_mc.rs` (model-checker concepts)

- `McConfig`: bound, runs, seed, assumption selection, violation specs,
  unexpected-event probability; plus its CLI wire format used by the *generated*
  binary's command line.
- `ViolationSpec::parse("label:prob:percent%")` with defaults
  (`prob = 1.0`, `percent = 100.0`).
- `McPhase`: the ordered per-step strategy (e.g. normal random input vs forced
  assumption violation vs unexpected-event injection).
- `McStatus`: per-run outcome taxonomy (completed, property violation,
  unexpected event, assumption stop, ...).

Important: the generated model-checker binary must not depend on this crate, so
each backend **embeds its own copy** of the parsing helpers in the generated
source. The formats defined here are the source of truth; keep the embedded
copies in sync when changing them.

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
- CLI parsing: list/help/backend-help, full generate with options, defaults,
  assumption label lists, and every documented parse-error case;
- the aggregate test-crate helper and model-checker generation.

End-to-end behavior of the binary (exit codes, extension enforcement, unknown
backend) is exercised by running `target/debug/rfs codegen ...` in CI-style
checks; keep any such checks aligned with section 4.2 above.

## 9. Invariants

1. Generated source is a single self-contained file per command; no external
   dependencies; compiles with stable Rust, edition 2021.
2. Codegen is deterministic: same IR + same options ⇒ byte-identical output.
3. Production code and model checker consume the **same IR module**; they must
   not diverge in transition semantics.
4. The core (`codegen*.rs`) contains no target-language syntax; the backends
   contain no IR analysis that belongs to the core (see placement rule).
