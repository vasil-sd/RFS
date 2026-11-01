# Phase 7, Task 7 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `ce-trace` format entry (D-P7-8): the MC counterexample trace - the JSON
document embedded in MC reports plus the default text trace format - described
in md + JSON Schema draft-07, with the D-P7-9 walk of REAL FAIL-variant reports
on both backends.

## Context

- Phase 7 plan: rows 7, 7.1-7.2; decisions D-P7-8/D-P7-9.
- Task 6 landed the shared schema emitter/walker (`src/json_schema.rs`); this
  task reuses it and extends it where ce-trace needs free-keyed value maps
  (correction 7.1).
- The counterexample JSON builder + `JSON_KEY_*` constants live in
  `src/codegen_mc.rs` (the plan's `mc_report_json.rs` does not exist - name
  drift, correction 7.1).
- Pinned FAIL variants: RING_NO_GRANT_SRC / ELEV_NO_UP_SRC + the two
  one-line-delta mutations, previously inlined in `tests/benchmark_mc_test.rs`.

## Scope

### In scope
- ce-trace field model (`src/info_format.rs`) + entry registration (outputs:
  md, schema).
- `json_schema.rs` extensions: `FreeObj(inner)`, `Union(alts)`,
  `SchemaDoc.extra_sections` (md-only sections - the text trace format has no
  schema); `render_md` renders a tagged-union ROOT's kind table.
- Shared FAIL variants moved to `tests/common/fail_variants.rs`.
- Tests: schema pin, md sections pin, walks of real reports (trace + lasso on
  both backends, deadlock-config via example 04, ref_mc in-process).
- BUILD.md row update.

### Out of scope
- mc-report / validate-json / info-json entries (tasks 8-10).

## Planned actions

1. **Field model** (`src/info_format.rs`): model built against `JSON_KEY_*`
   constants: `kind` enum (`trace` | `lasso` | `deadlock-config`),
   `initial_state` / `initial_vars` / `steps` (trace), `prefix` / `cycle` /
   `cycle_length` (lasso), `depth` / `state` / `vars` (deadlock); step shape:
   `input{variant,fields}`, `outputs[]`, `from_state`, `to_state`, `vars`; plus
   a md section describing the default text trace format (`step N` lines as
   printed by MC). Register entry (outputs: md, schema).
2. **Tests** (`tests/info_format_test.rs`): pinned FAIL-variant benchmark runs
   (reuse the specs from `benchmark_mc_test.rs`) produce real reports on both
   backends; walk check against the model; pin.
3. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | changed - ce-trace field model (trace/lasso/deadlock-config kinds, free-keyed value maps) + entry registration; md section for the text trace format |
| `src/json_schema.rs` | changed - `FreeObj(inner)` (arbitrary keys; `additionalProperties` = inner schema), `Union(alts)` (plain anyOf; ce_value = boolean\|integer\|string), `SchemaDoc.extra_sections` (md-only sections); `render_md` renders a tagged-union ROOT's kind table |
| `tests/common/fail_variants.rs` | new - pinned FAIL variants shared by `benchmark_mc_test` and `info_format_test` (RING_NO_GRANT_SRC, ELEV_NO_UP_SRC, the two one-line-delta mutations + `spec_source`/`mutate_spec`) |
| `tests/common/mod.rs` | changed - `pub mod fail_variants;` |
| `tests/benchmark_mc_test.rs` | changed - consumes the shared variants from `tests/common/fail_variants.rs` instead of inlining them |
| `tests/info_format_test.rs` | changed - 5 new tests (schema pin, md sections pin, trace+lasso walk both backends, deadlock-config walk both backends, ref_mc in-process) |
| `BUILD.md` | changed - section 3 ce-trace row; baseline re-pin |

## Task deliverables

- `rfs info format ce-trace --output md|schema` works.
- Real FAIL-variant reports on both backends walk clean against the model.
- Full test matrix green.

## Task exit criteria

- [x] Field model covers all three kinds + step shape (DONE: built against `src/codegen_mc.rs` `JSON_KEY_*` constants)
- [x] Free-keyed value maps expressible (`initial_vars`/`vars`/`fields`) (DONE: `FreeObj`/`Union` added to `json_schema.rs`, correction 7.1)
- [x] Schema document pin + md sections pin incl. the text trace format section (DONE: `ce_trace_schema_document_pin`, `ce_trace_md_pins_sections`)
- [x] Walk of real FAIL-variant reports on both backends (trace + lasso) (DONE: `ce_trace_walk_trace_and_lasso_both_backends`)
- [x] Deadlock-config kind walked (example 04, both backends) (DONE: `ce_trace_walk_deadlock_config_both_backends`)
- [x] ref_mc in-process walk (DONE: `ce_trace_walk_ref_mc_in_process`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 506 / workspace 530 / smt 508, info_format_test 18)

## Non-goals

- No changes to the counterexample JSON wire format.
- The walker enforces value types on free-keyed objects but never their keys -
  the sync guarantee stays "no undocumented KEY at any object level" for fixed
  objects and "every value conforms" for maps.

## Task corrections

- **7.1** (2026-10-27): the plan's `mc_report_json.rs` does not exist - the
  `JSON_KEY_*` constants and the counterexample JSON builder live in
  `src/codegen_mc.rs`. The model is built against those (module alias `mc`).
  Name drift between the plan (written from the Phase 5 record) and the shipped
  tree; the constants are the same, only the home file differs.
- **7.1/7.2** (2026-10-27): the pinned FAIL variants moved OUT of
  `tests/benchmark_mc_test.rs` into a new shared module
  `tests/common/fail_variants.rs` (RING_NO_GRANT_SRC, ELEV_NO_UP_SRC, the two
  one-line-delta mutations + `spec_source`/`mutate_spec`); benchmark_mc_test now
  consumes them from there. The ce-trace walk additionally uses example 04 for
  the `deadlock-config` kind. Duplicating ~220 lines of variant spec text into a
  second suite is exactly the drift class this project pins against (the
  mutate_spec anchor pattern exists for that reason). No benchmark_mc_test FAIL
  variant produces a deadlock, so the third kind needs the example 04 case
  backend_parity_test already pins.
- **7.1** (2026-10-27): the task 6 field model could not express ce-trace's
  name->value maps (`initial_vars`/`vars`/`fields` - KEYS are spec-defined
  variable/field names, values are bool | bit<N> | enum). `json_schema.rs`
  gains two variants: `FreeObj(inner)` (arbitrary keys; JSON Schema
  `additionalProperties` = inner schema, no fixed properties) and `Union(alts)`
  (plain anyOf; the ce_value = boolean|integer|string encoding), plus
  `SchemaDoc.extra_sections` for md-only sections (the text trace format has no
  schema). `render_md` also renders a tagged-union ROOT's kind table in the
  document section. Tasks 8-10 reuse the same emitter/walker unchanged.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (no per-task work notes existed for this task). Sources: commit
  `c5c5308` message + diff, `PHASE_7_PLAN.md` corrections table, BUILD.md
  baseline re-pins. NOTE: the TASKS.md entry for task 7 was never recorded -
  see the protocol-violation report.
- Committed as `c5c5308` (2026-10-27), 8 files changed (+1242/-376).
