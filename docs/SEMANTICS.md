# RFS Formal Semantics

Normative semantics of the RFS v1 language surface as implemented by the
toolchain. This document is the reference that Phase 6 tasks implement and test
against: the brute-force reference model checker (`src/ref_mc.rs`, task 3.2) and
the independent evaluators (tasks 4.4/4.5) are coded directly from it, so any
document/implementation mismatch surfaces as a test failure.

Where the implementation is ambiguous, this document states the IMPLEMENTED
behavior and flags it in a footnote. Source pointers at the end of sections are
for verification, not reading - the document is self-contained.

---

## 1. Notation and scope

**Scope.** The semantics apply to ONE RFS module. A source file may contain
several modules; every codegen command (`production`, `test`, `mc` of both
backends) runs on the FIRST module of the file, and multi-module files are
otherwise rejected by validation when items conflict across modules.

**Machine.** An RFS module describes a finite input/output transition system

```text
M = (Q, Sigma, V, q0, delta, Lambda)
```

- `Q` - the finite set of control states (variants of the `State` enum);
- `Sigma` - the finite set of input events (flattened constructors with finite
  payload fields);
- `V` - the finite set of variable valuations;
- `q0 in Q` - the initial state;
- `delta` - the guarded transition relation (section 3);
- `Lambda` - the finite set of output actions.

**Notation.**

| Symbol | Meaning |
|---|---|
| `c = (q, v, m)` | a configuration: control state `q`, variable valuation `v`, monitor transition state `m` (section 9.2) |
| `pi = c0 -> c1 -> ...` | a run (path); `ck` is reached by step `k` from `c(k-1)` |
| sample `s` | the per-position record the property layer evaluates (section 4); position 0 of `pi` is sample 0, position `k >= 1` is the sample of step `k` |
| `v[x]` / `v'` | value of variable `x` in valuation `v`; valuation after a step |

**Value convention.** A `bit<N>` value is an unsigned N-bit vector; every
arithmetic result is taken mod 2^N. The Rust backend computes in a u128
intermediate and masks; the C backend relies on fixed-width wrapping plus a
mask - identical results for all defined cases, with one flagged latent
divergence in generated C (footnote [^c-ub] in section 2.2). Booleans are
`false < true`. Enum values order by declaration where an order is needed
(section 9.4).

---

## 2. Automaton model

### 2.1 Module structure

A module contains, in any order: enum declarations, one `initial` state,
event groups, output groups, variable declarations, transitions, `prop`
formulas, `assume` formulas, and at most one `on_error` block. The module header
may carry an optional version string:

```rfs
module name version "1.0.0" { ... }
```

The version is a quoted string (no escapes, no whitespace) stored in the IR
(`Module.version`) and emitted as `SPEC_VERSION` (Rust) / `#define
RFS_SPEC_VERSION` (C). It is METADATA: it has no effect on any semantics
below. The parser accepts any such string; format validation of the v1.0
header (`^\d+\.\d+(\.\d+)?$`) is added by Phase 6 task 2 (decision D-P6-1) -
until then, specs without a header are legacy v0.x and behave identically.

### 2.2 Types and values

| Type | Values |
|---|---|
| `bool` | `false`, `true` |
| `bit<N>` | unsigned integers `0 ..= 2^N - 1` |
| `EnumName` | the enum's declared variants, in declaration order |

The expression language is pure and total. Operators and their semantics (all
results of a bitvector operation are reduced mod 2^w of the result width):

| Operator | Semantics |
|---|---|
| `+`, `-`, `*` | wrapping arithmetic mod 2^w |
| `<<`, `>>` | logical shift; if the shift amount `>= w` the result is `0`; `>>` never sign-extends (values are unsigned) [^c-ub] |
| `~`, unary `-` | bitwise complement / two's complement within width w (`-x = 0 - x mod 2^w`) |
| `&`, `\|`, `^` | bitwise; on `bool` operands `&`/`\|` act as logical connectives |
| `==`, `!=` | equality over the full value (enums by variant identity) |
| `<`, `<=`, `>`, `>=` | unsigned comparison of the full values |
| `!` | boolean negation |
| `&&`, `\|\|` | boolean connectives, short-circuit in generated code; truth-functional for model checking |

