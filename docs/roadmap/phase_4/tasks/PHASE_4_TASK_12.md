# Phase 4, Task 12 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Update the training data for the Phase 4 language + MC changes: SFT examples for
`eventually`/`until` syntax + weak-until semantics, exact MC statuses
(`pass-complete`/`fail`/`unknown-state-limit`), input abstraction, and liveness
counterexamples; regenerate snippets + aggregate; `cargo test-training` green.

## Context

- Files (per plan): `training/properties_temporal_logic/rfs_training.jsonl`
  (change), `training/model_checking_properties/rfs_training.jsonl` (change),
  `training/<cat>/snippets/**` + `all_rfs_training.jsonl` (regenerated via the
  cargo aliases).
- Follow the EXISTING jsonl format of each file (inspect current entries first -
  field names, valid/invalid pairing conventions, snippet references); new entries
  must match or the structure tests fail.
- Content topics:
  1. `eventually`/`until` syntax + weak-until semantics (valid specs using them;
     invalid variants: missing parens on `eventually`, non-boolean operands, deeper
     nesting beyond D-P4-9 scope).
  2. Exact MC statuses: the three-status taxonomy + summary-line format in
     model-checking context examples.
  3. Input abstraction: specs with unused payload fields (like example 43's
     `bit<32>` checksum) - what gets abstracted and why.
  4. Liveness counterexamples: prefix+cycle shape in trace output.
- Snippets + aggregate regenerate via `cargo extract-training-snippets && cargo
  build-training` (BUILD.md section 4); counts will change - record old->new for
  task 14's status file.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | Liveness language extension | the syntax being taught |
| 6. | Liveness engine - Rust | semantics/statuses/counterexamples as implemented (ground truth for examples) |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13. | Docs sync | docs and training data must agree on syntax/statuses |
| 14. | Close-out | final training counts recorded in PHASE_4_STATUS.md |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/properties_temporal_logic/rfs_training.jsonl` | change | `eventually`/`until` syntax + weak-until semantics examples (valid + invalid) |
| `training/model_checking_properties/rfs_training.jsonl` | change | exact MC statuses, input abstraction, liveness counterexamples |
| `training/<cat>/snippets/**` | regenerate | via `cargo extract-training-snippets` |
| `all_rfs_training.jsonl` | regenerate | via `cargo build-training` |

## Planned actions

### Action 1: Inspect existing format
Read the current entries of both jsonl files + one snippet set; note field names,
pairing conventions, and how invalid examples are marked. New entries follow these
conventions exactly.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - inspection only) | - | format ground truth for Actions 2-3 |

### Action 2: Temporal-logic examples
Add SFT entries to `training/properties_temporal_logic/rfs_training.jsonl`:
`eventually (p)` / `p until q` usage with weak-until semantics; invalid variants
(missing parens, non-boolean operands, out-of-scope nesting per D-P4-9).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/properties_temporal_logic/rfs_training.jsonl` | change | new temporal-logic entries |

### Action 3: Model-checking examples
Add SFT entries to `training/model_checking_properties/rfs_training.jsonl`: exact
MC status taxonomy + summary line, input abstraction (unused payload fields),
liveness counterexample shape (prefix+cycle).

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/model_checking_properties/rfs_training.jsonl` | change | new MC entries |

### Action 4: Regenerate + test
`cargo extract-training-snippets && cargo build-training && cargo test-training`;
record old->new snippet/aggregate counts for task 14.

#### Affected files
| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/<cat>/snippets/**` | regenerate | snippets from the updated jsonl files |
| `all_rfs_training.jsonl` | regenerate | aggregate with new counts |

## Task deliverables
- Updated training jsonl files covering the four topics; regenerated snippets +
  aggregate; `cargo test-training` green.
- Old->new counts recorded (status file).

## Task exit criteria
- [ ] `cargo extract-training-snippets && cargo build-training && cargo test-training` green.
- [ ] New entries cover: eventually/until syntax + weak-until semantics, exact MC statuses, input abstraction, liveness counterexamples.
- [ ] Entries match the existing jsonl format (structure tests pass).
- [ ] Old->new counts recorded for task 14.

## Non-goals
- Model training runs (`training.py` GPU work - out of scope), docs (task 13).

## Other information
- Keep example specs in the training data SMALL and self-contained (they are SFT
  material, not benchmarks); reuse shapes from examples 43-45 where helpful but do
  not copy them verbatim wholesale.
