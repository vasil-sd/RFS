# Phase 16: MC-ref functionality unification with codegen MC

## Goal
Make the in-process reference model checker (`rfs mc-ref`, interpreted spec) support
EVERY mode the compiled codegen MC binaries support - `--nondet` exact search,
seeded random testing (`--mode fuzz`), and SMT-bounded checking (`--mode
smt-bounded`) - with a near-exact CLI surface, so that (a) each mode gains an
independent ground-truth engine cross-checked byte-for-byte against both backends,
and (b) users can run all three modes on their specifications without any
compilation cycle. The mc-ref source is refactored from one 4971-line file into a
common core plus one file per mode, and every piece of code that CAN be shared
between mc-ref and the codegen MCs is extracted into distinct shared sources.

```
rfs mc-ref <spec> --mode exact|fuzz|smt-bounded   ==   rfs codegen <backend> mc --mode ...
(one interpreted engine per mode, byte-parity-pinned against BOTH compiled backends)
```

## Scope
1. Refactor `src/mc/ref_mc.rs` (4971 lines) into a common core + entry points in
   `ref_mc.rs` and one file per mode: `ref_mc_exact.rs`, `ref_mc_fuzz.rs`,
   `ref_mc_smt.rs` (layout table + rationales in the analysis section below).
2. `--nondet` support in the ref exact engine (every enabled transition becomes a
   successor) + CLI flag + cross-backend differential tests.
3. Seeded random-testing mode (`--mode fuzz`) in ref-mc: the 3-phase input strategy
   with the EXACT RNG draw sequence of the generated runtimes (shared LCG),
   `--violate` forced-violation directives, per-run violation reports, fuzz summary
   line + CLI flags + cross-backend byte-parity tests.
4. SMT-bounded mode (`--mode smt-bounded`) in ref-mc: in-process z3 execution over
   the SHARED `SmtSpec` encoding model (one encoding, three consumers) + CLI flag +
   cross-backend byte-parity tests under both host and nix z3 versions.
5. Shared-source extraction: fuzz + SMT summary-line wire formats single-sourced in
   `mc_concepts.rs` (the exact-mode precedent, D-P6-10) and interpolated into both
   backends' templates; CLI surface unified across all MCs.
6. Docs: BUILD.md mc-ref CLI section, docs/dev MC-engine fragments, TASK_MAP /
   CHANGE_IMPACT rows, structure inventories (same-commit rule), PHASE_16_STATUS.md.

## Source subdivision analysis (planning outcome)

### Current state (measured 2026-11-17)

| File | Lines | Role |
|---|---|---|
| `src/mc/ref_mc.rs` | 4971 | EVERYTHING: common core (values/typing, evaluator, signals/inputs/outputs, monitor compiler+stepper, representative inputs, transition effects, liveness decomposition, formula/expression printing), exact engine (BFS + liveness SCC + minimal-violation labels), config/result types, text+JSON rendering, counterexample verification, ~670 lines of unit tests. Imports only `std`, `lang::ir`, and `mc_concepts` report types/helpers (D-P6-10); independence audit-tested in-file. |
| `src/mc/plan.rs` | 1130 | Shared exact-mode PLAN consumed by both codegen backends (input abstraction, config encoding, monitor layouts, traversal spec, liveness decomposition). NOT imported by ref_mc (independence). |
| `src/mc/smt_plan.rs` | 692 | Backend-agnostic SMT encoding MODEL: `Se`/`Sb`/`Sf` symbolic terms, `SmtSpec`, `build_smt_spec(module, plan)`, `se_type` typing. Shared by both backends today. |
| `src/mc/smt_emit.rs` | 1849 | Emitters: `SmtSpec` -> Rust source (`emit_rust_spec`) / C source (`emit_c_spec`). |
| `src/codegen/mc_concepts.rs` | 863 | Shared status taxonomy, JSON report model, `exact_summary_line`, `parse_assume_selection`, `ViolationSpec`, `McPhase`, `McConfig`, exact defaults. The D-P6-10 sharing surface. |
| `src/utils/rng.rs` | ~90 | The single LCG (`LCG_MULT`/`LCG_ADD` + `Rng`), interpolated into both generated fuzz runtimes - one RNG family by construction. |
| `src/codegen/backends/{rust,c}/mc.rs` | 2068 / 2902 | Per-backend templates: exact engine, `FUZZ_RUNTIME` (3-phase strategy + RNG inline), SMT engine (unrolling + queries inline), runtime flag parsing, `MAIN_DISPATCH` (`--mode` exact/fuzz switch). Fuzz + SMT summary lines rendered INLINE in each template (duplicated across backends, parity-pinned by tests). |

