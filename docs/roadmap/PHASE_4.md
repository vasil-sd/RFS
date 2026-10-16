# Phase 4: Scalable model checking

## Goal

Move from toy exhaustive checking to a useful verification tool.

## Scope

- exact reachable-state engine in generated Rust or `rfs-mc-core` (visited set, parent pointers for counterexamples, memory limits, state count reporting, deterministic and nondeterministic modes);
- statuses: `PASS (complete)`, `FAIL`, `UNKNOWN (state limit reached)`;
- input abstraction for payload fields not used by any guard, action, or property;
- optional symbolic/SMT bounded mode (z3 bindings, k-liveness; clearly labeled incomplete);
- liveness engine: `eventually p`, `p until q` (Büchi product or k-liveness) with a documented fairness policy;
- performance features: state-space statistics, limits, parallel search where safe, incremental MC cache maybe later.

## Exact reachable-state engine

Implement proper state-space search in generated Rust or in `rfs-mc-core`:

- visited set;
- parent pointers for counterexamples;
- memory limits;
- state count reporting;
- input abstraction for unused payload fields;
- deterministic and nondeterministic modes.

Statuses:

```text
PASS (complete)
FAIL
UNKNOWN (state limit reached)
```

## Input abstraction

If an event payload field is not used by any guard, action, or property, the model checker can treat it as a don't-care value.

Example:

```rfs
event Recv(Packet(len: bit<8>, tag: bit<32>))
```

If `tag` is never used in transitions or properties, MC does not need to iterate over all `tag` values. This is sound when the field is truly behaviorally irrelevant.

## Optional symbolic/SMT bounded mode

For larger bitvectors, add an optional backend:

- SMT-based bounded model checking;
- k-liveness for bounded liveness;
- clearly labeled as incomplete unless completion criteria are met.

Possible Rust dependencies later:

- `z3` bindings;
- custom bitvector encodings;
- SAT/SMT solver backend trait.

This mode is for bug hunting and larger-data exploration, not a replacement for exact finite MC.

## Liveness engine

Add strong liveness support:

```rfs
eventually p
p until q
```

Implementation options:

1. Büchi product + fair cycle search in Rust.
2. k-liveness transformation for bounded checking.

Fairness policy must be documented.

Recommended default fairness for deterministic systems:

- environment assumptions define input fairness;
- system behavior is deterministic;
- no hidden scheduler nondeterminism.

For nondeterministic MC mode:

- use strong transition fairness or explicitly mark fair transitions;
- document clearly because liveness under arbitrary unfair scheduling can be misleading.

## Performance features

Add:

- state-space statistics;
- transition count;
- property monitor size;
- memory usage estimate;
- timeout and state-limit options;
- parallel search where safe;
- incremental MC cache for CI, maybe later.

## Deliverables

- Exact MC engine with complete/unknown reporting and actionable counterexamples.
- Bounded SMT mode behind a feature flag (optional; bug hunting, not exact).
- Liveness checking in at least bounded mode with documented fairness.
- Benchmark suite with several protocol-like examples tracking parse/codegen/MC metrics.

## Exit criteria

- exact MC completes on medium-sized finite specs;
- bounded MC works on larger bitvector specs;
- liveness properties can be checked in at least bounded mode;
- counterexamples are actionable;
- tool reports complete vs bounded vs unknown clearly;
- benchmark suite exists with several protocol-like examples.

## Non-goals

- replacing exact finite MC by symbolic methods (the SMT mode is for bug hunting and larger-data exploration only);
- full toolchain, `build.rs` integration, diagrams, IDE support (Phase 5);
- production hardening, formal semantics document, v1.0 stability (Phase 6).
