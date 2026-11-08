# Phase 12, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Parse the entire 13 MB `commands.md` into a compact structured call inventory
(`dev_tools/analysis/calls.jsonl`, one JSON row per recorded call) with a
COMMITTED python extractor, and prove the parse is exact by reconciling
per-kind counts against the raw file. Rationale: the raw file is not
classifiable in-context - this inventory is the sole input of task 3's
classification subtasks and of every future `dev_tools/PROTOCOL.md` re-run;
committing the script (not its output) makes the analysis reproducible without
repo bloat.

## Context
- Entry format (verified by direct reading during planning): each entry starts
  with a line `**tool call:** \`<kind>\`` followed by kind-specific fields -
  `terminal`: `**command:**`; `edit_file`: `**path:**` + `**edits:**` (fenced
  JSON array of `{old_string, new_string[, replace_all]}`; `old_string: ""`
  means whole-new-file creation); `python`: `**code:**`; `search_conversation`
  and `web_search`: `**query:**` (+ optional `image_queries`/`url`/`top_k`).
  Payloads occur in TWO forms: inline-backticked on one line, or in a fenced
  code block (multi-line commands/code only in the fenced form). The extractor
  must handle both.
- Expected totals (planning reconnaissance; the extractor's summary MUST
  reconcile against these, and any discrepancy is a parser bug to fix, not a
  finding to accept): terminal 9,623 / edit_file 2,038 / python 818 /
  search_conversation 83 / web_search 34 = **12,596** entries. Note the
  reconnaissance parsed 9,620 of 9,623 terminal payloads - the extractor must
  therefore carry an explicit `unparsed` bucket: entries whose payload field is
  missing or malformed get a row with `payload: null, "unparsed": true`, and
  the reconciliation is `entries = parsed + unparsed` per kind.
- Row schema (fixed now; task 3 and the future `rfs-dev calls extract` build
  on it): `{"ordinal": <1-based entry number>, "kind": <str>, "payload":
  <str|null>, "paths": [<explicit field paths, e.g. edit_file **path:**>],
  "path_classes": [<docs|sources|scratch|other> per path], "unparsed":
  <bool>}` plus for edit_file only: `"n_edits": <int|null>` and
  `"is_new_file": <bool|null>` (derived from the edits JSON). Terminal/python
  target paths are NOT extracted here - task 3's classifier derives them from
  payload text (extracting shell targets in the extractor would couple it to
  shell syntax it does not need to understand).
- Path class labels: `docs` = `docs/**` + the root-level living documents
  (START.md, BUILD.md, TOOLCHAIN.md, KB.md, DESIGN.md, CODEGEN.md, TASKS.md,
  README.md, SRC_STRUCTURE.md, TEST_STRUCTURE.md); `sources` = `src/`,
  `crates/`, `tests/`, `fuzz/`, `examples/`, `training/`, `ide/`, `*.rs`,
  `Cargo.toml`, `.cargo/config.toml`, `training.py`; `scratch` = `/tmp/**`,
  `target/**`, `unsloth-tmp/**`, `.unsloth_tool_output/**`; else `other`.
- The script is python3 STDLIB ONLY (re, json, pathlib) - no new dependencies,
  following the Phase 10/11 session-script practice; it lives in the crate so
  task 5.6 can port it to Rust line-for-line (`rfs-dev calls extract`).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Scaffold the dev_tools crate | `dev_tools/analysis/` must exist inside the crate directory; the gitignore line for calls.jsonl is already in place |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Classify all tool calls | consumes calls.jsonl exclusively - never the raw file |
| 5.6 (inside task 5) | `rfs-dev calls extract` implementation | ports this script's logic to Rust; the script is its specification |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/analysis/extract_calls.py` | new | the committed extractor (stdlib-only python3) |
| `dev_tools/analysis/calls.jsonl` | new | the inventory output - GITIGNORED, regenerable in seconds |

## Planned actions

