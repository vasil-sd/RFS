# Phase 16 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation immediately without
> re-deriving scope. The source-subdivision analysis with rationales lives in
> `PHASE_16.md` ("Source subdivision analysis" section) - read it first.

## Context

**Baselines (Phase 15 close-out, 2026-11-16 - the numbers this phase must preserve):**
`cargo test` **557**/0, `--workspace` **685**/0, `--features smt` **559**/0,
`test-generated-code` **1683**/1683, clippy 55 warning lines / 0 errors (34 unique,
clippy 1.99.0), fmt clean; nix doCheck build **42/42**; nix develop full smt suite
559/0. New tests in this phase ADD to these counts - record the new totals in the
status file, never silently change the old ones.

**The ref_mc.rs structure map (measured 2026-11-17, line numbers drift - use
`rfs-dev context` / grep, not these ranges):**
- Common core (stays in `ref_mc.rs`): values/typing (`Val`, `RType`, `expr_type`,
  `binary_type`, ~56-244), evaluator (`EvalCtx`, `eval_expr`, `bool_of`, `val_eq`,
  ~246-409), signals/inputs/outputs (`Sig`, `InputVal`, `OutputVal`, `Sample`,
  `eval_sig`, `FlatVariant`, `flatten`, `input_matches`, `output_matches`,
  ~411-617), monitors (`Range`, `Mon`, `compile_formula/check/signal`, `MonSt`,
  `mon_init/step`, `min_trigger`, `mon_key/bits`, ~619-1076), representative inputs
  (`referenced_fields`, `field_domain`, `representative_value`, `all_inputs`,
  ~1078-1283), effects (`Effect`, `pattern_bindings`, `match_effect`,
  `input_matches_variant`, ~1285-1497), liveness decomposition (`Leaf`, `Skel`,
  `LivProp`, `decompose_top/properties`, ~1499-1640), formula/expression printing
  (~1642-1882), config/result types (`RefConfig`, `IoRec`, `StepRec`, `CeData`,
  `RefResult`, ~1884-1989), rendering (`render_*`, `render_text(_impl)`,
  `to_json_report`, ~3064-3414), counterexample verification (`initial_config`,
  `step_trace`, `check_initial/step_config`, `property_confirms_at`,
  `verify_lasso/deadlock/counterexample`, ~3620-4296).
- Exact engine (moves to `ref_mc_exact.rs`): `BNode`/`CfgKey`/`val_key`/`bnode_key`
  (~1991-2053), `run()` BFS (~2055-2465), `path_to`/`state_name`/`io_in`/`io_out`/
  `trace_steps` (~2467-2546), liveness analysis (`LivWitness`, `leaf_signals`,
  `skel_eval`, `liv_analyze`, `prod_delta`, `leaf_good/bits`, `liv_note`,
  ~2548-3062), `minimal_violation_labels`/`collect_min_labels` (~3416-3618).
- Unit tests (~4298-4971): follow their code - exact-engine tests move with the
  engine; common-core tests stay. The `independence_import_audit` test stays in
  `ref_mc.rs` but must be extended to read ALL `src/mc/ref_mc_*.rs` files (it
  currently self-reads only `ref_mc.rs`) and keep the forbidden list
  (`mc::plan`, `monitor`, `codegen::fsm`, `printer`, `backends`) - with the
  documented exceptions that `ref_mc_smt.rs` MAY import `crate::mc::plan`
  (McPlan::compute) AND `crate::mc::smt_plan` (build_smt_spec + SmtSpec model,
  D-P16-3), and `ref_mc_fuzz.rs` MAY import `crate::utils::rng`.

**What is shared today (do not break):** `mc_concepts` report types +
`exact_summary_line` + `parse_assume_selection` + `ViolationSpec` + `McPhase::ALL`
+ `McConfig` defaults + exact defaults (D-P6-10); `utils::rng` LCG
(`LCG_MULT`/`LCG_ADD`, interpolated into both generated fuzz runtimes via the
`__LCG_MULT__`/`__LCG_ADD__` placeholders - the established template-interpolation
precedent to copy for the new summary-line constants).

