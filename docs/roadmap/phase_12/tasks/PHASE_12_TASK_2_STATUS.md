# Phase 12, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_12_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `dev_tools/analysis/extract_calls.py` | the committed extractor (stdlib-only python3: re, json, sys, pathlib). Splits on the line-anchored entry marker, extracts per-kind payloads in BOTH recorded forms (inline-backticked on the field line; fenced code block with width-matched close), validates edit_file edits JSON (n_edits + is_new_file from old_string==""), labels explicit paths docs/sources/scratch/other per the task file's fixed classes, emits one JSON row per call with the fixed schema, and prints a per-kind parsed/unparsed/total reconciliation that must equal the marker-line count or the script exits 1. Task 5.6 ports this file rule-by-rule to Rust. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | commands.md is read-only input; no other file touched |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/phase_12/tasks/PHASE_12_TASK_2.md` | changed | "Task corrections" section appended (two entries: widened four-backtick fences; web_search alternate recorded forms) - the planned-task sections are untouched per FORMATTING.md section 5 |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `dev_tools/analysis/calls.jsonl` | new (gitignored) | the inventory: 12,596 rows, one JSON object per line, schema per the task file Context. Regenerable in ~2 s via the committed script; NOT tracked (git check-ignore verified in task 1) |
| `TASKS.md` | changed | task 2 entry appended |

## Changed functionality
- None (analysis infrastructure only; no tool behavior exists yet - all rfs-dev subcommands remain task-1 stubs).

## Removed functionality
- None.

## New functionality
- A machine-readable, reconciled inventory of every recorded assistant tool
call from the Phases 0-11 sessions: 12,596 rows covering terminal 9,623 /
  edit_file 2,038 / python 818 / search_conversation 83 / web_search 34. This
  is the sole input of task 3's classification subtasks and of every future
  `dev_tools/PROTOCOL.md` re-run.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (no committed test) | - | the verification is the extractor's built-in reconciliation (rows == marker lines, per-kind parsed+unparsed == kind total) plus the raw-file cross-check and 5 spot-checks below - recorded here, not as a repo test. Task 5.6's port-equality test will commit this verification for the Rust implementation. |

## Tests run status

| Name | Status |
|---|---|
| extractor internal reconciliation | EXACT - 12,596 rows == 12,596 marker lines; per-kind totals 9,623 / 2,038 / 818 / 83 / 34 with parsed+unparsed summing per kind (terminal 9,621 parsed + 2 unparsed) |
| raw-file cross-check (`grep -c '^\*\*tool call:\*\*' commands.md` + per-kind grep) | EXACT - 12,596 total; 9,623 / 2,038 / 818 / 83 / 34 per kind, matching the inventory |
| spot-checks (ordinals 1, 3149, 6298, 9447, 12596 - first / ~25% / ~50% / ~75% / last) | PASS x5 - kind matches the raw entry head; payload occurs VERBATIM inside the raw entry text (independent substring check, no extractor code reused); covers both inline form (ordinals 1, 3149, 9447, 12596) and fenced form (ordinal 6298: a 5,922-char python heredoc) |
| widened-fence rows re-verified (ordinals 408, 1305 - the two four-backtick edit_file entries found during debugging) | PASS - edits JSON parses, n_edits=1, is_new_file=false, path labeled docs (SRC_STRUCTURE.md) |
| `git check-ignore dev_tools/analysis/calls.jsonl` | OK (.gitignore line 13, set in task 1) |

## Found problems/issues/mismatches
- **Widened fences (parser bug found by the reconciliation):** 102 edit_file entries fence their edits JSON with FOUR backticks because the JSON content itself contains triple-backtick lines (markdown fences inside old_string/new_string). The first parser version assumed triple-backtick fences and left those 102 rows unparsed. Fixed with a width-matched close (opening fence of 3+ backticks, closing fence = same width at line start). Recorded as a task correction.
- **web_search has three recorded forms, not one:** the task file Context described `**query:**` (+ optional extras), but 24 of 34 entries carry only `**url:**` (URL-fetch form) and 7 carry only `**image_queries:**` (image-search form, a fenced JSON array). The extractor now falls back query -> url -> image_queries; all 34 parse. Recorded as a task correction; task 3.5's format description must state all three forms.
- **Two terminal entries are genuinely payload-less** (kept in the unparsed bucket by design): ordinal 4367 is an empty recorded call (kind line only); ordinal 4439 is a context-compaction artifact that recorded a user/assistant conversation dump instead of a command. Neither has a `**command:**` field - no parser change can recover a payload that was never written.

## Other important findings/information
- Final parsed/unparsed breakdown: terminal 9,621+2, edit_file 2,038+0, python 818+0, search_conversation 83+0, web_search 34+0. Task 3's per-kind reconciliation targets are these numbers.
- edit_file shape data (for task 3.2's expected values): is_new_file true on 324 rows (whole-new-file creations - more than the planning estimate of ~290, which undercounted because its ad-hoc JSON parse choked on the widened fences); edits-per-call histogram: 1 edit x1,443, 2 x272, 3 x127, 4 x71, 5 x37, 6 x31, >6 on the remainder. Path classes: sources 1,069 / docs 945 / other 18 (session-local scripts like audit_src_structure.py + .gitignore) / scratch 6.
- The extractor's marker regex is line-anchored (`(?m)^\*\*tool call:\*\* `), identical in spirit to the grep cross-check - a payload containing that string at line start would inflate both counts equally and be caught by the per-kind expected totals instead.
