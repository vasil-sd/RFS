# Phase 7, Task 8 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The `mc-report` format entry (D-P7-8): the full exact-mode MC report document
(`--counterexample-format json`; status, metrics, statistics + counterexample)
described in md + JSON Schema draft-07, with the D-P7-9 walk of REAL reports.

## Context

- Phase 7 plan: rows 8, 8.1-8.2; decisions D-P7-8/D-P7-9.
- Task 7 landed the ce-trace entry; this task's `counterexample` definition
  REUSES the shared ce-trace model via `$ref` (correction 8.1), so the two
  documents cannot drift.
- The report JSON constants live in `src/codegen_mc.rs` (module alias `mc`).

## Scope

### In scope
- mc-report field model (`src/info_format.rs`) + entry registration (outputs:
  md, schema).
- Shared `counterexample_model()` builder extracted from the ce-trace entry;
  `json_schema::emit_type` made pub for the cross-check.
- Tests: schema pin, md sections pin, walk of real reports (6 benchmark specs
  pass + FAIL variants + deadlock + unknown-state-limit on both backends,
  ref_mc in-process).
- BUILD.md row update.

### Out of scope
- validate-json / info-json entries (tasks 9-10).

## Planned actions

1. **Field model** (`src/info_format.rs`): model from `JSON_KEY_*`: `status`
   enum (`pass-complete` | `fail` | `unknown-state-limit`),
   `metrics{states,transitions,max_depth}`, fail fields (`reason`, `label`,
   `step`), `statistics{unexpected_terminations, monitor_size_bits,
   memory_estimate_bytes, peak_rss_kb}`, `counterexample` via `$ref` to the
   ce-trace model (D-P7-8). Register entry (outputs: md, schema).
2. **Tests** (`tests/info_format_test.rs`): all 6 benchmark specs x both
   backends (pass + fail variants) produce real reports; walk check against the
   model; pin.
3. **Docs** (`BUILD.md`): section 3 row update.

#### Affected files

| File | Operation |
|---|---|
| `src/info_format.rs` | changed - mc-report field model + entry registration (outputs: md, schema); ce-trace root + sub-definitions extracted into the shared `counterexample_model()` builder |
| `src/json_schema.rs` | changed - `emit_type` made pub (cross-check of the reused counterexample definition) |
| `tests/info_format_test.rs` | changed - 4 new tests (schema pin, md sections pin, benchmarks walk both backends, ref_mc in-process) |
| `BUILD.md` | changed - section 3 mc-report row; baseline re-pin |

## Task deliverables

- `rfs info format mc-report --output md|schema` works.
- The emitted `counterexample` definition is byte-identical to the ce-trace
  root (pinned by test).
- Full test matrix green.

## Task exit criteria

- [x] Field model covers status/metrics/fail fields/statistics/counterexample (DONE: built against `src/codegen_mc.rs` constants)
- [x] `counterexample` reuses the shared ce-trace model via `$ref`; byte-identical definition pinned (DONE: `counterexample_model()` builder; cross-check test)
- [x] Schema document pin + md sections pin (DONE: `mc_report_schema_document_pin`, `mc_report_md_pins_sections`)
- [x] Walk of real reports: 6 benchmark specs pass + FAIL variants + deadlock + unknown-state-limit on both backends (DONE: `mc_report_walk_benchmarks_both_backends`)
- [x] ref_mc in-process walk (DONE: `mc_report_walk_ref_mc_in_process`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 510 / workspace 534 / smt 512, info_format_test 22)

## Non-goals

- No changes to the MC report JSON wire format.
- No if/then conditionals in the schema (draft-07 widest-validator-support
  choice - D-P7-8); fail-field IFF-fail presence is pinned by the walk test
  over real runs instead.

## Task corrections

- **8.1** (2026-10-27): ce-trace's root + sub-definitions were extracted into a
  shared `counterexample_model() -> (FieldType, Vec<definitions>)` builder: the
  mc-report entry's `counterexample` definition REUSES it (D-P7-8's `$ref` to
  the ce-trace model), so the two documents cannot drift - a test pins the
  emitted `counterexample` definition byte-identical to the ce-trace root.
  `json_schema::emit_type` became pub for that cross-check.
- **8.1** (2026-10-27): the status/reason enum values have no pub string
  constants (only `ExactStatus::wire()` / `FailReason::wire()` methods) - the
  model derives them from the methods at build time, so a wire-name change
  follows automatically (no string literals, per the D-P7-9 policy). The fail
  fields (`reason`/`label`/`step`/`counterexample`) are modeled OPTIONAL
  (draft-07 without if/then conditionals - D-P7-8's widest-validator-support
  choice); their IFF-fail presence is pinned by the walk test over real runs
  instead.

## Other information

- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (no per-task work notes existed for this task). Sources: commit
  `aa49438` message + diff, `PHASE_7_PLAN.md` corrections table, BUILD.md
  baseline re-pins. NOTE: the TASKS.md entry for task 8 was never recorded -
  see the protocol-violation report.
- Committed as `aa49438` (2026-10-27), 5 files changed (+689/-115).
