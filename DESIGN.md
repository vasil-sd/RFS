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

> Normative semantics of the language this pipeline compiles: `docs/SEMANTICS.md` (Phase 6 task 1).

```text
protocol.rfs  <----+  rfs parse (format conversion, human review)
     |             |
     v             v
nom parser      IR JSON (hand-rolled json_rw) <---- external tooling via JSON
(src/parser.rs)    |
     |             v
     +------> IR v0 (versioned, serializable)
                 |
                 +--> frontend CLI: rfs parse / rfs validate  [Phase 1]
                 |        RFS<->JSON conversion, semantic validation,
                 |        .rfs-validation.err reports (text mode) or
                 |        structured --json diagnostics (Phase 5 task 13)
                 |
                 +--> rfs info  [Phase 5 task 3]
                 |        static spec introspection (text or --json)
                 |
                 +--> codegen plugin registry: rfs codegen  [Phase 1]
                 |        core (src/codegen.rs): Backend API, BACKENDS registry,
                 |        CLI parsing, help rendering
                 |        shared analysis: codegen_fsm / codegen_test / codegen_mc /
                 |        mc_plan / monitor / smt_plan+smt_emit
                 |        rust backend (src/backends/rust*):
                 |          production -> enum State/Input/Output/Vars, Fsm::step(...)
                 |                       (+ --no-std variant, + opt-in ProtocolFsm impl)
                 |          test       -> extended unit tests + fuzz skeleton extra file
                 |          mc         -> exact model-checker executable (text|json report)
                 |        c backend (src/backends/c*): same three commands, parity-pinned
                 |
                 +--> rfs test  [Phase 5 tasks 7/17]
                 |        generate + compile + run unit/fuzz suites in one invocation
                 |
                 +--> rfs diagram  [Phase 5 task 8]
                        state diagrams / transition tables (dot/puml/mmd/table/md)
```

Workspace crates around the toolchain (Phase 5): `crates/rfs-build` (build.rs
helper), `crates/rfs-adapters` (ProtocolFsm trait + generic adapters), and the
example crates under `crates/examples/`; IDE support ships in-repo under
`ide/` (TextMate grammar + snippets, Phase 5 task 13).

The IR is the single source of truth for all generators. This prevents production code and model-checking code from diverging.

Because both `.rfs` sources and IR JSON files normalize to the same IR, every tool in the chain (parser, validator, code generators, frontend CLI) accepts either representation: RFS text is parsed into IR, IR JSON is deserialized into IR, and formatted output can be written back as either `.rfs` or `.json`.

### Code generation as a plugin system [Phase 1]

RFS targets two languages (Rust and C, both shipped - the C backend since
Phase 2), so code generation is built as a plugin system:

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

> The precise, normative form of this section (transition relation, simultaneous
> updates, priorities, deterministic vs nondeterministic modes, sampling): `docs/SEMANTICS.md` sections 2-4.

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

> The normative semantics these constraints enable (configurations, abstraction
> rules, exact exploration, status labeling): `docs/SEMANTICS.md` section 9.

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
- `bounded_eventually[N] formula`;
- `eventually (formula)` (Phase 4): unbounded liveness - the body must hold at some position of the path; the parentheses are mandatory;
- `p until q` (Phase 4): WEAK until - `q` holds at some step with `p` holding before it, or `q` never holds and `p` holds forever (section 11).

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

`eventually`/`until` are MODEL-CHECKING-ONLY (D-P4-8/D-P4-9): they parse and validate, but any formula containing them is outside the online monitor subset - production/test codegen in every backend skips it with the identical warning, and only the model checker checks it. Remaining planned operators: `disable iff` and sequence operators. A separate `weak_until` operator is no longer planned: RFS `until` IS weak until, and strong until is expressible as `p until q && eventually q`.

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
// ||, ->, &&, until, !. `&&`/`||` are formula-level operators; each operand is
// parsed independently via primary_formula, whose fallthrough is the NON-BOOLEAN
// expression level (comparisons and below) - so mixed atom kinds like
// `state == S && output_has(O)` parse. `until` sits between `&&` and `!`
// (Phase 4): `!a until b` = `(!a) until b`, `a && b until c` = `a && (b until c)`.
formula         = or_formula ;
or_formula      = impl_formula { "||" impl_formula } ;
impl_formula    = and_formula [ "->" impl_formula ] ;
and_formula     = unary_formula { "&&" unary_formula } ;
unary_formula   = until_formula ;
until_formula   = neg_operand { "until" neg_operand } ;  // left-associative
neg_operand     = "!" neg_operand | primary_formula ;
primary_formula = "next" "(" formula ")"
                | "always" "[" unsigned_integer_literal "]" "(" formula ")"
                | "always" "(" formula ")"
                | "bounded_eventually" "[" unsigned_integer_literal "]" "(" formula ")"
                | "eventually" "(" formula ")"
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