There is NO division. Typing rules: arithmetic and bitwise operators require
both operands to have the same bitvector width (an integer literal adapts to
the other operand's width; two bare literals form a `bit<8>`); comparisons
require identical types or a literal against a vector; shifts require a
bitvector left operand and a bitvector-or-literal right operand (no width
equality required); logical operators require boolean operands. Guard, formula
expression atoms, and assignment values must type-check as shown above.

[^c-ub]: IMPLEMENTED BEHAVIOR, flagged divergence (found in the Phase 6 task 1
    cross-check pass). The RUST backend implements the table exactly: shifts
    are guarded (`amount >= width -> 0`), arithmetic uses u128 intermediates
    with wrapping ops + mask. The GENERATED C code instead renders `((l) op
    (r)) & mask` relying on C integer promotion, which is well-defined only
    while the intermediate stays in range: (a) a shift by a VARIABLE amount
    >= the promoted left-operand width (>= 32 for `bit<8..32>` left operands,
    >= 64 for `bit<64>`) is undefined behavior in C - Rust defines it as `0`;
    (b) multiplying two `bit<16>` values whose product exceeds INT_MAX is
    signed overflow (undefined behavior) in C. No current valid example or
    test exercises either case (all variable shift amounts are `bit<2>`; no
    example uses `*`), so all existing parity pins hold - but the divergence
    is reachable from a spec such as `set w = w << k;` with `k: bit<8>`. Fix
    tracked as plan correction / new task 2a (C expression lowering to
    uint64_t-wide operands + guarded shifts, plus cross-backend differential
    pins). Until then, specs relying on large variable shift amounts or wide
    multiplications have UNSPECIFIED C behavior.

### 2.3 Events and outputs

Events and outputs are declared in groups; each group has named constructor
variants with finite payload fields of the types above:

```rfs
event Recv(Syn)
event Recv(Ack(ack_seq: bit<4>))
output Reply(Ok(code: bit<8>))
```

For code generation and model checking, groups are FLATTENED into one variant
list (inputs and outputs separately, declaration order): a single-variant group
whose variant name equals the group name keeps the bare name (`event Tick` ->
input `Tick`); otherwise the base name is `group_variant`; colliding names get
numeric suffixes (`Name`, `Name2`, ...). Every input alphabet element used by
the model checker and every production `Input::`/`Output::` enum variant is one
flattened constructor.

### 2.4 Variables and the initial configuration

```rfs
var seq: bit<4> init 0
var mode: Mode init Mode::Normal
var enabled: bool init false
```

Each variable has a type (`bool`, `bit<N>`, enum) and an initializer expression.
The initial configuration `c0` is `(q0, v0, m0)` where `v0[x]` is the value of
x's initializer and `m0` is the monitors' initial state (section 9.2).

Initializer expressions are typed against the FULL module environment (all
variables visible, no pattern bindings). In practice they are constants or enum
paths; an initializer that references another variable does not type-error but
generates code that cannot compile, so such specs are ill-formed in effect.
[^init-order]

[^init-order]: IMPLEMENTED BEHAVIOR, flagged ambiguity: the validator types each
`init` expression with every module variable in scope (declaration order is
ignored), and codegen renders initializers as a struct literal over constants -
there is no defined cross-variable evaluation order. Specs should use constant
initializers; this document treats `v0` as well-defined exactly for such specs.

---

## 3. Transition relation

### 3.1 Transition structure and pattern matching

```rfs
transition S on EventPattern where Guard {
  next T;
  emit OutputExpr;      # zero or more
  set var = expr;       # zero or more, distinct vars
  priority N;           # optional
}
```

- `S`, `T` are `State` variants; exactly one `next` is required.
- The event pattern selects a variant of an event group and constrains its
  payload fields. Field patterns: `_` (wildcard), `name` (bind the field to
  `name`), an integer/boolean literal, or `Enum::Variant`; a named pattern
  match `name: <pattern>` binds while constraining. A BARE group pattern
  (`on Recv`) matches every variant of the group. Anonymous enum-variant field
  patterns are NOT supported (`on E(Mode::Normal)` is rejected; the named form
  `on E(x: Mode::Normal)` is supported).
- The guard is a boolean expression, optional (absent guard = `true`).
- A transition matches an input iff its pattern matches AND its guard evaluates
  to true.

### 3.2 Guard and action evaluation environment

Guards, emit arguments, and assignment values are evaluated in an environment
containing: the CURRENT control state (`state`), the OLD variable values,
and the pattern bindings of the matched input. All right-hand sides read OLD
values - see 3.3.

### 3.3 Actions and simultaneous updates

When a transition fires on input `e` from configuration `(q, v)`, it produces
next state `T`, outputs (the emitted output expressions, in declaration order),
and a new valuation `v'`. Variable updates are SIMULTANEOUS: every assignment
right-hand side is evaluated against the OLD valuation `v` (generated code uses
temporary locals in both backends), then all assignments are applied together.
There is no sequential dependency between assignments in one transition, and a
transition may not assign the same variable twice (parse error).

The step relation is:

```text
(q, e, v) -->* (T, v', O)
```

where `O` is the output list; `*` marks that several transitions may be enabled
at once (resolved per mode, section 3.5).

### 3.4 Priorities and firing order

Within one state, transitions are ordered by EFFECTIVE priority: `priority N`
descending, default 0 for unmarked transitions; ties keep declaration order
(stable sort). This is the FIRING ORDER of the generated FSM and of the model
checker's successor enumeration. Cross-state ordering is irrelevant (only the
active state can match).

Validator constraints on overlapping transitions (same state):

| Situation | Severity |
|---|---|
| identical pattern + identical guard (both absent or structurally equal) | error: duplicate transition (the second is unreachable) |
| identical pattern + two different explicit priorities | allowed (deterministic order pinned) |
| identical pattern + duplicate explicit priority value | error: duplicate priority |
| identical pattern, no priorities on either, guards differing or one missing | warning: the pair "share[s] an event pattern with different guards and no priorities; first match wins in source order" - pin intent with distinct `priority N;` |

Priorities need only be unique per (state, pattern); numbers may be reused
across different patterns in the same state. Guards are compared structurally -
overlapping guards like `x < 5` / `x >= 3` produce the warning even if some
inputs are disjoint.

### 3.5 Deterministic vs nondeterministic modes

