# Phase 7, Task 12 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `diagram-puml` format entry (md only): the same pattern as task 11 for the
PlantUML subset that `rfs diagram --format puml` generates - state declarations
per kind, note-on-link forms, dashed bracket edges for on_error, `\n` label
convention, escaping - with the coverage sync test against generated output.

## Context

- Phase 7 plan: row 12; decisions D-P7-9 (sync guarantees).
- Task 2 landed the PlantUML generator (`src/diagrams/plantuml.rs` `generate` -
  the single source of truth), including the correction-2.2/2.3 emission
  change (bare `note on link`, one line per output); this task documents that
  final subset.
- The entry has outputs: md only (no JSON schema - it describes a text format,
  not a JSON document).

## Scope

### In scope
- PlantUML subset documentation (`PUML_SUBSET_MD`) + entry registration in
  `src/info_format.rs` (outputs: md).
- Coverage sync test over all 50 valid examples x every module.
- BUILD.md row update.

### Out of scope
- diagram-mmd entry (task 13), artifact-header entry (task 14).

## Planned actions

1. **Subset documentation + entry** (`src/info_format.rs`): document the frame
   lines with IFF-initial `[*] -->` line, transition label grammar
   (`Event \n guard` with the literal two-char `\n`), note-on-link lines for
   emitted outputs, dashed bracketed on_error edges, verbatim names without
   escaping - derived from `diagrams/plantuml.rs` `generate`. Register entry
   (outputs: md).
2. **Coverage sync test** (`tests/info_format_test.rs`): same pattern as task
   11 - every generated line is one of the documented shapes filled with IR
   data (no undocumented tokens) + corpus pins for guard/struct-event/
   on_error/note-on-link coverage.
3. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | changed - `PUML_SUBSET_MD` documentation + `diagram_puml_render` + entry registration (outputs: md) |
| `tests/info_format_test.rs` | changed - 2 new tests (+ `check_puml_documented_subset` helper): md sections pin, generated-output coverage sync over all 50 valid examples x every module |
| `BUILD.md` | changed - section 3 diagram-puml row; baseline re-pin |

## Task deliverables

- `rfs info format diagram-puml --output md` works.
- Every generated PlantUML line is one of the documented shapes filled with IR
  data (pinned by test over the whole corpus).
- Full test matrix green.

## Task exit criteria

- [x] PlantUML subset documented: frame lines with IFF-initial `[*] -->` line, transition label grammar `Event \n guard` (literal two-char `\n`), note-on-link lines for emitted outputs, dashed bracketed on_error edges, verbatim names without escaping (DONE: `PUML_SUBSET_MD`)
- [x] Entry registered (outputs: md) (DONE: in `FORMATS`)
- [x] Coverage sync test over all 50 valid examples x every module: no undocumented token appears in generated output (DONE: `diagram_puml_generated_output_matches_documented_subset`)
- [x] Corpus pins for guard/struct-event/on_error/note-on-link coverage (DONE: inside the coverage test)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 522 / workspace 546 / smt 524, info_format_test 34)

## Non-goals

- No generator changes - the documentation follows `generate()` as-is.
- No JSON schema output (text format, not a JSON document).

## Task corrections

None recorded.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (no per-task work notes existed for this task). Sources: commit
  `24c84f9` message + diff, `PHASE_7_PLAN.md`, BUILD.md baseline re-pins.
  NOTE: the TASKS.md entry for task 12 was never recorded - see the
  protocol-violation report.
- Committed as `24c84f9` (2026-10-27), 3 files changed (+303/-10).
