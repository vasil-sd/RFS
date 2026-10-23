# Phase 6, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Write `docs/SEMANTICS.md`: the precise, normative formal semantics document
required by `PHASE_6.md` (automaton model, transition relation, deterministic vs
nondeterministic modes, property sampling, PSL operator semantics, strong/weak
liveness, fairness, assumptions, error/deadlock semantics, abstraction rules).
It is "essential for user trust" and becomes the reference that tasks 3/4/5
implement and test against: the brute-force ref MC (task 3.2) and the independent
evaluators (tasks 4.4/4.5) are coded directly from this document, so any
doc/implementation mismatch surfaces as a test failure later.

## Context

- This is a DOCUMENTS task: no source changes unless the cross-check pass
  (action 4) finds a real mismatch - in that case record it in "Plan corrections"
  of `PHASE_6_PLAN.md` and fix the document here (code fixes only via a
  correction-added task, never silently).
- Sources to read before writing (targeted chunks, per START.md section 5):
  - `DESIGN.md` sections 2-4 (architecture, core semantics, MC constraints),
    section 20 (Phase 2 decision tables D1-D10: priorities, on_error, bit-widths),
    section 21 (property/monitor decisions), section 22 (D-P4-1..14: exact engine,
    liveness, statuses), section 23 (D-P5-1..14).
  - `KB.md` section 2.2 (core semantics), section 2.6 (property language decisions),
    section 3 (RFS v0 syntax reference - the surface the semantics applies to).
  - Implementation ground truth: `src/mc_plan.rs` (representative inputs,
    configuration packing, traversal order, liveness obligations), `src/monitor.rs`
    (MonitorPlan + the reference interpreter - sample definition, operator
    evaluation), `src/codegen_mc.rs` (status taxonomy, McPhase strategy,
    forced-violation format), `src/backends/RUST.md` + `C.md` (value
    representation, wrap semantics).
- Key semantic facts to document (VERIFY each against the implementation before
  writing it down):
  - Sample sequence: sample 0 = initial configuration with NO input/outputs
    (`input_is`/`output_has` false there); later samples carry post-step values.
  - Simultaneous variable updates: all assignment RHS evaluated on OLD values,
    then applied together (no sequential dependency).
  - Guarded transitions + explicit priorities resolve overlaps in PRODUCTION mode
    (deterministic: at most one enabled transition per input); MC mode explores
    ALL enabled transitions (nondeterministic choice).
  - `on_error` fallback vs `Result`-based unexpected-event handling.
  - Representative inputs per event variant (the exact rule lives in
    `src/mc_plan.rs` - document it precisely; this is the core abstraction rule).
  - Deadlock: configuration with no enabled transition for ANY representative
    input (verify the exact definition incl. `Tick` handling in mc_plan).
  - PSL subset: boolean atoms (`state_is`, `input_is`, `output_has`, expr atoms),
    `next`, `always`, `always[N]`, `bounded_eventually[N]`, Phase 4's
    `eventually`/`until`; strong vs weak liveness; `--nondet` fairness mode.
  - Assumptions: same property language; `AssumptionHandler` policies
    (continue/stop/panic) and how assumption handling interacts with the search.
  - Status taxonomy + LABELING RULES: `PassComplete` only on full exploration;
    `UnknownStateLimit` never a completeness claim (the "no false PASS" invariant).
  - Bounded/fuzz-mode sequence strategy: phase order forced-assumption-violation
    -> unexpected injection -> allowed random; seed determinism (same seed +
    config => identical run).
  - Bit-width semantics: Rust wide-intermediate wrapping vs C native unsigned
    wrap (document as one observable behavior both backends share).
- Style: precise but readable; configurations as tuples `(state, vars,
  monitor_bits)`; a short "notation" preamble; every operator gets a formal-ish
  definition over sample sequences; where the implementation is ambiguous, document
  the IMPLEMENTED behavior and flag it in a footnote. No images (project rule).
