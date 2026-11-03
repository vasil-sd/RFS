# Phase 10, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_5.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 66 function rows (codegen.rs 13, codegen_mc.rs 17, codegen_test.rs 36) + 38 type rows (codegen.rs 10, codegen_mc.rs 11, codegen_test.rs 17) appended to `## All functions` / `## All types`; Component/Functionality still blank |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The codegen-core cluster is now dependency-annotated: codegen.rs's plugin
  API (Backend/BACKENDS registry, CodegenCommandKind::run/run_module dispatch
  with the D-P6-3 spec-hash stamping via Ir::to_json + spec_hash16,
  parse_codegen_args with all command-scope option rejections) references the
  IR document types and the codegen_mc wire enums; codegen_mc.rs's exact-mode
  taxonomy (ExactStatus/FailReason wire/human/exit_code, the D-P4-4 summary
  line) and the Phase 5/6 JSON report model (JsonIo/JsonStep/
  JsonCounterexample/McJsonReport with the pinned key-order contract) build on
  the json_rw value type + constructors; codegen_test.rs's planner
  (build_test_plan -> plan_transitions/select_invariants/
  plan_monitor_violations/build_random_trace/build_fuzz_decode) references the
  validator typing spine (Env::for_module/with_binding, type_to_expr_type,
  type_expr for width_of/is_bool_expr), the monitor signal family
  (formula_to_signal, SignalExpr/SignalAtom), codegen_fsm's FsmVariants, and
  its own static-evaluation core (eval_expr mirroring the backend lowering
  exactly, capped assignment enumeration, LCG shared with the fuzz wire
  format).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green; no unescaped pipes in the new rows |

## Found problems/issues/mismatches

- The plan's counting command (`grep -cE '^[[:space:]]*(pub )?(struct|enum|
  trait|type) '`) counts codegen.rs as 12 types, but 2 of those matches are
  `type Err = String;` associated-type bindings INSIDE the FromStr impls of
  McMode/AssumptionSelection - not standalone data types. Authoritative
  read-through count: 10 real types in codegen.rs, so 66 fns / 38 types for
  the chunk (was ~66/~40 per correction 2a). Same class of counter phantom as
  the emitter template fns noted in correction 2a; recorded here as a task
  correction - no plan correction needed (correction 2a already anticipates
  that read-through establishes authoritative counts).
- The same stale-table pattern as task 4: the task file's Scope/Planned-
  actions said "~110 function rows" (test-inclusive) while the Goal and
  correction 2a say 66 - resolved to 66 non-test fns per the boundary rule
  (the 44 inline tests belong to task 29).

## Other important findings/information

- First cross-chunk forward edge in the inventory: CodegenCommandKind::run
  depends on `src/spec_hash.rs::spec_hash16`, whose row arrives with chunk K
  (task 12). The edge is recorded now so impact analysis is complete once all
  chunks land.
- Trait-impl identifier convention applied to five hand-written impls:
  McMode(FromStr)::from_str, AssumptionSelection(FromStr)::from_str,
  BackendOptions(Default)::default, McConfig(Default)::default,
  EvalEnv(Debug)::fmt; derived impls (Debug/Clone/PartialEq/Eq) get no rows,
  consistent with tasks 2-3.
