# Phase 16, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope. Shared context (baselines, structure map) lives in
> `PHASE_16_PLAN.md` ("Context"); the layout rationale in `PHASE_16.md`.

## Goal
Add the seeded random-testing mode (`--mode fuzz`) to `rfs mc-ref`: the 3-phase
input strategy with the EXACT RNG draw sequence of the generated runtimes (shared
LCG), `--violate` forced-violation directives, per-run violation reports, and the
fuzz summary line - single-sourcing the wire format in `mc_concepts.rs` and pinning
everything with a cross-backend byte-parity suite. This delivers the user's primary
prototyping goal: random testing without any compilation cycle.

## Context
- **The draw sequence (parity-critical contract - mirror EXACTLY, from
  PHASE_16_PLAN.md "Context"):** per run: (1) for each violation spec in order:
  `rng.chance(spec.prob)` -> `violating[i]`. Per step 0..bound: (2a) forced phase -
  for each spec index in order, IF `violating[idx]`: `rng.chance(target_fraction[idx])`;
  if true, scan ALL_INPUTS in order with DRAW-FREE trial steps (clone + deterministic
  first-match step); the first input whose trial yields AssumptionViolation with
  `label == spec.label` OR PropertyViolation with that label wins; step the real
  state with it (Stop handler); on AssumptionViolation -> status
  `assumption-violated:<label>`, count, break; then check property violation ->
  report + exit 1; continue. (2b) unexpected phase - `rng.chance(unexpected_prob)`;
  if true: `pick_index(ALL_INPUTS.len())`, step with the CONTINUE assumption handler;
  on UnexpectedEvent -> status `unexpected-event`, count, break; check property
  violation; continue. (2c) allowed phase - build the allowed list by scanning
  ALL_INPUTS in order with draw-free trial steps (Stop handler): keep inputs whose
  trial is Ok OR PropertyViolation; if empty -> status `stuck`, break;
  `pick_index(allowed.len())`, step with Stop handler, map errors to statuses; check
  property violation. After the loop: status `completed` increments the counter.
  After ALL runs: print the fuzz summary line. Exit 1 ONLY on a property violation -
  IMMEDIATELY, so NO summary line is printed in that case; else exit 0. Trial steps
  consume NO draws (they run on clones before any draw of the real step).
- **RNG:** `utils::rng::Rng` already provides `chance` and - FINDING vs the plan's
  table - `pick_index` ALREADY EXISTS with exactly the required f64 form
  `((next_f64() * len as f64) as usize) % len` (the plan said "add"; no rng.rs
  change is needed, only use). The generated templates keep their inline Rng copies
  interpolating `__LCG_MULT__`/`__LCG_ADD__` - unchanged. Note: the generated
  `chance` clamps p to [0,1] while `utils::rng::Rng::chance` does not - behaviorally
  identical because next_f64() is in [0,1) (p > 1 always true, p < 0 always false
  either way); no action needed.
- **The generated fuzz runtime (pinned locations):** Rust template `FUZZ_RUNTIME`
  at `src/codegen/backends/rust/mc.rs` ~1604-1789; C mirror at
  `src/codegen/backends/c/mc.rs` ~2384-2580. Defaults: bound=5, runs=20, seed=12345,
  assumes="all", violate=[], unexpected_prob=0.0. Summary line (rust ~1779 / C
  ~2572): `model-check summary: runs={} bound={} completed={} unexpected={}
  assumption_stops={} property_violations={}` - the `property_violations` counter is
  always 0 (a violation exits immediately) but the field stays for a stable format.
- **Per-run violation report (byte-parity target, from report_property_violation +
  print_trace in the templates):**
  ```text
  run {run} property violation: {label}
    trigger step: {t}                    (only when Some)
    vars at trigger step: {vars_at(t)}   (only when Some)
    violation step: {step}
    vars at violation step: {vars_at(step)}
  counterexample trace:
    initial state={state} vars={Vars {...}}
    step {i} input={Input::...} outputs=[...] from={...} to={...} vars={...}
  ```
  then exit(1). `vars_at(0)` = initial vars, `vars_at(n)` = post-step-(n-1) vars.
  ref_mc's renderers (`render_input`/`render_output`/`render_vars` in ref_mc.rs
  ~3064-3131) already produce the byte-identical `Input::`/`Output::`/`Vars { }`
  formats (documented for the exact text report) - reuse them.
- **Step semantics for the fuzz engine:** the generated FSM steps DETERMINISTICALLY
  (first matching transition; fuzz randomizes inputs, not transitions). ref_mc's
  fuzz engine therefore uses `match_effect` (first match) - no nondet involvement.
  Assumptions: only SELECTED assumptions are monitored (`parse_assume_selection`);
  a selected-assumption failure under the Stop handler = AssumptionViolation{label};
  under the Continue handler the step proceeds; no matching transition and no
  on_error = UnexpectedEvent. Property monitors run after assumption monitors (a
  pruned successor is never reported as a property violation) - same order as the
  exact BFS.
