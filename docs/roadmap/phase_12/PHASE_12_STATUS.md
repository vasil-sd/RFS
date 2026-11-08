# Phase 12 - Implementation status

> Written once when all exit criteria of `PHASE_12.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `dev_tools/src/lib.rs` | Library half of the `rfs-dev` binary: declares the six tool modules (`sync`, `context`, `edit`, `doc_section`, `inventory`, `calls`). Nothing in the rfs build/test path links it. |
| `dev_tools/src/main.rs` | The `rfs-dev` CLI dispatch: the full pinned subcommand surface (task 1), filled subtask by subtask (5.1-5.6). Exit-code convention 0/1/2; `resolve_root` walks up to the repo marker (`tests/dev_docs_manifest.txt`). |
| `dev_tools/src/sync.rs` | 5.1 sync-check / sync-fix: a shared reference-extraction core (check and fix cannot drift) walking `//! Docs:` pointer lines, `tests/dev_docs_manifest.txt`, and backticked source-path refs in the living doc set (docs/dev + docs/llm - exactly the dev_docs_test gate's scope); emits a compact fix-list, exit 1 if any. sync-fix applies `--map OLD=NEW` plan-first with `--dry-run`. |
| `dev_tools/src/context.rs` | 5.2 context: one-command impact context for a fn/type - inventory-row lookup across both structure documents, depth-bounded Depends closure (BFS, cycle/diamond-safe), Documented-in fragment content by anchor, related DECISIONS.md entries; renders a FIXED five-heading markdown template (the output contract). |
| `dev_tools/src/edit.rs` | 5.3 edit-rename / edit-move / edit-replace-item over one syn-based rewrite core (span collection + back-to-front text replacement - only rewritten tokens change bytes). String literals and comments are not Path/Ident nodes so they can never match (the phantom guarantee is structural); `visit_macro` descends macro token streams. |
| `dev_tools/src/doc_section.rs` | 5.4 doc-section: fence-aware ATX heading-path walk printing the addressed section verbatim; `all` searches every docs file with filename prefixes. Line-based (not pulldown-cmark) because the parser exposes no source spans - plan correction 5.4. |
| `dev_tools/src/inventory.rs` | 5.5 inventory add/update/remove/audit: syn-based identifier extraction in the inventories' naming conventions, the completeness audit deriving its conventions from the documents (the Phase 10/11 method, productized), and row ops keeping the six-column tables mechanically valid. Plan correction 5.5 (syn instead of string-masked grep). |
| `dev_tools/src/calls.rs` | 5.6 calls extract / classify: the rule-by-rule Rust port of `analysis/extract_calls.py` + `classify_calls.py`. Hand-rolled JSON writer matching python `json.dumps(ensure_ascii=False)` byte-for-byte + a recursive-descent JSON parser for edits payloads; the four classify modes with the verbatim abstract-pattern dicts; a `cached_regex` helper storing `Arc<Regex>` (a bare `Regex::clone()` deep-copies the automaton - see findings). |
| `dev_tools/tests/sync_test.rs` | 5 integration tests on committed fixture repos (clean/stale/moved): planted stale/missing/orphan cases, dry-run no-write pin, mapping-rewrite-then-clean round trip. |
| `dev_tools/tests/context_test.rs` | 8 integration tests byte-pinning the template contract + closure-depth behavior on a fixture repo (2-deep Depends chain, missing dep, ambiguous name, fragment boundaries, DECISIONS continuation/fenced controls). |
| `dev_tools/tests/edit_test.rs` | 13 integration tests on a fixture mini-crate: every import site rewritten, string/comment phantoms byte-identical after rename AND move, dry-run no-write pins, not-found/ambiguous exit-1, move mod surgery (cross-parent + intermediate-dir creation), replace-item swap with sibling survival. |
| `dev_tools/tests/doc_section_test.rs` | 10 integration tests: verbatim single-heading output, nested-path == direct-addressing bytes, first-occurrence-wins, depth scoping, the `(none)` marker, fenced fake-headings ignored, all-mode cross-file search, no-match/missing-file exit 1, usage exit 2. |
| `dev_tools/tests/inventory_test.rs` | 5 integration tests: audit EXACT on a clean fixture tree, planted stale+missing detection (exit 1), add->update->remove round-trip restoring the table byte-for-byte, dry-run writes nothing. |
| `dev_tools/tests/calls_test.rs` | 4 port-equality integration tests: extract + classify byte-identical to the python oracle on the committed fixture AND the real 12,596-call `commands.md`. Availability-gated on `python3`; a shared lock serializes the TSV-backup guard so parallel runs cannot clobber the committed analysis TSVs. |
| `dev_tools/tests/dispatch_test.rs` | Binary-level dispatch contract tests (unknown subcommand, grouped-subverb validation, usage errors). Lost its pinned-stub-message test when 5.6 filled the last stubs. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` (repo root) | changed | `[workspace] members` gained `dev_tools` (task 1) - the crate is a workspace member so `cargo test --workspace`, `clippy --all-targets` and `fmt --check` cover it automatically. |
| `.gitignore` | changed | Added `dev_tools/analysis/calls.jsonl` (the regenerable ~12,596-row inventory; only the compact TSVs + scripts are committed). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `dev_tools/README.md` | new | Both sections filled: `## Analysis of tool calls patterns` (the frozen master category table summing to 12,596 + per-kind subtables + the "what this means for the tool set" paragraph) and `## Proposed tools` (the Tool name / CLI / Purpose / rationale table matching what is implemented, + the no-tool decisions + the DOCS_STRUCTURE.md REJECTED decision). |
| `dev_tools/PROTOCOL.md` | new | The out-of-scope companion written after planning: the protocol for FUTURE analysis + tool-set update on new `commands.md` versions (references tasks 2-4 as the procedure to repeat). Not part of this phase's task set. |
| `BUILD.md` | changed | Section 3: workspace row 559->663 + a new `-p rfs-dev` suite row (104). Section 4: a dev_tools build/test/run command row. Section 5: a "touched dev_tools" checklist bullet + the known-good baseline re-measured at Phase 12 close-out (663 workspace). |
| `START.md` | changed | Section 1 snapshot: added the missing Phase 10/11 completion lines + the Phase 12 line, and re-measured the baseline (663 workspace, 535 root unchanged). Section 3 source tree: a `dev_tools/` map line. |
| `KB.md` | changed | Section 4.2 file map: a `dev_tools/` row describing the crate, binary, six tools, tests/fixtures, analysis oracle, and dep set (edited by grep+sed-range, never read fully). |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase 12 row -> COMPLETE with close-out date + full file list. |
| `TASKS.md` | changed | Close-out entry appended per its first-paragraph template (append-only). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `dev_tools/tests/fixtures/sync/{clean,stale,moved}/` | new | Committed synthetic mini-repos for the sync tests (2 .rs files with `//! Docs:` lines, a manifest, docs with backticked refs - one stale in the `stale` tree). |
| `dev_tools/tests/fixtures/context/` | new | Mini inventories (a 2-deep Depends chain) + doc files carrying the fragments + a DECISIONS.md, for the context tests. |
| `dev_tools/tests/fixtures/edit/` | new | A mini crate (lib.rs + alpha.rs + beta.rs with cross-imports) where one file carries the rename pattern inside a string literal AND a comment - the phantom negative control. |
| `dev_tools/tests/fixtures/docs/` | new | Two synthetic docs with overlapping heading names at different depths, for the doc-section tests. |
| `dev_tools/tests/fixtures/inventory{,_stale}/` | new | A mini source tree + matching inventories (audit-clean EXACT) and a stale variant (one phantom row + one un-inventoried fn). |
| `dev_tools/tests/fixtures/calls/commands.md` | new | The 31-row synthetic equality corpus: all five kinds, both payload forms, a widened four-backtick fence, the `foo.rs.rs` / `mysrc/foo` edge cases, web_search's three forms, one payload-less terminal. |
| `dev_tools/analysis/extract_calls.py`, `classify_calls.py` | new | The committed python oracle (task 2 extractor + task 3 four-mode classifier). Stdlib-only; the port-equality reference for `calls`. |
| `dev_tools/analysis/{terminal,edit_file,python,misc}_categories.tsv` | new | The four committed category TSVs (the machine-readable source of the README analysis section; regenerable by the classifier). |
| `docs/roadmap/phase_12/PHASE_12.md`, `PHASE_12_PLAN.md`, `tasks/PHASE_12_TASK_{1..6}.md` (+ `_STATUS` for 1-5) | new | The phase description, the implementation plan (with the 5.4/5.5/5.6 plan corrections), and the per-task files + status reports - the phase's own bookkeeping. |