### Findings driving the layout

- **What is already shared and must stay shared:** report types + pure formatters
  (`mc_concepts`, D-P6-10), the LCG (`utils::rng`), `ViolationSpec::parse`,
  `McPhase::ALL` (strategy order), `McConfig` defaults, exact defaults.
- **What is duplicated and CAN be single-sourced:** the fuzz summary line
  (`runs=… bound=… completed=… unexpected=… assumption_stops=… property_violations=…`)
  and the SMT summary lines (`status=fail|pass-within-bound|unknown mode=smt-bounded
  bound=k [reason=… label=… step=…]`) exist as inline copies in BOTH backend
  templates. They become shared pure helpers + format constants in `mc_concepts.rs`
  (ref-mc calls them; templates interpolate the constants - generated code stays
  standalone, output byte-identical).
- **What must NOT be shared (independence):** ref_mc's evaluator, monitor stepper,
  representative-input computation, effect matching, BFS/liveness algorithms stay
  ref-mc-own for exact and fuzz modes - that independence is the ground-truth value.
  The generated backends derive from `plan.rs`; ref_mc does not.
- **The SMT exception (deliberate):** ref-mc's SMT mode consumes the shared
  `SmtSpec` model (`smt_plan::build_smt_spec`) instead of re-implementing the
  symbolic encoding - one encoding, three consumers (C emission, Rust emission,
  in-process execution). It keeps its OWN unrolling engine + query construction +
  z3 execution. Rationale: the user's unification goal, a far smaller effort than a
  fourth expression-conversion implementation, and the independent ground truth
  remains where it matters most (exact/nondet/fuzz). Trade-off recorded as D-P16-3.
- **Feature gating:** the z3 crate is an optional dependency (`smt` feature).
  Codegen of smt-bounded artifacts is NOT feature-gated (pure string rendering),
  but in-process EXECUTION requires the crate - so `ref_mc_smt.rs` is
  `#[cfg(feature = "smt")]` and `rfs mc-ref --mode smt-bounded` on a non-smt build
  exits with a clear diagnostic.
- **CLI unification:** the generated MC binaries expose, per mode: exact
  (`--max-states`, `--nondet`, `--timeout-sec`, `--threads`, `--fast`,
  `--counterexample-format`, `--assumes`), fuzz (`--bound`, `--runs`, `--seed`,
  `--assumes`, `--violate`, `--unexpected-prob`), smt-bounded (`--bound`). mc-ref
  adopts the same names/defaults/wire formats/exit codes; `--threads`/`--fast` are
  documented N/A (single-threaded in-process; `--fast` is parity-breaking by design).
- **Test conventions to reuse:** eligible examples = `examples/valid` minus the
  pinned EXCLUDED list (>=40 files); random specs via `specgen` with pinned seeds
  (`Bounds::small()`); generated binaries cached per test process; byte-parity
  assertions on summary lines + reports, status/exit-code agreement always,
  transition counts NOT asserted cross-engine (documented convention).

### New source layout (the deliverable table)

