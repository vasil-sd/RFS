# RFS Design Document

Status: Phase 0 design baseline
Language name: **RFS** — Rust Finite-State Specification Language
File extension: `.rfs`
Target ecosystem: Rust / Cargo

This document records the design choices for a constrained DSL used to specify finite-state protocol automata. The same specification is intended to generate:

1. production Rust code implementing the automaton;
2. Rust model-checking executables;
3. tests, diagrams, and later IDE/tooling artifacts.

The immediate Phase 0 goal is to freeze the core syntax/semantics boundary, define the IR, and implement a parser for the RFS v0 subset.

---

## 1. Purpose

RFS is a domain-specific language for protocol control logic. It is intentionally constrained so that specifications can be model-checked by translating them into Rust, compiling the result, and executing the generated model checker.

The main use cases are:

- endpoint protocol state machines, for example handshake, retry, timeout, close, reset;
- event-driven automata with finite payload fields;
- PSL-like temporal properties over inputs, outputs, states, and variables;
- deterministic production Rust code generation;
- assumption-based environment constraints for model checking.

RFS is not intended to be a general-purpose programming language. It is a constrained specification language whose core semantics are finite, pure, and translatable to Rust.

---

## 2. High-level architecture

```text
protocol.rfs  <----+  rfs parse (format conversion, human review)
     |             |
     v             v
Lexer / Parser   IR JSON (serde) <---- external tooling via JSON
     |
     v
AST (parser-specific)
     |
     v
IR v0 (versioned, serializable)
     |
     +--> frontend CLI: rfs parse / rfs validate  [Phase 1]
     |        RFS<->JSON conversion, semantic validation,
     |        .rfs-validation.err reports for directory runs
     |
     +--> codegen plugin registry: rfs codegen  [Phase 1]
     |        core (src/codegen.rs): Backend API, BACKENDS registry,
     |        CLI parsing, help rendering
     |        shared analysis: codegen_fsm / codegen_test / codegen_mc
     |        rust backend (src/backends/rust*):
     |          production -> enum State/Input/Output/Vars, Fsm::step(...)
     |          test       -> unit tests (positive runs, negative search)
     |          mc         -> bounded model-checker executable
     |        future: C and other target-language plugins
     |
     +--> docs / diagrams / LSP later
```

The IR is the single source of truth for all generators. This prevents production code and model-checking code from diverging.

Because both `.rfs` sources and IR JSON files normalize to the same IR, every tool in the chain (parser, validator, code generators, frontend CLI) accepts either representation: RFS text is parsed into IR, IR JSON is deserialized into IR, and formatted output can be written back as either `.rfs` or `.json`.

### Code generation as a plugin system [Phase 1]

RFS will target more than one language (Rust now, C later), so code generation is built as a plugin system:

- **Core (`src/codegen.rs`)**: target-language-agnostic. Defines the `Backend` value (name, description, per-command help descriptors, and one function pointer per command kind: production FSM, unit tests, bounded model checker), the compile-time registry `BACKENDS`, CLI request parsing for `rfs codegen`, and help rendering. Adding a backend means appending one static to `BACKENDS`; listing, help, and dispatch pick it up automatically.
- **Shared analysis (`src/codegen_fsm.rs`, `codegen_test.rs`, `codegen_mc.rs`)**: IR-level logic any backend reuses — variant flattening with collision-free names, representative input values, unit-test planning (positive runs / static unexpected-input detection), model-checker configuration and run-phase/outcome taxonomy. No target-language syntax lives here.
- **Backends (`src/backends/<lang>*.rs`)**: everything language-specific — identifier mangling, type mapping, expression lowering, and the rendering of the three commands. The Rust backend is documented in `src/backends/RUST.md`; the plugin API reference for authors (including LLM assistants) is `CODEGEN.md`.

Design decisions:

1. **Static registration, plain function pointers.** No trait objects or dynamic loading: backends are `pub static` values listed in a const slice. Simplest possible plugin model for a single-binary toolchain; dispatch is one indirect call via `CodegenCommandKind::run`.
2. **Analysis/rendering split.** The core decides *what* to generate (variant lists, test plans, MC strategy); backends decide *how* it looks in the target language. A second backend reuses all analysis for free.
3. **Self-contained artifacts.** Each command emits one file with no dependency on the `rfs` crate or external crates; codegen is deterministic (same IR + options ⇒ identical bytes).
4. **MC is production core + runtime.** The generated model checker embeds the same `Fsm` code that ships, so checked behavior equals shipped behavior.
5. **CLI mirrors the registry.** `rfs codegen --list`, `--help`, and `<backend> --help` are rendered from backend data, never hardcoded; unknown backends/commands are errors with exit code 1.

---

## 3. Core semantics

An RFS module describes a finite input/output transition system:

```text
M = (Q, Sigma, V, q0, delta, Lambda)
```

where:

- `Q` is a finite set of control states;
- `Sigma` is a finite set of input events with finite payload fields;
- `V` is a finite set of variable valuations;
- `q0` is the initial state;
- `delta` is the transition relation;
- `Lambda` is a finite set of output actions.

A step has the form:

```text
(state, input_event, current_vars)
   -->
(next_state, next_vars, outputs)
```

For model checking, the system is interpreted as an I/O automaton:

- the environment chooses input events;
- the system chooses enabled transitions;
- if multiple transitions are enabled, that represents nondeterminism unless priorities make production behavior deterministic;
- if no transition is enabled for an input, the system enters an error/deadlock condition unless explicitly handled.

For production code generation, the default target semantics should be deterministic or explicitly prioritized. Real protocol handlers should not depend on hidden ordering.

---

## 4. Model-checking constraints

To make model checking feasible and sound, the core DSL is constrained as follows.

| Aspect | Constraint | Reason |
|---|---|---|
| States | Finite Rust-like enum | Enables finite state-space exploration |
| Events | Finite constructors with finite payload fields | Enables finite input alphabet |
| Variables | Fixed-width bitvectors, enums, booleans | Enables finite valuations |
| Expressions | Pure, total, bounded expression language | No side effects, no unbounded computation |
| Transitions | Guarded, action-based, simultaneous variable updates | Easy translation to Rust `match` and model-checking relation |
| Time | Explicit `Tick` event plus timer variables | Avoids real-time complexity while supporting timeouts |
| Production target | Deterministic or explicitly prioritized | Real protocol code should behave predictably |
| Properties | PSL-like temporal formulas over finite samples | Translatable to monitors/automata |
| Assumptions | Separate PSL-like environment assumptions | Separates system behavior from environment guarantees |

### Deliberately excluded from the core DSL

| Excluded feature | Reason |
|---|---|
| Unbounded integers | Breaks finite-state model checking |
| Dynamic allocation in transitions | Makes state space and semantics hard to reason about |
| Arbitrary Rust function calls in guards/actions | Prevents automatic model-checking translation |
| Floating point | Nondeterminism, precision issues, poor model-checking fit |
| Strings/maps/vectors as core data | Unbounded or complex; use finite encodings or ignore/abstract fields |
| Real-time clocks with continuous time | Much harder than discrete tick-based model checking |

These may be added later as extensions or abstractions, but they should not be part of the initial core.

---

