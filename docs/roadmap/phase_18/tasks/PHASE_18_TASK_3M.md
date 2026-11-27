# Phase 18, Task 3M - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Close the last cheaply-fixable residual difference between smt-bounded and exact
mode: a selected assumption that is already violated at the initial configuration
(no input consumed at sample 0, so every `input_is`/`output_has` atom is statically
false) currently makes smt-bounded's base constraint UNSAT at sample 0 - z3 returns
a trivial model and the run reports a VACUOUS `PASS WITHIN BOUND` - while exact mode
rejects the run with `assumption '<label>' violated at initial configuration (sample
0)` (mc-ref exit 1; generated exact binaries panic in their constructor). After this
task, ALL THREE smt-bounded engines (in-process `ref_mc_smt` + both generated SMT
binaries) perform the identical check before any solver work and surface the
byte-identical error. Rationale: this is the one case where smt-bounded reports a
passing verdict that exact mode refutes; unifying it removes the last divergence a
user can hit without changing what bounded MC is (D5/D6 stay inherent).

## Context

### The divergence, precisely (verified against the code during research)
- **Exact mode** (`src/mc/ref_mc_exact.rs:376-392`): compiles every SELECTED
  assumption with ref_mc's OWN monitor compiler (`src/mc/ref_mc.rs::compile_formula`
  -> `Mon`), then steps each monitor ONCE at sample 0 with
  `Sample { input: None, outputs: [], to_state: q0, vars: init_vars }`. If any
  assumption monitor fails that step -> `Err("assumption '{label}' violated at
  initial configuration (sample 0)")` - the FIRST violated label in declaration
  order. Out-of-monitor-subset assumptions (compile failure) get a skip warning and
  are NOT enforced anywhere in exact mode - they can never trigger the sample-0
  error.
- **smt-bounded** (`src/mc/ref_mc_smt.rs:1079-1120`): asserts each selected cheap
  assumption as `NOT alive[i] OR formula(i)` for `i in 0..hi`. At sample 0 the
  input atoms are false by construction, so a positively input-constraining body
  (`assume force_tick : input_is(Tick)`, desugared to `always(...)` by task 3G)
  makes the base UNSAT at sample 0 -> vacuous PASS WITHIN BOUND. No check exists
  before or after solving that distinguishes this from a genuine pass.
- **Triggering shapes:** cheap-class assumptions whose sample-0 instance is false -
  an unconsumed input atom at depth 0, NOT under `next`. The documented workaround
  `always (next (input_is(...)))` is safe: `next` shifts the check to sample 1
  (verified: `nested_temporal` classifies it cheap; the monitor is pending at
  sample 0). Negative constraints (`always (!input_is(Bad))`) hold at sample 0.
- **mc-ref's error surface** (`src/cli/commands/mc_ref.rs:536-540`): `Err(e)` from
  `run_mc_ref` -> `eprintln!("error: {e}")` + exit 1, NO usage text on this path.
  The generated SMT binaries use `std::process::exit(N)` directly and already mirror
  mc-ref's 0/1/2 codes - so exit 1 + the identical line to stderr is the consistent
  choice (the generated EXACT binaries panic in their constructor; that surface is
  not mirrored - documented as a deliberate difference).

### The design (parity by construction - one shared computation, three consumers)
The sample-0 violation status of an assumption depends ONLY on the module (initial
state + init vars) and the formula - not on `--bound`, not on solver behavior, not
on selection beyond activeness. So:
1. Extract into `src/mc/ref_mc.rs` (where `compile_formula`/`mon_init`/`mon_step`
   already live as `pub(crate)`): `initial_configuration(module) ->
   Result<(usize, Vec<Val>), String>` (the q0 + init-vars computation currently
   inline in `ref_mc_exact.rs:327-366`, incl. the task-3L rule-4 masking) and
   `sample0_violated_assumptions(module, selected: &[&str]) -> Vec<String>`
   (compile each selected assumption with the ACTUAL monitor compiler - no
   re-derivation of temporal semantics; compile failure = skip, matching exact
   mode's warning+skip; step sample 0 with no input; return violated labels in
   declaration order).
2. `ref_mc_exact.rs`: replace its inline sample-0 loop with the helper call -
   behavior byte-identical (same message, first label).
