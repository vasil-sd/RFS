# RFS Knowledge Base (KB)

Status: **2026-10-13**
Project root: `/workspace/host/RFS/`
Audience: future engineers and LLM assistants continuing RFS development.

This file is the compact handoff knowledge base. It summarizes:

1. current project state;
2. design decisions;
3. updated roadmap;
4. important tooling / agent-workflow findings;
5. Rust source, build, test, and dependency facts;
6. known limitations and corner cases;
7. detailed next R&D plan.

Read order for a new assistant:

1. `KB.md`
2. `DESIGN.md`
3. `docs/roadmap/INTRO.md` (+ the specific `PHASE_N*.md` file you need)
4. `src/ir.rs`
5. `src/parser.rs`
6. `examples/valid/`
7. `tests/`

---

## 1. Current project state

| Area | Status | Notes |
|---|---:|---|
| Session entry point | Active | `START.md` - read this first in any new LLM-assisted session (orientation + lazy-reading protocol). |
| Build/run/test how-to | Active | `BUILD.md` - build files, commands, test suites and expected results, verification checklist. |
| Toolchain | Pinned facts | `TOOLCHAIN.md` - Rust toolchain locations/versions, activation, reinstall. |
| Design document | Done, maintained | `DESIGN.md` contains the RFS v0 design baseline and parser decision. |
| Roadmap | Active | `docs/roadmap/` (split from the old `ROADMAP.md` on 2026-10-16): `INTRO.md` + `PHASE_0..6.md`; status reports for completed phases 0-2. |
| Knowledge base | Maintained | This file: design decisions, syntax reference, source map, agent workflow experience. |
| Cargo project | Working | Package `rfs`, version `0.1.0`, edition `2021`. |
| Parser | Phase 0 complete | Implemented with `nom` 8 in `src/parser.rs`. |
| IR | Phase 0 complete | Serializable Rust structs/enums in `src/ir.rs`. |
| JSON round-trip | Working | `serde` + `serde_json`; tested. |
| CLI | Working | Subcommand-based: `rfs parse` (format conversion), `rfs validate` (semantic validation), `rfs codegen` (backend registry dispatch). No positional-argument legacy mode. |
| Phase 2 core DSL | Complete | Version headers, `on_error` fallbacks, priorities in generated code, Phase 2 validator checks (errors + warnings channel), C backend (production/test/mc) with the Rust MC wire format. See `DESIGN.md` section 20. |
| Frontend CLI (`parse`) | Phase 1 working | `rfs parse --input <rfs\|json> --output <rfs\|json> <file-or-dir> [--target-dir DIR]`; all four RFS/JSON format combinations, structure-preserving target dirs, collision guard. |
| Frontend CLI (`validate`) | Phase 1 working | `rfs validate [--input <rfs\|json>] <file-or-dir>`; file mode reports to stderr, directory mode writes `<name>.rfs-validation.err` next to each failing source. |
| Valid examples | 40 files | All parse, validate, round-trip, and translate to Rust **and C**; includes TLA-inspired adaptations, Phase 1 assumption/unexpected-input demos, and the Phase 2 endpoint-protocol + on_error-recovery showcases. |
| Invalid examples | 11 files | All fail as expected (parse or validation), including TLA-syntax anti-patterns and Phase 2 semantic errors (duplicate priority, shadowed duplicate transition, on_error without next, bad version header, duplicate set). |
| Validator | Phase 1 working | `src/validator.rs` performs name resolution, pattern/output checks, expression typing, and formula typing. |
| Code generator | Phase 2 working (plugin system) | `src/codegen.rs` defines the Backend plugin API, the `BACKENDS` registry, and `rfs codegen` CLI parsing; shared analysis in `codegen_fsm/test/mc.rs`; **Rust and C backends** under `src/backends/` each implement production/test/mc (C MC reuses the Rust MC wire format). See `CODEGEN.md`. |
| Tests | Passing | 125 non-doc tests in the main crate (52 lib, 41 frontend CLI logic, 19 codegen, 3 examples, 4 training, 4 parser-library comparison, 2 Phase 2 integration); generated aggregate unit-test crate currently has 492 passing tests. |
| LLM training set | Maintained, 11 categories | `training/<category>/rfs_training.jsonl` stores category-specific SFT examples (design, syntax_semantics, properties_temporal_logic, model_checking, model_checking_space, model_checking_properties, codegen_rust, examples_explain, frontend_cli, project_tooling, tla_adaptation); `all_rfs_training.jsonl` is the generated aggregate (219 examples). |
| Training snippets | Generated | `training/<category>/snippets/valid/*.rfs` and `snippets/invalid/*.rfs` are extracted from training JSONL, validated, and tested by the parser suite. |
| Lint/format | Clean | `cargo clippy --all-targets` and `cargo fmt` were clean at last verification. |

Current high-level state:

```text
RFS source (.rfs)  <----+  rfs parse (format conversion, review)
      |                 v
      v          IR JSON (serde)  <---- external tooling via JSON
nom-based parser (src/parser.rs)
      |
      v
IR v0 (src/ir.rs, serde JSON)
      |
      +--> frontend CLI: rfs parse / rfs validate  [DONE, Phase 1]
      |        RFS<->JSON conversion, .rfs-validation.err reports
      |
      +--> semantic validator (src/validator.rs)  [DONE, Phase 1]
      |
      +--> codegen plugin registry: rfs codegen  [DONE, Phase 1]
      |        core src/codegen.rs (Backend API, BACKENDS) + shared analysis
      |        rust backend src/backends/rust*: production / test / mc
      |        model checker = production core + runtime executable
```

---

## 2. Design decisions

### 2.1 Purpose of RFS

RFS is a constrained Rust-like DSL for finite-state protocol specifications.

Main goal:

> One constrained specification -> one versioned IR -> multiple Rust targets.

Targets:

- production Rust automata;
- model-checking executables;
- generated tests;
- diagrams/docs later;
- IDE/LSP later.

RFS is intentionally not a general-purpose language. It is designed so that the same specification can be used for:

1. real protocol handling code;
2. model checking by generating Rust, compiling it, and executing it;
3. tests and diagnostics.

### 2.2 Core semantics

An RFS module describes a finite input/output transition system:

```text
M = (Q, Sigma, V, q0, delta, Lambda)
```

Where:

- `Q` finite control states;
- `Sigma` finite input event constructors with finite payloads;
- `V` finite variable valuations;
- `q0` initial state;
- `delta` guarded transition relation;
- `Lambda` finite output actions.

A step is:

```text
(state, input_event, current_vars)
   -->
(next_state, next_vars, outputs)
```

Important semantic rules:

- The environment chooses input events.
- If multiple transitions are enabled, the system is nondeterministic unless priorities are later used to make production behavior deterministic.
- If no transition is enabled for an input, that is an error/deadlock condition unless explicitly handled later.
- Variable updates in one transition are simultaneous: all right-hand sides read old variable values.
- Time is modeled discretely using explicit `Tick` events and timer variables, not real-time clocks.

### 2.3 Model-checking constraints

| Aspect | Constraint | Reason |
|---|---|---|
| States | Finite enum | Finite state-space search. |
| Events | Finite constructors with finite fields | Finite input alphabet. |
| Variables | `bool`, `bit<N>`, finite enums | Finite valuations. |
| Expressions | Pure, total, bounded | No hidden side effects or divergence. |
| Transitions | Guarded actions with simultaneous updates | Easy Rust translation and relation generation. |
| Time | Discrete `Tick` + timers | Avoids continuous real-time complexity. |
| Properties | PSL-like finite temporal subset | Translatable to monitors/automata. |
| Assumptions | Separate environment assumptions | Separates system obligations from environment guarantees. |

