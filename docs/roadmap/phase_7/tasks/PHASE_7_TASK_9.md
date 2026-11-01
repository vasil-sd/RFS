# Phase 7, Task 9 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `validate-json` format entry (D-P7-8): `rfs validate --json` diagnostics in
md + JSON Schema draft-07, with the D-P7-9 walk of real validation runs over
the example corpus.

## Context

- Phase 7 plan: rows 9, 9.1-9.2; decisions D-P7-8/D-P7-9.
- Tasks 6-8 landed the shared schema emitter/walker and three entries; this
  task reuses them unchanged (no `json_schema.rs` changes needed).
- The example corpus: 50 valid + 13 invalid examples (both error kinds present
  among the invalid ones).

## Scope

### In scope
- validate-json field model (`src/info_format.rs`) + entry registration
  (outputs: md, schema).
- Tests: schema pin, md sections pin, walks over all 13 invalid examples
  (file mode), all valid files (`ok=true`), directory mode over both example
  directories.
- BUILD.md row update.

### Out of scope
- info-json entry (task 10), diagram-subset entries (tasks 11-14).

## Planned actions

1. **Field model** (`src/info_format.rs`): file mode: `{file, ok,
   errors:[{kind: parse|semantic, line, col, message}]}`; directory mode:
   array of the same objects. Register entry (outputs: md, schema).
2. **Tests** (`tests/info_format_test.rs`): run in-process over all 13 invalid
   examples (both error kinds must be observed) + valid files (`ok=true`);
   walk check; pin.
3. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | changed - validate-json field model (union root = file-mode object \| directory-mode array over the shared `validate_file` definition; `kind` enum parse/semantic; strict definitions) + entry registration |
| `tests/info_format_test.rs` | changed - 5 new tests (schema pin, md sections pin, invalid-examples walk both kinds, valid-files ok=true walk, directory-mode walk) |
| `BUILD.md` | changed - section 3 validate-json row; baseline re-pin |

## Task deliverables

- `rfs info format validate-json --output md|schema` works.
- Real validation runs over the whole example corpus walk clean against the
  model.
- Full test matrix green.

## Task exit criteria

- [x] Field model covers file mode + directory mode (DONE: union root over the shared `validate_file` definition)
- [x] Schema document pin + md sections pin (DONE: `validate_json_schema_document_pin`, `validate_json_md_pins_sections`)
- [x] Walk of all 13 invalid examples in file mode, both error kinds observed (DONE: `validate_json_walk_invalid_examples_both_kinds`)
- [x] Walk of every valid example with `ok=true` (DONE: `validate_json_walk_valid_files_ok_true`)
- [x] Directory-mode walk over both example directories (all-failing array + empty all-valid array) (DONE: `validate_json_walk_directory_mode`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 515 / workspace 539 / smt 517, info_format_test 27)

## Non-goals

- No changes to the `rfs validate --json` wire format.
- No `json_schema.rs` changes - the task 6/7 emitter/walker sufficed.

## Task corrections

None recorded.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (no per-task work notes existed for this task). Sources: commit
  `b7e1cae` message + diff, `PHASE_7_PLAN.md`, BUILD.md baseline re-pins.
  NOTE: the TASKS.md entry for task 9 was never recorded - see the
  protocol-violation report.
- Committed as `b7e1cae` (2026-10-27), 3 files changed (+404/-10).
