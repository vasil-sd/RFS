# Phase 5, Task 14 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Extend the LLM training set with the Phase 5 user-facing surface (new CLI
commands, external-tool parameterization, --print-script, rfs-build, adapters,
no_std, MC counterexample JSON, diagrams) following the established format, and
regenerate snippets + aggregate.

## Context

- Format precedent (Phase 4 task 12): each entry has a think block (reasoning
  the model should learn) + assistant messages quoting tool output VERBATIM -
  so every quoted command/output in a new entry must be produced by actually
  running the shipped binary (no hand-written outputs). Invalid-variant entries
  quote real error text.
- Target categories: `frontend_cli` (info/test/diagram commands with options,
  exit codes, --print-script, validate --json), `project_tooling` (rfs-build
  build.rs usage incl. RFS_BIN, workspace layout, example-crate walkthroughs),
  `codegen_rust` (--no-std, --with-protocol-fsm, adapters, MC
  --counterexample-format json). ~10-15 new examples total across categories -
  quality over quantity; each entry must teach a distinct behavior.
- Snippets: valid rfs blocks in entries become snippet files (extractor runs
  them through the parser) - keep new spec snippets small and valid; invalid
  quotes stay inline (not extracted).
- Counts to expect after this task: training aggregate grows from 231 lines;
  snippet counts move accordingly; `cargo test-training` must pass with the new
  numbers.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-13. | All implementation tasks | entries quote the SHIPPED behavior - everything must exist and be final before this task |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 15. | Docs sync | documents the same surface; training data is its behavioral twin |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/frontend_cli/rfs_training.jsonl` | change | info/test/diagram/validate --json entries (real outputs) |
| `training/project_tooling/rfs_training.jsonl` | change | rfs-build/build.rs/RFS_BIN/workspace/example-crate entries |
| `training/codegen_rust/rfs_training.jsonl` | change | no_std/--with-protocol-fsm/adapters/MC-JSON entries |
| `training/*/snippets/**` | regenerated | new valid snippets from the entries |
| `all_rfs_training.jsonl` | regenerated | aggregate with the grown count |

## Planned actions

### Action 1: draft entries per category
For each behavior in Context: run the real command, capture verbatim output,
write think+assistant entries following the existing style (read 2-3 existing
entries per category first for tone/format). Include at least one
invalid/error-output entry per new command area (e.g. --no-std on mc rejected;
diagram with dot missing falling back).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| the three category jsonl files | change | the new entries |

### Action 2: regenerate snippets + aggregate
`cargo extract-training-snippets && cargo build-training`; verify every new
valid snippet parses (the extractor validates) and invalid quotes stay inline.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/*/snippets/**` | regenerated | extracted snippets |
| `all_rfs_training.jsonl` | regenerated | aggregate |

### Action 3: training tests green
`cargo test-training` (structure, snippet validity, aggregation, freshness) -
record the new counts.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - test run only) | - | count record for the status file |

## Task deliverables
- ~10-15 new training entries across the three categories (all outputs verbatim).
- Regenerated snippets + aggregate; training tests green with recorded counts.

## Task exit criteria
- [ ] Every quoted tool output in new entries was produced by the shipped binary (no hand-written outputs - spot-checkable).
- [ ] At least one error/invalid-output entry per new command area.
- [ ] `cargo extract-training-snippets && cargo build-training && cargo test-training` green; new aggregate count recorded.
- [ ] New valid snippets all parse; no snippet drift in existing categories.

## Non-goals
- Retraining the model itself (training.py runs are user/GPU work).
- New training CATEGORIES (the three existing ones cover the surface).
- Rewriting existing entries (additive only).

## Other information
- Keep entries self-contained: a future reader must be able to learn the
  behavior from the entry alone (command + output + what it means in the think
  block).