Deliberately excluded from core v0:

| Excluded | Reason |
|---|---|
| Unbounded integers | Breaks finite model checking. |
| Floating point | Nondeterminism/precision issues. |
| Strings/maps/vectors as core data | Unbounded or too complex for initial state space. |
| Dynamic allocation in transitions | Makes generated semantics harder to reason about. |
| Arbitrary Rust calls in guards/actions | Prevents automatic model checking. |
| Continuous real-time clocks | Much harder than discrete tick semantics. |

### 2.4 IR decisions

The IR is the single source of truth for generators.

Current decisions:

- IR is versioned: `Ir.ir_version`.
- IR is serializable with `serde`.
- JSON is the Phase 0 inspection/round-trip format.
- The parser returns IR directly in Phase 0; there is no separate public AST yet.
- The IR is intentionally clean: no parser spans are stored in the serializable IR.
- Future validator/codegen may need an annotated AST or span side-table for better diagnostics.

Important IR types:

| Type | Meaning |
|---|---|
| `Ir` | Top-level document with modules. |
| `Module` | Name, enums, initial state, events, outputs, vars, transitions, properties, assumptions. |
| `EnumDecl` | Finite enum type. |
| `Type` | `Bool`, `Bit { width }`, `Enum { name }`. |
| `ConstructorGroup` | Logical event/output group with named variants. |
| `ConstructorVariant` | `Unit` or `Struct` variant. |
| `FieldDecl` | Named field with finite type. |
| `VarDecl` | Variable name, type, initializer expression. |
| `Transition` | State, event pattern, guard, next state, emits, assignments, optional priority. |
| `EventPattern` / `OutputPattern` | Group + optional variant pattern. |
| `PatternVariant` | Unit or struct pattern. |
| `PatternField` | Optional binding name + pattern expression. |
| `PatternExpr` | Wildcard, bind, int, bool, enum variant path. |
| `OutputExpr` | Output group, optional variant, argument expressions. |
| `Assignment` | Variable update. |
| `Property` / `Formula` | Named PSL-like property or assumption. |
| `Expr` | Pure expression language value. |

### 2.5 Parser decisions

Final Phase 0 parser choice: **`nom` 8**.

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Hand-rolled lexer + recursive descent | Full control, no dependency | More duplicated low-level logic | Replaced after prototype. |
| `nom` 8 | Mature, stable, direct `IResult`, good recursive-descent fit | Some lifetime/API care needed | **Chosen**. |
| `chumsky` 0.13 | Ergonomic PEG style, modern API, promising diagnostics | Not needed yet; some API details require explicit annotations | Dev-dependency comparison only; future candidate. |
| LALR/PEG generator | Strong grammar discipline | Toolchain complexity, overkill | Rejected for Phase 0. |

Parser architecture:

- No separate token-pass lexer.
- Comments are stripped first.
- Parsing is recursive descent over `&str`.
- Tokens are recognized on demand with nom combinators and small helpers.
- The parser performs structural validation but not full semantic type checking.
- Errors carry a byte span and optional source text for line/column display.

Current parser validations:

- duplicate enum declarations;
- duplicate enum variants;
- duplicate initial state declaration;
- duplicate event/output variant inside the same group;
- duplicate variable declarations;
- duplicate property/assumption names within a module;
- duplicate field names in constructor structs;
- transition body must contain exactly one `next`;
- duplicate assignment to the same variable in one transition is rejected;
- duplicate `priority` action in one transition is rejected.

Current parser non-validations:

- unknown state names;
- unknown event/output groups or variants;
- pattern field count/type compatibility;
- expression type checking;
- bit-width overflow checks;
- initial state membership in `State` enum;
- output argument count/type compatibility.

### 2.6 Property language decisions

RFS uses a PSL-like temporal subset.

Supported formula forms in Phase 0:

```rfs
prop name : formula ;
assume name : formula ;
```

Formula atoms:

```rfs
input_is(EventPattern)
output_has(OutputPattern)
<expression>
```

Temporal/boolean operators:

```rfs
! f
f && g
f || g
f -> g
next (f)
always (f)
bounded_eventually[N] (f)
```

Conventions:

- `state` is an ordinary path expression by convention, usually referring to the current control state.
- `input_is(...)` and `output_has(...)` are formula atoms recognized by the parser.
- Assumptions describe environment guarantees; properties describe system obligations.

Known precedence corner case:

```rfs
a || b -> c
```

currently parses as:

```rfs
a || (b -> c)
```

because the right-hand side of `||` is parsed with implication included. For clarity, future specs should parenthesize implications:

```rfs
(a || b) -> c
```

or:

```rfs
always ((a || b) -> c)
```

### 2.7 Code generation architecture (implemented)

Codegen is a **plugin system**: `src/codegen.rs` defines a target-language-
agnostic core (the `Backend` plugin API, the `BACKENDS` registry, CLI request
parsing for `rfs codegen`, help rendering), and each target language lives in a
self-contained module under `src/backends/`. The Rust backend (`rust`, with
`production` / `test` / `mc` commands) is the only registered backend so far;
C is planned next.

Shared, language-agnostic analysis lives in `src/codegen_fsm.rs` (variant
flattening, representative values, property-subset analysis),
`src/codegen_test.rs` (unit-test planning), and `src/codegen_mc.rs`
(model-checker configuration, violation specs, run phases/outcomes).

The generated Rust shape is one self-contained file per command:
`enum State/Input/Output`, `struct Vars`, trace runtime,
`AssumptionHandler` trait with policy enum, `StepError`, and
`Fsm::new() / Fsm::step(input) -> Result<Vec<Output>, StepError>`.

Design intent (all implemented):

- generated production code is deterministic or explicitly priority-ordered;
- variable updates are simultaneous;
- model checker and production code consume the same IR (the MC is literally
  the production core plus a runtime);
- adding a backend = new module under `src/backends/` + one registry entry.

Full API documentation for backend authors: `CODEGEN.md`. Rust backend
architecture: `src/backends/RUST.md`.
- bitvectors should map to fixed-width Rust primitives where practical.

---

## 3. RFS v0 syntax reference

### 3.1 Source file and module header

A source file contains zero or more modules. The module header optionally
carries a version string (Phase 2; `DESIGN.md` section 20, D7):

```rfs
module name {
  ...
}

module name version "1.2.3" {
  ...
}
```

The version is a quoted string without escapes; it round-trips through the IR
(`Module.version`) and becomes `SPEC_VERSION` in generated code. Module items:
enums, `initial`, events, outputs, vars, transitions, `prop`, `assume`, and
the Phase 2 error fallback (next subsection).

### 3.1a Error fallback (`on_error`, Phase 2)

```rfs
module m {
  ...
  on_error {
    next Safe;          // required: state entered when no transition matches
    emit Alert(Code(9)); // optional emits, no pattern bindings, no assignments
  }
}
```

- At most one per module; `next` is mandatory (missing `next` is a parse error).
- Executed in **any** state when no transition matched: the generated `step`
  performs the fallback instead of returning an unexpected-event error.