- **Shared wire format (D-P16-5):** `mc_concepts.rs` gains
  `fuzz_summary_line(runs, bound, completed, unexpected, assumption_stops,
  property_violations) -> String` + a `FUZZ_SUMMARY_FORMAT` constant; both backend
  templates interpolate the constant into their inline println/printf (the
  LCG-interpolation precedent). Generated output must stay BYTE-IDENTICAL - prove it
  with the 1683/1683 generated-code suite + the existing backend_parity fuzz tests
  BEFORE adding the ref-mc engine. If any byte moves, stop: that is a plan
  correction, not a fix.
- **CLI (D-P16-4):** `--mode exact|fuzz` (default exact) + fuzz flags `--bound N`
  (5), `--runs N` (20), `--seed N` (12345), `--assumes`,
  `--violate label:prob:percent%,...`, `--unexpected-prob P` (0.0). Exit codes:
  0 no property violation / 1 violation found. The exact-mode flags keep working
  under `--mode exact`. `--counterexample-format`/`--max-states`/`--timeout-sec`
  are EXACT-MODE-ONLY: the generated fuzz binary has no such flags, so combining
  them with `--mode fuzz` is a usage error (same for smt-bounded in task 4).
- **Test conventions:** reuse tests/ref_mc_diff_test.rs patterns - eligible examples
  minus EXCLUDED (>=40), specgen random specs with pinned seeds, BIN_CACHE
  per-process binary cache; byte-compare summary lines + per-run violation reports
  against BOTH backends' generated fuzz binaries; assert exit-code mirroring.
- **Verification bar:** canonical sweep unchanged except the new test file adds
  tests; fmt clean; clippy at baseline; inventory EXACT; sync-check clean.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Split ref_mc.rs | the fuzz engine is a new file built on the isolated common core; the mode-dispatch entry point exists after the split |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/mc_concepts.rs` | change | fuzz_summary_line helper + FUZZ_SUMMARY_FORMAT constant |
| `src/codegen/backends/rust/mc.rs` | change | FUZZ_RUNTIME interpolates the format constant (output byte-identical) |
| `src/codegen/backends/c/mc.rs` | change | C fuzz runtime interpolates the format constant (output byte-identical) |
| `src/mc/ref_mc_fuzz.rs` | new | the seeded engine: run loop, 3-phase strategy with the exact draw sequence, per-run violation report, summary line |
| `src/mc/ref_mc.rs` | change | mode dispatch entry (run() routes exact vs fuzz); the fuzz config type |
| `src/cli/commands/mc_ref.rs` | change | --mode exact\|fuzz + fuzz flags + exit contract + usage text |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/llm/SRC_STRUCTURE.md` | change | new rows: ref_mc_fuzz.rs items, fuzz_summary_line, format constant |
| `docs/dev/mc_engine.md` | change | new dev:mc-engine-fuzz fragment (strategy + draw sequence + report format) |
| `BUILD.md` | change | mc-ref section: fuzz mode flags/defaults/exit codes |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_fuzz_test.rs` | new | ref-mc fuzz vs both generated fuzz binaries: byte-parity on summary lines + per-run reports, exit-code mirroring, over eligible examples + random specs |

## Planned actions

### 1. Single-source the fuzz summary line in mc_concepts.rs and interpolate it into both templates
Add `FUZZ_SUMMARY_FORMAT` + `fuzz_summary_line(...)` to `src/codegen/mc_concepts.rs`
(the exact_summary_line precedent, D-P6-10). Replace the inline format strings in
BOTH backend templates with interpolation of the constant (Rust: `format!` over the
constant; C: the printf format string built from the constant - keep the rendered
template standalone). Prove byte-identity FIRST: run the 1683/1683 generated-code
suite + backend_parity_test's fuzz/violate tests + smt_bounded_test before touching
anything else in this task. Rationale: D-P16-5 - one wire format, three consumers;
the proof order matters because a moved byte is a plan correction, not a fix.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/mc_concepts.rs` | change | new helper + constant with doc comment citing D-P16-5 |
| `src/codegen/backends/rust/mc.rs` | change (~1779) | template interpolates the constant; emitted bytes unchanged |
| `src/codegen/backends/c/mc.rs` | change (~2572) | same for C |

