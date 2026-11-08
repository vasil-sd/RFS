# Phase 12, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Classify EVERY one of the 12,596 recorded calls into operation categories with
abstract patterns - the huge analysis split into four per-kind subtasks (3.1
terminal, 3.2 edit_file, 3.3 python, 3.4 misc) plus the consolidation (3.5)
that writes the result into `dev_tools/README.md` section
`## Analysis of tool calls patterns` and freezes it. Rationale: this section is
the evidence base of the whole phase - task 4's proposal must trace row-by-row
to it, so it is produced as auditable per-kind TSVs first and consolidated
second.

## Context
- Input is EXCLUSIVELY `dev_tools/analysis/calls.jsonl` (task 2). Never read raw
  `commands.md` chunks during classification - spot-checks of a few entries
  against the raw file are allowed as validation only (token economy, plan
  Context rule).
- The STABLE category vocabulary is fixed now (plan Context rule: task 3.5 and
  task 4 reuse these names verbatim; new categories may appear only if no
  existing one fits, recorded as a finding):
  - terminal (11): `lookup-docs`, `lookup-sources`, `lookup-vendored-deps`
    (reading `$CARGO_HOME/registry/src/**` of layout-rs/arbitrary/z3/etc.),
    `build-test-run` (cargo build/test/clippy/fmt/run/check + direct rustc/gcc/
    make + fuzz `timeout` runs), `git`, `shell-write` (heredoc / `cat >` / `tee`
    - creating or appending file content from the shell), `shell-edit`
    (`sed -i`, `perl -pi` in-place edits), `file-ops` (mv/rm/cp/mkdir/ln/touch
    without cargo), `env-check` (whoami/id/uname/env/which/command -v, tool
    version probes), `python-via-shell` (python3 invocations from the shell,
    incl. heredoc scripts), `other`.
  - edit_file (4): `docs-new`, `docs-modify`, `sources-new`, `sources-modify`
    (class per task-2 path labels; scratch/other paths go to `other` rows if
    any). Edit-batch SHAPE stats (edits-per-call histogram, replace_all usage,
    whole-file-rewrite share) are computed alongside but are NOT categories -
    they ship as extra columns in the TSV and into the 3.5 subtable.
  - python (6): `bulk-rewrite` (str.replace/re.sub over file text),
    `audit-mask` (string-masking, diffing, completeness audits - the Phase
    10/11 audit scripts), `transform` (read-modify-write without a rewrite
    pattern: line surgery, table row insertion, encoding fixes),
    `json-processing`, `orchestration` (subprocess-driven), `analysis`
    (read-only computation over files/text).
  - misc (2): `conversation-recall` (search_conversation), `web-docs-lookup`
    (web_search).
- Expected distribution (planning reconnaissance - the classifier's output is
  compared against it; deviations ARE findings to record in the status file,
  not errors to hide): terminal lookup ~5,880 (~61%), build-test-run ~2,300
  (~24%), git 557, shell-write 208, env-check ~110, file-ops 58, shell-edit 51,
  python-via-shell ~43, lookup-vendored-deps ~33, other the remainder;
  edit_file sources ~1,088 / docs ~945 / scratch 3; python bulk-rewrite 231 /
  analysis 228 / transform 181 / audit-mask 99 / json 56 / orchestration 23.
- Classification method: ONE committed script
  `dev_tools/analysis/classify_calls.py` with four modes (`--kind
  terminal|edit_file|python|misc`), stdlib-only python3, deterministic rules
  (ordered regex/keyword tests per the vocabulary above; first match wins;
  every rule is a named function so task 5.6's Rust port maps rule-by-rule).
  The script is committed because its rule set IS the specification of the
  future `rfs-dev calls classify` tool - session-local would repeat the Phase
  10/11 failure of having to reconstruct audit scripts later.
- TSV output format (one file per kind under `dev_tools/analysis/`):
  header `category, count, abstract_pattern, example1, example2, example3`
  then one row per category that occurred; examples are payload excerpts
  truncated to <=160 chars, whitespace-collapsed; `abstract_pattern` is a
  short template describing the call shape (e.g. lookup-docs: `cd <root> &&
  sed -n '<a>,<b>p' <doc>.md | head/tail/grep - optional second doc in the
  same command`). Counts MUST sum to the kind's call count (reconciliation,
as in task 2).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Extract commands.md into a call inventory | calls.jsonl is the sole input; its schema and path labels are consumed as-is |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | Elaborate the proposed tools table | consumes the FROZEN `## Analysis of tool calls patterns` section read-only; every proposal row cites a category from it |
| 5.6 (inside task 5) | `rfs-dev calls extract/classify` | ports classify_calls.py's rule set to Rust - the committed script is its spec |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/analysis/classify_calls.py` | new | the four-mode classifier (stdlib-only, deterministic rules) |
| `dev_tools/analysis/terminal_categories.tsv` | new | 3.1 output |
| `dev_tools/analysis/edit_file_categories.tsv` | new | 3.2 output (+ shape-stat columns) |
| `dev_tools/analysis/python_categories.tsv` | new | 3.3 output |
| `dev_tools/analysis/misc_categories.tsv` | new | 3.4 output |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/README.md` | change | subtask 3.5 fills `## Analysis of tool calls patterns` (placeholder from task 1) - the only README edit of this task |

## Planned actions