- No pattern bindings are available; variable values carry over unchanged.
- Without `on_error`, behavior is exactly as in Phase 1
  (`StepError::UnexpectedEvent` / `RFS_UNEXPECTED_EVENT`).


Comments:

```rfs
# line comment
// line comment
```

Comments are replaced by spaces before parsing. Newlines are preserved. Multi-line comments are not supported.

### 3.2 Enum and initial state

```rfs
enum State { Idle, Sending, Established }
initial Idle
```

By convention, control states use enum name `State`. Other enums may be used for variables or payloads.

### 3.3 Events and outputs

Unit event/output:

```rfs
event Tick
output Drop
```

Grouped variants:

```rfs
event Recv(Syn)
event Recv(Ack(ack_seq: bit<4>))

output Send(SynAck)
output Reply(Ok(code: bit<8>))
output Reply(Err(msg: bit<8>))
```

Important current limitation:

```rfs
event E(Value(x: bit<8>))   # supported
event E(x: bit<8>)          # NOT supported in current IR/parser
```

Payload constructors require a named variant inside the group. Anonymous payload groups are a future IR extension.

### 3.4 Variables

```rfs
var seq: bit<4> init 0
var timer: bit<3> init 7
var mode: Mode init Mode::Normal
var enabled: bool init false
```

Supported types:

```rfs
bool
bit<N>
EnumName
```

Trailing semicolon is optional in declarations.

### 3.5 Transitions

General form:

```rfs
transition State on EventPattern where Guard {
  next NextState;
  emit OutputExpr;
  set var = expr;
  priority N;   # Phase 2: orders overlapping transitions (larger wins, default 0)
}
```

Rules:

- `where Guard` is optional.
- Exactly one `next` action is required.
- Zero or more `emit` actions are allowed.
- Zero or more `set` actions are allowed.
- Each action must end with `;`.
- Duplicate `next`, duplicate `priority`, or duplicate `set var` in one body is rejected.
- Phase 2 priority semantics (`DESIGN.md` section 20, D1-D5): within one state and
  identical event pattern, arms are checked in descending priority (default 0);
  equal priorities on the same pattern are a validation error; different patterns
  may reuse numbers. Without priorities, source order decides (and a warning is
  emitted for guarded pairs so intent can be pinned).

Example:

```rfs
transition SynSent on Recv(Ack(ack_seq)) where ack_seq == seq {
  next Established;
  emit Send(Ready);
  set timer = 0;
}
```

### 3.6 Event patterns

Supported forms:

```rfs
on Tick
on Recv(Syn)
on Recv(Ack(ack_seq))
on Recv(Ack(_))
on Recv(Ack(ack_seq: _))
on Cmd(Value(0))
on Flag(Value(true))
on Packet(Frame(len, _))
```

Pattern field forms:

| Pattern | Meaning |
|---|---|
| `_` | wildcard |
| `name` | bind field to `name` |
| `0`, `1`, `true`, `false` | literal match |
| `name: expr-pattern` | named pattern match |
| `name: Enum::Variant` | named enum-variant pattern |

Corner case:

```rfs
on E(Mode::Normal)       # currently NOT supported as anonymous field pattern
on E(x: Mode::Normal)    # supported
```

### 3.7 Output expressions

Supported forms:

```rfs
emit Drop
emit Send(SynAck)
emit Reply(Ok(1))
emit Action(Stop(7))
```

Trailing commas are allowed in argument lists.

### 3.8 Expression language

Literals and paths:

```rfs
0
42
true
false
seq
Mode::Normal
```

Operators by current precedence, from tighter to looser:

| Precedence | Operators | Notes |
|---:|---|---|
| 1 | `!`, `~`, unary `-` | Unary operators. |
| 2 | `*` | Multiplication. Division is not currently used in examples. |
| 3 | `+`, `-` | Additive. `-` stops before `->`. |
| 4 | `<<`, `>>` | Shifts. |
| 5 | `&` | Bitwise and. Does not consume `&&`. |
| 6 | `^` | Bitwise xor. |
| 7 | `|` | Bitwise or. Does not consume `||`. |
| 8 | `<`, `<=`, `>`, `>=` | Relational. |
| 9 | `==`, `!=` | Equality. |
| 10 | `&&` | Logical and. |
| 11 | `\|\|` | Logical or. |

Note: current implementation gives relational operators tighter precedence than equality. This is a parser design detail, not yet a frozen semantic standard. Use parentheses when mixing comparison/equality in nontrivial ways.

Examples:

```rfs
where timer > 0
where (flags & 3) == 3
where x >= 3 && x <= 10
where (a < 10 && b > 5) || (x == y)
set window = window << 1;
set flags = flags | 1;
```

### 3.9 Properties and assumptions

Examples:

```rfs
prop no_bad : always (state != Bad) ;
prop returns_good : always (input_is(ToGood) -> next (state == Good)) ;
prop timeout_progress : always (state == SynSent -> bounded_eventually[10] (state != SynSent)) ;
assume env_tick : always (bounded_eventually[5] (input_is(Tick))) ;
```

---

## 4. Rust source code map

### 4.1 Dependencies

`Cargo.toml`:

```toml
[dependencies]
nom = "8.0.0"
serde = { version = "1.0.229", features = ["derive"] }
serde_json = "1.0.154"

[dev-dependencies]
chumsky = "0.13"
```

Meaning:

| Dependency | Role |
|---|---|
| `nom` | Main parser combinator library. |
| `serde` | IR serialization/deserialization. |
| `serde_json` | JSON output and round-trip tests. |
| `chumsky` | Dev-only comparison smoke test; not used by main parser. |

### 4.2 Source files

| File | Responsibility |
|---|---|
| `src/lib.rs` | Public crate API, module declarations, `collect_rfs_files`. |
| `src/error.rs` | `Span`, `ParseError`, line/column and rich display. |
| `src/ir.rs` | IR v0 data model and serde serialization. |
| `src/parser.rs` | nom-based parser, comment stripping, structural validation, unit tests. |
| `src/frontend.rs` | Frontend CLI logic: RFS/JSON format conversion (`run_conversion`, `plan_conversion`), output path planning (target-dir structure, extension replacement, collision guard via normalized absolute paths), validation dispatch (`run_validate`) including `.rfs-validation.err` writing; reusable from tests and external tools. |
| `src/training.rs` | Training-set aggregation, JSONL validation, and RFS snippet extraction. |
| `src/validator.rs` | Semantic validation and expression/formula typing; Phase 2 adds priority/duplicate-transition errors and the non-fatal warning channel (`module_warnings`: unreachable states, dead events, unused vars, impossible guards). Used by code generation, CLI validate, and snippet classification. |
| `src/codegen.rs` | Codegen plugin core: `Backend`/`BackendCommandInfo` API, `BACKENDS` registry, `parse_codegen_args` CLI parsing, help rendering, shared option/output types. See `CODEGEN.md`. |
| `src/codegen_fsm.rs` | Language-agnostic production-FSM helpers: variant flattening (`build_variants`), representative values, property-subset analysis. |
| `src/codegen_test.rs` | Language-agnostic unit-test planning: `TestPlan`, unexpected-input/deadlock detection. |
| `src/codegen_mc.rs` | Language-agnostic model-checker concepts: `McConfig` wire format, `ViolationSpec`, `McPhase`, `McStatus`. |
| `src/backends/mod.rs` | Backend plugin module root. |
| `src/backends/rust.rs` | Rust backend registration (`RUST_BACKEND`) + shared Rust helpers (ident mangling, type mapping). See `src/backends/RUST.md`. |
| `src/backends/rust_fsm.rs` | Rust backend: production FSM source generation (core reused by test/mc). |
| `src/backends/c.rs` / `c_fsm.rs` / `c_test.rs` / `c_mc.rs` | C backend: registration + helpers, production FSM, plain-assert tests, bounded model checker. See `src/backends/C.md`. |
| `src/backends/rust_test.rs` | Rust backend: unit-test generation + aggregate test-crate tooling. |
| `src/backends/rust_mc.rs` | Rust backend: bounded model-checker executable generation. |
| `src/bin/build_training.rs` | Cargo bin behind `cargo build-training`; writes `all_rfs_training.jsonl`. |
| `src/bin/extract_training_snippets.rs` | Cargo bin behind `cargo extract-training-snippets`; refreshes `training/*/snippets`. |
| `src/bin/generate_code_tests.rs` | Generates the aggregate generated-code unit-test crate. |
| `src/bin/test_generated_code.rs` | Generates, compiles, and runs the aggregate generated-code unit tests. |
| `src/main.rs` | CLI entry point: `parse` subcommand (format conversion), `validate` subcommand (file/dir validation), `codegen` subcommand (backend registry dispatch). Unknown subcommands print the usage and exit 1. Argument parsing and exit codes live here; logic lives in `src/frontend.rs` / `src/codegen.rs`. |

