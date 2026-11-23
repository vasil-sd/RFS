# Phase 18, Task 3D - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3D.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None.

## Changed/removed source files

| Name | Change |
|---|---|
| `src/mc/smt_plan.rs` | dropped all six D8 rejections: (1) enum-typed vars encode as `bit<ceil_log2(N)>` holding the variant's DECLARATION-ORDER INDEX (`SmtVar.is_enum` + `enum_variants` carried for witness display; literal inits resolve a variant path - qualified `Color::Red` or, for a State-typed var, the bare state-variant name); (2) enum-typed concrete payload fields encode the same way (`SmtClass.fields` gains the variant list), abstracted enum fields fold to their representative's INDEX; (3) `SmtTrans.class: usize` -> `classes: Vec<usize>` - a bare group pattern collects ALL of the group's variant classes (multi-variant wildcard), a named-variant pattern yields one; ONE guard conversion serves every class because a bare-group transition carries no payload bindings (validator rule); (4) `convert_expr`'s Path arm resolves an enum path to an index constant of the enum's width (also fixed the field-reference arm's domain match, which hit `unreachable!` on `ValueDomain::Enum` - a fourth rejection site the task file's line hints did not name); (5)+(6) enum-variant patterns in `output_has`/`input_is` become index constraints in the existing `(pos, value, is_bool)` triple (abstracted input fields fold against their representative's index like Int/Bool). Module-header v1-scope comment updated |
| `src/mc/ref_mc_smt.rs` | in-process engine: `enabled_t(i)` built as the OR over a transition's classes of (state-match AND `ev_i == c` AND guard) - a one-class transition renders exactly as before; input-atom arms contribute every covering transition (`tr.classes.contains(ci)`); **SOUNDNESS FIX found while writing the pins**: the fire-based `input_atom_expr` gained a per-arm `ev_i == ci` conjunct (multi-class only) - without it a wildcard made EVERY covered arm true at once, so `input_is(Msg(Good))` held while the consumed input was `Msg(Bad)` and exact-mode violations were MASKED (reproduced: exact FAIL step 1 vs smt PASS-within-bound before the fix); with the conjunct the fire-based form coincides with the ev-based one at every live sample, multi-class included; witness trace prints enum vars/fields by VARIANT NAME (`color=Green`, `Go_G(c=Blue)`), not the raw index |
| `src/mc/smt_emit.rs` | both renderers: per-transition class tables (`TRANS_CLASSES`/`TRANS_CLS_OFF`/`TRANS_CLS_LEN`) replacing the old `(state, next_state, class)` triple; enum witness-display tables (`VAR_ENUM_NAMES/OFF/LEN`, `FIELD_ENUM_NAMES/OFF/LEN` - C gets a dummy name slot so gcc -Warray-bounds stays quiet when no var is an enum); input-atom covering-transition lists precomputed at emission (a wildcard covers every arm class it spans); the fire-based `build_input_atom` gains the step's event variable + per-arm `ev_i == ci` conjunct ONLY for multi-class specs, so single-class codegen stays byte-identical to before |
| `src/codegen/backends/rust/mc.rs` | emitted Rust runtime: `enabled_t` ORs over `TRANS_CLASSES[off..off+len]`; enum var/field witness display by variant name; the input-atom call site passes `ev_vars.as_deref()` on multi-class specs |
| `src/codegen/backends/c/mc.rs` | emitted C runtime: same three changes (OR over the class table, named witness display, `ev_vars` threaded into `build_input_atom`) - C output stays gcc -Wall clean |
| `src/lang/parser.rs` | `pattern_field` accepts an UNNAMED enum-variant path (`P(Color::Red)`) - a qualified path in the pattern position constrains the field to that variant without binding it, exactly like the unnamed int/bool literals. Required because formula patterns may not bind (Phase 1 validator rule rejects named fields), so the task file's `input_is(Color(Red))` / `output_has(Color(Red))` surface was unparseable before; exact-mode matching (`enum_match`) and the plan layer already handled `PatternExpr::EnumVariant`, only the parser arm was missing |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/SEMANTICS.md` §9.9 | changed | the v1-scope rejection paragraph is replaced by a normative **Enum support and multi-variant wildcards (task 3D)** paragraph: declaration-order index encoding in `bit<ceil_log2(N)>` faithful to exact mode, named witness display, the wildcard's OR-over-classes `enabled_t`, the no-payload-bindings rule, the per-arm `ev_i == ci` conjunct and why it is soundness-critical, and the profiling result (16-variant group at bound 5 in milliseconds -> no `--full-properties` gating) |
| `docs/user/MODEL_CHECKING.md` | changed | *What it checks* gains the enum-support sentence (vars/fields, enum paths, enum-variant patterns, bare-group wildcards all accepted; index encoding; named witness display). No D8 limitation note existed to retire - the rejections were only ever in SEMANTICS.md |
| `KB.md` section 7 | unchanged | no enum/wildcard/pattern limitation row was ever recorded there (its only "D8" is the unrelated C-backend-widths decision) - nothing to remove or annotate |
| `docs/llm/SRC_STRUCTURE.md` | changed | new rows for `rust_str_lit`/`c_str_lit`; `build_smt_spec`, `convert_expr`, `resolve_input_is` descriptions updated (enums encoded, not rejected); `pattern_field` description + Depends gain the unnamed-enum-variant arm and `ident_path`; llm-budget 1831 -> 1833 |
| `docs/llm/TEST_STRUCTURE.md` | changed | new rows for `parity_task3d`, `parity_task3d_exit_only`, the four `task3d_*` parity tests, and `smt_bounded_task3d_enum_and_wildcards`; llm-budget 1448 -> 1455 |

