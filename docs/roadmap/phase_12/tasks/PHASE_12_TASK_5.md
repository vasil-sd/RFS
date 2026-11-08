# Phase 12, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Implement every tool in the finalized `## Proposed tools` table (task 4) as a
module of the `rfs-dev` binary, each with unit tests, an integration test on a
fixture tree, and a DEMONSTRATION on a real maintenance operation recorded in
the status file - then run full verification with dev_tools in the workspace.
Rationale: per-tool subtasks keep each implementation session bounded and
independently verifiable; the demonstration-on-real-operation is the phase's
exit criterion made executable (PHASE_12.md: "each tool is exercised on a real
maintenance operation as its acceptance test").

## Context
- The AUTHORITATIVE tool list is `dev_tools/README.md` section
  `## Proposed tools` as finalized by task 4. Expected subtasks (the table may
  differ - deltas are plan corrections, and the subtask numbering follows the
  table's row order): 5.1 sync-check/sync-fix, 5.2 context, 5.3
  edit-rename/edit-move/edit-replace-item, 5.4 doc-section (+ DOCS_STRUCTURE.md
  if task 4 decided yes), 5.5 inventory add/update/remove/audit, 5.6 calls
  extract/classify; 5.x = final verification.
- Implementation ground rules:
  - Each tool is a module `dev_tools/src/<tool>.rs` declared in `lib.rs`; the
    dispatch stub body from task 1 is replaced, the CLI surface (names, flags,
    exit codes) stays exactly as task 4's table says.
  - Rust-source parsing uses `syn` (full feature): edit-* tools parse, never
    text-match - a rename must not touch string literals or comments containing
    the pattern (Phase 10's template-phantom failure class). `context` and
    `inventory audit` extract fn/type identifiers with syn too.
  - Markdown uses `pulldown-cmark`: doc-section walks headings; sync-check
    extracts backticked references. If a mutable GFM AST proves necessary,
    switch to `comrak` (already in the lock file) and record a plan correction
    (plan Context fallback).
  - Mutating tools (`sync-fix`, `edit-*`, `inventory add/update/remove`) print
    the exact edit plan before applying and support `--dry-run`; they operate
    on `--root` (default: repo root).
  - Tests: unit tests in-module; ONE integration test file per tool under
    `dev_tools/tests/` exercising the binary end-to-end on fixtures under
    `dev_tools/tests/fixtures/` - small synthetic trees (mini src/ + docs/ +
    inventories) so tests NEVER mutate the real repo tree. Fixtures are
    committed.
  - Demonstrations run against the REAL tree: read-only tools (sync-check,
    context, doc-section, inventory audit, calls extract/classify) directly;
    mutating tools on a scratch copy under /tmp (e.g. edit-rename on a copied
    module, then compile the copy with rustc to prove import-site rewrites
    work). Each demonstration's command + outcome is recorded in the status
    file.
  - The string-masked audit (5.5) re-implements the Phase 10/11 close-out
    method: extract every fn/type identifier from source with strings masked,
    diff against inventory rows per file; conventions from
    PHASE_11_TASK_11_STATUS.md findings (marked-form derivation, mod-scope
    handling, nested-type naming, associated-type phantoms, raw identifiers).
    Exit criterion: EXACT - zero per-file diffs on the current tree.
  - `calls extract/classify` (5.6) ports `dev_tools/analysis/extract_calls.py`
    + `classify_calls.py` rule-by-rule to Rust; its output on the current
    commands.md must reproduce task 2's counts and task 3's TSVs exactly
    (that equality IS the port's test).
- Dependency constraint: nothing beyond syn/proc-macro2/pulldown-cmark may be
  added (PHASE_12.md exit criterion); a needed capability that implies a new
  dep is a plan correction, not a silent addition.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Scaffold the dev_tools crate | dispatch stubs to fill, dep set, fixture-free test harness conventions |
| 4. | Elaborate the proposed tools table | the authoritative tool list + CLI shapes + the DOCS_STRUCTURE.md decision |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Close-out | verifies this task's work in the full workspace run and records new baselines |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/src/lib.rs` | change | module declarations for every implemented tool |
| `dev_tools/src/main.rs` | change | stub bodies replaced by real dispatch to the tool modules |
| `dev_tools/src/sync.rs` | new | 5.1 sync-check / sync-fix |
| `dev_tools/src/context.rs` | new | 5.2 context extraction |
| `dev_tools/src/edit.rs` | new | 5.3 edit-rename / edit-move / edit-replace-item |
| `dev_tools/src/doc_section.rs` | new | 5.4 doc-section (+ DOCS_STRUCTURE.md builder if decided) |
| `dev_tools/src/inventory.rs` | new | 5.5 inventory add/update/remove/audit |
| `dev_tools/src/calls.rs` | new | 5.6 calls extract/classify (Rust port of the two analysis scripts) |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/tests/sync_test.rs` | new | 5.1 integration test on fixtures |
| `dev_tools/tests/context_test.rs` | new | 5.2 integration test on fixtures |
| `dev_tools/tests/edit_test.rs` | new | 5.3 integration test on fixtures (incl. string/comment phantom negative controls) |
| `dev_tools/tests/doc_section_test.rs` | new | 5.4 integration test on fixtures |
| `dev_tools/tests/inventory_test.rs` | new | 5.5 integration test on fixtures (audit EXACT on a fixture tree) |
| `dev_tools/tests/calls_test.rs` | new | 5.6 port-equality test: Rust output == python script output on the same input |
| `dev_tools/tests/fixtures/**` | new | committed synthetic trees per tool (mini src/docs/inventories/commands.md sample) |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `dev_tools/README.md` | change | ONLY if implementation reveals a better flag shape - via plan correction, CLI column corrected to match the shipped binary |

## Planned actions

### Action 1 (subtask 5.1): implement sync-check / sync-fix
Per the task-4 row: walk `//! Docs:` pointer lines in every manifest-listed
`.rs` file, `tests/dev_docs_manifest.txt`, and every backticked source-path
reference in `docs/**/*.md`; check each against the actual tree; emit a compact
fix-list (stale/missing/orphan, one line each) on stdout, exit 1 if any,
0 if clean. `sync-fix` applies the fix-list (plan-first, `--dry-run`). Unit
tests on fixture trees with planted stale pointers; integration test runs the
binary over a fixture repo. Demonstration: `sync-check` on the REAL current
tree - expected outcome CLEAN (the tree is in sync after Phase 11); any hit is
a real finding to record, not to auto-fix. Rationale: this is the Phase 11
task-10 operation productized - the demonstration proves it reproduces a known
correct result on the live tree.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/sync.rs` | new | checker + fixer sharing one reference-extraction core - check and fix cannot drift apart |
| `dev_tools/tests/sync_test.rs` | new | fixture-tree integration test with planted stale/missing/orphan cases |
| `dev_tools/tests/fixtures/sync/**` | new | mini repo: 2 .rs files with //! Docs: lines, a manifest, 2 docs with backticked refs (one stale) |

### Action 2 (subtask 5.2): implement context
Per the task-4 row: given a fn/type name (+ optional file), pull from
SRC_STRUCTURE.md / TEST_STRUCTURE.md the row itself, its Depends closure
(depth-bounded, default 1), the Documented-in fragments' content (extracted
from the named docs by fragment anchor), and related DECISIONS.md entries -
rendered as a FIXED markdown template (the template is part of the output
contract: `## Context for <name>` / `### Inventory row` / `### Depends closure`
/ `### Documented in` / `### Related decisions`). Unit tests on fixture
inventories; integration test asserts the template shape byte-for-byte on a
fixture name. Demonstration: `context` for a real function (e.g. one from
`src/lang/parser.rs`) yields the populated template. Rationale: operationalizes
the PROTOCOL.md section-4 impact rule as one command - replaces the
lookup-sources grep+sed+read sequence.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/context.rs` | new | inventory-row lookup + closure walk + fragment/decision extraction + fixed template renderer |
| `dev_tools/tests/context_test.rs` | new | template-shape byte-pins + closure-depth behavior on fixtures |
| `dev_tools/tests/fixtures/context/**` | new | mini inventories (2-3 rows with a 2-deep Depends chain) + a doc file carrying the fragments |

### Action 3 (subtask 5.3): implement edit-rename / edit-move / edit-replace-item
Per the task-4 row: syn-parsed edits over `--root`'s Rust sources.
`edit-rename <old> <new>`: rename a fn/type across all files, rewriting use-
declarations and path references, leaving string literals and comments
touching the pattern UNTOUCHED (phantom negative control). `edit-move <from>
<to>`: move a file and rewrite every import site to the new path. `edit-
replace-item <file> <item>`: replace one item by name with stdin/new-file
content. All three: plan-first output, `--dry-run`, exit 1 if the symbol is not
found or is ambiguous. Unit tests per operation on fixture sources; integration
tests include the phantom negative control (a string literal containing the
old name must survive). Demonstration: `edit-rename` on a /tmp copy of a real
module - all import sites rewritten, the copy compiles with rustc. Rationale:
replaces the 231+51 pattern-fragile rewrite calls; the compile-after-rename
demonstration is the proof the rewrites are semantically complete.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/edit.rs` | new | three operations over one syn-based rewrite core (visit-mut over use-trees + path expressions) |
| `dev_tools/tests/edit_test.rs` | new | per-operation fixture tests + string/comment phantom negative controls + ambiguity/missing-symbol error pins |
| `dev_tools/tests/fixtures/edit/**` | new | mini crate: 3 files with cross-imports, one file carrying the pattern inside a string literal and a comment |

### Action 4 (subtask 5.4): implement doc-section (+ DOCS_STRUCTURE.md if task 4 decided yes)
Per the task-4 row: `doc-section <file|all> <heading...>` prints the content of
the section addressed by the heading path (pulldown-cmark heading walk;
`all` searches every docs file and prefixes matches with the file name). If
task 4 decided to build `DOCS_STRUCTURE.md`: generate it as a section-level
inventory of the docs tree (one row per ## / ### section: file, heading path,
line range, one-line summary from the first paragraph) with a `--check` mode
that reports drift. Unit tests on fixture docs (nested headings, same heading
in two files); integration test pins the output shape. Demonstration:
`doc-section` for a real section of KB.md or DESIGN.md returns exactly that
section's content - the token-saving replacement for sed-range reads. Rationale:
replaces the ~4,480 lookup-docs chunked reads with one structured command.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/doc_section.rs` | new | heading-path walker + cross-doc search (+ DOCS_STRUCTURE.md generator/checker iff task 4 decided yes) |
| `dev_tools/tests/doc_section_test.rs` | new | nested-heading, duplicate-heading-across-files, and missing-section error pins on fixtures |
| `dev_tools/tests/fixtures/docs/**` | new | two fixture docs with overlapping heading names at different depths |

### Action 5 (subtask 5.5): implement inventory add/update/remove/audit
Per the task-4 row: row operations on SRC_STRUCTURE.md / TEST_STRUCTURE.md
keeping the tables mechanically valid (column count, pipe escaping, sort order
per the documents' conventions); `audit` re-runs the string-masked completeness
check (Phase 10/11 method, conventions from PHASE_11_TASK_11_STATUS.md
findings) and reports per-file diffs, exit 1 on any diff. Unit tests on fixture
inventories (add/update/remove round-trips + a planted stale row); integration
test runs `audit` on a fixture tree to EXACT. Demonstration: `inventory audit`
on the REAL current tree - expected EXACT (zero per-file diffs, as Phase 11
left it); any diff is a real finding. Rationale: productizes the audit that
Phase 11 had to reconstruct by hand - the demonstration proves the
reconstruction is no longer needed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/inventory.rs` | new | table row operations + the string-masked audit (masker conventions ported from the Phase 10/11 method) |
| `dev_tools/tests/inventory_test.rs` | new | round-trip row ops + planted-stale-row detection + audit-EXACT on a fixture tree |
| `dev_tools/tests/fixtures/inventory/**` | new | mini source tree + matching inventory document (audit-clean) plus one deliberately stale variant |

### Action 6 (subtask 5.6): implement calls extract/classify (Rust port)
Port `dev_tools/analysis/extract_calls.py` and `classify_calls.py` rule-by-rule
to Rust: `calls extract <commands.md> --out calls.jsonl` (same row schema,
same reconciliation summary) and `calls classify <calls.jsonl> --kind
<terminal|edit_file|python|misc>` (same category vocabulary, same TSV output).
Unit tests on a fixture commands.md sample; integration test = PORT EQUALITY:
run the python scripts and the Rust tool over the same input, assert byte-
equal JSONL/TSV output (the current 12,596-call file is the equality corpus).
Demonstration: `calls extract` + `calls classify --kind terminal` on the real
commands.md reproduces task 2's counts and task 3's terminal TSV. Rationale:
this is what makes dev_tools/PROTOCOL.md future re-runs a single command pair
instead of committed-python-plus-manual-runs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/calls.rs` | new | Rust port of both analysis scripts - rule-by-rule, so the python pair stays the oracle until it is retired |
| `dev_tools/tests/calls_test.rs` | new | port-equality test: python output vs Rust output byte-identical on the fixture sample AND the real commands.md |
| `dev_tools/tests/fixtures/calls/**` | new | a small synthetic commands.md (all five kinds, both payload forms) as the fast equality corpus |

### Action 7 (subtask 5.x): full verification with dev_tools in the workspace
Run `cargo build && cargo test --workspace && cargo clippy --all-targets &&
cargo fmt --check` (toolchain activation per BUILD.md section 2). Record exact
test counts (root package count must be UNCHANGED - dev_tools adds its own;
workspace count grows by exactly dev_tools' tests). Any clippy/fmt finding in
dev_tools is fixed here, not deferred. Rationale: the phase exit criterion
"full verification green - dev_tools included" is checked once, at the end,
with the numbers task 6 will record as the new baselines.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `dev_tools/src/*.rs`, `dev_tools/tests/*.rs` | change | clippy/fmt fixes only - no behavior changes after the per-tool subtasks passed their own tests |

## Task deliverables
- Every tool of the task-4 table implemented in `rfs-dev` with its CLI exactly
  as proposed.
- Unit + integration tests per tool (fixture-based, committed fixtures) all
  green; port-equality test for calls.
- One recorded real-operation demonstration per tool (commands + outcomes in
  the status file).
- Full workspace verification green with exact counts recorded.

## Task exit criteria
- [ ] Every row of the finalized `## Proposed tools` table has a working
      subcommand whose behavior matches its Purpose/CLI cells.
- [ ] `cargo test -p rfs-dev` green (unit + all integration files).
- [ ] Each tool's demonstration on a real maintenance operation is recorded in
      the status file; sync-check and inventory audit report CLEAN/EXACT on the
      current tree; edit-rename's scratch-copy demonstration compiles.
- [ ] calls port-equality holds: Rust output byte-identical to the python
      scripts' output on the real commands.md.
- [ ] No dependency beyond syn/proc-macro2/pulldown-cmark added (or a plan
      correction recorded for any exception).
- [ ] `cargo test --workspace`, `cargo clippy --all-targets`, `cargo fmt
      --check` green; root-package test count unchanged.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| sync doc-scope narrowed to docs/dev + docs/llm | Action 1 said "every backticked source-path reference in `docs/**/*.md`", but the demonstration on the real tree produced ~1,700 hits - almost all in `docs/roadmap/phase_N/*.md`, the append-only history that DELIBERATELY names pre-refactor paths (INTRO.md: history entries keep their pre-split paths by design), plus prose refs in docs/SEMANTICS.md and a fenced anchor-format example in docs/dev/README.md. The gate (tests/dev_docs_test.rs) scans exactly docs/dev/** + docs/llm/** - the tool must mirror that scope or it diverges from the gate on what counts as a reference. Corrected: check() and fix() walk only those two directories; roadmap history is never rewritten by sync-fix (rewriting it would falsify the record). After the correction the demo reports CLEAN (exit 0), matching the task's expected outcome. |
| anchor collection: fence tracking + heading rule | The first draft collected `<a id="...">` anchors from raw lines, so the README's fenced anchor-format example (`dev:name`) was reported as an orphan. Corrected to mirror the gate: skip fenced code blocks when collecting anchors, and require each anchor to be immediately followed by a heading (blank lines allowed) - both rules are now checked and reported. The fixture docs were updated to the conforming anchor->heading form (they had been written without it). |
| 5.6 dependency exception: `regex` crate | Action 6's port originally assumed hand-rolled std pattern matching (the Context dependency constraint names only syn/proc-macro2/pulldown-cmark). The user authorized adding the `regex` crate as a third declared exception during execution: ~15 python `re` patterns port directly to `regex` syntax (alternation, `\b`, lazy/greedy quantifiers), which is strictly safer than hand-rolled matching for the greedy-backtrack cases (`\b[\w.-]+\.rs\b` on `foo.rs.rs`, the 5-alternative PATH_MENTION_RE). `regex 1.13.1` was already in Cargo.lock transitively - no new lock entry, offline rebuilds unaffected. The one pattern using a backreference + lookahead (fenced-form close) uses a regex-open + same-width line-scan hybrid because `regex` omits both by design. Reflected in dev_tools/Cargo.toml and the PHASE_12_PLAN.md Plan corrections table. |

## Non-goals
- No changes to the `rfs` package, its CLI, codegen output, or tests (dev_tools
  is maintenance-only - PHASE_12.md exit criterion).
- No GUI, no network access from any tool, no shell-out where a Rust library
  suffices (PHASE_12.md non-goal).
- No restructuring of docs/ or src/ - the tools operate on them; moving things
  is a later refactoring phase's job consuming these tools.
- The python analysis scripts are NOT deleted in this task - they remain the
  port-equality oracle (retirement, if ever, is a future decision recorded in
  dev_tools/PROTOCOL.md terms).