**The fuzz RNG draw sequence (the parity-critical contract - mirror EXACTLY):**
Per run: (1) for each violation spec in order: `rng.chance(spec.prob)` ->
`violating[i]`. Per step 0..bound: (2a) forced phase - for each spec index in
order, IF `violating[idx]`: `rng.chance(target_fraction[idx])`; if true, scan
`ALL_INPUTS` in order with DRAW-FREE trial steps (`fsm.clone()` +
`step_with_handler`); the first input whose trial yields `AssumptionViolation`
with `label == spec.label` OR `PropertyViolation` with that label wins; step the
real FSM with it (Stop handler); on AssumptionViolation -> status
`assumption-violated:<label>`, count, break; then check property violation ->
report + exit 1; continue. (2b) unexpected phase - `rng.chance(unexpected_prob)`;
if true: `pick_index(ALL_INPUTS.len())`, step with the CONTINUE assumption handler;
on UnexpectedEvent -> status `unexpected-event`, count, break; check property
violation; continue. (2c) allowed phase - build the allowed list by scanning
`ALL_INPUTS` in order with draw-free trial steps (Stop handler): keep inputs whose
trial is Ok OR PropertyViolation; if empty -> status `stuck`, break;
`pick_index(allowed.len())`, step with Stop handler, map errors to statuses;
check property violation. After the loop: status `completed` increments the
counter. After all runs: print the fuzz summary line. Exit 1 ONLY on a property
violation (immediate), else 0.
**CRITICAL:** `pick_index(len)` is `((next_f64() * len as f64) as usize) % len` -
the f64-based form, NOT `utils::rng::Rng::pick` (which uses u64 modulo and would
desynchronize the stream). Add a `pub fn pick_index(&mut self, len: usize)
-> Option<usize>` to `utils::rng::Rng` implementing exactly this expression and
use it from `ref_mc_fuzz.rs`; the generated templates keep their inline copies
(pinned by the new byte-parity tests) - same pattern as the LCG constants.
Trial steps must consume NO draws (they run on clones before any draw of the
real step).

**The --nondet semantics (mirror the generated engines):** the exact engine
computes ALL matching transition effects per input and fires `effects[0]`
(deterministic) or EVERY effect (nondet). ref_mc's `match_effect` returns only the
first match - add an all-matches enumeration and a nondet BFS strategy that enqueues
one successor per (input, enabled transition). Deadlock in nondet = no transition
fires for ANY input. **Open question to pin during task 2:** the generated
liveness engine applies a nondet fairness filter on the stay-edge subgraph (see the
`graph.nondet && !(adj.len()==1 && complete)` guard in `backends/rust/mc.rs`
liveness section) - ref_mc's `liv_analyze` needs the matching nondet variant;
reproduce it and pin it with the cross-backend nondet liveness cases (the eligible
examples include liveness properties).

**The SMT in-process execution (mirror the parity-critical conventions from the
SMT engine templates):** step variables are z3 constants with fixed names declared
in a fixed order; base constraints asserted in a fixed order (initial config at
step 0, then per-step: enabled_t = state_eq and class_eq and guard, fires_t =
enabled_t and NOT(any earlier enabled), any_fire, next-state ITE chain, per-var
ITE chains); alive_i = alive_{i-1} and any_fire_i; queries in fixed order - safety
properties in declaration order (viol = OR over i<k of alive_i and NOT phi_i;
sat -> FAIL at the first i where alive and not phi, read from the model), then
liveness properties in declaration order (never = AND over i of NOT(alive_i and
phi_i); cyc = OR over a<b of config_a == config_b; sat -> FAIL liveness at pair.0,
cycle length b-a). `SmtSpec` (from `smt_plan::build_smt_spec(module, plan)`) carries
guards/properties/effects as `Se`/`Sf` symbolic terms - the in-process executor
walks them with the z3 crate (feature `smt`), using `se_type` for typing. The
executor is ref-mc's OWN code (D-P16-2/D-P16-3): it shares the MODEL, not the
emission. **Inherited restriction:** `build_smt_spec` rejects specs with
assumptions or on_error blocks ("smt-bounded mode does not support ...") -
mc-ref --mode smt-bounded surfaces the SAME error, same as codegen.

**CLI unification (D-P16-4) - target surface:**
`rfs mc-ref <spec.rfs|spec.json> [--mode exact|fuzz|smt-bounded] [mode flags]
[--log-level ...]`. exact (default): `--max-states N` (1000000), `--nondet`,
`--timeout-sec N` (0=off), `--assumes all|none|labels`, `--counterexample-format
json|text`. fuzz: `--bound N` (5), `--runs N` (20), `--seed N` (12345),
`--assumes`, `--violate label:prob:percent%,...`, `--unexpected-prob P` (0.0).
smt-bounded: `--bound N` (10 = `SMT_DEFAULT_BOUND`). Exit codes mirror the
generated binaries per mode: exact 0/1/2, fuzz 0 (no property violation) / 1
(violation found), smt 0 (pass-within-bound) / 1 (fail) / 2 (unknown). Smt-bounded
on a non-smt build: clear diagnostic + usage exit (the z3 crate is optional;
codegen of smt artifacts is NOT feature-gated, in-process execution IS).
`--threads`/`--fast`: documented N/A in the usage text.

