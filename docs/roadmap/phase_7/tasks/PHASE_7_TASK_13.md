# Phase 7, Task 13 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `diagram-mmd` format entry (md only): the same pattern as tasks 11-12 for
the Mermaid subset that `rfs diagram --format mmd` generates -
`stateDiagram-v2`, `[*]` edges, edge label line grammar incl. `[on_error]`
prefix and multi-line labels, escaping - with the coverage sync test against
generated output.

## Context

- Phase 7 plan: row 13; decisions D-P7-9 (sync guarantees).
- Task 1 landed the Mermaid generator (`src/diagrams/mermaid.rs` - the single
  source of truth); this task documents its emitted subset. Runs after tasks
  1-3 since generator adjustments there could change the documented subset.
- The entry has outputs: md only (no JSON schema - it describes a text format,
  not a JSON document).
- Current state at planning time: `src/info_format.rs` `FORMATS` holds 8 entries
  (spec-rfs, spec-json, ce-trace, mc-report, validate-json, info-json,
  diagram-dot, diagram-puml); `diagram-mmd` and `artifact-header` are the two
  missing ones (tasks 13/14).
- Mermaid specifics to document: no notes-on-links and no dotted/dashed edges
  in state diagrams (both verified as unsupported), so ALL transition
  information goes into the multi-line edge label (literal two-char `\n`):
  event, then guard (if any), then one `emits X` line per emitted output;
  `on_error` fallbacks prefix the first label line with `[on_error]`.

## Scope

### In scope
- Mermaid subset documentation + entry registration in `src/info_format.rs`
  (outputs: md).
- Coverage sync test over all 50 valid examples x every module.
- BUILD.md row update.

### Out of scope
- artifact-header entry (task 14), training-set update (task 15), closeout
  (task 16).

## Planned actions

1. **Subset documentation + entry** (`src/info_format.rs`): document the
   `stateDiagram-v2` header, state declarations, `[*]` start edges, transition
   edge lines with the multi-line label grammar (event / guard / `emits X`
   lines, `[on_error]` prefix), string escaping - derived from
   `diagrams/mermaid.rs` generation. Register entry (outputs: md).
2. **Coverage sync test** (`tests/info_format_test.rs`): same pattern as tasks
   11-12 - every generated line is one of the documented shapes filled with IR
   data (no undocumented tokens) + corpus pins for guard/struct-event/
   on_error/emits coverage.
3. **Docs** (`BUILD.md`): section 3 row update; baseline re-pin.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | change - Mermaid subset documentation + render fn + entry registration (outputs: md) |
| `tests/info_format_test.rs` | change - 2 new tests (+ check helper): md sections pin, generated-output coverage sync over all 50 valid examples x every module |
| `BUILD.md` | change - section 3 diagram-mmd row; baseline re-pin |

## Task deliverables

- `rfs info format diagram-mmd --output md` works.
- Every generated Mermaid line is one of the documented shapes filled with IR
  data (pinned by test over the whole corpus).
- Full test matrix green.

## Task exit criteria

- [ ] Mermaid subset documented: `stateDiagram-v2`, `[*]` edges, edge label line grammar incl. `[on_error]` prefix and multi-line labels, escaping
- [ ] Entry registered (outputs: md)
- [ ] Coverage sync test over all 50 valid examples x every module: no undocumented token appears in generated output
- [ ] Corpus pins for guard/struct-event/on_error/emits coverage
- [ ] Full test matrix green; BUILD.md baseline re-pinned

## Non-goals

- No generator changes - the documentation follows the generator as-is.
- No JSON schema output (text format, not a JSON document).
