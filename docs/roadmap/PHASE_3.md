# Phase 3: PSL-like properties and monitors

## Goal

Add the requested PSL-like temporal property language in a monitor-friendly form.

## Scope

- PSL-like property subset: `not`, `&&`, `||`, `->`, `next`, `always`, `always[N]`, `bounded_eventually[N]`;
- atoms: `state == S`, `state != S`, `var_expr`, `input_is(EventPattern)`, `output_has(OutputPattern)`;
- assumptions: `assume name : formula ;`;
- monitor codegen: each property compiles to a Rust monitor via a restricted PSL-to-monitor compiler;
- model checker integration: monitors in the product state, assumption monitors prune environment branches;
- counterexample enhancement with property-specific information;
- optional production debug monitors (`--with-property-monitors`, `#[cfg(debug_assertions)]` hooks).

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

Each property becomes a Rust monitor.

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

The model-checking product state includes:

```text
system_state + system_vars + property_monitor_states + assumption_monitor_states
```

Assumption monitors prune invalid environment branches. Property monitors detect violations.

## Counterexample enhancement

Counterexamples should show:

- which property failed;
- which step triggered the obligation;
- when the deadline expired;
- relevant variable values;
- input/output sequence.

## Production debug monitors

Optional feature:

```bash
rfs codegen production protocol.rfs --with-property-monitors
```

Generated code can call monitors in debug builds:

```rust
#[cfg(debug_assertions)]
self.property_monitors.step(sample)?;
```

This allows runtime detection of environment or implementation violations during testing/fuzzing.

## Deliverables

- Parser/validator support for the property subset and assumptions (extends `src/parser.rs`, `src/ir.rs`, `src/validator.rs`).
- PSL-to-monitor compiler plus Rust monitor codegen.
- MC product-state integration: assumption pruning, property violation detection, enhanced counterexamples (`src/codegen_mc.rs` + generated MC runtime).
- CLI option `--with-property-monitors` for production codegen (debug-mode monitors).
- Valid/invalid examples exercising properties, updated tests, and training-set update.

## Exit criteria

- PSL-like properties parse and validate;
- monitors are generated in Rust;
- model checker can verify safety and bounded liveness properties;
- assumptions restrict input sequences correctly;
- counterexamples include property-specific information;
- optional runtime property monitors compile and work in debug mode.

## Non-goals

- strong/unbounded liveness (`eventually p`, `p until q`) - Phase 4;
- exact reachable-state search engine beyond the current bounded runtime - Phase 4;
- symbolic/SMT bounded mode - Phase 4 (optional);
- full PSL beyond the listed subset (release, until, case, etc.).