## 5. RFS v0 language subset

Phase 0 implements a parser for a constrained v0 subset. The subset is large enough to express useful protocol automata and small enough to keep the parser and IR stable.

### 5.1 Source file

A source file contains zero or more modules. Phase 0 examples usually contain one module.

```rfs
module name {
  ...
}
```

Comments:

- `//` line comment, Rust-style;
- `#` line comment, useful for headers such as `# rfs 1.0`.

Comments are ignored by the parser in v0. A future version may add explicit directives.

### 5.2 Module items

A module can contain:

- enum type declarations;
- initial state declaration;
- event declarations;
- output declarations;
- variable declarations;
- transitions;
- properties;
- assumptions.

Order is not semantically significant in v0, but examples should declare types before use for readability. Validation may later enforce or relax ordering rules.

### 5.3 Enum types and states

Enum syntax is Rust-like:

```rfs
enum State {
  Idle,
  WaitingAck,
  Established
}
```

By convention, the control-state enum is named `State`. Other enums may be used as finite payload or variable types.

Example:

```rfs
enum Mode {
  Normal,
  Retransmit,
  Error
}
```

### 5.4 Initial state

```rfs
initial Idle
```

The initial state must name a variant of the module's `State` enum. Validation is outside the minimal parser scope but should be added early.

### 5.5 Events

Events are input constructors. v0 supports unit events and grouped event variants.

Unit event:

```rfs
event Tick
```

Grouped event variants:

```rfs
event Recv(Syn)
event Recv(Ack(ack_seq: bit<4>))
```

This syntax allows several lines to contribute variants to the same logical event group `Recv`.

The IR should group them as:

```text
EventGroup::Recv {
  variants: [
    Unit(Syn),
    Struct(Ack, [Field { name: ack_seq, type: Bit(4) }])
  ]
}
```

Event payload fields use finite types.

### 5.6 Outputs

Outputs are action constructors and use the same shape as events.

```rfs
output Send(SynAck)
output Send(Fin(seq: bit<4>))
output Drop
```

The IR groups outputs similarly to events.

### 5.7 Variables

Variables are finite and explicitly initialized.

```rfs
var seq: bit<4> init 0
var timer: bit<3> init 7
var mode: Mode init Mode::Normal
```

Supported v0 types:

- `bool`;
- `bit<N>`;
- enum type names.

Aliases may be added later, for example `u8 == bit<8>`.

Variable updates in a transition are evaluated simultaneously from the old variable values. This avoids sequential-update ambiguity and makes model checking cleaner.

### 5.8 Transitions

General transition form:

```rfs
transition <state> on <event_pattern> where <guard> {
  next <next_state>;
  emit <output_expr>;
  set <var> = <expr>;
}
```

The `where <guard>` clause is optional.

Example:

```rfs
transition WaitingAck on Recv(Ack(ack_seq)) where ack_seq == seq {
  next Established;
  emit Send(Ready);
  set timer = 0;
}
```

#### Event patterns

Unit event pattern:

```rfs
on Tick
```

Grouped unit variant pattern:

```rfs
on Recv(Syn)
```

Grouped struct variant pattern with bindings:

```rfs
on Recv(Ack(ack_seq))
```

Wildcard and named-pattern forms are supported:

```rfs
on Recv(Ack(_))
on Recv(Ack(ack_seq: _))
```

Literal patterns may be supported for finite fields, for example:

```rfs
on Recv(Ack(ack_seq: 0))
```

#### Guards

Guards are pure boolean expressions over current variables and event bindings.

Examples:

```rfs
where ack_seq == seq
where timer > 0
where mode == Mode::Normal && len <= max_len
```

#### Actions

Allowed actions:

```rfs
next StateName;
emit OutputName;
emit OutputName(expr, expr);
set var = expr;
```

Rules:

- exactly one `next` action is required in a transition body;
- zero or more `emit` actions are allowed;
- zero or more `set` actions are allowed;
- in v0 validation should reject multiple assignments to the same variable in one transition;
- all right-hand-side expressions are evaluated before assignments are applied.

### 5.9 Expression language

The v0 expression language is small, pure, and total.

Supported:

- integer literals: `0`, `1`, `42`;
- boolean literals: `true`, `false`;
- identifiers: variables, event bindings, enum variants such as `Mode::Normal`;
- parenthesized expressions;
- boolean operators: `!`, `&&`, `||`;
- equality/inequality: `==`, `!=`;
- comparisons: `<`, `<=`, `>`, `>=`;
- arithmetic on bitvectors: `+`, `-`, `*`;
- bitwise operators: `&`, `|`, `^`, `~`;
- shifts: `<<`, `>>`.

Precedence, from lowest to highest:

```text
||
&&
==  !=
<  <=  >  >=
|  ^
&
+  -
*  /   // division may be added later; v0 parser can support token but validation may restrict
<<  >>
unary ! ~ -
primary literal/identifier/parenthesized expression
```

For Phase 0, the parser accepts the grammar. Semantic type checking is a later validator concern.

### 5.10 Properties and assumptions

v0 includes a minimal PSL-like property subset so that the IR can carry properties from the start.

Property syntax:

```rfs
prop name : formula ;
assume name : formula ;
```

Supported v0 formulas:

- boolean expressions;
- `next formula`;
- `always formula`;
- `always[N] formula` (Phase 3): the body must hold at samples 0..N-1, N >= 1;
- `bounded_eventually[N] formula`.

Atomic proposition forms:

```rfs
state == Idle
state != WaitingAck
timer > 0
input_is(Tick)
input_is(Recv(Syn))
input_is(Recv(Ack(_)))
output_has(Drop)
output_has(Send(SynAck))
```

Examples:

```rfs
prop syn_ack :
  always (input_is(Recv(Syn)) -> output_has(Send(SynAck))) ;

prop timeout_progress :
  always (state == WaitingAck -> bounded_eventually[10] (state != WaitingAck)) ;

assume env_tick :
  always (bounded_eventually[5] (input_is(Tick))) ;
```

The `->` operator is syntactic sugar for implication. Formula-level boolean operators are `&&`, `||`, `!`, and parentheses; comparison atoms (`==`, `!=`, `<`, ...) combine with them at the formula level, so mixed atom kinds like `state == S && output_has(O)` parse (formula operands fall through to the non-boolean expression level - see section 6).

**Monitor semantics (Phase 3, D-P3-3).** Every in-subset property/assumption compiles to a per-label online monitor that steps once per sample; sampling is post-step (`input_is`/`output_has` refer to the transition just taken; sample 0 is the initial configuration, where both atoms are false).

- `always[N] p`: p must hold at samples 0..N-1 (countdown); violation at the first failing sample. N >= 1 (validator error otherwise).
- `bounded_eventually[N] b` (obligation): triggered at sample i by its trigger signal (the implication antecedent when directly the consequent of `->`, else true), the body must hold at some sample j with i <= j <= i + N - 1; if not, the violation is confirmed at the deadline sample i + N - 1. Top-level `be[N] p` is pending from sample 0.
- `next p`: delay register initialized to true (vacuous at sample 0).

