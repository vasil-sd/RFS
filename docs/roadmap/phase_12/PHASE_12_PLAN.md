# Phase 12 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

**The evidence base - `commands.md`.** The repo root carries `commands.md`
(13 MB, 161,362 lines): the chronological log of every assistant tool call from
the Phases 0-11 chat sessions. Entry format (verified by direct reading):
each entry starts with a line `**tool call:** \`<kind>\`` followed by kind-
specific fields -

| Kind | Count | Fields |
|---|---|---|
| `terminal` | 9,623 | `**command:**` - the shell command, either inline-backticked on one line OR in a fenced code block (both forms occur; multi-line commands only in the fenced form) |
| `edit_file` | 2,038 | `**path:**` (inline backticks) + `**edits:**` (fenced JSON array of `{old_string, new_string[, replace_all]}`; `old_string: ""` = whole-new-file creation) |
| `python` | 818 | `**code:**` - inline or fenced Python source |
| `search_conversation` | 83 | `**query:**` (inline backticks) |
| `web_search` | 34 | `**query:**` (+ optional `**image_queries:**`, `**url:**`, `**top_k:**`) |

Total: **12,596 calls**. Planning-stage reconnaissance (session-local python
pass over the whole file; the pass is re-run and productized by tasks 2-3) gives
the category distribution the plan is sized against:

- terminal: file-content lookup ~5,880 (~61% - grep/ls/find/wc/head/tail/cat
  ~4,480 + `sed -n '<a>,<b>p'` range reads ~1,449 + `dd` byte-chunked reads ~13
  + misc); cargo build/test/run ~2,123 plus fuzz `timeout` runs 116 and direct
  rustc/gcc/make ~40 (~24%); git 557; shell-writes (heredoc/`cat >`/`tee`) 208;
  env-checks/echo ~110; file-ops (mv/rm/cp/mkdir) 58; shell-edits (`sed -i`,
  `perl -pi`) 51; python-via-shell ~43; vendored-crate-source inspection
  (reading `$CARGO_HOME/registry/src/...` of layout-rs/arbitrary/z3) ~33;
  install/network ~6.
- edit_file: sources ~1,088 (979 modify + 87 new-file), docs ~945 (662 modify
  + 204 new-file), scratch/tmp 3. Docs include the root-level living documents
  (START/BUILD/TOOLCHAIN/KB/DESIGN/CODEGEN/TASKS/SRC_STRUCTURE/TEST_STRUCTURE).
- python: bulk rewrites (str.replace/re.sub) 231, ad-hoc analysis 228, file
  transforms 181, audit/diff/mask scripts 99, JSON processing 56, shell
  orchestration 23 - i.e. the scripted half of the same operations that the
  terminal/edit_file rows do by hand.
- search_conversation + web_search: 117 calls, all lookup (conversation recall,
  crate/API documentation) - no dedicated tool proposed; noted in the analysis.

**Why this drives the tool set.** The dominant cost classes are exactly the
mechanical ones: (1) locating and reading file content in chunks (the token-
economy protocol of `START.md` section 0 is a workaround for the absence of a
structured lookup tool); (2) docs<->source reference re-syncing after any
refactor (Phase 11 task 10, done as a scripted path mapping over 1,794 stale
references); (3) string-masked inventory audits (Phase 10 close-out, a
hand-written character-level masker that had to be RECONSTRUCTED in Phase 11
because the script was session-local - the exact failure a committed tool
prevents); (4) bulk symbol/path rewrites across files (231 python rewrite calls
+ 51 `sed -i` calls, all pattern-fragile); (5) appending TASKS.md/status files
in fixed templates (204 docs new-file creations, most of them template-fills).

**Planning decisions taken now** (the draft left these open):
- `dev_tools/` is a WORKSPACE MEMBER (added to the root `Cargo.toml`
  `[workspace] members`), directory at the repo root per the phase description,
  crate + binary name `rfs-dev`. Consequence: `cargo test --workspace`,
  `cargo clippy --all-targets` and `cargo fmt --check` cover it automatically;
  plain `cargo build`/`cargo test` from the root still select only the `rfs`
  package (existing workspace note in BUILD.md section 2), so dev_tools runs
  via `-p rfs-dev` / `--workspace`. The known-good baseline counts in
  BUILD.md/START.md therefore GROW by dev_tools' test count - task 6 records
  the new baselines.
- Markdown parsing: `pulldown-cmark` (decided; `comrak` remains the declared
  fallback only if a mutable GFM AST proves necessary during task 5).
