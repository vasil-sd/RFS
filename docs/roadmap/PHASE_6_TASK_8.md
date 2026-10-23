# Phase 6, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Extend the SFT training data with coverage of the new v1.0 surface: the version
header + versioning policy/spec hash, trustworthy MC reports + forced violations,
the two new CLI commands (`rfs mc-ref`, `rfs gen-spec`), and the new benchmark
specs - following the established category-local format, then regenerate
snippets + aggregate.

## Context

- **Format rules** (KB.md section 4.6): each line of
  `training/<category>/rfs_training.jsonl` is one JSON object with exactly two
  messages: user question + assistant answer; the assistant content starts with
  a `< think>\n...< /think>` block (numbered reasoning steps) followed by the
  answer; answers quote REAL tool output verbatim where they show commands or
  results (run the commands to capture - never invent output). Snippets are
  AUTO-EXTRACTED from the jsonl files by `cargo extract-training-snippets`
  (valid/invalid code blocks); `cargo build-training` aggregates to
  `all_rfs_training.jsonl`; `cargo test-training` verifies structure/freshness.
- **Current baseline**: 244 aggregate lines; 135 valid + 127 invalid snippets;
  11 categories. This task touches FIVE existing categories (no new categories -
  the set is stable for v1.0).
- **Content per category** (3-5 entries each, mirroring the style of existing
  entries in that file):
  - `syntax_semantics`: the module version header - what it means (`version
    "1.0"`), the format rule (`X.Y` / `X.Y.Z`, validator check), legacy no-header
    specs = v0.x still accepted, why the existing mechanism was formalized
    instead of a comment-style header.
  - `design`: the v1.0 versioning policy - spec hash (SHA-256 of canonical IR
    JSON, first 16 hex chars) + toolchain version in generated headers and what
    they let users verify; IR_VERSION 0 -> 1 with the {0,1} read policy; why
    hand-rolled SHA-256 (no-dependency stance).
  - `model_checking`: trustworthy reports - status taxonomy (PassComplete /
    Fail / UnknownStateLimit), the no-false-PASS labeling guarantee, forced
    violations (`--violate label:prob:percent%`), and `rfs mc-ref` for in-process
    prototyping (flags + exit codes 0/1/2).
  - `examples_explain`: walkthroughs of the new benchmark specs 47 (HTTP/2-like
    stream), 48 (USB-like control transfer), 49 (CAN-like arbitration) - what
    each abstracts, key states/events/vars, and their properties.
  - `frontend_cli`: `rfs mc-ref` + `rfs gen-spec` as CLI commands - full flag
    lists, exit codes, a real example run of each (capture actual output),
    mutation mode (`--base SPEC --mutations K`) and its validity guarantee.
- **Entry quality bar**: same as existing entries - concrete, repo-specific,
  no filler; every quoted command/output must be real (run it).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2./3./6. | Versioning / CLI commands / benchmark specs | the entries document these in their SHIPPED form - all must be complete first |
| 7. | External user documentation set | entries can point at `docs/user/` pages that exist by then |

### Affected training files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `training/syntax_semantics/rfs_training.jsonl` | change | version-header entries |
| `training/design/rfs_training.jsonl` | change | versioning policy / spec hash entries |
| `training/model_checking/rfs_training.jsonl` | change | trustworthy reports + forced violations + mc-ref entries |
| `training/examples_explain/rfs_training.jsonl` | change | benchmark specs 47/48/49 entries |
| `training/frontend_cli/rfs_training.jsonl` | change | mc-ref / gen-spec command entries |
| snippets (`training/<cat>/snippets/{valid,invalid}/`) + `all_rfs_training.jsonl` | regenerated | via the cargo aliases below |

## Planned actions

### Action 1: draft + capture entries
For each of the five categories: run the relevant commands to capture real
output (e.g. `rfs mc-ref examples/valid/47_http2_stream.rfs`, `rfs gen-spec
--base ... --mutations 3 --out /tmp/x.rfs && rfs validate /tmp/x.rfs`,
`rfs info` on a versioned spec), then write the entries in the established
format (think block + verbatim quotes).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| the five category jsonl files | change | new entries appended |

### Action 2: regenerate + verify
`cargo extract-training-snippets && cargo build-training && cargo test-training`.
Record the new snippet counts (valid/invalid) and aggregate line count; they
feed task 9's BUILD.md baseline update.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `training/<cat>/snippets/{valid,invalid}/` | regenerated | auto-extracted snippets (new entries may add valid/invalid code blocks) |
| `all_rfs_training.jsonl` | regenerated | aggregate SFT set |

## Task deliverables
- 15-25 new training entries across the five categories (3-5 each), all with
  real captured output.
- Regenerated snippets + aggregate; `cargo test-training` green.
- New counts recorded for task 9.

## Task exit criteria
- [ ] Every touched category file still passes the structure tests (two messages, think block format) - `cargo test-training` green.
- [ ] All quoted command outputs verified real (captured during action 1, not invented).
- [ ] Snippets + aggregate regenerated; new counts recorded in "Other information" below.
- [ ] No other category files touched (category-local rule).

## Non-goals
- No new training categories; no GRPO/reward changes (`training.py` untouched); no GPU training runs (that is a separate, user-triggered activity per BUILD.md);
- no rewording of existing entries (append-only within this task's scope).

## Other information
- Record here after action 2: old -> new counts (aggregate lines, valid/invalid
  snippets) for the BUILD.md baseline in task 9.
