# Phase 9, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately
> without re-deriving scope.

## Goal
Synchronize `START.md` - the session entry point every future LLM session
reads first - with the restructured documentation set, then run the final
repo-wide sweep and full verification that closes the phase. Rationale: if
START.md's documentation map and section maps still describe the old KB.md
layout, every future session mis-navigates on day one; the phase is only done
when the entry point matches reality.

## Context
- **START.md section 2 (documentation map table)**: the `KB.md` row currently
  reads "Design decisions, RFS v0 syntax reference, source code map, CLI
  behavior, training-set organization, agent workflow experience" with read-
  when "For language semantics questions, source layout, CLI rules, 'how is X
  implemented'". After tasks 2-5, KB.md holds: project state; design decisions
  AND syntax reference as REFERENCES to DESIGN.md; source code map (CLI
  behavior, training-set organization, TLA adaptation); build/test/run
  pointers; agent workflow experience; current limitations; roadmap pointer;
  quick-start pointer; rules for future work. Rewrite the row's Content and
  Read-it-when cells accordingly (language semantics questions now route to
  DESIGN.md / docs/SEMANTICS.md first). The `DESIGN.md` row's content
  description gains nothing structurally (no renumbering happened) - leave it,
  unless its "Read it when..." should now also say "the single source of truth
  for the language surface" - a one-phrase addition is fine.
- **START.md section 5 (large-doc section maps)**: the KB.md bullet currently
  lists sections 1-13 ("1 current project state / 2 design decisions / 3 RFS v0
  syntax reference (3.1-3.9) / 4 Rust source code map (...) / 5 build-test-run
  pointers / 6 agent workflow experience (...) / 7 known limitations / 8
  updated roadmap / 9 historical R&D plan (superseded) / 10 quick-start pointer
  / 11 rules for future work / 12 what to work on next / 13 Phase 1 codegen +
  bounded MC"). Rewrite it to the final structure: "1 current project state /
  2 design decisions (references to DESIGN.md 1-4, 7, 8, 12) / 3 RFS v0 syntax
  reference (reference to DESIGN.md section 5) / 4 Rust source code map (4.1
  deps, 4.2 files, 4.3 public API, 4.4 parser internals, 4.5 CLI behavior, 4.6
  training-set organization, 4.7 TLA+ adaptation) / 5 build-test-run pointers /
  6 agent workflow experience (6.1-6.6) / 7 known limitations / 8 roadmap
  pointer / 9 quick-start pointer / 10 rules for future work". The DESIGN.md
  bullet is unchanged (sections 1-25 keep their numbers; content was
  actualized in place).
- **START.md section 6 (task -> read/do/verify table)**: rows that say "KB.md
  sections 3, 6.5" or "KB.md section 4.6" etc. still resolve (those sections
  survived) - re-check each row's KB citation against the final numbering and
  fix any that now point at a stub (a stub is acceptable as a pointer target,
  but a row whose real need is language semantics should cite DESIGN.md
  directly). Expected fixes: the "Change RFS syntax/grammar" row (currently
  "DESIGN.md sections 5-6 + KB.md sections 3, 6.5") can drop the KB 3 citation
  in favor of DESIGN 5 alone; verify the rest.
