# Phase 12, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_12_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `dev_tools/analysis/classify_calls.py` | the committed four-mode classifier (terminal / edit_file / python / misc) plus a fifth `consolidate` mode added by subtask 3.5 that regenerates the README analysis section from the four TSVs with zero manual numbers - making the freeze mechanically verifiable and the PROTOCOL.md re-run reproducible. Stdlib only; every rule a named function for the task-5.6 Rust port; deterministic ordered rule sets (first match wins) per the stable category vocabulary fixed in this task file's Context |
| `dev_tools/analysis/terminal_categories.tsv` | 3.1 output: 11 categories, counts sum exactly to 9,623 |
| `dev_tools/analysis/edit_file_categories.tsv` | 3.2 output: 5 categories + shape stats as extra columns (single_edit_share, replace_all_count, whole_rewrite_share) + histogram comment lines; sums to 2,038 |
| `dev_tools/analysis/python_categories.tsv` | 3.3 output: 6 purpose categories; sums to 818 |
| `dev_tools/analysis/misc_categories.tsv` | 3.4 output: conversation-recall + web-docs-lookup; sums to 117 |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | calls.jsonl and commands.md are read-only inputs |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `dev_tools/README.md` | changed | `## Analysis of tool calls patterns` filled (task-1 placeholder replaced) and FROZEN: format description table (incl. the widened-fence and three-form web_search findings from task 2), master category table (24 rows, counts sum to 12,596, share-of-total per row, authored implication-for-dev_tools per category), the four per-kind TSVs embedded as subtables, and the closing "what this means for the tool set" paragraph naming the tool-bearing classes and the no-tool classes with reasons. `## Proposed tools` placeholder untouched (task 4) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `TASKS.md` | changed | one entry per executed subtask (3.1, 3.2, 3.3, 3.4) + this close-out entry |

## Changed functionality
- None (analysis infrastructure; all rfs-dev subcommands remain task-1 stubs).

## Removed functionality
- None.

## New functionality
- The complete, frozen evidence base of Phase 12: every one of the 12,596 recorded calls assigned to exactly one category of the stable vocabulary, with abstract patterns, examples, and per-category tooling implications - stored both as machine-readable TSVs (regenerable) and as the README section (human/LLM-readable, byte-derived from the TSVs).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (no committed test) | - | verification is the classifiers' built-in reconciliation (per-kind category sums == kind row counts, master sum == 12,596, exit 1 otherwise) plus the README re-verification from the rendered tables - task 5.6's port-equality test will commit this for the Rust implementation |

## Tests run status

| Name | Status |
|---|---|
| terminal classification reconciliation | EXACT - 9,623 = 11 category counts (2,035+3,376+125+2,126+583+182+32+47+207+207+703) |
| edit_file classification reconciliation | EXACT - 2,038 = 225+720+91+978+24; cross-checks vs task-2 shape data all agree (docs 945, sources 1,069, other 24, new-files 324, histogram identical) |
| python classification reconciliation | EXACT - 818 = 229+68+170+51+26+274 |
| misc classification reconciliation | EXACT - 117 = 83+34 |
| master table arithmetic (re-verified from the rendered README, not the script) | EXACT - 24 rows sum to 12,596; per-kind subtables re-sum to 9,623/2,038/818/117 from the markdown itself |
| `docs_md_test` (5) | OK - all repo docs incl. the new tables pass the mdr-stack gate (no broken pipe tables: 0 row-blank-row spots) |

## Found problems/issues/mismatches
- **3.1 deviations vs plan-Context expectations (>5%), all root-caused as recon artifacts** (planning recon used first-segment-only substring heuristics): python-via-shell 43->207 (recon missed python in later segments of compound commands), lookup-vendored-deps 33->125 (recon counted only var-assignment-led registry reads; the classifier matches .cargo/registry anywhere incl. ~/.cargo and direct grep paths), env-check 110->207 (version probes + toolchain-dir exploration captured; a first-draft regex anchored absolute paths to segment start and missed `find / -name X` - fixed), shell-edit 51->32 (compound rewrite+`&& cargo test` commands classify build-test-run per the documented ordered rule - verified on real payloads, e.g. ordinal 1033), shell-write 208->182 (heredoc-to-python excluded by design), file-ops 58->47, lookup total 5,880->5,536 (recon counted grep-as-output-filter on cargo commands as lookups), other 383->703 (absorbs scratch/system lookups without repo target, job polling, npm internals, installs). git 557->583 (+4.7%) within tolerance.
- **3.2**: no deviations - the distribution matched task-2's measured shape data exactly (it was derived from it).
- **3.3 deviations (>5%), all root-caused row-by-row against the recon rule set**: audit-mask 99->68 (recon's bare-substring 'diff'/'grep' matched prose like "different"; a first-draft classifier over-corrected with \b...\b and missed exactly the underscore-joined audit function names mask_code/audit_src_structure - fixed with an identifier-aware rule: bare word, _name, or call form, but not English words), transform 181->170 (recon's substring 'write' mislabeled 131 read-only scripts as transform while missing 66 pathlib transforms - both errors net to -6.1%), json-processing 56->51 (substring 'json' matched filenames without json-module calls), orchestration 23->26 (documented ordering: subprocess+rewrite scripts classify bulk-rewrite first), analysis 228->274 (absorbs the recon-mislabeled read-only scripts). The 11 scripts defining mask*/audit* helpers but classified bulk-rewrite are CORRECT per the ordered rule (primary operation is writeback + .replace()) - verified, not a miss.
- **3.4**: no deviations (direct kind mapping); expected "no dedicated tool" outcome confirmed for both categories.
- **3.5 consolidator bugs found and fixed during execution**: (1) duplicate `"other"` key in the implications dict silently clobbered the terminal entry - restructured to a (kind, category) keyed table; (2) eager default-argument evaluation raised KeyError on the kind-keyed lookup - two-step lookup; (3) master-table rows appended as separate parts got blank lines between them, terminating the markdown table - assembled as one part; (4) main() derived the output path from the kind name, writing a stray consolidate_categories.tsv for the markdown mode - explicit output argument for consolidate. All four fixed before the section was spliced into the README.

## Other important findings/information
- The stable category vocabulary survived classification without any rename or merge - no new categories were needed and none of the 24 was dropped.
- Combined-count inputs for task 4's proposal (cited in the frozen section): pattern-fragile rewrites = 229 python bulk-rewrite + 32 terminal shell-edit; audit-method lineage = 68 python audit-mask scripts; chunked lookup = 5,536 terminal calls (44% of all terminal); docs<->source reference upkeep is embedded in the 720 docs-modify + 978 sources-modify edit_file calls.
- The `consolidate` mode makes the freeze checkable: re-running it against the four TSVs must reproduce the README section byte-for-byte (the TSVs are the single source of truth; the README carries no numbers of its own).
- Task 4 consumes this section READ-ONLY; any proposal row must cite a category + count from it.
