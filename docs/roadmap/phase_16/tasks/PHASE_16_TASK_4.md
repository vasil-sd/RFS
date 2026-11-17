# Phase 16, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope. Shared context (baselines, structure map) lives in
> `PHASE_16_PLAN.md` ("Context"); the layout rationale in `PHASE_16.md`.

## Goal
Add SMT-bounded checking (`--mode smt-bounded`) to `rfs mc-ref` as an in-process z3
execution over the SHARED `SmtSpec` encoding model (one encoding, three consumers:
C emission, Rust emission, in-process execution), single-source the SMT
status/summary wire format in `mc_concepts.rs`, and pin it with a cross-backend
byte-parity suite under BOTH host z3 5.3.1 and nix z3 5.3.2. This completes the mode
unification and catches emitter/executor divergence in either backend.

## Context
- **The model chain (pinned):** `McPlan::compute(module, &assumptions)`
  (associated fn on `McPlan`, `src/mc/plan.rs` ~218; `assumptions:`
  `&crate::codegen::AssumptionSelection` - All/None/Labels) ->
  `smt_plan::build_smt_spec(module, &plan)` (`src/mc/smt_plan.rs` ~172). The same two
  calls both backends make (rust/mc.rs ~43-44, c/mc.rs ~46-47). `SmtSpec` carries:
  states (declaration order), initial_state, state_bits = max(1, ceil_log2(
  states.len())), vars (SmtVar: name/width/is_bool/init), classes (input
  abstraction), transitions in FIRING order (priority desc, declaration ties) with
  guard/assignment symbolic terms, properties (SmtPropKind 0=safety / 1=liveness +
  formula). `Se`/`Sb`/`Sf` are the symbolic term types; `se_type(se, spec)` gives
  Bool/Bit(w)/IntLit typing per the validator's rules.
- **Inherited restriction:** `build_smt_spec` REJECTS specs with assumptions or
  on_error blocks ("smt-bounded mode does not support ...") - mc-ref --mode
  smt-bounded surfaces the SAME error, same as codegen. No new restriction handling.
- **Encoding conventions (parity-critical, pinned from SMT_ENGINE_RUST at
  rust/mc.rs ~1790-2068; C mirror c/mc.rs ~2583-2890):** step variables are z3
  constants with fixed names declared in a fixed order: `s_{i}` (state, STATE_BITS),
  `v_{i}_{j}` (per var, VAR_WIDTHS), `ev_{i}` (event class, only when N_CLASSES > 1),
  `fld_{i}_{g}` (class fields). Base constraints asserted in a fixed order: initial
  config at step 0 (state eq, then per-var eq), then per step i < k-1: enabled_t =
  state_eq AND class_eq AND guard (firing order), fires_t = enabled_t AND NOT(any
  earlier enabled), any_fire, next-state ITE chain (last transition first), per-var
  ITE chains (unchanged when unassigned). alive_i = alive_{i-1} AND any_fire_i.
  Queries in fixed order: safety properties in declaration order (viol = OR over i<k
  of alive_i AND NOT phi_i; sat -> FAIL at the first i where alive and not phi, read
  from the model), then liveness properties in declaration order (never = AND over i
  of NOT(alive_i AND phi_i); cyc = OR over a<b of config_a == config_b on state+vars;
  sat -> FAIL liveness at pair.0 with cycle length b-a, pair = first matching a<b
  from the model).
- **Wire format + human lines (byte-parity targets):** summary lines -
  `model-check summary: status=fail mode=smt-bounded bound={k}
  reason=property-violation label={} step={vstep}` / `status=unknown
  mode=smt-bounded bound={k}` / `status=fail ... reason=liveness-violation label={}
  step={pair.0}` / `status=pass-within-bound mode=smt-bounded bound={k}`. Human
  lines - "FAIL" + "property violation: {label}" + "  violation step: {vstep}" +
  witness trace ("witness trace (solver model; bound k={k}):" + per-step
  "  step {i}: state={} vars={{...}} event=..." with bools rendered true/false and
  the class field list); "UNKNOWN (solver inconclusive within bound {k})"; liveness
  adds "liveness violation: {label}" + "  violation step: {pair.0}" +
  "  cycle length: {b-a}"; pass prints "PASS WITHIN BOUND {k} (incomplete - no
  completeness claim)". Exit codes: 1 fail, 2 unknown, 0 pass-within-bound.
- **Shared wire format (D-P16-5):** `mc_concepts.rs` gains a `SmtStatus` enum
  (PassWithinBound/Fail/Unknown with wire names + exit codes 0/1/2) +
  `smt_summary_line(status, bound, reason: Option<FailReason>, label: Option<&str>,
  step: Option<usize>) -> String` + SMT summary format constants; both backend
  templates interpolate the constants (byte-identical output - prove with 1683/1683
  + smt_bounded_test BEFORE adding the executor).
- **Feature gating:** the z3 crate is optional (`smt` feature). Codegen of smt
  artifacts is NOT feature-gated; in-process execution IS. `ref_mc_smt.rs` is
  `#[cfg(feature = "smt")]`; `rfs mc-ref --mode smt-bounded` on a non-smt build exits
  with a clear diagnostic + usage (exit 1, the command convention).