### 4.3 Public API

Main public items:

```rust
rfs::parse_source(source: &str) -> Result<Ir, ParseError>
rfs::parse_file(path: &Path) -> Result<Ir, ParseError>
rfs::collect_rfs_files(root: impl AsRef<Path>) -> Result<Vec<PathBuf>, std::io::Error>
rfs::Ir
rfs::ParseError
```

`collect_rfs_files` accepts a `.rfs` file or a directory. Directories are traversed recursively and results are sorted. This is what makes the example test suite extensible without changing test code.

Frontend API (Phase 1), re-exported from `src/frontend.rs`:

```rust
rfs::Format                       // Rfs | Json, parsed from "rfs"/"json", mapped from extensions
rfs::run_conversion(input: Format, output: Format, path: &Path,
                    target_dir: Option<&Path>) -> (Vec<PathBuf>, Vec<String>)
rfs::plan_conversion(...)         // input/output pairs + planning errors (extension/collision)
rfs::convert_file_to_ir(path, format) -> Result<Ir, String>
rfs::render_ir(&Ir, format) -> Result<String, String>   // formatted .rfs or pretty JSON
rfs::run_validate(input: Option<Format>, path: &Path) -> Result<ValidateResult, Vec<String>>
rfs::validate_one(path, format) -> Result<(), Vec<String>>
rfs::normalize_abs(&Path) -> PathBuf      // lexical absolute normalization (collision guard)
rfs::collect_files_with_ext(root, ext) -> Vec<PathBuf>
rfs::err_file_path(source) / rfs::write_err_file(source, &[String])   // .rfs-validation.err
```

`ValidateResult { ok_files, failed: Vec<FailedFile>, write_errors, is_directory }` where `FailedFile { path, errors, err_file: Option<PathBuf> }`. In directory mode `run_validate` itself writes the `.rfs-validation.err` files; in file mode `err_file` is `None` and the CLI prints errors to stderr.

Codegen API (Phase 1 plugin system), defined in `src/codegen.rs`; full reference in `CODEGEN.md`:

```rust
rfs::codegen::Backend                  // name, description, commands, 3 fn pointers
rfs::codegen::BACKENDS: &[Backend]     // the plugin registry (compile-time statics)
rfs::codegen::list_backends()          // Vec<(name, description)>
rfs::codegen::get_backend(name)        // Option<&'static Backend>
rfs::codegen::common_help() / backend_help(&Backend)
rfs::codegen::AssumptionSelection      // All | None | Labels(Vec<String>), FromStr
rfs::codegen::BackendOptions { assumptions, steps }
rfs::codegen::CodegenOutput { code, warnings }
rfs::codegen::CodegenCommandKind       // Production | UnitTests | ModelChecker
rfs::codegen::CodegenCommandKind::run(self, &Backend, &Module, &BackendOptions)
rfs::codegen::parse_codegen_args(&[String]) -> Result<CodegenRequest, Vec<String>>
```

The pre-plugin monolithic API (`CodegenOptions`, `generate_module_code`, `generate_model_checker_code`) was removed during Phase 2 preparation; the auxiliary bins and tests call the backend functions directly (for example `rfs::backends::rust_test::generate_aggregate_test_crate(&files, &BackendOptions)`).

### 4.4 Parser internals

Important functions in `src/parser.rs`:

| Function | Purpose |
|---|---|
| `parse_source` | Public entry; strips comments, runs nom parser, checks trailing input. |
| `parse_file` | Reads file and calls `parse_source`. |
| `strip_comments` | Replaces `#` and `//` comments with spaces while preserving newlines. |
| `rfs_source` | Parses zero or more modules. |
| `module` | Parses module header, items, closing brace, builds module. |
| `module_item` | `alt` over enum/initial/event/output/var/transition/prop/assume. |
| `enum_decl`, `enum_variants` | Enum parsing and duplicate-variant checks. |
| `event_decl`, `output_decl` | Grouped constructor declarations. |
| `constructor_variant`, `field_list` | Named variant and field-list parsing. |
| `var_decl` | Variable declaration parsing. |
| `transition_decl`, `action_block`, `action` | Transition body parsing and structural action validation. |
| `build_module` | Assembles module and performs duplicate-name checks. |
| `event_pattern`, `output_pattern`, `group_pattern` | Pattern parsing. |
| `pattern_field_list`, `pattern_field`, `pattern_expr` | Pattern field/expression parsing. |
| `output_expr`, `expr_list` | Output action expression parsing. |
| `expr`, `logic_or`, `logic_and`, `equality`, `relational`, bitwise/shift/additive/multiplicative/unary/primary | Expression parser. |
| `formula`, `or_formula`, `impl_formula`, `and_formula`, `unary_formula`, `primary_formula` | Formula parser. |
| `identifier`, `ident_path`, `keyword`, `soft_keyword`, `int_lit` | Low-level token helpers. |
| `p_tag`, `opt_p_tag`, `p_space0`, `p_space1`, `opt_semi` | nom helper wrappers. |
| `semantic_err`, `syntax_err` | Nom error constructors for semantic/syntax failures. |

### 4.5 CLI behavior

Three subcommands, dispatched in `src/main.rs` (logic in `src/frontend.rs` / `src/codegen.rs`); any other first argument prints the usage and exits 1:

#### `parse` subcommand (Phase 1): format conversion

```sh
cargo run -- parse --input <rfs|json> --output <rfs|json> <file-or-dir> [--target-dir DIR]
```

All four format combinations are supported: rfs->json, rfs->rfs (reformat), json->json (reformat), json->rfs. Behavior rules:

- **File input**: the extension must match `--input` (`.rfs` or `.json`); anything else is an error.
- **Directory input**: scanned recursively for files with the `--input` extension only; other extensions are ignored; an empty result is an error.
- **Output naming**: the original extension is replaced, never stacked (`a.rfs` -> `a.json`, not `a.rfs.json`).
- **Placement**: without `--target-dir`, each output is written next to its input file; with `--target-dir`, files are written into the target directory and, for directory inputs, the input directory structure is preserved relative to the scanned root.
- **Collision guard**: normalized absolute paths of input and output are compared; if they are identical (e.g. rfs->rfs without `--target-dir`), the conversion is refused with an error instead of overwriting the input.
- **Exit codes**: 0 on success; 1 for any parse/planning/IO error, reported on stderr. Per-file parse errors in a directory run are collected and do not stop the remaining files.