| File | Operation | Content | Rationale |
|---|---|---|---|
| `src/mc/ref_mc.rs` | change (shrinks to the common core) | values/typing, evaluator, signals/inputs/outputs, monitor compiler+stepper, representative inputs, transition effects, liveness decomposition, formula/expression printing, `RefConfig`/`RefResult`/`IoRec`/`StepRec`/`CeData`, text+JSON rendering, counterexample verification helpers; `run()` entry dispatching by mode; the independence audit (extended to read ALL `ref_mc_*.rs`) | User instruction: common code + entry points in `ref_mc.rs`. Everything used by >=2 modes or part of the public API stays here. Still large (~2500 lines) but it is the shared substrate with distinct section banners; no mode engine lives here. |
| `src/mc/ref_mc_exact.rs` | new (moved out of ref_mc.rs, then extended) | exact engine: `BNode`/`CfgKey`/keying, BFS run with BOTH successor strategies (deterministic first-match and `--nondet` all-enabled), path/trace reconstruction, liveness analysis (`liv_analyze`, product graph, witnesses, `liv_note`), `minimal_violation_labels`; exact-mode unit tests | One mode per file. Exact is the largest engine (~1500 lines incl. liveness); keeping it inline would leave a ~4000-line `ref_mc.rs` and defeat the refactor. `--nondet` lands as a strategy switch inside the existing BFS (the generated engines select `effects[0]` vs all effects - ref-mc mirrors with its own effect enumeration). |
| `src/mc/ref_mc_fuzz.rs` | new | seeded random-testing engine: run loop over bound/runs/seed, the 3-phase input strategy in `McPhase::ALL` order with the EXACT RNG draw sequence (per-run `chance(prob)` per spec; per-step forced-phase `chance(percent)` with draw-free trial steps; `chance(unexpected_prob)` + `pick_index(ALL_INPUTS)`; `pick_index(allowed)`) via `utils::rng::Rng`; `--violate` handling via shared `ViolationSpec`; per-run property-violation report + exit 1; fuzz summary line via `mc_concepts::fuzz_summary_line`; fuzz-mode unit tests | One mode per file. The draw-sequence parity contract is localized here and pinned by cross-backend byte-parity tests - a future strategy change cannot silently skew one engine. |
| `src/mc/ref_mc_smt.rs` | new, `#[cfg(feature = "smt")]` | SMT-bounded engine: in-process z3 execution over the shared model chain (`McPlan::compute` -> `smt_plan::build_smt_spec`) - step variables, base constraints, alive chain, safety queries (exists i<k: alive_i and not phi_i) and liveness queries (never + cycle), model reading, smt summary line via `mc_concepts::smt_summary_line`; smt-mode unit tests | One mode per file. Shares the plan + encoding MODEL with both codegen backends (D-P16-3) while keeping its own unrolling engine + query construction + execution. Feature-gated because the z3 crate is optional. Inherits the no-assumptions/no-on_error restriction of `build_smt_spec`. |
| `src/mc/mod.rs` | change | module declarations (+ `ref_mc_exact`, `ref_mc_fuzz`, `ref_mc_smt`); doc comment updated - the independence contract is now scoped per file (exact/fuzz: no intra-`mc/` imports; smt: `smt_plan` model only) | Mechanical; keeps the cluster's self-description true. |
| `src/codegen/mc_concepts.rs` | change | + `fuzz_summary_line()` pure helper + fuzz summary format constant; + `SmtStatus` enum (pass-within-bound / fail / unknown, exit codes 0/1/2) + `smt_summary_line()` pure helper + SMT summary format constants | The two remaining duplicated wire formats become single-source shared helpers - the exact-mode `exact_summary_line` precedent (D-P6-10). Unifies the report surface across ALL MCs (ref + both backends x 3 modes). |
| `src/codegen/backends/rust/mc.rs` | change | `FUZZ_RUNTIME` + SMT engine templates interpolate the summary-line format constants from `mc_concepts` (rendered copies stay standalone; emitted bytes unchanged) | Single source for the wire format without making generated code call library functions it cannot. Byte-identity proven by the existing parity pins (1683 generated tests + smt_bounded_test). |
| `src/codegen/backends/c/mc.rs` | change | same template interpolation for the C fuzz + SMT engines | Backend parity (PRINCIPLES section 1): both templates consume the same constants. |
| `src/cli/commands/mc_ref.rs` | change | `--mode exact\|fuzz\|smt-bounded` + per-mode flags (`--nondet`; `--bound`, `--runs`, `--seed`, `--violate`, `--unexpected-prob`; smt `--bound`); usage text; per-mode exit codes mirroring the generated binaries (exact 0/1/2, fuzz 0/1, smt 0/1/2); clear diagnostic for smt-bounded on a non-smt build | CLI unification: near-exact flag surface with the codegen MC modes - same names, same defaults, same wire formats, same exit codes. `--threads`/`--fast` documented N/A in the usage text. |
| `tests/ref_mc_diff_test.rs` | change | + `--nondet` comparison variant: ref-mc vs both generated engines on status/reason/label/step over eligible examples + random specs (states exact; transitions NOT asserted - documented convention) | First independent ground truth for nondet semantics - today only Rust-to-C parity exists and both derive from the same `plan.rs`. |
| `tests/ref_mc_fuzz_test.rs` | new | ref-mc fuzz vs BOTH generated fuzz binaries at fixed seed/bound/runs/violate/unexpected-prob over eligible examples + specgen random specs; byte-compares summary lines + per-run violation reports; asserts exit-code mirroring | Independent verification of the seeded strategy and the draw-sequence contract; directly serves the no-compile-cycle prototyping goal. |
| `tests/ref_mc_smt_test.rs` | new | ref-mc smt vs BOTH generated SMT binaries over the SMT spec family (fail/liveness/pass shapes) at multiple bounds, under host z3 5.3.1 AND nix z3 5.3.2; byte-compares summary lines + exit codes | Third execution target of the unified `SmtSpec` encoding catches emitter/executor divergence in either backend; cross-solver-version coverage per the Phase 15 task-2b lesson (parity must hold under ANY z3 version). |
| `BUILD.md` | change | mc-ref CLI surface documented: modes, per-mode flags, defaults, exit codes, the smt feature requirement, N/A flags | BUILD.md is the canonical build/test/run reference (START.md section 8 rule). |
| `docs/dev/*.md`, `docs/llm/TASK_MAP.md`, `docs/llm/CHANGE_IMPACT.md`, `docs/llm/SRC_STRUCTURE.md` | change | MC-engine fragments updated for the new file layout + three modes; TASK_MAP functionality-to-files rows; CHANGE_IMPACT change-type rows; SRC_STRUCTURE inventory rows (moved items change file column; new fns/types added) - same commits per PROTOCOL.md section 5 | Protocol obligations; `dev_docs_test` + `inventory audit` stay green. |

