# Phase 9 - Implementation status

> Written once when all exit criteria of `PHASE_9.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

(none - docs-only phase)

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `README.md` | removed + new | the outdated copy of docs/user/README.md deleted; new repository entry point: intro + main principles (one IR for all targets, backend parity, MC cross-checking), mermaid pipeline overview (gate-verified shape), users -> docs/user/README.md, developers -> docs/dev/README.md (+docs/llm/, START.md) |
| `DESIGN.md` | changed | single source of truth for core design: section 1 merged with KB's target list + framing; section 2 carries the current pipeline diagram (info format, mc-ref, gen-spec, in-process SVG); section 5 actualized (version header, on_error, priorities, literal patterns + corner case, anonymous-payload limitation) and its WRONG precedence list replaced with the table pinned against the verified parser chain; section 7 gained the IR type table + hand-rolled-JSON rule 5; new section 8.5 parser-level checks; section 12 gained formula-level operators + the `\|\|`/`->` corner case; section 2 Backends bullet names C.md + cross-refs section 10; section 7 rule 4 CJK artifact fixed |
| `KB.md` | changed | sections 2.1-2.7 and 3 are reference stubs to DESIGN.md (headings kept); section 1 diagram -> pointer to DESIGN 2; header rewritten (status 2026-11-02, new summary list, read order from START.md); section 8 -> roadmap pointer; sections 9/12/13 removed; 10/11 renumbered to 9/10. File shrank 1056 -> 674 lines |
| `docs/SEMANTICS.md` | changed | "Design documents" citation re-pointed from KB 2.2/2.6/3 to DESIGN 3/12/5 |
| `START.md` | changed | section 2 KB.md row + section 5 KB section map rewritten to the final structure; section 6 rows re-checked (syntax/grammar -> DESIGN 5-6, properties -> DESIGN 12+5.10); workflow-rules heading cites renumbered KB section 10 |
| `docs/dev/DECISIONS.md` | changed | the "KB.md section 13" citation re-targeted to the Phase 1 status report; all 117 cross-doc `.md` mentions converted to clickable relative links |
| `docs/dev/{README,DEPENDENCIES,REFERENCES,ALGORITHMS,backend_c,backend_rust,cli_commands,codegen_core,ir_json_rw,mc_engine,monitor_compiler,parser,toolchain_fuzz,validator}.md` | changed | 14 files: all 112 bare cross-doc `.md` mentions converted to clickable relative links (README index table fully clickable; repo-rooted docs/dev/... forms -> same-dir; backend docs -> ../../src/backends/); toolchain_fuzz's three wrong "KB section 3" training-count pointers fixed to section 1 first |
| `docs/llm/toolchain_fuzz.md` | changed | one same-line "section 3" -> "section 1" fix (Class B line budget preserved - file stays 78 lines) |
| `docs/roadmap/phase_9/PHASE_9.md`, `PHASE_9_PLAN.md`, `tasks/PHASE_9_TASK_{1..8}.md` (+ `_STATUS`) | new | the phase description, plan (with 3 corrections recorded), and per-task files + status reports |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

- None - no parser/validator/codegen/MC or other source changed; the toolchain
  behaves exactly as before (535/535 baseline preserved).

## Removed functionality

- The duplicated core-design content in KB.md sections 2.1-2.7 and 3 (now
  pointer stubs to DESIGN.md).
- The stale KB sections: 8's per-phase status view (duplicated CURRENT_STATE.md
  and said Phase 8 IN PROGRESS), 9 (superseded historical R&D plan), 12
  (obsolete next-work), 13 (superseded Phase 1 codegen/MC).
- The false statement "The Rust backend ... is the only registered backend so
  far; C is planned next" (C shipped in Phase 2).
- DESIGN section 5.9's incorrect expression-precedence list (shifts tighter
  than `*`, nonexistent `/`) - replaced by the parser-verified table.
- The outdated root README (stale copy of the user docs).

## New functionality

- Root `README.md` as the repository orientation page (principles, pipeline
  diagram, routing to user/developer/LLM doc sets).
- DESIGN.md as the single source of truth for language design, semantics, and
  core design decisions - actualized with every newer KB fact and the
  parser-verified precedence.
- Clickable cross-document navigation across the whole docs/dev Class A set
  (229 links total: 117 in DECISIONS.md + 112 across 14 files), uniform with
  docs/user style; source-code doc-to-code references and fragment anchors
  untouched.
- KB.md restructured to its non-core role: project state, DESIGN.md pointers,
  source/CLI/training/workflow facts, current limitations, roadmap pointer.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (535) | OK - exactly the known-good baseline, incl. docs_md_test 5/5 and dev_docs_test 9/9 |
| `cargo clippy --all-targets` | OK - no errors; documented pre-existing warning baseline (46 lines / 35 unique) unchanged |
| `cargo fmt --check` | OK - clean |

## Found problems/issues/mismatches
- The root README was not a byte-identical copy of docs/user/README.md but an
  OUTDATED one (missing Phase 7 updates + dev/llm rows) - confirmed at task 1.
- DESIGN section 5.9 stated an expression precedence the parser does not
  implement (contradicted KB 3.8); resolved in favor of the re-verified
  `src/parser.rs` chain (task 3).
- KB section 2.7 claimed the C backend was "planned next" - false since Phase 2
  (task 4.4).
- A stray Chinese word (污染) sat in DESIGN section 7 rule 4 (fixed task 8 per
  plan correction).
- Stale cross-references: SEMANTICS.md cited KB 2.2/2.6/3 for content that
  belongs to DESIGN; DECISIONS.md cited removed KB section 13; four
  toolchain_fuzz pointers cited "KB section 3" for training counts that live in
  KB section 1 (all fixed, tasks 2 + 5).
- The planned docs/dev link path rules used the wrong depth for root documents
  (`../` instead of `../../`) - docs_md_test caught all 99 affected links on
  task 6's first run; corrected and inherited by task 7 (plan correction).
- Backticked doc mentions converted inside their code spans would render as
  literal `[...](...)` text - two unwrap passes (15 + 15 links, tasks 6-7).
- The repo-wide docs_md_test gate covers roadmap planning documents too: bare
  illustrative link syntax in them fails resolution - wrapped in code spans
  (task 1; recurred once in a task-8 status file, fixed the same way).
- Environmental (not project): this sandbox session's puppeteer cache lacked
  chrome-headless-shell, failing the availability-gated external half of
  diagram_render_parity_test; PUPPETEER_CACHE_DIR pointed at an existing
  session's cache (exact version 156.0.8078.47) restores 13/13.

## Other important findings/information
- Commit discipline for this phase (user rule): one commit per successful task
  or subtask - 8174b08 (T1), 5eb3172 (T2), cc16d14 (T3), ab68aa6 / 14b2943 /
  99c9775 / 2cf2b44 / 75ade54 (T4.1-T4.4 + bookkeeping), f325d53 (T5),
  de07b97 (T6), 06cfeec (T7), 8e38a9e (T8).
- The "Planned tasks" table of PHASE_9_PLAN.md was never modified; three
  corrections were recorded (CJK-artifact assignment to task 8, illustrative-
  link wrapping lesson, root-doc link depth).
- DESIGN.md section numbering is unchanged (1-25) - all actualization happened
  in place, so every existing "DESIGN.md section N" reference repo-wide still
  resolves.
