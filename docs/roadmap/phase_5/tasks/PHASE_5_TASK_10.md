# Phase 5, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Add `--counterexample-format json|text` to the model checker: a machine-readable
JSON report mirroring the (parity-pinned) text report - status, metrics,
reason/label/step, statistics block, and the counterexample (trace or lasso) -
with TEXT remaining the byte-identical default. Parity extends to byte-identical
JSON across backends.

## Context

- The text report format is PARITY-PINNED (byte-identical across backends,
  asserted by the parity + benchmark suites). `text` stays the default and its
  output must not change at all in this task.
- CRITICAL constraint: generated MC binaries are SELF-CONTAINED (no crate dependencies) - and after task 2 the toolchain itself has no serde. The JSON document is emitted by an EMBEDDED COPY of the shared `src/json_rw.rs` writer (task 2) in each backend's engine
  (string escaping for labels/paths, number formatting identical to the text
  report). Keep the writer minimal: objects, arrays, strings, integers, booleans.
- Flag surface: codegen-time `--counterexample-format json|text` (default
  text) embedded as a runtime-overridable default in the generated binary
  (same pattern as the other MC flags - parsed by the generated CLI).
- JSON document shape (planned; pinned by tests):
  ```json
  {
    "status": "pass-complete | fail | unknown-state-limit",
    "metrics": { "states": N, "transitions": M, "max_depth": D },
    "reason": "property-violation | liveness-violation | deadlock (fail only)",
    "label": "<property label> (fail only)",
    "step": S,
    "statistics": { "unexpected_terminations": N, "monitor_size_bits": B, "memory_estimate_bytes": E },
    "counterexample": { "kind": "trace|lasso|deadlock-config", ... }
  }
  ```
  `counterexample.trace`: array of steps (input, outputs, from_state, to_state,
  vars) matching the text trace; `lasso`: prefix + cycle arrays + cycle length;
  `deadlock-config`: the terminal configuration. Field names pinned in tests.
- Where both formats carry the same data, values must agree (a test compares a
  JSON run against the text run's parsed fields for pass/fail/liveness cases).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Hand-rolled JSON reader-writer + migration off serde | the shared writer this task embeds in the generated engines |

### Dependent tasks

(none directly; tasks 14/15 document and train it)

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen.rs` | change | `--counterexample-format` flag (codegen-time, embedded default) |
| `src/codegen_mc.rs` | change | format enum + JSON document model (field names, ordering) shared by both renderers |
| `src/backends/rust_mc.rs` | change | hand-rolled JSON writer + report branching (text unchanged) |
| `src/backends/c_mc.rs` | change | mirror renderer producing byte-identical JSON |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/backend_parity_test.rs` | change | byte-identical JSON across backends (pass, fail-property, fail-liveness, fail-deadlock, unknown cases); text output unchanged assertion |
| lib unit tests in `src/codegen_mc.rs` | change | JSON writer escaping/formatting pins; document shape pins |

## Planned actions

### Action 1: flag + shared document model
Add the format enum + field-name constants + document builder in
`codegen_mc.rs` (language-agnostic); wire the codegen flag with the embedded-
default pattern.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen_mc.rs` | change | model + constants + unit pins |
| `src/codegen.rs` | change | flag parsing |

### Action 2: Rust JSON renderer
Hand-rolled writer (escaping: backslash, quote, control chars; integers as in
the text report) + report branching in the generated engine. Text path
untouched.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/rust_mc.rs` | change | writer + branch |

### Action 3: C JSON renderer (byte-identical)
Mirror producing byte-identical output (same key order, same number format,
same escaping). Verify with the parity suite.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/backends/c_mc.rs` | change | mirror renderer |

### Action 4: parity extension
JSON byte-identity across backends for: pass-complete (benchmark spec 43),
property-violation, liveness-violation (lasso), deadlock, unknown-state-limit;
plus a text-unchanged guard (re-pin the existing text expectations - they must
still hold verbatim).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | the JSON parity cases |

## Task deliverables
- `--counterexample-format json|text` (codegen + runtime override), default text.
- Byte-identical JSON across backends for all five status/reason combinations.
- Text report untouched (pinned).

## Task exit criteria
- [ ] Default output is byte-identical to pre-task text reports (existing pins still pass verbatim).
- [ ] JSON output parses and matches the document shape pins; values agree with the text report's data for the same run.
- [ ] JSON is byte-identical across backends for all five cases (parity test green).
- [ ] Generated binaries remain dependency-free (no serde in generated code - asserted by an existing generation marker check).
- [ ] Full main suite green, clippy/fmt clean.

## Non-goals
- JSON for the fuzz mode (fuzz keeps its own output; revisit if ever needed).
- Streaming/chunked JSON (reports are small; single emission is fine).
- Schema file publication (the shape is pinned by tests + task 14 docs).

## Other information
- Key ORDERING is part of the byte-parity contract - document it in
  codegen_mc.rs next to the constants.