### Action 1: write the extractor
`dev_tools/analysis/extract_calls.py`: split the raw text on entry boundaries
(`\n**tool call:** `), take the kind from the first backticked token, extract
the payload field per kind handling BOTH forms (inline backticks - take to the
closing backtick at line end; fenced - take to the closing fence), validate
edit_file edits JSON (count entries, detect `old_string: ""`), label explicit
paths per the Context classes, and emit one JSON row per entry with the fixed
schema. On completion print a summary table: per-kind parsed/unparsed/total
plus the grand total. Rationale: deterministic single-pass parse; the summary
is the reconciliation artifact of Action 2.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/extract_calls.py` | new | the extractor per Action 1 text - committed so PROTOCOL.md re-runs and the task-5.6 Rust port have a stable specification |

### Action 2: run it and reconcile against the raw file
Run the extractor on `/workspace/host/RFS/commands.md`. Reconcile: grand total
must equal `grep -c '^\*\*tool call:\*\*' commands.md` (12,596) and per-kind
totals must equal the Context numbers (9,623 / 2,038 / 818 / 83 / 34), with
parsed+unparsed summing to the kind total in every case. ANY mismatch: inspect
the offending entries in the raw file, fix the parser (never the expected
counts), re-run until exact. Rationale: an inventory that silently drops or
duplicates entries would poison every downstream category count - exactness is
checked here, once, at the source.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/calls.jsonl` | new | the reconciled inventory - gitignored output of Action 1's script |

### Action 3: spot-check rows against the raw file
Pick 5 ordinals spread across the file (first, ~25%, ~50%, ~75%, last). For
each, print the JSONL row and the corresponding raw-file region (`sed -n`
around the entry) side by side; verify payload text is byte-identical
(whitespace-normalized for the inline-vs-fenced difference) and path labels are
sensible. Record the 5 checks in the status file. Rationale: reconciliation
catches count errors, spot-checks catch CONTENT errors (truncated payloads,
wrong field captured) - both failure classes must be closed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/calls.jsonl` | change | no content change expected - if a spot-check fails, the parser is fixed and the file regenerated (recorded as a task correction) |

## Task deliverables
- `dev_tools/analysis/extract_calls.py` (committed, stdlib-only).
- `dev_tools/analysis/calls.jsonl` (gitignored): 12,596 rows, schema per
  Context, per-kind counts reconciled exact.
- The reconciliation summary + 5 spot-checks recorded in the status file.

## Task exit criteria
- [ ] calls.jsonl row count = 12,596; per-kind totals = 9,623 / 2,038 / 818 /
      83 / 34 with parsed+unparsed reconciling per kind (unparsed rows listed
      in the status file if any).
- [ ] every edit_file row has valid `n_edits`/`is_new_file` or is flagged
      unparsed.
- [ ] 5 spread spot-checks pass byte-identical payload comparison.
- [ ] script committed; `git check-ignore dev_tools/analysis/calls.jsonl`
      succeeds.
- [ ] no other file changed (commands.md itself is read-only input).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Widened fences (four backticks) | The Context described fenced payloads as triple-backtick blocks, but 102 edit_file entries use a FOUR-backtick fence (````json ... ````): when the edits JSON itself contains triple-backtick lines (markdown fences inside old_string/new_string), the recorder widens the fence by one. The extractor's fenced form is width-matched - opening fence of 3+ backticks, closing fence = same width at line start - which is the only unambiguous terminator. Without this fix those 102 rows were unparsed. |
| web_search alternate recorded forms | The Context said web_search entries carry `**query:**` (+ optional extras), but 31 of 34 entries have NO query field: 24 are URL-fetch form (only `**url:**`) and 7 are image-search form (only `**image_queries:**`, a fenced JSON array). The extractor falls back query -> url -> image_queries; all 34 now parse. The format description in task 3.5's README section must state these three forms. |

## Non-goals
- No classification of any kind (task 3) - the extractor emits raw structure
  only.
- No terminal/python target-path extraction (task 3's classifier owns that).
- No Rust implementation yet (task 5.6 ports this script).
