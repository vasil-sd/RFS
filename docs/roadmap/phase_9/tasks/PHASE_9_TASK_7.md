# Phase 9, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Apply the same cross-document link conversion as task 6 to the remaining
docs/dev files that carry `.md` mentions, so the whole Class A set navigates
comfortably like docs/user/. Rationale: task 6 covered only DECISIONS.md (the
largest file); the other 14 files with mentions get identical treatment for a
consistent developer experience.

## Context
- Mention inventory per file (planning counts, outside code fences):
  README 7, DEPENDENCIES 2, REFERENCES 4, ALGORITHMS 9, backend_c 16,
  backend_rust 12, cli_commands 8, codegen_core 11, ir_json_rw 8, mc_engine 8,
  monitor_compiler 6, parser 14, toolchain_fuzz 4, validator 8 - 117 total
  across 14 files. Three files have ZERO mentions and stay untouched:
  ARCHITECTURE.md, CODE_MAP.md, workspace_crates.md (verify with grep at
  execution time).
- **Conventions: exactly task 6's** (path rules from `docs/dev/`, link the
  filename token only, no `#anchors`, never touch `.rs/.c/.h` doc-to-code
  spans / fragment anchors / fenced text). Re-read task 6's Context if in
  doubt - consistency across the set matters more than per-file invention.
- **File-specific notes**:
  - `README.md` (the Class A index): its Index table's File column lists
    backticked sibling names (`ARCHITECTURE.md`, `CODE_MAP.md`, ...) - convert
    each to a same-dir link, keeping the table structure. The "see the Class B
    set in `docs/llm/`" mention links as `[docs/llm/](../llm/)` (directory
    link) or is left plain if it names no file - prefer linking the directory.
  - `backend_rust.md` / `backend_c.md`: mentions of `src/backends/RUST.md` /
    `C.md` (the backend-internal docs) link as
    `[RUST.md](../src/backends/RUST.md)` / `[C.md](../src/backends/C.md)` -
    these are markdown docs, NOT source-code references, so they ARE converted
    (the "links to source code files leave as is" rule covers `.rs/.c/.h` only).
  - `toolchain_fuzz.md`: task 5 already fixed its three "KB.md section 3" ->
    "section 1" pointers; linkify the corrected text (`[KB.md](../KB.md)
    section 1`). Its 4 mentions also include roadmap/training-doc targets.
  - `cli_commands.md`: carries repo-rooted `docs/dev/codegen_core.md`-style
    mentions (3) - convert to same-dir `codegen_core.md` links.
  - `ir_json_rw.md`: mentions `docs/user/MIGRATION.md` (2) ->
    `[MIGRATION.md](../user/MIGRATION.md)`.
  - `ALGORITHMS.md` / `mc_engine.md` / `monitor_compiler.md`: already carry a
    few clickable `REFERENCES.md#anchor` links - leave those existing links
    byte-identical; convert only the still-plain mentions.
- **Gate safety**: identical to task 6 (dev_docs_test matches only `.rs/.c/.h`
  backticked spans; docs_md_test resolves every relative link from `docs/dev/`).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | KB.md stale-section cleanup + cross-references | fixed toolchain_fuzz.md's "section 1" text first - this task linkifies the corrected pointers |