- **Deterministic mode (production, default MC):** for each input, at most one
  transition fires - the FIRST matching transition in firing order. Generated
  production code is a `match`/if-chain in exactly this order, so production
  behavior is deterministic by construction. A spec is "deterministic" in the
  strong sense (at most one enabled transition per input) when no two
  transitions of the same state with overlapping patterns can both be enabled;
  priorities make the OBSERVED behavior deterministic even when several are.
- **Nondeterministic MC mode (`--nondet`):** every enabled transition is a
  separate successor of the configuration (all matches, in firing order). This
  explores scheduler/environment choices that deterministic mode hides; it is
  used to check properties against ALL possible resolutions of overlapping
  guards. See section 6.8 for the fairness policy that applies here.

### 3.6 `on_error` fallback

```rfs
on_error {
  next Safe;
  emit Alert(Code(9));   # optional emits
}
```

At most one per module; `next` is mandatory. When NO transition matches an
input in the current state, `on_error` executes INSTEAD of reporting an
unexpected event: it moves to its declared state and emits its outputs. It has
no pattern bindings and cannot assign variables - all variable values carry
over unchanged. It fires from ANY state. In model checking it is just a final
fallback arm (section 9.5); without `on_error`, an unmatched input is an
unexpected event (section 8).

### 3.7 `Tick` and timers

There is NO special-casing of `Tick` in the semantics: `Tick` is an ordinary
unit event variant, enumerated by the model checker like any other input, and
matched by production code like any other input. Timeouts are modeled by
declaring a timer variable and decrementing it (or resetting it) on `Tick`
transitions; "time passing" in a model-checking run is exactly the sequence of
`Tick` inputs the environment chooses.

---

## 4. Property sampling

A run is observed as a SEQUENCE OF SAMPLES. Position 0 is the initial
configuration:

```text
sample 0:  state = q0, vars = v0, input = NONE, outputs = []
```

For each step `k >= 1`, sample `k` records the transition just taken:

```text
sample k:  input      = the input consumed by step k
           outputs    = the outputs emitted by step k (declaration order)
           from_state = state before the step
           to_state   = state after the step
           vars       = the POST-step variable valuation
```

Consequences, which every operator below relies on:

- Atomic propositions (expression atoms) are evaluated at POST-STEP values:
  `state` is the target state of the step, variables are the new values.
- `input_is(...)` / `output_has(...)` refer to the TRANSITION JUST TAKEN. At
  sample 0 there is no input and no output, so both atoms are FALSE there.
- Monitors are stepped at sample 0 (when the machine is constructed) and after
  every successful step; a property can therefore already be violated at
  sample 0 (e.g. an invariant on the initial state).
- The same configuration reached via different inputs carries DIFFERENT labels
  for `input_is`/`output_has` atoms - labels are edge-based, not purely
  configuration-based (section 6.5).

---

## 5. PSL operator semantics

Formulas are built from atoms with boolean connectives and temporal operators.
This section defines the MONITORABLE subset: its meaning over a (finite or
infinite) sample sequence, as evaluated by the online monitors that every
backend generates from the shared `MonitorPlan`. The unbounded liveness
operators (`eventually`, `until`) are NOT monitorable; their path semantics are
given in section 6.

### 5.1 Atoms (per-sample signals)

| Atom | True at sample s iff |
|---|---|
| `<boolean expression>` | the expression evaluates to true on sample s's post-step values (`state` = target state, `vars` = new valuation; at s=0 the initial configuration) |
| `input_is(EventPattern)` | the input consumed at sample s matches the pattern (false at s=0). Pattern matching is as in 3.1; wildcards and bindings constrain nothing |
| `output_has(OutputPattern)` | AT LEAST ONE of the outputs emitted at sample s matches the pattern (false at s=0) |

### 5.2 Boolean connectives

`! f`, `f && g`, `f || g`, `f -> g` are truth-functional over the per-sample
values of their operands, with `f -> g` = `(!f) || g`. Formula-level `&&`/`||`
are distinct from the expression ladder: an operand that is not a keyword atom
or parenthesized formula falls through to the NON-BOOLEAN expression level.
Known precedence corner case (implemented): `a || b -> c` parses as
`a || (b -> c)`; parenthesize implications for clarity.

Composition rule for compound monitors: a conjunction fails when ANY child is
failing at the current sample; a disjunction fails only while ALL children are
failing AT THE SAME SAMPLE. Children may fail and recover on different samples,
so per-sample (not sticky) child status is what composes - `p || q` is NOT
violated by `p` failing at sample 0 and `q` failing at sample 1.

### 5.3 `next`

`next (f)` requires the signal-level formula `f` to hold one sample later:

- top-level `next (f)`: `f` must hold at every sample s >= 1; sample 0 is
  vacuous (a trace with only the initial sample never violates it).
- `p -> next q` (p, q signals): for every check sample s >= 1, `p(s-1) ->
  q(s)` - a one-sample delay register holds the antecedent; the register starts
  false, so no check fires before the first antecedent evaluation. Under a
  temporal operator with anchor range R (e.g. `always[N]`), checks occur at
  samples in R+1.
- `next` requires a temporal-operator-free body (restriction R4).

### 5.4 `always` and `always[N]`

- `always (f)`: `f` must hold at EVERY sample from 0 onward (unbounded range).
- `always[N] (f)` with N >= 1: `f` must hold at samples `0 ..= N-1`; no check
  occurs after sample N-1. `always[0]` is a validation error.
