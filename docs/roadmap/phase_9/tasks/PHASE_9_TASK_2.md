# Phase 9, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Make DESIGN.md the single source of truth for RFS's purpose, core semantics,
and model-checking constraints by merging the newer KB.md content into
DESIGN.md sections 1-4, moving the current pipeline diagram (KB section 1) to
DESIGN section 2, and turning KB sections 2.1-2.3 into short reference stubs.
Rationale: these three KB sections duplicate DESIGN sections 1/3/4 nearly
verbatim; one source of truth eliminates drift (the duplication already
caused the stale-diagram mismatch this phase fixes).

## Context
Verified at planning by reading both files section-by-section:
- **KB 2.1 "Purpose of RFS" vs DESIGN 1 "Purpose"**: KB is NEWER - its target
  list includes production Rust AND C automata (parity-pinned), model-
  checking executables, generated tests + fuzz skeletons, `rfs diagram`, and
  IDE support (TextMate grammar + snippets, `validate --json`), plus the
  "intentionally not a general-purpose language" framing and the three-use
  list (real protocol handling code / model checking / tests and diagnostics).
  DESIGN 1 has the use-case bullets (endpoint protocols, event-driven
  automata, PSL-like properties, deterministic production codegen,
  assumption-based constraints) but the older, Rust-centric framing. Merge:
  keep DESIGN 1's use-case list, adopt KB's target list + framing.
- **KB 2.2 "Core semantics" vs DESIGN 3 "Core semantics"**: near-identical
  (same `M = (Q, Sigma, V, q0, delta, Lambda)` tuple, same step form, same
  semantic rules). DESIGN 3 is canonical: it carries the pointer to
  `docs/SEMANTICS.md` sections 2-4 and an extra production-determinism
  paragraph. No KB-unique content expected - verify while merging; fold in
  anything found.
- **KB 2.3 "Model-checking constraints" vs DESIGN 4**: same constraint table
  (states/events/variables/expressions/transitions/time/properties/
  assumptions) and same "deliberately excluded" table, slightly different
  wording. DESIGN 4 is canonical (more polished + SEMANTICS section 9
  pointer). No KB-unique content expected - verify while merging.
- **Pipeline diagram**: KB section 1's "Current high-level state" text diagram
  is NEWER than DESIGN section 2's architecture diagram: it includes `rfs info
  format` (Phase 7), `rfs mc-ref` + `rfs gen-spec` (Phase 6), and the Phase 7
  in-process SVG note on `rfs diagram`. DESIGN 2's diagram stops at `rfs
  diagram [Phase 5 task 8]`. The canonical pipeline diagram moves to DESIGN 2;
  KB section 1 keeps its status TABLE and gets a one-line pointer to the
  diagram. Both files' surrounding workspace-crate paragraphs are equivalent -
  keep DESIGN 2's.
- **Cross-reference stability**: live docs cite "KB.md section 2.2" (e.g.
  `docs/SEMANTICS.md:913`). The KB stubs therefore KEEP their heading numbers
  and names (`### 2.1 Purpose of RFS`, `### 2.2 Core semantics`,
  `### 2.3 Model-checking constraints`) - only the bodies become pointers.
- **Stale cross-reference to fix**: `docs/SEMANTICS.md` line ~913 says
  "`KB.md` section 2.2 (core semantics), 2.6 (property language decisions), 3
  (RFS v0 syntax reference - the surface this document applies to)". After
  this task, core semantics live in DESIGN 3; sections 2.6/3 are handled by
  tasks 3-4. Rewrite the whole citation to point at DESIGN.md sections 3,
  12, 5 (the final targets after tasks 3-4 land) so SEMANTICS.md is edited
  once: "`DESIGN.md` section 3 (core semantics), 12 (property language plan),
  5 (RFS v0 language subset - the surface this document applies to)."

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Root README replacement | none functionally - ordered first by the plan; no shared files |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | KB->DESIGN dedup: syntax reference | edits the same two files (KB.md, DESIGN.md) - must run after this task to avoid conflicting edits |
| 4. | KB->DESIGN dedup: IR/parser/property/codegen | same reason |
| 5. | KB.md stale-section cleanup | renumbers/removes KB sections - needs the final stub structure from tasks 2-4 in place first |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `DESIGN.md` | change | section 1 merged with KB 2.1's newer target list/framing; section 2 diagram replaced by the current pipeline diagram; sections 3-4 verified canonical (fold in anything KB-unique) |
| `KB.md` | change | sections 2.1-2.3 bodies become reference stubs (headings kept); section 1's diagram replaced by a pointer to DESIGN section 2 |
| `docs/SEMANTICS.md` | change | the "Design documents" citation (~line 913) re-pointed from KB 2.2/2.6/3 to DESIGN 3/12/5 |

## Planned actions

### Action 1: Merge KB 2.1 into DESIGN section 1
Read KB section 2.1 and DESIGN section 1 in full. Rewrite DESIGN section 1 to
keep its use-case bullets and add KB's target list (production Rust AND C
automata parity-pinned; model-checking executables with exact engine + text or
JSON report; generated tests incl. fuzz skeletons, both backends; state
diagrams / transition tables via `rfs diagram`; IDE support - in-repo TextMate
grammar + snippets and structured `validate --json` diagnostics, with a full
LSP an explicit non-goal) and the "intentionally not a general-purpose
language" framing plus the three-use list. Rationale: DESIGN 1 becomes the
complete purpose statement; KB's newer content is not lost.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 1, ~lines 18-32) | adopt KB 2.1's target list + framing; keep DESIGN's use-case bullets |