- Section skeleton (use these `##` headings, in order):
  1. Notation and scope
  2. Automaton model (states, variables, events/outputs, initial configuration)
  3. Transition relation (guards, priorities, simultaneous updates, on_error,
     deterministic vs nondeterministic modes, Tick/timers)
  4. Property sampling (sample sequence definition)
  5. PSL operator semantics (per-operator definitions + bounds)
  6. Liveness: strong vs weak, fairness
  7. Assumptions and handler policies
  8. Error and deadlock semantics
  9. Model checking: configurations, exact exploration, abstraction rules,
     bounded/fuzz strategy, status taxonomy and labeling rules, SMT note
  10. Cross-references (DESIGN/KB sections, user docs from task 7)

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | Test infrastructure + user-facing reference MC & spec generator | `src/ref_mc.rs` implements this document directly (D-P6-5) - it is the executable check that the doc matches the implementation |
| 4. | Differential soundness suites | the independent evaluators (4.4/4.5) are coded from sections 4-7 of this document |
| 7. | External user documentation set | `docs/user/PROPERTIES.md` and `MODEL_CHECKING.md` derive their semantics from this document |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/SEMANTICS.md` | new | the formal semantics document (all 10 sections above) |
| `KB.md` | change | section 1 state note + pointer to SEMANTICS.md as the normative semantics source |
| `START.md` | change | documentation map: add a row for `docs/SEMANTICS.md` |

## Planned actions

### Action 1: automaton model + transition relation (subtask 1.1)
Write sections 1-3: notation, automaton model, transition relation. Cover states/
enums, variables (`bool`/enum/`bit<N>` with wrap semantics), events/outputs with
finite payload fields, guarded transitions, explicit priorities, simultaneous
updates (old-value RHS), initial configuration, `on_error`, deterministic vs
nondeterministic modes, `Tick` + timer variables. Verify every claim against
`DESIGN.md` sections 20-21 and the parser/validator (`src/parser.rs`,
`src/validator.rs`) for the exact accepted surface.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | new | sections 1-3 |

### Action 2: property layer semantics (subtask 1.2)
Write sections 4-8: sampling, per-operator PSL semantics (each operator: syntax,
meaning over sample sequences, bound behavior at the end of a finite trace),
strong vs weak liveness, fairness (`--nondet`), assumptions + handler policies,
error/deadlock semantics. Verify against `src/monitor.rs` (the reference
interpreter is the executable spec of operator evaluation) and `DESIGN.md`
sections 21-22.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | sections 4-8 |

### Action 3: model-checking layer semantics (subtask 1.3)
Write section 9: configuration encoding, exact reachable-state exploration,
abstraction rules (representative inputs - the precise rule from `src/mc_plan.rs`,
assumption handling during search, monitor-bit coupling), bounded/fuzz strategy
(phase order + seed determinism), status taxonomy + labeling rules (incl. the
"no false PASS" invariant), SMT mode as feature-gated extension outside the v1
guarantee. Verify against `src/mc_plan.rs`, `src/codegen_mc.rs`, and the Phase 4
pins in `tests/backend_parity_test.rs` / `tests/benchmark_mc_test.rs`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | section 9 |

### Action 4: implementation cross-check pass (subtask 1.4)
Re-read the finished document claim by claim against the implementation sources
listed in Context. Fix any mismatch in the document; if a claim reveals an actual
implementation bug or ambiguity, STOP and record it in `PHASE_6_PLAN.md` "Plan
corrections" (do not fix code in this task). Add cross-references: `DESIGN.md`
sections 2-4 get a pointer to SEMANTICS.md as the normative semantics source;
`KB.md` section 1 gets the state note + pointer; `START.md` doc map gets the new
row.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | corrections from the cross-check pass |
| `DESIGN.md` | change | pointer lines in sections 2-4 (one line each, no content moves) |
| `KB.md` | change | section 1 state note + pointer |
| `START.md` | change | doc map row for `docs/SEMANTICS.md` |

## Task deliverables
- `docs/SEMANTICS.md` covering ALL ten topics of `PHASE_6.md`'s "Formal semantics
document" section, with the 10-section skeleton above.
- Cross-reference pointers in `DESIGN.md`, `KB.md`, `START.md`.

## Task exit criteria
- [ ] All 10 required topics present as dedicated sections (checklist from PHASE_6.md: automaton model; transition relation; deterministic vs nondeterministic modes; property sampling; PSL operator semantics; strong/weak liveness; fairness; assumptions; error/deadlock semantics; abstraction rules).
- [ ] Every semantic claim verified against the implementation sources listed in Context (the cross-check pass is done and recorded in "Other information" below with the list of checked sources).
- [ ] The representative-input rule, deadlock definition, and status labeling rules are stated precisely enough that `src/ref_mc.rs` (task 3.2) can be implemented from the document alone.
- [ ] Cross-references added (`DESIGN.md` sections 2-4, `KB.md` section 1, `START.md` doc map).
- [ ] No source files changed (docs-only task); `cargo test` still green (sanity run).

## Non-goals
- No new language features or syntax changes (v1.0 surface is frozen by this phase's task 2, which runs after this one - the document describes the CURRENT implemented surface).
- No code fixes for mismatches found (those go through plan corrections + separate tasks).
- Not the external-user reference (that is task 7's `docs/user/` set, which derives from this document).

## Other information
- Record here after action 4: the list of implementation sources actually checked
  and any ambiguities found (with how they were resolved in the document).
- Keep the document self-contained: a reader should not need to open source files
  to understand the semantics (source pointers are for verification, not reading).

### Cross-check pass results (action 4)

**Implementation sources actually checked (claim-by-claim):**

| Source | What it pinned in the document |
|---|---|
| `DESIGN.md` sections 2-4, 11, 12, 20, 21, 22, 23 | automaton model (M tuple), MC constraints, sampling model, exact engine + liveness algorithm + corrected weak-until equation, PSL subset, D1-D10 (priorities/on_error/version/C types), D-P3-1..6 (R1-R4, deadline semantics), D-P4-1..14 (statuses, fairness, abstraction), D-P5-10 (JSON report) |
| `KB.md` sections 2.2, 2.6, 3 | core-semantic rules, property-language precedence corner cases, the full RFS v0 syntax surface (the document's scope) |
| `src/mc_plan.rs` | representative-input rule (referenced-field analysis incl. binding-vs-use and `input_is` constraining patterns), configuration bit encoding (state/vars/monitor bits, sticky flags excluded), canonical value domains + successor order, liveness skeleton decomposition |
| `src/monitor.rs` | MonitorPlan node model, per-operator compile rules (next range-shift, always[N] range, bounded_eventually obligation trigger/anchor incl. top-level `{0}` one-shot and implication antecedent-as-trigger), R1-R4 enforcement, reference-interpreter stepping algorithm (pending model, deadline confirmation at i+N-1, per-sample And/Or composition) |
| `src/codegen_mc.rs` | status taxonomy + wire/human names + exit codes, exact summary line format, fuzz phase order + LCG constants + forced-violation wire format, JSON report key order, exact-mode defaults |
| `src/backends/rust_fsm.rs` / `rust_mc.rs` | production step/commit (simultaneous updates via temps, on_error fallback arm with `transition_id = usize::MAX`, panic-on-sample-0-assumption, sticky property violation), expression lowering (u128 intermediates, guarded shifts `amount >= width -> 0`), exact engine BFS (visited-at-enqueue, canonical commit, deadlock = no surviving successor, counters) |
| `src/backends/c_fsm.rs` / `c_mc.rs` + RUST.md/C.md | C parity (tagged unions, tmp-var simultaneous updates, handler policies, monitor rendering), C expression lowering (the source of the finding below) |
| `src/validator.rs` / `src/parser.rs` | accepted surface: priority/duplicate diagnostics + exact warning texts, on_error validation, version-header parsing (any quoted string), init typing against full module env, pattern forms |
| `examples/valid/*.rfs` (scan) | confirmed no example uses `*`; all variable shift amounts are `bit<2>` (the finding is latent) |

**Ambiguities / problems found and how the document resolved them:**

1. **C expression lowering - undefined behavior (REAL implementation divergence).**
   Generated C renders bitvector arithmetic/shifts as `((l) op (r)) & mask`
   relying on integer promotion. A shift by a VARIABLE amount >= the promoted
   LHS width (>= 32 for bit<8..32>, >= 64 for bit<64>) is UB in C while Rust
   defines `amount >= width -> 0`; multiplying two bit<16> values with product
   > INT_MAX is signed-overflow UB. Verified on a scratch spec (`set w = w <<
   k;`, `k: bit<8>`): Rust emits a guarded form, C emits the promotion-based
   form (on x86-64 gcc the shift count wraps mod 32, so `w=5, k=32` gives C=5
   vs Rust=0). Latent - no current example/test exercises it. The document
   states the RFS/Rust semantics as normative and flags the C divergence in
   footnote c-ub (section 2.2). Per action 4 the code was NOT fixed here; a new
   plan task **2a** (C lowering fix + example 50 + targeted parity test) was
   added to `PHASE_6_PLAN.md` "Plan corrections", and task 6.1's count re-pin
   was corrected 49/13 -> 50/13. See `PHASE_6_TASK_2A.md`.
2. **Variable initializer evaluation order (ambiguity).** The validator types
   each `init` expression with ALL module variables in scope (declaration order
   ignored) and codegen renders initializers as a struct literal over
   constants, so an initializer referencing another variable is ill-formed in
   effect. Documented the implemented typing rule and flagged it in footnote
   init-order (section 2.4); `v0` is treated as well-defined for constant
   initializers.
3. **Out-of-subset assumptions are not enforced (gap).** An assumption outside
   the monitor subset (R1-R4, or containing eventually/until) generates no
   monitor and is skipped with a warning - it is NOT enforced by production
   code or the model checker. Documented in section 7.6 + footnote assume-skip.
4. **`bounded_eventually` top-level implication is one-shot (precision).** The
   anchor range for a top-level `p -> be[N] q` (no enclosing temporal operator)
   is `{0}` only - `p(0)` starts the countdown and later `p(i)` never re-triggers.
   Made explicit in section 5.5 (a naive reading of "enclosing range" would
   have been wrong).
5. **Deadlock vs missing-input (precision).** A state merely missing a
   transition for some input does NOT deadlock (that path is an unexpected
   termination); deadlock requires NO input to have a surviving successor.
   Corrected in section 8.4.
6. **`output_has` is any-of (precision).** Confirmed against the generated
   signal rendering (`s.outputs.iter().any(...)`): true iff AT LEAST ONE emitted
   output matches - documented in section 5.1.
