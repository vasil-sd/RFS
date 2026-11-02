# Phase 9: Documentation cleanup - structure, deduplication, navigability

## Goal
Make the documentation set more structured and clear without duplication:
DESIGN.md becomes the single source of truth for all core language design,
semantics, and design decisions (KB.md stops duplicating it and references it
instead); KB.md keeps what is not core functionality (project state, CLI
behavior, file layout, tests, agent workflow); docs/dev/ gets comfortable
clickable navigation like docs/user/; and the repository root carries a proper
README that orients readers to the right documentation set instead of being an
outdated copy of the user README.

## Scope
- Root `README.md`: delete the outdated copy of `docs/user/README.md`; write a
  new one with (a) short project intro + main principles (one IR for all
targets, backend parity, MC cross-checking), (b) high-level RFS pipeline
overview (spec -> IR -> backends/MC/tests/diagrams), (c) a users section
linking to `docs/user/README.md`, (d) a developers section linking to
`docs/dev/README.md`.
- `docs/dev/*.md` (all 18 files): convert cross-document references to other
  `*.md` files into clickable markdown links (`[...](...)`) with correct
  relative paths, in the style of `docs/user/`. Source-code references
  (`.rs`/`.c`/`.h` doc-to-code spans) stay as-is; fragment anchors stay intact.
- `KB.md` <-> `DESIGN.md` deduplication: every section about language design,
  semantics, or core design decisions (algorithms included) is actualized and
  moved into `DESIGN.md` (where KB.md is newer, DESIGN.md is updated with the
  newer content); the corresponding KB.md sections become short references to
  DESIGN.md. KB.md keeps project state, CLI behavior, source/file layout,
  tests, training-set organization, and agent-workflow experience.
- Outdated-information cleanup in both files: stale roadmap status, explicitly
  superseded historical sections, the stale "C backend planned next" claim, the
  wrong expression-precedence list in DESIGN.md section 5.9 (contradicts the
  actual parser), a stray non-English word in DESIGN.md section 7, and the
  stale KB.md header.
- Stale cross-references fixed wherever they point at moved/removed KB.md
  sections (`docs/SEMANTICS.md`, `docs/dev/DECISIONS.md`,
  `docs/dev/toolchain_fuzz.md`, `docs/llm/toolchain_fuzz.md`).
- `START.md` documentation map + large-doc section maps updated to the new
  KB.md structure.

## Deduplication map (planning findings)

| KB.md section | DESIGN.md counterpart | Disposition |
|---|---|---|
| 1 "Current high-level state" diagram | 2 "High-level architecture" diagram | KB's diagram is NEWER (has `rfs info format`, `mc-ref`, `gen-spec`, in-process SVG); the canonical pipeline diagram moves to DESIGN.md section 2; KB keeps its status table and references the diagram |
| 2.1 Purpose of RFS | 1 Purpose | KB newer (C parity, IDE target); merge into DESIGN.md 1; KB -> reference |
| 2.2 Core semantics | 3 Core semantics | Near-identical; DESIGN.md canonical (has SEMANTICS.md pointer + production-determinism paragraph); KB -> reference |
| 2.3 Model-checking constraints | 4 Model-checking constraints | Same tables; DESIGN.md canonical; KB -> reference |
| 2.4 IR decisions | 7 IR v0 schema | KB's Rust-type table + hand-rolled-JSON decision move into DESIGN.md 7; KB -> reference |
| 2.5 Parser decisions | 8 Parser design | nom/chumsky table already in DESIGN.md 8.1 (canonical); KB's parser validation/non-validation lists move to DESIGN.md 8; KB -> reference |
| 2.6 Property language decisions | 12 PSL-like property language plan (+ 5.10) | KB-unique content (formula-level boolean operators / non_boolean_expr fall-through, `a \|\| b -> c` corner case, Phase 4 precedence/scope rules) merges into DESIGN.md 12 after checking D-P4-8 for existing coverage; KB -> reference |
| 2.7 Code generation architecture | 2 "Code generation as a plugin system" (+ 10) | KB STALE ("C is planned next" - shipped since Phase 2); generated-Rust shape + design intent merge into DESIGN.md; stale claim removed; KB -> reference |
| 3 RFS v0 syntax reference (3.1-3.9) | 5 RFS v0 language subset (5.1-5.10) | Heavy duplication; KB newer on version header, `on_error`, priorities, literal patterns + enum-variant pattern corner cases; DESIGN.md newer on property operators + monitor semantics; ALL merged into DESIGN.md 5; KB -> reference |
| 7 Known limitations and risks (current) | 16 Risks and mitigations | NOT a duplication: DESIGN.md 16 = Phase 0 design risks (stays); KB.md 7 = current project-state limitations (stays in KB) |
| 8 Updated roadmap | - (docs/roadmap/CURRENT_STATE.md) | Stale duplicate of the phase-state table (says Phase 8 IN PROGRESS); replaced by a short pointer to docs/roadmap/ |
| 9 Historical R&D plan | 19-22 (actual decisions) | Explicitly superseded; removed (history preserved in TASKS.md + roadmap status files) |
| 12 What to work on next | - | Explicitly obsolete; removed, pointer to docs/roadmap/CURRENT_STATE.md |
| 13 Phase 1 codegen + bounded MC | 19-22, CODEGEN.md | Explicitly superseded (Phase 4); removed |