### Design decisions (recorded here, implemented per the plan)

- **D-P16-1 (file layout):** common core + entry points in `ref_mc.rs`; one file
  per mode (`ref_mc_exact.rs`, `ref_mc_fuzz.rs`, `ref_mc_smt.rs`). The exact engine
  gets its own file although it is the default mode - keeping it inline would leave
  a ~4000-line `ref_mc.rs` and defeat the refactor's purpose.
- **D-P16-2 (independence contract, scoped per file):** exact and fuzz engines stay
  algorithmically independent - own data structures, own evaluator/monitors/
  inputs/effects, no imports from `mc::plan`, `monitor`, `codegen::fsm`,
  `printer`, or `backends`. Permitted shared imports: `mc_concepts` (report types +
  pure formatters, D-P6-10 precedent) and `utils::rng` (one RNG family by
  construction). The SMT mode additionally imports the plan + encoding MODEL chain
  (`mc::plan::McPlan::compute` -> `smt_plan::build_smt_spec`, D-P16-3). The audit
  test enforces this per file over all `ref_mc_*.rs`.
- **D-P16-3 (SMT shares the plan + encoding model):** ref-mc's SMT mode consumes
  the full model chain `McPlan::compute` -> `smt_plan::build_smt_spec` instead of
  re-implementing planning + symbolic conversion - one encoding, three consumers.
  It inherits `build_smt_spec`'s restrictions verbatim (specs with assumptions or
  on_error blocks are rejected with the same error). Trade-off: a plan/encoding bug
  is now shared by all three (no independent encoding check for SMT); gain:
  unified functionality, one smaller engine to maintain, and the independent ground
  truth remains in exact/nondet/fuzz.
- **D-P16-4 (CLI unification contract):** mc-ref adopts the generated binaries'
  flag names, defaults, wire formats and exit codes per mode. Documented N/A:
  `--threads`/`--fast` (single-threaded in-process; `--fast` breaks parity by
  design). Smt-bounded requires an `--features smt` build (z3 crate) - a clear
  diagnostic otherwise, since codegen of smt artifacts is not feature-gated but
  in-process execution is.