Worked example (N = 3, trigger at i, body never holds): countdown 3 -> 2 at s=i, 2 -> 1 at s=i+1, 1 -> 0 at s=i+2 - violation confirmed at i+2; if the body holds at any of i..i+2 the obligation clears. Formulas outside the monitor-compilability restrictions R1-R4 (section 21) parse and validate but are skipped with a warning in every backend.

Full PSL-like operators such as unbounded `eventually`, `until`, `weak_until`, and `disable iff` are planned later.

---

## 6. EBNF-style grammar for RFS v0 parser

The following grammar is a practical baseline for the Phase 0 parser. It is not necessarily the final published grammar, but it defines the current implementation target.

```ebnf
source        = module* ;
module        = "module" identifier "{" module_item* "}" ;

module_item   = enum_decl
              | initial_decl
              | event_decl
              | output_decl
              | var_decl
              | transition
              | prop_decl
              | assume_decl
              ;

enum_decl     = "enum" identifier "{" identifier { "," identifier } ","? "}" ;
initial_decl  = "initial" identifier ;
event_decl    = "event" event_head ;
output_decl   = "output" output_head ;

event_head    = identifier
              | identifier "(" event_variant_payload ")"
              ;
event_variant_payload = identifier
                      | identifier "(" field_list ")"
                      ;

output_head   = identifier
              | identifier "(" output_variant_payload ")"
              ;
output_variant_payload = identifier
                       | identifier "(" field_list ")"
                       ;

field_list    = field { "," field } ","? ;
field         = identifier ":" type ;

type          = "bool"
              | "bit" "<" unsigned_integer_literal ">"
              | identifier
              ;

var_decl      = "var" identifier ":" type "init" expression ;

transition    = "transition" identifier "on" event_pattern
                [ "where" expression ]
                "{" action* "}" ;

event_pattern = identifier
              | identifier "(" pattern_variant_payload ")"
              ;
pattern_variant_payload = identifier
                        | identifier "(" pattern_field_list ")"
                        ;
pattern_field_list = pattern_field { "," pattern_field } ","? ;
pattern_field    = identifier
                 | identifier ":" pattern_expr
                 | "_"
                 ;
pattern_expr     = identifier
                 | "_"
                 | literal
                 ;

action          = next_action
                | emit_action
                | set_action
                ;
next_action     = "next" identifier ";" ;
emit_action     = "emit" output_expr ";" ;
set_action      = "set" identifier "=" expression ";" ;

output_expr     = identifier
                | identifier "(" expression { "," expression } ","? ")"
                ;

prop_decl       = "prop" identifier ":" formula ";" ;
assume_decl     = "assume" identifier ":" formula ";" ;

// Formula level (properties and assumptions). Precedence, loosest to tightest:
// ||, ->, &&, !. `&&`/`||` are formula-level operators; each operand is parsed
// independently via primary_formula, whose fallthrough is the NON-BOOLEAN
// expression level (comparisons and below) - so mixed atom kinds like
// `state == S && output_has(O)` parse.
formula         = or_formula ;
or_formula      = impl_formula { "||" impl_formula } ;
impl_formula    = and_formula [ "->" impl_formula ] ;
and_formula     = unary_formula { "&&" unary_formula } ;
unary_formula   = "!" unary_formula | primary_formula ;
primary_formula = "next" "(" formula ")"
                | "always" "[" unsigned_integer_literal "]" "(" formula ")"
                | "always" "(" formula ")"
                | "bounded_eventually" "[" unsigned_integer_literal "]" "(" formula ")"
                | "input_is" "(" event_pattern ")"
                | "output_has" "(" output_pattern ")"
                | "(" formula ")"
                | non_boolean_expr
                ;

non_boolean_expr = equality_expr ;

// Expression level (var init, guards, set actions): the full ladder including
// the logic levels. Formulas use only `equality_expr` and below.
expression      = logic_or ;
logic_or        = logic_and { "||" logic_and } ;
logic_and       = equality_expr { "&&" equality_expr } ;
equality_expr   = compare_expr { ("==" | "!=") compare_expr } ;
compare_expr    = add_expr { ("<" | "<=" | ">" | ">=") add_expr } ;
add_expr        = mul_expr { ("+" | "-") mul_expr } ;
mul_expr        = shift_expr { "*" shift_expr } ;
shift_expr      = unary_expr { ("<<" | ">>") unary_expr } ;
unary_expr     = ("!" | "~" | "-") unary_expr | primary_expr ;
primary_expr    = literal
                | identifier_path
                | "(" expression ")"
                ;

literal         = unsigned_integer_literal
                | "true"
                | "false"
                ;
identifier_path = identifier { "::" identifier } ;
identifier      = letter { letter | digit | "_" } ;
```

Notes:

- The parser should accept trailing commas in lists where convenient.
- The parser should report useful span-based errors.
- Semantic validation, such as unknown identifiers and type checking, is separate from parsing but should be added immediately after Phase 0.

---

## 7. IR v0 schema

The IR is versioned and serializable. Phase 0 uses JSON for human inspection and tests.

Top-level shape:

```json
{
  "ir_version": 0,
  "modules": []
}
```

Module shape:

```json
{
  "name": "handshake",
  "enums": [],
  "initial_state": "Idle",
  "events": [],
  "outputs": [],
  "vars": [],
  "transitions": [],
  "properties": [],
  "assumptions": []
}
```

Enum:

```json
{
  "name": "State",
  "variants": ["Idle", "WaitingAck", "Established"]
}
```

Event group:

```json
{
  "name": "Recv",
  "variants": [
    { "kind": "unit", "name": "Syn" },
    {
      "kind": "struct",
      "name": "Ack",
      "fields": [
        { "name": "ack_seq", "type": { "kind": "bit", "width": 4 } }
      ]
    }
  ]
}
```

Output group has the same shape as an event group.

Variable:

```json
{
  "name": "timer",
  "type": { "kind": "bit", "width": 3 },
  "init": { "kind": "int", "value": 7 }
}
```

Type:

```json
{ "kind": "bool" }
{ "kind": "bit", "width": 8 }
{ "kind": "enum", "name": "Mode" }
```

Transition:

```json
{
  "id": 0,
  "state": "WaitingAck",
  "pattern": {
    "group": "Recv",
    "variant": "Ack",
    "fields": [
      { "name": "ack_seq", "pattern": { "kind": "bind", "name": "ack_seq" } }
    ]
  },
  "guard": {
    "kind": "binop",
    "op": "==",
    "left": { "kind": "ident", "path": ["ack_seq"] },
    "right": { "kind": "ident", "path": ["seq"] }
  },
  "next_state": "Established",
  "emits": [],
  "assignments": [
    { "var": "timer", "value": { "kind": "int", "value": 0 } }
  ],
  "priority": null
}
```

Pattern field patterns:

```json
{ "kind": "wildcard" }
{ "kind": "bind", "name": "ack_seq" }
{ "kind": "literal_int", "value": 0 }
{ "kind": "literal_bool", "value": true }
{ "kind": "enum_variant", "path": ["Mode", "Normal"] }
```

Expression nodes are represented as tagged JSON objects. Examples:

```json
{ "kind": "int", "value": 7 }
{ "kind": "bool", "value": true }
{ "kind": "ident", "path": ["timer"] }
{ "kind": "enum_variant", "path": ["Mode", "Normal"] }
{ "kind": "binop", "op": "+", "left": {}, "right": {} }
{ "kind": "unary", "op": "!", "expr": {} }
```

Property:

```json
{
  "name": "syn_ack",
  "formula": {
    "kind": "always",
    "inner": {
      "kind": "implies",
      "left": { "kind": "input_is", "pattern": {} },
      "right": { "kind": "output_has", "pattern": {} }
    }
  }
}
```

Assumptions use the same property shape but are stored separately.

Phase 3 adds one additive formula variant: `always[N] (inner)` serializes as
`{ "kind": "bounded_always", "bound": N, "inner": {} }`. It is purely additive -
no `IR_VERSION` bump; existing IR JSON round-trips unchanged (D-P3-1).

### IR design rules

1. The IR must be lossless for the v0 language subset.
2. The IR must be serializable to JSON and deserializable back to an equivalent structure.
3. The IR should be stable enough that later code generators can depend on it.
4. Parser-specific source spans should not污染 the core IR unless a separate annotated AST is used. Phase 0 can keep spans in parser diagnostics and optionally in an extended AST, but the serializable IR should remain clean.

---

## 8. Parser design

### 8.1 Parser implementation choice

| Option | Pros | Cons | Evaluation in the RFS workspace | Decision |
|---|---|---|---|---|
| Hand-rolled lexer + recursive descent parser | Full control, good errors, no parser-library version risk, easy to test, small dependency surface | More initial code; duplicated low-level token logic | Used for the first prototype, then replaced after the requested library evaluation | Replaced |
| `nom` 8 parser combinators | Mature, complete parsers, direct `IResult` values, stable integration with Rust/Cargo, easy unit tests, good fit for recursive descent | Some combinators are direct functions while others require `.parse_complete`, lifetime/error-type annotations are needed in helpers, rich diagnostics require extra work | Installed as a normal dependency; RFS parser rewritten on nom 8; all unit and example tests pass | **Phase 0 choice** |
| `chumsky` 0.13 parser combinators | Ergonomic PEG style, modern API, useful `text::unicode` helpers, potentially better error recovery | Some 0.13 API details require explicit output-type annotations and `ParseResult::into_result`; not needed for the current minimal parser | Installed as a dev-dependency; working comparison smoke test added; kept as a future candidate for richer diagnostics/recovery | Future candidate |
| LALR/PEG generated parser | Strong grammar discipline | Toolchain complexity, less flexible errors, overkill for v0 | Not suitable for the current minimal demonstrator | No |

Phase 0 uses `nom` 8 for the RFS parser. The implementation is recursive descent over `&str` input. A small preprocessing pass replaces `#` and `//` comments with spaces while preserving newlines, so line/column diagnostics remain stable. The parser builds the versioned IR directly.

### 8.2 Token recognition

RFS v0 does not use a separate token-pass lexer. Tokens are recognized on demand by nom combinators and small helper parsers.

Token categories include:

- identifiers;
- integer literals;
- keywords: `module`, `enum`, `initial`, `event`, `output`, `var`, `transition`, `on`, `where`, `next`, `emit`, `set`, `prop`, `assume`, `bool`, `bit`, `init`, `true`, `false`;
- punctuation: `{ } ( ) [ ] < > : ; , == != <= >= && || -> << >> + - * & | ^ ~ ! =`;
- comments removed before parsing.

Helper parsers use maximal-munch behavior where needed, for example identifiers consume all ASCII alphanumeric/underscore characters, integer literals consume all ASCII digits, and two-character operators are tried before conflicting one-character operators.

### 8.3 Parser error model

Parser errors should include:

- message;
- byte span or line/column range;
- source snippet if available;
- expected token description where practical.

Phase 0 does not require full error recovery. The parser may stop at the first error. Later tooling can improve recovery for LSP use.

### 8.4 Parser API

Recommended public API:

```rust
pub fn parse_source(source: &str) -> Result<Ir, ParseError>;
pub fn parse_file(path: &std::path::Path) -> Result<Ir, ParseError>;
```

The parser returns the IR directly in Phase 0. If a separate AST becomes useful, it can be introduced without changing the external API significantly.

---

## 9. Major design choices

### 9.1 DSL hosting

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| External Rust-like DSL, `.rfs` files | Stable syntax, own parser, good diagnostics, easy to version, multiple targets from IR | Separate language, requires tooling | High | **Primary** |
| Embedded Rust procedural macro DSL | Lives in Rust files, familiar | Macro errors can be poor, harder multi-target generation, syntax constrained by Rust macros | Medium | Optional later |
| YAML/TOML plus Rust types | Simple data model | Too weak for guarded transitions and temporal properties | Low | No |
| IR-only JSON/protobuf | Tool interchange | Not user-friendly as primary authoring language | Internal only | Use as internal representation |

### 9.2 FSM authoring style

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Explicit transition table | Easy determinism checks, compact for tiny machines | Verbose with guards/actions | Medium | No as primary |
| Graph/diagram syntax | Visual | Poor for data, properties, version control | Low | Generated diagrams only |
| Rust-like pattern-matching transition blocks | Familiar, maps to Rust `match`, supports bindings/guards/actions | Requires clear priority semantics | High | **Primary** |
| Statecharts/nested regions | Hierarchical protocols | More complex semantics and code generation | Later | Not in v0 |

### 9.3 Data model

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Arbitrary Rust types | Expressive | Unbounded, side effects, impossible or unsound MC | Low | No for core |
| Fixed-width finite types: `bool`, enums, `bit<N>` | Finite state space, direct Rust mapping, sound MC | May require abstraction for large protocol fields | High | **Core** |
| Symbolic SMT-style unbounded integers | More abstract | Incomplete without bounded/SMT techniques, harder production mapping | Advanced optional | Later |
| Opaque/pass-through payload fields | Reduces state space when fields are irrelevant | Sound only if behavior is independent of those fields | Limited | Allow unused fields as don't-care later |

### 9.4 Determinism and transition selection

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Pure nondeterministic transitions | Good over-approximation | Production needs deterministic behavior | MC-only | Optional MC mode later |
| Deterministic by declaration order | Simple | Hidden semantics, fragile ordering | Low | No |
| Explicit priorities plus disjointness validation | Precise, production-safe, good diagnostics | More authoring effort | High | **Primary** |
| Hybrid deterministic production + optional nondeterministic MC | One spec, precise production, stronger verification option | Requires clear documentation of modes | High | Recommended later |

v0 parser may accept an optional `priority` extension later. The initial grammar does not require priorities.

### 9.5 Time and timeouts

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| No time, only events | Simplest | Cannot express timeouts | Low for protocols | No |
| Explicit `Tick` event plus timer variables | Discrete, finite, easy MC/codegen | User must model ticks explicitly | High | **Core** |
| Timed automata with real-time clocks | Natural timeouts | Continuous time, much harder MC/codegen | Later extension | No for v0 |
| Implicit `after N` delays | Concise | Desugars into tick/timer anyway | Sugar later | Maybe later |