### Action 2: Verify DESIGN sections 3-4 cover KB 2.2-2.3
Read KB 2.2/2.3 against DESIGN 3/4 line by line. Expected: no KB-unique
content (DESIGN carries the SEMANTICS pointers + production-determinism
paragraph). If anything unique is found, fold it into the DESIGN section and
note it in the task status file. Rationale: confirm canonical coverage before
deleting the KB bodies.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 3 or 4, only if KB-unique content is found) | fold in any unique semantic rule; expected: no change |

### Action 3: Move the current pipeline diagram to DESIGN section 2
Replace DESIGN section 2's architecture text diagram with KB section 1's
"Current high-level state" diagram (it adds `rfs info format`, `rfs mc-ref`,
`rfs gen-spec`, and the in-process SVG note). Keep DESIGN 2's surrounding
prose: the SEMANTICS.md pointer line, the workspace-crate paragraph, the
"IR is the single source of truth" paragraph, and the "Code generation as a
plugin system" subsection. Rationale: one canonical, CURRENT pipeline diagram
in the architecture section; KB's copy was the newer one.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 2 diagram block, ~lines 36-70) | replace the stale Phase-5-era diagram with KB's current high-level state diagram |

### Action 4: Turn KB sections 2.1-2.3 into reference stubs
Replace each body with a 1-3 line pointer, KEEPING the heading number and
name:
- `### 2.1 Purpose of RFS` -> "The purpose statement, target list, and
  framing live in `[DESIGN.md](DESIGN.md)` section 1 (single source of truth).
  Normative semantics: `docs/SEMANTICS.md`."
- `### 2.2 Core semantics` -> pointer to DESIGN section 3 + SEMANTICS sections
  2-4.
- `### 2.3 Model-checking constraints` -> pointer to DESIGN section 4 +
  SEMANTICS section 9.
Rationale: cross-references like "KB.md section 2.2" stay resolvable and land
on a stub that routes to the source of truth.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 2.1, ~lines 116-139) | body -> reference stub, heading kept |
| `KB.md` | change (section 2.2, ~lines 140-172) | body -> reference stub, heading kept |
| `KB.md` | change (section 2.3, ~lines 173-196) | body -> reference stub, heading kept |

### Action 5: Replace KB section 1's diagram with a pointer
In KB section 1, delete the "Current high-level state:" text-diagram block and
replace it with one line: "Current high-level pipeline diagram:
`[DESIGN.md](DESIGN.md)` section 2." Keep the status table above it and the
workspace-crate paragraph below it (or drop the paragraph if it now duplicates
DESIGN 2's - it does, so drop it). Rationale: the diagram has a single home.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `KB.md` | change (section 1 diagram block, ~lines 78-112) | diagram -> one-line pointer to DESIGN section 2; drop the now-duplicated workspace-crate paragraph |

### Action 6: Re-point the SEMANTICS.md citation
In `docs/SEMANTICS.md` (~line 913, the "**Design documents:**" paragraph),
replace "`KB.md` section 2.2 (core semantics), 2.6 (property language
decisions), 3 (RFS v0 syntax reference - the surface this document applies
to)." with "`DESIGN.md` section 3 (core semantics), 12 (property language
plan), 5 (RFS v0 language subset - the surface this document applies to)."
Rationale: after tasks 2-4 the cited content lives in DESIGN.md; editing once
now avoids a second SEMANTICS.md edit later.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change (~line 913) | citation re-pointed KB 2.2/2.6/3 -> DESIGN 3/12/5 (final targets after tasks 3-4) |

## Task deliverables
- DESIGN section 1: complete purpose statement (merged from KB 2.1).
- DESIGN section 2: the current pipeline diagram (from KB section 1).
- DESIGN sections 3-4: verified canonical (any KB-unique content folded in).
- KB sections 2.1-2.3: reference stubs with unchanged headings.
- KB section 1: status table kept, diagram replaced by a pointer.
- `docs/SEMANTICS.md`: citation re-pointed to DESIGN 3/12/5.

## Task exit criteria
- [ ] DESIGN section 1 contains KB 2.1's target list (Rust AND C parity,
      tests + fuzz skeletons, diagram, IDE) and the not-a-general-purpose-
      language framing; no information from KB 2.1 is lost.
- [ ] DESIGN section 2's diagram includes `rfs info format`, `rfs mc-ref`,
      `rfs gen-spec`, and the in-process SVG note; the workspace-crate
      paragraph and plugin-system subsection are intact.
- [ ] KB sections 2.1/2.2/2.3 headings are unchanged and their bodies are
      short pointers to DESIGN 1/3/4 (+ SEMANTICS where applicable).
- [ ] KB section 1 no longer contains the text diagram; it points at DESIGN
      section 2.
- [ ] `docs/SEMANTICS.md` cites DESIGN sections 3/12/5 (no "KB.md section
      2.2/2.6/3" citation remains).
- [ ] `cargo test --test docs_md_test --test dev_docs_test` passes (both files
      are covered by the gates; no doc-to-code span was touched, but verify).

## Non-goals
- No changes to KB sections other than 1 (diagram) and 2.1-2.3.
- No renumbering of any section in either file.
- No changes to `docs/SEMANTICS.md` beyond the one citation sentence.
- No source/test/training changes.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- Line numbers above are from the 2026-11-02 planning read; locate sections by
  heading (`grep -n "^### 2\." KB.md`, `grep -n "^## 1\. Purpose" DESIGN.md`)
  rather than trusting the numbers.
