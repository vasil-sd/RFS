# Phase 16, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope. Shared context (baselines, structure map) lives in
> `PHASE_16_PLAN.md` ("Context"); the layout rationale in `PHASE_16.md`.

## Goal
Give the ref exact engine a `--nondet` successor strategy (every enabled transition
becomes a successor, mirroring the generated engines) with the matching nondet
liveness fairness semantics, expose it as the `--nondet` CLI flag on `rfs mc-ref`,
and pin it with a cross-backend differential variant. This is the first independent
ground truth for nondet semantics - today only Rust-to-C parity exists over one
shared plan.

## Context
- **Generated-engine nondet BFS (pinned from `src/codegen/backends/rust/mc.rs`
  ~1218-1246; C mirror at c/mc.rs ~1027-1050):** `expand_config` computes ALL
  matching effects per input (`mc_step_all`); empty -> `LevelOutcome::Unexpected`
  (the path terminates, counted as unexpected); the selection is
  `if nondet { all effects } else { [effects[0]] }`; each selected effect yields its
  own successor carrying its `transition_id`. The runtime flag `--nondet` sets
  `nondet = true` (default `DEFAULT_NONDET` = false).
- **Generated-engine nondet liveness filter (pinned - Rust ~791-802, C ~1550-1578,
  D-P4-11 strong fairness):** when building the stay-edge subgraph per limit vector
  q, a cycle-start candidate c is skipped unless it passes
  `!(nondet && !(adj[c].len() == 1 && complete[c]))` - i.e. in nondet mode a bad
  cycle must be STRONGLY FAIR: every enabled successor of a cycle state is taken on
  the cycle, which in the explored graph means each cycle state has exactly one
  explored successor AND was fully expanded when truncated. **This pins the plan's
  open question:** ref_mc's `liv_analyze` gets a `nondet: bool` parameter and
  applies exactly this guard when collecting stay successors.
- **ref_mc today:** `match_effect` (ref_mc.rs ~1322) returns only the FIRST matching
  effect (or the on_error fallback, or None = unexpected input). The BFS loop
  (~2240-2350 in ref_mc_exact.rs after task 1) processes one effect per input;
  `any_successor`/deadlock logic and the `adj`/`parent` edge records already carry a
  transition id per edge (`(iidx, tid, target)`), so trace/lasso reconstruction under
  nondet works without changes to `trace_steps`.
- **What to add:** an all-matches enumeration (every matching transition in firing
  order - effective priority descending, declaration order for ties - plus the
  on_error fallback when nothing matched; empty = unexpected input) and a BFS
  strategy switch: deterministic fires effect[0] only, nondet fires every effect as
  its own successor. Deadlock in nondet = no effect fires for ANY input (the
  existing `any_successor` check generalizes directly).
- **Config/CLI:** `RefConfig` gains `nondet: bool` (default false); the engine's
  `run()` passes it to the BFS and to `liv_analyze`. `src/cli/commands/mc_ref.rs`:
  new `--nondet` flag, `McRefOptions.nondet`, usage text updated - the current
  "Not applicable" note for `--nondet` (module doc ~line 17-20 + the usage string)
  is REMOVED and replaced with the real description. BUILD.md's mc-ref row (~124)
  lists `--nondet` under N/A - update it.
- **Cross-test convention (from tests/ref_mc_diff_test.rs):** eligible examples =
  examples/valid minus the pinned EXCLUDED list (>=40); random specs via specgen
  with pinned seeds; generated binaries cached per test process (BIN_CACHE pattern);
  comparison goes through JSON reports (`--counterexample-format json` on the
  generated side, `to_json_report` on the ref side). The nondet variant runs ref-mc
  with `nondet: true` and BOTH generated binaries with `--nondet`, asserting
  status/reason/label/step agreement and EXACT state counts; transition counts are
  NOT asserted cross-engine (documented convention - the engines count differently
  by design).
- **Verification bar:** canonical sweep unchanged except the new diff-test variant
  adds tests (record the new totals); fmt clean; clippy at baseline; inventory
  EXACT; sync-check clean.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Split ref_mc.rs | the BFS + liveness live in ref_mc_exact.rs after task 1; this task extends them there |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/ref_mc_exact.rs` | change | all-matches effect enumeration + nondet BFS strategy switch + nondet liveness fairness guard |
| `src/mc/ref_mc.rs` | change | RefConfig.nondet field; run() passes it through to the engine |
| `src/cli/commands/mc_ref.rs` | change | --nondet flag, McRefOptions.nondet, usage text (remove the N/A note) |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/llm/SRC_STRUCTURE.md` | change | new fn rows (all-matches enumeration), RefConfig field row |
| `docs/dev/mc_engine.md` | change | dev:mc-engine-exact / dev:mc-engine-liveness: nondet strategy + fairness filter documented |
| `BUILD.md` | change | mc-ref row: --nondet moves from the N/A list to the flag list |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_diff_test.rs` | change | nondet comparison variant (ref-mc vs both generated engines with --nondet) |

## Planned actions

### 1. Pin the generated nondet semantics and implement the all-matches enumeration
Re-read the pinned locations (Context bullets 1-2, BOTH backends) to confirm the
selection rule and the guard text, then add to `ref_mc_exact.rs` an all-matches
effect enumeration built on the common core's `pattern_bindings`/
`input_matches_variant`/`eval_expr` - same firing order as `match_effect`
(priority desc, declaration ties), on_error fallback when nothing matched, empty vec
= unexpected input. Rationale: one enumeration function serves both strategies -
deterministic mode selects index 0, nondet iterates all - so the two modes cannot
drift in matching semantics.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_exact.rs` | change | new pub(crate) all-matches enumeration fn + unit tests (a multi-effect transition fires both successors; on_error fallback; empty = unexpected) |