3. `smt_plan.rs::build_smt_spec`: compute `assumption_sample0_violated: Vec<bool>`
   (parallel to `spec.assumptions`, selection = ALL assumption names) via the
   helper; add the field to `SmtSpec`.
4. `smt_emit.rs`: both emitters render the constant array (Rust:
   `const ASSUME_SAMPLE0_VIOLATED: &[bool] = ...`; C: the mirror). Both engine
   templates, AFTER `selected_assumes` is computed and BEFORE any z3 work, check
   selected ∩ violated -> `error: assumption '<label>' violated at initial
   configuration (sample 0)` to stderr + exit 1 (first label in declaration order).
5. `ref_mc_smt::run`: after `parse_assume_selection`, before `run_unrolled` -
   check selected ∩ `spec.assumption_sample0_violated` -> `Err` with the identical
   message; `mc_ref.rs:205` already propagates it via `?` as exit 1 + the
   `error: {e}` line.

### Verified blast radius (research, 2026-11-26)
Every `assume` line in `examples/valid/*`, `crates/examples/*/pass.rfs|full.rfs`,
and both SMT test suites is either a pending obligation (`always(bounded_eventually[N](...))`),
a negative constraint (`always(!input_is(Bad))` - true at sample 0), or the
`next`-shifted workaround form. NO spec uses a sample-0-false assumption, so no
existing pin moves: the check only fires on the currently-vacuous shape.

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3C. | Implement assumptions (D3) + on_error blocks (D4) in smt-bounded | provides the per-step assumption assertion machinery, the `--assumes` selection on both smt surfaces, and nominated this divergence for documentation (complete) |
| 3J. | Merged end-of-arc documentation pass | wrote MODEL_CHECKING.md section 10.5 documenting this divergence as residual - this task closes it and moves the doc entry (complete) |

### Dependent tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 10. | Phase close-out | reads the final docs state; no change to its own scope |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/ref_mc.rs` | change | new `pub(crate) initial_configuration` + `pub(crate) sample0_violated_assumptions` helpers (the shared sample-0 check) |
| `src/mc/ref_mc_exact.rs` | change | inline q0/init-vars computation + sample-0 loop replaced by the helper calls - behavior byte-identical |
| `src/mc/smt_plan.rs` | change | `SmtSpec` gains `assumption_sample0_violated: Vec<bool>`; `build_smt_spec` computes it via the helper (selection = all) |
| `src/mc/smt_emit.rs` | change | both emitters render the constant array; both engine templates gain the runtime check after `selected_assumes`, before z3 work |
| `src/mc/ref_mc_smt.rs` | change | `run` checks selected ∩ violated after `parse_assume_selection`, before `run_unrolled` -> identical `Err` message |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/MODEL_CHECKING.md` | change | section 10.5 deleted (the divergence is closed); the intro's closed list gains the task-3M entry; the summary table row becomes "none - unified (task 3M)"; the "What REMAINS" sentence drops the sample-0 item |
| `docs/user/FAQ.md` | change | section 2's smt-bounded bullet: the "One difference" sentence replaced with the unified-check statement |
| `docs/SEMANTICS.md` | change | §9.9 closing pointer: the sample-0 divergence dropped from the residual list (now closed by task 3M) |
| `docs/dev/mc_engine.md` | change | the SMT assumptions description gains one sentence: a selected assumption already violated at sample 0 is rejected before solving with exact mode's identical message (task 3M) |
| `docs/llm/SRC_STRUCTURE.md` | change | inventory rows for the two new helpers + the changed functions (same commit per PROTOCOL.md section 5) |
| `docs/llm/TEST_STRUCTURE.md` | change | inventory rows for the new tests (same commit) |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_smt_test.rs` | change | parity-pinned positive: a sample-0-violating assumption spec errors identically on ref + both generated backends (byte-identical line, exit 1); negative controls: the workaround form passes all three, `--assumes none` skips the check, an out-of-subset assumption never triggers it |
| `tests/smt_bounded_test.rs` | change | both generated backends run the violating spec: identical error line + exit 1 (C output gcc -Wall clean) |

## Planned actions

