# Phase 6, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Build the independent test/reference infrastructure - a seeded random spec
generator with incremental mutation (`src/specgen.rs`, D-P6-11) and a brute-force
reference model checker with unified report rendering (`src/ref_mc.rs`, D-P6-5)
- and EXPOSE both to the user as two new CLI commands: `rfs mc-ref` (run the
reference MC in-process, no compilation - rapid prototyping in shell/REPL,
D-P6-10) and `rfs gen-spec` (write a random or base-mutated spec to a file).

## Context

- **Precedent**: the reference interpreter in `src/monitor.rs` is a public lib
  module used by integration tests - `specgen`/`ref_mc` follow the same pattern
  (public modules of the `rfs` lib, documented as test/reference infrastructure).
- **Determinism family**: reuse the LCG algorithm family of the codegen_test
  random-trace class (read `src/codegen_test.rs` for the exact constants and the
  byte-pinned sequence) so all seeded machinery in the project shares one RNG
  lineage. Pin new seed sequences in unit tests.
- **Independence (D-P6-5)**: `ref_mc` must share NO search/evaluation code with
  the generated engines: compute representative inputs DIRECTLY from the IR
  (read `src/mc_plan.rs` to learn the rule, then reimplement it independently),
  use plain data structures (Vec frontier + HashMap config keys, no bit-packing),
  and evaluate properties with its OWN small evaluator over recorded traces
  (deliberately a different algorithm from `src/monitor.rs`). Reusing
  `src/codegen_mc.rs` DATA TYPES for the report (status taxonomy, JSON model) is
  REQUIRED by D-P6-10 - independence is about algorithms, not report formatting.
- **Unified reports (D-P6-10)**: `ref_mc` renders the SAME text report format as
  the generated exact-mode binaries (read the text-report rendering in
  `src/backends/rust_mc.rs`: summary line, violation lines, trace printing) and
  the SAME JSON document model (`McJsonReport` + builder in `src/codegen_mc.rs`).
  Where deterministic, output is byte-comparable with the generated binaries -
  task 4.1 relies on this.
- **Generated-binary flag reference** (for unification): exact mode accepts
  `--max-states`, `--nondet`, `--timeout-sec`, `--threads`, `--fast`,
  `--counterexample-format json|text`, `--assumes`; exit codes 0 pass-complete /
  1 fail / 2 unknown-state-limit. Fuzz mode (`--mode fuzz`) accepts
  `--bound/--runs/--seed/--assumes/--violate/--unexpected-prob`. `rfs mc-ref`
  mirrors the EXACT-mode surface where applicable: `--max-states N`,
  `--timeout-sec N`, `--assumes all|none|labels` (same wire format as
  `parse_assume_selection` in `src/codegen_mc.rs`),
  `--counterexample-format json|text` (default text), `--log-level`. NOT
  applicable (documented in the usage text): `--seed`/`--steps`/`--violate`/
  unexpected-injection (fuzz-mode concepts; exact search is seedless, D-P4-7) and
  `--threads`/`--fast` (generated-engine tuning; ref_mc is single-threaded).
  `--nondet`: ref_mc explores all enabled transitions by construction - document
  that its liveness semantics match the generated engine's DEFAULT exact mode;
  differential tests (task 4.1) compare against default-mode runs.
- **CLI dispatch**: `src/main.rs` match arms (~lines 50-55) + the usage text
  block at the top of main.rs; cmd functions return `ExitCode`. New commands:
  `mc-ref`, `gen-spec` (no collisions with existing names or cargo aliases).
- **Mutation design (D-P6-11)**: operations = add/remove/rename states, events,
  outputs, transitions, properties/assumptions; tweak guards, payload field
  widths, priorities, `on_error`. Validity at EVERY step: after each candidate
  mutation run the validator; on failure roll back and redraw from the RNG
  (bounded attempts per step, e.g. 100 - then skip the step and continue;
  document the behavior). `k = 0` is identity.
- **gen-spec modes**: fully random: `--seed N` + size bounds (`--size small|medium`
  or explicit caps); mutation mode: `--base SPEC.rfs|SPEC.json --mutations K
  [--seed N]`. Output is ALWAYS validated before writing (parse + validate the
  rendered text; on failure exit 1 with a diagnostic - should not happen by
  construction, but guard anyway).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | Formal semantics document | `ref_mc` implements `docs/SEMANTICS.md` directly - the doc must exist first so the engine is coded from it and mismatches surface as test failures |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | Differential soundness suites | consumes `ref_mc` (ground truth + unified reports) and `specgen` (random specs); 4.1 compares `rfs mc-ref` output vs generated binaries directly |