#### `validate` subcommand (Phase 1): semantic validation

```sh
cargo run -- validate <file.rfs | file.json>
cargo run -- validate --input <rfs|json> <directory>
```

- **File input**: the extension selects the format; `--input` is optional and must not conflict with the extension. Validation errors are printed to stderr; exit code 1 on failure, 0 on success.
- **Directory input**: `--input` is required (the directory may contain either kind of file). All matching files are parsed and semantically validated. Each failing source file gets a `<name>.rfs-validation.err` text file written next to it (e.g. `bad.rfs.rfs-validation.err`) listing its parse/semantic errors; passing files get no err file. Exit code 1 if any file failed.
- Validation runs the full `src/validator.rs` pipeline: name resolution, pattern/output checks, expression and formula typing.

#### `codegen` subcommand (Phase 1): backend registry dispatch

```sh
cargo run -- codegen --list
cargo run -- codegen --help
cargo run -- codegen rust --help
cargo run -- codegen rust production spec.rfs --out fsm.rs
cargo run -- codegen rust test spec.json --steps 6 --out fsm_test.rs
cargo run -- codegen rust mc spec.rfs --assumes a1,a2 --out fsm_mc.rs
```

- `--list` prints registered backends; `--help` prints common usage; `<backend> --help` lists that backend's commands and options (all rendered from the registry, no hardcoded text).
- A generate request resolves the backend by name, selects the input format from the file extension (`.rfs`/`.json`; anything else is an error), converts the file to IR, and runs the command on `ir.modules[0]`.
- Common options: `--assumes all|none|label1,label2` (default `all`), `--steps N` for the test command (default 4), `--out FILE` (default stdout; parent directories are created).
- Warnings go to stderr; generated source goes to stdout or `--out`. Exit code 0 on success, 1 for unknown backend, bad extension, parse, or generation errors.
- The plugin API and the procedure for adding a new target language are documented in `CODEGEN.md`; the Rust backend's internals in `src/backends/RUST.md`.

### 4.6 Training-set organization

The SFT training data is stored by category under `training/`:

```text
training/
  design/rfs_training.jsonl
  syntax_semantics/rfs_training.jsonl
  properties_temporal_logic/rfs_training.jsonl
  model_checking/rfs_training.jsonl
  model_checking_space/rfs_training.jsonl
  model_checking_properties/rfs_training.jsonl
  codegen_rust/rfs_training.jsonl
  examples_explain/rfs_training.jsonl
  frontend_cli/rfs_training.jsonl
  project_tooling/rfs_training.jsonl
  tla_adaptation/rfs_training.jsonl
```

Category coverage: `codegen_rust` covers generated-code invariants including the backend plugin architecture, the `rfs codegen` CLI, and adding new target languages; `frontend_cli` covers the Phase 1 frontend subcommands (`rfs parse`, `rfs validate`), format round-trips, error reporting, exit codes, and the frontend/code split; `project_tooling` keeps current implementation status and training-pipeline maintenance examples so future sessions start from an accurate picture.

Each category directory also contains extracted RFS snippets:

```text
training/<category>/snippets/valid/*.rfs
training/<category>/snippets/invalid/*.rfs
```

Snippets are extracted from fenced ` ```rfs ` blocks in both user and assistant messages. The extractor parses each unique snippet with the current RFS parser:

- snippets that parse successfully are written to `snippets/valid/`;
- snippets that fail to parse are written to `snippets/invalid/`.

This means some fragments are intentionally in `invalid/` because they are partial syntax examples, not complete modules. The parser test suite treats them like `examples/valid` and `examples/invalid`.

The aggregate training file is generated at the project root:

```text
all_rfs_training.jsonl
```

Regenerate it with:

```sh
cargo extract-training-snippets
cargo build-training
```

To add new training examples:

1. Choose or create a category directory under `training/`.
2. Append JSONL lines to that category's `rfs_training.jsonl`.
3. Keep the required message format and `< think> ... < /think>` assistant format.
4. Run `cargo extract-training-snippets` to refresh extracted snippets.
5. Run `cargo build-training` to refresh `all_rfs_training.jsonl`.
6. Run `cargo test-training` or full `cargo test`.

### 4.7 TLA+ adaptation sources and strategy

RFS examples and training data include adaptations from two public TLA+ resources:

- `https://github.com/tlaplus/tla-by-example/tree/main/src/content/intro`
  - used for TLA+ module structure, variables/constants, `Init`/`Next`, primed variables, `UNCHANGED`, boolean operators, `TypeOK`, and stuttering intuition.
- `https://github.com/tlaplus/Examples/tree/master/specifications`
  - used as a source of protocol-style specifications, especially `tcp`, `GermanProtocol`, `SingleLaneBridge`, `echo`, and bounded-buffer/producer-consumer patterns.

Current TLA-inspired RFS examples are in:

```text
examples/valid/tla_adapted/
```

Included adaptations:

| RFS example | TLA+ inspiration | Adaptation strategy |
|---|---|---|
| `31_traffic_light_tla.rfs` | TLA+ by Example traffic light | Autonomous cycle becomes `Tick`-driven transitions. |
| `32_tcp_endpoint_tla.rfs` | `specifications/tcp/tcp.tla` | Two-peer TCP FSM becomes one endpoint I/O automaton; network queues become `Net(...)` inputs and `Send(...)` outputs. |
| `33_german_transfer_tla.rfs` | `specifications/GermanProtocol` | Reliable transfer idea becomes sender endpoint with alternating-bit ACK, timeout retransmission, and give-up. |
| `34_single_lane_bridge_tla.rfs` | `specifications/SingleLaneBridge` | Shared bridge controller grants one direction at a time. |
| `35_bounded_buffer_tla.rfs` | bounded-buffer/producer-consumer patterns | Capacity-2 buffer controller with `Put`/`Get` inputs and consistency property. |
| `36_echo_node_tla.rfs` | `specifications/echo/Echo.tla` | One non-initiator Echo node with two neighbors; sets/inboxes abstracted to booleans/counters/events. |

General adaptation rules:

- model one endpoint/controller rather than the full distributed system when possible;
- replace other processes with input events and assumptions;
- replace unbounded sequences/sets/functions with finite enums, booleans, small bitvectors, or explicit abstraction;
- make autonomous TLA+ steps input-driven in RFS, usually with `Tick`, `Command`, or `Net` events;
- translate `TypeOK` into static types plus RFS properties for consistency/range constraints;
- document what was abstracted away, especially network queues, fairness, and global distributed invariants.

---

## 5. Build, test, and run instructions

Moved out of this file to keep it focused on design knowledge:

- **`BUILD.md`** — where the build files are, how to build/run/test, common commands
  (including all cargo aliases), expected current test results, and the post-change
  verification checklist.
- **`TOOLCHAIN.md`** — Rust toolchain locations/versions, activation env vars,
  reinstall procedure, environment corner cases.

Entry point for a new session: **`START.md`**.

## 6. Tool-calling and agent workflow experience

This section records practical lessons for future LLM-assisted work in this repository.