### 9.6 Property language

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| PSL-like subset | Requested style, practical for event sequences, compiles to monitors | Requires parser and careful semantics | High | **Primary** |
| Pure LTL | Simpler theory | Less practical for bounded protocol patterns | Medium | Internal foundation |
| SVA-like | Familiar in hardware/SystemVerilog context | Dialect differences from PSL | Medium | Avoid; use PSL-like naming |
| Rust closures as properties | Flexible | Not automatically model-checkable, unbounded code | Low for formal properties | No for core properties |
| Separate monitor DSL | Precise | Another language to learn | Low | Internal only |

### 9.7 Model-checking engine strategy

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Brute-force input sequence enumeration up to depth K | Extremely simple, good for MVP | Exponential, no state reuse, incomplete | MVP only | Phase 1 |
| Exhaustive reachable-state BFS/DFS with state caching | Complete for small finite systems, gives counterexamples | Memory/time explosion for large data | Core exact mode | Phase 2/3 |
| Symbolic BDD/bitset model checking | Handles larger structured state spaces, can be complete | Significant implementation complexity | Advanced | Later optional |
| SMT-based bounded model checking | Good bug hunting with wider bitvectors | Incomplete, liveness requires k-liveness/fairness tricks, solver dependency | Advanced optional | Later |
| k-liveness for liveness | Converts some liveness checks to bounded safety checks | Bound may be hard to choose | Advanced | Later with SMT/BMC |

The model checker should report statuses such as:

```text
PASS (complete)
PASS (bounded K)
FAIL (counterexample available)
UNKNOWN (resource limit reached)
```

This avoids presenting bounded bug hunting as full proof.

### 9.8 Production integration style

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Generated standalone pure `step` function | Simple, testable, no runtime coupling | User must wire I/O themselves | High | Core |
| Trait-based adapters over the pure FSM | Composable, supports sync/async runtimes | More abstraction layers | High | Recommended |
| Runtime-specific generated code, e.g. tokio-only | Convenient for specific stack | Couples DSL toolchain to async runtime | Medium | Optional feature only |

### 9.9 Error handling in production code

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Panic on unexpected event | Simple | Unsafe for production protocol handlers | Low | No as default |
| Return `Result` / error type | Recoverable, testable, explicit | Caller must handle errors | High | Default |
| Explicit error state or default transition | Protocol can reset/drop/recover | More specification surface | High | Supported later |

### 9.10 Assumptions about the environment

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| No assumptions, fully nondeterministic environment | Strongest verification | Often infeasible or unrealistic | Useful stress mode | Keep as option |
| PSL-like `assume` properties | Concise, close to property language, easy MC integration | Assumptions are MC-only unless separately enforced | High | **Core** |
| Separate environment automaton | Very expressive for complex peer behavior | More authoring, larger product state | Later extension | Optional later |

### 9.11 Composition

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Single endpoint FSM | Simple MC/codegen, easier v1 | Cannot model client/server interaction directly | High for v0/v1 | **Primary** |
| Modules with ports and product composition | More realistic multi-party protocols | State explosion, more complex codegen | Later | Phase 5/6 extension |
| Shared global event space | Simpler than full composition | Can hide protocol structure | Medium | Maybe intermediate step |

---

## 10. Production Rust code generation plan

The production generator will emit idiomatic Rust.

Example generated shape:

```rust
pub mod handshake {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum State {
        Idle,
        WaitingAck,
        Established,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Input {
        RecvSyn,
        RecvAck { ack_seq: u8 },
        Tick,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub enum Output {
        SendSynAck,
        Drop,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
    pub struct Vars {
        pub seq: u8,
        pub timer: u8,
    }

    #[derive(Debug, Clone, Copy, PartialEq, Eq)]
    pub struct UnexpectedEvent {
        pub state: State,
        pub input: Input,
    }

    #[derive(Debug, Clone)]
    pub struct Fsm {
        pub state: State,
        pub vars: Vars,
    }

    impl Fsm {
        pub fn new() -> Self;
        pub fn step(&mut self, input: Input) -> Result<Vec<Output>, UnexpectedEvent>;
    }
}
```

Code generation rules:

- generated files must include source hash and generator version;
- variable updates must preserve simultaneous-update semantics using temporary next variables;
- transition dispatch should map naturally to Rust `match` arms;
- ambiguous transitions must be rejected or resolved by explicit priority before deterministic code generation;
- optional debug property monitors may be generated behind a feature flag later.

---

## 11. Model-checking plan

The model checker will also be generated in Rust, but it can use a shared `rfs-mc-core` library.

Architecture:

```text
generated spec crate
   |
   +-- concrete types: State, Input, Output, Vars
   +-- transition function
   +-- property monitors
   +-- assumption monitors
   |
   v
rfs-mc-core library
   |
   +-- state-space search
   +-- counterexample recording
   +-- bounded/exact modes
   +-- optional symbolic/SMT backend later
```

### Sampling model

A run is a sequence of samples.

Position `0` is the initial configuration:

```text
state = initial_state
vars = initial_vars
no incoming input event
no output action
```

For each step, the system consumes an input event, emits outputs, and moves to the next configuration. Atomic propositions in v1 property evaluation default to post-step values, except explicit `input_is` and `output_has` atoms refer to the transition just taken.

This avoids ambiguity between pre-state and post-state.

### Exact reachable-state checking

For small finite systems, the model checker performs graph search over:

```text
(system_state, system_vars, property_monitor_states)
```

If it finishes without violation, it can report `PASS (complete)`.

### Bounded checking

For larger systems, support bounded depth or state-limit modes. These must be reported as bounded results, not complete proofs.

### Liveness

v0/v1 should start with:

- safety properties;
- bounded liveness such as `bounded_eventually[N]`.

Unbounded strong liveness such as `eventually p` requires fairness and Büchi-style or fair-cycle analysis. This is planned later.

---

## 12. PSL-like property language plan

### v0 parser subset (implemented, Phase 3)

```rfs
next (formula)
always (formula)
always[N] (formula)
bounded_eventually[N] (formula)
boolean_formula
```

The whole subset is implemented and monitored ONLINE in every backend (Phase 3,
section 21): each prop/assume compiles to a per-label monitor from the shared
`MonitorPlan`. Formulas outside the monitor-compilability restrictions R1-R4
still parse and validate, but codegen skips them with an identical warning in
every backend.

### Planned full subset

```rfs
eventually formula
p until q
weak_until q
disable iff d
```

and possibly sequence operators if needed.

### Strong vs weak liveness

| Operator class | Meaning | Model-checking complexity |
|---|---|---|
| Safety, e.g. `always p` | Violation has a finite prefix | Easier, exact with reachable-state search |
| Bounded liveness, e.g. `bounded_eventually[N] p` | Must happen within N steps | Easy for small N, monitor uses countdown |
| Strong liveness, e.g. `eventually p` | Must happen in every fair infinite run | Requires fairness/Büchi/cycle detection |
| Weak liveness / weak until | May allow finite termination or no obligation | Depends on exact semantics; must be documented |

---

## 13. Diagnostics and validation plan

The validator should produce rich diagnostics.