- The commands.md extractor (task 2) is a COMMITTED python script
  (`dev_tools/analysis/extract_calls.py`) following the Phase 10/11 practice of
  session audit scripts - but committed this time, so `dev_tools/PROTOCOL.md`
  re-runs are reproducible. Its output `calls.jsonl` (~12,596 rows) is
  GITIGNORED: it is regenerable in seconds and would bloat the repo; only the
  compact classification tables (task 3) and the README sections are committed.
- `DOCS_STRUCTURE.md` (section-level docs inventory backing the doc-section
  tool): NOT decided here - task 4 decides it from the analysis data (the
  lookup-category split between docs and sources is the deciding evidence).
- Examples / training sets: NOT affected by this phase (dev_tools never parses
  or generates RFS specs; no `examples/` or `training/` changes) - stated here
  so the "include examples/training updates where applicable" rule is explicitly
  discharged as N/A.
- `dev_tools/PROTOCOL.md` (future re-run protocol for analysis + tool-set
  update) was written AFTER this planning stage per the user's instruction and
  is OUT OF SCOPE of this phase's task set; it references tasks 2-4 as the
  procedure to repeat on new `commands.md` versions.

**Token economy for the implementation sessions.** Tasks 3.x MUST work on the
`calls.jsonl` inventory, never on raw `commands.md` chunks (spot-checks of a
few entries against the raw file are allowed and expected as validation). The
raw file is read in full only by task 2's extractor.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Scaffold the `dev_tools` crate + README skeleton | `dev_tools/Cargo.toml` (new), `dev_tools/src/main.rs` (new), `dev_tools/src/lib.rs` (new), `dev_tools/README.md` (new), `Cargo.toml` (change - members list), `.gitignore` (change - `dev_tools/analysis/calls.jsonl`) | Create the workspace-member crate: edition 2021, binary `rfs-dev`, deps exactly `syn` (+`proc-macro2`) and `pulldown-cmark` plus nothing else; `main.rs` is a dispatch stub (subcommand parsing that errors "not implemented" for every planned subcommand - the stable CLI surface is fixed NOW so later tasks only fill bodies); `lib.rs` holds the shared modules. README carries the two section headers (`## Analysis of tool calls patterns`, `## Proposed tools`) with placeholder text, filled by tasks 3.5 and 4. Rationale: every later task needs a buildable home and the committed section structure; fixing the CLI surface early keeps task 5's subtasks independent. |
| 2. | Extract `commands.md` into a structured call inventory | `dev_tools/analysis/extract_calls.py` (new), `dev_tools/analysis/calls.jsonl` (new, gitignored) | Session-local python extractor: parse all 12,596 entries (all five kinds; BOTH payload forms - inline-backticked and fenced; the edit_file JSON arrays validated) into one JSONL row per call: `{ordinal, kind, payload, paths[]}` where `paths` are the target file paths with a class label (`docs` / `sources` / `scratch` / `other`, using the root-level living-docs list from Context). Emits a summary line (per-kind counts) that MUST match the Context table - mismatch means a parser bug, stop and fix. Rationale: the 13 MB raw file is not classifiable in-context; this compact inventory is the input of ALL task-3 subtasks and of every future `dev_tools/PROTOCOL.md` re-run; committing the script (not its output) makes the analysis reproducible without repo bloat. |
| 3. | Classify all tool calls into operation categories (the split analysis) | `dev_tools/analysis/terminal_categories.tsv` (new), `dev_tools/analysis/edit_file_categories.tsv` (new), `dev_tools/analysis/python_categories.tsv` (new), `dev_tools/analysis/misc_categories.tsv` (new) | The huge analysis task, split per the phase description. Each subtask reads ONLY `calls.jsonl`, assigns every call of its kind to exactly one category, and writes a TSV: `category, count, abstract_pattern, example1, example2, example3` (examples truncated to <=160 chars). 3.1 terminal (~9,620 rows): categories at minimum - lookup-docs, lookup-sources, lookup-vendored-deps, build-test-run, git, shell-write, shell-edit, file-ops, env-check, python-via-shell, other (the Context distribution is the expected result; deviations are findings to record). 3.2 edit_file (~2,038): docs vs sources x new-file vs modify vs whole-file-rewrite, plus edit-batch shape stats (edits-per-call histogram, replace_all usage). 3.3 python (~818): purpose categories (bulk-rewrite / audit-mask / transform / json-processing / orchestration / analysis) x target class (docs / sources / inventories). 3.4 search_conversation + web_search (117): query-topic categories; expected outcome: no dedicated tool, recorded as such. Rationale: one kind per subtask keeps each pass small enough to finish and verify in one session; the TSVs are the auditable intermediate between extraction (2) and the README consolidation (3.5). |
| 3.5 | Consolidate the analysis into `dev_tools/README.md` | `dev_tools/README.md` (change - fill `## Analysis of tool calls patterns`) | Write the master section: (a) the format description of `commands.md` (the Context table), (b) the MASTER category table - one row per category across all kinds: `category, kind, count, share of total, abstract pattern, implication for dev_tools` - whose counts MUST sum to 12,596 (the exit-criterion check), (c) the four per-kind TSVs embedded as subtables, (d) a short "what this means for the tool set" paragraph naming the operation classes that become tools and those that do not (with reasons). Close-out of the analysis: after this task, `## Analysis of tool calls patterns` is frozen - task 4 consumes it read-only. Rationale: the phase description stores the analysis result in exactly this section; freezing it before the proposal keeps proposal-vs-evidence auditable. |
| 4. | Elaborate the proposed tools table | `dev_tools/README.md` (change - fill `## Proposed tools`) | Write the table `| Tool name | CLI | Purpose | Detailed description and rationale |` covering, at minimum, the five PHASE_12.md scope categories - docs<->sources sync (`rfs-dev sync-check` / `sync-fix`), context extraction (`rfs-dev context <name> [--file F] [--depth N]`), token-efficient source editing (`rfs-dev edit-rename`, `edit-move`, `edit-replace-item`), docs section extraction (`rfs-dev doc-section <file|all> <heading...>`), inventory maintenance (`rfs-dev inventory add|update|remove|audit`) - plus any further tool the frozen analysis shows to be needed (expected: `rfs-dev calls extract|classify`, productizing tasks 2-3 for PROTOCOL.md re-runs). Each row's rationale names the concrete phase-operation it replaces (Phase 10 close-out audit, Phase 11 task-10 re-sync, the ~5,880 chunked-lookup calls, the 231+51 pattern-fragile rewrites, the 204 template-fill file creations) and its CLI shape follows the analysis patterns (compact structured stdout, exit codes 0/1/2, `--json` where a machine consumes the output). Also DECIDE here, with rationale from the data: whether `DOCS_STRUCTURE.md` is built (and as which task-5 subtask). Rationale: this is the phase's design deliverable - the proposal must be traceable row-by-row to the frozen analysis section. |
| 5. | Implement the proposed tools + tests | `dev_tools/src/<one module per tool>.rs` (new), `dev_tools/tests/<one integration test file per tool>.rs` (new), `dev_tools/README.md` (change - CLI column corrected if implementation reveals a better flag shape, via plan correction) | One subtask per tool group of the task-4 table (expected 5.1 sync-check/fix, 5.2 context, 5.3 edit-rename/move/replace-item, 5.4 doc-section (+ DOCS_STRUCTURE.md if task 4 decided yes), 5.5 inventory add/update/remove/audit, 5.6 calls extract/classify; the AUTHORITATIVE list is the `## Proposed tools` table as finalized by task 4 - deltas are plan corrections). Every subtask: implement against the dispatch stub from task 1 (CLI surface fixed), unit tests in-module, one integration test file exercising the tool end-to-end on a fixture tree under `dev_tools/tests/fixtures/`, and a DEMONSTRATION on a real maintenance operation recorded in the task status file (acceptance test per the phase description - e.g. `sync-check` reports clean on the current tree; `context` on a named function yields the markdown template; `edit-rename` on a scratch copy of a real module rewrites all import sites and compiles). 5.x final subtask: full verification (`cargo test --workspace`, clippy, fmt) with dev_tools included. Rationale: per-tool subtasks keep each implementation session bounded and independently verifiable; the demonstration-on-real-operation requirement is the phase's exit criterion made executable. |
| 6. | Close-out: verification, docs sync, status report | `BUILD.md` (change - section 3/4 rows for `-p rfs-dev` + new baseline counts), `START.md` (change - source-tree map row for `dev_tools/`, section-1 snapshot line), `KB.md` (change - section 4.2 file-map row), `docs/roadmap/CURRENT_STATE.md` (change - phase 12 row COMPLETE), `TASKS.md` (change - entry per its first-paragraph template), `docs/roadmap/phase_12/PHASE_12_STATUS.md` (new) | Run the full checklist with dev_tools in the workspace; record the NEW known-good baselines (main-test count grows by dev_tools' tests; workspace count likewise); sync every document that inventories the tree or the commands (BUILD.md command table, START.md source tree + snapshot, KB.md 4.2); write `PHASE_12_STATUS.md` per FORMATTING.md section 4 (added files, functionality deltas, tests run status with exact counts, findings - including any analysis deviations from the Context expectations); update CURRENT_STATE.md; append the TASKS.md entry. Rationale: PROTOCOL.md section 3 steps r-s - the phase exits only with docs synchronized and the status report written. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 5.4 | Implement the proposed tools + tests (subtask 5.4: doc-section) | change | The plan Context said "Markdown uses pulldown-cmark: doc-section walks headings", but pulldown-cmark 0.13.6's event API exposes NO source spans (the 0.12+ rewrite dropped `sourcepos`), so byte-accurate section extraction is impossible from the parser alone. doc-section therefore uses a line-based fence-aware ATX heading walk - which is also exactly the semantics context.rs's `extract_fragment` and sync-check's anchor rule already use, keeping section boundaries byte-identical to the two existing consumers of "what a heading is" in this repo. No new dependency; pulldown-cmark stays declared (used by sync) but doc-section does not call it. Recorded here at task-5 close-out (the 5.4 session noted it in its TASKS.md entry but did not append this row). |
| 5.5 | Implement the proposed tools + tests (subtask 5.5: inventory) | change | The plan Context said "the string-masked audit (5.5) re-implements the Phase 10/11 close-out method: extract every fn/type identifier from source with strings masked, diff against inventory rows per file". The implementation uses syn parsing instead of string-masking + grep: syn is strictly more robust (it cannot confuse a string literal's contents for an identifier - exactly the failure the masking existed to prevent) and reproduces the Phase 10/11 naming conventions exactly, proven by the EXACT real-tree demonstration (SRC 936 fns / 196 types, TEST 942 fns / 34 types - matching the Phase 11 close-out record). No new dependency (syn was already allowed). Recorded here at task-5 close-out (the 5.5 session noted it in its TASKS.md entry but did not append this row). |
| 5.6 | Implement the proposed tools + tests (subtask 5.6: calls extract/classify) | change | Dependency constraint extended with a THIRD declared exception, `regex` (user-authorized during 5.6 execution): the rule-by-rule port of `extract_calls.py` + `classify_calls.py` matches ~15 python `re` patterns (alternation, `\b`, lazy/greedy quantifiers), which the `regex` crate reproduces directly - hand-rolled std matching was the original assumption but is strictly more error-prone for the greedy-backtrack cases (e.g. `\b[\w.-]+\.rs\b` on `foo.rs.rs`). `regex 1.13.1` is already in Cargo.lock transitively, so no new lock entry and offline rebuilds keep working; the PHASE_12.md exit criterion ("no dependency outside the RFS set + syn + pulldown-cmark") is read as allowing this because the user explicitly added it as a declared exception alongside syn/pulldown-cmark. The single python pattern using a backreference + lookahead (the fenced-form close `^\1[ \t]*$(?=\n|\Z)`) is handled with a regex-open + same-width line-scan hybrid, since `regex` omits both by design. Recorded in dev_tools/Cargo.toml and PHASE_12_TASK_5.md Task corrections. |

## Other information

- Open question for task 4: whether `doc-section` needs the full `DOCS_STRUCTURE.md` inventory or a on-the-fly heading walk suffices (pulldown-cmark makes the walk trivial; the inventory pays off only if cross-doc section lookup by name becomes frequent - the analysis' lookup-docs category data decides).
- Risk: `syn` + `pulldown-cmark` are new to the lock file - the FIRST build needs network access to crates.io (available in this environment per TOOLCHAIN.md section 3); afterwards the registry cache serves offline rebuilds. If the fetch fails, fall back to `comrak` (already in the lock file as a root dev-dependency) for the markdown side and record a plan correction.
- The dispatch stub (task 1) must NOT be a dead-code warning source: every planned subcommand gets a real `unimplemented!()`-free stub returning a pinned "not implemented yet" error, so clippy stays clean from task 1 onward.
- Task 3's TSV category names are the STABLE vocabulary of the analysis; task 3.5 and task 4 must reuse them verbatim (no re-naming) so the proposal traces to the evidence.
