# Phase 18, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Create the 10 Protocol FSMs A1-A10 (transport / routing / signaling) example projects (`tcp_full_states`, `tls13_handshake`, `quic_loss_recovery`, `dhcp_client`, `dns_resolver`, `arp_resolution`, `stp_port_states`, `bgp_fsm`, `ospf_neighbor`, `sip_call_signaling`) - each a real FSM spec that
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
- **Large-state-space twins.** For the FSMs below marked with a `full.rfs` twin (`tcp_full_states`, `quic_loss_recovery`, `dns_resolver`), ALSO write `full.rfs` - the un-simplified, real-width model whose state space is too large for exact MC and for which no meaningful abstraction lowers it. Check it with `rfs mc-ref --mode smt-bounded --bound N` (a k-step safety proof) and `--mode fuzz --runs R --seed S` (random testing), NOT exact MC. Capture the smt-bounded verdict + a pinned fuzz transcript. The README gains an "Model checking - large state space (smt-bounded + fuzz)" subsection explaining why exact MC is infeasible. This demonstrates that even very large specifications can be analyzed and checked with RFS where adequate simplification is impossible.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | Cargo-member production-integration subset | needs the `tcp_full_states` spec from this task |
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
`crates/examples/tcp_full_states/`, `crates/examples/tls13_handshake/`, `crates/examples/quic_loss_recovery/`, `crates/examples/dhcp_client/`, `crates/examples/dns_resolver/`, `crates/examples/arp_resolution/`, `crates/examples/stp_port_states/`, `crates/examples/bgp_fsm/`, `crates/examples/ospf_neighbor/`, `crates/examples/sip_call_signaling/` | new | pass.rfs + fail.rfs + fixed.rfs + run.sh + captures/ + README.md each |

## Planned actions

### 1.0 - The per-FSM workflow (applies to EVERY FSM in this task)
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
6. Add a test leg (see 1.11): assert positive PASS, the negative PINNED CE excerpt
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

### 1.1 - `tcp_full_states`
Full TCP connection machine (RFC 793/9293): LISTEN..ESTABLISHED + teardown FIN_WAIT_1/2, CLOSE_WAIT, CLOSING, LAST_ACK, TIME_WAIT; seq numbers (bit<32>), retransmit timer, RTO.
- `fail.rfs` defect: Drop the `FIN_WAIT_2 on ACK -> CLOSED` guard so a half-open connection deadlocks - pinned `reason=deadlock` CE ending in FIN_WAIT_2.
- `fixed.rfs`: Restore the FIN_WAIT_2->CLOSED transition.
- **Large-state-space twin (`full.rfs`):** full 32-bit sequence numbers + retransmit buffer - too large for exact MC, no meaningful abstraction lowers it; check via `--mode smt-bounded --bound N` (no loss/reorder within N steps) + `--mode fuzz` (wrap-around/retransmission violations). See PHASE_18.md A1.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A1.

### 1.2 - `tls13_handshake`
TLS 1.3 handshake (RFC 8446): ClientHello->ServerHello->key exchange->Finished + abort paths; negotiated cipher, transcript hash.
- `fail.rfs` defect: Remove the downgrade-protection guard so a legacy ChangeCipherSpec path is accepted - pinned property-violation CE on the 'no app data before Finished' prop.
- `fixed.rfs`: Re-add the downgrade-protection guard.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A2.

### 1.3 - `quic_loss_recovery`
QUIC (RFC 9000) loss recovery: packet-number space, RTT, PTO timer, congestion window, retransmission; cwnd/ssthresh/rtt/pto_counter vars.
- `fail.rfs` defect: Remove the PTO-backoff increment so the timer never grows - pinned liveness-violation (lasso) CE showing an infinite retransmit loop.
- `fixed.rfs`: Restore the PTO backoff increment.
Extra RFS mode: Also demonstrate `--mode fuzz` (random loss patterns).
- **Large-state-space twin (`full.rfs`):** full packet-number space + congestion state - too large for exact MC; check via `--mode smt-bounded --bound N` (ack-or-retransmit within bound) + `--mode fuzz` (stuck retransmit loop). See PHASE_18.md A3.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A3.

### 1.4 - `dhcp_client`
DHCP client (RFC 2131): INIT->SELECTING->REQUESTING->BOUND->RENEWING->REBINDING; lease time, T1/T2 timers, offered address.
- `fail.rfs` defect: Let T1 fire without transitioning to RENEWING - pinned deadlock CE in BOUND.
- `fixed.rfs`: Add the T1->RENEWING transition.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A4.

### 1.5 - `dns_resolver`
Recursive resolver per-query machine: query->retry(next nameserver)->timeout->validate->cache; retry count, TTL cache entry.
- `fail.rfs` defect: Remove the TTL-expiry check so a stale cache entry is served - pinned property-violation CE on 'no stale answer'.
- `fixed.rfs`: Re-add the TTL-expiry guard.
- **Large-state-space twin (`full.rfs`):** a large TTL cache (the cache content IS the state) - too large for exact MC; check via `--mode smt-bounded --bound N` (no stale answer within bound) + `--mode fuzz` (stale-answer/collision). See PHASE_18.md A5.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A5.