| Diagnostic | Example |
|---|---|
| Unknown state referenced | `next Unknown` |
| Unknown event field | `Recv(Ack(missing_field))` |
| Non-finite type used | `var x: String` |
| Ambiguous transitions | Two enabled transitions without priority |
| Unreachable state | State never reached from initial |
| Dead event | Event never matched in any transition |
| Unused variable | Variable declared but not read/written meaningfully |
| Impossible guard | Guard always false, detected by bounded/SMT check later |
| Duplicate variable assignment in one transition | `set x = 1; set x = 2;` |
| Missing `next` | Transition body without next state |
| Assumption unsatisfiable | No run satisfies environment assumptions |
| Property vacuous | Property never activated due to assumptions |

Phase 0 focuses on parser diagnostics. Semantic validation is the next step.

---

## 14. Toolchain CLI plan

Planned commands:

```bash
rfs parse protocol.rfs
rfs validate protocol.rfs
rfs codegen production protocol.rfs --out src/generated/tcp.rs
rfs codegen mc protocol.rfs --out target/rfs-mc/tcp
rfs mc protocol.rfs --bound 20
rfs mc protocol.rfs --exact --state-limit 1000000
rfs test protocol.rfs --generate-tests tests/generated
rfs diagram protocol.rfs --format svg
```

Implemented so far: the Phase 1 frontend CLI - `rfs parse` (all four RFS/JSON format
combinations through the IR, optional `--target-dir`, collision guard) and
`rfs validate` (file or directory input, `.rfs-validation.err` reports). The Phase 0
positional parser mode was removed during Phase 2 preparation. Run instructions live
in `BUILD.md`; CLI behavior rules in `KB.md` section 4.5.

Conversion rules implemented in `src/frontend.rs`:

- a file input must carry the extension matching `--input` (`.rfs` or `.json`);
- a directory input is scanned recursively for files with that extension only;
- output names replace the original extension (never stack it: `a.rfs` -> `a.json`, not `a.rfs.json`);
- without `--target-dir`, each output is written next to its input file; with `--target-dir`, the input directory structure is preserved inside the target directory;
- a collision guard compares normalized absolute paths and refuses any conversion whose output would overwrite its input (exit code 1);
- on success the exit code is 0; parse, planning, or IO errors go to stderr with exit code 1.

Validation rules:

- file input: the extension selects the format; validation errors are printed to stderr and the exit code is 1;
- directory input: `--input` is required; every failing source file gets a `<name>.rfs-validation.err` text file written next to it listing its parse/semantic errors; the exit code is 1 if any file failed.

---

## 15. Testing strategy

### Parser unit tests

Unit tests cover:

- lexer tokens;
- maximal-munch operators;
- comments;
- identifiers and keywords;
- enum declarations;
- event/output grouping;
- variable declarations;
- transitions with guards/actions;
- expressions and precedence;
- property formulas;
- error spans.

### Example-driven integration tests

All valid examples under `examples/valid/` must parse successfully and produce IR.

All invalid examples under `examples/invalid/`, if present, must fail parsing with a non-empty diagnostic.

The test suite should be extensible by adding new `.rfs` files without changing test code.

Optional per-example metadata can be added later using sidecar files, for example:

```text
example.rfs.expect.json
```

For Phase 0, successful parsing plus basic IR sanity checks are sufficient.

### Round-trip tests

IR serialization tests should verify:

```text
source -> IR -> JSON -> IR
```

is stable for the v0 subset.

---

## 16. Risks and mitigations

| Risk | Impact | Mitigation |
|---|---|---|
| State explosion due to variables/events | MC becomes unusable for real protocols | Fixed-width finite types, variable abstraction, bounded modes, clear `UNKNOWN` reporting |
| Gap between MC model and production behavior | False confidence | Same IR for both targets, deterministic production mode, runtime debug monitors, fuzzing |
| PSL translation bugs | Incorrect verification results | Restricted subset first, differential testing, reference tests, property monitor unit tests |
| DSL complexity creep | Toolchain becomes hard to maintain | Strict v0 scope, IR versioning, deprecation process, examples-driven design |
| Large protocol fields cannot be modeled exactly | Limited applicability | Event-level modeling, don't-care abstraction for unused fields, later bounded/SMT modes with clear labels |
| Liveness semantics misunderstood by users | Misuse of `eventually` | Document strong/weak liveness, fairness, and bounded alternatives |
| Generated code is hard to debug | User friction | Stable generated API, source spans in errors, spec hash, readable Rust output |
| Model checker falsely reports completeness | Trust issue | Explicit status: complete, bounded, unknown; resource limits enforced |

---

## 17. Phase 0 deliverables

Phase 0 delivers:

1. `DESIGN.md` with core design choices;
2. `ROADMAP.md` with development plan (now split into `docs/roadmap/`);
3. Rust parser for the RFS v0 subset;
4. IR v0 data structures;
5. JSON serialization/deserialization for IR v0;
6. CLI entry point for parsing files or directories;
7. unit tests for lexer/parser/IR;
8. 20-30 valid RFS examples;
9. extensible example-based parser test suite.

Phase 0 non-goals:

- semantic validation beyond basic parse success;
- production Rust code generation;
- model-checking executable generation;
- full PSL operators;
- priorities;
- composition;
- LSP/IDE support.

---

## 18. Example RFS v0 specification

```rfs
# rfs 1.0

module handshake {
  enum State {
    Idle,
    WaitingAck,
    Established
  }

  initial Idle

  event Recv(Syn)
  event Recv(Ack(ack_seq: bit<4>))
  event Tick

  output Send(SynAck)
  output Drop

  var seq: bit<4> init 0
  var timer: bit<3> init 7

  transition Idle on Recv(Syn) {
    next WaitingAck;
    emit Send(SynAck);
    set seq = 1;
  }

  transition WaitingAck on Recv(Ack(ack_seq)) where ack_seq == seq {
    next Established;
    set timer = 0;
  }

  transition WaitingAck on Tick where timer > 0 {
    next WaitingAck;
    set timer = timer - 1;
  }

  transition WaitingAck on Tick where timer == 0 {
    next Idle;
    emit Drop;
  }

  prop syn_ack :
    always (input_is(Recv(Syn)) -> output_has(Send(SynAck))) ;

  prop timeout_progress :
    always (state == WaitingAck -> bounded_eventually[10] (state != WaitingAck)) ;

  assume env_tick :
    always (bounded_eventually[5] input_is(Tick)) ;
}
```

---

## 19. Phase 1 code-generation decisions

Phase 1 introduces a validator and Rust translator. The key decisions are recorded in `KB.md` section 13, summarized here:

- Code generation consumes the validated IR, not parser text.
- `bit<N>` maps to the smallest suitable primitive integer; arithmetic uses wrapping `u128` intermediates and masks to the declared width.
- Variable updates are simultaneous: all assignment right-hand sides are evaluated before any variable is written.
- Assumption violations use a trait-based handler passed to `step_with_handler`; the default `step` panics.
- The translator accepts an assumption-selection option: `all`, `none`, or comma-separated labels. Default is `all`.
- Only safety formulas in the current translated subset become runtime checks or property checks. Formulas containing `bounded_eventually` are skipped with a warning. **Phase 3 update (D-P3-4/D-P3-5):** all in-subset formulas - including `bounded_eventually` - are now online-monitored in both backends; only out-of-subset formulas (R1-R4) are skipped. See section 21.
- Generated unit tests include initial-state, positive bounded-sequence, and negative unexpected-input tests. The sequence length is configurable, default 4.
- The generated bounded model checker uses representative input constructors by default, can inject unexpected events, and can force selected safety assumptions to violate with per-run probabilities.