KB.md sections that STAY (not core functionality): 1 project state (table),
4 source code map (CLI behavior, training organization, TLA adaptation), 5
build/test/run pointers, 6 agent workflow experience, 7 current limitations,
10 quick-start pointer, 11 rules for future work (renumbered after removals).

## Deliverables
- New root `README.md` (intro + principles, pipeline overview, users section ->
  `docs/user/README.md`, developers section -> `docs/dev/README.md`); old copy
  deleted.
- `DESIGN.md` actualized: sections 1-5, 7, 8, 12 hold the merged core-design
  content; section 2 carries the current pipeline diagram; section 5.9
  precedence table matches `src/parser.rs`; no stray non-English text.
- `KB.md`: sections 2.1-2.7 and 3 are short references to DESIGN.md; stale
  sections removed/pointered; header current; CLI/layout/tests/workflow content
  intact.
- All cross-document `*.md` references in `docs/dev/*.md` (outside code fences)
  are clickable relative links; source-code references untouched.
- Stale cross-references fixed in `docs/SEMANTICS.md`,
  `docs/dev/DECISIONS.md`, `docs/dev/toolchain_fuzz.md`,
  `docs/llm/toolchain_fuzz.md`.
- `START.md` documentation map + section maps synchronized.

## Exit criteria
- [ ] Root `README.md` matches the spec above; the old copy is gone; no
      dangling reference to it anywhere in the repo.
- [ ] DESIGN.md is the single source of truth for language design, semantics,
      and core design decisions: all KB-newer content (version header,
      `on_error`, priorities, literal patterns, IR type table, parser check
      lists, property-language corner cases) is present; section 5.9
      precedence matches the verified parser chain; the CJK artifact in
      section 7 is fixed.
- [ ] KB.md sections 2.1-2.7 and 3 are reference stubs pointing at DESIGN.md;
      sections 8/9/12/13 removed or replaced by pointers; header current;
      sections on CLI, file layout, tests, training organization, and agent
      workflow remain.
- [ ] Every cross-document `*.md` mention in `docs/dev/*.md` outside code
      fences is a clickable relative link that resolves (verified by
      `cargo test --test docs_md_test`); doc-to-code `.rs/.c/.h` references and
      fragment anchors are unchanged (verified by `cargo test --test
      dev_docs_test`).
- [ ] Stale cross-references fixed: SEMANTICS.md points at DESIGN.md sections
      3/5/12 (not KB 2.2/2.6/3); DECISIONS.md's "KB.md section 13" points at
      DESIGN.md section 19; toolchain_fuzz (dev + llm) training-count pointers
      say KB.md section 1.
- [ ] START.md documentation map and large-doc section maps reflect the new
      KB.md structure.
- [ ] Full verification green: `cargo test` (incl. docs_md_test, dev_docs_test),
      `cargo clippy --all-targets`, `cargo fmt --check`.

## Non-goals
- No parser/validator/codegen/MC or other source changes - docs-only phase.
- No renumbering of DESIGN.md sections (content is actualized in place).
- No content rewrite of `docs/user/` or `docs/llm/` beyond the two stale
  reference fixes (same-line replacements, Class B line budgets preserved).
- No `#anchor` targets on new docs/dev links (file-level links only; anchor
  slugs are not machine-checked and would be a maintenance burden).
- No changes to TASKS.md history (append-only; one entry at close-out per the
  protocol).
- No visual/mermaid changes to existing docs/dev diagrams.
