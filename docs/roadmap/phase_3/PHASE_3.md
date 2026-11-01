# Phase 3: PSL-like properties and monitors

## Goal

Add the requested PSL-like temporal property language in a monitor-friendly form.
All codegen backends (Rust and C, and any later backend) generate functionally
identical monitors with exactly the same behavior - per the backend parity
principle in `INTRO.md`.

## Scope

- PSL-like property subset: `not`, `&&`, `||`, `->`, `next`, `always`, `always[N]`, `bounded_eventually[N]`;
- atoms: `state == S`, `state != S`, `var_expr`, `input_is(EventPattern)`, `output_has(OutputPattern)`;
- assumptions: `assume name : formula ;`;
- monitor codegen: each property compiles to a per-backend monitor (Rust AND C) via a shared, language-agnostic restricted PSL-to-monitor compiler - identical functionality and behavior in every backend;
- model checker integration in every backend: monitors in the product state, assumption monitors prune environment branches;
- counterexample enhancement with property-specific information, identical across backends;
- optional production debug monitors (`--with-property-monitors`) in every backend, with per-language gating (Rust: `#[cfg(debug_assertions)]`; C: compile-time define) and the same zero-cost-when-disabled semantics;
- cross-backend parity testing: differential tests run Rust and C artifacts on identical specs/seeds and require identical results.

## Property subset

Support:

```rfs
not p
p && q
p || q
p -> q
next p
always p
always[N] p
bounded_eventually[N] p
```

Atoms:

```rfs
state == S
state != S
var_expr
input_is(EventPattern)
output_has(OutputPattern)
```

Assumptions:

```rfs
assume name : formula ;
```

## Monitor code generation

A shared, language-agnostic PSL-to-monitor compiler turns each property into a
monitor plan; every backend renders that plan in its own language. All backends
must produce monitors with identical semantics: same state after each sample,
violation detected at the same sample, same counterexample information. The
conceptual example below is Rust-flavored for readability; the C rendering must
behave exactly the same way.

Example conceptual monitor for:

```rfs
always (input_is(Recv(Fin)) -> bounded_eventually[3] output_has(Send(Ack)))
```

Monitor state:

```rust
struct Monitor {
    pending: bool,
    remaining: u8,
}
```

Step logic:

```text
if input_is(Recv(Fin)):
    pending = true
    remaining = 3

if pending:
    if output_has(Send(Ack)):
        pending = false
    else if remaining == 0:
        violation
    else:
        remaining -= 1
```

For general formulas, the tool should generate monitors from a restricted PSL-to-monitor compiler.

## Model checker integration

In every backend, the model-checking product state includes:

```text
system_state + system_vars + property_monitor_states + assumption_monitor_states
```

Assumption monitors prune invalid environment branches. Property monitors detect violations.

## Counterexample enhancement

Counterexamples in every backend should show:

- which property failed;
- which step triggered the obligation;
- when the deadline expired;
- relevant variable values;
- input/output sequence.

## Production debug monitors

Optional feature, available for every backend:

```bash
rfs codegen rust production protocol.rfs --with-property-monitors
rfs codegen c production protocol.rfs --with-property-monitors
```

Generated code steps monitors only in debug/test builds - per-language gating,
same semantics (zero cost when disabled):

```rust
// Rust: gated by cfg(debug_assertions)
#[cfg(debug_assertions)]
self.property_monitors.step(sample)?;
```

```c
/* C: gated by a compile-time define, e.g. gcc -DRFS_PROPERTY_MONITORS */
#ifdef RFS_PROPERTY_MONITORS
    monitors_step(&f->monitors, &sample);
#endif
```

This allows runtime detection of environment or implementation violations during testing/fuzzing.

## Deliverables

- Parser/validator support for the property subset and assumptions (extends `src/parser.rs`, `src/ir.rs`, `src/validator.rs`).
- Shared language-agnostic PSL-to-monitor compiler plus per-backend monitor codegen in ALL backends (Rust and C) with identical behavior.
- MC product-state integration in all backends: assumption pruning, property violation detection, enhanced counterexamples (`src/codegen_mc.rs` + generated MC runtimes).
- CLI option `--with-property-monitors` for production codegen in all backends (debug-mode monitors, per-language gating).
- Cross-backend differential tests comparing Rust and C monitor/MC behavior on identical specs/seeds.
- Valid/invalid examples exercising properties, updated tests, and training-set update.

## Exit criteria

- PSL-like properties parse and validate;
- monitors are generated in ALL backends (Rust and C) with identical behavior - no skip warnings for in-subset formulas in any backend;
- cross-backend differential tests pass: identical specs/seeds give Rust and C the same MC results, violation labels, and counterexample information;
- model checker can verify safety and bounded liveness properties in all backends;
- assumptions restrict input sequences correctly in all backends;
- counterexamples include property-specific information in all backends;
- optional runtime property monitors compile and work in debug mode in all backends.

## Non-goals

- strong/unbounded liveness (`eventually p`, `p until q`) - Phase 4;
- exact reachable-state search engine beyond the current bounded runtime - Phase 4;
- symbolic/SMT bounded mode - Phase 4 (optional);
- full PSL beyond the listed subset (release, until, case, etc.).