## 20. Phase 2 core-DSL decisions

Phase 2 makes the DSL practical for real endpoint protocol FSMs. The IR already
carried `priority`, pattern bindings, and output payloads (forward-compatible
since Phase 0); Phase 2 wires them through validation and code generation, adds
two new syntax forms, and adds the C backend. Decisions are recorded as
comparison tables; the chosen option is marked.

### D1. Priority direction

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Larger number wins (checked first) | Matches "priority 10 beats priority 5" intuition; easy to explain in specs | Inverted vs interrupt/nice conventions | **Chosen** |
| Smaller number wins | Matches POSIX nice / interrupt priorities | Counter-intuitive for spec authors ("is 1 top or bottom?") | Rejected |

### D2. Ambiguity and duplicate detection (same state, identical event pattern)

Two transitions in one state can conflict only when their patterns are
identical (different variants of different groups never match the same input).

| Situation | Severity | Rationale |
|---|---|---|
| Identical pattern + identical guard (both absent or structurally equal) | **Error** ("duplicate transitions") | The second transition is provably unreachable (shadowed). |
| Identical pattern + two *different* explicit priorities | Allowed | Deterministic ordering; the author pinned intent. |
| Identical pattern + duplicate explicit priority value | **Error** ("duplicate priority") | A tie cannot be ordered deterministically. |
| Identical pattern, no priorities, different (or one missing) guard | **Warning** ("order-dependent") | First match wins in source order; legal if/else chains stay valid, but the author is told to pin intent with priorities. |

Guards are compared structurally (no value analysis): overlapping guards like
`x < 5` / `x >= 3` produce the warning even when some inputs are disjoint.
This is the "at least syntactically" level required by the roadmap; bounded
checking is left to Phase 4.

### D3. Duplicate-priority scope

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Priorities must be unique per (state, pattern) | Numbers can be reused across unrelated patterns in the same state; minimal constraint | Slightly more complex rule to explain | **Chosen** |
| Priorities must be unique per state (regardless of pattern) | Simplest rule | Forces a global numbering scheme per state even for patterns that cannot conflict (rejected after it broke a natural spec during implementation) | Rejected |

### D4. Severity of reachability/usage diagnostics

| Check | Severity | Rationale |
|---|---|---|
| Unreachable state | **Warning** | Over-approximated (guards ignored); legitimate specs may have defensive states; must not break the 40 existing valid examples |
| Dead event (constructor never in any pattern) | **Warning** | Same reasoning; bare group patterns count as using all of the group's variants |
| Unused variable (never read or written) | **Warning** | May be intentional documentation |
| Impossible guard (literal `false`; same variable compared equal to two different integer literals in one top-level conjunction) | **Warning** | Syntactic detection only; the transition is dead but the spec still compiles |

Warnings never block code generation or validation exit codes; they are printed
by `rfs validate` and surfaced through codegen output warnings.

### D5. Effective priority default

A transition without `priority N;` has effective priority 0 (lowest). Within a
state, arms are checked in descending effective priority (stable for ties; ties
are validator errors). Cross-state ordering is irrelevant because only the
active state can match. Generated code groups transitions by state and sorts
each group, so specs without priorities generate behaviorally identical code to
Phase 1 (source order preserved within a state).

### D6. Error handling for unexpected inputs

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Module-level `on_error { next S; [emit O(...);] }` block | Explicit, one per module; no new pattern grammar; works in every state; MC-compatible (it is just a fallback arm) | Cannot vary the recovery target per state | **Chosen** |
| Wildcard transition (`transition A on _ { ... }`) | Per-state flexibility | Requires a new pattern form that would clash with the existing `_` field wildcard; easy to misuse | Rejected |
| Policy keyword (`on_error policy: ignore/error/recover`) | Compact | "ignore" silently drops protocol errors (bad for model checking); less readable in specs | Rejected |

Semantics: when no transition matches, `on_error` executes in the current state:
it moves to its declared state and emits its outputs; it has **no pattern
bindings** and cannot assign variables (values carry over unchanged). Without
`on_error`, behavior is unchanged from Phase 1 (`StepError::UnexpectedEvent` /
`RFS_UNEXPECTED_EVENT`).

### D7. Module version header syntax

| Option | Pros | Cons | Decision |
|---|---|---|---|
| `module Name version "x.y.z" {` on the module line | Visible at a glance; one optional token; trivially round-trips | Version is per-module (a file with several modules repeats it) | **Chosen** |
| `meta version = "..."` item inside the module | Extensible to more metadata later | A new item kind for what is currently a single value; no second metadata key exists yet | Rejected (revisit if more metadata keys appear) |

The version is stored in the IR (`Module.version: Option<String>`, serde-defaulted,
`skip_serializing_if = "Option::is_none"`), printed by the RFS printer, and
emitted as `pub const SPEC_VERSION` (Rust) / `#define RFS_SPEC_VERSION` (C).

### D8. C backend type mapping and scope

| Item | Decision | Notes |
|---|---|---|
| `bool` | `bool` (`<stdbool.h>`) | |
| `bit<N>` | `uintN_t` for N in {8, 16, 32, 64}; other widths are a **codegen error** | C has no wider fixed-width types; Rust uses u128 intermediates, C relies on native unsigned wrap-around (identical results mod 2^N) |
| enums | `typedef enum { NAME_VARIANT } Name_t` | State enum variants get the `STATE_` prefix |
| inputs/outputs | payload structs + `kind` int + anonymous union (`input_t` / `output_t`) | Classic C tagged-union representation |
| simultaneous updates | temp variables, exactly like Rust | All RHS evaluated from old values before any assignment |
| assumption handling | function pointer `rfs_policy_t (*handler)(void*, const char*)` with CONTINUE/STOP/PANIC policies; default handler aborts | Mirrors the Rust `AssumptionHandler` trait |
| unit tests | plain-assert harness in `main()`, failure counting (no framework) | Same `TestPlan` semantics as the Rust backend, including the on_error landing check |
| model checker | **same runtime wire format** as the Rust MC: `--bound --runs --seed --assumes --violate --unexpected-prob`, same LCG constants (6364136223846793005 / 1442695040888963407), same three-phase step strategy, same summary line | Counterexample traces print state/input kinds (C has no Rust-style Debug) |
| property translation | same translated safety subset as Rust (Expr/Not/And/Or/Implies/Next/InputIs/OutputHas). **Phase 3 update (D-P3-4/D-P3-5):** all in-subset formulas - including bounded liveness - are now online-monitored in BOTH backends from the shared MonitorPlan; only out-of-subset formulas (R1-R4) are skipped, with identical warning text. See section 21 | |

### D9. Feature flags (std/no_std, property-monitors, logging hooks)

Deferred beyond Phase 2: the roadmap marks `no_std` as "later"; no consumer
needs the other two yet. Recorded so they are not silently dropped - they remain
Phase 5 candidates. Generated code is already dependency-free single files, which
keeps this door open.