Phase 4 adds two more additive formula variants: `eventually (inner)`
serializes as `{ "kind": "eventually", "inner": {} }` and `p until q` as
`{ "kind": "until", "left": {}, "right": {} }`. Purely additive - no
`IR_VERSION` bump; pre-liveness IR JSON round-trips unchanged (D-P4-8).

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
- keywords: `module`, `enum`, `initial`, `event`, `output`, `var`, `transition`, `on`, `where`, `next`, `emit`, `set`, `prop`, `assume`, `bool`, `bit`, `init`, `true`, `false` (the full RFS v0 keyword set is 28 words - it is maintained as the single source of truth `parser::KEYWORDS` since Phase 5 task 13, shared with the in-repo TextMate grammar under `ide/`; a drift-guard test scans the parser's own call sites bidirectionally, so a new keyword must be added to `KEYWORDS` or the pin fails);
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

One dedicated diagnostic exists beyond generic span-based errors: an `eventually` keyword whose body is not a parenthesized formula can never parse (the top-level `many0(module)` recovery would otherwise swallow the failure into "trailing input after RFS source"), so on parse failure `parse_source` runs a targeted whole-word scan and reports `expected '(' after 'eventually' (the body must be a parenthesized formula)` at the keyword's exact position (Phase 4 task 13; `bounded_eventually` is excluded by the word-boundary check, string literals are skipped).

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

The shipped exact engine reports the three-status taxonomy of section 11 (D-P4-4): `pass-complete` / `fail` (reason: `property-violation`, `liveness-violation`, or `deadlock`) / `unknown-state-limit`, with human lines `PASS (complete)` / `FAIL` / `UNKNOWN (state limit reached)` and exit codes 0/1/2. This avoids presenting bounded runs as full proof.

### 9.8 Production integration style

| Option | Pros | Cons | Fit | Decision |
|---|---|---|---|---|
| Generated standalone pure `step` function | Simple, testable, no runtime coupling | User must wire I/O themselves | High | Core (shipped) |
| Trait-based adapters over the pure FSM | Composable, supports sync/async runtimes | More abstraction layers | High | **Shipped in Phase 5** (D-P5-11: `ProtocolFsm` trait + generic adapters in `crates/rfs-adapters`, wired via the opt-in `--with-protocol-fsm` codegen flag) |
| Runtime-specific generated code, e.g. tokio-only | Convenient for specific stack | Couples DSL toolchain to async runtime | Medium | No (the async adapter works on ANY executor - no tokio dependency) |

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

**As shipped (Phase 5 final state):** the production generator emits one
self-contained `.rs` file with NO dependencies - flattened `State`/`Input`/
`Output` enums, a `Vars` struct, a trace runtime (`Sample`/`TraceStep`/`Trace`),
an assumption runtime (`AssumptionPolicy` + handlers, `StepError`), online
property/assumption monitors rendered from the shared `MonitorPlan` (debug-gated
in production output, D-P3-6), and `Fsm { state, vars, trace, ... }` with
`new()` / `step(input) -> Result<Vec<Output>, StepError>` / `step_with_handler`.
Two Phase 5 extensions: `--no-std` (production only - `#![no_std]` +
`extern crate alloc`, body byte-identical, identical public API; D-P5-9) and
`--with-protocol-fsm` (opt-in `impl ProtocolFsm for Fsm`; the one documented
exception to the no-dependency invariant; D-P5-11). The C backend renders the
same surface with parity-pinned behavior. Full anatomy: `src/backends/RUST.md`
section 2 and `CODEGEN.md` section 5. The original Phase 0/1 sketch below is
kept for history - its module-wrapper shape and `UnexpectedEvent` type were
superseded by the shipped flat-enum + `StepError` design (Phase 1 codegen
decisions, section 19).

Original planned shape (historical):

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

Code generation rules (as shipped, except where noted):

- ~~generated files must include source hash and generator version~~ - superseded: the header is the pinned `// Generated by RFS ...` comment line; determinism comes from codegen being a pure function of IR + options (invariant 2 in `CODEGEN.md` section 9);
- variable updates preserve simultaneous-update semantics using temporary next variables (shipped);
- transition dispatch maps to Rust `match` arms over `(state, input)` with priority-ordered arms and the `on_error` fallback (shipped, Phase 2);
- ambiguous transitions are rejected or resolved by explicit priority before deterministic code generation (shipped - validator errors/warnings);
- debug property monitors are generated behind `#[cfg(debug_assertions)]` in production output (shipped in Phase 3, D-P3-6 - the "feature flag" became a cfg gate; test/mc output always includes them).

---

## 11. Model-checking plan (Phase 4, implemented)

Phase 4 replaced the Phase 1/3 seeded-fuzz model checker with an EXACT
reachable-state engine as the default `mc` mode (D-P4-2/D-P4-7). The legacy
fuzz engine is retained behind `--mode fuzz`. An optional SMT-based bounded
mode (`--mode smt-bounded`) landed in task 11 (D-P4-12).

Architecture (D-P4-1): per-backend GENERATED engines + ONE shared codegen-time
plan module. `src/mc_plan.rs` computes the plan once in the Rust tool - input
abstraction analysis, state-encoding layout, canonical traversal spec,
liveness obligation decomposition - and both backends embed it as constants/
tables in the generated binary, so behavior is identical by construction.
The SMT mode uses the same pattern via `src/smt_plan.rs` + `src/smt_emit.rs`
(both backends render the SAME shared spec into structurally identical z3
constraints).

```text
rfs codegen <rust|c> mc spec.rfs
   |
   v
codegen-time plan (src/mc_plan.rs): abstraction, encoding, traversal, obligations
   |  embedded as constants/tables in BOTH generated binaries
   v
generated self-contained MC binary (per-backend runtime)
   +-- exact level-synchronous BFS engine (default; --mode exact)
   +-- legacy fuzz engine (--mode fuzz)
   +-- SMT k-step unrolling engine (--mode smt-bounded, codegen-time `smt` feature)
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

### Exact reachable-state checking (default mode)

Level-synchronous BFS over configurations = (state index, packed vars, monitor
states) - monitors stay part of the configuration because bounded operators
need history. Visited check at ENQUEUE time; parent pointers (parent config id
+ input index) for counterexample reconstruction. Canonical successor order
(D-P4-2): events in declaration order, payload fields in declaration order,
values ascending (bool false<true, int 0..max, enum declaration order).
Determinism is independent of data-structure internals: Rust and C may use
different containers but must report identical status/counterexample.

Semantics (D-P4-5/D-P4-6): a reachable configuration with NO successor at all
(no enabled transition and no on_error) = FAIL(deadlock). Inputs matching no
transition while other inputs are enabled terminate that path; they are
recorded as `unexpected_terminations` in statistics (+ first counterexample)
but do NOT change the status (RFS treats unexpected events as a normal
Result-based outcome). Active assumptions PRUNE inputs from exploration;
`--assumes none` explores everything.

Input abstraction (D-P4-3): a payload field is abstracted to a single
representative value iff it is not referenced by any transition guard, output
payload expression, variable update expression, or property/assumption formula
(`input_is` patterns with field values count as references). Such a field
cannot influence enabled transitions, outputs, variable evolution, or property
labels - every concrete run has an identical abstract counterpart (the same
argument carries over to liveness; see below).

### Statuses and report format (D-P4-4)

Three statuses with matching exit codes:

| status | human line | exit code | meaning |
|---|---|---|---|
| `pass-complete` | `PASS (complete)` | 0 | full reachable graph explored; all properties hold |
| `fail` | `FAIL` | 1 | counterexample found; reason is `property-violation`, `liveness-violation`, or `deadlock` |
| `unknown-state-limit` | `UNKNOWN (state limit reached)` | 2 | BFS stopped at `--max-states`; no completeness claim |

Every report starts with the machine-readable summary line (parity-checked,
exact format):

```text
model-check summary: status=<s> states=N transitions=M max_depth=D
```

plus, on failure, ` reason=<r> label=<L> step=S`. A statistics block follows:
`unexpected terminations`, conditional `first unexpected`, `monitor size`
(encoded bit width of all property + assumption monitors), `memory estimate`.
FAIL reports then print the property-specific counterexample: property
violation (trigger/violation steps + vars + full I/O trace), deadlock (the
terminal configuration), or liveness (a lasso, below).

### Bounded checking

`--max-states N` (default 1,000,000) caps the BFS. When the cap is hit before
the graph completes, the status is `unknown-state-limit`: the run performs a
TRUNCATED analysis over the explored subgraph - a counterexample found inside
it is sound (all its edges are real transitions), so such runs may still report
`fail`; otherwise no completeness claim is made. A truncated run NEVER reports
pass-complete. `--timeout-sec N` (default 0 = off) is wall-clock only and NOT
parity-guaranteed (the same spec can time out on one machine and not another);
it is never used by parity or benchmark assertions.

### Liveness (Phase 4, implemented)

The property language supports two unbounded liveness operators (D-P4-8):

```rfs
eventually (p)          # p holds at some position of the path
p until q               # WEAK until (semantics below)
```

**Scope (D-P4-9).** A top-level property may combine obligations
`eventually p` / `p until q` with boolean operators (`!`, `&&`, `||`, `->`)
at the top level; obligation operands must be BOOLEAN formulas over atomic
propositions (no temporal operators inside). Deeper nesting produces a
codegen diagnostic (clear message, not a parse error) and the property is
skipped. Any formula containing `eventually`/`until` is outside the monitor
subset: it is checked by the model checker only (MC-only property), with the
identical skip warning as other out-of-subset formulas.

**Semantics.** A top-level property holds in a model iff it holds on every
infinite path from the initial configuration. On a path π, position 0 is the
initial sample and position k ≥ 1 is the sample of step k:

- `eventually p` (F p): true on π iff p holds at some position of π.
- `p until q` — WEAK until: true on π iff
  `[∃k ≥ 0: q(π[k]) ∧ ∀j < k: p(π[j])] ∨ [∀k: p(π[k])]`.
  Strong (standard) until is expressible as `p until q && eventually q`.

**Obligation decomposition.** Each liveness property is decomposed at codegen
time (`src/mc_plan.rs`) into a top-level BOOLEAN SKELETON whose leaves are
obligations or safety subformulas:

- safety leaf s: true on π iff s holds at every position of π;
- `eventually p` leaf: true on π iff p holds at some position of π;
- `p until q` leaf: true on π per the weak-until semantics above.

The property is violated in the model iff some infinite path falsifies the
skeleton (evaluates it to false).

**Algorithm (D-P4-10, as implemented).** The analysis runs on the COMPLETED
reachable graph after exact BFS (or on the explored subgraph when truncated —
see below). Each leaf is tracked by a small deterministic PROGRESS AUTOMATON
whose state is eventually constant on every infinite path:

| leaf | states | transition | satisfied limit states |
|---|---|---|---|
| safety s | ok, bad | ok → bad when s is false (bad absorbing) | ok |
| eventually p | wait, done | wait → done when p holds (done absorbing) | done |
| p until q | ok, sat, bad | ok → sat when q holds; ok → bad when p fails and q has not held yet (sat, bad absorbing) | ok, sat |

(The `until` transition checks q before ¬p: a sample where both hold
satisfies the formula.)

For a property with L leaves, the product of its automata has at most 3^L
states (mixed radix). The check is:

1. **Product reachability.** BFS over (config, product state) pairs from
   (initial config, δ(q₀, label₀)), where label₀ is the position-0 sample and
   each graph edge contributes its step sample's label.
2. **Limit vectors.** A product cycle realizes exactly one limit vector:
   because every automaton transition is one-way, all states on a cycle carry
   the same per-leaf value. For each product state q with a reachable cycle in
   its stay-edge subgraph (edges that keep the product state q), the realized
   limit vector is t_k = "q's k-th component is a satisfied limit state".
3. **Skeleton evaluation.** The property is violated iff some realizable
   limit vector falsifies the skeleton.

This is equivalent to the per-obligation bad-set fixpoints of D-P4-10, stated
over the POSITION graph (positions = samples; position 0 = initial sample,
position k = step-k sample):

- neg(eventually p) = greatest fixpoint of {pos: ¬p(pos) ∧ ∃ successor ∈ B} —
  positions from which an infinite path avoiding p exists. In finite systems
  `G F p` (always eventually p) has the SAME failure set as `F p`: both fail
  exactly when a reachable cycle avoids p.
- neg(p until q), WEAK until = backward reachability, staying in
  {pos: ¬q(pos)}, to the target {pos: ¬p(pos)}.

  **Plan correction (D-P4-10).** The original D-P4-10 text targeted
  T = gfp(¬q ∧ ¬p ∧ ∃ successor ∈ T); that equation is correct for STRONG
  until but diverges from the weak-until semantics of D-P4-8 (verified: it
  disagrees with brute-force path enumeration on 185/1200 random tiny graphs;
  counterexample: a path where p fails once and then recovers, with q never
  holding — weak until is falsified, the literal equation says satisfied).
  The corrected equation above is what the engine implements.

**Edge-based labels.** Position labels are evaluated per SAMPLE: position 0
uses the initial-configuration sample (no input), position k uses the sample
of step k. `input_is`/`output_has` atoms inside obligation bodies therefore
depend on the EDGE taken, not only on the target configuration — the same
configuration reached via different inputs can carry different labels. The
product analysis reads labels per edge; the fixpoint statements above are over
the position graph where this is intrinsic.

**Counterexamples.** A liveness violation is reported as a LASSO: prefix path
+ cycle, printed in the Phase 3 trace format (one cycle iteration). Summary
line fields (D-P4-4): `reason=liveness-violation label=<L> step=S` where S is
the position where the cycle begins. Human-facing fields: the property
formula, the violation step, the cycle length, and a note naming each
falsified leaf ("safety subformula false at step k", "eventually body never
holds on the path", "until left fails at step k without right holding").

**Bounded / truncated mode (D-P4-10).** When BFS stops at `--max-states` or a
timeout, the liveness analysis runs on the explored subgraph only. A witness
found inside it is sound (all its edges are real transitions), so the result
is FAIL; otherwise the UNKNOWN status stands. A truncated run never reports
pass-complete for a liveness-bearing spec.

**Fairness policy (D-P4-11).** Deterministic mode (default): environment
fairness by construction — every input is explored adversarially and there is
no scheduler nondeterminism, so no fairness filter applies. `--nondet` mode:
every enabled transition is a separate successor; liveness is checked under
STRONG TRANSITION FAIRNESS — a bad cycle is a valid counterexample only if
every transition enabled at some state of the cycle is taken on the cycle.
Since a simple cycle takes exactly one outgoing edge per visit, this means
each cycle state must have exactly ONE explored successor (and, in truncated
mode, must have been fully expanded). The filter is implemented by restricting
the stay-edge subgraph to such "closed" configs; any fair bad cycle consists
of closed configs, so the restriction preserves completeness.

**Input abstraction soundness (D-P4-3, liveness).** Abstraction collapses an
input payload field to a single representative value when no transition guard,
output expression, variable update, or property/assumption formula references
it. Such a field cannot influence which transitions are enabled, what outputs
are produced, how variables evolve, or the truth of any property atom — so
each concrete input maps to its representative with identical behavior and
identical per-sample property labels. Liveness results are therefore sound in
both directions: a violation found in the abstract model is witnessed by a
real path (lift the representative inputs to any concrete value), and a
pass-complete result on the abstract model holds for the concrete one because
every concrete path projects to an abstract path with identical labels.

**Complexity.** For a property with L leaves the product has at most 3^L
states; the analysis is O(3^L · (N + E)) time and O(3^L · N) bits for the
reachability sets (N configs, E explored edges), plus O(E · L) label
evaluations. Typical top-level skeletons have 1–3 leaves.

### SMT bounded mode (task 11, optional; D-P4-12)

`--mode smt-bounded` is a third MC mode, feature-gated: the `smt` cargo
feature pulls in the optional z3 dependency and the parity tests; default
builds are unaffected. The generated binary is standalone (no FSM core):
k-step unrolling of the transition system over bitvectors from a SHARED plan
(`src/smt_plan.rs`) rendered by a SHARED emitter (`src/smt_emit.rs`) - both
backends build structurally identical z3 constraints against the SAME libz3
5.3.1 (TOOLCHAIN.md section 5), so results are byte-identical by construction.
Liveness uses k-liveness (never + cycle query); exact-MC shift semantics
(`amount >= width -> 0`) and full-value literal comparisons are encoded
faithfully (zero-extend/guard/truncate + comparison widening). Exit codes:
0 pass-within-bound / 1 fail / 2 unknown; PASS results are always labeled
INCOMPLETE (`PASS WITHIN BOUND k (incomplete - no completeness claim)`).
v1 scope rejections (shared error text in both backends): assumptions, on_error
blocks, enum-typed vars, and property formulas beyond top-level
always/eventually over boolean combinations of expressions. Codegen flag:
`--smt-bound N` (default 10), overridable at runtime.

### Flags and parallelism (D-P4-7; task 10B)

Codegen-time flags are embedded as defaults in the generated binary and
overridable at runtime: `--mode exact|fuzz`, `--max-states N` (default
1,000,000), `--nondet`, `--timeout-sec N` (default 0 = off), `--threads N`
(default 1). Exact mode is level-parallel for `--threads N > 1`: two phases
per level (parallel expansion of disjoint frontier ranges, then a sequential
canonical commit that fixes the next level's order and detects violations), so
results are BYTE-IDENTICAL to `--threads 1` for any N in both backends -
determinism by construction, no RNG/seed anywhere. The C backend mirrors this
with pthreads + a C11 atomic abort flag (inline fallback if thread creation
fails). `--fast` is runtime-only: non-deterministic early abort (workers stop
scheduling new configurations once any finds a violation; the first found
violation is reported) - sound (never a false pass), documented as non-parity.
The default stays N=1 because it is still fastest on this phase's workloads
(benchmark data: largest run ~1 s).

---

## 12. PSL-like property language plan

### v0 parser subset (implemented, Phases 3-4)

```rfs
next (formula)
always (formula)
always[N] (formula)
bounded_eventually[N] (formula)
eventually (formula)      # Phase 4: unbounded liveness, MC-only
p until q                 # Phase 4: WEAK until, MC-only
boolean_formula
```

The bounded subset is implemented and monitored ONLINE in every backend
(Phase 3, section 21): each prop/assume compiles to a per-label monitor from
the shared `MonitorPlan`. Formulas outside the monitor-compilability
restrictions R1-R4 - including ANY formula containing `eventually`/`until`
(D-P4-9) - still parse and validate, but codegen skips them with an identical
warning in every backend; the liveness operators are checked by the model
checker only (section 11).

### Planned full subset

```rfs
disable iff d
```

and possibly sequence operators if needed. (`eventually`/`p until q` landed in
Phase 4; a separate `weak_until` is no longer planned - RFS `until` IS weak
until, and strong until is `p until q && eventually q`.)

### Strong vs weak liveness

| Operator class | Meaning | Model-checking complexity |
|---|---|---|
| Safety, e.g. `always p` | Violation has a finite prefix | Easier, exact with reachable-state search |
| Bounded liveness, e.g. `bounded_eventually[N] p` | Must happen within N steps | Easy for small N, monitor uses countdown |
| Strong liveness, e.g. `eventually p` | Must happen in every fair infinite run | Requires fairness/Büchi/cycle detection |
| Weak liveness / weak until | May allow finite termination or no obligation | Depends on exact semantics; must be documented |

Phase 4 implements `eventually` and WEAK `until` in the exact engine (section
11): per-leaf progress automata + product limit-vector analysis on the
COMPLETED reachable graph, lasso counterexamples, `--nondet` strong transition
fairness. The weak-until semantics are documented and brute-force-verified
(D-P4-8/D-P4-10, incl. the plan correction recorded in section 11).

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

Implemented so far (Phase 5 final state): SIX subcommands - the Phase 1 frontend
CLI (`rfs parse`, all four RFS/JSON format combinations through the IR, optional
`--target-dir`, collision guard; `rfs validate`, file or directory input with
`.rfs-validation.err` reports in text mode and structured `--json` diagnostics for
editor integrations since Phase 5 task 13), plus the Phase 5 commands: `rfs info`
(static spec introspection, text or `--json`; D-P5-4), `rfs test` (generate +
compile + run the selected unit/fuzz suites in one invocation; D-P5-5), and
`rfs diagram` (state diagrams / transition tables as plugins, pure-text primary
behaviour with strict `--render svg`; D-P5-6). The Phase 0 positional parser mode
was removed during Phase 2 preparation. Every command that invokes external tools
is parameterizable through the shared resolver (`RFS_*` env vars preferred,
CLI flags override, bare name on PATH as default; D-P5-7) and accepts
`--print-script` (deterministic POSIX sh emission instead of execution; D-P5-8).
Exit-code convention across all six: 0 success, 1 any error (MC generated
binaries keep their own 0/1/2 status taxonomy). Run instructions live in
`BUILD.md`; CLI behavior rules in `KB.md` section 4.5.

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

**Phase 5 resolution (D-P5-9):** `--no-std` production codegen shipped (task 9:
`#![no_std]` + `extern crate alloc`, identical public API shape; C no-op);
property monitors shipped in Phase 3 (debug-gated, D-P3-6); no logging hook
(the CLI has `--log-level` instead).

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

---

## 22. Phase 4 model-checking decisions (as shipped)

Record of D-P4-1..D-P4-14 as IMPLEMENTED (plan: `docs/roadmap/PHASE_4_PLAN.md`;
corrections below point at the recorded deviations). Full semantics: section 11.

### D-P4-1. Architecture: shared codegen-time plan, per-backend generated engines
As planned. `src/mc_plan.rs` computes the plan once (abstraction, encoding,
traversal spec, obligation decomposition); both backends embed it as
constants/tables, so behavior is identical by construction. The SMT mode reuses
the pattern via `src/smt_plan.rs` + `src/smt_emit.rs`.

### D-P4-2. Exact engine: level-synchronous BFS over configurations
As planned. Configurations = (state index, packed vars, monitor states);
visited at enqueue; parent pointers for counterexamples; canonical successor
order (events/fields in declaration order, values ascending). Determinism is
independent of container choice - Rust and C report identical
status/counterexample (parity suites pin this).

### D-P4-3. Input abstraction
As planned. A payload field is abstracted to a single representative value iff
no transition guard, output payload expression, variable update expression, or
property/assumption formula references it (`input_is` patterns with field
values count as references). Soundness arguments for safety and liveness:
section 11.

### D-P4-4. Statuses and summary line
As planned. `pass-complete` / `fail` (reason: `property-violation` |
`liveness-violation` | `deadlock`) / `unknown-state-limit`; exit codes 0/1/2;
machine line `model-check summary: status=<s> states=N transitions=M
max_depth=D` plus ` reason=<r> label=<L> step=S` on fail; human lines `PASS
(complete)` / `FAIL` / `UNKNOWN (state limit reached)`.

### D-P4-5. Deadlock and unexpected-input semantics
As planned. No-successor configuration = FAIL(deadlock); inputs matching no
transition while others are enabled terminate that path and are counted as
`unexpected_terminations` without changing the status.

### D-P4-6. Assumptions prune exploration
As planned. Active assumptions remove violating inputs from the search;
`--assumes none` explores everything.

### D-P4-7. Modes and flags
As planned, plus task 10B additions (see corrections below): `--mode
exact|fuzz`, `--max-states N` (default 1,000,000), `--nondet`, `--timeout-sec
N` (default 0 = off; documented as wall-clock only, NOT parity-guaranteed),
`--threads N` (default 1; byte-identical results for any N), runtime-only
`--fast` (non-parity early abort). Codegen flag `--smt-bound N` (default 10)
for the SMT mode. Plan deviation recorded in task 10's status: the
`--timeout-sec` help lives in `src/codegen.rs` (generic codegen help) and the
per-backend command descriptions (`rust.rs`/`c.rs`) - NOT in `main.rs` usage
text, which does not document MC flags.

### D-P4-8. Liveness language: `eventually (p)` + infix `until` (WEAK until)
As planned, with the task 5 correction: the `until` level sits between `&&`
and `!` such that `!` binds TIGHTER than `until` (`!a until b` = `(!a) until
b`) and `&&`/`||` bind looser (`a && b until c` = `a && (b until c)`). The
plan's EBNF sketch (operands are primaries) contradicted its own pinned
consequence; the implementation follows the pinned consequence (standard
PSL/CTL), recorded in `PHASE_4_TASK_5.md` "Task corrections". IR: additive
`Formula::Eventually` / `Formula::Until`, no version bump. Weak-until
semantics + strong-until encoding: section 11.

### D-P4-9. Liveness scope
As planned. Top-level obligations `eventually p` / `p until q` where p, q are
BOOLEAN formulas over atomic propositions (no temporal operators inside);
obligations combinable at top level with `&&`/`||`/`!`/`->`. Deeper nesting ->
codegen diagnostic + property skipped (binary still compiles/runs). Any
formula containing eventually/until is outside the monitor subset (identical
skip warning, MC-only).

### D-P4-10. Liveness algorithm: per-leaf progress automata on the completed graph
As implemented WITH THE CORRECTED WEAK-UNTIL EQUATION (plan correction recorded
in section 11): neg(p until q) = backward reachability, staying in {pos: ¬q},
to {pos: ¬p}. Verified against brute-force lasso enumeration (1200 equation +
2000 full-algorithm checks, 0 mismatches). Bounded mode: truncated analysis -
sound FAIL if the witness is inside the explored part, else
`unknown-state-limit`; never a false pass-complete.

### D-P4-11. Fairness policy
As planned (no correction needed). Deterministic mode: environment fairness by
construction, no filter. `--nondet`: strong transition fairness - a bad cycle
counts only if closed (every enabled successor of each cycle state is taken on
the cycle); implemented by restricting stay-edges to fully expanded configs.
Discriminator test-pinned (det FAIL vs nondet pass-complete).

### D-P4-12. SMT bounded mode (optional)
Gate PASSED, IMPLEMENTED in task 11. z3 5.3.1 at `/workspace/host/z3`
(TOOLCHAIN.md section 5); feature-gated `smt` cargo feature (z3 crate 0.21 /
z3-sys 0.13.1) - default builds unaffected. `--mode smt-bounded`: k-step
unrolling over bitvectors from the shared plan/emitter (parity by
construction), k-liveness, exit codes 0/1/2, PASS always labeled incomplete.
v1 scope rejections: assumptions, on_error blocks, enum-typed vars, property
formulas beyond top-level always/eventually over boolean combinations of
expressions. Details: section 11 + `PHASE_4_TASK_11_STATUS.md` (incl. semantic
bugs found/fixed: shift-amount width mismatch, oversized-literal comparison
wrap, z3 5.3.1 C API deltas).

### D-P4-13. Benchmarks
As planned. Three protocol-like specs in `examples/valid`: 43 sliding-window
ARQ (task 6; liveness + abstraction showcase), 44 token ring and 45 elevator
(task 8; sizing 129,985 / 11,098 states). Benchmark suite
`tests/benchmark_mc_test.rs` (task 9/10B): pass-complete metric parity in both
backends, inline FAIL variants with counterexample parity, `--threads`/
`--fast` gates, JSON report (`target/mc_benchmarks.json`, development
artifact).

### D-P4-14. Test migration to seedless exact mode
As planned (task 4). `backend_parity_test.rs` + `monitor_integration_test.rs`
migrated from same-seed fuzz to seedless exact mode; values re-pinned from the
deterministic traversal - including 8 NEW deadlock findings (terminal states
the fuzz walk never reported) and 1 new property violation.

### Plan corrections during execution (pointers)

- **10a** (retroactive entry): task 10's `--threads N` subtask DEFERRED -
task 9 data showed no benchmark needed it (~1.1 s largest run vs ~30 s budget)
and canonical violation selection under parallel expansion looked fragile;
revisit trigger recorded in `PHASE_4_TASK_10_STATUS.md`.
- **10b** (new task): revived the deferred `--threads` with a corrected design
agreed with the user (2026-10-19): determinism from CANONICAL SELECTION with
arbitrary parallel work distribution (two-phase per level) - byte-identical to
`--threads 1` for any N in both backends, no RNG/seed; plus runtime-only
non-parity `--fast`. Rejected alternatives (shared-seed RNG switching,
RNG-driven exploration order) recorded in `PHASE_4_TASK_10B.md`.
- Task-level corrections: task 5 (until precedence - see D-P4-8), task 10
(`--timeout-sec` help location - see D-P4-7).

---

## 23. Phase 5 toolchain decisions (as shipped)

Record of D-P5-1..D-P5-14 as IMPLEMENTED (plan: `docs/roadmap/PHASE_5_PLAN.md`;
corrections below point at the recorded deviations). Full command behavior:
`KB.md` section 4.5; codegen surface: `CODEGEN.md`; backend internals:
`src/backends/RUST.md` / `src/backends/C.md`.

### D-P5-1. Workspace layout (task 1)
The single `rfs` package became a cargo workspace with an EXPLICIT member list
(no globs - `crates/examples` has no manifest): root package `rfs` +
`crates/rfs-build` + `crates/rfs-adapters` + example crates
(`sync_handler`, `async_handler`, `fuzz_rust`). The C example projects
(`c_sync_handler`, `fuzz_c`) are Makefile-driven and NOT cargo members. Plain
`cargo build/test/clippy` at the root still select only the root package;
`--workspace` covers everything. All `.cargo/config.toml` aliases re-verified
unchanged after the conversion.

### D-P5-2. Hand-rolled JSON reader-writer, ALL sources (task 2)
A full hand-rolled JSON READER + WRITER (`src/json_rw.rs`) replaced
serde/serde_json EVERYWHERE: IR serialization/round-trip, all `--json` CLI
outputs (info/validate/MC counterexample), and the generated MC JSON via an
embedded copy of the writer. Rationale (full detail in the module doc):
guarantee KEY ORDERING (declaration order, pinned) and ALWAYS byte-identical
text output across platforms, Rust versions, and dependency updates - external
crates give no such contract; it also keeps the toolchain dependency-light and
consistent with the generated-code constraint. Correction C3: the sketch's
infallible `to_json` became FALLIBLE `Ir::to_json(&self) -> Result<Json, String>`
- unsigned IR fields can hold values up to `u64::MAX` (the parser accepts them),
which the i64 value model cannot represent; failing with a clear error beats
silent clamping. Output proved byte-identical to the pre-migration format on
all 45 valid examples, so no test re-pinning was needed.

### D-P5-3. rfs-build: build.rs helper (task 1)
`crates/rfs-build` is a dependency-free Rust API for build scripts: locates the
toolchain (`RFS_BIN` env var first - empty treated as unset, nonexistent path
an error - then `rfs` on PATH with a printed note), runs `codegen <backend>
production|test`, writes the artifact to `OUT_DIR/<stem>.rs|c`, and emits
`cargo:rerun-if-changed=<spec>`, `cargo:rerun-if-env-changed=RFS_BIN`,
`cargo:rerun-if-changed=<RFS_BIN file>` (when it names an existing file), and
`cargo:include=<artifact>`. Builder options: `.backend()`, `.production()`
(default) / `.test_command()`, `.with_property_monitors(bool)`, `.no_std(bool)`,
`.with_protocol_fsm(bool)`. Consumer pattern (task 11 correction C11-1):
`include!` rejects inner attributes, so build.rs strips the `#!` lines into a
body file in OUT_DIR and lib.rs uses `include!` under a mirroring outer
`#[allow]` - generated code stays out of the source tree (cargo fmt never sees
it).

### D-P5-4. rfs info: static spec introspection (task 3)
New stable command: per module, states, events/outputs with variants + field
types, variables with types + init, transition count, and properties/assumptions
each with monitor-subset status (`monitored` / `skipped: reason`). File input
only (`.rfs`/`.json` by extension); NO model checking. `--json` emits a
machine-readable document (stable key order, ALL keys present - empty arrays and
null instead of omitted fields) for IDE/tooling reuse; `--log-level` filters
stderr progress. Exit 0 valid / 1 parse-validation error.

### D-P5-5. rfs test: generate + compile + run (tasks 7/17)
One invocation per selected suite (`--suite unit|fuzz`, repeatable or
comma-separated, default `unit`): codegen the `test` artifact into a temp dir
(cleaned on every exit path), compile it (`rustc --edition 2021 --test` /
`gcc -Wall`), run it; per-test results on stdout, progress on stderr. The fuzz
suite (task 17) builds + bounded-runs the libFuzzer target from the EMITTED
skeleton extra file: C = `clang -O1 -fsanitize=fuzzer,address` over
`fuzz_target.c` ONLY (C17-1: compiling the production source too is a
multiple-definition link error - the skeleton `#include`s it under its hardcoded
name), bounded run with `ASAN_OPTIONS=detect_leaks=0` merged (C17-2: the
generated C FSM has no free API; ASan reads env only); Rust = a temp cargo-fuzz
scaffold (lib root = generated production source, adapted skeleton,
`[package.metadata] cargo-fuzz = true` (C17-4), `libfuzzer-sys = "=0.4.13"`
pinned to the local registry cache for offline builds, `rust-toolchain.toml`
pinning nightly because cargo-fuzz needs `-Zsanitizer=address`, with a
non-nightly inherited `RUSTUP_TOOLCHAIN` stripped from the subprocess (C17-3))
run via `cargo fuzz build` + `cargo fuzz run fuzz_target -- -max_total_time=N`
(`--fuzz-max-total-time`, default 30; extra runtime flags via `RFS_FUZZ_FLAGS` /
`--fuzz-flags`). Missing clang/cargo-fuzz is an error ONLY when the fuzz suite
is selected - the default unit path stays tool-free. Exit 0 = every selected
suite passes (fuzzer exits 0 within budget); 1 = test failure, fuzzer crash,
compile error, missing tool for a selected suite, or usage/parse/validation
error.
**Task 16 close-out correction to C17-2:** the generated C core now carries an
explicit `fsm_free` lifecycle end and the fuzz skeleton calls it once per input
(single-exit decode) - per-input allocations no longer accumulate, so long runs
stay within libFuzzer's RSS limit (regression: spec 01 OOM'd at ~200k runs
before the fix; bounded runs now pass with millions of iterations). The
`detect_leaks=0` merge stays pinned as defense in depth.