**Test conventions to reuse (from `tests/ref_mc_diff_test.rs`):** eligible examples
= `examples/valid` minus the pinned EXCLUDED list (assert >=40); random specs via
`rfs::lang::specgen::SpecGen::new(seed, Bounds::small())` with pinned seeds;
generated binaries cached per test process (`BIN_CACHE` pattern); byte-parity on
summary lines/reports, status + exit-code agreement always asserted, transition
counts NOT asserted cross-engine (documented convention). The SMT spec shapes for
`ref_mc_smt_test` mirror `tests/smt_bounded_test.rs`'s inline specs (fail at initial
config / liveness cycle / pass-within-bound) - define them in the new test file;
do not restructure smt_bounded_test.

**Nix verification discipline (Phase 15 close-out finding):** after any HOST
canonical sweep, `target/` holds host-rustc rlibs that make nix-develop runs fail
E0514 in `nostd_minimal.rs` - wipe `target/` before nix develop suite runs. Nix
binary: `/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin/nix`
(not on PATH), `--impure` required. The external mmd half of
diagram_render_parity_test needs a short TMPDIR in this sandbox (Chrome socket
path artifact - pre-existing, unrelated).

**Docs-sync obligation:** every task that moves/adds/removes fns or types updates
the SRC_STRUCTURE inventory rows in the SAME commit (PROTOCOL.md section 5) and
finishes with `rfs-dev inventory audit` EXACT + `rfs-dev sync-check` clean. The
moved exact-engine items change their file column to `src/mc/ref_mc_exact.rs`;
new mode engines add new rows. `//! Docs:` pointer lines move with the code; the
docs/dev MC-engine fragments gain per-mode sections.