| 5. | Fuzzing the toolchain itself | the codegen fuzz target builds random specs via specgen-style Arbitrary; the slow suite uses `ref_mc` as ground truth for "no false PASS (complete)" |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/specgen.rs` | new | seeded random spec builder + incremental mutation API |
| `src/ref_mc.rs` | new | brute-force reference MC + unified text/JSON report rendering |
| `src/mc_ref_cmd.rs` | new | `rfs mc-ref` command logic |
| `src/gen_spec_cmd.rs` | new | `rfs gen-spec` command logic |
| `src/main.rs` | change | dispatch arms + usage text for both commands |
| `src/lib.rs` | change | module declarations |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| lib unit tests (in `src/specgen.rs`, `src/ref_mc.rs`) | new | determinism pins, validity invariants, hand-computed micro-specs, report format pins |
| `tests/frontend_test.rs` | change | end-to-end for both commands: flags, exit codes 0/1/2, output agreement with the generated engine on a pinned spec, error cases |

## Planned actions

### Action 1: random spec generator core (subtask 3.1a)
`src/specgen.rs`: `SpecGen { rng, bounds }` building IR values programmatically
(states/enums, events with finite payloads, `bool`/enum/`bit<N>` vars, guarded
transitions with priorities, `on_error`, properties/assumptions from the
implemented subset), rendered to source text via `src/printer.rs` (guarantees
parseability + exercises the printer). Bounds presets: `small` (<= 6 states,
for MC fuzzing) and `medium` (benchmark-like); explicit caps overridable.
Deterministic from a u64 seed (LCG family per Context). Unit tests: seed-
determinism pins (same seed => byte-identical source), N=50 generated specs all
parse + validate, bounds respected.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/specgen.rs` | new | generator core + presets |
| `src/lib.rs` | change | `pub mod specgen;` |

### Action 2: incremental mutation mode (subtask 3.1b)
Extend `src/specgen.rs` with `pub fn mutate(base_ir: &Ir, rng: &mut Rng, k: usize) -> Ir`
applying K seeded mutation steps (operation list per Context). Each candidate
mutation is validated; invalid candidates roll back + redraw (bounded attempts,
skip step on exhaustion). Unit tests: `k = 0` identity (byte-identical render),
per-step validity invariant (mutate with k=1..20 always yields valid specs),
determinism pins, and a coverage smoke test (over many seeds the operations
actually exercise adds AND removes - not just one op type).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/specgen.rs` | change | mutation API + operation set |

### Action 3: brute-force reference MC engine (subtask 3.2a)
`src/ref_mc.rs`: exhaustive reachable-state exploration per `docs/SEMANTICS.md`
section 9: level-synchronous BFS over configurations `(state, vars,
monitor_bits)` with a Vec frontier + HashMap visited set; representative inputs
computed independently from the IR (rule per SEMANTICS.md - reimplemented, not
imported from `mc_plan`); its own property evaluator over recorded traces
(different algorithm from `src/monitor.rs`); liveness via end-component analysis
(a liveness violation exists iff a reachable end component never discharges the
obligation - construct the lasso from parent pointers); deadlock detection per
the documented definition. Public API: `run(ir, config) -> RefResult` with
status (pass/fail/deadlock), first-violation label + step + counterexample
(trace/lasso/deadlock-config in the `codegen_mc` JSON types), total state count,
max depth; PLUS `minimal_violation_labels(ir, config) -> Vec<String>` (sorted set
of labels violated at minimal depth - order-independent, used by task 4.1 for
set comparison when multiple violations tie). Config: max_states, timeout_sec,
assumes selection. Unit tests on hand-computed micro-specs: a passing spec,
a safety FAIL with known label+step, a liveness FAIL (lasso), a deadlock,
a max-states truncation (UnknownStateLimit).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/ref_mc.rs` | new | engine + API |
| `src/lib.rs` | change | `pub mod ref_mc;` |