- `always[N]` is allowed only at top level or under boolean connectives - never
  under another temporal operator (restriction R3).

### 5.5 `bounded_eventually[N]`

`bounded_eventually[N] (f)` is an OBLIGATION with a countdown, where `f` must
be temporal-operator-free:

- **Top level** (not under a temporal operator): the obligation is pending from
  sample 0: `f` must hold at some sample j in `[0, N-1]`; if it never does, the
  violation is CONFIRMED at sample N-1 (the deadline). For N = 0 the body must
  hold at sample 0 itself (violation confirmed at sample 0).
- **As the consequent of an implication** `p -> bounded_eventually[N] (f)`
  (N >= 1): for every anchor sample i in the enclosing range where `p(i)`
  holds, `f` must hold at some j in `[i, i+N-1]`; the violation is confirmed at
  sample i+N-1 if the body never held. A re-trigger while an obligation is
  pending is IGNORED (the pending obligation has the earliest deadline; the
  monitor stops at the first confirmed violation). At TOP LEVEL (no enclosing
temporal operator) the anchor range is `{0}` only: the implication is
ONE-SHOT - `p(0)` starts the countdown and `p(i)` for i > 0 never triggers it.
- **Inside `always`/`always[N]` bodies:** same trigger model, with anchors in
  that operator's range (all samples for `always`, samples `0..=N-1` for
  `always[N]`) - this is the reactive "whenever p, then f within N" form.
- The obligation must not be negated (R1) and may not appear under another
  `bounded_eventually` or under `next` (R2).

**Obligation stepping algorithm** (the exact model both backends render; sample
s, trigger signal a, body signal b, bound N >= 1):

```text
a = in_range(s) && eval(trigger at s)
b = eval(body at s)
if not pending and a:      pending = true; remaining = N; trigger_step = s
if pending:
    if b:                  pending = false
    else:                  remaining -= 1
                           if remaining == 0: VIOLATION confirmed at sample s
```

Worked example (N = 3, trigger at i, body never holds): s=i remaining 3->2,
s=i+1 remaining 2->1, s=i+2 remaining 1->0 - violation confirmed at i+2; if the
body holds at any of i..i+2 the obligation clears.

**Monitor-compilability restrictions (R1-R4).** Formulas outside R1-R4 still
parse and validate, but codegen in EVERY backend emits the identical warning
(`property '<name>' is outside the monitor subset; skipped` / `assume '<name>'
is outside the monitor subset; skipped`) and generates no monitor for that
label:

- **R1:** no `bounded_eventually` under negation (directly or indirectly).
- **R2:** a `bounded_eventually[N] b` node requires `b` temporal-operator-free
  and may appear (a) at top level, or (b) inside an `always`/`always[N]` body -
  except under another `bounded_eventually` and not under `next`; its trigger
  is the implication antecedent when it is directly the consequent of `->`,
  else the constant `true`.
- **R3:** `always[N]` only at top level or under boolean connectives - never
  under `next`, `always`, `always[N]`, or `bounded_eventually`.
- **R4:** `next` allowed anywhere safety is allowed, except inside a
  bounded-eventually body (see R2).

---

## 6. Liveness: strong vs weak, fairness

### 6.1 Operators and path semantics

Two unbounded liveness operators exist; both are MODEL-CHECKING-ONLY (any
formula containing them is outside the monitor subset - identical skip warning
in production/test codegen of every backend):

```rfs
eventually (p)          # p holds at some position of the path
p until q               # WEAK until
```

On a (finite or infinite) path pi, position 0 is the initial sample and
position k >= 1 is the sample of step k:

- `eventually (p)` (strong liveness, F p): true on pi iff p holds at SOME
  position of pi.
- `p until q` - WEAK until: true on pi iff
  `[exists k >= 0: q(pi[k]) and for all j < k: p(pi[j])]`
  OR `[for all k: p(pi[k])]` (q never holds but p holds forever).
  A sample where both p and q hold satisfies the formula (q is checked before
  not-p). STRONG until is expressible as `p until q && eventually q`; a
  separate `weak_until` operator does not exist - RFS `until` IS weak until.

**Precedence** (implemented): `!` binds TIGHTER than `until` (`!a until b` =
`(!a) until b`); `&&`/`||`/`->` bind looser (`a && b until c` = `a && (b
until c)`); compound operands need parentheses: `(a && b) until c`.

### 6.2 Scope rules

A top-level property may combine obligations `eventually p` / `p until q` with
boolean operators (`!`, `&&`, `||`, `->`) at the top level; obligation operands
must be BOOLEAN formulas over atomic propositions (no temporal operators
inside). Deeper nesting of liveness under a temporal operator, or temporal
operators inside a liveness property, produces a CODEGEN diagnostic (clear
message, never a parse error) and the property is SKIPPED - the binary still
compiles and runs without that property.

### 6.3 Model-level validity and skeleton decomposition

A top-level liveness property HOLDS in the model iff it holds on EVERY infinite
path from the initial configuration (under the fairness policy of section 6.8).
At codegen time each such property is decomposed into a top-level BOOLEAN
SKELETON whose leaves are:

- **safety leaf s:** true on pi iff s holds at every position of pi;
- **`eventually p` leaf:** true on pi iff p holds at some position of pi;
- **`p until q` leaf:** true on pi per the weak-until semantics above.

The property is violated in the model iff some (fair) infinite path falsifies
the skeleton.

