# Phase 8, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/DECISIONS.md` (Class A): the rationale index over EVERY
decision entry in `DESIGN.md` - section 9 subsections 9.1-9.11 + section 19
bullets (subtask 5.1), section 20 D1-D10 + section 21 D-P3-1..6 (5.2),
section 22 D-P4-1..14 + section 23 D-P5-1..14 (5.3), section 24 D-P6-1..11 +
section 25 D-P7-1..13 (5.4). Every entry: the chosen option, a 2-4 line
"why" summary, a link to the full comparison table in `DESIGN.md`, and
doc-to-code references to the implementing code.

## Context

- **Skeleton state (from task 1.2)**: `docs/dev/DECISIONS.md` exists with
  exactly four anchored fragments, each followed by a TBD line:
  `dev:decisions-early` (5.1), `dev:decisions-p2-p3` (5.2),
  `dev:decisions-p4-p5` (5.3), `dev:decisions-p6-p7` (5.4). ALL FOUR are
  already owned by the single `//! Docs:` line of `src/lib.rs` (verified
  2026-10-29) - so NO new anchors and NO source edits are needed for any
  subtask; only `docs/dev/DECISIONS.md` changes.
- **ORPHAN RULE**: do NOT introduce new fragment IDs in this task. A new
  anchor would demand a new owner on a source file's `//! Docs:` line;
  keeping the existing four fragments avoids any source change (the plan
  rows for 5.1-5.4 list only `docs/dev/DECISIONS.md` as affected).
- **Source material - `DESIGN.md` line ranges (measured 2026-10-29)**:
  section 9 = lines 805-909 (9.1 DSL hosting, 9.2 FSM authoring style,
  9.3 data model, 9.4 determinism/transition selection, 9.5 time/timeouts,
  9.6 property language, 9.7 MC engine strategy, 9.8 production integration
  style, 9.9 error handling, 9.10 assumptions, 9.11 composition - each a
  comparison table with a marked Decision column); section 19 = lines
  1535-1547 (eight Phase 1 code-generation bullets); section 20 = lines
  1548-1675 (D1-D10 + the "Phase 2 summary of what was already present"
  note at 1666); section 21 = lines 1676-1771 (D-P3-1..6); section 22 =
  lines 1772-1900 (D-P4-1..14 + plan-corrections pointer at 1884);
  section 23 = lines 1901-2137 (D-P5-1..14 + corrections at 2117);
  section 24 = lines 2138-2312 (D-P6-1..11 + execution notes at 2298);
  section 25 = lines 2313-2499 (D-P7-1..13 + execution notes at 2468).
- **Entry format (house style, cf. `docs/dev/ARCHITECTURE.md`)**: one
  `### <ID>. <title>` subsection per decision entry; first line states the
  chosen option(s); then a 2-4 line "why" summary (rationale, not a restatement
  of the table); then one line: `Table:` DESIGN.md section X.Y (plain text -
  NOT a code span) + `Code:` backticked doc-to-code references.
- **doc-to-code reference rules (gate check 1)**: a backticked string shaped
  like `path/file.rs` or `path/file.rs::item` MUST resolve to an existing
  file and (for `.rs` + `::item`) to an item declared in that file - the gate
  matches declaration lines (`fn`/`struct`/`enum`/`trait`/`type`/`const`/
  `static`/`mod`/impl headers; private items are fine). VERIFY every new
  reference against a declaration line before writing it (grep). Plain-text
  mentions without backticks are exempt. `.rfs`/`.json` paths do not match
  the reference shape and are safe to backtick.
- **Docs gate**: `docs_md_test` parses every repo `.md` (comrak GFM + mermaid
  render); DECISIONS.md is a prose index - no mermaid blocks needed, but the
  markdown must stay well-formed. Run `cargo test --test dev_docs_test`
  (fast) after each subtask; `docs_md_test` at task end.
- **Bookkeeping (established Phase 8 practice)**: mark the plan row's Status
  column DONE in `PHASE_8_PLAN.md` after each verified subtask; check off the
  matching exit-criteria boxes in this file; one git commit per subtask with a
  detailed message; ONE `TASKS.md` entry when task 5 as a whole is complete
  (after 5.4).
- **Verified doc-to-code references for 5.1** (declaration lines checked
  2026-10-29): `src/parser.rs::parse_source`, `src/ir.rs::Ir`,
  `src/ir.rs::Module`, `src/backends/rust_fsm.rs::transition_block`,
  `src/backends/rust_fsm.rs::on_error_block`,
  `src/backends/rust_fsm.rs::step_with_handler`,
  `src/backends/rust_fsm.rs::generate_production`,
  `src/backends/rust_fsm.rs::gen_expr`, `src/backends/c_fsm.rs::transition_block_c`,
  `src/validator.rs::type_expr`, `src/validator.rs::validate_module`,
  `src/backends/rust.rs::rust_type`, `src/backends/rust.rs::bit_rust_type`,
  `src/backends/c.rs::c_type`, `src/monitor.rs::compile_formula`,
  `src/mc_plan.rs::McPlan`, `src/backends/rust_mc.rs::exact_codegen`,
  `src/ref_mc.rs::run`, `src/codegen.rs::run`,
  `src/codegen.rs::AssumptionSelection`,
  `src/codegen_test.rs::build_test_plan`,
  `src/codegen_fsm.rs::representative_values`,
  `src/codegen_mc.rs::ViolationSpec`,
  `crates/rfs-adapters/src/lib.rs::ProtocolFsm`. Still to verify at
  execution: the parser's formula entry (candidate `src/parser.rs::primary_formula`)
  and a C-side on_error item for 9.9.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.2 | DECISIONS.md skeleton with the four anchored fragments, all owned by `src/lib.rs` |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 14 | pointer/doc sync may cross-link from `START.md`/`KB.md` into the filled DECISIONS fragments; IDs are stable from now |

## Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/DECISIONS.md` | change | replace the four TBD sections with the rationale index (one section per subtask 5.1-5.4) |

No source files change: all four fragments are already owned by
`src/lib.rs`, and no new fragment IDs are introduced.

## Planned actions

### Action 5.1: `dev:decisions-early` - section 9 (9.1-9.11) + section 19 bullets

Replace the TBD under `dev:decisions-early`: one `###` subsection per
section-9 entry (chosen option + 2-4 line why + Table/Code line) and a
"Section 19 - Phase 1 code-generation decisions" part with one bullet per
the eight section-19 bullets (same shape). Use only the verified references
listed in Context (verify the two open candidates first). Run the gate.

### Action 5.2: `dev:decisions-p2-p3` - D1-D10 + D-P3-1..6

Replace the TBD under `dev:decisions-p2-p3`: one `###` subsection per D1-D10
(priority direction, ambiguity/duplicate detection, duplicate-priority scope,
diagnostic severity, effective priority default, unexpected-input error
handling, version header syntax, C type mapping/scope, feature flags, IR
evolution rule) + one per D-P3-1..6 (always[N] IR representation, zero-bound
rules, exact monitor semantics, online monitors replace post-hoc checks, C
monitor parity, CLI flag + per-language gating). Verify each new reference
against declaration lines. Run the gate.

### Action 5.3: `dev:decisions-p4-p5` - D-P4-1..14 + D-P5-1..14

Replace the TBD under `dev:decisions-p4-p5`: one `###` subsection per
D-P4-1..14 (shared plan/per-backend engines, level-synchronous BFS, input
abstraction, statuses/summary line, deadlock semantics, assumption pruning,
modes/flags, liveness language, liveness scope, liveness algorithm, fairness
policy, SMT bounded mode, benchmarks, seedless test migration) + one per
D-P5-1..14 (workspace layout, hand-rolled JSON, rfs-build, rfs info, rfs
test, rfs diagram, external-tool parameterization, --print-script, no_std,
MC counterexample JSON, ProtocolFsm + adapters, extended test generation,
IDE support, example-crate matrix). Verify each new reference. Run the gate.

### Action 5.4: `dev:decisions-p6-p7` - D-P6-1..11 + D-P7-1..13

Replace the TBD under `dev:decisions-p6-p7`: one `###` subsection per
D-P6-1..11 (version header formalized, IR_VERSION 0->1 policy, spec hash +
toolchain version stamp, two-layer toolchain fuzzing, independent reference
MC, production-FSM parity drivers, hand-written monitor references, PSL
translation checks, benchmark-in-CI wiring, rfs mc-ref, rfs gen-spec) + one
per D-P7-1..13 (internal/external renderer default, internal_render_svg
hook, plantuml-core git dep, mermaid-rs-renderer promotion, --print-script
internal variant, rfs info shape, EBNF model mirrors parser, draft-07 shared
emitter, sync tests guarantee, cross-renderer syntax-only parity, no new
examples, category-local training, table/md + .err excluded). Verify each
new reference. Run the full gate (dev_docs_test + docs_md_test).

## Task exit criteria

- [x] 5.1: section 9.1-9.11 + all eight section-19 bullets present under
      `dev:decisions-early` with chosen option + why + Table/Code lines; no
      TBD remains in that section; gate green.
- [x] 5.2: D1-D10 + D-P3-1..6 present under `dev:decisions-p2-p3`; no TBD;
      gate green.
- [ ] 5.3: D-P4-1..14 + D-P5-1..14 present under `dev:decisions-p4-p5`; no
      TBD; gate green.
- [ ] 5.4: D-P6-1..11 + D-P7-1..13 present under `dev:decisions-p6-p7`; no
      TBD anywhere in DECISIONS.md; full gate green (dev_docs_test +
      docs_md_test).
- [ ] Every doc-to-code reference resolves (gate check 1); no new fragment
      IDs introduced; no source files changed.
- [ ] Plan rows 5.1-5.4 marked DONE; this file's boxes checked; one git
      commit per subtask; `TASKS.md` entry appended after 5.4.
- [ ] Full verification green: cargo build/test/clippy/fmt (+ --workspace).

## Non-goals

- No new fragment IDs/anchors (the four skeleton fragments cover the task);
  no source edits of any kind (doc comments unchanged - ownership already in
  place via `src/lib.rs`).
- No Class B changes; no mermaid diagrams (prose index); no restatement of
  the DESIGN.md comparison tables (summarize + link only); no DESIGN.md
  edits.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