### 3M.1 - Extract the shared sample-0 check
Move the q0/init-vars computation out of `ref_mc_exact.rs` into
`src/mc/ref_mc.rs::initial_configuration` (keeping the task-3L rule-4 masking and
the var-referencing-init error message verbatim), and add
`src/mc/ref_mc.rs::sample0_violated_assumptions(module, selected)` that compiles
each selected assumption with the existing `compile_formula`, steps sample 0 with
no input via `mon_init`/`mon_step`, and returns the violated labels in declaration
order (compile failure = skip). Rewrite `ref_mc_exact.rs` to call both helpers -
its sample-0 error path stays byte-identical (same message, first label, same
position relative to the property check).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc.rs` | change | the two new `pub(crate)` helpers - one shared computation so all three engines agree by construction |
| `src/mc/ref_mc_exact.rs` | change | calls the helpers instead of inline code - zero behavior change, verified by the existing exact-mode pins |

### 3M.2 - Wire the check into the SMT model + all three engines
`build_smt_spec` computes `assumption_sample0_violated` (selection = all) and
stores it in `SmtSpec`; both emitters render the constant array; both generated
engine templates check selected ∩ violated after `selected_assumes` is computed
and before any z3 work, printing the byte-identical `error: assumption '<label>'
violated at initial configuration (sample 0)` line to stderr and exiting 1;
`ref_mc_smt::run` performs the same check in-process before `run_unrolled`,
returning the identical `Err` message (surfaced by `mc_ref.rs` as exit 1).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/smt_plan.rs` | change | `SmtSpec.assumption_sample0_violated` + the computation in `build_smt_spec` - the shared table all consumers read |
| `src/mc/smt_emit.rs` | change | constant-array rendering (Rust + C) + the runtime check in both engine templates - parity by construction |
| `src/mc/ref_mc_smt.rs` | change | the in-process pre-solve check - closes the divergence on the `rfs mc-ref --mode smt-bounded` surface |

### 3M.3 - Parity-pinned tests + negative controls
Add to `tests/ref_mc_smt_test.rs`: a spec constant with
`assume force_tick : input_is(Tick)` (plus a Tick-consuming transition) pinned to
the byte-identical error line + exit 1 on ref AND both generated backends (the
existing parity helper pattern); negative controls - the workaround form
`always (next (input_is(Tick)))` PASSes WITHIN BOUND on all three, `--assumes none`
on the violating spec skips the check (PASS WITHIN BOUND, no error), and an
out-of-subset assumption (`always (always[2] (input_is(Tick)))`, R3) never triggers
the sample-0 error even with `--full-assumptions` (exact mode skips it too). Add to
`tests/smt_bounded_test.rs`: both generated backends run the violating spec -
identical error line + exit 1, C output gcc -Wall clean.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change | the three-engine parity pin + the three negative controls - pins the NEW behavior in both directions so a regression is invisible no more |
| `tests/smt_bounded_test.rs` | change | the generated-backends leg of the same contract |

### 3M.4 - Move the doc entry from residual to closed
Delete MODEL_CHECKING.md subsection 10.5; add the task-3M closure to the intro's
closed list and flip the summary table row to "none - unified (task 3M)"; update
the "What REMAINS" sentence; replace FAQ.md section 2's "One difference" sentence;
drop the sample-0 item from SEMANTICS.md §9.9's residual pointer; add the one-line
capability note to docs/dev/mc_engine.md's SMT assumptions description.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/MODEL_CHECKING.md` | change | 10.5 deleted; intro closed list + summary table row + "What REMAINS" sentence updated - the section now lists only inherent differences (D5, D6) + declared omissions |
| `docs/user/FAQ.md` | change | the smt-bounded assumption bullet states the unified check instead of the difference |
| `docs/SEMANTICS.md` | change | §9.9 closing pointer no longer names the sample-0 divergence as residual |
| `docs/dev/mc_engine.md` | change | one sentence in the SMT assumptions description - the pre-solve rejection with exact mode's identical message |