### 6.4 Algorithm: per-leaf progress automata

The analysis runs on the COMPLETED reachable graph (section 9.5), or on the
explored subgraph when truncated (section 6.7). Each leaf is tracked by a small
deterministic PROGRESS AUTOMATON whose state is eventually constant on every
infinite path:

| leaf | states | transition | satisfied limit states |
|---|---|---|---|
| safety s | ok, bad | ok -> bad when s is false (bad absorbing) | ok |
| eventually p | wait, done | wait -> done when p holds (done absorbing) | done |
| p until q | ok, sat, bad | ok -> sat when q holds; ok -> bad when p fails and q has not held yet (sat, bad absorbing) | ok, sat |

For a property with L leaves the product of its automata has at most 3^L
states. The check: (1) BFS over (configuration, product state) pairs from the
initial pair; (2) every product cycle realizes exactly one LIMIT VECTOR (all
automaton transitions are one-way, so all states on a cycle carry the same
per-leaf value); (3) the property is violated iff some realizable limit vector
falsifies the skeleton.

This is equivalent to the fixpoint characterizations over the POSITION graph
(positions = samples): `neg(eventually p)` = greatest fixpoint of `{pos: not
p(pos) and exists successor in B}` (in finite systems, `always eventually p`
and `eventually p` fail on exactly the same paths: a reachable cycle avoiding
p); `neg(p until q)` for WEAK until = backward reachability, staying in
`{pos: not q(pos)}`, to the target `{pos: not p(pos)}`.

### 6.5 Edge-based labels

Position labels are evaluated per SAMPLE: position 0 uses the initial-
configuration sample (no input), position k uses the sample of step k. Therefore
`input_is`/`output_has` atoms inside obligation bodies depend on the EDGE
taken, not only on the target configuration - the same configuration reached
via different inputs can carry different labels. The product analysis reads
labels per edge; in the fixpoint statements this is intrinsic to the position
graph.

### 6.6 Counterexamples

