# Phase 4, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Create the shared codegen-time MC plan module `src/mc_plan.rs`: input-abstraction
analysis, state-encoding layout, and traversal spec computed ONCE in the Rust tool
and embedded as constants/tables in both generated binaries, so the exact engines
of tasks 2-3 are identical by construction (D-P4-1..3).

## Context

- Mirrors the Phase 3 `MonitorPlan` pattern (`src/monitor.rs`): a plan computed at
  codegen time in the Rust tool, rendered per backend into self-contained binaries.
- The current MC runtime (`MC_RUNTIME` in `src/backends/rust_mc.rs`, mirror in
  `c_mc.rs`) explores ONE representative input per event variant via
  `representative_input_expr`. Exact mode replaces that with full canonical payload
  enumeration - the plan module is what tells each engine WHICH fields to enumerate
  and how to encode/visit configurations.
- Three plan components (all pure functions over the IR, deterministic):
  1. Input abstraction (D-P4-3): a payload field of an event variant is ABSTRACTED
     (single representative value) iff it is not referenced by any transition guard,
     output payload expression, variable update expression, or property/assumption
     formula. `input_is` patterns that test the field's value COUNT as references.
     The soundness argument goes into DESIGN section 11 (task 6 writes it; task 13
     finalizes).
  2. State-encoding layout: configuration = (state index, packed vars, monitor
     states) per D-P4-2. Compute byte offsets/widths for each variable's packing and
     the total config size; monitor state size comes from the existing MonitorPlan.
  3. Traversal spec (D-P4-2): canonical successor order - events in declaration
     order, payload fields in declaration order, values ascending (bool false<true,
     int 0..max, enum declaration order); det vs `--nondet` successor rules;
     assumption pruning list (D-P4-6: active assumptions prune inputs; `--assumes
     none` explores everything).
- IR access: guards/output payloads/var updates/properties/assumptions are all in
  the parsed+validated IR. `src/codegen_fsm.rs` already collects variant/field info
  for codegen - reuse that collection rather than re-walking.
- The plan is NOT consumed by any backend until task 2; this task changes no
  generated output.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Exact engine - Rust | consumes the plan (encoding + traversal) to render the BFS runtime |
| 3. | Exact engine - C | same plan tables rendered into C constants |
| 6. | Liveness engine - Rust | extends the plan with liveness obligation decomposition |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc_plan.rs` | new | plan data structures + IR analysis (abstraction, encoding, traversal) + `#[cfg(test)]` unit tests |
| `src/lib.rs` | change | declare `mod mc_plan;` |
| `src/codegen_fsm.rs` | change | expose/reuse existing variant/field collection for the plan |

## Planned actions

### Action 1: Plan data structures + module wiring
Define `McPlan` (or equivalent) holding the three components as plain serializable
tables (so both backends can render them verbatim): per-event-variant field
abstraction flags, variable packing layout (offset/width per var, config byte size),
traversal spec (canonical input order description, successor rules, assumption
pruning). Wire `mod mc_plan` into `src/lib.rs`. Reuse the variant/field info
collection in `src/codegen_fsm.rs` (extract a shared helper if needed) - do not
duplicate IR walking logic.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | new | data structures + module skeleton |
| `src/lib.rs` | change | module declaration |
| `src/codegen_fsm.rs` | change | shared variant/field collection helper |

### Action 2: Input-abstraction analysis
Walk every transition guard, output payload expression, variable update expression,
and property/assumption formula; for each event variant collect the set of payload
fields referenced (including `input_is` patterns with field values). A field outside
the union is abstracted -> one representative value (existing
`representative_input_expr`). Record per-variant: concrete fields (with canonical
value order) vs abstracted fields.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | change | abstraction analysis function over the IR |

### Action 3: State-encoding layout
Compute the configuration byte layout: state index, each variable packed at a fixed
offset/width (bit widths from IR types), monitor states appended (size from
MonitorPlan). The layout must be deterministic for a given IR (same bytes every
run) - both engines hash configs as raw bytes.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | change | packing layout computation |

### Action 4: Traversal spec
Encode the canonical successor order (events declaration order; payload fields
declaration order; values ascending: bool false<true, int 0..max, enum declaration
order), det vs nondet successor rules (D-P4-2/D-P4-11 input), and the assumption
pruning list (D-P4-6).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | change | traversal spec construction |

### Action 5: Unit tests
`#[cfg(test)]` in `src/mc_plan.rs` with inline specs covering: (a) each reference
kind keeps a field concrete (guard / output payload expr / var update expr /
property / assumption / `input_is` with field value); (b) an unused field is
abstracted; (c) encoding layout determinism (same IR -> same bytes, twice);
(d) canonical input order for a multi-variant/multi-field event set.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_plan.rs` | change (`#[cfg(test)]`) | unit tests per Action 5 list |

## Task deliverables
- `src/mc_plan.rs`: deterministic plan (abstraction + encoding + traversal) computed
  once at codegen time, with unit tests.
- No generated-output changes yet (plan not consumed until task 2).

## Task exit criteria
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.
- [ ] Unit tests cover all five reference kinds + abstraction + layout determinism + canonical order.
- [ ] Plan output is byte-deterministic for a fixed IR (tested).
- [ ] No codegen output changed: `cargo test-generated-code` still 513 pass without regeneration (regenerate only if generation output actually changed - it must not have).

## Non-goals
- Engine implementation (tasks 2/3), liveness decomposition (task 6), flag wiring
  (task 2), docs (task 13).

## Other information
- Keep the plan a pure function of the IR: no environment reads, no randomness.
- If `codegen_fsm.rs`'s variant/field collection is entangled with rendering, extract
  the minimal shared helper rather than refactoring the renderer.
- Baseline to preserve: 170 main tests / 513 generated tests (2026-10-17).
