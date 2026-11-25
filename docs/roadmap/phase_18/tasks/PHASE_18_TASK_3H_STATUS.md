# Phase 18, Task 3H - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3H.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None.

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/mc/smt_plan.rs` | changed | `build_smt_spec` var loop: the `other => Err("smt-bounded mode requires literal initial values...")` arm replaced by a constant-expression fold through the shared evaluator (`crate::mc::ref_mc::eval_expr`) in the EXACT-MODE init context - `state` = the initial state as `Val::Enum`, empty `vars`/`binds`/`bind_types` (mirroring `ref_mc_exact.rs`, which rebuilds an empty vars map per var, so a later var's init can never reference an earlier one); the resulting `Val` converts to the init constant (`Bit(n) => n`, `Bool(b) => u64::from(b)`, `Enum(name) =>` the var's own enum variant index - defensive arm, a Path init is caught by the fast arm above); on `Err` the message is byte-identical to exact mode's ("variable '{}' initializer is not a constant expression (unsupported)"); the literal fast paths (`Expr::Int`/`Expr::Bool`/enum `Expr::Path`) are kept as-is and the existing "does not fit in N bits" width check stays, now also guarding folded constants; leading comment updated |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/SRC_STRUCTURE.md` | changed | the `build_smt_spec` row's description: "literal inits only" retired from the remaining-limitations list, replaced by the task-3H constant-expression fold note (state = initial state, no vars bound, width check guards the folded constant); the sole remaining limitation is now concrete fields <= 64 bits - same-commit inventory sync per PROTOCOL.md section 5 |

## Other file changes

None.

## Changed functionality

- `rfs mc-ref --mode smt-bounded` and the generated SMT binaries (both backends) ACCEPT variable initializers that are constant expressions - literals, state-variant paths, and arithmetic/comparison/boolean combinations over them (`init 5 + 1`, `init 2 * 3`, `init state == A`) - folding each to its constant in the exact-mode init context before the width check. Previously every non-literal init was rejected with "smt-bounded mode requires literal initial values".

## Removed functionality

- The S1 rejection: `"smt-bounded mode requires literal initial values (var '{}' has ...)"` - gone from the planner (audit row S1; E4's exact-mode counterpart is unchanged - it was never a restriction, just the semantics now mirrored).

## New functionality

- None beyond the lift: no engine, emitter or wire-format change - an init constant is a single `u64` asserted as `var_0 == <init>` at step 0 by every engine, so folding `5 + 1` to `6` at plan time changes no formula structure. Var inits referencing another var stay rejected in BOTH modes with the identical message (exact mode's empty-vars-per-init semantics is mirrored, not relaxed).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | deliberately no new tests or pins in this task - exact-vs-smt init-acceptance parity coverage is task 3I's, per the plan. Verified beforehand that no existing test pins the old "requires literal initial values" string (`grep -rn "requires literal initial values" tests/` empty) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, plain build, RUSTC_WRAPPER exported) | OK - 612 passed / 0 failed: EXACTLY the task-3F/3G baseline; all pre-existing pins byte-identical (no committed spec uses a non-literal init) |
| `cargo test --features smt` | OK - 639 passed / 0 failed: identical to the task-3G run; covers the planner change incl. every 1A/3A-3D parity pin and the generated-binary suites |
| `cargo clippy --all-targets` (default + smt) | OK - no errors; 71 warning lines = the exact documented baseline, zero new (an initial "unreachable pattern" warning from a redundant `Ok(_)` arm was fixed in-work: `Val` has exactly the three handled variants, so the final arm is `Err(_)` only) |
| `cargo fmt --check` | OK - clean |
| `cargo test --test dev_docs_test` (9) | OK - after the same-commit inventory description sync |
| `rfs-dev inventory audit` | EXACT - SRC 1031 fns / 217 types, TEST 1107 fns / 35 types, zero per-file diffs (unchanged counts vs task 3G - no functions added or removed) |
| Scratch-spec parity (in /tmp, not committed) | `init 5 + 1` / `init 2 * 3` / `init state == A` now ACCEPTED by smt-bounded with the same verdict as exact mode; observation specs prove the folded constants encode identically (`state == A -> n == 6` after `init 2 * 3` FAILs at exactly step 2 under BOTH modes - a wrong fold would fail at step 0; `state == A -> f` after `init state == A` PASSes under both); `init a + 1` (var reference) REJECTED by both with the byte-identical "not a constant expression" message; enum-var init from a state-variant path unchanged (fast arm) |

## Found problems/issues/mismatches

- The task file's exit-criterion-4 parenthetical is empirically wrong: `var x: bit<1> init 1 + 1 ;` is NOT rejected by exact mode - `eval_expr` folds arithmetic at the EXPRESSION's own type (bare int literals default to Bit(8)), so `1 + 1` = 2, and exact mode carries `Val::Bit(2)` unmasked in a bit<1> var (verified: exit 0, pass-complete). SMT's width check rejects it ("does not fit in 1 bits") - which the task file itself anticipated ("a folded `5 + 1` into a `bit<1>` must still fail there"). The residual difference is that exact mode ACCEPTS out-of-range initial values (bare literals like `bit<1> init 3` or `bit<8> init 300` were already accepted by exact and rejected by SMT BEFORE this task) while the bit-vector encoding requires an init numeral of exactly `width` bits. Keeping the width check is the honest choice - masking instead would silently change the initial value (diverging from exact mode's unmasked carry), and removing it would emit invalid z3 numerals. Recorded as a candidate note for task 3J's residual-differences section (SMT-strict direction, pre-existing for literals, now also covering folds).
- The `Ok(Val::Enum)` arm in the fold is defensive dead code for validated specs (every Path init is caught by the fast arm above; no other expression yields `Val::Enum`) - kept because it makes the match total over `Val` and mirrors the fast arm's index encoding, so a future parser change cannot silently mis-encode an enum init.

## Other important findings/information

- With 3H done, all three behavior tasks of the validator-restriction arc (3F V17+S3, 3G S4, 3H E4/S1) are complete; the next executable task is 3I (tests for the 3F-3H changes + test-suite adoption), which pins the new acceptance in both directions and flips any remaining old-rejection assertions.