### 2. Implement the ref_mc_fuzz.rs engine
New file (no feature gate): a fuzz run loop over bound/runs/seed with the EXACT
draw sequence (Context bullet 1) via `utils::rng::Rng` (`chance`, `pick_index` -
both already exist). Build a private step primitive on the common core:
deterministic first-match transition (`match_effect`), selected-assumption monitors
under Stop/Continue handler semantics, property monitors after assumptions,
UnexpectedEvent when nothing matches and no on_error. Per-run property violation
renders the byte-parity report (Context bullet 4) using ref_mc's renderers and exits
1 immediately (no summary line in that case); after all runs prints
`fuzz_summary_line`. Rationale: the draw-sequence contract is localized in one file
and pinned by cross-backend byte parity - a future strategy change cannot silently
skew one engine.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_fuzz.rs` | new | the engine + unit tests (draw-sequence pins on tiny specs: forced-violation hit, unexpected injection, stuck, completed; report byte-pins) |

### 3. Wire the mode dispatch + CLI
Extend the ref_mc entry with a fuzz configuration (bound/runs/seed/assumes/violate/
unexpected_prob - parsed via the shared `ViolationSpec::parse` and
`parse_assume_selection`) and route by mode. In `src/cli/commands/mc_ref.rs`:
`--mode exact|fuzz` (default exact), the six fuzz flags with the generated defaults,
exit codes 0/1 for fuzz, usage text documenting the full surface (keeping
--threads/--fast documented N/A; exact-only flags rejected under `--mode fuzz`).
Rationale: D-P16-4 - near-exact CLI surface; same names, defaults, wire formats,
exit codes as the generated fuzz binaries.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc.rs` | change | mode dispatch + fuzz config type |
| `src/cli/commands/mc_ref.rs` | change | --mode + fuzz flags + exit contract + usage text + module doc |

### 4. Add the cross-backend byte-parity suite and run the full sweep
New `tests/ref_mc_fuzz_test.rs`: at fixed seed/bound/runs/violate/unexpected-prob
(several combinations, incl. a --violate directive that actually fires and an
--unexpected-prob > 0 case), over eligible examples + specgen random specs with
pinned seeds, byte-compare the summary line AND per-run violation reports against
BOTH backends' generated fuzz binaries (BIN_CACHE pattern); assert exit-code
mirroring (0/1) - including that a violating run prints NO summary line. Then run
the full canonical sweep and record the new totals. Rationale: independent
verification of the seeded strategy and the draw-sequence contract - today this mode
has only shared-plan backend parity.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_fuzz_test.rs` | new | the byte-parity suite |
| `docs/llm/SRC_STRUCTURE.md` | change | new rows - same commit |
| `docs/dev/mc_engine.md` | change | dev:mc-engine-fuzz fragment - same commit |
| `BUILD.md` | change | mc-ref fuzz mode documentation - same commit |

## Task deliverables
- `rfs mc-ref --mode fuzz` with the full generated flag surface and 0/1 exit contract.
- `src/mc/ref_mc_fuzz.rs` - the seeded engine with the exact draw sequence, built on
  the common core + `utils::rng`.
- `mc_concepts::fuzz_summary_line` + `FUZZ_SUMMARY_FORMAT`, interpolated into both
  backend templates with byte-identical output.
- `tests/ref_mc_fuzz_test.rs` green: byte-parity summary lines + per-run reports vs
  both backends, exit-code mirroring.
- Inventory + dev-docs + BUILD.md sync in the same commit; full canonical sweep green
  with new totals recorded.

## Task exit criteria
- [ ] `rfs mc-ref --mode fuzz` produces byte-identical summary lines and per-run violation reports to BOTH generated fuzz binaries at fixed seeds over eligible examples + random specs; exit codes mirror (0/1, no summary line on a violating run); `ref_mc_fuzz_test` green.
- [ ] The fuzz summary format is single-sourced in mc_concepts.rs and interpolated into both templates; generated artifact bytes unchanged (1683/1683, backend_parity fuzz tests unmodified and green).
- [ ] Trial steps consume no RNG draws (unit-pinned); the draw sequence matches the plan's contract step for step.
- [ ] `--mode exact` remains the default and is byte-unchanged; all pre-existing tests pass without modification.
- [ ] Canonical sweep green with new totals recorded; fmt clean; clippy at baseline; inventory EXACT; sync-check clean.

## Non-goals
- No nondet fuzzing (the generated fuzz runtimes step deterministically - first match; fuzz randomizes inputs only).
- No libFuzzer/coverage-guided mode in mc-ref (compile-based by nature).
- No changes to the LCG constants or the templates' inline Rng copies.
- No `--threads`/`--fast` (documented N/A).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The plan's Context said "add pick_index to utils::rng" - it already exists with the
  exact f64 form; verify by reading src/utils/rng.rs and record this finding in the
  status file (no rng.rs change needed).
- If a per-run report byte differs from the generated output, diff the RENDERERS
  first (render_input/render_output/render_vars vs the generated rfs_render_*
  functions) before suspecting the draw sequence - the exact-mode text report is
  already byte-pinned to the same formats.
- The `property_violations` summary field is always 0 by construction (violation
  exits immediately) - keep it, pin it.
