# Phase 12, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_12_TASK_5.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| `dev_tools/src/calls.rs` | The Rust port of `analysis/extract_calls.py` + `classify_calls.py`, rule-by-rule (subtask 5.6): the extract half (marker split, fenced/inline payload extraction with a regex-open + same-width line-scan hybrid for the one backreference+lookahead pattern, edit_file edits-JSON parsing, path-class labelling, the reconciliation summary) and the classify half (all four modes - terminal's ordered rule set incl. the five-alternative PATH_MENTION_RE, edit_file's nine-column TSV + histogram footer, python's six code-shape tests, misc's kind mapping - plus the four verbatim abstract-pattern dicts). Hand-rolled JSON writer matching python `json.dumps(ensure_ascii=False)` byte-for-byte and a recursive-descent JSON parser for edits payloads. A `cached_regex` helper stores `Arc<Regex>` so compiled patterns are reused (a bare `Regex::clone()` deep-copies the automaton - see findings). 10 in-module unit tests. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `dev_tools/src/lib.rs` | changed | Added `pub mod calls;` and noted it in the crate doc comment (the other five tool modules were already declared by subtasks 5.1-5.5). |
| `dev_tools/src/main.rs` | changed | Replaced the two `calls extract` / `calls classify` pinned stubs with real dispatch to `run_calls_extract` / `run_calls_classify` (CLI: `calls extract <commands.md> --out FILE`, `calls classify <calls.jsonl> --kind K [--out FILE]`; exit 0/1/2 per convention). Updated the in-module dispatch unit test - the "every unimplemented subcommand is a stub" assertion became a usage-error check now that no stubs remain. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `dev_tools/README.md` | unchanged | The `## Proposed tools` CLI column already matched the shipped `calls extract` / `calls classify` surface exactly, so no correction was needed (the task file allows a README change ONLY if implementation reveals a better flag shape - it did not). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `dev_tools/Cargo.toml` | changed | Added `regex = "1"` as the third declared dependency exception (user-authorized; plan + task correction recorded) with a comment explaining it is already in Cargo.lock transitively (no new lock entry, offline-safe) and why the one backreference+lookahead pattern uses a hybrid. |
| `dev_tools/tests/fixtures/calls/commands.md` | new | The committed synthetic equality corpus: 31 entries covering all five kinds, both payload forms (inline-backticked + fenced), a widened four-backtick fence (payload contains triple backticks), the `foo.rs.rs` greedy-backtrack and `mysrc/foo` no-word-boundary edge cases, web_search's three recorded forms (query / url / image_queries), and one payload-less terminal entry (the unparsed case). |
| `docs/roadmap/phase_12/PHASE_12_PLAN.md` | changed | "Plan corrections" table completed: the 5.4 (pulldown-cmark has no source spans -> line-based fence-aware walk) and 5.5 (syn parsing instead of string-masked grep) rows that their sessions noted only in TASKS.md, plus the 5.6 `regex` dependency-exception row - all three now recorded here per PROTOCOL.md step o so the plan stays analyzable. |
| `docs/roadmap/phase_12/tasks/PHASE_12_TASK_5.md` | changed | "Task corrections" table: added the 5.6 `regex` dependency-exception row (the two 5.1 rows - sync doc-scope narrowing and anchor fence-tracking - were already present). |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase 12 row updated: subtasks 5.1-5.6 implemented, only the 5.x final-verification subtask remains (now complete with this status file). |

## Changed functionality

None. dev_tools is maintenance-only; nothing in the `rfs` build/test/codegen path invokes it (PHASE_12.md exit criterion). The root `rfs` package is byte-for-byte untouched - its test count stayed at exactly 535.

## Removed functionality

None. (The two `calls` dispatch stubs were replaced by real implementations, not removed - the subcommand surface is unchanged.)

## New functionality

