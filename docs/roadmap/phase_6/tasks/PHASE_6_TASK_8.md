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
- [x] Every touched category file still passes the structure tests (two messages, think block format) - `cargo test-training` green. DONE 2026-10-25: `cargo test-training` 5/5 (4 dedicated + the codegen aggregate test), incl. `committed_all_rfs_training_file_is_fresh`.
- [x] All quoted command outputs verified real (captured during action 1, not invented). DONE 2026-10-25: every quote in the 17 new entries was captured live during this task's execution from the v1.0.0 toolchain (mc-ref on specs 43/44/46 incl. --max-states 100, bad-version-header validate error, IR-version-2 rejection error, info on spec 46, codegen production headers rust+c on spec 46, gen-spec random seed 42 + mutation base 03 seed 7 + diff + validate, mc-ref/gen-spec missing-file errors, spec-48 --violate fuzz run) - or reused verbatim from captures made earlier in this session and re-verified (spec 13 three-engine byte-parity, spec 45 liveness lasso).
- [x] Snippets + aggregate regenerated; new counts recorded in "Other information" below. DONE 2026-10-25: see Other information.
- [x] No other category files touched (category-local rule). DONE 2026-10-25: git diff shows exactly the five category jsonl files changed + regenerated snippets + all_rfs_training.jsonl.

## Non-goals
- No new training categories; no GRPO/reward changes (`training.py` untouched); no GPU training runs (that is a separate, user-triggered activity per BUILD.md);
- no rewording of existing entries (append-only within this task's scope).

## Other information
- **Counts (recorded after action 2, for the BUILD.md baseline in task 9):**
  - aggregate `all_rfs_training.jsonl`: **244 -> 261 lines** (+17 = the new entries).
  - snippets: **135 valid + 127 invalid -> 138 valid + 128 invalid** (+3 valid, +1 invalid; most quoted outputs are ```text/```sh blocks which snippet extraction does not collect - only RFS code blocks count).
  - category line counts: syntax_semantics 11->14, design 12->15, model_checking 27->31, examples_explain 36->39, frontend_cli 11->15.
  - generated-code aggregate crate (development artifact under target/, regenerated for tree consistency): **189 modules / 1601 tests -> 192 modules / 1683 tests** (`cargo generate-code-tests && cargo test-generated-code` green) - the +3 modules are the new valid training snippets, each carrying the full extended plan classes.
- **New entry inventory (17):** syntax_semantics x3 (version header meaning/format + real pinned error; optionality/legacy v0.x; why the existing mechanism was formalized over a comment header), design x3 (stale-artifact detection via line-1 header with real rust+c headers; what the spec hash covers + hand-rolled SHA-256 rationale; IR_VERSION 0->1 read policy with the real version-2 rejection error), model_checking x4 (when a PASS is trustworthy + real spec-46 output; --violate mechanics + real spec-48 run 161/200 assumption stops; rfs mc-ref quick-check workflow + real spec-43 output; unknown-state-limit semantics + real spec-44 --max-states 100 output), examples_explain x3 (full-spec walkthroughs of 47/48/49 in the established structured format, each with its real exact-mode MC result), frontend_cli x4 (mc-ref flags/exit codes + real missing-file error; gen-spec both modes + real random/mutation outputs + validity guarantee; mutation workflow with real diff; mc-ref vs generated-binary choice).
