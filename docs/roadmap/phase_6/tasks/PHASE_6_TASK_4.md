# Phase 6, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make `PHASE_6.md`'s "Soundness testing" section executable: five differential
suites that compare the engines against independent ground truth (task 3's
`ref_mc`) and against each other on identical inputs - ref-MC vs generated MC,
cross-backend forced-violation parity, production-FSM random-input parity,
monitors vs hand-written reference monitors, and PSL translation checks.

## Context

- **Existing infrastructure to build on (do not duplicate)**:
  `tests/backend_parity_test.rs` (exact-mode MC parity over ALL property-bearing
  examples - field-identical results + byte-identical JSON reports; generated
  test-executable agreement incl. a byte-identical random-trace line; dynamic
  enumeration via `rfs::collect_rfs_files`), the LCG family from task 3,
  the scenario-parity pattern in `tests/example_projects_test.rs` (C/Rust
  byte-identical data lines driven from identical input scripts).
- **Comparison semantics for 4.1** (important - different algorithms, so be
  precise about what MUST agree):
  - Status agreement is order-INDEPENDENT and must always hold: ref_mc says
    PassComplete iff the generated engine says PassComplete; Fail iff Fail;
    deadlock iff deadlock.
  - When failing: compare the ORDER-INDEPENDENT ground truth first -
    `ref_mc::minimal_violation_labels` (sorted set at minimal depth) vs the
    generated engine's reported label. If the generated engine's label is in the
    set AND its counterexample replays validly through ref_mc's independent
    evaluator (cross-validation: feed the engine's CE trace to ref_mc, assert it
    violates the claimed label at the claimed step), the check passes. Label+
    step EQUALITY is additionally asserted for specs with a UNIQUE minimal-depth
    violation (the common case) - measure which eligible specs are ambiguous
    (multiple labels tie at minimal depth) and pin that list; ambiguous specs get
    set-membership + CE-replay checks only.
  - Metrics: `states` must agree exactly (same reachable set); `max_depth`
    agrees when both use level-synchronous BFS (ref_mc does); `transitions` may
    differ in traversal accounting - do NOT assert it cross-engine (it IS
    asserted cross-BACKEND by the existing parity suite).
- **4.2 forced violations**: generated binaries' fuzz mode is seeded and
  deterministic for a fixed seed + config; the existing liveness-parity section
  of `backend_parity_test.rs` already pins "same seed => exit code + full stdout
  identical across backends" - extend that exact pattern with `--violate`
  directives. Generated-binary fuzz flags: `--mode fuzz --bound N --runs N
  --seed S --assumes ... --violate label:prob:percent%` (comma-separated list).
- **4.3 production parity drivers**: Rust driver = a template file whose body is
  `include!("<artifact>")` + a `fn main()` reading an input script from stdin
  (one input per line: variant name + field values, e.g. `Go` / `Req 5`) and
  printing one line per step (state, outputs, monitor verdicts, assumption
  events). The generated production artifact starts with a comment + inner
  attributes - including it at the TOP of the driver file works for rustc
  single-file compilation; if inner-attribute placement fails, strip them in the
  test exactly like `rfs-build` does (record which path was needed). C driver =
  `#include "<artifact>.c"` + a `main()` with the same stdin/stdout contract;
  compile with `gcc -Wall`. Input scripts: seeded LCG over representative inputs
  (same RNG lineage as task 3), N steps (e.g. 200) INCLUDING some unexpected-
  event injections to exercise `on_error` paths; both drivers must consume the
  identical script and produce byte-identical stdout.
- **4.4 monitor references**: hand-written reference monitors are INDEPENDENT
  Rust AND C code written directly in the test suite for the properties of specs
  43/44/45 (the benchmark set at this point in the plan - task 6 adds 47-49
  later; do not wait for them). They must NOT reuse `src/monitor.rs` or any
generated code. Trace source: drive the generated production FSM (Rust side,
  monitors on, debug build) with a seeded LCG input script to produce the trace;
  feed the SAME trace to (a) the generated monitor (in the production artifact),
  (b) the hand-written Rust reference, (c) the hand-written C reference (C side:
  drive the C production FSM via a 4.3-style driver that also prints monitor
  verdicts). Compare verdicts at EVERY sample + first-violation label and index.
