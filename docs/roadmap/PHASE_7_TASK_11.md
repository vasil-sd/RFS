# Phase 7, Task 11 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `diagram-dot` format entry (md only): a human-readable description of the
DOT subset that `rfs diagram --format dot` generates - digraph/rankdir, node
attributes (fillcolor/filled/shape per state kind), edge label grammar
(`Event [guard] -> next`, dashed on_error edges), string escaping rules - with
a coverage sync test proving no undocumented token appears in generated output.

## Context

- Phase 7 plan: row 11; decisions D-P7-9 (sync guarantees).
- Tasks 1-3 landed the DOT generator (`src/diagrams/dot.rs` `generate` - the
  single source of truth); this task documents its emitted subset.
- The entry has outputs: md only (no JSON schema - it describes a text format,
  not a JSON document).

## Scope

### In scope
- DOT subset documentation (`DOT_SUBSET_MD`) + entry registration in
  `src/info_format.rs` (outputs: md).
- Coverage sync test over all 50 valid examples x every module.
- BUILD.md row update.

### Out of scope
- diagram-puml / diagram-mmd entries (tasks 12/13), artifact-header entry
  (task 14).

## Planned actions

1. **Subset documentation + entry** (`src/info_format.rs`): document the fixed
   3-line header, node line shapes (filled/fillcolor IFF initial state), edge
   label grammar (`Event [guard] -> next`, dashed on_error edges), quote
   escaping - derived from `diagrams/dot.rs` `generate`. Register entry
   (outputs: md).
2. **Coverage sync test** (`tests/info_format_test.rs`): assert the documented
   attribute/construct set covers everything `generate()` can emit for all 50
   examples (no undocumented token appears in generated output) + corpus pins
   for guard/struct-event/on_error coverage.
3. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | changed - `DOT_SUBSET_MD` documentation + `diagram_dot_render` + entry registration (outputs: md) |
| `tests/info_format_test.rs` | changed - 2 new tests (+ `check_dot_documented_subset` helper): md sections pin, generated-output coverage sync over all 50 valid examples x every module |
| `BUILD.md` | changed - section 3 diagram-dot row; baseline re-pin |

## Task deliverables

- `rfs info format diagram-dot --output md` works.
- Every generated DOT line is one of the documented shapes filled with IR data
  (pinned by test over the whole corpus).
- Full test matrix green.

## Task exit criteria

- [x] DOT subset documented: fixed 3-line header, node line shapes with filled/fillcolor IFF initial state, edge label grammar `Event [guard] -> next`, dashed on_error edges, quote escaping (DONE: `DOT_SUBSET_MD`)
- [x] Entry registered (outputs: md) (DONE: in `FORMATS`)
- [x] Coverage sync test over all 50 valid examples x every module: no undocumented token appears in generated output (DONE: `diagram_dot_generated_output_matches_documented_subset`)
- [x] Corpus pins for guard/struct-event/on_error coverage (DONE: inside the coverage test)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 520 / workspace 544 / smt 522, info_format_test 32)

## Non-goals

- No generator changes - the documentation follows `generate()` as-is.
- No JSON schema output (text format, not a JSON document).

## Task corrections

None recorded.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (no per-task work notes existed for this task). Sources: commit
  `536c8e8` message + diff, `PHASE_7_PLAN.md`, BUILD.md baseline re-pins.
  NOTE: the TASKS.md entry for task 11 was never recorded - see the
  protocol-violation report.
- Committed as `536c8e8` (2026-10-27), 3 files changed (+292/-10).