### D-P5-6. rfs diagram: plugin registry, strict rendering (task 8)
State diagrams + tables as plugins mirroring the codegen backend pattern (`src/
diagram.rs` + `src/diagrams/`): v1 formats `dot` (Graphviz), `puml` (PlantUML
state diagram - edge label `event \n guard`, emitted data via `note on link`),
`mmd` (Mermaid stateDiagram-v2 - all info in multi-line edge labels), plus
`table`/`md` transition tables + variable/event summaries. PRIMARY behaviour is
pure text generation with NO tool calls. `--render svg` (requires `--out
FILE.svg`; source written next to it) invokes the resolved renderer (`dot -Tsvg`
/ `plantuml -tsvg` / `mmdc` + auto temp puppeteer config for root). STRICT error
semantics (user decision 2026-10-21): missing/failing render tool = exit 1 +
diagnostic incl. captured tool stderr, NO graceful degradation - RFS runs in
CI/CD pipelines, a command either does what was requested or throws; users
compose their own fallbacks (e.g. via `--print-script`). Runs on the first
module of the spec.

### D-P5-7. External-tool parameterization (task 3, extended task 17)
Every CLI command that invokes external tools/compilers/build systems is
parameterizable by tool PATHS and additional tool PARAMETERS. Environment
variables are the PREFERRED mechanism - `RFS_RUSTC`/`RFS_GCC`/`RFS_DOT`/
`RFS_PLANTUML`/`RFS_MERMAID` (task 3) + `RFS_CLANG`/`RFS_CARGO_FUZZ` (task 17)
for paths and the matching `*_ARGS` vars (space-separated extra parameters);
rfs-build uses `RFS_BIN`. Per-command CLI flags (`--rustc PATH`,
`--rustc-args "..."`, ... incl. task 17's `--clang/--cargo-fuzz` pair, decision
D17-1) override the env vars; without either, the bare tool name is looked up
on PATH. One shared resolver + invocation helper (`src/ext_tool.rs`) implements
the precedence (flag > env var > PATH) and renders/executes command lines
identically for all commands; task 17 extended `ExtTool` with a fixed
`subcommand` field (cargo-fuzz = resolved cargo path + `fuzz`, rendered before
extra args).