## Other file changes

None.

## Changed functionality

- `rfs mc-ref --mode smt-bounded` and the generated SMT binaries ACCEPT enum-typed variables, enum-typed concrete payload fields, enum paths in expressions, enum-variant patterns in `input_is`/`output_has`, and bare group patterns over multi-variant groups - all encoded as declaration-order indices faithful to exact mode. Witness traces print the variant name, not the index.

## Removed functionality

- The six shared D8 rejections are gone from `build_smt_spec`: `"smt-bounded mode does not support enum-typed variables (var '…')"`, `"…enum-typed payload fields (field '…')"`, `"…wildcard event patterns over multi-variant groups (transition … on '…')"`, `"…enum paths in expressions (…)"`, `"…enum-variant output patterns (v1)"`, `"…enum-variant input patterns (v1)"` - plus the latent `unreachable!("concrete field domain")` panic when a guard referenced an enum-typed payload field.

## New functionality

- **D8 (all six encodings):** enum values are their variant's declaration-order index in `bit<ceil_log2(N)>` - the same encoding exact mode uses at configuration build time, so every boolean fact exact mode derives about enum values holds identically under smt-bounded. Multi-variant wildcards fire on ANY of the group's variants (`enabled_t` = OR over classes); large groups are enabled by default (16 variants at bound 5 solves in milliseconds - no `--full-properties` gating triggered).
- **input_is wildcard soundness:** the fire-based input atom now tests that the ACTUAL input was the arm's class (`ev_i == ci`). Before the fix a multi-variant wildcard made every covered arm true simultaneously, masking exact-mode violations (e.g. `always (!input_is(Msg(Bad)))` false-PASSed while exact mode FAILs on a Bad step).
- **Unnamed enum-variant pattern fields:** `P(Color::Red)` parses in any pattern position (formula and transition), constraining without binding - the syntax the task file specifies for `input_is`/`output_has`.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_smt_test.rs::parity_task3d` | added | parity helper: ref vs both generated backends at the listed bounds, byte-identical stdout + exit code (the 3D specs carry no assumptions and no FULL-class properties) |
| `tests/ref_mc_smt_test.rs::parity_task3d_exit_only` | added | exit-code-only parity at bounds where the witness trace carries solver-version-free vars (multi-class FAIL specs past path death - the verdict is deterministic, the trace bytes are not); byte-parity is pinned at the forced-trace bounds by the caller |
| `tests/ref_mc_smt_test.rs::task3d_enum_var_pass_and_fail` | added | (a) enum-typed var: PASS variant (Blue unreachable) + FAIL variant (deterministic step-1 violation; the CE prints `color=Green`, and a raw-index leak assertion guards the display), parity-pinned at every bound (single class - fully forced trace) |
| `tests/ref_mc_smt_test.rs::task3d_enum_field_pass_and_fail` | added | (b) enum-typed concrete payload field: PASS variant (only Red admitted) + FAIL variant (Go+Blue is the only transition, so step 0 is forced; the CE prints `Go_G(c=Blue)` and `color=Blue`), byte-parity at bounds [1,2] + exit parity at 5 |
| `tests/ref_mc_smt_test.rs::task3d_wildcard_pass_and_fail` | added | (c) multi-variant wildcard: PASS variant pins the per-arm `ev_i == ci` conjunct (`!(input_is(Ping) && input_is(Pong))` - exactly one variant per live sample; this spec FALSE-FAILED before the soundness fix) + FAIL variant (Bad forced at step 0 via a two-state design; the CE shows `event=Msg_Bad()`), parity-pinned |
| `tests/ref_mc_smt_test.rs::task3d_enum_patterns_pass_and_fail` | added | (d) enum-variant patterns: `output_has` FAIL (emitted Green mismatches pattern Red - deterministic step 1, single class) + PASS twin (pattern matches the emitted variant), `input_is` FAIL (Go+Green forced; the pattern matches at sample 1) + PASS twin (only Red admitted), parity-pinned |
| `tests/smt_bounded_test.rs::smt_bounded_task3d_enum_and_wildcards` | added | all ten specs run in BOTH generated backends: rust/c stdout byte-parity, pinned exit codes and the variant names in the witness traces; C output gcc -Wall clean |
| every pre-existing smt pin (task 1A/3A/3C families, ref-vs-generated parity) | unchanged | all stay byte-identical - single-class specs generate exactly the same code as before (the ev parameter + conjunct are emitted only for multi-class specs), and the multi-class `enabled_t` OR over a one-element class list reduces to the old single equality |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --features smt --test ref_mc_smt_test --test smt_bounded_test` | OK - 19/19 + 4/4 (baseline before 3D was 15/15 + 3/3; the delta is the new task-3D tests) |
| `cargo test` (full non-smt suite) | OK - every target green (267 lib unit tests + all integration suites, incl. the 42-leg example_projects build matrix) |
| `cargo build --features smt` | OK - zero warnings |
| `cargo clippy --features smt --all-targets` | OK at the pre-task baseline: 103 warning lines both on the stashed pristine tree and with this task's changes (one needless-borrow instance introduced by the new conjunct was caught and removed) |
| `cargo fmt --check` | OK - clean |
| `rfs-dev inventory audit` | EXACT - SRC 1032 fns / 217 types, TEST 1107 fns / 35 types, zero per-file diffs |
| `cargo test --test dev_docs_test --test docs_md_test` | OK - 9/9 + 5/5 (llm-budgets raised 1831 -> 1833 and 1448 -> 1455 for the new rows) |
| z3 under test | in-process z3 4.8.12.0 (z3-sys) + generated binaries against the unified resolver prefix `/workspace/host/z3` - both named in every assertion message via `version_note()`; cross-solver-version parity holds on every forced-trace bound |

## Notes for follow-up tasks

- The witness trace prints BARE variant names (`color=Green`), matching the existing `state=A` convention for the State enum in the same line; exact mode's JSON CE uses qualified names (`Color::Red`). If task 3E's residual-differences section covers counterexample form (D6), this display difference belongs there.
- Multi-class FAIL specs have solver-version-free vars past path death (the unconstrained ev/fld vars take different values under z3 4.8.12 vs 5.3.1) - the verdict stays deterministic, the trace bytes do not. The parity convention adopted: byte-parity at forced-trace bounds (<= 2 for those specs), exit-code parity beyond. Task 3E may want to document this as part of D6.
- `PASS WITHIN BOUND` on an enum spec whose exact-mode twin deadlocks (e.g. the (a)-PASS variant: after color=Green no transition fires) is expected - smt-bounded v1 does not model unexpected terminations; another 3E residual difference.
- The unnamed-enum-variant parser arm also accepts `on Go(G(Color::Red))` in TRANSITION patterns (consistent with the existing unnamed int/bool literal support there); no validator change was needed - formula patterns still bind nothing.
