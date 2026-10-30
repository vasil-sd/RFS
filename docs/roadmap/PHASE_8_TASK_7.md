# Phase 8, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Fill in `docs/dev/REFERENCES.md`: a consolidated index of external
articles/books/publications with one-line annotations on what each covers
and which RFS component uses it. Two sections: parsing theory, automata
theory, compiler construction (7.1) and model checking, SMT/SAT solving
(7.2). Each subtask also wires links from the relevant `ALGORITHMS.md`
sections into the new entries - the ALGORITHMS intro promises "External
references are collected in `REFERENCES.md` and linked from the relevant
sections".

## Context

- **Skeleton state (from task 1.2)**: `docs/dev/REFERENCES.md` exists with
  two UNANCHORED sections, each followed by a TBD line naming its filling
  task ("TBD (task 7.x). Fragment anchor added in task 7.x."). As in task 6,
  EACH subtask adds its own `<a id="dev:..."></a>` anchor line immediately
  above the heading AND names the new fragment on the owning source file's
  `//! Docs:` line (orphan rule: every fragment needs at least one owner).
- **Fragment IDs + owners (pre-planned in PHASE_8_TASK_1.md, "Planned
  primary fragment IDs per doc" table)**: 7.1 uses `dev:references-theory`
  owned by `src/parser.rs`; 7.2 uses `dev:references-model-checking` owned
  by `src/ref_mc.rs` (anchors added in chunks 7.1-7.2 per that table).
- **Entry format**: one bullet per work - bold title (authors - title,
  venue/edition), then a one-line annotation of what it covers, then
  "RFS uses:" naming the component(s) with doc-to-code references. Entries
  are grouped under `###` subheadings by area (parsing theory / automata
  theory / compiler construction for 7.1; model checking / SMT-SAT solving
  for 7.2). Curated, not exhaustive: only works RFS actually uses.
- **Link style (house style per docs/user)**: standard relative Markdown
  links with GFM heading anchors, e.g.
  `[parsing theory, automata, compiler construction](REFERENCES.md#parsing-automata-compiler-construction)`.
  Each wired ALGORITHMS section gets a trailing `Refs:` line after its last
  `Code:` line (auditable, like the Code: lines). docs_md_test checks that
  every relative link target resolves to an existing path.
- **doc-to-code reference rules (gate check 1)**: same as task 6 - a
  backticked string shaped like `path/file.rs` or `path/file.rs::item` MUST
  resolve to an existing file and (for `.rs` + `::item`) to an item declared
  in that file. VERIFY every new reference against a declaration line before
  writing it (grep). Plain-text mentions without backticks are exempt.
- **Docs gate**: run `cargo test --test dev_docs_test` (fast) after each
  subtask; `docs_md_test` at task end.
- **Bookkeeping (established Phase 8 practice)**: mark the plan row's Status
  column DONE in `PHASE_8_PLAN.md` after each verified subtask; check off the
  matching exit-criteria boxes in this file; one git commit per subtask with
  a detailed message; ONE `TASKS.md` entry when task 7 as a whole is complete
  (after 7.2).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 1.2 | REFERENCES.md skeleton with the two TBD sections (anchors added per subtask) |
| 2. | 6.2 / 6.4 | ALGORITHMS parser/validation/codegen sections (7.1 links from) and MC/monitor sections (7.2 links from) exist and are filled |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 14 | pointer/doc sync may cross-link from `START.md`/`KB.md` into the filled REFERENCES sections |

## Affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/dev/REFERENCES.md` | change | replace the two TBD sections with grouped entry lists (one section per subtask) + one new anchor per section |
| `docs/dev/ALGORITHMS.md` | change | trailing `Refs:` line in the wired sections (6.1+6.2 for 7.1; 6.3+6.4 for 7.2) |
| owner source files (`src/parser.rs` for 7.1; `src/ref_mc.rs` for 7.2) | change | add the new fragment ID to the `//! Docs:` line ONLY (doc comment, no code) |

## Planned actions

### Action 7.1: `dev:references-theory` - parsing theory, automata theory, compiler construction

Replace the TBD under "Parsing, automata, compiler construction" with a short
intro line + grouped entries (one bullet each: bold title + one-line
annotation + "RFS uses:" with doc-to-code references):
- **Parsing theory**: Pratt, "Top-Down Operator Precedence" (CACM 16(4),
  1973) - precedence climbing for the 12-level expr chain and 8-level formula
  chain (`src/parser.rs`); ISO/IEC 14977 (EBNF) - the machine-readable
  grammar mirror (`src/ebnf_grammar.rs::GRAMMAR_RULES`, 61 rules pinned to
  the parser by sync tests); nom 8 - the parser-combinator foundation with
  the two-tier error model (`src/parser.rs`).
- **Automata theory**: Hopcroft/Motwani/Ullman, "Introduction to Automata
  Theory, Languages, and Computation" - the generated production artifact is
  a finite-state machine over event inputs (`src/backends/rust_fsm.rs`,
  `src/backends/c_fsm.rs`); monitor check trees evaluated as per-sample state
  machines over traces (`src/monitor.rs`).
- **Compiler construction**: Aho/Lam/Sethi/Ullman, "Compilers: Principles,
  Techniques, and Tools" (Dragon Book) - the two-stage parse-then-validate
  frontend, recursive descent with single-lookahead decision points, IR as
  the single normalized form rendered by per-language backends
  (`src/parser.rs`, `src/validator.rs`, `src/codegen.rs`).
Add the anchor above the heading; add the fragment ID to the `//! Docs:` line
of `src/parser.rs`. Wire a trailing `Refs:` line into ALGORITHMS.md sections
"Parser construction + validation passes" and "IR normalization + codegen
pipeline". Run the gate.

### Action 7.2: `dev:references-model-checking` - model checking, SMT/SAT solving

Same shape for the MC/monitor area. Candidate entries (pick at execution time
from what the code actually uses): Clarke/Grumberg/Peled "Model Checking"
(the standard text - LTL model checking, Büchi automata, fairness); Tarjan,
"Depth-first search and linear-time algorithms for strongly-connected
components" (1972) - SCC in the liveness cycle analysis (`src/ref_mc.rs`
iterative Tarjan; generated engines use Kosaraju over the stay-edge subgraph);
IEEE 1850 PSL standard - RFS's property language is a PSL-like subset
(`src/monitor.rs`, `src/parser.rs`); de Moura/Bjørner, "Z3: An Efficient SMT
Solver" (TACAS 2008) + the z3 tool - the bounded path (`src/smt_plan.rs`,
`src/smt_emit.rs`). Read `src/mc_plan.rs`, `src/ref_mc.rs`, `src/smt_plan.rs`,
`src/monitor.rs` first. Owner: `src/ref_mc.rs`. Wire a trailing `Refs:` line
into ALGORITHMS.md sections "MC engine - exact / liveness / SMT paths" and
"Monitor compilation (PSL translation)". Run the full gate (dev_docs_test +
docs_md_test).

## Task exit criteria

- [x] 7.1: parsing/automata/compiler-construction section filled (grouped
      entries, each with one-line annotation + RFS component); anchor added;
      fragment owned by `src/parser.rs`; Refs: lines wired into ALGORITHMS
      sections "Parser construction + validation passes" and "IR normalization
      + codegen pipeline"; no TBD in that section; gate green.
- [ ] 7.2: model-checking/SMT section filled the same way; no TBD anywhere
      in REFERENCES.md; full gate green (dev_docs_test + docs_md_test).
- [ ] Every doc-to-code reference resolves (gate check 1); each new fragment
      owned (no orphans); source edits limited to `//! Docs:` lines.
- [ ] Plan rows 7.1-7.2 marked DONE; this file's boxes checked; one git
      commit per subtask; `TASKS.md` entry appended after 7.2.
- [ ] Full verification green at task end: cargo build/test/clippy/fmt
      (+ --workspace).

## Non-goals

- No exhaustive bibliography - a curated index of what RFS actually uses;
  no new external dependencies; no behavior changes of any kind; no links
  from docs outside ALGORITHMS.md (broader cross-linking is task 14's scope);
  no mermaid/plantuml diagrams in this file (prose index).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