### D-P5-8. --print-script: script-generation mode (task 3)
Commands that invoke external tools accept `--print-script`: instead of
executing, they print a deterministic POSIX sh script to stdout (shebang,
`set -e`, header comment with rfs version + spec + RESOLVED tool paths/args,
one commented step per invocation) and exit 0. Self-contained: generated
sources are embedded in quoted heredocs (`rfs test`: the full test artifact;
`rfs diagram --render svg`: the diagram source, plus a temp puppeteer-config
step for mermaid). Output is unit-test-pinned; the scripts were manually
executed with `sh` during tasks 7/8/17 (exit 0 + self-cleanup).

### D-P5-9. no_std production codegen (task 9)
`--no-std` applies to the PRODUCTION command only (`test`/`mc` reject it with
the pinned `NO_STD_PRODUCTION_ONLY_ERROR` at CLI and library level - they need
std's allocation-heavy machinery). Generated Rust becomes `#![no_std]` +
`extern crate alloc` with an IDENTICAL public API shape (Vec/String/ToString
from alloc; body byte-identical to the std variant - pinned by a 45-example x
2-modes compile sweep + body-parity check). The C backend accepts the flag as a
documented no-op (plain C is already dependency-free) adding one header comment
line.

### D-P5-10. MC counterexample JSON (task 10)
`--counterexample-format json|text` on `codegen <backend> mc` (and at runtime
on the generated binary); default `text` - the parity-pinned human-readable
report is byte-for-byte unchanged. `json` emits ONE machine-readable document
mirroring the text report: status, metrics (states/transitions/max_depth),
reason/label/step (fail only), statistics block, and the counterexample as
structured data (`trace` / lasso with prefix+cycle+cycle_length /
deadlock-config). Exact mode only - fuzz and smt-bounded keep their own output.
Parity extends to byte-identical JSON across backends for the same spec/mode
(pinned over ALL property-bearing examples + a golden full-document pin). The
generated binary carries an EMBEDDED copy of the hand-rolled writer (D-P5-2)
- generated code stays dependency-free, and key ordering is part of the
byte-parity contract.

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
emits `impl ProtocolFsm for Fsm` behind `--with-protocol-fsm` (OFF by default -
every byte of default output unchanged; the one documented exception to
invariant 1 in `CODEGEN.md` section 9), placed after the inherent `impl Fsm`,
before `#[cfg(test)]` in test artifacts, forwarding explicitly so it cannot
recurse. `mc` rejects the flag (pinned `PROTOCOL_FSM_MC_ERROR`, CLI + library
level, both backends); the C backend accepts and ignores it byte-identically.
Rust-ecosystem-only by design per the parity principle (no C trait exists).
The crate is `#![no_std]` with `std` (default) / `async` features; futures 0.3
is optional behind `async` - the only external dependency of the workspace
beyond the root package, never leaking into the toolchain or generated code.

### D-P5-12. Extended test generation: one plan, both backends (tasks 4/5/6)
The shared planner (`src/codegen_test.rs`) was extended with language-agnostic
plans for per-transition unit tests (representative inputs + statically computed
guard taken/not-taken cases; sibling-priority/shadowing rules), invariant tests
(safety property atoms re-checked at every driven step), property-monitor debug
tests (injected violating trace must fire the monitor - planner guarantee:
no higher-firing sibling preempts, task 5 correction C5-1), seeded random-trace
tests (deterministic LCG, fixed seed 12345, K steps via `--steps`, one
byte-identical coverage line), and fuzz-target skeleton planning (per-variant
decode strategy). ONE plan rendered by BOTH backends with identical coverage and
stable test names - the parity principle made executable (the cross-backend
suite compares per-test result lists + the random-trace line). Fuzz skeletons
are SEPARATE extra files (`fuzz_target.rs`/`fuzz_target.c`) so the main artifact
(and the aggregate generated-code crate) stay dependency-free; the CLI writes
them next to `--out` and refuses stdout mode (strict - no silent file loss).

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

### D-P5-14. Example-crate matrix (tasks 11/12)
Rust: `sync_handler` (spec-43 sliding-window ARQ over std mpsc channels via the
channel adapter) and `async_handler` (the same FSM as a futures Stream) - both
generate via rfs-build with `--with-protocol-fsm`, carry their session as a
test + demo binary, and observe identical pinned behavior (19 ticks, in-order
[0..8] frames/acks, 5 dropped out-of-band acks, max window exactly 4). C mirror:
`c_sync_handler` (Makefile-driven plain-C scripted peer; `gcc -Wall`
zero-warning bar; scenario data lines byte-identical to the Rust side). Fuzzing
harness examples for BOTH ecosystems wiring the emitted skeletons: `fuzz_rust`
(cargo-fuzz layout, detached `fuzz/` workspace) and `fuzz_c` (clang + libFuzzer;
generation works without clang, building needs it).

### Plan corrections during execution (pointers)

- **C1** (phase-wide): `PHASE_5.md`'s `--state-limit`/`--time-limit` were NOT
introduced - Phase 4 shipped `--max-states`/`--timeout-sec` (parity-pinned in
tests, docs, training data, and the generated binaries' wire format). The
shipped names stay canonical; no aliases.
- **C2** (phase-wide): `PHASE_5.md`'s `--features` option was dropped - its
intent is covered by the explicit `--no-std` / `--with-property-monitors`
flags; rfs-build exposes cargo feature passthrough separately.
- **C3** (task 2): fallible `Ir::to_json` for u64-range values - recorded in
D-P5-2.
- **C4** (new task 17, user request 2026-10-22): the `rfs test --suite fuzz`
extension with both libFuzzer pipelines + suite selection + env-var
parameterization; see `PHASE_5_TASK_17.md` and D-P5-5.
- Task-level corrections: C5-1 (task 5, monitor-violation planner guarantee -
in D-P5-12), C9/C10 series (tasks 9/10, in their status files), C11-1..C11-4
(task 11, incl. the include! consumer-pattern fix - in D-P5-3/D-P5-11),
C12-1..C12-3 (task 12), C13-1..C13-3 + D13-1 (task 13 - in D-P5-13),
C17-1..C17-5 + D17-1 (task 17 - in D-P5-5/D-P5-7).


## 24. Phase 6 production-hardening decisions (as shipped)

Record of D-P6-1..D-P6-11 as IMPLEMENTED (plan: `docs/roadmap/PHASE_6_PLAN.md`;
its "Plan corrections" table records the deviations found during execution -
2a/4b/5a/5b/6a/6b/6c - and the 2026-10-25 user rule change is recorded in
`PHASE_6_TASK_7.md` Task corrections). Normative semantics: `docs/SEMANTICS.md`
(task 1); external user documentation: `docs/user/` (task 7); compatibility
policy: `docs/user/MIGRATION.md` (task 2.4).

### D-P6-1. The v1.0 syntax version header is the EXISTING `version "..."` mechanism, formalized (task 2.1)
The parser already accepted an optional module-level `version "<string>"`
header (stored in `Module.version`, round-tripped by the printer, shown by
`rfs info`). Decision: formalize THAT mechanism as the v1.0 syntax version
header instead of adding PHASE_6.md's illustrative `# rfs 1.0` comment syntax -
two parallel mechanisms would be ambiguous about which is authoritative, and the
existing one is IR-stored and visible to every tool. Shipped: a VALIDATOR format
check `^\d+\.\d+(\.\d+)?$` (a malformed header is a SEMANTIC error with the
pinned message `module '<name>' invalid version header '<v>' (expected X.Y or
X.Y.Z)`; the parser keeps accepting any quoted string so old files still parse);
specs without a header are legacy v0.x and remain valid. New examples:
`46_version_header.rfs` (valid, carries `version "1.0"`) +
`invalid_bad_version_format.rfs`.

### D-P6-2. IR schema version bump 0 -> 1 with a backward-readable policy (task 2.2)
`IR_VERSION = 1` for v1.0. `from_json` accepts `ir_version` in `0..=IR_VERSION`
(current + all previous minor versions keep loading - the documented evolution
rule, MIGRATION.md section 2.3); a HIGHER version is rejected with the pinned
error `unsupported IR version {n} at $.ir_version (this toolchain reads
0..={IR_VERSION})`. `to_json` writes 1. Rationale: the versioned IR is the
stability contract between toolchain releases; schema evolution must stay
backward-readable (additive optional fields), and v0 documents keep working.

### D-P6-3. Spec hash + toolchain version in every generated artifact header (task 2.3)
Every generated artifact (production/test/mc, BOTH backends, plus fuzz
skeletons) starts with the unified line-1 comment
`// Generated by RFS v{toolchain_version} | spec sha256:{hash16} | module {name}`
(C: `/* ... */`). The hash is SHA-256 over the CANONICAL IR JSON of the WHOLE
input file (deterministic key order per D-P5-2), first 16 hex chars; computed in
`src/codegen.rs` from `Ir::to_json()` and passed to both backends. The SHA-256 is
HAND-ROLLED (`src/spec_hash.rs`, minimal implementation with NIST test vectors) -
no new dependencies, consistent with the D-P5-2 hand-rolled-JSON stance. Crate
version bumped 0.1.0 -> 1.0.0 (the `--print-script` header picks it up via
`CARGO_PKG_VERSION`). Stable contract = the three FIELDS are present and correct;
the line's wording may move in a MINOR bump (MIGRATION.md section 2.5). Users can
now detect spec/toolchain drift in checked-in artifacts by regeneration + diff.

### D-P6-4. Toolchain fuzzing is TWO layers: fast libFuzzer targets + slow seeded CI suite (tasks 5.1/5.2)
Fast layer: a root-level DETACHED cargo-fuzz workspace `fuzz/` (package
`rfs-fuzz`, pattern of `crates/examples/fuzz_rust`) with four in-process
libFuzzer targets - `parser` (no panic on malformed-but-parseable input; on
success validate + IR round-trip), `validator` (random IR via a specgen-style
Arbitrary builder), `ir_roundtrip` (to_json/from_json/to_json byte-stable),
`codegen` (3 commands x 2 backends generate without error + structural
invariants: task-2 header present, balanced delimiters, no placeholder tokens).
Works OFFLINE (`libfuzzer-sys 0.4.13` + `arbitrary 1.5.0` cached); Rust-side runs
need nightly (TOOLCHAIN.md section 7.5). Slow layer: `tests/toolchain_fuzz_test.rs`
in plain `cargo test` - pinned seeds x small specgen specs -> all six artifacts
generated + compiled (rustc / gcc -Wall) + MC runs in exact & fuzz mode vs
`ref_mc` ground truth (status agreement, NO false `PASS (complete)`, exit codes
{0,1,2}, stable stderr). Rationale: libFuzzer sessions are long-running and stay
manual/nightly; the seeded suite is the CI-runnable layer. Random valid specs come
from `src/specgen.rs` via a shared wrapper - raw IR types are deliberately NOT
`Arbitrary` (validity by construction). The new targets found two real bugs,
recorded as plan corrections 5a (planner binding-name keying - codegen FAIL on a
fully valid spec) and 5b (C MC fuzz-mode OOB read at trigger step 0).

### D-P6-5. The brute-force reference MC is an independent public lib module (task 3.2)
`src/ref_mc.rs`: naive exhaustive reachable-state exploration implementing
`docs/SEMANTICS.md` directly with DELIBERATELY different data structures from the
generated engines (Vec frontier, plain HashMap configuration keys - NO bit-
packing) and its own small property evaluator over recorded traces (different
algorithm from `src/monitor.rs`). Shipped surface: status (pass/fail/deadlock),
first-violation label + step, total state count; PLUS the public APIs added for
the differential suites - `step_trace` (drive a fixed input sequence) and
`verify_counterexample` (replay a generated engine's JSON counterexample against
ref_mc's independent transition relation / monitor compilers / liveness product).
Report rendering is UNIFIED with the generated engines per D-P6-10 (same status
taxonomy, text format, and `src/codegen_mc.rs` JSON document model). A seeded
byte-parity sweep harness (`tests/ref_mc_sweep_test.rs`, fail-fast + parallel)
plus a non-fail-fast diagnostic companion (`tests/ref_mc_diag_test.rs`) verify
ref_mc against the generated Rust MC on all eligible examples + pinned specgen
seeds. Rationale: an independent implementation is what makes the differential
results meaningful - shared code would share bugs.

### D-P6-6. Production-FSM cross-backend parity via thin drivers + identical seeded scripts (task 4.3)
`tests/prod_parity_test.rs` (+ shared `tests/common/prod_driver.rs`): for EVERY
valid example the production FSM is generated for both backends (property
monitors ON), wrapped in a thin driver (Rust: `include!`-based main template; C:
`#include` + main reading an input script from stdin - the scenario-parity
pattern from `example_projects_test`), and both are driven by IDENTICAL seeded
LCG input scripts (seed 43210, 200 steps). Asserted: byte-identical stdout +
exit code. Driver contract: ASSUME/PROP stop lines echo the recorded sample
(recorded but not applied - `commit_step` traces before the assumption/property
check). Rationale: PHASE_6.md's "production FSMs driven by identical random input
sequences" - the production path had no cross-backend differential coverage
before this task.

### D-P6-7. Monitor references are HAND-WRITTEN independent Rust + C in the test suite (task 4.4)
`tests/monitor_ref_test.rs`: for benchmark specs 43/44/45, reference monitors
written directly in the test file - an independent Rust engine AND independent C
compiled per spec with `gcc -Wall` - deliberately NOT reusing `src/monitor.rs`
(toolchain-side, could share bugs with the generated monitors). Driven over random
traces (seed 90717) AND crafted boundary scenarios (bit-width wraparound,
obligation deadlines exactly at trigger+bound-1, re-trigger after clear, sticky
failure, same-sample declaration-order tie-breaks); verdicts + first-violation
sample index compared against the generated monitor output in both backends.

### D-P6-8. PSL translation checks: independent in-repo evaluator + documented external hook (task 4.5)
`tests/psl_translation_test.rs`: an independent second property evaluator -
direct recursive WINDOW evaluation per `docs/SEMANTICS.md` (own Formula/Expr
descent, own bit-vector semantics, obligation scanning by window search rather
than countdown automaton) cross-checked against generated monitor verdicts over
random traces for ALL property-bearing examples. External-model-checker hook:
the `RFS_PSL_REF_TOOL` env var names a tool that receives the trace + formula and
must agree; availability-gated (no external PSL checker exists in this
environment - documented; z3 is SMT-only, not a trace checker). Rationale:
PHASE_6.md's "where possible" - satisfied by the independent in-repo reference
plus a documented plug-in point.

### D-P6-9. "Benchmark suite runs in CI" = standard pipeline + documented Actions wiring (task 6.3)
No `.github/workflows` exists in this environment, so the exit criterion is met
by: (1) the benchmark MC suite being part of plain `cargo test` (one command:
`cargo test --test benchmark_mc_test`; writes `target/mc_benchmarks.json` + prints
per-backend tables) and (2) GitHub Actions wiring DOCUMENTED as a snippet in
BUILD.md section 7 so any external CI can run it. Wall-clock/timing/RSS values
stay INFORMATIONAL (never asserted except the order-of-magnitude RSS bound) -
CI machines vary; Phase 4 discipline against flaky gates.

### D-P6-10. `rfs mc-ref`: the reference MC exposed as a CLI with UNIFIED flags and reports (tasks 3.2/3.3)
User request (2026-10-23 plan review): expose the brute-force reference MC for
rapid prototyping in shell/REPL without compiling a generated MC. Shipped:
`rfs mc-ref <spec.rfs|spec.json>` (`src/mc_ref_cmd.rs`) - in-process exact
exhaustive search; FLAGS UNIFIED with the generated exact-mode binary where
applicable: `--max-states N` (default 1000000), `--timeout-sec N` (default 0 =
off), `--assumes all|none|labels`, `--counterexample-format json|text` (default
text), `--log-level`. NOT applicable (documented in the usage text): `--seed` /
`--steps` / `--violate` (fuzz-mode concepts - exact mode is seedless per D-P4-7),
`--threads` / `--fast` (single-threaded by construction), `--nondet` (ref_mc
explores ALL enabled transitions by construction). Exit codes MIRROR the
generated binaries: 0 pass-complete, 1 fail, 2 unknown-state-limit. The report
format (text + JSON) is identical to the generated engines' so outputs are
directly comparable - byte-for-byte where deterministic (the sweep harness and
`tests/ref_mc_diff_test.rs` assert exactly this; `peak_rss_kb` excepted - each
generated binary self-reports it, ref_mc reports 0).

### D-P6-11. `rfs gen-spec`: random spec generation with incremental base mutation (tasks 3.1/3.4)
User request (2026-10-23 plan review): expose the spec generator too, EXTENDED
with incremental generation from a base spec - mutating a known base "shape"
gives much better corner-case coverage than fully random specs. Shipped:
`rfs gen-spec` (`src/gen_spec_cmd.rs`) with two modes - fully random
(`--seed N`, default pinned 12345, + `--size small|medium` or explicit caps
`--max-states/--max-events/--max-vars/--max-transitions/--max-props`) and
incremental mutation (`--base SPEC.rfs|SPEC.json --mutations K [--seed N]`,
applying K seeded mutation steps from the 17-op `specgen::mutate` API). The
output is ALWAYS a valid spec: it is parsed + validated before writing (error +
exit 1 otherwise); invalid mutation candidates are rolled back and redrawn at
every step. Deterministic from the seed: same args => byte-identical file. The
generator lives in `src/specgen.rs` shared with the fuzz targets and the slow
toolchain-fuzz suite.

### Phase 6 execution notes (drift from plan, as shipped)
- Plan corrections 2a/4b/5a/5b/6a/6b/6c (see `PHASE_6_PLAN.md`): a C expression-
  lowering UB fix + `50_wide_ops.rfs` (2a), the Rust exact-MC extra-BFS-level fix
  (4b), the planner binding-name fix (5a), the C MC trigger-step-0 OOB fix (5b),
  the `__rfsb` pattern-binding shadowing mangling in both backends (6a), the
  exact `expect_progress_from_initial` planner computation (6b), and the spec-48
  `Error on Setup` arm removal (6c). Final example counts: 50 valid / 13 invalid.
- The 2026-10-25 user rule change (recorded in `PHASE_6_TASK_7.md` Task
  corrections + plan correction row 7): mermaid diagrams in markdown code blocks
  are allowed and used extensively in `docs/user/`; the page-size limit is 3000
  lines; document quality is machine-checked by `tests/docs_md_test.rs`, which
  validates every repo `.md` with mdr's parser stack (dev-deps `comrak 0.55` +
  `mermaid-rs-renderer 0.3` - the exact versions mdr 0.6.1 renders with; mdr
  itself is binary-only and cannot be a library dependency).