- **Independence exception (D-P16-2/D-P16-3):** `ref_mc_smt.rs` MAY import
  `crate::mc::plan` (McPlan::compute) AND `crate::mc::smt_plan` (build_smt_spec +
  the SmtSpec/Se/Sb/Sf model + se_type) - the audit exceptions were encoded in task
  1. Everything else stays off-limits; the executor is ref-mc's OWN code: own Se->z3
  conversion, own unrolling engine, own query construction. It shares the MODEL, not
  the emission.
- **CLI (D-P16-4):** `--mode smt-bounded` + `--bound N` (default
  `SMT_DEFAULT_BOUND` = 10, already in mc_concepts.rs ~597). Exit codes 0/1/2 mirror
  the generated SMT binaries. The generated SMT binary parses ONLY `--bound` - no
  --assumes (specs with assumptions are rejected by build_smt_spec anyway), no other
  flags.
- **Test conventions:** the SMT spec shapes mirror `tests/smt_bounded_test.rs`'s
  inline specs (fail at initial config / liveness cycle / pass-within-bound) -
  define them in the new test file; do NOT restructure smt_bounded_test. Cross-test:
  ref-mc smt vs BOTH generated SMT binaries over the spec family at multiple bounds,
  byte-comparing summary lines + exit codes, under host z3 5.3.1 AND nix z3 5.3.2
  (the task-2b lesson: parity must hold under ANY solver version). Nix half runs in
  the dev shell after wiping target/ (Phase 15 discipline - PHASE_16_PLAN.md
  Context).
- **Verification bar:** canonical sweep unchanged except the new test file adds
  tests; fmt clean; clippy at baseline; inventory EXACT; sync-check clean.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Split ref_mc.rs | the mode-dispatch entry point + the audit's smt import exceptions exist after task 1 |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/mc_concepts.rs` | change | SmtStatus enum + smt_summary_line helper + SMT summary format constants |
| `src/codegen/backends/rust/mc.rs` | change | SMT_ENGINE_RUST interpolates the constants (output byte-identical) |
| `src/codegen/backends/c/mc.rs` | change | SMT_ENGINE_C interpolates the constants (output byte-identical) |
| `src/mc/ref_mc_smt.rs` | new, #[cfg(feature = "smt")] | in-process executor: Se->z3 conversion, unrolling per the pinned conventions, safety+liveness queries, model reading, summary line |
| `src/mc/ref_mc.rs` | change | mode dispatch + non-smt-build diagnostic path |
| `src/cli/commands/mc_ref.rs` | change | --mode smt-bounded + --bound + exit contract + usage text |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/llm/SRC_STRUCTURE.md` | change | new rows: ref_mc_smt.rs items, SmtStatus, smt_summary_line, constants |
| `docs/dev/mc_engine.md` | change | dev:mc-engine-smt fragment (~337) extended with the in-process executor (third consumer) |
| `BUILD.md` | change | mc-ref section: smt-bounded mode, --features smt requirement, exit codes |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_smt_test.rs` | new | ref-mc smt vs both generated SMT binaries: byte-parity summary lines + exit codes over the fail/liveness/pass spec family at multiple bounds, under both z3 versions |

## Planned actions

### 1. Single-source the SMT status/summary wire format and interpolate it into both templates
Add `SmtStatus` (wire names pass-within-bound/fail/unknown; exit codes 0/1/2) +
`smt_summary_line(...)` + the format constants to `src/codegen/mc_concepts.rs`.
Replace the inline summary format strings in BOTH SMT engine templates with
interpolation of the constants - there are FIVE distinct printf/println sites per
backend (rust ~1985/1994/2042/2052/2060, C ~2808/2815/2871/2879/2886), not one.
Prove byte-identity FIRST: 1683/1683 generated-code suite + smt_bounded_test (incl.
its backend parity pins) before adding the executor. Rationale: D-P16-5; a moved
byte is a plan correction, not a fix.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/mc_concepts.rs` | change | SmtStatus + smt_summary_line + constants with D-P16-5 doc comments |
| `src/codegen/backends/rust/mc.rs` | change (~1985-2060) | template interpolates the constants; emitted bytes unchanged |
| `src/codegen/backends/c/mc.rs` | change (~2808-2886) | same for C (five sites) |

### 2. Implement the in-process executor ref_mc_smt.rs
New file, `#[cfg(feature = "smt")]`: consume the model chain (`McPlan::compute(
module, &selection)` -> `build_smt_spec`), surface build_smt_spec's rejection errors
verbatim (assumptions/on_error specs). Walk `Se`/`Sb`/`Sf` with the z3 crate using
`se_type` for sort selection (Bool vs BV of the given width; IntLit lifted per parent
context - mirror the emitters' rules). Build step variables with the pinned names/
order, assert base constraints in the pinned order (initial config, then per-step
enabled/fires/any_fire/alive/next-state/per-var ITE chains), run safety queries in
declaration order then liveness queries (never + cycle), read models exactly as the
templates do (first violating i; first equal-config a<b pair), print the human lines
+ witness trace in the pinned formats, and end with `smt_summary_line`. Exit codes
0/1/2. Rationale: D-P16-3 - one encoding, three consumers; the executor keeps its
OWN unrolling + queries so an emitter bug and an executor bug remain distinguishable,
while a model bug is caught by all three agreeing.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_smt.rs` | new | the executor + unit tests (spec-shape pins: fail-at-initial, liveness-cycle, pass-within-bound; rejection-error pin for assumptions/on_error specs) |

