# Phase 18, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Create the 10 Protocol FSMs A11-A20 (application / storage / wireless / security) example projects (`mqtt_qos_session`, `coap_retransmit`, `ppp_lcp_ipcp`, `iscsi_login`, `fibrechannel_login`, `nvme_controller`, `sata_link_power`, `usb_device_link`, `bt_gatt_session`, `kerberos_auth`) - each a real FSM spec that
model-checks to PASS, a one-defect variant that fails with a PINNED counterexample,
the fix, a run script, pinned captures, and a template README - plus one test leg per
FSM. Rationale: this is the A-group slice of the Phase 18 real-world-FSM catalogue;
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
- **Large-state-space twins.** For the FSMs below marked with a `full.rfs` twin (`mqtt_qos_session`, `nvme_controller`), ALSO write `full.rfs` - the un-simplified, real-width model whose state space is too large for exact MC and for which no meaningful abstraction lowers it. Check it with `rfs mc-ref --mode smt-bounded --bound N` (a k-step safety proof) and `--mode fuzz --runs R --seed S` (random testing), NOT exact MC. Capture the smt-bounded verdict + a pinned fuzz transcript. The README gains an "Model checking - large state space (smt-bounded + fuzz)" subsection explaining why exact MC is infeasible. This demonstrates that even very large specifications can be analyzed and checked with RFS where adequate simplification is impossible.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | EXAMPLES.md "Other examples" section | needs this task's READMEs to link |
| 9. | Training-set update | references these examples |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | 10 new test legs (one per FSM) |
| `docs/llm/TEST_STRUCTURE.md` | change | 10 new rows (same-commit inventory rule) |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
`crates/examples/mqtt_qos_session/`, `crates/examples/coap_retransmit/`, `crates/examples/ppp_lcp_ipcp/`, `crates/examples/iscsi_login/`, `crates/examples/fibrechannel_login/`, `crates/examples/nvme_controller/`, `crates/examples/sata_link_power/`, `crates/examples/usb_device_link/`, `crates/examples/bt_gatt_session/`, `crates/examples/kerberos_auth/` | new | pass.rfs + fail.rfs + fixed.rfs + run.sh + captures/ + README.md each |

## Planned actions

### 2.0 - The per-FSM workflow (applies to EVERY FSM in this task)
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
6. Add a test leg (see 2.11): assert positive PASS, the negative PINNED CE excerpt
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

### 2.1 - `mqtt_qos_session`
MQTT (RFC 8223) session + QoS handshakes: CONNECT->CONNACK, PUBLISH QoS0/1/2 (PUBACK; PUBREC/PUBREL/PUBCOMP), SUBSCRIBE/SUBACK, DISCONNECT; packet-id space.
- `fail.rfs` defect: Allow a duplicate PUBLISH to be delivered twice by removing the packet-id dedup guard - pinned property-violation CE on 'exactly-once'.
- `fixed.rfs`: Re-add the packet-id dedup guard.
Extra RFS mode: Flagship delivery-guarantee example.
- **Large-state-space twin (`full.rfs`):** full 16-bit packet-id space (exactly-once at scale) - too large for exact MC; check via `--mode smt-bounded --bound N` (no duplicate/loss within bound) + `--mode fuzz` (QoS2 exactly-once violation). See PHASE_18.md A11.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A11.

### 2.2 - `coap_retransmit`
CoAP (RFC 7252) confirmable exchange: CON->ACK, retransmit with exponential backoff, RESET on token mismatch; token, retransmit counter, timeout.
- `fail.rfs` defect: Remove the retry cap so a lost ACK loops forever - pinned liveness CE.
- `fixed.rfs`: Add the cap->abort transition.
Extra RFS mode: Also demonstrate `--mode fuzz`.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A12.

### 2.3 - `ppp_lcp_ipcp`
PPP (RFC 1661/1332): LCP dead->req-sent->ack-rcvd/ack-sent->open, then IPCP; PAP/CHAP auth sub-state; negotiated options.
- `fail.rfs` defect: Allow IPCP to open while LCP is still ack-sent by removing the phase guard - pinned property-violation CE on 'no IPCP before LCP open'.
- `fixed.rfs`: Re-add the LCP-open-before-IPCP guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A13.

### 2.4 - `iscsi_login`
iSCSI (RFC 7143/SPC-4) login: nop-in/out->full-feature->operational; logout + recovery; session/target IDs.
- `fail.rfs` defect: Allow I/O in the full-feature phase by removing the phase guard - pinned property-violation CE on 'no I/O before operational'.
- `fixed.rfs`: Re-add the operational-phase guard on I/O.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A14.

### 2.5 - `fibrechannel_login`
Fibre Channel (FC-FS) FLOGI/PLOGI: NOP->F_LOGI->P_LOGI->logged-in + RSCN + logout; domain/node/port IDs.
- `fail.rfs` defect: Allow a second P_LOGI while already logged-in by removing the state guard - pinned property-violation CE on 'single logged-in'.
- `fixed.rfs`: Re-add the logged-in-state guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A15.

