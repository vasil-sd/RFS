# Phase 9 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation immediately without
> re-deriving scope.

## Context

Planning findings (all verified by reading the actual files on 2026-11-02):

**Root README.** `README.md` (4788 B) is NOT a byte-identical copy of
`docs/user/README.md` (5156 B) - it is an OUTDATED copy: repo-root-relative
links, missing the Phase 7 updates (in-process SVG rendering note, `info
format` mode) and the `docs/dev/` + `docs/llm/` rows. Nothing in the repo links
to the root README (the only `[README.md](README.md)` hit is
`docs/user/LANGUAGE.md` linking its own directory's file), so deletion breaks
no link. The new README must pass `tests/docs_md_test.rs` (comrak parse,
relative-link resolution, every ```mermaid block rendered through
mermaid-rs-renderer) - model the pipeline diagram on the one in
`docs/user/README.md` section "The toolchain at a glance", which already
passes that gate.

**docs/dev link conversion.** 18 files; ~235 cross-document `.md` mentions
OUTSIDE code fences (counted per file: DECISIONS 117, backend_c 16, parser 14,
backend_rust 12, codegen_core 11, ALGORITHMS 9, cli_commands 8, ir_json_rw 8,
mc_engine 8, validator 8, README 7, monitor_compiler 6, REFERENCES 4,
toolchain_fuzz 4, DEPENDENCIES 2; ARCHITECTURE/CODE_MAP/workspace_crates 0).
Six files already carry a few clickable links (all `REFERENCES.md#anchor`
style). Gate safety was verified in the test sources:
- `tests/dev_docs_test.rs` doc-to-code references match ONLY backticked inline
  code spans ending in `.rs`/`.c`/`.h` with at least one `/` (`parse_code_ref`,
  tests/dev_docs_test.rs:89). Backticked or plain-text `.md` names never match,
  so converting them to `[text](target)` links cannot break the gate. Fenced
  blocks are not scanned at all.
- `tests/docs_md_test.rs` checks that EVERY relative link target resolves from
  the linking file's directory - so each new link needs the correct relative
  path: same-dir `X.md`, root docs `../DESIGN.md` / `../KB.md` / `../BUILD.md`
  / `../TOOLCHAIN.md` / `../CODEGEN.md` / `../SEMANTICS.md`, other dirs
  `../user/MIGRATION.md`, `../roadmap/phase_N/...`, backend docs
  `../src/backends/RUST.md` / `C.md`. Repo-rooted mentions like
  `docs/dev/codegen_core.md` become same-dir `codegen_core.md`.
- Fragment anchor lines (`<a id="dev:..."></a>`) must stay byte-identical;
  add NO `#anchor` targets on new links (file-level only).
- Link style: link the filename token, keep surrounding prose outside the
  brackets - e.g. `DESIGN.md section 20, D7` becomes
  `[DESIGN.md](../DESIGN.md) section 20, D7`. This matches docs/user practice.

**KB <-> DESIGN duplication.** Verified section-by-section (see the table in
`PHASE_9.md`). Key facts:
- KB 2.1/2.2/2.3 duplicate DESIGN 1/3/4; DESIGN's versions are canonical
  (they carry the `docs/SEMANTICS.md` pointers); only KB 2.1's target list is
  newer (C parity, IDE) and must be merged into DESIGN 1.
- KB 1's "Current high-level state" text diagram is NEWER than DESIGN 2's
  architecture diagram: it includes `rfs info format` (Phase 7), `rfs mc-ref`
  + `rfs gen-spec` (Phase 6), and the in-process SVG note. The canonical
  pipeline diagram goes to DESIGN 2; KB 1 keeps its status table and a pointer.
- KB 3 vs DESIGN 5: KB is newer on the version header (3.1), `on_error`
  (3.1a), transition priorities (3.5), literal patterns + the
  `on E(Mode::Normal)` corner case (3.6), and the anonymous-payload limitation
  (3.3); DESIGN is newer on property operators + monitor semantics (5.10).
  Merge everything into DESIGN 5.
- PRECEDENCE CONFLICT: KB 3.8's table matches the ACTUAL parser chain in
  `src/parser.rs` (`expr -> logic_or -> logic_and -> equality -> relational ->
  bitwise_or -> bitwise_xor -> bitwise_and -> shift -> additive ->
  multiplicative -> unary_expr -> primary_expr`; there is NO `/` operator -
  verified by reading the functions). DESIGN 5.9's list is WRONG (it puts
  `<< >>` tighter than `* /` and lists a nonexistent `/`). Replace DESIGN 5.9's
  list with the verified table.