### Action 1 (subtask 3.1): classify terminal calls
Write the `terminal` mode of classify_calls.py implementing the 11-category
rule set (ordered tests: cargo-invocation check first, then git, then
shell-write heredoc/redirect patterns, shell-edit -i patterns, file-ops verbs,
python-via-shell, env-check probes, vendored-deps registry paths, and finally
the lookup family split docs-vs-sources by the path labels of mentioned files,
`other` as the residual). Run it over calls.jsonl; reconcile the category
count sum to 9,623 (parsed+unparsed); compare the distribution against the
Context expectations and record every deviation >5% as a finding with an
example. Write `terminal_categories.tsv`. Rationale: terminal is 76% of all
calls - its categories dominate the tool-set rationale, so it gets its own
subtask and the strictest reconciliation.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/classify_calls.py` | new | terminal mode - the rule set is the spec for the future Rust classifier |
| `dev_tools/analysis/terminal_categories.tsv` | new | 3.1 result: category, count, abstract pattern, <=3 examples per category |

### Action 2 (subtask 3.2): classify edit_file calls
Write the `edit_file` mode: category = path class x new/modify (4 categories
+ residual `other`); compute shape stats alongside - edits-per-call histogram,
replace_all usage count, share of single-edit calls, share of whole-file
rewrites (`is_new_file` or a single edit replacing the entire content). Run,
reconcile to 2,038, write `edit_file_categories.tsv` with the shape stats as
extra columns. Rationale: the docs-vs-sources split and the new-file share are
the direct evidence for the template-fill and sync tool proposals in task 4.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/classify_calls.py` | change | edit_file mode added (same file, second named rule set) |
| `dev_tools/analysis/edit_file_categories.tsv` | new | 3.2 result incl. shape-stat columns |

### Action 3 (subtask 3.3): classify python calls
Write the `python` mode: purpose categories per the Context vocabulary
(bulk-rewrite / audit-mask / transform / json-processing / orchestration /
analysis) assigned by ordered code-shape tests (replace/re.sub presence,
audit/mask/diff keywords, subprocess usage, json load/dump, read-modify-write
residual = analysis). Run, reconcile to 818, write `python_categories.tsv`.
Rationale: the python calls are the scripted half of the same operations the
terminal rows do by hand - their categories must line up with the terminal
ones so task 4 can cite combined counts (e.g. rewrites = 231 python + 51
shell-edit).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/classify_calls.py` | change | python mode added |
| `dev_tools/analysis/python_categories.tsv` | new | 3.3 result |

### Action 4 (subtask 3.4): classify misc calls
Write the `misc` mode: search_conversation -> `conversation-recall`, web_search
-> `web-docs-lookup`; record query-topic notes (what was looked up - crate
APIs, conversation history) as the abstract patterns. Run, reconcile to 117,
write `misc_categories.tsv`. Rationale: small but must be accounted for - the
master table's sum-to-12,596 check requires every kind classified; expected
outcome is "no dedicated tool" which task 4 records as a decision.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/analysis/classify_calls.py` | change | misc mode added |
| `dev_tools/analysis/misc_categories.tsv` | new | 3.4 result |

### Action 5 (subtask 3.5): consolidate into the README section and freeze
Fill `dev_tools/README.md` `## Analysis of tool calls patterns` (replacing the
task-1 placeholder) with: (a) the commands.md format description (the kind/
field table from task 2's Context); (b) the MASTER category table - one row
per category across all kinds: `category, kind, count, share of total,
abstract pattern, implication for dev_tools` - whose counts MUST sum to
12,596 (verify arithmetically before writing); (c) the four per-kind TSVs
embedded as subtables; (d) a closing "what this means for the tool set"
paragraph naming the operation classes that become tools and those that do
not, with reasons. Then FREEZE: no further edits to this section without a plan
correction. Rationale: freezing before task 4 keeps proposal-vs-evidence
auditable - the exit criterion of PHASE_12.md (counts sum to the total) is
checked here, at consolidation.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/README.md` | change (`## Analysis of tool calls patterns` section only) | placeholder replaced by the consolidated master table + subtables + implications paragraph - the frozen evidence base of the phase |

## Task deliverables
- `dev_tools/analysis/classify_calls.py` (four modes, committed).
- Four TSVs: terminal / edit_file / python / misc categories with counts,
  abstract patterns, examples.
- `dev_tools/README.md` section `## Analysis of tool calls patterns` filled and
  frozen: master table summing to 12,596 + per-kind subtables + implications
  paragraph.

## Task exit criteria
- [ ] All four TSVs exist; each kind's category counts sum exactly to that
      kind's call count (9,623 / 2,038 / 818 / 117).
- [ ] The README master table's counts sum to 12,596 (verified arithmetically
      in the status file).
- [ ] Every category row has an abstract pattern and >=1 example (<=160 chars);
      no category was renamed or merged against the Context vocabulary without
      a recorded finding.
- [ ] Distribution deviations from the Context expectations (>5%) are recorded
      as findings with examples.
- [ ] The section is frozen: its content matches the TSVs exactly (no manual
      numbers).

## Non-goals
- No proposal content - `## Proposed tools` stays placeholder until task 4.
- No tool implementation (task 5); classify_calls.py is an analysis artifact,
  not part of the rfs-dev binary yet.
- No changes to commands.md or calls.jsonl (read-only inputs).
