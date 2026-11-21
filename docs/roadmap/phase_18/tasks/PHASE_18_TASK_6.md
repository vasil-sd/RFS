# Phase 18, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Create the 7 Other FSMs C9-C15 (security / deployment / UI / AI / workflow) example projects (`oauth2_authcode`, `fw_conntrack`, `login_lockout`, `update_rollout`, `media_player`, `game_ai_fsm`, `approval_workflow`) - each a real FSM spec that
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
| 8. | EXAMPLES.md "Other examples" section | needs this task's READMEs to link |
| 9. | Training-set update | references these examples |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | 7 new test legs (one per FSM) |
| `docs/llm/TEST_STRUCTURE.md` | change | 7 new rows (same-commit inventory rule) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
`crates/examples/oauth2_authcode/`, `crates/examples/fw_conntrack/`, `crates/examples/login_lockout/`, `crates/examples/update_rollout/`, `crates/examples/media_player/`, `crates/examples/game_ai_fsm/`, `crates/examples/approval_workflow/` | new | pass.rfs + fail.rfs + fixed.rfs + run.sh + captures/ + README.md each |

## Planned actions

### 6.0 - The per-FSM workflow (applies to EVERY FSM in this task)
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
6. Add a test leg (see 6.8): assert positive PASS, the negative PINNED CE excerpt
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

### 6.1 - `oauth2_authcode`
OAuth2 (RFC 6749) authorization-code grant: redirect->authorize->code-exchange->token->refresh->revoke; state/nonce, code, token expiry.
- `fail.rfs` defect: Issue a token on a mismatched state by removing the state-check guard - pinned property-violation CE on 'no token without matching state'.
- `fixed.rfs`: Re-add the state/nonce check.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C9.

### 6.2 - `fw_conntrack`
Conntrack entry: NEW->ESTABLISHED->RELATED + INVALID; 5-tuple, connection state, idle timeout.
- `fail.rfs` defect: Accept an outside-initiated reply to a NEW flow by removing the direction guard - pinned property-violation CE on 'no outside reply to NEW'.
- `fixed.rfs`: Re-add the direction guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C10.

### 6.3 - `login_lockout`
Auth machine: idle->authenticating->success/failure-count->locked->session-active->token-refresh; MFA step; failure count, lockout timer, session expiry.
- `fail.rfs` defect: Allow a refresh past the session window by removing the expiry guard - pinned property-violation CE on 'no refresh past expiry'.
- `fixed.rfs`: Re-add the session-expiry guard on refresh.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C11.

### 6.4 - `update_rollout`
Updater: idle->download->verify-signature->stage->canary->promote/rollback->reboot; image hash, canary health, stage index.
- `fail.rfs` defect: Boot an image with a bad signature by removing the verify gate - pinned property-violation CE on 'no boot from unverified image'.
- `fixed.rfs`: Re-add the signature-verify gate.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C12.

### 6.5 - `media_player`
Player: stopped->buffering->playing->paused->seeking->ended->error; buffer level, play position, target position.
- `fail.rfs` defect: Honor a seek during an underrun by removing the buffer guard - pinned property-violation CE on 'no seek during underrun'.
- `fixed.rfs`: Re-add the buffer-underrun guard on seek.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C13.

### 6.6 - `game_ai_fsm`
Game AI: idle->patrol->alert->chase->attack->flee->dead + platformer controller (grounded/airborne/jumping/falling/wall-slide); health, distance-to-target, on-ground.
- `fail.rfs` defect: Allow attack and flee simultaneously by removing the behavior-exclusion guard - pinned property-violation CE on 'no two exclusive behaviors'.
- `fixed.rfs`: Re-add the behavior mutual exclusion.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C14.

### 6.7 - `approval_workflow`
Workflow: draft->submitted->in-review->approved/rejected->escalated->executed; delegation + SLA timers; approver, SLA deadline, delegation depth.
- `fail.rfs` defect: Execute a draft that was never approved by removing the approval gate - pinned property-violation CE on 'no execution without approval'.
- `fixed.rfs`: Re-add the approval gate on execute.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group C, section C15.


### 6.8 - Test legs + inventory rows
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
| `tests/example_projects_test.rs` | change | 7 new test fns - pin the failures so a regression in CE format or codegen is caught |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- 7 example projects under `crates/examples/` (pass/fail/fixed specs + run script + pinned
  captures + template README each).
- 7 pinned test legs + 7 TEST_STRUCTURE inventory rows.

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
