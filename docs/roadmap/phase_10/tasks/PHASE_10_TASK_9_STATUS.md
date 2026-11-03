# Phase 10, Task 9 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_9.md` are reached. Not
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
| `SRC_STRUCTURE.md` | changed | 89 function rows (main.rs 11, frontend.rs 21, ext_tool.rs 16, cli_log.rs 8, test_cmd.rs 19, gen_spec_cmd.rs 7, mc_ref_cmd.rs 7) + 19 type rows appended to `## All functions` / `## All types`; Component/Functionality still blank |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 9a recorded (frontend.rs associated-type phantom: authoritative 89/19, not ~89/~20; non-test src/ totals ~1,024 fns / ~225 types) |
| `docs/roadmap/phase_10/tasks/PHASE_10_TASK_9.md` | changed | count figures corrected to the re-verified 89/19 (Goal, cluster notes, Planned actions, Scope table); "five" -> "seven" manifest files; deliverables list completed with gen_spec_cmd.rs + mc_ref_cmd.rs; forward-edge note added to Context |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only phase.

## Removed functionality

None.

## New functionality

- The CLI-entry + tooling-command cluster is now dependency-annotated:
  main.rs's eight subcommand handlers are the single dispatch context every
  command row references (cmd_parse/cmd_validate on frontend, cmd_codegen on
  the codegen registry, cmd_test on test_cmd, cmd_diagram on the diagram
  cluster, mc-ref/gen-spec as thin run_cli shells); frontend.rs is the shared
  parse/validate spine (Format + conversion planning with the collision guard,
  text and structured validation paths); ext_tool.rs's resolver/Script pair is
  the shared external-tool contract every later tool command reuses; cli_log.rs
  is the stderr progress logger the new commands share.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - first run green after escaping two literal pipes in descriptions (cmd_diagram's internal\|external, run_cli's small\|medium); the trait-impl dependency form BackendOptions(Default)::default used per the task 5 convention |

## Found problems/issues/mismatches

- MAJOR type-count mismatch vs correction 2a ("T9 89/20"): frontend.rs's sixth
  type grep hit is `type Err = String;` - an associated-type binding inside the
  FromStr impl, not a standalone type. Same phantom class task 5's codegen.rs
  correction established (two `type Err = String` bindings there). The fn count
  (89) matched the plan exactly: main.rs 11/0, frontend.rs 21/5, ext_tool.rs
  16/4, cli_log.rs 8/2, test_cmd.rs 19/5, gen_spec_cmd.rs 7/1, mc_ref_cmd.rs
  7/2. Authoritative chunk H total: 89 fns / 19 types. Plan correction 9a
  recorded; non-test src/ totals become ~1,024 fns / ~225 types.
- MINOR task-file inconsistencies corrected: Planned actions said ~90/~17 while
  the Goal said ~89/~20 (neither matched the re-verified 89/19); "all five are
  manifest files" said five where seven files are in scope; the deliverables
  list named only five of the seven files.

## Other important findings/information

- Forward edges (task 5's spec_hash16 precedent): main.rs's cmd_info/
  run_info_format reference the info cluster (src/info.rs, src/info_format.rs -
  task 10) and cmd_diagram/tool_override_mut the diagram cluster (src/diagram.rs
  - task 11); gen_spec_cmd.rs references src/specgen.rs and mc_ref_cmd.rs
  references src/ref_mc.rs (both task 12). All 24 forward-edge identifiers are
  listed in Depends anyway; they resolve when those chunks land.
- The trait-impl method identifier form `<file>::<Type>(<Trait>)::<method>`
  (established by task 5 for McMode(FromStr)::from_str & co.) is used here for
  BackendOptions(Default)::default in run_test's Depends - inherent-method rows
  stay `<file>::<Type>::<method>`.
- test_cmd.rs's Guard is a local struct defined inside run_test (RAII temp-dir
  cleanup) - it gets a type row like any other type; its drop method is the
  only nested fn in this chunk beyond impl methods.
