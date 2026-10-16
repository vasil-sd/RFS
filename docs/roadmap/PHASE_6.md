# Phase 6: Production hardening and v1.0

## Goal

Make the toolchain trustworthy enough for production protocol work.

## Scope

- formal semantics document (automaton model, transition relation, deterministic vs nondeterministic modes, property sampling, PSL operator semantics, strong/weak liveness, fairness, assumptions, error/deadlock semantics, abstraction rules if any);
- soundness testing: differential MC comparisons, monitor-vs-reference comparisons, PSL translation checks, codegen fuzzing;
- fuzzing the toolchain itself (parser, validator, IR serialization, code generator, model checker on small generated specs);
- benchmark suite with protocol-like examples and tracked metrics;
- stability and versioning: RFS syntax version header, IR schema version, generated-code compatibility policy, migration guide process;
- documentation set for external users (language reference, property reference, MC modes reference, integration guide, examples, migration guide, FAQ).

## Formal semantics document

Write a precise semantics document covering:

- automaton model;
- transition relation;
- deterministic vs nondeterministic modes;
- property sampling;
- PSL operator semantics;
- strong/weak liveness;
- fairness;
- assumptions;
- error/deadlock semantics;
- abstraction rules, if any.

This is essential for user trust.

## Soundness testing

Add differential testing:

- compare brute-force MC with reachable-state MC on small specs;
- compare generated monitors against hand-written reference monitors;
- compare PSL translation against an external model checker or library where possible;
- fuzz random specs and check invariants of code generation.

## Fuzzing the toolchain itself

Fuzz:

- parser;
- validator;
- IR serialization;
- code generator;
- model checker on small generated specs.

Goals:

- no crashes on malformed but parseable input;
- no invalid Rust generated from valid specs;
- no false `PASS (complete)` when search was incomplete;
- stable diagnostics.

## Benchmark suite

Create benchmark specs:

- simple handshake;
- timeout/retry protocol;
- sliding window control abstraction;
- HTTP/2-like stream state machine;
- USB-like control transfer FSM;
- CAN-like arbitration/state machine, if appropriate.

Track:

- parse time;
- codegen time;
- MC states explored;
- MC time;
- memory usage;
- counterexample quality.

## Stability and versioning

Define:

- RFS syntax version header;
- IR schema version;
- generated code compatibility policy;
- migration guide process.

Example:

```rfs
# rfs 1.0
```

Generated code should include the spec hash and toolchain version.

## Documentation

Provide:

- language reference;
- property language reference;
- model-checking modes reference;
- production integration guide;
- examples;
- migration guide from earlier versions;
- FAQ on abstraction and assumptions.

## Deliverables

- Formal semantics document.
- Differential + fuzz test harnesses for parser/validator/IR/codegen/MC.
- Benchmark suite running in CI with tracked metrics.
- Versioning scheme (RFS syntax version header, IR schema version) and migration guide process.
- Full documentation set sufficient for external users.

## Exit criteria

- v1.0 syntax is stable;
- toolchain passes fuzzing and differential testing;
- model checker reports are trustworthy and clearly labeled;
- production code generation is used in at least one real or realistic protocol project;
- documentation is sufficient for external users;
- benchmark suite runs in CI.

## Non-goals

- multi-party composition, real-time timed automata, unbounded integers, arbitrary Rust function calls in transitions, byte-level protocol codecs (v1 exclusions; post-v1 candidates);
- full strong unbounded liveness unless implementation goes smoothly (see "Recommended v1 scope" in `INTRO.md`).