- **Final sweep greps** (all must come back clean or be explained):
  1. `grep -rn 'KB\.md section [9]\|KB\.md section 1[23]' --include='*.md' .`
     excluding TASKS.md + frozen `docs/roadmap/phase_*/` files -> no hits.
  2. Backticked cross-doc `.md` refs remaining in docs/dev outside fences:
     per-file grep as in tasks 6-7 -> none (self-references excepted).
  3. Root README links resolve: `docs/user/README.md`, `docs/dev/README.md`
     exist (they do) - docs_md_test already proves this.
  4. `grep -rn 'C is planned next' KB.md DESIGN.md` -> no hits (stale claim
     gone).
  5. `grep -nP '[\x{4e00}-\x{9fff}]' KB.md DESIGN.md docs/dev/*.md` -> no hits.
     The one CJK artifact found at planning (DESIGN.md ~line 748, "should
     not污染 the core IR") is NOT inside any section that tasks 2-4 rewrite,
     so this task fixes it as final polish (Action 2); the assignment is
     recorded in the plan's corrections table.
- **Full verification** (BUILD.md section 5, docs-only phase): `cargo build &&
  cargo test && cargo clippy --all-targets && cargo fmt --check`. Expected:
  535 root-package tests pass (docs_md_test 5, dev_docs_test 9 among them), no
  source changes so no generated-code/training re-runs needed - but if ANY `.rs`
  file was accidentally touched by an earlier task, the full checklist applies
  and the incident is recorded in the status file.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Root README replacement | the sweep verifies its links; must be done first |
| 2.-5. | KB->DESIGN dedup + KB cleanup | START.md's maps describe their FINAL result - all must be complete before this task writes them |
| 6.-7. | docs/dev link conversion | the sweep greps for remaining unconverted mentions - both must be done first |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `START.md` | change | section 2 KB.md row rewritten; section 5 KB.md section map rewritten to the final 1-10 structure; section 6 rows re-checked against final numbering |
| `DESIGN.md` | change | one-line artifact fix: section 7 IR design rule 4 "should not污染 the core IR" -> "should not pollute the core IR" (the only CJK occurrence in any doc, found at planning) |

## Planned actions

### Action 1: Update START.md's maps and table
Apply the three START.md edits from Context (section 2 KB row; section 5 KB
section map; section 6 row re-check). Rationale: the entry point must describe
the post-phase documentation set exactly.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `START.md` | change (section 2 table, KB.md row ~line 60) | Content/Read-it-when cells rewritten for the reference-stub structure; language-semantics questions routed to DESIGN.md/SEMANTICS.md |
| `START.md` | change (section 5, KB.md bullet ~line 130) | section map rewritten: stubs for 2-3, pointer for 8, renumbered 9-10, removed 9/12/13 entries gone |
| `START.md` | change (section 6 table rows citing KB sections) | citations re-checked against final numbering; syntax-grammar row cites DESIGN 5 directly |

### Action 2: Fix the last stray artifact in DESIGN.md
Replace the Chinese word in DESIGN section 7's IR design rule 4 ("should
not污染 the core IR") with "pollute". Rationale: found at planning as the only
non-English artifact in the doc set; it sits in a section tasks 2-4 did not
otherwise rewrite, so it is fixed here as final polish.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `DESIGN.md` | change (section 7, ~line 748) | CJK artifact "污染" -> "pollute"; the rule reads in English like its siblings |

### Action 3: Final repo-wide sweep
Run the five greps from Context. Any hit is either fixed on the spot (and
listed in the status file) or explained as an allowed exception (TASKS.md
history, frozen roadmap files). Rationale: the phase promise is "no
duplication, no stale pointers" - the sweep is the proof.

#### Affected files

None expected (grep only; fixes would be recorded here if needed).

### Action 4: Full verification
Run `cargo build && cargo test && cargo clippy --all-targets && cargo fmt
--check` with the standard toolchain activation. Record exact pass counts in
the status file (expected: 535 root-package tests, docs_md_test 5 pass,
dev_docs_test 9 pass; clippy without errors - the known 46-warning/35-unique
baseline unchanged; fmt clean). Rationale: BUILD.md section 5 minimum for any
non-trivial change; docs-only phase must still leave the whole suite green.

#### Affected files

None (build + test runs only).

## Task deliverables
- `START.md` documentation map, section maps, and task table synchronized with
  the restructured docs.
- DESIGN.md free of the CJK artifact.
- Clean sweep results (five greps) recorded in the status file.
- Full verification green with exact counts.

## Task exit criteria
- [ ] START.md section 2's KB.md row describes the reference-stub structure;
      section 5's KB map lists exactly sections 1-10 as they exist; no stale
      "9 historical R&D plan / 12 what to work on next / 13 Phase 1 codegen"
      entries remain.
- [ ] `grep -nP '[\x{4e00}-\x{9fff}]' DESIGN.md KB.md` returns nothing.
- [ ] All five sweep greps clean (or every hit explained as an allowed
      exception in the status file).
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt
      --check` green; exact counts recorded.

## Non-goals
- No changes to BUILD.md / TOOLCHAIN.md / CODEGEN.md (no commands or
  toolchain facts changed by this phase).
- No edits to TASKS.md history or frozen roadmap files found by the sweep.
- No source changes (if the sweep forces one, it becomes a task correction
  with full-checklist consequences).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The phase close-out itself (PHASE_9_STATUS.md per FORMATTING.md section 4,
  CURRENT_STATE.md row update, TASKS.md entry, commit) happens AFTER this task
  passes, per PROTOCOL.md section 3 steps r-t - it is not part of this task's
  actions.