### 6.1 Environment corner cases

- The current working directory may not be `/workspace/host/RFS`. Always `cd /workspace/host/RFS` or use absolute paths.
- Shell environment does not persist between tool calls; re-export the toolchain env vars in every command that needs cargo (see `TOOLCHAIN.md` section 2, which also lists lock-file-error recovery and other observed corner cases).
- The terminal tool is suitable for build/test loops. Use `tail -n 120` or `grep` on long cargo output to keep results readable.

### 6.2 Editing workflow

- Use small targeted edits for bug fixes.
- Use larger scripted rewrites only when replacing a module or many patterns.
- After large rewrites, immediately run:
  - `cargo build`
  - `cargo test --lib`
  - `cargo test --test examples_test`
  - `cargo test --test library_comparison`
- If an integration example test fails, it stops at the first failing file. For broad feedback, use:
  ```sh
  cargo run -- validate --input rfs examples/valid
  ```
  This reports every failing file in the directory instead of stopping at the first.
- When debugging a parser failure, isolate the smallest failing snippet into `/tmp/t.rfs` and run:
  ```sh
  cargo run -- validate /tmp/t.rfs
  ```
- Temporary `eprintln!` debug statements can be useful inside parser functions, but remove them before finalizing.

### 6.3 Rust/nom findings

Important nom 8 lessons from this project:

1. `IResult<I, O>` is `Result<(I, O), Err<E>>`. Therefore:
   ```rust
   let (input, value) = parser(input)?;
   ```
   Do not write:
   ```rust
   let value = parser(input)?;
   ```
   unless you actually want the tuple.

2. Some combinators are direct functions, such as `tag`, `multispace0`, and `satisfy`. Others, such as `many0`, `opt`, and `alt`, return parser objects that require:
   ```rust
   use nom::Parser;
   many0(parser).parse_complete(input)?
   opt(parser).parse_complete(input)?
   alt((a, b)).parse_complete(input)?
   ```

3. Use complete parsers for finite source files. `.parse_complete` is the safer choice in this project.

4. Helper functions with multiple reference parameters often need explicit lifetimes:
   ```rust
   fn p_tag<'a>(t: &'a str, i: &'a str) -> IResult<&'a str, ()> { ... }
   ```

5. Avoid type aliases like `type NomErr = nom::error::Error<&str>;` unless lifetimes are handled carefully. Prefer inline `nom::error::Error<&str>` or explicit lifetime parameters.

6. For backtracking alternatives, keyword mismatches should usually return recoverable syntax errors, not hard failures. Reserve hard failures for semantic conditions after a construct has been committed.

7. `multispace0` skips spaces, tabs, carriage returns, and newlines. Use it instead of `space0` for RFS source.

### 6.4 Chumsky findings

Chumsky 0.13 was evaluated successfully for a smoke test, but nom was chosen for the main parser.

Useful lessons:

- `Parser::parse` returns `ParseResult`, not `std::result::Result`.
- For binary success/failure in tests, use:
  ```rust
  let result = parser.parse(input).into_result();
  ```
- `text::unicode::keyword` and `text::unicode::ident` are convenient for text parsing.
- Some tuple/map closures may need explicit output-type annotations, for example:
  ```rust
  .map(|(_, name): (&str, &str)| name.to_string())
  ```
- Chumsky is a good future candidate if RFS needs richer error recovery or better diagnostic labels.

### 6.5 Parser grammar corner cases discovered

These were real bugs or design traps during the nom rewrite:

- `on Group(Variant)` needs whitespace handling before the group identifier.
- Temporal formula keywords such as `always`, `next`, and `bounded_eventually` may be followed by spaces before parentheses. The parser must skip spaces before expecting `(` or `[`.
- Bitwise `|` must not consume the first character of `||`.
- Bitwise `&` must not consume the first character of `&&`.
- Additive `-` must not consume the first character of formula implication `->`.
- Shift operators `<<` and `>>` must be checked before relational `<` and `>` in contexts where they can appear.
- Trailing commas are supported in enum variants, field lists, pattern field lists, and expression argument lists.
- Transition action bodies currently require semicolons after actions.
- A transition body with zero actions parses syntactically but fails semantically because `next` is missing.
- Duplicate `next` is a semantic failure, not a syntax failure.

### 6.6 Documentation workflow

When making a design-relevant change:

1. Update `DESIGN.md` if the language or architecture changed.
2. Update the relevant `docs/roadmap/PHASE_N*.md` file (plan corrections / status report) if phase status or stack changed; keep the phase-state table in `docs/roadmap/INTRO.md` current.
3. Update `KB.md` if future agents need to know the new state, command, corner case, or limitation.
4. Add or update examples/tests for the behavior.
5. Run `cargo test`, `cargo clippy --all-targets`, and `cargo fmt`.

---

## 7. Known limitations and risks (current)

Resolved during Phases 1-2: name resolution, pattern/output arity, state-name
checks, expression typing, bit-vector masking semantics (wrapping via wide
intermediates in Rust / native unsigned wrap in C), formula precedence
(parenthesized implementation, documented in section 3.9), priority semantics
(D1-D5 in `DESIGN.md` section 20), and the pre-plugin codegen API removal.

| Area | Current limitation | Impact | Recommended next action |
|---|---|---|---|
| Spans in IR | IR is clean and span-free. | Validator/codegen errors lack source locations (message + transition id only). | Add a span side-table before IDE diagnostics (Phase 5). |
| Anonymous payloads | `event E(x: bit<8>)` is not supported; the variant name must be written (`event E(E(x: bit<8>))`). | Some natural syntax is rejected. | Decide whether to add anonymous constructor variants later. |
| Ambiguity analysis | Guard overlap is detected structurally only (identical guards, contradictory equalities); general overlapping guarded pairs yield a warning, not a proof. | Truly conflicting guarded transitions can pass with a warning. | Bounded/decision-procedure checking in Phase 4. |
| C backend widths | Only bit<8/16/32/64> (D8). | Wider bit vectors cannot target C. | Document as a hard limit; Rust keeps u128 intermediates. |
| Feature flags | No `no_std`/monitor/logging codegen options yet (D9, deferred). | Fine for current targets. | Phase 5 candidates. |
| Error recovery | Parser stops at first error per file. | Less useful IDE experience. | Acceptable; improve later. |

## 8. Updated roadmap

`docs/roadmap/` (INTRO.md + PHASE_N.md files) is the authoritative long-term
roadmap; per-phase implementation reports live in `docs/roadmap/PHASE_N_STATUS.md`.
This section is a compact status view for future work.

### Phase 0: Parser/IR baseline — complete with caveats

Completed:

- DESIGN and roadmap documents.
- Cargo project.
- IR v0 with serde JSON.
- nom-based parser for RFS v0 subset.
- CLI parser.
- 30 valid examples.
- 4 invalid examples.
- file-driven example test suite.
- parser-library comparison: nom chosen, chumsky retained as dev-only smoke test.
- passing `cargo test`, `cargo clippy`, and `cargo fmt`.

Caveats (as of Phase 0 completion; most were resolved in Phases 1-2 - see section 7
for what remains):

- No semantic type checking or name resolution.
- No spans in IR.
- No code generation.
- No model checking.
- Priority is parsed but not semantically used.

### Phase 1: Minimal end-to-end demonstrator — complete