## Changed functionality

None in the `rfs` package. dev_tools is maintenance-only; nothing in the rfs build/test/codegen path invokes it (PHASE_12.md exit criterion). The root `rfs` package is byte-for-byte untouched - its test count stayed at exactly 535 and `--features smt` at 537.

## Removed functionality

None.

## New functionality

- The `rfs-dev` binary (the `dev_tools/` workspace member) - six maintenance tools derived from an evidence-based analysis of the 12,596 recorded assistant tool calls in `commands.md`:
  - **sync-check / sync-fix** - docs<->source reference re-syncing: walks `//! Docs:` pointers, the manifest, and backticked source-path refs in the living doc set; emits a compact fix-list (exit 1 if any) / applies `--map OLD=NEW` plan-first with `--dry-run`. Replaces the Phase 11 task-10 scripted path mapping.
  - **context** - one-command impact context for a fn/type: inventory row + depth-bounded Depends closure + Documented-in fragment content + related DECISIONS.md entries, rendered as a fixed markdown template. Operationalizes the PROTOCOL.md section-4 impact rule.
  - **edit-rename / edit-move / edit-replace-item** - syn-parsed source edits immune to string/comment pattern phantoms (the Phase 10 template-phantom failure class); plan-first + `--dry-run`.
  - **doc-section** - retrieve any doc's content by heading path (`all` searches every docs file); the token-saving replacement for chunked sed/head/tail/dd reads.
  - **inventory add/update/remove/audit** - structure-inventory row maintenance + the string-masked (syn-based) completeness check, productized from the Phase 10/11 close-out method.
  - **calls extract / classify** - the Rust port of the analysis pipeline, byte-identical to the committed python oracle; makes every `dev_tools/PROTOCOL.md` future re-run one command pair.