A liveness violation is reported as a LASSO: prefix path + cycle, printed with
one cycle iteration (text report) or as `prefix`/`cycle`/`cycle_length`
(JSON). Summary line fields: `reason=liveness-violation label=<L> step=S`
where S is the position where the CYCLE BEGINS. Human-facing fields: the
property formula, the violation step, the cycle length, and a note naming each
falsified leaf ("safety subformula false at step k", "eventually body never
holds on the path", "until left fails at step k without right holding").

### 6.7 Truncated mode soundness

When BFS stops at `--max-states` or a timeout, the liveness analysis runs on
the explored subgraph only. A witness found inside it is SOUND (all its edges
are real transitions), so the result is FAIL; otherwise the UNKNOWN status
stands. A truncated run NEVER reports pass-complete for a liveness-bearing spec.

### 6.8 Fairness

- **Deterministic mode (default):** environment fairness by construction -
  every input is explored adversarially and there is no scheduler
  nondeterminism, so NO fairness filter applies; every reachable cycle is a
  valid counterexample candidate.
- **`--nondet` mode:** strong TRANSITION FAIRNESS. A bad cycle is a valid
  counterexample only if EVERY transition enabled at some state of the cycle is
taken on the cycle. Since a simple cycle takes exactly one outgoing edge per
visit, each cycle state must have exactly ONE explored successor (and, in
truncated mode, must have been fully expanded). The filter restricts the
stay-edge subgraph to such "closed" configurations; any fair bad cycle consists
of closed configurations, so the restriction preserves completeness.

---

## 7. Assumptions and handler policies

### 7.1 Language and role

Assumptions use the SAME property language as properties:

```rfs
assume name : formula ;
```

Properties describe SYSTEM obligations; assumptions describe ENVIRONMENT
guarantees. An assumption is a monitor-subset formula (R1-R4 apply identically).

### 7.2 Online enforcement and handler policies

Assumption monitors are ALWAYS ACTIVE in generated code (never gated by build
mode, unlike property monitors which are debug-gated in production output). A
confirmed assumption violation at a step is reported through the ASSUMPTION
HANDLER path with the violated label; the handler returns a policy:

| Policy | Effect |
|---|---|
| `Continue` | the step proceeds as if nothing happened (violation recorded only in statistics/traces) |
| `Stop` | the step returns an assumption-violation error (`StepError::AssumptionViolation { label }` / C `RFS_ASSUMPTION_VIOLATION`); state is NOT updated |
| `Panic` | the program panics/aborts with the label |

The plain production entry point `Fsm::step(input)` uses the PANIC handler; the
delegate `Fsm::step_with_handler(input, handler)` lets callers choose. The C
backend mirrors this with a function-pointer handler and CONTINUE/STOP/PANIC
policies (default handler aborts).

### 7.3 Sample-0 behavior

The machine steps its assumption monitors at sample 0 during construction.
If an ACTIVE assumption is already violated at the initial configuration, the
constructor PANICS (production and model-checker binaries alike) - assumptions
must hold at the initial configuration by design.

### 7.4 Selection

`--assumes all|none|labels` selects which assumptions are active (default
`all`). `labels` is a comma-separated list filtered against the module's known
assumption labels; UNKNOWN LABELS ARE SILENTLY DROPPED. Only selected
assumptions contribute monitor state to MC configurations (section 9.2) and
only they prune or panic.

### 7.5 Interaction with model-checking search

In exact MC, an assumption violation on a SUCCESSOR prunes that successor:
the input is removed from exploration from that configuration (other inputs
from the same configuration are still explored). Pruning is checked BEFORE
property violations: a successor that violates both an active assumption and a
property is pruned, not reported as a property violation. `--assumes none`
explores everything.

### 7.6 Out-of-subset assumptions

An assumption outside the monitor subset (R1-R4, or containing
`eventually`/`until`) generates NO monitor and is skipped with the identical
warning - it is NOT enforced by production code or by the model checker.
[^assume-skip]

[^assume-skip]: IMPLEMENTED BEHAVIOR, flagged gap: unbounded liveness
assumptions are documented but not checked anywhere; only monitor-subset
assumptions (including bounded liveness) are enforced. Users needing an
`eventually`-style environment guarantee must express it as a bounded form or
accept that it is advisory.

---

## 8. Error and deadlock semantics

### 8.1 Unexpected events in production code

When no transition matches the input AND the module has no `on_error` block,
the step FAILS with an unexpected-event error WITHOUT updating state: Rust
returns `Err(StepError::UnexpectedEvent)`, C returns status
`RFS_UNEXPECTED_EVENT`. The caller decides recovery (this is the Result-based
handling; the FSM itself stays in its current configuration).
With an `on_error` block, the fallback of section 3.6 executes instead.

### 8.2 Property violations in production code

When property monitors are enabled (always in test/mc output; in PRODUCTION
output only under debug builds - release builds pay zero monitor cost), a
confirmed property violation at a step:

- records a STICKY violation `{ label, step, trigger_step }` that is never
  cleared (`trigger_step` = the sample that started the failing obligation,
  absent for plain per-sample failures);
- makes the step return `Err(StepError::PropertyViolation { label, step })`
  WITHOUT applying the transition (state/vars unchanged).

### 8.3 Unexpected inputs in model checking

In exact MC, an input that matches no transition and has no `on_error`
TERMINATES THAT PATH: the successor is not enqueued, the event is counted in
the `unexpected terminations` statistic (with the first occurrence recorded as
a counterexample), and the STATUS IS UNCHANGED - RFS treats unexpected events
as a normal Result-based outcome of the production semantics, not as a model
failure. A configuration whose inputs are ALL unexpected-or-pruned is a
deadlock (next subsection).

### 8.4 Deadlock - precise definition

A reachable configuration c is DEADLOCKED iff it has NO SUCCESSOR at all:
for EVERY input in the canonical input enumeration (section 9.4), either no
transition matches and there is no `on_error` fallback, or every enabled
successor of that input is pruned by an active assumption monitor. Notes:

- `Tick` participates like any other input in the enumeration. A state that is
  merely MISSING a transition for some input (e.g. no Tick transition) does NOT
  deadlock: that input's path terminates as an unexpected termination
  (section 8.3) while the other inputs still have successors. Deadlock requires
  that NO input at all has a surviving successor - e.g. a terminal state with
  no outgoing transitions.
- With an `on_error` block, unmatched inputs always have a fallback successor,
  so deadlock can still occur ONLY via assumption pruning of every fallback
  (and every real) successor.
- Deadlock is a FAILURE: status `fail`, `reason=deadlock`, `label=-`,
  `step=` the depth of the dead configuration; the counterexample is the
  terminal configuration itself (depth, state, vars).

---

## 9. Model checking

### 9.1 Architecture

The model checker is GENERATED per backend from ONE shared codegen-time plan
(`src/mc_plan.rs`): input abstraction analysis, configuration encoding layout,
canonical traversal spec, and liveness obligation decomposition are computed
once in the Rust tool and embedded as constants/tables in BOTH generated
binaries, so behavior is identical by construction. The generated MC binary
embeds the SAME production FSM core that ships (MC = production core +
runtime), so checked behavior equals shipped behavior.

### 9.2 Configuration encoding

A configuration is the tuple `(state index, packed variables, monitor
transition states)`, bit-packed little-endian into a fixed byte buffer that the
visited set hashes:

1. **State index:** `ceil_log2(#states)` bits (0 when there is at most one
   state).
2. **Variables,** declaration order: `bool` = 1 bit, `bit<N>` = N bits, enum =
   `ceil_log2(#variants)` bits.
3. **Monitor blocks:** properties in declaration order, then SELECTED
   assumptions in declaration order. Only monitor nodes with persistent state
   contribute bits: an obligation node with bound >= 1 contributes 1 bit
   (pending) + `ceil_log2(bound+1)` bits (remaining counter); a delayed-check
   (`next`) node contributes 1 bit (antecedent register). Per-sample checks and
   boolean composition carry no persistent state. The sticky `failed`/
   `trigger_step` flags are deliberately NOT part of the configuration: every
   enqueued configuration has no confirmed failure yet (a safety failure
   terminates the search before enqueue), so those flags are invariant on all
   live configurations.

### 9.3 Abstraction rules (representative inputs)

**The representative-input rule.** For each event variant, each payload field
is either CONCRETE or ABSTRACTED:

- A field is **concrete** iff it is REFERENCED by at least one of:
  (a) a transition guard, emit argument, or assignment value - via a pattern
      BINDING whose bound name is actually USED in that expression (binding a
      field without using it does NOT reference it);
  (b) an `input_is` pattern with CONSTRAINING field patterns (integer/boolean
      literals or enum-variant matches) anywhere in any property OR assumption
      formula, including under temporal operators and inside liveness bodies.
      Wildcards and binds in `input_is` constrain nothing and do not count.
- An **abstracted** field is collapsed to a single REPRESENTATIVE value:
  `bool` -> `false`, `bit<N>` -> `0`, enum -> the FIRST declared variant.

Concrete fields are enumerated over their FULL canonical domain (section 9.4);
abstracted fields take only their representative. The unit-test generator and
the fuzz-mode input list use ONE representative input per variant (all fields
representative).

**Soundness.** An abstracted field cannot influence which transitions are
enabled, what outputs are produced, how variables evolve, or the truth of any
property atom - so every concrete run has an identical ABSTRACT counterpart
(same enabled transitions, same outputs, same variable evolution, same
per-sample property labels). Liveness results are sound in both directions: a
violation found in the abstract model is witnessed by a real path (lift the
representative inputs to any concrete values), and a pass-complete result on
the abstract model holds for the concrete one (every concrete path projects to
an abstract path with identical labels).

### 9.4 Canonical enumeration and successor order

Determinism is defined by CANONICAL ORDER, independent of data-structure
internals (Rust and C may use different containers but must report identical
status/counterexample):

- **Inputs:** event variants in DECLARATION order; within a variant, payload
  fields in declaration order; values ASCENDING - bool `false < true`;
  `bit<N>` `0 ..= 2^N-1`; enum in declaration order.
- **Transitions:** per state, firing order (section 3.4: effective priority
  descending, declaration order for ties). In deterministic mode the first
  matching transition fires; in `--nondet` mode all matches are successors, in
  this order. The `on_error` fallback is a final additional effect appended
  only when no real transition matched (it is never a sibling of a matched
  transition for the same input).

### 9.5 Exact reachable-state exploration (default mode)

Level-synchronous BFS over configurations, starting from c0 (whose sample-0
monitors have already been stepped; a sample-0 property violation is reported
immediately as FAIL with step 0):

1. Expand the current level: for each configuration, enumerate all inputs in
   canonical order and, per input, the enabled effects (deterministic: first
   only; nondet: all), in firing order.
2. For each effect, step the monitors at sample index = depth + 1:
   - assumption monitor confirms a violation -> PRUNED (not enqueued);
   - property monitor confirms a violation -> FAIL(property-violation): the
     search STOPS and the counterexample is the path to this successor;
   - otherwise the successor configuration is produced.
3. Visited check at ENQUEUE time on the packed configuration bytes; parent
   pointers (parent config id + input index + transition id) support
   counterexample reconstruction; graph edges are recorded for every success
   (including edges to already-visited configurations) for the liveness
   analysis.
4. A configuration with NO surviving successor (all inputs unexpected or
   pruned) -> FAIL(deadlock) (section 8.4).
5. After BFS completes, run the liveness analysis of section 6.4 on the
   completed graph.

Counters: `states` = configurations enqueued (including c0); `transitions` =
enabled effects processed (success + pruned + violation; unexpected inputs do
NOT count); `max_depth` = greatest depth reached.

### 9.6 Bounded / fuzz mode (`--mode fuzz`, legacy)

The seeded random engine drives the production FSM with RANDOM representative
input sequences. Deterministic RNG: a 64-bit LCG with constants
6364136223846793005 (multiplier) and 1442695040888963407 (increment); doubles
are the top-53 bits scaled to [0,1). SAME SEED + SAME CONFIGURATION =>
IDENTICAL RUN in both backends.

Flags: `--bound N` (steps per run, default 5), `--runs N` (default 20),
`--seed N` (default 12345), `--assumes all|none|labels`, `--violate
label:prob:percent%` (repeatable/comma-separated; missing fields default to
prob=1.0, percent=100.0), `--unexpected-prob P` (default 0.0).

At EVERY step the engine attempts phases in this ORDER (first success wins):

1. **Forced assumption violation:** for each active directive (active when a
   per-run draw with its `prob` succeeds), and at each step with probability
   `percent%`, scan the representative inputs in canonical order for one whose
   trial step (with a stop handler) confirms a violation of that label -
   assumption OR property labels - and inject it. A forced input that stops on
   an assumption violation ends the run as `assumption-violated:<label>`; a
   forced input that confirms a property violation ends the checker with the
   counterexample trace (exit 1).
2. **Unexpected injection:** with probability `unexpected-prob`, pick a UNIFORM
   random representative input from ALL inputs and inject it; if it is
   unexpected, the run ends as `unexpected-event`.
3. **Allowed random input:** pick uniformly at random among the representative
   inputs that are currently ALLOWED - allowed = a trial step with a stop
   handler returns Ok OR confirms a property violation (inputs that confirm
   property violations are allowed ON PURPOSE: the checker wants to find them).
   If NO input is allowed, the run ends as `stuck`. Injecting an allowed input
   can still end the run: unexpected -> `unexpected-event`; assumption stop ->
   `assumption-violated:<label>`.

After each injected step, a confirmed property violation terminates the whole
checker with the trace (exit 1). Run statuses aggregate into the summary line:
`model-check summary: runs=R bound=B completed=C unexpected=U
assumption_stops=A property_violations=P` (the last field is always 0 - a
violation exits immediately; it stays for format stability).

### 9.7 Status taxonomy and labeling rules (exact mode)

| status | human line | exit code | meaning |
|---|---|---|---|
| `pass-complete` | `PASS (complete)` | 0 | the FULL reachable graph was explored AND every property (safety via monitors, liveness via the section-6 analysis) holds on it |
| `fail` | `FAIL` | 1 | a counterexample exists; `reason` is `property-violation`, `liveness-violation`, or `deadlock` |
| `unknown-state-limit` | `UNKNOWN (state limit reached)` | 2 | the search stopped at `--max-states` (or timeout) before completion; NO completeness claim |

Every report starts with the machine-readable SUMMARY LINE (parity-checked,
exact format):

```text
model-check summary: status=<s> states=N transitions=M max_depth=D
```

plus, on failure, ` reason=<r> label=<L> step=S` (deadlock uses `label=-` and
the dead configuration's depth as `step`). A statistics block follows:
`unexpected terminations`, conditional `first unexpected`, `monitor size`
(encoded bit width of all property + assumption monitors), `memory estimate`.
FAIL reports then print the property-specific counterexample: property
violation (trigger/violation steps + vars + full I/O trace), deadlock (the
terminal configuration), or liveness (a lasso, section 6.6). With
`--counterexample-format json` the same data is emitted as ONE compact JSON
document (fixed key order; byte-identical across backends).

**LABELING RULES (the "no false PASS" invariant):**

1. `pass-complete` is reported ONLY when the reachable graph was FULLY explored
   (state cap not hit, no timeout) and all safety monitors stayed clean AND the
   liveness analysis found no falsifying limit vector on the completed graph.
2. `unknown-state-limit` is NEVER a completeness claim: a truncated run may
   still report `fail` (a counterexample inside the explored subgraph is sound
   - all its edges are real transitions), but it can NEVER report
   `pass-complete`.
3. `fail` always carries a concrete counterexample of the stated reason; the
   reported violation step/label is the FIRST confirmed one in canonical
   traversal order (deterministic, parity-pinned).
4. `--timeout-sec` (default 0 = off) is wall-clock only and NOT
   parity-guaranteed; it never affects which result a given machine would
   otherwise report.

### 9.8 Determinism and parallelism

Exact mode contains NO randomness: the same spec + flags give byte-identical
reports in both backends. `--threads N > 1` runs a two-phase per-level expansion
(parallel expansion of disjoint frontier ranges, then a SEQUENTIAL canonical
commit that fixes the next level's order and detects violations), so results
are BYTE-IDENTICAL to `--threads 1` for any N. `--fast` is runtime-only and
NON-PARITY: workers stop scheduling once any finds a violation and the first
found one is reported - sound (never a false pass) but run-dependent.

### 9.9 SMT bounded mode (feature-gated, outside the v1 guarantee)

`--mode smt-bounded` (codegen `smt` feature + `--smt-bound N`, default 10;
runtime `--bound N`) performs k-step unrolling of the transition system over
bitvectors with a shared z3 spec rendered identically by both backends. Exit
codes 0/1/2; PASS results are ALWAYS labeled INCOMPLETE (`PASS WITHIN BOUND k
(incomplete - no completeness claim)`). v1 scope rejections (shared error text
in both backends): assumptions, `on_error` blocks, enum-typed variables, and
property formulas beyond top-level `always`/`eventually` over boolean
combinations of expressions. This mode is a bug-hunting extension; the v1.0
completeness guarantees above apply to exact mode only.

---

## 10. Cross-references

**Normative sources this document was verified against (implementation ground
truth):** `src/mc_plan.rs` (abstraction analysis, configuration encoding,
canonical traversal spec, liveness decomposition), `src/monitor.rs` (MonitorPlan
compiler + reference interpreter - the executable spec of section 5),
`src/codegen_mc.rs` (status taxonomy, summary line, forced-violation wire
format, phase strategy, JSON report model), `src/backends/rust_fsm.rs` /
`c_fsm.rs` (transition lowering, simultaneous updates, on_error, step/commit,
monitor rendering), `src/backends/rust_mc.rs` / `c_mc.rs` (exact engine,
liveness engine, fuzz runtime), `src/validator.rs` + `src/parser.rs` (accepted
surface: priorities, duplicates, patterns, typing).

**Design documents:** `DESIGN.md` sections 2-4 (architecture, core semantics,
MC constraints), 11 (model-checking plan incl. liveness algorithm and the
corrected weak-until equation), 12 (property language plan), 20 (Phase 2
decisions D1-D10: priorities, on_error, version header, C type mapping), 21
(Phase 3 monitor decisions D-P3-1..6 incl. R1-R4 and deadline semantics), 22
(Phase 4 MC decisions D-P4-1..14), 23 (Phase 5 toolchain decisions D-P5-1..14).
`KB.md` section 2.2 (core semantics), 2.6 (property language decisions), 3
(RFS v0 syntax reference - the surface this document applies to).

**User documentation (Phase 6 task 7):** `docs/user/LANGUAGE.md`,
`docs/user/PROPERTIES.md`, `docs/user/MODEL_CHECKING.md` derive their semantics
from this document; `docs/user/FAQ.md` covers abstraction and assumptions.

**Executable references:** the brute-force reference MC (`src/ref_mc.rs`,
Phase 6 task 3.2) implements sections 2-9 directly with deliberately different
data structures; the differential suites (Phase 6 tasks 4.x) cross-check it
against the generated engines.