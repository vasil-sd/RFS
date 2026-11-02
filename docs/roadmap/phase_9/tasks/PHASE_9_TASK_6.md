# Phase 9, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Convert every cross-document reference in `docs/dev/DECISIONS.md` - the
rationale index over all DESIGN.md decision entries, the single most
navigation-heavy developer document (~117 `.md` mentions outside code fences)
- into clickable markdown links with correct relative paths, in the style of
`docs/user/`. Rationale: developers navigate from DECISIONS.md to DESIGN.md,
KB.md, TOOLCHAIN.md, CODEGEN.md, SEMANTICS.md, and roadmap plan files
constantly; comfortable navigation is the point of this task.

## Context
- Mention inventory (planning count, outside code fences): 117 `.md` filename
  tokens. Dominant targets: `DESIGN.md` (~91 mentions, usually as "DESIGN.md
  section N" or "Table: DESIGN.md section 9.1 - Code: ..."), plus `KB.md`,
  `TOOLCHAIN.md`, `CODEGEN.md`, `docs/SEMANTICS.md`, roadmap files (`PHASE_6.md`,
  `PHASE_4_TASK_5.md`, `PHASE_4_TASK_11_STATUS.md`, phase plan files), and
  same-dir siblings. Locate them at execution time with:
  `grep -n '[A-Za-z0-9_/.-]*\.md' docs/dev/DECISIONS.md` (then skip lines
  inside ``` fences - there are none at planning, re-verify).
- **Gate safety (verified in the test sources)**: `tests/dev_docs_test.rs`
  doc-to-code references match ONLY backticked inline code spans ending in
  `.rs`/`.c`/`.h` with at least one `/` (`parse_code_ref`,
  tests/dev_docs_test.rs:89) - `.md` names never match, so converting them to
  links cannot break the gate; fenced blocks are not scanned at all.
  `tests/docs_md_test.rs` requires every relative link target to resolve from
  the linking file's directory (`docs/dev/`) - hence the path rules below.
- **Path rules** (from `docs/dev/DECISIONS.md`): same-dir sibling ->
  `ARCHITECTURE.md`; root docs -> `../DESIGN.md`, `../KB.md`, `../BUILD.md`,
  `../TOOLCHAIN.md`, `../CODEGEN.md`, `../SEMANTICS.md` (a mention written as
  `docs/SEMANTICS.md` becomes the link `[SEMANTICS.md](../SEMANTICS.md)`);
  roadmap files -> `../roadmap/phase_N/PHASE_N_PLAN.md` etc.; user docs ->
  `../user/MIGRATION.md`; backend docs -> `../src/backends/RUST.md` /
  `C.md`. Repo-rooted mentions like `docs/dev/X.md` become same-dir `X.md`.
- **Link style**: link the filename token only; keep surrounding prose outside
  the brackets - "DESIGN.md section 20, D7" becomes
  "`[DESIGN.md](../DESIGN.md)` section 20, D7"; "Table: DESIGN.md section 9.1 -
  Code: `src/parser.rs::parse_source`" becomes "Table:
  `[DESIGN.md](../DESIGN.md)` section 9.1 - Code: `src/parser.rs::parse_source`"
  (the code span stays backticked and untouched). This matches docs/user
  practice. NO `#anchor` targets on new links.
- **Do NOT touch**: every `.rs`/`.c`/`.h` doc-to-code span (backticked),
  fragment anchor lines (`<a id="dev:..."></a>`), text inside ``` fences,
  and the "Section N" headings that organize this file.
- **Ordering constraint**: task 5 already re-targeted the ~line-143 citation
  ("recorded in detail in KB.md section 13" -> Phase 1 status report) - linkify
  the CORRECTED text, and do not reintroduce the old target.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | KB.md stale-section cleanup + cross-references | fixes this file's "KB.md section 13" citation first, so the links written here point at existing targets |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | docs/dev clickable links: remaining files | establishes the conversion conventions (path rules, link style) that task 7 applies to the other 14 files |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/DECISIONS.md` | change | all ~117 cross-document `.md` mentions outside fences become clickable relative links; code spans, anchors, and fenced text untouched |

## Planned actions

### Action 1: Convert the mentions
Walk `docs/dev/DECISIONS.md` top to bottom. For every `.md` filename mention
outside a code fence that refers to ANOTHER document (not this file itself),
apply the path rules + link style from Context. Mentions of DECISIONS.md's own
name (if any) stay plain text. Work in contiguous batches (e.g. per "Section
N" block) so a mistake is localized. Rationale: mechanical but voluminous -
batches keep the diff reviewable.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/DECISIONS.md` | change (all ~117 mention sites, outside fences) | cross-document references become clickable relative links per the Context path rules and link style; developers get one-click navigation to every cited document |

### Action 2: Gate verification
Run `cargo test --test dev_docs_test --test docs_md_test`. dev_docs_test
proves no doc-to-code span was damaged and fragment ownership is intact;
docs_md_test proves every new relative link resolves. If a link fails
resolution, the relative path is wrong - fix it (do not delete the link).
Rationale: both gates are machine-checked and part of plain `cargo test`.

#### Affected files

None (test run only).

## Task deliverables
- `docs/dev/DECISIONS.md` with every cross-document `.md` reference (outside
  code fences) as a clickable relative link.
- Both doc gates green.

## Task exit criteria
- [ ] Grep-verified: no backticked or plain-text cross-document `.md` mention
      remains outside code fences in DECISIONS.md (self-references excepted).
- [ ] Every new link target resolves from `docs/dev/` (docs_md_test green).
- [ ] No `.rs/.c/.h` doc-to-code span changed; no fragment anchor line
      changed (dev_docs_test green, 9 pass).
- [ ] The ~line-143 citation links to the Phase 1 status report target fixed
      by task 5 (not to a removed KB section).

## Non-goals
- No changes to DECISIONS.md's decision content, ordering, or headings.
- No `#anchor` link targets introduced.
- No changes to any other file.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- A mention like "PHASE_4_TASK_5.md" inside a sentence about roadmap history
  links as `[PHASE_4_TASK_5.md](../roadmap/phase_4/tasks/PHASE_4_TASK_5.md)` -
  task files live in `phase_N/tasks/`, phase descriptions/plans/status in
  `phase_N/` directly. Check each roadmap target's real location before
  writing the path (docs_md_test will catch a wrong one).