- CLI conventions fixed for all: compact structured stdout, exit 0/1/2, `--root` defaulting to the repo root, plan-first + `--dry-run` for mutating tools.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `dev_tools/tests/{sync,context,edit,doc_section,inventory,calls}_test.rs` | new | One integration test file per tool on committed fixtures (5+8+13+10+5+4 = 45 integration tests). |
| `dev_tools/tests/dispatch_test.rs` | new (task 1) / changed (5.6) | Binary-level dispatch contract tests; the pinned-stub-message test removed when the last stubs were filled. |
| `dev_tools/src/*.rs` in-module `#[cfg(test)]` | new | 52 lib unit tests across the six tool modules (incl. 10 for calls). |

## Tests run status

| Name | Status |
|---|---|
| `cargo build --workspace` | OK |
| `cargo test` (root rfs, 535) | OK - UNCHANGED from the pre-Phase-12 baseline (proof nothing in the rfs package was touched). |
| `cargo test --features smt` (537) | OK - UNCHANGED. |
| `cargo test --workspace` (663) | OK - the 559 pre-dev_tools baseline + rfs-dev's 104 tests, exactly. |
| `cargo test -p rfs-dev` (104) | OK - 52 lib unit + 4 dispatch unit + 4 calls + 8 context + 3 dispatch + 10 doc-section + 13 edit + 5 inventory + 5 sync integration, 0 doc-tests. |
| `cargo clippy --all-targets` (46 warning lines) | OK - exactly the known-good baseline (all pre-existing in rfs lib + tests/common); ZERO in any dev_tools file. |
| `cargo fmt --check` | OK - clean workspace-wide. |
| `cargo test --test docs_md_test` (5) / `--test dev_docs_test` (9) | OK - the doc-quality + code<->doc sync gates pass with the new status file and doc edits. |

## Found problems/issues/mismatches