Validator, plugin-based Rust codegen (production/test/mc), bounded model checker,
and the `rfs parse`/`rfs validate` CLI rework are implemented. The historical
M1-M6 milestone plan is superseded by the actual outcome recorded in
`docs/roadmap/PHASE_1_STATUS.md`.

### Phase 2: Core DSL and production code generation — complete

Module version headers, `on_error` fallbacks, semantic priorities, validator
warnings, and the C backend are implemented; details in
`docs/roadmap/PHASE_2_STATUS.md`.

### Phase 3 and later — next work

- **Phase 3 (next):** PSL-like property subset + monitor codegen + MC integration - `docs/roadmap/PHASE_3.md`; start via the two-stage protocol in `docs/roadmap/INTRO.md` (plan first, user review, then implement).
- Phase 4: scalable model checking (exact reachable-state engine, liveness, optional SMT) - `docs/roadmap/PHASE_4.md`.
- Phase 5: full toolchain and production integration (CLI hardening, `build.rs`, adapters, diagrams, IDE) - `docs/roadmap/PHASE_5.md`.
- Phase 6: production hardening and v1.0 (formal semantics, fuzzing, benchmarks, docs) - `docs/roadmap/PHASE_6.md`.

---

## 9. Historical R&D plan (Phase 0 -> Phase 1)

> **Superseded:** every item in this section was implemented during Phase 1; the
> actual architecture differs from the sketches below (see §13 and `CODEGEN.md`
> for the real plugin-based codegen API). Kept as planning history only — do not
> treat the code sketches in this section as current interfaces.

### 9.1 Immediate next sprint: semantic validator

Recommended new file:

```text
src/validator.rs
```

Public API sketch:

```rust
pub struct ValidatorError {
    pub message: String,
    pub module: Option<String>,
    pub item: Option<String>,
}

pub fn validate(ir: &Ir) -> Result<(), Vec<ValidatorError>>;
```

Initial validator checks:

1. Each module has at most one `State` enum by convention, or decide whether multiple state enums are allowed.
2. `initial` names a variant of the module's `State` enum.
3. Every transition `state` and `next_state` names a `State` variant.
4. Every event pattern group exists in `module.events`.
5. Every event pattern variant exists in that event group.
6. Pattern struct field count matches the selected constructor variant field count.
7. Pattern bindings are unique within a transition.
8. Guard expressions only reference declared vars and pattern bindings.
9. `set` targets declared vars.
10. Output expression groups/variants exist in `module.outputs`.
11. Output argument count matches the selected output constructor variant.
12. Variable initializer type matches variable type.
13. Expression types are checked:
    - `bit<N>` arithmetic stays `bit<N>` or defined promotion rules;
    - comparisons produce `bool`;
    - logical operators require `bool`;
    - enum equality only for same enum type.
14. Property/assumption formulas reference only valid states, events, outputs, vars, and bindings where allowed.

Important design decision before implementation:

- Either add spans to an AST so validator errors can point to source locations, or start with name-based errors and improve diagnostics later.
- Recommended: introduce a lightweight `Spanned<T>` AST in Phase 1 if good error messages are important. Keep serializable IR clean.

Acceptance criteria:

- All valid examples still parse and validate.
- New invalid examples fail validation with useful messages.
- `cargo test` passes.
- No parser behavior regression.

### 9.2 Production code generator MVP

Recommended new file:

```text
src/codegen.rs
```

Public API sketch:

```rust
pub struct CodegenOptions {
    pub crate_name: String,
    pub module_name: String,
}

pub fn generate_module(module: &Module, options: &CodegenOptions) -> Result<String, CodegenError>;
```

Generated shape:

```rust
pub enum State { ... }
pub enum Input { ... }
pub enum Output { ... }
pub struct Vars { ... }
pub struct Fsm { state: State, vars: Vars }
pub struct UnexpectedEvent;

impl Fsm {
    pub fn new() -> Self;
    pub fn step(&mut self, input: Input) -> Result<Vec<Output>, UnexpectedEvent>;
}
```

Implementation order:

1. Generate code for specs with only unit events and no vars.
2. Add variable structs and `set` actions.
3. Add payload events and pattern matching.
4. Add guards.
5. Add outputs with payloads.
6. Add enum variables.
7. Add bit-vector arithmetic with masking.

Bit-vector representation options:

| Option | Pros | Cons | Recommendation |
|---|---|---|---|
| Native integer types by width (`u8`, `u16`, `u32`, `u64`, `u128`) | Fast, simple, no dependency | Need width dispatch; widths > 128 require custom handling | Best for Phase 1 if v0 limits widths to <= 128. |
| Fixed `u128` for all bit-vectors | Simple uniform codegen | Wastes space; less explicit | Acceptable for very small MVP. |
| External bit-vector crate | Uniform arbitrary width | Extra dependency, codegen complexity | Later if needed. |
| Multiple limbs / custom struct | Arbitrary width | Complex | Not for Phase 1. |

Recommended Phase 1 rule:

- Allow `bit<N>` where `1 <= N <= 128` in generated code.
- Map to the smallest native integer type that fits.
- Use wrapping arithmetic and mask results:
  ```rust
  const MASK: u32 = (1u32 << WIDTH) - 1;
  let next = (a.wrapping_add(b)) & MASK;
  ```
- For `N == 128`, avoid shifting by 128 in a 128-bit mask; use `u128::MAX`.

Acceptance criteria:

- Generated Rust compiles with `rustc` or a temporary Cargo crate.
- Generated `Fsm::step` matches hand-written behavior for small examples.
- Unit tests cover generated code snippets for key constructs.

### 9.3 Model-checker generator MVP

Recommended new file:

```text
src/modelcheck.rs
```

Public API sketch:

```rust
pub struct McOptions {
    pub depth: usize,
    pub max_states: usize,
}

pub fn generate_model_checker(module: &Module, options: &McOptions) -> Result<String, McError>;
```

Phase 1 strategy:

- Generate a Rust program or test that:
  1. constructs the FSM;
  2. enumerates input sequences up to depth `K`;
  3. records traces as `(input, outputs, state, vars)`;
  4. evaluates properties over bounded traces;
  5. prints a counterexample if a property is violated.

Initial property support:

- `always (boolean_expr)`;
- `always (a -> b)`;
- `always (input_is(E) -> next p)`;
- `bounded_eventually[N] p` over bounded traces.

Later property strategy options:

| Option | Pros | Cons | Recommendation |
|---|---|---|---|
| Post-hoc trace evaluation | Easiest for MVP | Less scalable, duplicate logic | Use first. |
| Generated monitor automata | Better separation, reusable | More compiler work | Phase 2. |
| SMT/BDD/symbolic | Larger state spaces | Heavy dependencies/complexity | Later research. |

Acceptance criteria:

- A correct small spec passes up to chosen depth.
- An intentionally broken spec produces a counterexample trace.
- State-space limit is enforced.
- Model checker runs as a normal Rust executable or test.

### 9.4 Testing strategy

Add layered tests:

| Layer | Purpose |
|---|---|
| Parser unit tests | Syntax corner cases. |
| Validator unit tests | Semantic errors. |
| IR round-trip tests | Serde stability. |
| Example suite | End-to-end parse/validate for valid and invalid specs. |
| Codegen snapshot tests | Stable generated code shape. |
| Codegen compile tests | Generated Rust actually compiles. |
| Model-checker behavior tests | Good specs pass, bad specs fail with traces. |
| Fuzzing later | Random specs and random inputs for parser/codegen robustness. |

Recommended invalid example categories to add:

