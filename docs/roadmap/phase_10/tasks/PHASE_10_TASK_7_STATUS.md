# Phase 10, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `SRC_STRUCTURE.md` | changed | 71 function rows (rust.rs 7, rust_fsm.rs 41, rust_test.rs 16, rust_mc.rs 7) + 3 type rows (Gen, ExprCtx, GeneratedCrate) appended to `## All functions` / `## All types`; Component/Functionality still blank |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 7a recorded (chunk F template phantoms: authoritative 71/3, not ~131/~26; src/ totals ~935 fns / ~210 types) |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_7.md` | changed | "Task corrections" section: the ~134/~26 figures superseded by 71/3 with the per-file phantom breakdown |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The Rust backend is now dependency-annotated: rust.rs's mangling/type-mapping
  helpers are the shared vocabulary every emitter row references; rust_fsm.rs's
  Gen pipeline (prelude -> enums -> vars -> input/output -> renderers ->
  trace/handlers -> monitors -> fsm impl) cross-references the validator typing
  spine, the monitor plan family, and codegen_fsm flattening; rust_test.rs
  renders every codegen_test plan case type into test source; rust_mc.rs's
  exact_codegen is the single row that ties mc_plan (encoding/traversal/
  liveness), smt_plan/smt_emit (SMT mode), printer (formula text), and the
  JSON report contract together.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green; no unescaped pipes in this chunk's descriptions |

## Found problems/issues/mismatches

- MAJOR count mismatch vs correction 2a ("T7 131/26"): the emitters' raw-string
  code templates contain fn AND type definitions that line counters miscount.
  Read-through (authoritative per correction 2a's own rule) found 60 phantom
  fns and 23 phantom types: rust_fsm.rs 5 fn + 13 type phantoms (ProtocolFsm /
  trace-handler / Fsm raw strings), rust_test.rs 9 fn phantoms in format!
  strings (and NO real test module - its awk-era "25 test fns" started counting
  at a DOC COMMENT mentioning #[cfg(test)]), rust_mc.rs 46 fn + 10 type
  phantoms across the EXACT_ENGINE / LIV_ENGINE / JSON_WRITER_RUST / EXACT_RUN /
  MAIN_DISPATCH / FUZZ_RUNTIME / SMT_ENGINE_RUST raw strings (LevelOutcome
  included - it is template text, not a toolchain type). Authoritative: rust.rs
  7/0, rust_fsm.rs 41/2, rust_test.rs 16/1, rust_mc.rs 7/0 = 71 fns / 3 types.
  Plan correction 7a recorded; src/ totals now ~935 fns / ~210 types.

## Other important findings/information

- The cross-backend parity vocabulary is now in place for task 8: every
  rust_fsm/rust_mc row names the exact shared identifiers (validator typing,
  monitor plan, mc_plan encoding, codegen_test cases) that the C-backend rows
  must mirror.
- rust_mc.rs's JSON_WRITER_RUST raw string is the embedded copy of the
  toolchain json_rw writer for `--counterexample-format json` - its template
  fns (build_mc_json_doc, mc_json_io/step, trace_steps_json, ce_*_json) are
  NOT inventory rows; the toolchain-side originals live in codegen_mc.rs
  (task 5) and json_rw.rs (task 3).
- rust_test.rs's emit_fuzz_target consumes codegen_test::build_fuzz_decode -
  the fuzz decode plan flows planner -> emitter with no intermediate state.