### 2.6 - `nvme_controller`
NVMe controller: reset->initialized->enabled; admin+IO queue lifecycle; AER; queue-pair indices, CQ/SQ heads.
- `fail.rfs` defect: Allow an IO submission in the initialized (not-enabled) state by removing the enable guard - pinned property-violation CE on 'no IO before enabled'.
- `fixed.rfs`: Re-add the enabled-state guard on IO submission.
- **Large-state-space twin (`full.rfs`):** many IO queue pairs with deep CQ/SQ (many concurrent entities) - too large for exact MC; check via `--mode smt-bounded --bound N` (no I/O before enabled, every submission completes within bound) + `--mode fuzz` (lost/out-of-order completion). See PHASE_18.md A16.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A16.

### 2.7 - `sata_link_power`
SATA link/power (ACS LPM): Active/Partial/DevSlumber/Slumber + H2D/D2H cross-port wake; link+device power state.
- `fail.rfs` defect: Allow a command in Slumber by removing the wake-before-command guard - pinned property-violation CE on 'no command while asleep'.
- `fixed.rfs`: Re-add the wake-before-command guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A17.

### 2.8 - `usb_device_link`
USB device/link: Default->Addressed->Configured->Suspend->Recovery + U0/U1/U2 resume; device address, config value, LPM state. (Distinct from existing usb_control_transfer.)
- `fail.rfs` defect: Allow a data token in Suspend by removing the state guard - pinned property-violation CE on 'no data while suspended'.
- `fixed.rfs`: Re-add the Suspend-state guard on data.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A18.

### 2.9 - `bt_gatt_session`
Bluetooth GATT (Core Vol 3): advertising->connecting->connected; services discovery, read/write/notify; connection interval, handle table.
- `fail.rfs` defect: Allow a notification to an unsubscribed client by removing the subscription guard - pinned property-violation CE on 'notify only when subscribed'.
- `fixed.rfs`: Re-add the subscription guard on notify.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A19.

### 2.10 - `kerberos_auth`
Kerberos (RFC 4120): AS-REQ/REP->TGS-REQ/REP->AP-REQ/REP + renewal; timestamps, ticket expiry, replay cache.
- `fail.rfs` defect: Accept a service request with an expired TGT by removing the expiry guard - pinned property-violation CE on 'no access with expired TGT'.
- `fixed.rfs`: Re-add the TGT-expiry guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A20.


### 2.11 - Test legs + inventory rows
Add one test fn per FSM to `tests/example_projects_test.rs`: run the real commands via
`rfs_bin()`, assert exit 0 on `pass.rfs` (PASS complete), assert exit 1 on `fail.rfs` WITH a
PINNED excerpt of the failure output (the CE reason/label/step line or diagnostic message -
copied from the real capture, never hand-written), assert exit 0 on `fixed.rfs`, and assert
both-backends production codegen compiles warning-free. The MC + production legs are
availability-gated on rustc/gcc like the existing c_sync_handler leg. For the two twin FSMs (`mqtt_qos_session`, `nvme_controller`), ALSO add an smt-bounded leg (`rfs mc-ref full.rfs --mode smt-bounded --bound N`, feature-gated on `smt` + availability-gated - skip-with-note if z3 is absent) asserting the pinned k-step verdict, and a fuzz leg (`rfs mc-ref full.rfs --mode fuzz --runs R --seed S`) asserting a pinned transcript excerpt. Append a
TEST_STRUCTURE.md row per new fn in the same commit.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/example_projects_test.rs` | change | 10 new test fns - pin the failures so a regression in CE format or codegen is caught |
| `docs/llm/TEST_STRUCTURE.md` | change | one row per new fn (same-commit inventory rule) |

## Task deliverables
- 10 example projects under `crates/examples/` (pass/fail/fixed specs + run script + pinned
  captures + template README each).
- 10 pinned test legs + 10 TEST_STRUCTURE inventory rows.

## Task exit criteria
- [ ] Every `pass.rfs` model-checks to PASS complete under `rfs mc-ref` exact (verified before pinning).
- [ ] Every `fail.rfs` produces the pinned counterexample/diagnostic and every `fixed.rfs` re-runs to PASS - both asserted by test legs with excerpts copied from REAL captures (no hand-written approximations).
- [ ] Every spec compiles warning-free in BOTH backends (`rfs codegen rust production` + `rfs codegen c production`) - asserted by test legs.
- [ ] The two large-state-space twins (`mqtt_qos_session`, `nvme_controller`) each ship a `full.rfs` checked via `--mode smt-bounded` (k-step verdict pinned) and `--mode fuzz` (pinned transcript), NOT exact MC, with a README subsection explaining why exact MC is infeasible.
- [ ] Every README follows the PHASE_18.md shared template (all sections; mermaid stateDiagram present; one RFS-mode subsection per demonstrated mode with description + rationale).
- [ ] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [ ] `cargo test --test example_projects_test` green; `cargo test --test docs_md_test` green (the new READMEs are in its file set).
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No changes to `examples/valid/` or `examples/invalid/` (the corpus stays as is).
- No new workspace members (task 7 promotes the three chosen FSMs).
- No training-set changes (task 9) and no EXAMPLES.md changes (task 8).