### 1.6 - `arp_resolution`
Per-entry ARP (RFC 826): INCOMPLETE->pending retransmit->RESOLVED + aging/gratuitous; retry counter, age timer, MAC.
- `fail.rfs` defect: Drop the retransmit-limit so an unresolvable entry loops forever - pinned liveness CE.
- `fixed.rfs`: Add the limit->deletion transition.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A6.

### 1.7 - `stp_port_states`
STP (IEEE 802.1D) port machine: DISCOVERING->LISTENING->LEARNING->FORWARDING + BLOCKING; root election via BPDU compare; bridge/root IDs, cost, timers.
- `fail.rfs` defect: Allow a backup port to FORWARD while the designated port also forwards - pinned property-violation CE on the loop-free prop.
- `fixed.rfs`: Keep the backup port BLOCKING.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A7.

### 1.8 - `bgp_fsm`
BGP peer FSM (RFC 4271): Idle->Connect->Active->OpenSent->OpenConfirm->Established + NOTIFICATION; BGP id, hold timer, keepalive count.
- `fail.rfs` defect: Accept an UPDATE in OpenConfirm by removing the state guard - pinned property-violation CE on 'no UPDATE before Established'.
- `fixed.rfs`: Re-add the Established-state guard on UPDATE.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A8.

### 1.9 - `ospf_neighbor`
OSPF neighbor FSM (RFC 2328): Down..TwoWay->ExStart->Exchange->Loading->Full; DD sequence-number handshake, master/slave role.
- `fail.rfs` defect: Let both sides believe they are master by removing the role-resolution guard - pinned property-violation CE on 'single master'.
- `fixed.rfs`: Add the master/slave tie-break.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A9.

### 1.10 - `sip_call_signaling`
SIP (RFC 3261) invite transaction + dialog: INVITE->100/180->200->ACK; BYE, re-INVITE, CANCEL; call-ID, branch, dialog state.
- `fail.rfs` defect: Drop CANCEL handling so a cancelled invite still reaches Established - pinned property-violation CE on 'no Established after CANCEL'.
- `fixed.rfs`: Add the CANCEL->terminated transition.
- Full description, RFS-mode rationale, files layout and README plan: PHASE_18.md Group A, section A10.


### 1.11 - Test legs + inventory rows
Add one test fn per FSM to `tests/example_projects_test.rs`: run the real commands via
`rfs_bin()`, assert exit 0 on `pass.rfs` (PASS complete), assert exit 1 on `fail.rfs` WITH a
PINNED excerpt of the failure output (the CE reason/label/step line or diagnostic message -
copied from the real capture, never hand-written), assert exit 0 on `fixed.rfs`, and assert
both-backends production codegen compiles warning-free. The MC + production legs are
availability-gated on rustc/gcc like the existing c_sync_handler leg. For the three twin FSMs (`tcp_full_states`, `quic_loss_recovery`, `dns_resolver`), ALSO add an smt-bounded leg (`rfs mc-ref full.rfs --mode smt-bounded --bound N`, feature-gated on `smt` + availability-gated - skip-with-note if z3 is absent) asserting the pinned k-step verdict, and a fuzz leg (`rfs mc-ref full.rfs --mode fuzz --runs R --seed S`) asserting a pinned transcript excerpt. Append a
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
- [x] Every `pass.rfs` model-checks to PASS complete under `rfs mc-ref` exact (verified before pinning).
- [x] Every `fail.rfs` produces the pinned counterexample/diagnostic and every `fixed.rfs` re-runs to PASS - both asserted by test legs with excerpts copied from REAL captures (no hand-written approximations).
- [x] Every spec compiles warning-free in BOTH backends (`rfs codegen rust production` + `rfs codegen c production`) - asserted by test legs.
- [x] The three large-state-space twins (`tcp_full_states`, `quic_loss_recovery`, `dns_resolver`) each ship a `full.rfs` checked via `--mode smt-bounded` (k-step verdict pinned) and `--mode fuzz` (pinned transcript), NOT exact MC, with a README subsection explaining why exact MC is infeasible.
- [x] Every README follows the PHASE_18.md shared template (all sections; mermaid stateDiagram present; one RFS-mode subsection per demonstrated mode with description + rationale).
- [x] TEST_STRUCTURE rows added; `cargo test --test dev_docs_test` green.
- [x] `cargo test --test example_projects_test` green; `cargo test --test docs_md_test` green (the new READMEs are in its file set).
- [x] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Task corrections

| # | Correction | Reason |
|---|------------|--------|
| 1a | The three large-state-space twins' `full.rfs` must use **field-name event parameters** in patterns/guards (e.g. `Reply(R(rkey)) where rkey == key`) and a **plain-boolean** invariant (no `output_has`, only expr/not/and/or/implies) to be checkable by `--mode smt-bounded`. | Discovered while authoring the `dns_resolver` twin: renaming an event parameter in the pattern (`R(k)` for field `v`) makes the SMT encoder report `unknown identifier 'k'`, and `output_has(...)` in a property is rejected by smt-bounded. The tcp/quic twins already followed this; the dns twin was corrected to match (a `served`/`last_serve_ttl` snapshot encodes "no stale answer" as a plain boolean). |

## Non-goals
- No changes to `examples/valid/` or `examples/invalid/` (the corpus stays as is).
- No new workspace members (task 7 promotes the three chosen FSMs).
- No training-set changes (task 9) and no EXAMPLES.md changes (task 8).