- KB 2.4's "Important IR types" table + hand-rolled-JSON decision are absent
  from DESIGN 7 (which has the JSON schema + design rules) - merge in.
- KB 2.5's parser validation/non-validation lists are absent from DESIGN 8 -
  move them in (new subsection). The nom/chumsky comparison table is already
  in DESIGN 8.1 (more detailed) - do not duplicate it back.
- KB 2.6's unique content: formula-level boolean operators (task 23 grammar
  fix, `non_boolean_expr` fall-through), the `a || b -> c` corner case, Phase 4
  liveness precedence/scope rules. Before merging into DESIGN 12, check
  DESIGN 11 + section 22 (D-P4-8/D-P4-9) for existing coverage and add only
  what is missing.
- KB 2.7 is STALE: "The Rust backend ... is the only registered backend so
  far; C is planned next" - the C backend shipped in Phase 2. Merge the
  generated-Rust shape + design-intent bullets into DESIGN 2's plugin-system
  subsection (or section 10 if it fits better) and drop the stale claim.
- KB 7 vs DESIGN 16 are NOT duplicates (current limitations vs Phase 0 design
  risks) - both stay where they are.

**Stale information.**
- `DESIGN.md:748` contains a stray Chinese word: "should not污染 the core IR"
  -> "should not pollute the core IR" (the only CJK occurrence in any doc).
- KB header: "Status: **2026-10-13**", a summary list describing the OLD
  structure ("updated roadmap", "detailed next R&D plan"), and a read order
  that starts with KB.md itself - `START.md` is the session entry point now.
- KB 8 "Updated roadmap" duplicates `docs/roadmap/CURRENT_STATE.md` and is
  stale (says Phase 8 IN PROGRESS; it closed 2026-11-01).
- KB 9 / 12 / 13 are explicitly marked superseded/obsolete in their own
  headers.

**Stale cross-references to fix.**
- `docs/SEMANTICS.md:913`: "`KB.md` section 2.2 (core semantics), 2.6
  (property language decisions), 3 (RFS v0 syntax reference ...)" -> point at
  DESIGN.md sections 3, 12, 5.
- `docs/dev/DECISIONS.md:143`: "recorded in detail in KB.md section 13" ->
  DESIGN.md section 19 (the actual Phase 1 decisions).
- `docs/dev/toolchain_fuzz.md` (3 places: ~line 39, ~164, ~199) and
  `docs/llm/toolchain_fuzz.md` (~line 77): "current training counts: KB.md
  section 3" - WRONG section (training counts live in KB 1's state table and
  BUILD.md). Fix to "KB.md section 1". The llm file is Class B with a machine-
  checked line budget: use SAME-LINE replacements only, no added lines.
- After KB sections 9/12/13 are removed, renumber KB 10 -> 9 and 11 -> 10;
  first grep the whole repo for "KB.md section 1[01]" / "KB section 1[01]"
  references (none were found during planning - re-verify).

**START.md sync.** Section 2's documentation-map row for KB.md ("Design
decisions, RFS v0 syntax reference, source code map, CLI behavior, ...") and
section 5's KB.md section map (lists sections 1-13) must be updated to the new
structure. The DESIGN.md map in section 5 stays valid (no renumbering).