### 3M.5 - Inventory sync + gates + full matrix
Add the inventory rows for the two new helpers (SRC) and the new tests (TEST) via
`rfs-dev inventory add`, update the changed rows' descriptions, then run
`rfs-dev sync-check` + `rfs-dev inventory audit` (must end EXACT), the doc gates
(`dev_docs_test` + `docs_md_test`), and the full matrix (`cargo build && cargo
test && cargo clippy --all-targets && cargo fmt --check`) with the standard
activation prefix (RUSTC_WRAPPER exported, PUPPETEER_CACHE_DIR set).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/SRC_STRUCTURE.md` | change | rows for `initial_configuration` + `sample0_violated_assumptions`; updated descriptions for the changed functions - same commit per PROTOCOL.md section 5 |
| `docs/llm/TEST_STRUCTURE.md` | change | rows for the new test fns - same commit |

## Task deliverables
- All three smt-bounded engines reject a selected assumption already violated at
  the initial configuration with the byte-identical `error: assumption '<label>'
  violated at initial configuration (sample 0)` line + exit 1, before any solver
  work - matching exact mode exactly.
- The shared helpers in `src/mc/ref_mc.rs` (`initial_configuration`,
  `sample0_violated_assumptions`) used by both exact and SMT paths - one
  computation, parity by construction.
- Parity-pinned tests + three negative controls (workaround form, `--assumes
  none`, out-of-subset assumption).
- MODEL_CHECKING.md section 10.5 closed (moved to the intro's closed list +
  summary table); FAQ + SEMANTICS §9.9 + dev/mc_engine.md updated.
- Inventories EXACT; all doc gates + full matrix green.

## Task exit criteria
- [ ] `rfs mc-ref --mode smt-bounded` on a spec with
      `assume force_tick : input_is(Tick)` exits 1 with the byte-identical line
      `error: assumption 'force_tick' violated at initial configuration (sample 0)`
      on stderr and EMPTY stdout - verified live before pinning.
- [ ] Both generated SMT binaries (rust + c) produce the identical line + exit 1
      on the same spec, parity-pinned in `tests/ref_mc_smt_test.rs` +
      `tests/smt_bounded_test.rs`; C output gcc -Wall clean.
- [ ] Negative controls pinned: the workaround form `always (next (input_is(Tick)))`
      PASSes WITHIN BOUND on all three engines; `--assumes none` on the violating
      spec skips the check (PASS WITHIN BOUND, no error); an out-of-subset
      assumption never triggers the sample-0 error even with `--full-assumptions`.
- [ ] Exact-mode behavior byte-identical after the helper extraction: all
      pre-existing exact-mode pins unchanged (the sample-0 error message, first-
      label ordering, and position relative to the property check preserved).
- [ ] No existing smt pin moves: every pre-existing PASS WITHIN BOUND / FAIL /
      unknown verdict on the corpus is unchanged (verified blast radius - no spec
      uses a sample-0-false assumption).
- [ ] MODEL_CHECKING.md section 10.5 deleted; the intro's closed list names task
      3M; the summary table row reads "none - unified (task 3M)"; FAQ + SEMANTICS
      §9.9 no longer describe the sample-0 divergence as residual.
- [ ] `rfs-dev sync-check` clean + `rfs-dev inventory audit` EXACT (zero per-file
      diffs).
- [ ] `cargo test --test dev_docs_test` + `cargo test --test docs_md_test` green.
- [ ] Full matrix (`cargo build && cargo test && cargo clippy --all-targets &&
      cargo fmt --check`) green - root count = 639 baseline + the new tests, no
      failures.

## Non-goals
- No change to D5 (completeness of PASS) or D6 (counterexample form) - inherent to
  bounded MC, stay documented as residual differences.
- No change to the declared-omission mechanism (`--full-properties` /
  `--full-assumptions`) - out-of-subset assumptions remain skipped by exact mode
  and asserted only with the flag by smt-bounded; that asymmetry is the documented
  10.4 kind, not this task's sample-0 detection.
- No mirroring of the generated EXACT binaries' constructor PANIC on the SMT
  surface - exit 1 + message (mc-ref's surface) is the deliberate choice, already
  consistent with the SMT binaries' 0/1/2 code mirroring.
- No new flags, no IR change, no formula-size change, no solver-call change.
- No training-set changes.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|
| (none yet) | - |

## Other information
- The check must run AFTER `--assumes` selection is known and BEFORE any z3 work:
  with `--assumes none` there are no active assumptions, so no error (matches exact
  mode's empty assume_mons).
- First violated label in declaration order - exact mode returns on the first one;
  the helper returns a Vec, consumers use element 0.
- The generated-runtime check prints to stderr via `eprintln!` (Rust) /
  `fprintf(stderr, ...)` (C) and exits 1 - byte-identical to mc-ref's
  `error: {e}` surface (no usage text on this path).
- If any pre-existing pin moves during execution, STOP: the verified blast radius
  says none should - a moving pin means a spec DOES use a sample-0-false
  assumption and needs its capture/README updated as a task correction.
