# Phase 18, Task 3A - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3A.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (all changes are to existing engine/emitter/runtime files).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/mc/ref_mc_smt.rs` | changed | D1: `SfCtx` gains an `alive: &[Bool]` field (populated in `run_unrolled`); the `sf_to_z3` `Sf::Next` arm now renders `b_or(&ctx.alive[step+1].not(), &inner)` - a next-obligation is weak at trace death. D2: the top-level `BoundedEventually(n)` violation is now `alive[hi-1] AND NOT(OR_j f(j))` (the per-disjunct `alive[j]` gate is dropped, the deadline-alive conjunction added; `hi == 0` keeps the pre-existing `NOT sat`) |
| `src/mc/smt_emit.rs` | changed | new `sf_uses_next` walker; both renderers' `Sf::Next` arm emits the liveness gate (rust `b_or(&alive[ns].not(), &b)`, c `Z3_mk_or(c, 2, { Z3_mk_not(c, alive[ns]), b })`); every emitted property/expansion fn gains an `alive` parameter (rust `&[Bool]`, c `const Z3_ast *`) named without the underscore/void-cast only when the body has a next node; `prop_fns`/`expand_fns`/`PROP_FNS`/`EXPAND_FNS` table types + both `expand_dummy`s updated to carry it |
| `src/codegen/backends/rust/mc.rs` | changed | emitted Rust runtime: all three `props[p](...)` call sites and both `expands[p](...)` call sites pass `&alive`; the top-level BE check (kind 2) asserts `alive[PROP_BE_BOUND[p]-1] AND NOT phi` (n == 0 keeps `NOT phi`) |
| `src/codegen/backends/c/mc.rs` | changed | emitted C runtime: all three `PROP_FNS[p](...)` call sites and both `EXPAND_FNS[p](...)` call sites pass `alive`; the BE check (`PROP_KINDS[p] == 2`) asserts the same deadline-alive conjunction via a `Z3_mk_and` of `alive[n-1]` and `NOT phi` |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/SEMANTICS.md` | changed | §9.9 normative closure: a `next` obligation is weak at trace death (matching §8.3 - the body is never checked against a frozen artifact); a top-level one-shot `bounded_eventually[N]` is unconfirmable if the path dies before its deadline sample N-1 (matching §5.5) |
| `docs/user/MODEL_CHECKING.md` | changed | the smt-bounded "What it checks" note gains the trace-death weak rule for `next` and the path-death clause for top-level `bounded_eventually[N]` |
| `docs/llm/SRC_STRUCTURE.md` | changed | +1 inventory row (`smt_emit.rs::sf_uses_next`) |
| `docs/llm/TEST_STRUCTURE.md` | changed | +1 inventory row (`task3a_trace_death_negative_and_positive_controls`); llm-budget raised 1440 -> 1441 |

## Other file changes

None.

## Changed functionality

- smt-bounded mode (in-process `ref_mc_smt` AND both generated SMT binaries,
  parity-pinned) no longer false-fails a `next`/shifted property when an unhandled
  input freezes the successor configuration (D1 closed): the obligation is now weak
  at trace death, exactly matching exact mode's terminated-path semantics.
- A top-level `bounded_eventually[N]` is now unconfirmable when the path dies before
  its deadline (D2 closed). The in-process and generated encodings are now IDENTICAL -
  this also closes a latent parity gap where the in-process engine gated each BE
  disjunct by `alive[j]` but the generated runtimes gated none at all.

## Removed functionality

None.

## New functionality

- None user-facing beyond the corrected soundness; the liveness gate is internal to
  the three SMT engines.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_smt_test.rs` (`task3a_trace_death_negative_and_positive_controls`) | added | four new spec constants + one test: D1 death variant (next body at a frozen successor - PASSES, the B6 false-FAIL reproduction) vs its alive twin (genuine next violation - FAILS for bound >= 2); D2 death variant (bounded_eventually[3] past path death - PASSES) vs its alive twin (genuine BE violation - FAILS for bound >= 3). Each pinned on ref AND both generated backends with byte-parity via `parity_task1a` |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --features smt --test ref_mc_smt_test` (11) | OK - 10 pre-existing + the new task3a control |
| `cargo test --features smt --test smt_bounded_test` (2) | OK |
| `cargo test --features smt` (all binaries) | OK - 382 pass / 0 fail |
| `cargo test` (default, all binaries) | OK - 378 pass / 0 fail |
| `rfs-dev inventory audit` | EXACT - SRC 1024 fns / 215 types, TEST 1093 fns / 35 types, zero per-file diffs |
| `cargo clippy --all-targets` (default + smt) | OK - no errors; NO new warnings vs HEAD (the old BE arm's "loop variable j indexes alive" warning is gone - net -1 lib warning); zero in this task's changed lines |
| `cargo fmt --check` | OK |
| `cargo test --test dev_docs_test` (9) + `--test docs_md_test` (5) | OK - after the TEST_STRUCTURE budget raise |

## Found problems/issues/mismatches
- Latent parity bug confirmed and closed: before this task the in-process top-level
  `bounded_eventually` gated each disjunct by `alive[j]` while the generated runtimes
  (via the emitter's synthetic `Sf::BoundedEventually` node) gated none at all - the two
  already disagreed on that property. The unified `alive[hi-1] AND NOT(OR_j f(j))`
  encoding makes all three identical.
- Sandbox environment: `/dev/null` is a regular file here and was again transiently
  overwritten with git-status text ("On branch master / Your branch is ahead of
  origin/master by 7 commits"), which the rustc target-probe (`rustc - ...`) read as
  source -> `E0762 unterminated character literal` on every cargo build/test. The
  pre-existing `/tmp/rustc-wrap.sh` RUSTC_WRAPPER redirects rustc's stdin from `/dev/null`;
  truncating `/dev/null` to empty + setting `RUSTC_WRAPPER=/tmp/rustc-wrap.sh` + deleting
  the corrupted `target/.rustc_info.json` restored the toolchain. Same issue class as
  recorded in tasks 2 and 3's statuses.

## Other important findings/information
- The D1/D2 fixes are CORRECTNESS, not performance: they add a single boolean OR/AND per
  temporal node and change no formula size on alive paths, so no existing PASS pin moved
  and no `--full-properties` gating is needed (that remains a 3D concern for genuine
  formula-blowup encodings).
- The controlled-experiment design (death variant PASSES while its one-line-different
  alive twin FAILS) proves the liveness gate is active and masks nothing: the only
  difference between each pair is whether the successor step fires a transition.
- `bounded_eventually[0]` (legal per the validator - "body must hold at the trigger
  sample") is a degenerate top-level case (empty window); both engines keep the
  pre-existing `NOT sat` assertion for `hi == 0` to avoid an `alive[-1]` index and to
  preserve prior behavior. Untouched by this task's scope.