- **Task 3 analysis deviations from plan-Context expectations (>5%), all root-caused as recon artifacts** (the planning recon used first-segment-only substring heuristics): python-via-shell 43->207, lookup-vendored-deps 33->125, env-check 110->207, shell-edit 51->32, shell-write 208->182, lookup total 5,880->5,536, other 383->703 (terminal); audit-mask 99->68, transform 181->170, json-processing 56->51, orchestration 23->26, analysis 228->274 (python). git 557->583 (+4.7%) within tolerance. edit_file and misc matched exactly. The stable category vocabulary survived with no rename or merge.
- **Plan correction 5.4**: pulldown-cmark 0.13.6's event API exposes no source spans, so doc-section uses a line-based fence-aware heading walk (matching context.rs + sync-check) instead of the planned pulldown-cmark walk. No new dependency.
- **Plan correction 5.5**: the inventory audit uses syn parsing instead of the planned string-masked grep - strictly more robust (cannot confuse a string literal's contents for an identifier) and reproduces the Phase 10/11 conventions exactly (proven by the EXACT demonstration). No new dependency.
- **Plan correction 5.6**: `regex` added as the third declared dependency exception (user-authorized) for the rule-by-rule port of the python scripts' ~15 `re` patterns. Already in Cargo.lock transitively - no new lock entry, offline-safe. The one backreference+lookahead pattern uses a regex-open + same-width line-scan hybrid.
- **`Regex::clone()` deep-copies the compiled automaton (performance, found in 5.6)**: the first port cached patterns by cloning the `Regex`, assuming cheap Arc-like clones - they are not. This made extract ~26x and terminal classify ~160x slower than the python oracle. Fixed by caching `Arc<Regex>`; output was always correct, only wall-clock gave it away.
- **The calls TSV backup guard deleted the committed analysis TSVs under parallel test runs (found in 5.6)**: two classify tests backed up to the same `.bak` names and the second drop took the delete branch. Fixed by serializing on a shared lock; TSVs regenerated byte-identical.

## Other important findings/information

- **DOCS_STRUCTURE.md decision: REJECTED for this phase.** From the frozen data, only 17.2% of the 2,035 lookup-docs calls mention two or more distinct doc files in one command, and those are directory listings / size checks / cross-reference greps - NOT heading-name hunts across files. The dominant patterns are single-file range reads against already-known line numbers (32.5%) and intra-file heading greps (13.2%). `doc-section`'s on-the-fly walk + `all` mode covers the observed need without a prebuilt index that would itself need re-syncing. Revisit under `dev_tools/PROTOCOL.md` if a future commands.md shows frequent cross-doc heading hunts.
- **Port equality holds on both corpora**: the Rust `calls` output is byte-identical to the python oracle for extract + all four classify kinds, on the committed fixture and the real 12,596-call file. The demonstration reproduced task 2's reconciliation exactly (TOTAL 12596) and task 3's terminal TSV byte-for-byte.
- **Every tool was demonstrated on a real maintenance operation** (the phase's exit criterion made executable): sync-check CLEAN on the live tree; context yielding the populated 776-line template for `parse_source`; edit-rename/edit-move/edit-replace-item each followed by a rustc compile of a /tmp copy of a real module (exit 0); doc-section byte-equal to the sed range read it replaces; inventory audit EXACT (SRC 936/196, TEST 942/34 - matching the Phase 11 close-out); calls reproducing the analysis. Full command + outcome records are in `tasks/PHASE_12_TASK_5_STATUS.md`.
- **The dependency constraint held with one authorized exception**: dev_tools' deps are syn + pulldown-cmark (the two planning-declared exceptions) + regex (the user-authorized third). Nothing else was added; the rfs package's dep set is untouched.
- **Phase 12 is COMPLETE.** All six exit criteria of `PHASE_12.md` are met: the dep constraint, the analysis section summing to 12,596, the proposed-tools table matching the implementation, per-tool demonstrations on real operations, full verification green (dev_tools included), and no rfs toolchain functionality/CLI/codegen change. The only follow-on is `dev_tools/PROTOCOL.md` future re-runs under their own phases.
