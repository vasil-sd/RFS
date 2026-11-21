# Phase 18, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Extend the SFT training set so it covers modelling REAL protocols/controllers - how to express
sequence numbers / timers / windows / counters as bit-vector vars, how to state the safety
invariants and bounded-liveness properties, and how to read the counterexamples on a real
machine - referencing the new Phase 18 examples. Then regenerate snippets + aggregate.
Rationale: the training set must reflect the new breadth (KB.md section 4.6).

## Context
- Depends on tasks 1-6 (the entries reference the new example projects by name/README).
- Format rules (KB.md section 4.6): each entry is ONE JSONL line with exactly two messages
  (user question, assistant answer); the assistant answer is wrapped in `< think> ... < /think>`
  carrying the reasoning steps, then the user-facing answer. Fenced ```rfs blocks in either
  message are auto-extracted as snippets by `cargo extract-training-snippets` (parse-valid ->
  `snippets/valid/`, parse-invalid -> `snippets/invalid/`). The aggregate is
  `all_rfs_training.jsonl` via `cargo build-training`; verify with `cargo test-training`.
- Category targets (per KB.md section 4.6 coverage descriptions):
  - `examples_explain`: how to model a real protocol/controller as an RFS spec (bit-vector vars
    for seq/timers/windows, guarded transitions, emitted outputs) - reference the new examples.
  - `model_checking`: reading counterexamples on REAL machines (a deadlock CE on tcp_full_states,
    a property-violation CE on bms_protection or traffic_light_ctrl) - reuse the pinned captures.
  - `codegen_rust`: production code generation of a real protocol in both backends (warning-free
    bar), and the adapter/monitor/no_std integrations from task 7.
  - `project_tooling`: the Phase 18 catalogue itself - what the 50 examples cover and how to run
    one (reference the EXAMPLES.md "Other examples" section).
- Keep any ```rfs fence to a COMPLETE, VALID spec (so it lands in snippets/valid, not invalid).
  If a fenced spec is added to snippets/valid, `cargo test-generated-code` will grow (the
  generator path-dedups) - that is expected and recorded, NOT a codegen regression.
- Write a FEW entries per cluster (not one per FSM) - enough to teach the real-FSM modelling
  pattern; the examples themselves are the reference.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-6. | all FSM batch tasks | the entries reference these examples |
| 7. | Cargo-member subset | the codegen_rust entry references the adapter/monitor/no_std integrations |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Close-out | verifies `cargo test-training` green + regenerated aggregate |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/examples_explain/rfs_training.jsonl` | change | new entries: modelling a real protocol/controller as an RFS spec |
| `training/model_checking/rfs_training.jsonl` | change | new entry: reading CEs on real machines (reuse pinned captures) |
| `training/codegen_rust/rfs_training.jsonl` | change | new entry: production codegen of a real protocol in both backends + integrations |
| `training/project_tooling/rfs_training.jsonl` | change | new entry: the Phase 18 catalogue (what the 50 examples cover, how to run one) |
| `training/<cat>/snippets/` | regenerated | via cargo extract-training-snippets |
| `all_rfs_training.jsonl` | regenerated | via cargo build-training |

## Planned actions

### 9.1 - Add the new coverage entries
Write the entries per Context: a few for examples_explain (real-FSM spec modelling), one for
model_checking (CE reading on a real machine, reusing a pinned capture from tasks 1-6), one for
codegen_rust (production codegen of a real protocol in both backends + the task-7 integrations),
one for project_tooling (the catalogue). Follow the format rules exactly (two messages, think
block, rfs fences only for complete valid specs).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/examples_explain/rfs_training.jsonl` | change (+few) | teach real-FSM spec modelling - the skill this phase adds |
| `training/model_checking/rfs_training.jsonl` | change (+1) | CE reading on a REAL machine, not just a teaching spec |
| `training/codegen_rust/rfs_training.jsonl` | change (+1) | production codegen of a real protocol + the integrations |
| `training/project_tooling/rfs_training.jsonl` | change (+1) | keep the catalogue picture current for future sessions |

### 9.2 - Regenerate snippets + aggregate, verify
`cargo extract-training-snippets && cargo build-training && cargo test-training`. Record the new
snippet/aggregate totals in the task status. If a new entry's rfs fence fails to parse and lands
in snippets/invalid unexpectedly, fix the fence - invalid snippets are for intentional partial-
syntax teaching, not accidents.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/<cat>/snippets/` | regenerated | the extractor is the source of truth for snippet placement |
| `all_rfs_training.jsonl` | regenerated | the aggregate must reflect every change |

## Task deliverables
- New training entries covering real-FSM modelling breadth (protocol / control / other).
- Regenerated snippets + aggregate, verified by `cargo test-training`.

## Task exit criteria
- [ ] New entries follow the format rules (two messages, think block) - `cargo test-training` green enforces the structural checks.
- [ ] Any rfs fence is a complete valid spec (lands in snippets/valid, not invalid by accident).
- [ ] Snippets + aggregate regenerated; new totals recorded in the task status.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No GPU training run (`training.py` untouched - only its data changes).
- No rewrites of existing entries (only new ones are added).