- `rfs-dev calls extract <commands.md> --out FILE` - the Rust port of `extract_calls.py`: parses a `commands.md` tool-call log into one JSON row per call (the fixed schema: ordinal, kind, payload, paths, path_classes, unparsed, plus n_edits/is_new_file for edit_file) and prints the per-kind parsed/unparsed/total reconciliation. Output is byte-identical to the python oracle.
- `rfs-dev calls classify <calls.jsonl> --kind <terminal|edit_file|python|misc>` - the Rust port of `classify_calls.py`: classifies an inventory into operation categories and writes the `<kind>_categories.tsv` (the stable category vocabulary, the authored abstract patterns, the examples, and - for edit_file - the single_edit_share / replace_all_count / whole_rewrite_share columns + histogram footer). Output is byte-identical to the python oracle.
- Together these make every `dev_tools/PROTOCOL.md` future re-run on a new `commands.md` a single command pair instead of committed-python-plus-manual-runs (the phase's stated purpose for this tool).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `dev_tools/tests/calls_test.rs` | new | The port-equality integration suite (4 tests): extract byte-equality on the fixture AND on the real 12,596-call `commands.md`; classify byte-equality for all four kinds on the fixture and for terminal on the real corpus. The python pair is the oracle; the tests are availability-gated (skip if `python3` is absent) and serialize their TSV-backup guard on a shared lock so parallel runs cannot clobber the committed analysis TSVs. |
| `dev_tools/src/calls.rs` (in-module `#[cfg(test)]`) | new | 10 unit tests pinning individual ported rules: path_class labelling, collapse truncation, json_escape vs python ensure_ascii=False, parse_json round-trip + invalid/non-array rejection, extract_fenced width-matched close (incl. the widened fence), extract_inline, segments env-prefix stripping, is_close_fence, and the ordered classify_terminal / classify_python rule sets. |
| `dev_tools/tests/dispatch_test.rs` | changed | Removed the `every_unimplemented_subcommand_is_a_stub_with_the_pinned_message` test - it asserted the `calls` stubs' "not implemented yet" message, which no longer exists now that 5.6 filled them. Replaced with a comment noting all subcommands are implemented and their contracts live in the per-tool test files. |

## Tests run status

| Name | Status |
|---|---|
| `cargo build --workspace` | OK |
| `cargo test --workspace` (663) | OK - rfs root 535 (UNCHANGED from baseline) + dev_tools 104 + async_handler 1 + rfs_adapters 12 + rfs_build 10 + sync_handler 1 + fuzz_rust 0. The workspace total grew by exactly dev_tools' 104 tests over the 559 pre-dev_tools baseline. |
| `cargo test -p rfs-dev` (104) | OK - 52 lib unit + 4 main-dispatch unit + 4 calls_test + 8 context_test + 3 dispatch_test + 10 doc_section_test + 13 edit_test + 5 inventory_test + 5 sync_test integration, 0 doc-tests. |
| `cargo clippy --all-targets` (46 warning lines) | OK - exactly the known-good baseline (46 lines / 35 unique, all pre-existing in `rfs` lib + `tests/common/{prod_driver,mc_json,fail_variants}.rs`); ZERO warnings in any dev_tools file. |
| `cargo fmt --check` | OK - clean across the whole workspace. |

## Found problems/issues/mismatches

- **`Regex::clone()` deep-copies the compiled automaton (performance).** The first port cached compiled patterns by cloning the `Regex` out of a map, assuming clones were cheap (Arc-like). They are not - the regex crate copies the whole DFA on clone. This made extract ~26x slower than the python oracle (15.8s vs 0.6s on the real file) and terminal classify ~160x slower (91s vs 0.58s), because every one of the 12,596 entries re-cloned the expensive automaton. Fixed by caching `Arc<Regex>` and cloning the cheap pointer instead; extract dropped to ~5.9s and terminal classify to ~0.78s (on par with the oracle). The port-equality test would have passed either way (output was always correct) - only the wall-clock gave it away.
- **The TSV backup guard deleted the committed analysis TSVs under parallel test runs.** `calls_test.rs` backs up the four `analysis/*_categories.tsv` files before the python classifier overwrites them and restores them on drop. Two classify tests ran concurrently, both backing up to the SAME `.bak` names; the first drop renamed its `.bak` back, the second found no `.bak` and took the "else delete the live file" branch - deleting the committed TSVs. Fixed by serializing the guard-using tests on a shared `Mutex` (which also prevents two python processes writing the same live file at once) and regenerating the four TSVs from the real `calls.jsonl` (byte-identical to before, since same input + same script). The guard now provably leaves the tree as it found it.

## Other important findings/information

- **Port equality holds on both corpora.** The Rust output is byte-identical to the python oracle's for extract AND all four classify kinds, on the committed fixture (31 rows) and on the real 12,596-call `commands.md`. This is the subtask's acceptance test and it passes.
- **Demonstrations on real operations (the phase's exit criterion, one per tool):**
  - 5.1 sync: `rfs-dev sync-check` on the real tree -> exit 0, empty fix-list = CLEAN (the tree is in sync after Phase 11; no findings).
  - 5.2 context: `rfs-dev context parse_source` -> exit 0, a 776-line populated template with all 7 depth-1 dependencies resolved (zero missing markers), 8 fragments extracted across docs/dev + docs/llm, related decisions 9.1 + 9.5; `--depth 2` expands correctly; `context new` -> exit 1 ambiguous listing all 21 matching rows; `context new --file src/lang/ir.rs` -> the Ir::new row with a pinned `(none)` empty closure.
  - 5.3 edit-*: on /tmp copies of the REAL `src/utils/hash.rs` + `rng.rs`, each followed by a rustc compile of the whole copied crate (exit 0): `edit-rename sha256_hex sha256_digest` rewrote the declaration + 8 references incl. 5 inside `assert_eq!`/`assert!` macro args while the string literal carrying the old name survived byte-identical; `edit-move src/utils/rng.rs src/utils/randgen/rng.rs` did the cross-parent mod surgery (removed the old declaration, created randgen/mod.rs, added `pub mod randgen;`, rewrote the import site); `edit-replace-item src/checksum.rs digest --new-from new_digest.txt` swapped the fn (doc comment included) leaving its sibling untouched.
  - 5.4 doc-section: on a real KB.md/DESIGN.md section, returned exactly that section's content - byte-identical to the `sed -n '<a>,<b>p'` range read it replaces (the token-saving substitution for chunked doc reads).
  - 5.5 inventory: `rfs-dev inventory audit` on the real tree -> exit 0, "EXACT: SRC 936 fns / 196 types, TEST 942 fns / 34 types - zero per-file diffs" - the counts match the Phase 11 close-out record exactly, proving the syn-based reconstruction is faithful and the audit no longer needs to be done by hand.
  - 5.6 calls: `rfs-dev calls extract commands.md --out ...` reproduced task 2's reconciliation exactly (edit_file 2038 / python 818 / search_conversation 83 / terminal 9621+2 unparsed / web_search 34 / TOTAL 12596); `rfs-dev calls classify <inv> --kind terminal` reproduced task 3's terminal TSV byte-for-byte (lookup-docs 2035, lookup-sources 3376, lookup-vendored-deps 125, build-test-run 2126, git 583, shell-write 182, shell-edit 32, file-ops 47, env-check 207, python-via-shell 207, other 703).
- **The `regex` dependency exception is lock-neutral.** `regex 1.13.1` was already in Cargo.lock transitively (pulled by other crates), so declaring it directly on rfs-dev added no new crate entry - only the `rfs-dev` package's own lock block (its first build) appeared, listing regex among its deps. Offline rebuilds keep working.
- **Task 5 is now COMPLETE** (all six tool subtasks 5.1-5.6 + the 5.x final verification). The only remaining Phase 12 work is task 6 (close-out: record the new baselines in BUILD.md/START.md/KB.md, write PHASE_12_STATUS.md, update CURRENT_STATE to COMPLETE, append the TASKS.md entry).