- **4.5 PSL translation**: the independent evaluator is a direct recursive window
evaluation per `docs/SEMANTICS.md` sections 4-7 (a different algorithm from
`monitor.rs`'s automaton construction), implemented as a module inside
`tests/psl_translation_test.rs`. Cross-check: for ALL property-bearing examples
(dynamic enumeration like the parity suite) x random traces (4.3-style drivers),
the generated monitor's verdict at every sample must equal the reference
evaluator's verdict. External hook: `RFS_PSL_REF_TOOL` env var - if set to an
executable, the test invokes it with a documented interface (trace JSON +
property on stdin, verdict on stdout) and compares; availability-gated, skipped
with a note when unset (no external PSL checker exists in this environment -
z3 is SMT-only).
- **Bug-found protocol**: these suites are EXPECTED to be able to find real bugs.
  If a mismatch appears: do NOT weaken the test. Record it in `PHASE_6_PLAN.md`
  "Plan corrections" (type `new`) with a fix task, implement the fix there, and
  re-run this suite. The suites only go green when engines genuinely agree.
- **Runtime budget**: keep each new suite under ~2 minutes in `cargo test` (the
  existing parity/benchmark suites set the bar). Pin eligible lists / spec counts
  so the budgets are stable.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | Stability and versioning (v1.0) | artifacts carry the final headers; re-pins already done there |
| 3. | Test infrastructure + user-facing reference MC & spec generator | `ref_mc` ground truth + unified reports + `specgen` random specs + the LCG lineage |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/ref_mc_diff_test.rs` | new | 4.1: ref-MC vs generated-MC differential (both backends) + no-false-PASS negative control |
| `tests/backend_parity_test.rs` | change | 4.2: forced-violation parity section (seeded fuzz mode + `--violate`) |
| `tests/prod_parity_test.rs` | new | 4.3: production-FSM cross-backend random-input parity with driver wrappers |
| `tests/monitor_ref_test.rs` | new | 4.4: hand-written reference monitors (Rust + C) vs generated monitors |
| `tests/psl_translation_test.rs` | new | 4.5: independent trace evaluator + external-tool hook |

## Planned actions

### Action 1: ref-MC vs generated-MC differential (subtask 4.1)
New `tests/ref_mc_diff_test.rs`. Eligible set: measure `ref_mc` runtimes over
ALL valid examples with a generous `--max-states`; pin the eligible list +
threshold in the test file (expect most examples to qualify; exclude only
`44_token_ring`-scale specs if too slow). For each eligible example + K=20
random specs from `specgen` (small bounds, pinned seed): run `rfs mc-ref
--counterexample-format json` AND both generated MC binaries (exact mode,
`--counterexample-format json`); assert per the comparison semantics in Context
(status agreement; minimal-label set membership; CE replay validity; label+step
equality for unique-minimal specs; exact `states` agreement). Negative control:
tiny `--max-states` on a large spec => `UnknownStateLimit` in BOTH backends and
in ref_mc, NEVER `PassComplete` (the "no false PASS" invariant).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_diff_test.rs` | new | the suite above with pinned eligible list + random-spec seeds |

### Action 2: cross-backend forced-violation parity (subtask 4.2)
Extend `tests/backend_parity_test.rs`: for every assumption-bearing example
(dynamic enumeration, same as the existing suite), for each assumption label:
run BOTH generated binaries in fuzz mode with a pinned seed (e.g. 777),
`--runs 5 --bound 10`, `--violate <label>:1.0:100%`; assert identical exit code
+ full stdout (text mode) across backends - the existing liveness-parity
pattern. Also one combined run with all labels in a single `--violate` list.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/backend_parity_test.rs` | change | new forced-violation parity section |

### Action 3: production-FSM cross-backend random-input parity (subtask 4.3)
New `tests/prod_parity_test.rs`: for EVERY valid example - generate production
for both backends (`--with-property-monitors`), build the Rust driver
(include! template) + C driver (#include + main template, per Context), drive
both with identical seeded LCG input scripts (200 steps incl. unexpected-event
injections), assert byte-identical stdout. Pin the seed + step count; keep the
suite within budget (compile once per example into a temp dir, reuse across the
script set).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/prod_parity_test.rs` | new | drivers + parity suite |

### Action 4: monitors vs hand-written reference monitors (subtask 4.4)
New `tests/monitor_ref_test.rs`: for specs 43/44/45 - write the hand-written
reference monitors (independent Rust code in the test file; independent C code
compiled per spec), drive with seeded random traces (reuse action 3's driver
machinery where practical), compare verdicts at every sample + first-violation
label/index between generated monitor and both references.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/monitor_ref_test.rs` | new | reference monitors (Rust + C) + comparison suite |

### Action 5: PSL translation checks (subtask 4.5)
New `tests/psl_translation_test.rs`: the independent recursive window evaluator
(per SEMANTICS.md), cross-checked against generated monitor verdicts over random
traces for ALL property-bearing examples (dynamic enumeration); the
`RFS_PSL_REF_TOOL` availability-gated hook with its documented interface in the
test module docs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/psl_translation_test.rs` | new | evaluator + suite + external hook |

## Task deliverables
- Five green differential suites: `ref_mc_diff_test`, extended
  `backend_parity_test` (forced violations), `prod_parity_test`,
  `monitor_ref_test`, `psl_translation_test`.
- Pinned eligible lists / seeds / step counts keeping every suite within budget.
- Any found engine mismatches recorded as plan corrections with fix tasks
  (and fixed before this task's exit criteria are claimed).

## Task exit criteria
- [x] 4.1: status agreement + CE-replay validity on all eligible examples + 20 random specs; label+step equality on unique-minimal specs; `states` exact; no-false-PASS negative control green.
- [x] 4.2: forced-violation runs byte-identical (exit code + stdout) across backends for every assumption-bearing example x label + the combined run.
- [x] 4.3: production drivers byte-identical on identical input scripts for EVERY valid example, incl. unexpected-event injections.
- [x] 4.4: generated monitor verdicts == hand-written Rust reference == hand-written C reference at every sample for specs 43/44/45 random traces.
- [x] 4.5: independent evaluator agrees with generated monitors on all property-bearing examples x random traces; external hook documented + skipped cleanly when unset.
- [x] Each suite < ~2 min; full `cargo test` green; clippy/fmt clean.

## Non-goals
- No SMT-mode differential coverage (feature-gated, outside the v1 guarantee); no performance claims or timing assertions; no changes to engine code in THIS task (fixes go through plan corrections + their own tasks); no new benchmark specs (task 6).

## Other information
- If action 3's Rust include! driver hits inner-attribute issues, use the
  rfs-build stripping pattern and record which path was needed.
- The C reference monitors in 4.4 are plain C compiled with `gcc -Wall` per spec
  into a temp dir (same toolchain assumptions as the existing suites: gcc at
  /usr/bin/gcc, rustc from the project toolchain).
