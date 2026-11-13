# Phase 14, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Rewrite the 14 training entries that teach the `cargo run -- <command>`
pattern so the training set teaches `rfs <command>` (goal 1's rule), audit the
codegen entries for old single-file `--out` semantics, and regenerate snippets
+ the aggregate set per the training workflow.

## Context

- Verified occurrence map (2026-11-13): `training/frontend_cli/
rfs_training.jsonl` carries 10 `cargo run --` occurrences (9 diagram, 9 info,
  1 mc-ref, 3 parse, 5 test, 4 validate across its entries - the counts overlap
  because entries show multiple commands), `training/codegen_rust/
rfs_training.jsonl` 3, `training/project_tooling/rfs_training.jsonl` 1. Total:
  14 affected ENTRIES (some entries contain the pattern more than once).
- Training-set format rules (KB.md section 4.6 / START.md section 8): each
  entry is 2 messages with a `<think> ... </think>` assistant block; snippets
  are auto-extracted from the entries into `training/<cat>/snippets/{valid,
  invalid}/`; the aggregate SFT set is `all_rfs_training.jsonl` at the repo
  root (264 lines at baseline). Category-locality: edits stay inside the
  category's own jsonl.
- The regeneration trio (BUILD.md): `cargo extract-training-snippets && cargo
  build-training && cargo test-training` - snippets are re-extracted, the
  aggregate is rebuilt, and the training tests validate format + counts.
- Task 6 has landed: the help text is the reference for what the CLI looks
  like; entries teaching codegen generation must show the NEW flag family /
  spec-stem default where they teach generation at all (most frontend_cli
  entries teach parse/validate/info/diagram/test - those only need the prefix
  swap).
- The training data covers the DSL + toolchain usage; it is NOT a place to
  teach every new flag - swap the invocation pattern, and update only entries
  that specifically demonstrate codegen output naming.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Help-text fix | The final CLI surface the entries must teach. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 11. | Close-out | `cargo test-training` is part of full verification. |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/frontend_cli/rfs_training.jsonl` | change | 10 entries: `cargo run -- <cmd>` -> `rfs <cmd>`; codegen-demonstrating entries updated to the new output options where they show them. |
| `training/codegen_rust/rfs_training.jsonl` | change | 3 entries: prefix swap + old `--out FILE`-as-filename semantics updated to the new flag family / spec-stem default. |
| `training/project_tooling/rfs_training.jsonl` | change | 1 entry: prefix swap. |
| `training/*/snippets/` (all categories) | regenerated | Auto-extracted from the edited entries. |
| `all_rfs_training.jsonl` | regenerated | Aggregated SFT set (line count stays at the baseline unless entries are added/removed - none are). |

## Planned actions

### Action 10.1 - Rewrite the affected entries

In each of the three jsonl files, replace every `cargo run -- <rfs-subcommand>`
with `rfs <subcommand>` (arguments unchanged). For entries that demonstrate
codegen generation with the old `--out FILE` single-file semantics, update the
demonstrated command to the new surface (e.g. `rfs codegen rust production
spec.rfs --out arq` producing `arq-fsm.rs`, or the no-flag spec-stem form) -
keeping the entry's teaching point intact. Do NOT add new entries and do NOT
rewrite entries that only mention cargo in legitimate contexts (workspace
builds, cargo-fuzz).

Rationale: the SFT set must not teach the invocation pattern this phase removes
from the binary's own help text - a model trained on `cargo run -- validate`
would contradict `rfs --help`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/frontend_cli/rfs_training.jsonl` | change (10 entries) | Prefix swap + codegen-demo updates. |
| `training/codegen_rust/rfs_training.jsonl` | change (3 entries) | Prefix swap + output-semantics updates. |
| `training/project_tooling/rfs_training.jsonl` | change (1 entry) | Prefix swap. |

### Action 10.2 - Regenerate snippets + aggregate, run the training tests

Run the regeneration trio: `cargo extract-training-snippets && cargo build-
training && cargo test-training`. Verify the aggregate line count matches the
baseline (264) and that no snippet file gained/lost entries unexpectedly.

Rationale: snippets are derived data - hand-editing them would desync them
from the entries; the trio is the only supported update path.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/*/snippets/` | regenerated | Derived from the edited entries. |
| `all_rfs_training.jsonl` | regenerated | Aggregated set. |

## Task deliverables
- Zero `cargo run -- <rfs-subcommand>` occurrences in any training jsonl.
- Codegen-demonstrating entries teaching the new output surface.
- Regenerated snippets + aggregate; training tests green.

## Task exit criteria
- [ ] `grep -rn "cargo run --" training/ --include="*.jsonl"` returns zero hits.
- [ ] Every codegen-demonstrating entry uses the new flag family / spec-stem
      default (no old `--out FILE`-as-filename teaching remains).
- [ ] `cargo extract-training-snippets && cargo build-training && cargo test-
      training` all pass; aggregate line count unchanged from baseline.

## Non-goals
- No new training entries (category-locality + no scope creep).
- No changes to non-affected categories.
- No GPU training runs (that is a separate workflow per START.md section 6).

## Other information

- When editing jsonl, preserve the exact 2-message structure and the
  `<think> ... </think>` block format - `cargo test-training` validates the
  shape, but a malformed line also breaks snippet extraction silently.
- Record in the status file which entries were prefix-swaps only vs which got
  codegen-semantics updates (the latter are the ones a future reviewer should
  re-check against the final CLI).