- unknown state;
- unknown event group;
- unknown event variant;
- wrong pattern field count;
- duplicate pattern binding;
- unknown variable in guard;
- assignment to unknown variable;
- output with wrong argument count;
- type mismatch in expression;
- invalid initial state;
- property referencing unknown event/output.

### 9.5 Research questions to resolve early

1. **Spans vs clean IR**
   - Should validator errors use source spans?
   - If yes, introduce AST or span side-table before codegen.

2. **Bit-vector semantics**
   - Are all `bit<N>` operations wrapping and masked?
   - Is comparison signed or unsigned? Recommended: unsigned.
   - Are shifts logical? Recommended: logical for model checking simplicity.

3. **Formula precedence**
   - Freeze PSL-like precedence or require parentheses for mixed implication/disjunction.

4. **Priority semantics**
   - If multiple transitions are enabled, does highest priority win?
   - Is equal priority a validation error for production targets?

5. **Deadlock semantics**
   - If no transition matches an input, is that an error state, panic, or ignored input?
   - Recommended for model checking: explicit unexpected-input error; for production: configurable handler later.

6. **Variable update timing**
   - Confirm all `set` right-hand sides use old variable values and pattern bindings, not partially updated values.

---

## 10. Quick-start for a new agent session

Moved to **`START.md`** (session entry point: orientation, lazy-reading protocol,
toolchain activation, essential commands, workflow rules). Build/test details:
`BUILD.md`; toolchain details: `TOOLCHAIN.md`.

## 11. Rules for future LLM-assisted work

- Preserve the core principle: one constrained RFS spec -> one IR -> multiple Rust targets.
- Keep the DSL finite and model-checkable. Avoid adding unbounded data, floating point, arbitrary functions, or dynamic allocation to the core language.
- Prefer small, testable increments over large rewrites.
- Do not break the file-driven example suite. New examples should work without changing test code.
- Keep training data category-local: add new SFT examples to `training/<category>/rfs_training.jsonl`, then run `cargo extract-training-snippets` and `cargo build-training`.
- Keep `cargo test`, `cargo clippy --all-targets`, and `cargo fmt` green.
- Keep `DESIGN.md`, the `docs/roadmap/` phase files, and `KB.md` synchronized when making design-relevant changes.
- Prefer `nom` for parser work unless a new evaluation justifies switching.
- If adding dependencies, state the reason and keep dev-only dependencies out of normal dependencies when possible.
- When uncertain about semantics, write the open question in `KB.md` and add tests that pin the chosen behavior after the decision is made.

---

## 12. What to work on next

Obsolete as of Phase 2 completion (core DSL, priorities, on_error, validator
warnings, and the C backend all exist).
Current priority: **`docs/roadmap/PHASE_3.md` - PSL-like properties and monitors**
(property subset, monitor codegen, MC integration), planned via the two-stage
protocol in `docs/roadmap/INTRO.md`, plus the session entry workflow in `START.md`.

---

## 13. Phase 1 code generation and bounded model checking

Phase 1 adds a validator, Rust translator, generated unit tests, assumption-handler API, and an initial bounded model checker. Codegen is organized as a plugin system (see §2.7, `CODEGEN.md`, `src/backends/RUST.md`): `src/codegen.rs` holds the language-agnostic core and backend registry; `src/codegen_fsm.rs` / `codegen_test.rs` / `codegen_mc.rs` hold shared analysis; `src/backends/rust*.rs` implement the Rust target. The user-facing CLI is `rfs codegen <backend> <production|test|mc> <spec> [--out ...]` (see §4.5).

### 13.1 New modules and binaries

- `src/validator.rs`: semantic validation and expression/formula typing.
- `src/codegen.rs`: plugin API, backend registry, CLI parsing, help rendering.
- `src/bin/generate_code_tests.rs`: generates the aggregate generated-code test crate.
- `src/bin/test_generated_code.rs`: generates and runs the aggregate unit tests.

### 13.2 Assumption selection and handler API

The translator option is:

`cargo run -- codegen rust test file.rfs --out out.rs --assumes all|none|label1,label2` (the `test` command generates production code plus unit tests; use `production` for the FSM source only)

Default is `all`. Only safety assumes selected by this option are translated into runtime checks. Liveness formulas containing `bounded_eventually` print a warning and are skipped. Safety formulas outside the current translated subset also print a warning and are skipped.

Handler options considered:

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Stored closure in Fsm | Easy one-time configuration | Breaks Clone, heap allocation, less testable | Rejected |
| Return error only | Simple | No panic fallback, caller must remember policy | Rejected |
| Editable stub function | Minimal API | Not type-safe, hard to test | Rejected |
| Trait handler passed to step | Fsm stays Clone, deterministic tests, explicit policy | Slightly more boilerplate | Chosen |

Generated API:

- `AssumptionPolicy { Continue, Stop, Panic }`
- `trait AssumptionHandler { fn on_violation(&mut self, label: &str) -> AssumptionPolicy; }`
- `PanicAssumptionHandler`, `StopAssumptionHandler`, `ContinueAssumptionHandler`
- `Fsm::step(input)` uses panic fallback.
- `Fsm::step_with_handler(input, handler)` uses the supplied handler.

### 13.3 Generated unit tests

`cargo generate-code-tests --test-steps N --assumes ...` generates an aggregate crate under `target/rfs_codegen_tests`. Each RFS module gets a Rust module with:

- initial-state test;
- positive adaptive sequence test up to `N` steps, default 4;
- negative unexpected-input search test; if an input variant is unused by all transitions, the test asserts that `UnexpectedEvent` is found.

The positive test uses trial clones and `StopAssumptionHandler`, so assumption violations do not panic during test discovery.

### 13.4 Bounded model checker

Generate a standalone model checker with:

`cargo run -- codegen rust mc file.rfs --out mc.rs --assumes all`

Compile the generated binary with `rustc --edition 2021 mc.rs -o mc`.

Runtime options:

- `--bound K`: maximum steps per trace, default 5.
- `--runs N`: number of random bounded runs, default 20.
- `--seed U64`: deterministic LCG seed, default 12345.
- `--assumes all|none|labels`: active safety assumes for the run, default all translated assumes.
- `--violate label:prob:percent,...`: force selected assumptions to violate. Prob defaults to 1; percent defaults to 100.
- `--unexpected-prob P`: probability of injecting an unexpected representative input at each step, default 0.

By default the sequence generator chooses only enabled inputs that satisfy active safety assumes. It evaluates translated safety properties over the final trace and prints a counterexample trace on violation.

### 13.5 Phase 1 build commands

See `BUILD.md` section 4: `cargo generate-code-tests` (aggregate test crate),
`cargo test-generated-code` (compile + run the 492 generated unit tests), and
`cargo test codegen` (lightweight main-crate codegen tests).

### 13.6 Current Phase 1 limitations

- Generated `step` chooses the first enabled transition in source order; it does not yet explore overlapping nondeterministic transitions as separate branches.
- Model checker enumerates representative input constructors, not all payload valuations.
- Translated safety subset supports `Always`, boolean operators, `Next`, state/variable expressions, `input_is`, and `output_has`. It skips `bounded_eventually` and some exotic formulas with warnings.
- Assumption checks are post-hoc over the current trace prefix; this is correct for Phase 1 but may be less efficient than compiled monitors.
- The validator is a semantic precondition for code generation, not a full type checker for all future RFS extensions.