### Action 4: unified report rendering (subtask 3.2b)
Render `RefResult` in BOTH formats identical to the generated exact-mode
binaries: text (copy the line formats from `src/backends/rust_mc.rs`'s text
report - summary line, violation lines, trace printing; verify against an actual
generated-binary run on a pinned spec) and JSON (build an `McJsonReport` via the
`src/codegen_mc.rs` builder so the document is canonical). Unit pins: byte-exact
text + JSON for each of the five micro-spec outcomes from action 3.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/ref_mc.rs` | change | text + JSON renderers + pins |

### Action 5: `rfs mc-ref` command (subtask 3.3)
`src/mc_ref_cmd.rs` + main.rs dispatch/usage: `rfs mc-ref <spec.rfs|spec.json>
[--max-states N] [--timeout-sec N] [--assumes all|none|labels]
[--counterexample-format json|text] [--log-level L]`. Pipeline: load (extension
selects format, same rule as other commands) -> validate -> `ref_mc::run` ->
render report to stdout. Exit codes MIRROR the generated binaries: 0
pass-complete, 1 fail, 2 unknown-state-limit; usage/parse/validation errors
follow the existing command convention (verify what `validate`/`info` use and
match). Usage text documents which generated-binary flags do NOT apply and why
(Context). Tests in `tests/frontend_test.rs`: exit-code pins for all three
statuses on pinned specs (incl. a tiny `--max-states` negative control),
flag-parsing errors, and text/JSON output AGREEMENT with the actual generated
binary (compile example 01's MC artifact, run both, compare - this is the bridge
to task 4.1).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc_ref_cmd.rs` | new | command logic |
| `src/main.rs` | change | dispatch + usage text |
| `tests/frontend_test.rs` | change | e2e tests above |

### Action 6: `rfs gen-spec` command (subtask 3.4)
`src/gen_spec_cmd.rs` + main.rs dispatch/usage: `rfs gen-spec --out FILE.rfs
[--seed N] [--size small|medium] [--max-states N] [--max-events N] [--max-vars N]
[--max-transitions N] [--max-props N] | --base SPEC.rfs|SPEC.json --mutations K
[--seed N]`. Default seed: a pinned constant (document it). Fully random mode
uses the action-1 builder; mutation mode parses the base, runs `mutate`, renders.
Output is parsed + validated before writing; on any failure exit 1 with a
diagnostic. Tests in `tests/frontend_test.rs`: seed determinism (same args =>
byte-identical file), base+mutations reproducibility, output validity sweep
(20 seeds x both modes all parse+validate), error cases (missing --out, bad base
file, unknown flag).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/gen_spec_cmd.rs` | new | command logic |
| `src/main.rs` | change | dispatch + usage text |
| `tests/frontend_test.rs` | change | e2e tests above |

## Task deliverables
- `src/specgen.rs`: deterministic random spec builder (small/medium presets) +
  incremental mutation API with per-step validity.
- `src/ref_mc.rs`: brute-force reference MC (independent algorithms) with unified
  text/JSON reports matching the generated engines, incl. the order-independent
  `minimal_violation_labels` API for differential testing.
- `rfs mc-ref` command (unified flags, exit codes 0/1/2).
- `rfs gen-spec` command (fully random + base-mutation modes, always-valid output).
- Lib unit tests + frontend e2e tests for all of the above.

## Task exit criteria
- [ ] specgen: determinism pins green; 50+ generated specs parse+validate; mutation per-step validity invariant holds for k up to 20; `k = 0` identity pinned.
- [ ] ref_mc: all five hand-computed micro-spec outcomes correct (pass / safety FAIL label+step / liveness lasso / deadlock / truncated UnknownStateLimit); independence review done (no imports from `mc_plan`/`monitor.rs` search code - record the checked import list in "Other information").
- [ ] Reports: text + JSON byte-pins for all five outcomes; on a pinned real spec, `rfs mc-ref` output agrees with the compiled generated MC binary (text and JSON).
- [ ] `rfs mc-ref`: exit codes 0/1/2 pinned for the three statuses; usage text documents the non-applicable flags.
- [ ] `rfs gen-spec`: both modes deterministic + always-valid output; error cases pinned.
- [ ] Full suite green: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No SMT mode in ref_mc (feature-gated, outside the v1 guarantee); no fuzz/bounded mode in `rfs mc-ref` (exact exhaustive search only - that is its purpose); no interactive REPL (shell one-liners are the target usage); no changes to the generated engines (if a mismatch vs them is found, record it as a plan correction + fix task - do NOT fix engines here).

## Other information
- Record after action 3: the import list of `src/ref_mc.rs` proving independence
  (allowed: `ir`, `json_rw`, `codegen_mc` data types, `parser`/`validator` for
  the CLI path; NOT allowed: `mc_plan`, `monitor`, backend modules).
- Timeout handling in ref_mc: wall-clock check between BFS levels (same spirit as
  the generated engine's `--timeout-sec`); on timeout => UnknownStateLimit.
- Keep both commands THIN: all logic in the lib modules so integration tests can
  use them without spawning the binary where possible.