- **D-P16-5 (wire formats single-sourced):** fuzz + SMT summary-line formats live
  once in `mc_concepts.rs` as pure helpers + format constants; ref-mc calls the
  helpers, both backend templates interpolate the constants. Generated output stays
  byte-identical (proven by the existing parity pins).

## Deliverables
- Refactored mc-ref cluster: `src/mc/ref_mc.rs` (common core + entry) +
  `src/mc/ref_mc_exact.rs` + `src/mc/ref_mc_fuzz.rs` + `src/mc/ref_mc_smt.rs`
  (feature-gated) + updated `src/mc/mod.rs`.
- `rfs mc-ref --mode exact|fuzz|smt-bounded` with the unified CLI surface
  (`--nondet`, `--bound`, `--runs`, `--seed`, `--violate`, `--unexpected-prob`,
  existing exact flags) and per-mode exit codes mirroring the generated binaries.
- Shared wire formats in `src/codegen/mc_concepts.rs` (`fuzz_summary_line`,
  `SmtStatus`, `smt_summary_line` + format constants) interpolated into both
  backends' templates (byte-identical output).
- Cross-backend differential suites: `--nondet` variant in
  `tests/ref_mc_diff_test.rs`; new `tests/ref_mc_fuzz_test.rs`; new
  `tests/ref_mc_smt_test.rs` (both z3 versions).
- Docs: BUILD.md mc-ref section, docs/dev MC-engine fragments, TASK_MAP /
  CHANGE_IMPACT rows, SRC_STRUCTURE inventory updates, PHASE_16_STATUS.md.

## Exit criteria
- [ ] `src/mc/ref_mc.rs` contains only the common core + entry points; each mode
      engine lives in its own `ref_mc_<mode>.rs`; the independence audit reads all
      `ref_mc_*.rs` and passes; no behavior change from the split alone (all
      canonical counts byte-unchanged: 557/685/559/1683, fmt clean, clippy at
      baseline, inventory EXACT, sync-check clean).
- [ ] `rfs mc-ref --nondet` agrees with BOTH generated engines on status/reason/
      label/step over the eligible examples + random specs (states exact; transitions
      not asserted); the diff suite's nondet variant is green.
- [ ] `rfs mc-ref --mode fuzz` produces byte-identical summary lines and per-run
      violation reports to BOTH generated fuzz binaries at fixed seeds over the
      eligible examples + random specs; exit codes mirror (0 no violation / 1
      violation found); `ref_mc_fuzz_test` green.
- [ ] `rfs mc-ref --mode smt-bounded` agrees with BOTH generated SMT binaries on
      status/label/step/bound over the SMT spec family at multiple bounds, under
      host z3 5.3.1 AND nix z3 5.3.2; on a non-smt build it exits with a clear
      diagnostic; `ref_mc_smt_test` green.
- [ ] Fuzz + SMT summary-line formats are single-sourced in `mc_concepts.rs` and
      interpolated into both backends' templates; generated artifact bytes unchanged
      (1683/1683 generated tests, smt_bounded_test parity pins unmodified).
- [ ] CLI surface documented in BUILD.md + the mc-ref usage text; per-mode exit
      codes mirror the generated binaries.
- [ ] Full canonical sweep green (557/685/559/1683 + new tests, fmt clean, clippy at
      baseline) and nix doCheck build still 42/42 with the dev-shell smt suite green.
- [ ] `PHASE_16_STATUS.md` written per FORMATTING.md section 4; `TASKS.md` entry
      appended; `CURRENT_STATE.md` phase-16 row updated.

## Non-goals
- No RFS language, IR, or semantics changes - this phase is engine + CLI + docs only.
- No deep refactor of `smt_emit.rs`'s emission architecture (no shared "engine
  model" layer between the emitters and the in-process executor) - recorded as a
  possible later phase; D-P16-3's model-level sharing is the stopping point.
- No `--threads`/`--fast` in mc-ref (single-threaded in-process; `--fast` is
  parity-breaking by design).
- No libFuzzer support in mc-ref - coverage-guided fuzzing of generated code is
  compile-based by nature (`rfs test --suite fuzz` stays as-is).
- No new external dependencies (z3 is already the optional `smt` dependency;
  `utils::rng` is internal).
- No changes to the generated artifacts' bytes beyond the template constant
  interpolation, which must prove byte-identical output.