### 2. Add the nondet BFS strategy switch
Thread `nondet: bool` through the engine's `run()` BFS: per input, enumerate
effects; deterministic keeps the current single-effect path byte-for-byte; nondet
enqueues one successor per effect (each with its own tid in parent/adj). The
existing deadlock check (`any_successor`) generalizes: no effect for any input =
deadlock. Rationale: mirrors expand_config's selection exactly (Context bullet 1);
the edge records already carry tids, so lasso/trace reconstruction is unchanged.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_exact.rs` | change (BFS loop) | strategy switch inside the per-input loop |
| `src/mc/ref_mc.rs` | change (~1884-1906) | RefConfig.nondet field + Default (false); run() passes it to the engine |

### 3. Add the nondet liveness fairness guard
Extend `liv_analyze` with a `nondet: bool` parameter; when collecting stay
successors per limit vector, skip candidate cycle starts that fail the pinned guard
(`adj[c].len() == 1 && complete[c]` required in nondet mode). Rationale: D-P4-11
strong fairness - without the guard, nondet liveness would accept cycles that skip
enabled successors, diverging from both generated engines.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_exact.rs` | change (liv_analyze) | nondet parameter + stay-successor guard; unit test with a nondet liveness spec where the filter changes the outcome |

### 4. Expose --nondet on the CLI
Add the flag to `run_cli`, the field to `McRefOptions`, pass it into `RefConfig`;
rewrite the usage text and module doc (remove the "Not applicable: --nondet" note,
document the flag). Update BUILD.md's mc-ref row accordingly. Rationale: D-P16-4 -
near-exact CLI surface with the generated exact-mode binaries.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/commands/mc_ref.rs` | change | flag parsing + options + usage text + module doc |
| `BUILD.md` | change (~124) | --nondet documented as an applicable mc-ref flag |

### 5. Add the cross-backend nondet diff variant and run the full sweep
In `tests/ref_mc_diff_test.rs`, add a nondet variant following the existing
conventions (Context bullet 6): ref-mc `nondet: true` vs both generated binaries
with `--nondet --counterexample-format json` over eligible examples + random specs;
assert status/reason/label/step + exact state counts, NOT transition counts.
Include at least one eligible example with a liveness property so the fairness guard
is exercised cross-engine. Then run the full canonical sweep and record the new
test totals. Rationale: independent ground truth for nondet - both generated engines
derive from the same plan.rs, so agreement with ref-mc's own enumeration is the new
signal.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_diff_test.rs` | change | nondet comparison variant + helpers |
| `docs/llm/SRC_STRUCTURE.md` | change | new rows (enumeration fn, RefConfig field) - same commit |
| `docs/dev/mc_engine.md` | change | nondet strategy + fairness filter sections - same commit |

## Task deliverables
- `--nondet` successor strategy + liveness fairness guard in `src/mc/ref_mc_exact.rs`,
  selected by `RefConfig.nondet`.
- `rfs mc-ref --nondet` CLI flag with updated usage text; BUILD.md row updated.
- The nondet cross-backend diff variant in `tests/ref_mc_diff_test.rs` (green over
  eligible examples + random specs, incl. a liveness case).
- Inventory + dev-doc sync in the same commit; full canonical sweep green with new
  totals recorded.

## Task exit criteria
- [ ] `rfs mc-ref --nondet` agrees with BOTH generated engines on status/reason/label/step over eligible examples + random specs (states exact; transitions not asserted); the diff variant is green, including a liveness-property example exercising the fairness guard.
- [ ] Deterministic mode is byte-unchanged: all pre-existing tests pass without modification (the strategy switch selects effect[0] exactly as before).
- [ ] `RefConfig.nondet` defaults to false; `--nondet` absent = deterministic.
- [ ] Usage text + BUILD.md no longer list --nondet as N/A.
- [ ] Canonical sweep green with new totals recorded (557/685/559/1683 plus the new tests); fmt clean; clippy at baseline; inventory EXACT; sync-check clean.

## Non-goals
- No `--nondet` for fuzz or SMT modes (the generated binaries don't have it either - fuzz steps deterministically by design, SMT encodes first-match firing).
- No change to the generated engines or their templates.
- No transition-count parity assertion (documented convention - the engines count differently).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The fairness guard is the main risk: if ref_mc's liv_analyze cannot express
  "exactly one explored successor AND fully expanded" without a rewrite of its
  stay-graph construction, that is a plan correction with scope re-estimate (record
  it in this file's corrections table - do not silently change semantics).
- Re-check the C backend's nondet liveness section too (c/mc.rs ~1550-1578) - the
  guard must match BOTH backends, not just Rust.