**Commit discipline:** one commit per task, FORMATTING.md section 7 format
(subject + Changes / Issues and findings / Verification results).

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Split ref_mc.rs into common core + ref_mc_exact.rs (pure move, zero behavior change) | `src/mc/ref_mc.rs` (change), `src/mc/ref_mc_exact.rs` (new), `src/mc/mod.rs` (change), `docs/llm/SRC_STRUCTURE.md` (change - moved rows' file column), docs/dev MC-engine fragments (change - pointers) | Move the exact engine (BNode/CfgKey/keying, run() BFS body, path/trace, liv_analyze + product graph + witnesses, minimal_violation_labels) and its unit tests into `ref_mc_exact.rs`; `ref_mc.rs` keeps the common core + a `run()` entry that calls the exact engine for now; extend `independence_import_audit` to read all `src/mc/ref_mc_*.rs`; adjust visibility (`pub(crate)` where needed); update mod.rs docs. Rationale: D-P16-1 - the split must prove byte-safe BEFORE any new mode lands, so every later task starts from a verified layout. Verify: full canonical sweep with UNCHANGED counts (557/685/559/1683), fmt clean, clippy at baseline, inventory EXACT, sync-check clean. |
| 2. | --nondet mode in the ref exact engine + CLI flag + cross-backend diff variant | `src/mc/ref_mc_exact.rs` (change - all-matches effect enumeration + nondet BFS strategy + nondet liveness variant), `src/mc/ref_mc.rs` (change - RefConfig.nondet, run() passes it), `src/cli/commands/mc_ref.rs` (change - --nondet flag + usage text), `tests/ref_mc_diff_test.rs` (change - nondet comparison variant), `docs/llm/SRC_STRUCTURE.md` + docs/dev fragments (change) | Implement the nondet successor strategy (one successor per enabled transition per input; deadlock = nothing fires for any input) and the matching liveness semantics (pin the generated engine's nondet fairness filter first - Context open question). CLI: `--nondet` on exact mode. Cross-test: ref-mc --nondet vs BOTH generated engines with --nondet on status/reason/label/step over eligible examples + random specs (states exact; transitions not asserted). Rationale: first independent ground truth for nondet semantics - today only Rust-to-C parity exists over one shared plan. |
| 3. | Seeded fuzz mode: shared summary helper + ref_mc_fuzz.rs + CLI + cross-backend byte-parity suite | `src/codegen/mc_concepts.rs` (change - fuzz_summary_line + format constant), `src/utils/rng.rs` (change - pick_index f64 method), `src/codegen/backends/rust/mc.rs` + `src/codegen/backends/c/mc.rs` (change - templates interpolate the format constant; output byte-identical), `src/mc/ref_mc_fuzz.rs` (new - the seeded engine per the Context draw sequence), `src/mc/ref_mc.rs` (change - mode dispatch), `src/cli/commands/mc_ref.rs` (change - --mode exact\|fuzz + fuzz flags + exit contract), `tests/ref_mc_fuzz_test.rs` (new), `docs/llm/SRC_STRUCTURE.md` + docs/dev fragments + BUILD.md mc-ref section (change) | The 3-phase strategy with the EXACT draw sequence (Context) via the shared LCG; per-run violation report + exit 1; fuzz summary line from the new shared helper. Templates keep standalone rendered copies but interpolate the format constant (D-P16-5, LCG precedent). Cross-test: fixed seed/bound/runs/violate/unexpected-prob over eligible examples + random specs, byte-comparing summary lines + per-run reports against BOTH backends. Rationale: the user's primary prototyping goal - random testing without compilation - plus independent verification of a mode that today has only shared-plan backend parity. |
| 4. | SMT-bounded mode: shared SmtStatus/summary helper + ref_mc_smt.rs (in-process z3 over SmtSpec) + CLI + cross-backend suite under both z3 versions | `src/codegen/mc_concepts.rs` (change - SmtStatus enum + smt_summary_line + format constants), `src/codegen/backends/rust/mc.rs` + `src/codegen/backends/c/mc.rs` (change - SMT engine templates interpolate the constants; output byte-identical), `src/mc/ref_mc_smt.rs` (new, #[cfg(feature = "smt")] - in-process executor per the Context conventions, consuming smt_plan::build_smt_spec), `src/mc/ref_mc.rs` (change - mode dispatch + non-smt-build diagnostic), `src/cli/commands/mc_ref.rs` (change - --mode smt-bounded + --bound), `tests/ref_mc_smt_test.rs` (new), `docs/llm/SRC_STRUCTURE.md` + docs/dev fragments + BUILD.md (change) | One encoding, three consumers (D-P16-3): the in-process executor walks SmtSpec with the z3 crate, own unrolling engine + query construction, fixed variable/constraint/query order per the template conventions. Cross-test: ref-mc smt vs BOTH generated SMT binaries over the fail/liveness/pass spec family at multiple bounds, under host z3 5.3.1 AND nix z3 5.3.2 (the task-2b lesson: parity must hold under ANY solver version). Rationale: completes the mode unification; catches emitter/executor divergence in either backend. |
| 5. | Phase close-out: full verification + bookkeeping | `docs/roadmap/phase_16/PHASE_16_STATUS.md` (new), `TASKS.md` (change - append), `docs/roadmap/CURRENT_STATE.md` (change - phase-16 row) | Run the complete exit-criteria sweep: canonical baselines unchanged + new test counts recorded (557/685/559/1683 plus the new fuzz/smt/nondet tests), fmt clean, clippy at baseline, inventory EXACT, sync-check clean; nix doCheck still 42/42 and dev-shell smt suite green (fresh target after host runs - Context nix discipline); write PHASE_16_STATUS.md per FORMATTING.md section 4 (files, functionality deltas, exact test counts, findings incl. the pinned nondet liveness semantics and any draw-sequence corrections), append the TASKS.md entry, update CURRENT_STATE.md. Rationale: PROTOCOL.md section 3 steps r-u. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|

## Other information

- **Task order matters:** task 1 (split) must land and verify BEFORE any mode work -
  it is the safety net that proves the refactor moved nothing. Tasks 2-4 are
  independent of each other after task 1 (any order), but the planned order
  (nondet -> fuzz -> smt) goes from smallest to largest blast radius.
- **Byte-identity stakes:** tasks 3-4 touch the backend templates. The interpolation
  must change NO emitted bytes - prove it with the 1683/1683 generated-code suite +
  the existing parity pins (backend_parity_test fuzz/violate tests,
  smt_bounded_test) BEFORE adding the new ref-mc engines. If any byte moves, stop:
  that is a plan correction, not a fix.
- **The nondet liveness filter is the main open question** (Context): pin it from
  the generated engine's source during task 2 and record the pinned semantics in
  the task status file. If ref_mc's liv_analyze cannot express it without a rewrite,
  that is a plan correction with scope re-estimate.
- **f64 draw parity is exact by construction** only if both sides use IEEE-754
  double with the same operations - Rust f64 and C double are the same type here;
  no rounding-mode flags are set anywhere in the project. Re-verify with the
  byte-parity suite, not by assumption.
- **SmtSpec consumption details (pinned):** the exact chain is
  `McPlan::compute(module, &assumptions)` (associated fn on `McPlan`,
  `src/mc/plan.rs`) -> `smt_plan::build_smt_spec(module, &plan)` - the same two
  calls both backends make (`backends/{rust,c}/mc.rs` lines ~43-44 / ~46-47).
  `build_smt_spec` reads `plan.abstraction` (variants + fields) for the input
  classes. The audit exception for `ref_mc_smt.rs` therefore covers BOTH
  `crate::mc::plan` and `crate::mc::smt_plan`; everything else in plan.rs stays
  off-limits to the other ref_mc files.
- **Token budget:** task 1 is mechanical but wide (~60 inventory rows move); tasks
  3-4 are the deep ones. All comfortably inside per-task budgets; task 4 may need
  several z3 iteration cycles.
