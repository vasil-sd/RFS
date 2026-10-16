# Phase 3, Task 19 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Planned task 19: training-set update - add SFT examples covering `always[N]`
syntax/semantics, the monitor-compilation concept, restrictions R1-R4, online monitors
in MC, assumption pruning, and enhanced counterexamples.

## Context

- Categories already exist (plan context): `training/properties_temporal_logic/` and
  `training/model_checking_properties/` - each with `rfs_training.jsonl` +
  `snippets/`. No new category needed.
- Format rules (KB.md section 4.6): one JSON object per line, exactly 2 messages
  (user prompt + assistant answer); assistant content wrapped in `< think> ... </think>`
  reasoning block; rfs code blocks inside examples are auto-extracted into snippets by
  `cargo extract-training-snippets` and classified by parser+validator - so every rfs
  block added here MUST parse and validate.
- Content must match FINAL Phase 3 behavior (hence depends on tasks 13-18): exact
  deadline semantics D-P3-3 (violation confirmed at the deadline sample), R1-R4 text
  from PHASE_3_PLAN.md "Other information", `--with-property-monitors` CLI, per-language
  gating (cfg(debug_assertions) / RFS_PROPERTY_MONITORS).
- Current aggregate: see BUILD.md sections 3-4 for exact category/aggregate counts.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 13.-18. | Examples, tests, codegen, aggregate green | training content must describe the final, verified behavior |

### Affected training files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/properties_temporal_logic/rfs_training.jsonl` | change | +2-3 SFT examples: always[N], monitor compilation concept, R1-R4 |
| `training/model_checking_properties/rfs_training.jsonl` | change | +1-2 SFT examples: online monitors in MC, assumption pruning, enhanced counterexamples |

## Planned actions

### Action 1: properties_temporal_logic additions (2-3 examples)

Add 2-3 new lines following the category's existing style (read a few existing lines
first for tone/structure). Cover:
1. `always[N]` syntax + exact semantics: p must hold at samples 0..N-1; violation at
   the FIRST failing sample; N >= 1 required (validator error otherwise);
   `bounded_eventually[0]` legal = body must hold at the trigger sample.
2. Monitor-compilation concept: each prop/assume compiles to a per-label ONLINE monitor
   (per-sample signals, delay registers for `next`, obligation monitors with
   trigger/deadline countdown for `be[N]`, violation confirmed at the deadline sample -
   D-P3-3 worked example); same plan rendered in every backend.
3. Restrictions R1-R4: which formulas compile to monitors and which are skipped with a
   warning ("outside monitor subset; skipped") - include one accepted and one rejected
   formula pair per restriction where practical.
Every rfs block must parse + validate (run `cargo run -q -- validate` on it before
appending).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/properties_temporal_logic/rfs_training.jsonl` | change (append 2-3 lines) | teaches the new property subset + monitor concept to the model |

### Action 2: model_checking_properties additions (1-2 examples)

Add 1-2 new lines covering:
1. Online monitors in the MC product state (system_state + vars + property/assumption
   monitor states); assumption monitors prune environment branches ONLINE (including
   liveness assumptions like `always(be[5] input_is(Tick))` - previously impossible).
2. Enhanced counterexamples: which property failed, which step triggered the
   obligation, when the deadline expired, relevant variable values, I/O sequence;
   identical information in Rust and C (parity principle).
Same format + parse/validate discipline as Action 1.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/model_checking_properties/rfs_training.jsonl` | change (append 1-2 lines) | teaches online MC monitoring + counterexample format |

## Task deliverables
- 3-5 new SFT examples across the two categories (exact count in status file), all
  valid JSON, 2-message format with think blocks, all rfs blocks parsing cleanly.

## Task exit criteria
- [ ] New lines appended; every line valid JSON with exactly 2 messages and a `< think>` block.
- [ ] Every rfs code block in the new examples passes `cargo run -q -- validate` (parse + validation).
- [ ] Content matches final behavior: D-P3-3 deadline semantics, R1-R4, CLI flag, per-language gating.

## Non-goals
- No snippet/aggregate regeneration here (task 20); no new category; no training.py
  changes; no GRPO-specific data (SFT format only).

## Other information

- Keep examples SHORT and self-contained (a minimal module + the property under
  discussion), matching the compact style of existing lines in these categories.
- Do not copy plan text verbatim - write as teaching material (question -> reasoned answer).
