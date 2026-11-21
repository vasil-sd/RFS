# Phase 18, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Create the 8 Other FSMs C1-C8 (consensus / resilience / hardware-digital) example projects (`raft_election_log`, `two_phase_commit`, `circuit_breaker`, `token_bucket_limiter`, `cache_writeback`, `pcie_ltssm`, `uart_i2c_line`, `bus_arbiter`) - each a real FSM spec that
model-checks to PASS, a one-defect variant that fails with a PINNED counterexample,
the fix, a run script, pinned captures, and a template README - plus one test leg per
FSM. Rationale: this is the C-group slice of the Phase 18 real-world-FSM catalogue;
together the six batch tasks cover all 50 machines.

## Context
- All examples are PLAIN DIRS under `crates/examples/` (NOT workspace members - that is
  task 7's job for the three promoted FSMs). Each dir carries: `pass.rfs`, `fail.rfs`,
  `fixed.rfs`, `run.sh`, `captures/` (pinned text + JSON CE/diagnostic), `README.md`.
- README template (BINDING, from PHASE_18.md "Shared example README template"): What it
  demonstrates / The machine (mermaid `stateDiagram-v2` ALWAYS - the subset rfs generates,
  guaranteed to parse under docs_md_test) / RFS modes demonstrated (ONE subsection per mode
  with description + rationale) / Files layout / Build-run-test / Expected results (incl. the
  negative CE walkthrough + fix proof). Relative links to user docs are `../../../docs/user/...`.
- Test-leg conventions (from the existing `tests/example_projects_test.rs`): `repo_root()` /
  `rfs_bin()` helpers, availability gating via `tests/common/toolchain.rs` (skip-with-note,
  never fail), scratch dirs under `target/`, pinned output excerpts asserted with `contains`.
  Compile generated artifacts from the example dir with BARE filenames (the generated MC
  artifacts reference their fsm by codegen-time name).
- Inventory rule (PROTOCOL.md section 5): every new test fn gets a TEST_STRUCTURE.md row in
  the SAME commit (match the existing row format exactly; use `rfs-dev inventory add --doc
  TEST ...` if available).
- Spec-authoring guidance: model each machine at its DISCRETE level - bit-vector vars for
  sequence numbers / timers / windows / counters, guarded transitions, emitted outputs. Use
  `assume` for environment contracts ("peer acknowledges", "network delivers", "responder
  replies") and `bounded_eventually[N]` for liveness properties. Keep the state count in the
  tens-to-low-thousands so exact MC is fast (cf. http2_stream ~434, sliding_window 94). The
  `fail.rfs` defect must produce a DETERMINISTIC, pin-able counterexample (a specific
  reason/label/step + CE steps) - avoid defects that only surface under fuzz. If a real
  protocol is too large to model cleanly within the state budget, SIMPLIFY it (fewer states /
  coarser abstraction) and note the simplification in its README - do not drop the example.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | Cargo-member production-integration subset | needs the `uart_i2c_line` spec from this task |
| 8. | EXAMPLES.md "Other examples" section | needs this task's READMEs to link |
| 9. | Training-set update | references these examples |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | 8 new test legs (one per FSM) |
| `docs/llm/TEST_STRUCTURE.md` | change | 8 new rows (same-commit inventory rule) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
`crates/examples/raft_election_log/`, `crates/examples/two_phase_commit/`, `crates/examples/circuit_breaker/`, `crates/examples/token_bucket_limiter/`, `crates/examples/cache_writeback/`, `crates/examples/pcie_ltssm/`, `crates/examples/uart_i2c_line/`, `crates/examples/bus_arbiter/` | new | pass.rfs + fail.rfs + fixed.rfs + run.sh + captures/ + README.md each |

## Planned actions

### 5.0 - The per-FSM workflow (applies to EVERY FSM in this task)
For each FSM below, do these six steps in order:
1. Write `pass.rfs` - a COMPLETE, VALID module modelling the real machine at its discrete
   level. Run `rfs mc-ref pass.rfs` and confirm PASS complete BEFORE pinning anything.
2. Write `fail.rfs` - introduce the ONE specific defect listed for that FSM. Run mc-ref,
   capture the REAL text FAIL report + counterexample AND the JSON form
   (`--counterexample-format json`) into `captures/`.
3. Write `fixed.rfs` - the one-line correction. Re-run mc-ref and confirm PASS complete.
4. Write `run.sh` - drives the demonstrated RFS modes: `rfs validate`, `rfs info`,
   `rfs mc-ref` (exact; plus fuzz / smt-bounded where noted for that FSM),
   `rfs codegen rust mc` + compile + run, `rfs codegen c mc` + compile + run (both must
   agree with mc-ref), `rfs codegen rust production` + compile, `rfs codegen c production`
   + compile (warning-free).
5. Write `README.md` per the PHASE_18.md shared template - one RFS-mode subsection per mode
   demonstrated, the mermaid stateDiagram, and the negative CE walkthrough + fix proof.
6. Add a test leg (see 5.9): assert positive PASS, the negative PINNED CE excerpt
   (copied from the real capture), the fix PASS, and both-backends production compile.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/<fsm>/pass.rfs` | new | the valid spec that model-checks to PASS |
| `crates/examples/<fsm>/fail.rfs` | new | the one-defect variant (pinned CE) |
| `crates/examples/<fsm>/fixed.rfs` | new | the corrected spec that re-runs to PASS |
| `crates/examples/<fsm>/run.sh` | new | drives all demonstrated RFS modes |
| `crates/examples/<fsm>/captures/` | new | pinned text + JSON CE/diagnostic excerpts |
| `crates/examples/<fsm>/README.md` | new | template-conformant walkthrough |

### 5.1 - `raft_election_log`
Raft: follower/candidate/leader; heartbeat, request-vote, append-entry; current term, vote-granted, last log index.
- `fail.rfs` defect: Let two nodes both believe they won a term by removing the majority-vote guard - pinned property-violation CE on 'at most one leader per term'.
- `fixed.rfs`: Re-add the majority-vote check.
Extra RFS mode: Flagship distributed-safety example.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C1.

### 5.2 - `two_phase_commit`
2PC coordinator+participant: in-doubt->prepared->commit/abort + timeout recovery; decision, prepared flag, timeout counter.
- `fail.rfs` defect: Let a participant commit on its own after a coordinator timeout by removing the coordinator-decision guard - pinned property-violation CE on 'no commit without coordinator commit'.
- `fixed.rfs`: Re-add the coordinator-decision guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C2.

### 5.3 - `circuit_breaker`
Circuit breaker: closed->open(failure threshold)->half-open(cooldown)->closed(success)/open(probe fail); failure count, cooldown timer, probe state.
- `fail.rfs` defect: Let requests pass while open by removing the state gate - pinned property-violation CE on 'no request while open'.
- `fixed.rfs`: Re-add the open-state gate.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C3.

### 5.4 - `token_bucket_limiter`
Rate limiter: refill->consume->burst->throttle; token count, bucket capacity, last-refill timestamp.
- `fail.rfs` defect: Allow consumption beyond the token count by removing the balance guard - pinned property-violation CE on 'never exceed rate'.
- `fixed.rfs`: Re-add the token-balance guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C4.

### 5.5 - `cache_writeback`
Cache line controller: idle->read-hit/read-miss-fetch/write-allocate/writeback-dirty/invalidate; line state (M/E/S/I), dirty bit, tag.
- `fail.rfs` defect: Replace a dirty line without writing it back by removing the writeback-before-replace guard - pinned property-violation CE on 'dirty line written back before replacement'.
- `fixed.rfs`: Re-add the writeback-before-replace guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C5.

### 5.6 - `pcie_ltssm`
PCIe LTSSM: Detect->Polling->Configuration->L0 + Recovery/L1/L2; link speed, width, training-token count.
- `fail.rfs` defect: Allow data TLPs during Recovery by removing the state gate - pinned property-violation CE on 'no data in recovery'.
- `fixed.rfs`: Re-add the L0-only data gate.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C6.

### 5.7 - `uart_i2c_line`
UART TX/RX line: idle->start-bit->data-bits->stop-bit + framing-error; I2C start/ack/data/stop; bit index, shift register, ack flag. (Bit-level.)
- `fail.rfs` defect: Emit a byte before the stop bit by removing the frame-complete guard - pinned property-violation CE on 'no byte mid-frame'.
- `fixed.rfs`: Re-add the frame-complete guard.
Extra RFS mode: Also becomes a cargo-member crate in task 7 (no_std).
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C7.

### 5.8 - `bus_arbiter`
Arbiter: request->grant->service->deassert + priority arbitration; request bitmask, grant index, priority level.
- `fail.rfs` defect: Grant two requestors at once by removing the single-grant guard - pinned property-violation CE on 'at most one grant'.
- `fixed.rfs`: Re-add the single-grant guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C8.


### 5.9 - Test legs + inventory rows
Add one test fn per FSM to `tests/example_projects_test.rs`: run the real commands via
`rfs_bin()`, assert exit 0 on `pass.rfs` (PASS complete), assert exit 1 on `fail.rfs` WITH a
PINNED excerpt of the failure output (the CE reason/label/step line or diagnostic message -
copied from the real capture, never hand-written), assert exit 0 on `fixed.rfs`, and assert
both-backends production codegen compiles warning-free. The MC + production legs are
availability-gated on rustc/gcc like the existing c_sync_handler leg. Append a
TEST_STRUCTURE.md row per new fn in the same commit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change | 8 new test fns - pin the failures so a regression in CE format or codegen is caught |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- 8 example projects under `crates/examples/` (pass/fail/fixed specs + run script + pinned
  captures + template README each).
- 8 pinned test legs + 8 TEST_STRUCTURE inventory rows.

## Task exit criteria
- [ ] Every `pass.rfs` model-checks to PASS complete under `rfs mc-ref` exact (verified before pinning).
- [ ] Every `fail.rfs` produces the pinned counterexample/diagnostic and every `fixed.rfs` re-runs to PASS - both asserted by test legs with excerpts copied from REAL captures (no hand-written approximations).
- [ ] Every spec compiles warning-free in BOTH backends (`rfs codegen rust production` + `rfs codegen c production`) - asserted by test legs.
- [ ] Every README follows the PHASE_18.md shared template (all sections; mermaid stateDiagram present; one RFS-mode subsection per demonstrated mode with description + rationale).
- [ ] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [ ] `cargo test --test example_projects_test` green; `cargo test --test docs_md_test` green (the new READMEs are in its file set).
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No changes to `examples/valid/` or `examples/invalid/` (the corpus stays as is).
- No new workspace members (task 7 promotes the three chosen FSMs).
- No training-set changes (task 9) and no EXAMPLES.md changes (task 8).