| 6. | docs/dev clickable links: DECISIONS.md | establishes the conventions; running second keeps the whole set stylistically uniform |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | Docs sync + full verification close-out | its final sweep greps for any remaining unconverted mentions - this task must be done first |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/README.md` | change | index-table File column + body mentions become same-dir/directory links |
| `docs/dev/DEPENDENCIES.md` | change | 2 mentions converted |
| `docs/dev/REFERENCES.md` | change | 4 mentions converted (sibling docs) |
| `docs/dev/ALGORITHMS.md` | change | 9 plain mentions converted; existing REFERENCES.md#anchor links untouched |
| `docs/dev/backend_c.md` | change | 16 mentions converted (incl. C.md backend doc + sibling deep dives) |
| `docs/dev/backend_rust.md` | change | 12 mentions converted (incl. RUST.md backend doc) |
| `docs/dev/cli_commands.md` | change | 8 mentions converted (incl. 3 repo-rooted docs/dev/... forms) |
| `docs/dev/codegen_core.md` | change | 11 mentions converted |
| `docs/dev/ir_json_rw.md` | change | 8 mentions converted (incl. 2 user/MIGRATION.md) |
| `docs/dev/mc_engine.md` | change | 8 plain mentions converted; existing anchor link untouched |
| `docs/dev/monitor_compiler.md` | change | 6 plain mentions converted; existing anchor links untouched |
| `docs/dev/parser.md` | change | 14 mentions converted |
| `docs/dev/toolchain_fuzz.md` | change | 4 mentions converted (the task-5-corrected KB pointers included) |
| `docs/dev/validator.md` | change | 8 mentions converted |

Unchanged (zero mentions - verify): `docs/dev/ARCHITECTURE.md`,
`docs/dev/CODE_MAP.md`, `docs/dev/workspace_crates.md`.

## Planned actions

### Action 1: Convert file by file
For each of the 14 files in any order: grep its `.md` mentions outside fences,
apply task 6's path rules + link style, respecting the file-specific notes
above. After EACH file (or at least after every two), run
`cargo test --test docs_md_test` so a wrong relative path is caught immediately
rather than at the end of a 14-file batch. Rationale: per-file gate runs keep
errors localized; the README index and the backend-doc links are the only
structurally delicate spots.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/README.md` | change | Index table File column -> same-dir links; `docs/llm/` mention -> directory link; developers land on a clickable map of the set |
| `docs/dev/DEPENDENCIES.md` | change | 2 cross-doc mentions -> relative links |
| `docs/dev/REFERENCES.md` | change | 4 sibling-doc mentions -> same-dir links |
| `docs/dev/ALGORITHMS.md` | change | 9 plain mentions -> links (DESIGN/KB/TOOLCHAIN/CODEGEN targets); the 5 existing REFERENCES.md#anchor links stay byte-identical |
| `docs/dev/backend_c.md` | change | 16 mentions -> links incl. `[C.md](../src/backends/C.md)` and sibling deep-dive names |
| `docs/dev/backend_rust.md` | change | 12 mentions -> links incl. `[RUST.md](../src/backends/RUST.md)` |
| `docs/dev/cli_commands.md` | change | 8 mentions -> links; repo-rooted docs/dev/codegen_core.md forms become same-dir codegen_core.md |
| `docs/dev/codegen_core.md` | change | 11 mentions -> links (CODEGEN.md x5, DESIGN.md, KB.md targets) |
| `docs/dev/ir_json_rw.md` | change | 8 mentions -> links incl. `[MIGRATION.md](../user/MIGRATION.md)` x2 |
| `docs/dev/mc_engine.md` | change | 8 plain mentions -> links; the existing REFERENCES.md#anchor link untouched |
| `docs/dev/monitor_compiler.md` | change | 6 plain mentions -> links (backend_rust/backend_c sibling refs); existing anchor links untouched |
| `docs/dev/parser.md` | change | 14 mentions -> links (DESIGN.md x6, KB.md, SEMANTICS targets) |
| `docs/dev/toolchain_fuzz.md` | change | 4 mentions -> links; the corrected "`[KB.md](../KB.md)` section 1" pointers included |
| `docs/dev/validator.md` | change | 8 mentions -> links (DESIGN.md x2, KB.md targets) |

### Action 2: Final gate run + zero-mention verification
Run `cargo test --test dev_docs_test --test docs_md_test`. Grep the three zero-
mention files to confirm they still have no `.md` mentions outside fences
(they must be untouched). Rationale: proves the whole set converted cleanly
and nothing was missed or over-converted.

#### Affected files

None (test runs + grep only).

## Task deliverables
- All 14 listed docs/dev files with every cross-document `.md` mention (outside
  code fences) as a clickable relative link, uniform with task 6's style.
- ARCHITECTURE/CODE_MAP/workspace_crates verified untouched.
- Both doc gates green.

## Task exit criteria
- [ ] Grep-verified: no backticked or plain-text cross-document `.md` mention
      remains outside code fences in ANY docs/dev file (self-references and the
      three zero-mention files excepted).
- [ ] `docs/dev/README.md`'s index table is fully clickable.
- [ ] Backend-internal doc links (`../src/backends/RUST.md`, `C.md`) resolve.
- [ ] No `.rs/.c/.h` doc-to-code span, fragment anchor line, or fenced text
      changed anywhere in docs/dev/ (dev_docs_test green, 9 pass).
- [ ] docs_md_test green (5 pass) - every new link resolves.

## Non-goals
- No content changes beyond the link conversions (no rewording, no section
  moves).
- No `#anchor` targets introduced (existing ones preserved as-is).
- No changes outside docs/dev/.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- If a mention's target file does not exist at the computed relative path
  (e.g. a roadmap task file that moved), docs_md_test names the exact line -
  fix the path to the file's real location per INTRO.md's layout note (task
  files in `phase_N/tasks/`), never delete the reference.
