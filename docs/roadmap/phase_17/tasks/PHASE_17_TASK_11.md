# Phase 17, Task 11 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Bring the SFT training set in line with the current toolchain: rewrite the
three stale mc-ref entries, add coverage for the new USAGE.md material (CE-trace
reading, artifact take-or-emit, RFS_* tool resolution) and the ten new example
projects, then regenerate snippets + aggregate.

## Context
- Format rules (KB.md section 4.6): each entry is ONE JSONL line with exactly
  two messages (user question, assistant answer); the assistant answer is
  wrapped in `< think> ... < /think>` carrying the reasoning steps, followed by
  the user-facing answer. Fenced ```rfs blocks in either message are auto-
  extracted as snippets by `cargo extract-training-snippets` (parse-valid ->
  `snippets/valid/`, parse-invalid -> `snippets/invalid/`). The aggregate is
  `all_rfs_training.jsonl` via `cargo build-training`; verify with
  `cargo test-training`.
- The three STALE entries (verified 2026-11-19, all pre-Phase 16):
  - `training/frontend_cli/rfs_training.jsonl` line index 11: "What does the
    rfs mc-ref subcommand do? Explain its flags and exit codes." - answer is
    exact-only, lists --seed/--violate/--nondet as NOT applicable.
  - `training/frontend_cli/rfs_training.jsonl` line index 14: "When should I
    use rfs mc-ref versus a generated model-checking binary?" - answer claims
    "it has no fuzz mode at all" and that the generated binary is "the only way
    to get fuzz mode".
  - `training/model_checking/rfs_training.jsonl` line index 29: "How do I
    model-check an RFS spec quickly without compiling any generated code?" -
    answer is exact-only with the same NOT-applicable list.
  (Line indexes are 0-based positions in the file as of preparation; locate by
  the user question text, not the index - it may shift if earlier tasks touched
  these files, which they do not.)
- The current mc-ref CLI facts for the rewrites: the plan Context section has
  the full flag/exit-code surface. The generated binary's UNIQUE value after
  Phase 16 is `--threads`/`--fast` + the production-identical engine (MC =
  production core + runtime) - that is the correct "when to use which" answer.
- NEW entry targets (category per KB.md section 4.6 coverage descriptions):
  - model_checking: CE-trace reading - how to read a deadlock lasso (prefix +
    cycle) and a property-violation trace (step-by-step, ce_fields), using task
    6's real captured traces as the ```rfs-adjacent examples (the traces are
    text/JSON, not rfs fences - keep any rfs fence to the spec itself).
  - codegen_rust: artifact take-or-emit semantics - `--out BASE` derivation,
    `--fsm-file` taken-vs-emitted, partial-pair usage error, stale-reuse
    warning (the Phase 14 model).
  - frontend_cli: RFS_* tool-resolution precedence - flag > env var > standard
    var > PATH, `--print-script` inspection, the clang-no-standard-tier rule.
  - project_tooling: the new example projects - what each of the ten new
    examples demonstrates and how to run it (one entry covering the set, or a
    few per cluster - implementation decides, but every new example must be
    named).
- Snippet counts: current baseline is 138 valid + 128 invalid snippets / 267
  aggregate lines. New entries carrying ```rfs fences (e.g. the fixed specs,
  small demo specs) will shift these - record the new totals in the task
  status; `cargo test-training` must pass with them.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Negative example projects | the CE-reading entry reuses its real captured traces |
| 10. | Write USAGE.md | entries should reference the same semantics USAGE.md documents (consistency) |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/frontend_cli/rfs_training.jsonl` | change | rewrite 2 stale entries + 1 new (tool resolution) |
| `training/model_checking/rfs_training.jsonl` | change | rewrite 1 stale entry + 1 new (CE reading) |
| `training/codegen_rust/rfs_training.jsonl` | change | 1 new entry (take-or-emit) |
| `training/project_tooling/rfs_training.jsonl` | change | new entr(y/ies) covering the ten new examples |
| `training/<cat>/snippets/` | regenerated | via cargo extract-training-snippets |
| `all_rfs_training.jsonl` | regenerated | via cargo build-training |

## Planned actions

### 11.1 - Rewrite the three stale mc-ref entries
Locate each entry by its user question text. Keep the user message unchanged;
rewrite the assistant answer (think block + body) against the current CLI:
per-mode flags with defaults, per-mode exit codes, `--nondet` applicable under
exact, not-applicable reduced to `--threads`/`--fast`, parity with BOTH
backends in all three modes. For entry 14 specifically: mc-ref now covers
fuzz/smt-bounded in-process; the generated binary's unique value is
`--threads`/`--fast` + the production-identical engine - update the rule of
thumb accordingly.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/frontend_cli/rfs_training.jsonl` | change (2 entries) | "no fuzz mode at all" actively mis-trains the model |
| `training/model_checking/rfs_training.jsonl` | change (1 entry) | same staleness class |

### 11.2 - Add the new coverage entries
Write the four new entr(y/ies) per Context: CE-trace reading (model_checking,
with task 6's real traces), take-or-emit semantics (codegen_rust), RFS_*
precedence (frontend_cli, with a real `# tools:` line from task 8.4's capture),
new example projects (project_tooling - every one of the ten named). Follow the
format rules exactly (two messages, think block, rfs fences only for actual
specs).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/model_checking/rfs_training.jsonl` | change (+1) | CE reading is the skill the negative examples teach - the model must learn it too |
| `training/codegen_rust/rfs_training.jsonl` | change (+1) | the artifact semantics are the most error-prone CLI surface |
| `training/frontend_cli/rfs_training.jsonl` | change (+1) | tool resolution is a top user question on non-standard systems |
| `training/project_tooling/rfs_training.jsonl` | change (+1..) | the category's job is keeping the implementation picture current |

### 11.3 - Regenerate snippets + aggregate, verify
`cargo extract-training-snippets && cargo build-training && cargo test-
training`. Record the new snippet/aggregate totals (baseline: 138 valid + 128
invalid / 267 lines) in the task status. If a new entry's rfs fence fails to
parse and lands in snippets/invalid unexpectedly, fix the fence - invalid
snippets are for intentional partial-syntax teaching, not accidents.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/<cat>/snippets/` | regenerated | the extractor is the source of truth for snippet placement |
| `all_rfs_training.jsonl` | regenerated | the aggregate must reflect every change |

## Task deliverables
- Three rewritten entries + four new coverage areas in the training set.
- Regenerated snippets + aggregate, verified by cargo test-training.

## Task exit criteria
- [ ] No training entry remains claiming mc-ref is exact-only or lacks fuzz /
      smt-bounded / `--nondet` (grep the jsonl files for "no fuzz mode" and
      "NOT applicable" near mc-ref).
- [ ] Every new entry follows the format rules (two messages, think block) -
      `cargo test-training` green enforces the structural checks.
- [ ] Snippets + aggregate regenerated; new totals recorded.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets
      && cargo fmt --check`.

## Non-goals
- No GPU training run (`training.py` untouched - only its data changes).
- No changes to other categories' existing entries (only the three stale ones
  are rewritten).
