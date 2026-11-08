# Phase 12, Task 4 - Implementation status

> Written once when all exit criteria of `PHASE_12_TASK_4.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | documentation-only task - the design deliverable is the README section |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | no code touched; all rfs-dev subcommands remain task-1 stubs |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `dev_tools/README.md` | changed | `## Proposed tools` filled (task-1 placeholder replaced): the 9-row table `\| Tool name \| CLI \| Purpose \| Detailed description and rationale \` covering all five PHASE_12.md scope categories + `calls`; a No-tool decisions list with per-class reasons; the DOCS_STRUCTURE.md decision (REJECTED) with its deciding data. The frozen `## Analysis of tool calls patterns` section above it is byte-identical to before (verified by regeneration diff) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | task 4 entry appended |

## Changed functionality
- None.

## Removed functionality
- None.

## New functionality
- The phase's design deliverable: the proposed tool set, traceable row-by-row to the frozen analysis. Nine tools across six groups - sync-check/sync-fix (docs<->source reference re-syncing), context (one-command impact context with a fixed markdown template), edit-rename/edit-move/edit-replace-item (syn-parsed structured edits), doc-section (heading-path retrieval, on-the-fly pulldown-cmark walk), inventory add/update/remove/audit (row maintenance + the string-masked completeness check), calls extract/classify (the Rust port of this phase's own analysis pipeline). CLI conventions fixed for all: compact structured stdout, exit 0/1/2, `--root`, plan-first + `--dry-run` for mutating tools.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (no committed test) | - | the verifications are mechanical checks recorded below; task 5's per-tool tests will commit them as executable form |

## Tests run status

| Name | Status |
|---|---|
| CLI-surface consistency (table subcommands vs task-1 stubs parsed from `dev_tools/src/main.rs`) | EXACT - all 15 stub subcommands covered, zero table names outside the stub list; NO plan corrections needed |
| frozen-section byte identity (consolidate-mode regeneration diffed against the README section) | IDENTICAL - no edits leaked into the frozen analysis section |
| evidence citation check (every table row cites >=1 frozen category with its count) | PASS - all 9 rows carry an "Evidence: <category> <count>" clause drawn from the master table |
| `docs_md_test` (5) | OK - the new table + subsections pass the mdr-stack gate |

## Found problems/issues/mismatches
- None in the deliverable. Two verification-script bugs were found and fixed in the CHECKS themselves (not the README): the stub-surface parser initially looked for SUBCOMMANDS in classify_calls.py (it lives in the Rust dispatch stub) and used a wrong manifest regex; the frozen-check regeneration initially wrote to an absolute path the fallback did not anticipate. Both re-ran clean.

## Other important findings/information
- DOCS_STRUCTURE.md decision data (recorded in the section): of 2,035 lookup-docs calls, only 17.2% mention >=2 distinct doc files in one command - and those are `ls`/`wc -c`/reference greps, not heading-name hunts across files; 32.5% are single-file range reads against known line numbers and 13.2% are intra-file heading greps to obtain line numbers (the START.md section-0 token-economy protocol). On-the-fly heading walks suffice; a prebuilt index would add sync-check surface for no measured benefit. Revisit trigger: a future commands.md showing frequent cross-doc heading hunts.
- Task 5's authoritative tool list is now this table - one implementation subtask per row group (expected 5.1 sync, 5.2 context, 5.3 edit-*, 5.4 doc-section WITHOUT DOCS_STRUCTURE.md, 5.5 inventory, 5.6 calls) + 5.x verification. The DOCS_STRUCTURE.md rejection means 5.4 stays a single-tool subtask.
- The no-tool decisions are part of the deliverable: eleven classes explicitly declined with reasons (build-test-run, git, env-check, lookup-vendored-deps, shell-write, docs-new, sources-new, file-ops residual, both 'other' classes, json-processing/orchestration/analysis, conversation-recall, web-docs-lookup) - absence of a tool is a recorded decision, per dev_tools/PROTOCOL.md.
