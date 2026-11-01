# Phase 7, Task 16 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Phase 7 closeout: full verification matrix green on the final tree, docs sync
(DESIGN.md section 25 with D-P7-1..D-P7-13, KB.md, START.md, docs/user),
`PHASE_7_STATUS.md` per template 2.3, INTRO.md row -> COMPLETE, TASKS.md entry,
single commit.

## Context

- Phase 7 plan: row 16; the phase ships as one documented unit (tasks 1-15
  leave a working tree that stays test-green after each task).
- Decisions to record in DESIGN.md section 25: D-P7-1..D-P7-13 (internal
  renderer default, renderer split, pinned git dependency, SVG-only crate
  features, print-script internal variant, info-format CLI shape, grammar model
  mirrors the parser, JSON Schema draft-07 choices, sync guarantees,
  syntax-correctness-only parity, no new examples, training scope).
- The plantuml-core pinned rev (`8066f6124a86ed5da59c6d7d570f0568891c378e`)
  must be recorded in `PHASE_7_STATUS.md` (D-P7-3 requirement).

## Scope

### In scope
- Full verification matrix from BUILD.md section 5.
- DESIGN.md new section 25 with D-P7-1..D-P7-13.
- KB.md sync: section 1 state, section 4.2 source map (+ `info_format.rs`,
  `ebnf_grammar.rs`, `json_schema.rs`), section 4.5 CLI behaviour (`diagram
  --renderer`, `info format`).
- docs/user: README (internal rendering in the pipeline picture), INTEGRATION.md
  line ~205 (tool-invoking commands no longer include diagram by default),
  MIGRATION.md v1.1 note (`--render svg` now renders in-process;
  `--renderer external` restores old behaviour).
- START.md update; INTRO.md table row -> COMPLETE with summary.
- `PHASE_7_STATUS.md` per template 2.3 (incl. the plantuml-core pinned rev).
- TASKS.md entry; single commit.

### Out of scope
- Any new functionality - closeout is verification + documentation only.

## Planned actions

1. **Full verification matrix** (BUILD.md section 5): build/test/clippy/fmt +
   workspace, test-generated-code, docs gate, benchmark suite - all green on
   the final tree; counts re-pinned in BUILD.md.
2. **DESIGN.md**: new section 25 with D-P7-1..D-P7-13 as shipped (plus
   execution notes where behaviour differs from the plan sketch).
3. **KB.md sync**: section 1 state, section 4.2 source map (+ `info_format.rs`,
   `ebnf_grammar.rs`, `json_schema.rs`), section 4.5 CLI behaviour (`diagram
   --renderer`, `info format`).
4. **docs/user**: README (internal rendering in the pipeline picture),
   INTEGRATION.md line ~205 (tool-invoking commands no longer include diagram
   by default), MIGRATION.md v1.1 note (`--render svg` now renders in-process;
   `--renderer external` restores old behaviour).
5. **Bookkeeping**: START.md update; INTRO.md table row -> COMPLETE with
   summary; `PHASE_7_STATUS.md` per template 2.3 (incl. the plantuml-core
   pinned rev); TASKS.md entry; single commit summarizing Phase 7.

#### Affected files

| File | Operation |
|---|---|
| `DESIGN.md` | change - new section 25 with D-P7-1..D-P7-13 |
| `KB.md` | change - section 1 state, section 4.2 source map (+ `info_format.rs`, `ebnf_grammar.rs`, `json_schema.rs`), section 4.5 CLI behaviour (`diagram --renderer`, `info format`) |
| `START.md` | change - phase-state update |
| `docs/user/README.md` | change - internal rendering in the pipeline picture |
| `docs/user/INTEGRATION.md` | change - line ~205: tool-invoking commands no longer include diagram by default |
| `docs/user/MIGRATION.md` | change - v1.1 note: `--render svg` now renders in-process; `--renderer external` restores old behaviour |
| `docs/roadmap/INTRO.md` | change - table row 7 -> COMPLETE with summary |
| `docs/roadmap/PHASE_7_STATUS.md` | new - per template 2.3 (incl. the plantuml-core pinned rev) |
| `TASKS.md` | change - closeout entry |

## Task deliverables

- Full verification matrix green on the final tree.
- All documentation synced to the shipped Phase 7 state.
- `PHASE_7_STATUS.md` written; INTRO.md row COMPLETE; single commit.

## Task exit criteria

- [ ] Full verification matrix from BUILD.md section 5 green (build/test/clippy/fmt + workspace, test-generated-code, docs gate, benchmark suite)
- [ ] DESIGN.md section 25 with D-P7-1..D-P7-13 as shipped
- [ ] KB.md synced (section 1 state, section 4.2 source map, section 4.5 CLI behaviour)
- [ ] docs/user updated (README pipeline picture, INTEGRATION.md tool-invoking list, MIGRATION.md v1.1 note)
- [ ] START.md + INTRO.md table row -> COMPLETE with summary
- [ ] `PHASE_7_STATUS.md` per template 2.3 (incl. the plantuml-core pinned rev `8066f6124a86ed5da59c6d7d570f0568891c378e`)
- [ ] TASKS.md entry; single commit summarizing Phase 7

## Non-goals

- No new functionality, no example changes.
- No re-opening of completed tasks - fixes go into follow-up phase plans.