### D10. IR evolution rule

Additive optional fields (serde `default` + skip-when-none) do **not** bump
`IR_VERSION`; existing IR JSON keeps deserializing. Breaking changes to the IR
shape must bump `IR_VERSION` and update round-trip tests. Phase 2 added
`Module.version` and `Module.on_error` under this rule (IR stays v0).

### Phase 2 summary of what was already present

Verified during implementation (no new code needed, only docs/tests): full finite
types; event payload patterns with bindings; guards on bound fields; output
payloads; multiple variable updates; simultaneous updates via temp variables;
derives on generated types; the clear error type (`StepError` / `RFS_UNEXPECTED_EVENT`);
parser acceptance of `priority N;`; duplicate-`set` rejection at parse time.

---

## 21. Phase 3 property/monitor decisions

Phase 3 implements the PSL-like property subset with ONLINE monitors in every
backend: each `prop`/`assume` compiles (in the shared, language-agnostic
`src/monitor.rs`) to a per-label `MonitorPlan`, and both backends render that
plan into generated monitor code with identical semantics. Decisions are
recorded as comparison tables; the chosen option is marked.

### D-P3-1. IR representation of `always[N]`

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Additive `BoundedAlways { bound, inner }` variant (serde tag `bounded_always`), no `IR_VERSION` bump | Old IR JSON round-trips unchanged; purely additive per the D10 rule | Variant set grows; consumers must tolerate unknown tags gracefully | **Chosen** |
| Bump `IR_VERSION` to v1 with a migration note | Explicit version boundary | Breaks all existing IR JSON for a purely additive change | Rejected |

### D-P3-2. Zero-bound rules

| Option | Pros | Cons | Decision |
|---|---|---|---|
| `always[N]` requires N >= 1 (validator error); `bounded_eventually[0]` stays legal and means "the body must hold at the trigger sample itself" | New syntax has no compatibility concern; `be[0]` keeps a meaningful degenerate form | Two different zero-bound rules to document | **Chosen** |
| Reject both zero bounds uniformly | One simple rule | Loses the useful "hold now" form of `be[0]`; no existing example uses either form (verified) | Rejected |

### D-P3-3. Exact monitor semantics (deadline confirmation)

Sampling follows section 11: per-sample values are post-step; `input_is`/
`output_has` refer to the transition just taken; sample 0 is the initial
configuration (no input/outputs, so both atoms are false there).

Obligation monitor algorithm per sample s:

```text
a = eval(trigger signal at s)   # implication antecedent, or true for bare obligations
b = eval(body signal at s)
if !pending and a: pending = true; remaining = N; trigger_step = Some(s)
if pending:
    if b: pending = false; trigger_step = None
    else: remaining -= 1; if remaining == 0: VIOLATION confirmed at sample s
```

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Violation confirmed at the deadline sample i + N - 1 (countdown reaches 0) | Tighter counterexamples (violation step = last allowed sample); matches the countdown implementation; pinned by monitor-compiler tests before any codegen rendered it | PHASE_3.md's conceptual pseudocode reports one sample later - docs had to be reconciled | **Chosen** |
| Confirm one sample after the deadline (i + N) | Matches the original conceptual pseudocode | Counterexamples point past the last allowed sample; off-by-one vs the countdown | Rejected |

Worked check (N = 3, trigger at i, body never holds): s=i rem 3->2, s=i+1 rem
2->1, s=i+2 rem 1->0 -> violation confirmed at i+2; if the body holds at any of
i..i+2 the obligation clears. `always[N] p`: p must hold at samples 0..N-1
(countdown; violation at the first failing sample). Top-level `be[N] p`:
obligation pending from sample 0. `next p` in a monitor: delay register
initialized to true (vacuous at sample 0), matching the pre-Phase-3 post-hoc
semantics.

### D-P3-4. Online monitors replace post-hoc checks

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Monitors step online in `commit_step`/`rfs_commit`; remove the generated post-hoc `property_check`/`assume_check` trace functions in BOTH backends | Single evaluation path; bounded-liveness obligations monitorable mid-run; assumption pruning works online in MC (incl. liveness assumptions) | Bigger refactor of both backends' generated code | **Chosen** |
| Keep post-hoc trace checks and add monitors alongside | Smaller change | Two evaluation paths can diverge; liveness still unmonitorable online; duplicated monitor state | Rejected |

Assumption enforcement stays on the existing `AssumptionHandler` path (a
monitor-driven assumption violation is reported through it), so all MC/test
call sites - trial-step forced violations, `--assumes`, integration tests - keep
working unchanged. The `PROPERTY_LABELS`/`ASSUME_LABELS` constants stay.

### D-P3-5. C backend: full monitor parity

| Option | Pros | Cons | Decision |
|---|---|---|---|
| Full monitor parity in C: the same `MonitorPlan` rendered in C (task 11), C MC gets online detection + enhanced counterexamples (task 12), cross-backend differential tests verify identical behavior (task 17) | Backend parity principle (INTRO.md): same functionality and exactly the same observable behavior in ALL backends; differential testing is meaningful | C rendering work is real scope; C.md doc update was deferred to this task (correction 11a) | **Chosen** |
| Rust-only monitors in Phase 3, C later | Less Phase 3 scope | Violates the backend parity principle; C MC would silently skip liveness assumptions and property detection | Rejected |

Monitor-compilability restrictions R1-R4 (enforced by `monitor::compile_formula`,
identical in every backend):

- **R1:** no `bounded_eventually` under negation (directly or indirectly).
- **R2:** a `bounded_eventually[N] b` node requires `b` to be temporal-operator-free and may appear (a) at top level, or (b) inside an `always`/`always[N]` body - except under another `bounded_eventually` and not under `next`. Its trigger signal is the implication antecedent when it is directly the consequent of `->`, else true.
- **R3:** `always[N]` only at top level or under boolean connectives - never under `next`, `always`, `always[N]`, or `bounded_eventually`.
- **R4:** `next` allowed anywhere safety is allowed, except inside a bounded-eventually body (see R2).

Formulas outside R1-R4: parse + validate succeed; codegen in EVERY backend
emits the same warning (`property '<name>' is outside the monitor subset;
skipped` / `assume '<name>' ...`) and generates no monitor for that label.

### D-P3-6. CLI flag and per-language gating

| Option | Pros | Cons | Decision |
|---|---|---|---|
| `--with-property-monitors` applies to the PRODUCTION command of every backend; mc and test always include monitors; gating is per-language with zero-cost-when-disabled semantics (Rust `#[cfg(debug_assertions)]`, C `#ifdef RFS_PROPERTY_MONITORS`) | Release production builds pay zero monitor cost; MC/tests never need an opt-in flag; one flag across backends | Two gating mechanisms to document | **Chosen** |
| Runtime toggle, or always-on monitors everywhere | Simplest mental model | Release builds pay monitor cost unconditionally; MC would need a way to disable (it must not - violations are the point) | Rejected |

New generated error surface: Rust `StepError::PropertyViolation { label,
step }`; C `RFS_PROPERTY_VIOLATION` status in `step_result_t` with
`prop_label`/`prop_step`/`prop_trigger_step` fields.