### 3. Wire the mode dispatch + CLI
Route `--mode smt-bounded` from the ref_mc entry into the executor (feature-gated
call site); on a non-smt build print a clear diagnostic naming the missing `smt`
feature + usage, exit 1. In `src/cli/commands/mc_ref.rs`: the mode value, `--bound N`
(default SMT_DEFAULT_BOUND), exit-code mapping 0/1/2, usage text. Rationale: D-P16-4
- same flag names/defaults/exit codes as the generated SMT binaries; the feature
diagnostic is the one deliberate surface difference (codegen of smt artifacts is not
gated, execution is).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc.rs` | change | dispatch + cfg-gated call + diagnostic |
| `src/cli/commands/mc_ref.rs` | change | --mode smt-bounded + --bound + exit contract + usage text |

### 4. Add the cross-backend suite under both z3 versions and run the full sweep
New `tests/ref_mc_smt_test.rs`: define the spec family (fail at initial config /
liveness cycle / pass-within-bound - shapes mirroring smt_bounded_test's inline
specs, defined locally) and run ref-mc smt vs BOTH generated SMT binaries at multiple
bounds (incl. bound 1 and a bound beyond the violation depth), byte-comparing summary
lines + exit codes. The host half runs in the canonical sweep (z3 5.3.1); the nix
half runs in the dev shell (z3 5.3.2) after wiping target/ - record both results.
Then run the full canonical sweep and record the new totals. Rationale: the third
execution target of the unified encoding catches emitter/executor divergence in
either backend; cross-solver-version coverage per the Phase 15 task-2b lesson.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | new | the cross-backend suite (host half in-crate; nix half documented for the dev-shell run) |
| `docs/llm/SRC_STRUCTURE.md` | change | new rows - same commit |
| `docs/dev/mc_engine.md` | change | dev:mc-engine-smt extended - same commit |
| `BUILD.md` | change | mc-ref smt-bounded documentation - same commit |

## Task deliverables
- `rfs mc-ref --mode smt-bounded --bound N` with 0/1/2 exit codes mirroring the
  generated SMT binaries; clear diagnostic on non-smt builds.
- `src/mc/ref_mc_smt.rs` - the in-process z3 executor over the shared SmtSpec model
  (feature-gated).
- `mc_concepts::SmtStatus` + `smt_summary_line` + format constants, interpolated into
  both templates with byte-identical output.
- `tests/ref_mc_smt_test.rs` green under host z3 5.3.1; nix z3 5.3.2 half verified in
  the dev shell.
- Inventory + dev-docs + BUILD.md sync in the same commit; full canonical sweep green
  with new totals recorded.

## Task exit criteria
- [ ] `rfs mc-ref --mode smt-bounded` agrees with BOTH generated SMT binaries on status/label/step/bound over the fail/liveness/pass spec family at multiple bounds, under host z3 5.3.1 AND nix z3 5.3.2; `ref_mc_smt_test` green (host half) + dev-shell run recorded (nix half).
- [ ] On a non-smt build, `--mode smt-bounded` exits with the clear diagnostic (exit 1), not a panic or a link error.
- [ ] Specs with assumptions or on_error blocks are rejected with build_smt_spec's verbatim error, same as codegen.
- [ ] The SMT summary format is single-sourced in mc_concepts.rs and interpolated into both templates; generated artifact bytes unchanged (1683/1683, smt_bounded_test parity pins unmodified and green).
- [ ] Canonical sweep green with new totals recorded; fmt clean; clippy at baseline; inventory EXACT; sync-check clean.

## Non-goals
- No deep refactor of smt_emit.rs's emission architecture (no shared "engine model" layer between emitters and executor) - D-P16-3's model-level sharing is the stopping point.
- No independent SMT encoding in ref-mc (the deliberate trade-off: a plan/encoding bug is shared by all three consumers; the independent ground truth remains in exact/nondet/fuzz).
- No `--assumes` flag for smt-bounded mode (build_smt_spec rejects assumption-bearing specs - same as codegen).
- No new solver features (no incremental solving, no tactics) - mirror the templates' plain Solver usage.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- This task may need several z3 iteration cycles (solver-version-sensitive behavior is
  exactly what it must cover). Budget accordingly.
- Re-verify the C template's summary-line sites (~2808/2815/2871/2879/2886) one by one
  when interpolating - there are five distinct printf sites, not one.
- The witness-trace format (smt_print_trace) is part of the byte-parity surface for
  FAIL runs - pin it in the test, not just the summary line.
