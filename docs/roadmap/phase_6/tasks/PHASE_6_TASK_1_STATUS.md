# Phase 6, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | documentation-only task |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| (none) | - | no `src/` changes; the one implementation divergence found in the cross-check pass (C expression lowering UB) is deliberately NOT fixed here - it is tracked as plan correction / new task 2a per the task file's action-4 rule |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/SEMANTICS.md` | new | the normative formal semantics document: 10 sections (notation/scope; automaton model; transition relation incl. priorities, simultaneous updates, on_error, deterministic vs nondeterministic modes, Tick; property sampling; PSL operator semantics incl. R1-R4 and the obligation pending model; liveness strong/weak + fairness; assumptions + handler policies; error/deadlock semantics; model checking incl. configuration encoding, the precise representative-input rule, exact exploration, fuzz strategy, status taxonomy + labeling rules, SMT note; cross-references). Every claim verified against the implementation (see `PHASE_6_TASK_1.md` "Cross-check pass results"). |
| `DESIGN.md` | changed | one pointer line each in sections 2-4 naming `docs/SEMANTICS.md` as the normative semantics source (no content moved) |
| `KB.md` | changed | section 1 state table: new "Formal semantics" row (done Phase 6 task 1, maintained) pointing at `docs/SEMANTICS.md` |
| `START.md` | changed | documentation map: new row for `docs/SEMANTICS.md` (content, when-to-read, how-to-read) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `docs/roadmap/PHASE_6_PLAN.md` | changed | "Plan corrections" table: row 2a (new task - C expression lowering fix, found in this task's cross-check) + row 6.1 (change - final examples_test count re-pin becomes 50/13 because task 2a adds example 50 first). The "Planned tasks" table is untouched. |
| `docs/roadmap/PHASE_6_TASK_2A.md` | new | task file for the correction-added task (C guarded shifts + uint64-wide multiplication, example 50_wide_ops.rfs, targeted cross-backend parity test, one re-pin sweep) |
| `docs/roadmap/PHASE_6_TASK_6.md` | changed | count mentions 49/13 -> 50/13 (4 places) per plan correction 6.1; no other content touched |
| `docs/roadmap/PHASE_6_TASK_1.md` | changed | "Other information": cross-check pass results appended (sources checked table + 6 findings with resolutions); planned sections untouched |

## Changed functionality

- None - documentation-only task; no toolchain behavior changed.

## Removed functionality

- None.

## New functionality

- Normative semantics reference `docs/SEMANTICS.md` covering ALL ten PHASE_6.md topics (automaton model; transition relation; deterministic vs nondeterministic modes; property sampling; PSL operator semantics; strong/weak liveness; fairness; assumptions; error/deadlock semantics; abstraction rules) - the document tasks 3.2 (ref MC) and 4.4/4.5 (independent evaluators) implement from, so doc/implementation mismatches will surface as test failures.
- Cross-reference pointers making SEMANTICS.md discoverable: `DESIGN.md` sections 2-4, `KB.md` section 1, `START.md` documentation map.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | no test changes; sanity run below confirms the baseline is intact |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (376) | OK - 376 passed / 0 failed, matches the Phase 5 close-out baseline exactly (docs-only change) |

## Found problems/issues/mismatches

1. **C expression lowering undefined behavior (latent cross-backend divergence)** - generated C renders bitvector `*`/`<<`/`>>` via integer promotion without widening: variable shift amount >= promoted LHS width is UB in C (Rust defines `amount >= width -> 0`; on x86-64 gcc the count wraps mod 32, so e.g. `w=5, k=32` gives C=5 vs Rust=0), and bit<16> products > INT_MAX are signed overflow. Verified with a scratch spec; latent (no example uses `*`; all variable shift amounts are `bit<2>`). Documented in `docs/SEMANTICS.md` section 2.2 footnote c-ub; fix added as plan correction / new task 2a (`PHASE_6_TASK_2A.md`) - NOT fixed in this task per its action-4 rule.
2. **Variable initializer evaluation order (ambiguity)** - `init` expressions are typed with ALL module variables in scope but rendered as a constant struct literal; cross-variable initializers are ill-formed in effect. Documented + flagged (footnote init-order, section 2.4).
3. **Out-of-subset assumptions are not enforced (gap)** - assumptions outside the monitor subset generate no monitor and are skipped with a warning in both production code and MC (section 7.6 + footnote assume-skip).
4. **Top-level `p -> bounded_eventually[N] q` is one-shot** - anchor range `{0}` only; made explicit in section 5.5 to prevent a misreading of "enclosing range".
5. **Deadlock precision** - a state merely missing a transition for some input does NOT deadlock (unexpected termination instead); corrected the initial draft's confusing Tick sentence (section 8.4).
6. **`output_has` is any-of** - confirmed against generated signal rendering (`s.outputs.iter().any(...)`); documented in section 5.1.

## Other important findings/information

- The cross-check pass found NO doc-vs-implementation mismatches requiring document corrections beyond the precision items above (4-6) and the flagged C divergence (1): the Phase 2-5 design records (DESIGN sections 20-23) agree with the shipped code on every checked claim, including the corrected weak-until equation (D-P4-10), the D-P3-3 deadline confirmation at i+N-1, and the exact-engine counter semantics (`transitions` counts success+pruned+violation, NOT unexpected).
- For task 2a: the re-pin scope of the C lowering fix is small - only C artifacts containing `<<`/`>>`/`*` change bytes (examples 20_shifts, 44_token_ring, 45_elevator + their aggregate-crate modules); no other example contains those operators.
- For task 3.2 (`src/ref_mc.rs`): sections 8.4 (deadlock), 9.3 (representative-input rule), 9.4 (canonical order), 9.5 (exploration + counters) and 9.7 (status/labeling) are written to be implementable from the document alone; the monitor stepping algorithm is verbatim in section 5.5.