**Verification.** Docs-only phase: `cargo test` (docs_md_test 5 pass,
dev_docs_test 9 pass, everything else unchanged), `cargo clippy --all-targets`,
`cargo fmt --check`. No generated-code or example impact. If any `.rs` file is
accidentally touched, the full BUILD.md section 5 checklist applies.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Root README replacement | `README.md` (remove); `README.md` (new) | Delete the outdated copy of docs/user/README.md; write the new root README: (a) short intro - what RFS is, purpose, main principles (one constrained spec -> one versioned IR -> multiple targets; backend parity with cross-backend differential tests; MC cross-checking via the independent reference MC + differential suites); (b) high-level pipeline overview section with a mermaid flowchart modeled on docs/user/README.md's (spec -> parse/validate -> IR -> codegen rust/c production/test/mc, rfs test, rfs diagram, rfs mc-ref, rfs gen-spec, info/info format); (c) "For users" section linking `[docs/user/README.md](docs/user/README.md)`; (d) "For developers" section linking `[docs/dev/README.md](docs/dev/README.md)` + one line pointing LLM assistants to docs/llm/. Rationale: the root README is the first thing any visitor sees; it must orient, not duplicate. |
| 1.1 | Verify the new README | (test run only) | `cargo test --test docs_md_test` (parse + link resolution + mermaid render); grep the repo for references to the old root README content (none expected). Rationale: the doc-quality gate is machine-checked; a broken mermaid block or dead link fails plain `cargo test`. |
| 2. | KB->DESIGN dedup: purpose, core semantics, MC constraints, pipeline diagram | `KB.md` (change), `DESIGN.md` (change), `docs/SEMANTICS.md` (change) | Merge KB 2.1's newer target list (C parity, IDE, "not a general-purpose language" framing) into DESIGN 1; confirm DESIGN 3/4 already cover KB 2.2/2.3 (fold in anything unique - none expected); replace DESIGN 2's architecture diagram with KB 1's current high-level state diagram (adds info format, mc-ref, gen-spec, in-process SVG), keeping the workspace-crate paragraph; turn KB 2.1-2.3 into short reference stubs that KEEP their heading numbers/names (cross-references like "KB.md section 2.2" must stay resolvable) and point at DESIGN 1/3/4 + docs/SEMANTICS.md; replace KB 1's diagram with a one-line pointer to DESIGN 2; fix docs/SEMANTICS.md:913 to cite DESIGN sections 3/5/12 instead of KB 2.2/2.6/3. Rationale: one source of truth for core semantics; stable KB numbering avoids breaking live cross-references. |
| 3. | KB->DESIGN dedup: syntax reference (KB 3 -> DESIGN 5) | `KB.md` (change), `DESIGN.md` (change) | Merge into DESIGN 5: version header + module items (5.1/5.2 from KB 3.1), `on_error` fallback (new 5.2a or folded into 5.2, from KB 3.1a), transition priorities + Phase 2 semantics (5.8, from KB 3.5), literal patterns + the `on E(Mode::Normal)` corner case + pattern-field table (5.8 event patterns, from KB 3.6), anonymous-payload limitation note (5.5, from KB 3.3); REPLACE DESIGN 5.9's wrong precedence list with the verified parser chain table from KB 3.8 (no `/`; shifts looser than arithmetic; relational tighter than equality) noting it is pinned against src/parser.rs; keep DESIGN 5.10 as-is (already newer). Turn KB 3 into a reference stub -> DESIGN 5 (+ docs/SEMANTICS.md for normative semantics), keeping the "## 3. RFS v0 syntax reference" heading. Rationale: the syntax reference is core language design; the precedence conflict must be resolved in favor of the actual parser. |
| 4. | KB->DESIGN dedup: IR, parser, property language, codegen architecture (KB 2.4-2.7) | `KB.md` (change), `DESIGN.md` (change) | Parent task: four merges below + turning KB 2.4-2.7 into reference stubs that keep their heading numbers and point at DESIGN 7/8/12/2 (+ CODEGEN.md for the backend-author API). Rationale: IR schema, parser design, property language, and codegen architecture are core design decisions - they belong in DESIGN.md. |
| 4.1 | IR decisions -> DESIGN 7 | `KB.md` (change), `DESIGN.md` (change) | Move KB 2.4's "Important IR types" table + hand-rolled-JSON decision into DESIGN 7 (extend the IR design rules); KB 2.4 becomes a reference stub. |
| 4.2 | Parser decisions -> DESIGN 8 | `KB.md` (change), `DESIGN.md` (change) | Move KB 2.5's parser validation/non-validation lists into DESIGN 8 as a new subsection (e.g. 8.5 "Parser-level checks"); do NOT duplicate the nom/chumsky table (DESIGN 8.1 is canonical); KB 2.5 becomes a reference stub. |
| 4.3 | Property language decisions -> DESIGN 12 | `KB.md` (change), `DESIGN.md` (change) | Merge KB 2.6's unique content (formula-level boolean operators + non_boolean_expr fall-through, `a \|\| b -> c` corner case, Phase 4 liveness precedence/scope rules) into DESIGN 12 AFTER checking DESIGN 11 + section 22 D-P4-8/D-P4-9 for existing coverage - add only what is missing; KB 2.6 becomes a reference stub. |
| 4.4 | Codegen architecture -> DESIGN 2/10 | `KB.md` (change), `DESIGN.md` (change) | Fix the stale "C is planned next" claim: merge KB 2.7's generated-Rust shape + design-intent bullets into DESIGN 2's plugin-system subsection (or section 10 if it fits better), stating both backends are shipped and parity-pinned; KB 2.7 becomes a reference stub. |
| 5. | KB.md stale-section cleanup + header + stale cross-references | `KB.md` (change), `docs/dev/DECISIONS.md` (change), `docs/dev/toolchain_fuzz.md` (change), `docs/llm/toolchain_fuzz.md` (change) | Update the KB header (current status date, summary list matching the NEW structure, read order starting from START.md); replace KB 8's stale per-phase status text with a short pointer to docs/roadmap/CURRENT_STATE.md (+ PRINCIPLES.md section 3 for post-v1 candidates); REMOVE KB 9 (superseded historical R&D plan), KB 12 (obsolete next-work), KB 13 (superseded Phase 1 codegen/MC) - their substance lives in TASKS.md, the roadmap status files, and DESIGN 19-22; renumber KB 10 -> 9 and 11 -> 10 after grepping the repo for references to those section numbers (none found at planning - re-verify); fix docs/dev/DECISIONS.md:143 "recorded in detail in KB.md section 13" -> DESIGN.md section 19; fix the four "KB.md section 3" training-count pointers (docs/dev/toolchain_fuzz.md x3, docs/llm/toolchain_fuzz.md x1) to "KB.md section 1" using same-line replacements only (Class B budget). Rationale: explicitly superseded sections are outdated information and duplicate the roadmap/DESIGN records; wrong section pointers mislead future sessions. |
| 6. | docs/dev clickable links: DECISIONS.md | `docs/dev/DECISIONS.md` (change) | Convert ALL ~117 cross-document `.md` mentions outside code fences into clickable relative links (e.g. `[DESIGN.md](../DESIGN.md)`, `[KB.md](../KB.md)`, `[TOOLCHAIN.md](../TOOLCHAIN.md)`, `[CODEGEN.md](../CODEGEN.md)`, `[docs/SEMANTICS.md](../SEMANTICS.md)` -> `[SEMANTICS.md](../SEMANTICS.md)`, roadmap plan files `[PHASE_6.md](../roadmap/phase_6/PHASE_6.md)` etc., same-dir siblings); keep "section N" prose outside the brackets; leave every `.rs/.c/.h` doc-to-code span and fragment anchor byte-identical. Run `cargo test --test dev_docs_test --test docs_md_test` after the batch. Rationale: DECISIONS.md is the rationale index - developers navigate from it to DESIGN.md constantly; it is also the largest single file, so it gets its own task. |
| 7. | docs/dev clickable links: remaining files | `docs/dev/README.md`, `DEPENDENCIES.md`, `REFERENCES.md`, `ALGORITHMS.md`, `backend_c.md`, `backend_rust.md`, `cli_commands.md`, `codegen_core.md`, `ir_json_rw.md`, `mc_engine.md`, `monitor_compiler.md`, `parser.md`, `toolchain_fuzz.md`, `validator.md` (all change) | Same conversion for the other 14 files with mentions (~117 total): README's index-table File column becomes links; repo-rooted `docs/dev/X.md` mentions become same-dir `X.md` links; backend docs link as `[RUST.md](../src/backends/RUST.md)` / `[C.md](../src/backends/C.md)`; ARCHITECTURE/CODE_MAP/workspace_crates have zero mentions - verify and leave untouched. Same gate runs after the batch. Rationale: comfortable navigation across the whole Class A set, consistent with task 6's conventions. |
| 8. | Docs sync + full verification close-out | `START.md` (change) | Update START.md section 2's KB.md documentation-map row (new content description: project state, references to DESIGN.md for core design, source code map, CLI behavior, training-set organization, agent workflow experience, current limitations) and section 5's KB.md section map (sections 1-8 + renumbered 9/10; drop 9/12/13 entries); the DESIGN.md map stays unchanged. Final sweep: grep for any remaining backticked cross-doc `.md` references in docs/dev, any stale "KB.md section 9/12/13" or "KB section 1[01]" references repo-wide (TASKS.md history excluded - append-only), and confirm the root README links resolve. Run the full verification: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`. Rationale: START.md is the session entry point - its maps must match the new structure or every future session mis-navigates. |

Execution order: 1 -> 2 -> 3 -> 4 -> 5 -> 6 -> 7 -> 8. Tasks 2-5 all edit
KB.md/DESIGN.md and run strictly one at a time; task 5's cross-reference fixes
must land BEFORE tasks 6-7 linkify those same files (so the links point at the
correct targets). Task 1 is independent of the rest.

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 8. | Docs sync + full verification close-out | change | The CJK artifact fix (DESIGN.md ~line 748, "should not污染 the core IR" -> "pollute") is explicitly assigned to task 8's Action 2. The plan Context lists it as stale information and PHASE_9.md's exit criteria require it fixed, but no planned-task row owned it: tasks 2-4 rewrite only their own sections, and the artifact sits in DESIGN section 7's design rules (which task 4.1 extends but does not rewrite). Assigned to the final-polish task so the fix is tracked; task 8's file records the same. |
| 1. | Root README replacement | change | During task 1 execution, `cargo test --test docs_md_test` failed on 6 of 255 docs - all Phase 9 planning/task documents (this file + tasks 2, 3, 4, 6, 7 files), NOT the new README: their bare illustrative markdown link examples are parsed by comrak as real relative links that must resolve from docs/roadmap/phase_9/. Corrected by wrapping every bare illustrative link in a code span (backticks) across those six documents. Lesson for tasks 6-7 and any future phase docs: inside roadmap planning documents, show link syntax only inside backticks; the repo-wide doc gate covers them too. No planned-task scope changed. |

(this table is appended to during implementation only; the "Planned tasks"
table above is never modified after user approval.)

## Other information

- Assumption to re-check at task 4.3: how much of KB 2.6's Phase 4 liveness
  precedence/scope content DESIGN 11/section 22 already covers - the merge must
  not create a NEW duplication inside DESIGN.md.
- The docs/dev link conversion is mechanical but voluminous; do it file by file
  and run the two doc gates after each file (or at least after each task batch)
  so a bad relative path is caught immediately.
- `docs/llm/toolchain_fuzz.md` carries a machine-checked line budget (Class B):
  the fix there is a same-line word replacement ("section 3" -> "section 1"),
  never a rewrap that changes line count.
- If the new root README's mermaid diagram fails the renderer, simplify it
  toward the docs/user/README.md shape (that one passes) rather than fighting
  the renderer subset.
- No training-set changes are needed: no SFT example quotes the root README or
  the removed KB sections (re-verify with `grep -l "section 13" training/` at
  task 5; if a hit appears, record it as a plan correction).
