# Phase 7, Task 10 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `info-json` format entry (D-P7-8): the `rfs info <spec> --json`
introspection document in md + JSON Schema draft-07, with the D-P7-9 walk of
real documents over all 50 valid examples.

## Context

- Phase 7 plan: rows 10, 10.1-10.2; decisions D-P7-8/D-P7-9.
- Tasks 6-9 landed the shared schema emitter/walker and four entries; this task
  reuses them unchanged (no `json_schema.rs` changes needed).
- The `rfs info --json` writer (`render_json` / `module_json`) lives in
  `src/info.rs` - NOT `ir.rs` as the plan said (name drift, correction 10.1).

## Scope

### In scope
- info-json field model (`src/info_format.rs`) + entry registration (outputs:
  md, schema).
- Tests: schema pin, md sections pin, walk of all 50 valid examples in-process
  via the exact command path.
- BUILD.md row update.

### Out of scope
- diagram-subset entries (tasks 11-14), artifact-header entry (task 14).

## Planned actions

1. **Field model** (`src/info_format.rs`): model from the `rfs info --json`
   writer: `name`, `version`, `initial_state`, `enums`, `events`, `outputs`,
   `vars`, `transitions` (an INT count - document explicitly, it is not an
   array), `on_error`, `properties`, `assumptions` with `monitor_status`.
   Register entry (outputs: md, schema).
2. **Tests** (`tests/info_format_test.rs`): run in-process over all 50 valid
   examples; walk check; pin.
3. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | changed - info-json field model (root = modules array over the shared `info_module` definition, 9 definitions; `transitions` documented as an integer count NOT an array; `monitor_status` a plain open-ended string) + entry registration |
| `tests/info_format_test.rs` | changed - 3 new tests (schema pin, md sections pin, walk of all 50 valid examples) |
| `BUILD.md` | changed - section 3 info-json row; baseline re-pin |

## Task deliverables

- `rfs info format info-json --output md|schema` works.
- Real `rfs info --json` documents for all 50 valid examples walk clean against
  the model (transitions count cross-checked against the IR).
- Full test matrix green.

## Task exit criteria

- [x] Field model covers the full introspection document (DONE: built against `src/info.rs` `render_json`/`module_json`; 9 definitions)
- [x] `transitions` documented as an integer count, not an array (DONE: explicit in md + schema)
- [x] Schema document pin + md sections pin (DONE: `info_json_schema_document_pin`, `info_json_md_pins_sections`)
- [x] Walk of all 50 valid examples in-process via the exact command path; transitions count cross-checked against the IR; corpus coverage on_error + assumptions + skipped monitors (DONE: `info_json_walk_all_valid_examples`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 518 / workspace 542 / smt 520, info_format_test 30)

## Non-goals

- No changes to the `rfs info --json` wire format.
- No `json_schema.rs` changes - the task 6/7 emitter/walker sufficed.

## Task corrections

- **10.1** (2026-10-27): the plan's `ir.rs` `render_json` does not exist there -
  the `rfs info --json` writer (`render_json` / `module_json`) lives in
  `src/info.rs`. The model is built against those; same name-drift class as
  task 7's `mc_report_json.rs` -> `codegen_mc.rs`.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (no per-task work notes existed for this task). Sources: commit
  `da548fe` message + diff, `PHASE_7_PLAN.md` corrections table, BUILD.md
  baseline re-pins. NOTE: the TASKS.md entry for task 10 was never recorded -
  see the protocol-violation report.
- Committed as `da548fe` (2026-10-27), 4 files changed (+498/-10).
