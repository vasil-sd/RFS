# Phase 18: Extra examples - a broad catalogue of real-world FSMs

## Goal
Add a large catalogue of NEW example FSMs drawn from real protocols, industrial
control systems, and other state-machine domains, so that users see RFS applied
across a wide range of industrial areas - not just the handful of teaching specs
and the few protocol benchmarks already in `examples/valid/`. Each example is a
runnable project under `crates/examples/<fsm_title>/` that demonstrates, for THAT
machine, as much of the RFS surface as is meaningful: specification, static
analysis, model checking (in-process and both generated backends, positive AND
negative with an introduced error + fix), production code generation in the Rust
and C backends (std / no_std / property-monitors / adapters where it fits),
testing/fuzzing, and diagrams. The aim is that a user can pick any example and see
the full RFS workflow applied to a recognizable real system.

## Rationale
The existing 50 `examples/valid/*.rfs` are mostly language-feature teaching specs
(bit ops, guards, patterns, properties) plus a thin slice of real machines
(`tcp_handshake_like` 3-state, `sliding_window` ARQ, `token_ring`, `elevator`,
`http2_stream`, `usb_control_transfer`, `can_arbitration`). Phase 17 added ten
integration-workflow examples but no new *machines*. This phase fills the gap: a
broad, industrial-range catalogue of FSMs (networking/transport/storage/application
protocols, embedded/industrial control, and consensus/workflow/hardware-digital/
security/UI/AI state machines) so RFS's applicability is demonstrated end to end on
systems users recognize.

## Scope
- 50 NEW example projects under `crates/examples/`, one per FSM in the catalogue
  below (20 protocol, 15 control, 15 other). Each is a plain-dir project by
  default (spec + optional build/run script + pinned captures + README); a curated
  subset that best showcases production integration is additionally built as a
  cargo-member crate (see the per-FSM "Production codegen" note).
- Every example demonstrates, at minimum: `rfs validate` + `rfs info` (analysis),
  `rfs mc-ref` in the modes that fit the machine (exact always; fuzz and/or
  smt-bounded where meaningful), a POSITIVE model-checking run (PASS complete) and
  a NEGATIVE run (an introduced error -> a pinned counterexample trace or
  diagnostic -> the fix -> re-run PASS), and production code generation in BOTH
  backends (`rfs codegen rust production` + `rfs codegen c production`) compiling
  warning-free. Where it adds value: generated MC binaries (rust + c, with
  `--threads`/`--fast`), property monitors, no_std, rfs-adapters, `rfs test`
  unit/fuzz, and `rfs diagram`.
- **Large-state-space twins.** For a few protocols whose faithful model has a state
  space too large for exact MC AND for which no meaningful abstraction lowers it (wide
  sequence/packet-number spaces, deep caches, many concurrent queues), the example ALSO
  ships a `full.rfs` - the un-simplified, real-width model - that is analyzed with
  `rfs mc-ref --mode smt-bounded --bound N` (a k-step safety proof) and `--mode fuzz`
  (random testing), NOT exact MC. This shows RFS can analyze and check very large
  specifications where adequate simplification is impossible. The twins: `tcp_full_states`
  (32-bit sequence numbers + retransmit buffer), `quic_loss_recovery` (full packet-number
  space + congestion state), `dns_resolver` (a large TTL cache - the cache content IS the
  state), `mqtt_qos_session` (full 16-bit packet-id space), and `nvme_controller` (many IO
  queue pairs with deep CQ/SQ).
- A shared README template (below) that every example follows, extended over the
  Phase 17 template with one section per demonstrated RFS mode.
- `docs/user/EXAMPLES.md`: a new "Other examples" section - a table referencing
  all 50 new examples (title | short description + rationale | link to README).
- Training-set update: new entries so the SFT data covers the breadth of real-FSM
  modelling this phase introduces (see PHASE_18_PLAN.md).

## Shared example README template (all 50 follow this)

Every `crates/examples/<fsm_title>/README.md` follows this layout. It extends the
Phase 17 example-README template with a dedicated "RFS modes demonstrated" part -
one subsection per RFS mode the example exercises, each carrying a description +
rationale (why this mode is meaningful for THIS machine).

```markdown
# <fsm_title> - <one-line title>

## What it demonstrates
- RFS functionality exercised (bullets, each linking the relevant docs/user page)
- Rationale: which usage scenario / industrial area this example covers
- The automaton itself: its purpose; the real protocol/standard it is drawn from
  (named, with the relevant clause where appropriate) or why it was constructed;
  states / events / outputs / variables summary

## The machine
- mermaid stateDiagram of the FSM (always)
- mermaid diagram of the property/monitor structure when properties are involved
- a transition table when it adds clarity

## RFS modes demonstrated
### Specification & static analysis (`rfs validate`, `rfs info`)
<what is checked, what the info report shows, rationale>
### Model checking - in-process (`rfs mc-ref`: exact / fuzz / smt-bounded)
<which modes and why; the POSITIVE run (PASS complete) and the NEGATIVE run -
the introduced error, the pinned counterexample trace or diagnostic, the fix,
and the re-run PASS>
### Model checking - generated binaries (rust + c, `--threads`/`--fast`)
<both backends, parity note, rationale for using the production-identical engine>
### Production code generation (rust std / no_std, c; property monitors; adapters)
<what is generated and compiled, warning-free bar, rationale>
### Testing & fuzzing (`rfs test` unit / fuzz)   [where applicable]
### Diagrams (`rfs diagram`)                     [where applicable]

## Files layout
| File | Role |
|---|---|

## Build / run / test
Exact commands + scripts + prerequisites (toolchain activation, RFS_BIN, external tools).

## Expected results
- Pinned output excerpts the user should see
- What to pay attention to (invariants held, bounds, dropped inputs, ...)
- Negative examples: what the counterexample trace shows, why the spec fails, and
  the fix (the corrected spec or guard change + the re-run result)
```

## The `docs/user/EXAMPLES.md` "Other examples" table

A new section titled **"Other examples"** is appended to `docs/user/EXAMPLES.md`,
after the existing example walkthroughs. It carries ONE row per new example:

| Title | Short FSM description + rationale (why selected, what it demonstrates) | README |
|---|---|---|
| `<fsm_title>` | <one or two sentences: what the machine is, the industrial area / standard it comes from, and which RFS functionality the example showcases> | `[README](../../crates/examples/<fsm_title>/README.md)` (link pattern; `<fsm_title>` is replaced per row) |

All 50 rows are added; the section intro states that these are real-world FSMs
modelled to demonstrate the full RFS workflow (specify -> analyze -> model-check
positive/negative -> generate production code in both backends), complementing the
language-feature specs in `examples/valid/` and the integration-workflow examples
from Phase 17.

## Group A - Protocol FSMs (networking / transport / storage / application)

Each protocol example models the real state machine at its discrete level
(states + sequence numbers / timers / windows as bit-vector vars + guarded
transitions + emitted frames), then demonstrates the full RFS workflow on it.
The NEGATIVE scenario for each is a specific, realistic defect (a dropped guard,
a retargeted transition, a missing timer) whose counterexample or diagnostic is
pinned, followed by the one-line fix and re-run proof - the Phase 17 negative-
example discipline applied to a real protocol.

### A1. `tcp_full_states` - the full TCP connection state machine
**Description.** The complete TCP connection machine (RFC 793 / RFC 9293):
LISTEN -> SYN_SENT -> SYN_RCVD -> ESTABLISHED, then teardown FIN_WAIT_1 /
FIN_WAIT_2 / CLOSE_WAIT / CLOSING / LAST_ACK / TIME_WAIT. Vars: send/recv
sequence numbers (bit<32>), retransmit timer, RTO. Outputs: SYN / ACK / FIN / RST /
data segments. Distinct from the existing 3-state `tcp_handshake_like` - this is
the teardown + retransmission half.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no-data-after-FIN-without-
ACK and that TIME_WAIT outlives the max segment lifetime (bounded liveness under a
"peer acknowledges" assume); both generated MC backends agree byte-for-byte.
*Negative:* drop the `FIN_WAIT_2 on ACK -> CLOSED` guard so a half-open connection
deadlocks - pinned `reason=deadlock` CE ending in FIN_WAIT_2; fix restores the
transition, re-run PASS. *Production:* rust + c production compile warning-free;
the sequence-number arithmetic is the bit-vector showcase.
**Files layout** (`crates/examples/tcp_full_states/`): `pass.rfs`, `fail.rfs`
(the deadlock variant), `fixed.rfs`, `full.rfs` (large-state-space twin), `run.sh`
(mc-ref + both codegen-mc legs + smt-bounded/fuzz on full.rfs), `captures/` (pinned
text+JSON CEs + the smt-bounded verdict + fuzz transcript), `README.md`.
**Large-state-space variant (`full.rfs`).** The faithful model uses full 32-bit send/recv sequence numbers plus a retransmission buffer - a state space far too large for exact MC, and narrowing the sequence width would destroy the wrap-around/reordering behavior that is the point of the example (no meaningful abstraction lowers it). `full.rfs` is therefore checked with `rfs mc-ref --mode smt-bounded --bound N` (proves no data loss / no reorder within N steps) and `--mode fuzz --runs R --seed S` (hunts for wrap-around and retransmission violations exact MC cannot reach). Captures: the smt-bounded verdict (pass-within-bound / fail / unknown) + a pinned fuzz transcript. README gains an "Model checking - large state space (smt-bounded + fuzz)" subsection explaining why exact MC is infeasible and what each mode contributes.
**README plan.** stateDiagram of all 11 states; RFS-mode sections for validate/info,
mc-ref exact (+ the deadlock negative), generated MC rust+c parity, production
rust+c; results explain the sequence-number invariants and the TIME_WAIT bound.

### A2. `tls13_handshake` - TLS 1.3 handshake state machine
**Description.** TLS 1.3 (RFC 8446): ClientHello -> ServerHello -> key exchange ->
Finished, with abort paths on version/cipher mismatch. Vars: negotiated cipher,
handshake-transcript hash. Outputs: the handshake messages.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the handshake reaches
Finished or aborts cleanly (no half-open state) and no application data is emitted
before Finished (ordering safety). *Negative:* allow a `ChangeCipherSpec`-era
downgrade by removing the downgrade-protection guard - pinned property-violation CE
on the "no app data before Finished" prop; fix re-adds the guard. *Production:* rust
+c production; the transcript-hash var demonstrates wide bit-vector use.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** handshake message-sequence diagram + stateDiagram; RFS-mode
sections for mc-ref exact (positive ordering proof + negative downgrade CE),
generated MC parity, production rust+c.

### A3. `quic_loss_recovery` - QUIC loss recovery & congestion control
**Description.** QUIC (RFC 9000) connection-level machine: packet-number space, RTT
estimation, PTO timer, congestion window, retransmission of ack-eliciting packets.
Vars: cwnd, ssthresh, rtt, pto_counter (bit-vectors). Outputs: packets, acks.
**RFS usage scenarios.** *Positive:* mc-ref exact + a "network delivers" assume prove
every ack-eliciting packet is acknowledged or retransmitted within bounded PTOs
(bounded liveness) and cwnd never underflows. *Negative:* remove the PTO-backoff
increment so the timer never grows - pinned liveness-violation (lasso) CE showing an
infinite retransmit loop; fix restores the backoff. *Production:* rust+c production;
the RTT/cwnd arithmetic is a strong bit-vector benchmark.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `full.rfs` (large-state-space
twin), `run.sh`, `captures/`, `README.md`.
**Large-state-space variant (`full.rfs`).** The faithful model carries the full packet-number space plus congestion-window / RTT state - too large for exact MC, and coarsening the packet numbers loses the loss-recovery behavior being demonstrated. `full.rfs` is checked with `--mode smt-bounded --bound N` (k-step: every ack-eliciting packet is acknowledged or retransmitted within the bound) and `--mode fuzz` (random loss patterns to hunt for a stuck retransmit loop). Captures: the smt-bounded verdict + a pinned fuzz transcript. README gains the large-state-space subsection.
**README plan.** stateDiagram + a cwnd/PTO variable-flow diagram; RFS-mode sections
for mc-ref exact (bounded-liveness proof + the lasso negative), generated MC rust+c,
production rust+c.

### A4. `dhcp_client` - DHCP client state machine
**Description.** DHCP (RFC 2131): INIT -> SELECTING -> REQUESTING -> BOUND ->
RENEWING -> REBINDING -> (back to INIT). Vars: lease time, T1/T2 timers, offered
address. Outputs: DISCOVER / OFFER / REQUEST / ACK / NAK.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the client always holds a
valid lease or is actively re-acquiring (no silent expiry - bounded liveness) and
never double-binds (one address at a time - mutual exclusion). *Negative:* let T1
fire without transitioning to RENEWING - pinned deadlock CE in BOUND; fix adds the
T1->RENEWING transition. *Production:* rust+c production; timer arithmetic showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 7 states + lease-timer diagram; RFS-mode
sections for mc-ref exact (lease-liveness + no-double-bind + deadlock negative),
generated MC parity, production rust+c.

### A5. `dns_resolver` - DNS resolver / query state machine
**Description.** A recursive resolver's per-query machine: query -> (retry with next
nameserver) -> timeout -> response validation -> cache. Vars: retry count, TTL cache
entry, recursion flag. Outputs: queries, cached answers.
**RFS usage scenarios.** *Positive:* mc-ref exact proves every query completes or
times out within N retries (bounded liveness) and no answer is served from an
expired cache entry (safety). *Negative:* remove the TTL-expiry check so a stale
entry is served - pinned property-violation CE on "no stale answer"; fix re-adds the
expiry guard. *Production:* rust+c production; the retry/TTL counters are the vars.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `full.rfs` (large-state-space
twin), `run.sh`, `captures/`, `README.md`.
**Large-state-space variant (`full.rfs`).** The faithful model holds a LARGE TTL cache - many entries with distinct keys/TTLs - and the cache content IS the state; abstracting it to a single entry removes the staleness/collision behavior being demonstrated. `full.rfs` is checked with `--mode smt-bounded --bound N` (k-step: no answer served from an expired entry within the bound) and `--mode fuzz` (random query/expiry sequences to hunt for a stale-answer or cache-collision violation). Captures: the smt-bounded verdict + a pinned fuzz transcript. README gains the large-state-space subsection.
**README plan.** stateDiagram + cache-entry lifecycle diagram; RFS-mode sections for
mc-ref exact (retry-liveness + no-stale-answer + negative), generated MC parity,
production rust+c.

### A6. `arp_resolution` - ARP resolution state machine
**Description.** Per-entry ARP (RFC 826): INCOMPLETE -> (pending retransmit) ->
RESOLVED, plus aging to deletion and gratuitous-ARP update. Vars: retry counter, age
timer, resolved MAC. Outputs: ARP request / reply.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no packet is forwarded on a
stale/expired entry (safety) and resolution completes under a "responder replies"
assume (bounded liveness), with at most one pending request per target (no storm).
*Negative:* drop the retransmit-limit so an unresolvable entry loops forever - pinned
liveness CE; fix adds the limit->deletion transition. *Production:* rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the per-entry machine; RFS-mode sections for mc-ref
exact (no-stale-forward + resolution-liveness + no-storm negative), generated MC
parity, production rust+c.

### A7. `stp_port_states` - Spanning Tree Protocol port states
**Description.** STP (IEEE 802.1D) port machine: DISCOVERING -> LISTENING -> LEARNING
-> FORWARDING, plus BLOCKING for backup ports; root-bridge election via BPDU
comparison. Vars: bridge/root IDs, port cost, forward-delay/hold timers. Outputs:
BPDUs.
**RFS usage scenarios.** *Positive:* mc-ref exact proves exactly one root bridge
(mutual exclusion), no port is FORWARDING on a loop (loop-free invariant), and
convergence happens within bounded timer ticks (bounded liveness). *Negative:* allow
a backup port to forward while the designated port is also forwarding - pinned
property-violation CE on the loop-free prop; fix keeps the backup BLOCKING.
*Production:* rust+c production; the BPDU-comparison guards are the showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram + a small topology diagram showing root/backup ports;
RFS-mode sections for mc-ref exact (single-root + loop-free + convergence + negative),
generated MC parity, production rust+c.

### A8. `bgp_fsm` - BGP peer finite state machine
**Description.** BGP (RFC 4271): Idle -> Connect -> Active -> OpenSent -> OpenConfirm
-> Established, with NOTIFICATION back to Active/Idle. Vars: BGP identifier, hold
timer, keepalive count. Outputs: OPEN / UPDATE / KEEPALIVE / NOTIFICATION.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no UPDATE is processed before
Established (ordering safety) and the session reaches Established or returns to Idle
(no stuck Connect/Active - bounded liveness under a "peer responds" assume).
*Negative:* accept an UPDATE in OpenConfirm by removing the state guard - pinned
property-violation CE on "no UPDATE before Established"; fix re-adds the guard.
*Production:* rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 6 states; RFS-mode sections for mc-ref exact
(ordering + no-stuck-session + negative), generated MC parity, production rust+c.

### A9. `ospf_neighbor` - OSPF neighbor state machine
**Description.** OSPF (RFC 2328): Down -> Attempt -> Init -> TwoWay -> ExStart ->
Exchange -> Loading -> Full; the DD packet sequence-number handshake. Vars: DD
sequence number, master/slave role, exchange state. Outputs: Hello / LSA / DD packets.
**RFS usage scenarios.** *Positive:* mc-ref exact proves neighbors reach Full or drop
to Down (no permanent ExStart - bounded liveness), DD sequence numbers stay continuous
(no desync/duplicate - safety), and exactly one master per adjacency (mutual exclusion).
*Negative:* let both sides believe they are master by removing the role-resolution
guard - pinned property-violation CE on "single master"; fix adds the tie-break.
*Production:* rust+c production; the DD sequence arithmetic is the showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram + a master/slave DD-exchange diagram; RFS-mode sections
for mc-ref exact (reach-Full-liveness + seq-continuity + single-master negative),
generated MC parity, production rust+c.

### A10. `sip_call_signaling` - SIP call signaling state machine
**Description.** SIP (RFC 3261) invite transaction + dialog: INVITE -> 100/180 ->
200 OK -> ACK; BYE teardown; re-INVITE modify; CANCEL. Vars: call-ID, branch, dialog
state. Outputs: the SIP messages.
**RFS usage scenarios.** *Positive:* mc-ref exact proves every INVITE receives a final
response (bounded liveness) and no dialog is orphaned (BYE always closes - safety).
*Negative:* drop the CANCEL handling so a cancelled invite still transitions to
Established - pinned property-violation CE on "no Established after CANCEL"; fix adds
the CANCEL->terminated transition. *Production:* rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the invite transaction + dialog; RFS-mode sections for
mc-ref exact (final-response-liveness + no-orphan + CANCEL negative), generated MC
parity, production rust+c.

### A11. `mqtt_qos_session` - MQTT session & QoS delivery machine
**Description.** MQTT (RFC 8223 / OASIS): CONNECT -> CONNACK, then PUBLISH with the
QoS0/1/2 handshakes (PUBACK; PUBREC/PUBREL/PUBCOMP), SUBSCRIBE/SUBACK, DISCONNECT.
Vars: packet-id space, session state. Outputs: the control packets.
**RFS usage scenarios.** *Positive:* mc-ref exact proves QoS2 = exactly-once (no
duplicate AND no loss - a strong safety property), QoS1 = at-least-once, and no PUBREL
without a matching PUBREC (ordering). *Negative:* allow a duplicate PUBLISH to be
delivered twice by removing the packet-id dedup guard - pinned property-violation CE on
"exactly-once"; fix re-adds the dedup. *Production:* rust+c production; this is a
flagship delivery-guarantee example.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `full.rfs` (large-state-space
twin), `run.sh`, `captures/`, `README.md`.
**Large-state-space variant (`full.rfs`).** The faithful model uses the full 16-bit packet-id space - exactly-once at scale - which is too large for exact MC, and shrinking the id space changes the duplicate/loss behavior being demonstrated. `full.rfs` is checked with `--mode smt-bounded --bound N` (k-step: no duplicate delivery, no loss within the bound) and `--mode fuzz` (random publish/ack sequences to hunt for a QoS2 exactly-once violation). Captures: the smt-bounded verdict + a pinned fuzz transcript. README gains the large-state-space subsection.
**README plan.** stateDiagram of the QoS2 handshake + a packet-id lifecycle diagram;
RFS-mode sections for mc-ref exact (exactly-once + at-least-once + no-PUBREL-without-
PUBREC + duplicate negative), generated MC parity, production rust+c.

### A12. `coap_retransmit` - CoAP confirmable exchange & retransmission
**Description.** CoAP (RFC 7252): confirmable (CON) vs non-confirmable (NON) exchanges,
CON -> ACK, retransmit with exponential backoff, RESET on token mismatch. Vars: token,
retransmit counter, timeout. Outputs: CON / NON / ACK / RESET.
**RFS usage scenarios.** *Positive:* mc-ref exact proves a confirmable request gets an
ACK or is abandoned after bounded retries (bounded liveness), no response is sent to a
mismatched token (safety), and the backoff never exceeds its max. *Negative:* remove the
retry cap so a lost ACK loops forever - pinned liveness CE; fix adds the cap->abort.
*Production:* rust+c production; a compact UDP-reliability showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the CON/ACK/retransmit machine + backoff diagram;
RFS-mode sections for mc-ref exact (ack-or-abort-liveness + no-mismatch-response +
retry-cap negative), generated MC parity, production rust+c.

### A13. `ppp_lcp_ipcp` - PPP link & IP control protocol negotiation
**Description.** PPP (RFC 1661/1332): LCP dead -> req-sent -> ack-rcvd/ack-sent -> open,
then IPCP negotiation; PAP/CHAP auth sub-state. Vars: negotiated options, auth state.
Outputs: Configure-Req/Ack/Nak/Reject, auth frames.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no data (IPCP) flows before LCP
is open (ordering safety), the link opens with a consistent option set or closes (no
half-negotiated state), and auth must succeed before IPCP. *Negative:* allow IPCP to open
while LCP is still ack-sent by removing the phase guard - pinned property-violation CE on
"no IPCP before LCP open"; fix re-adds the guard. *Production:* rust+c production; a
multi-phase negotiation showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of LCP + IPCP phases; RFS-mode sections for mc-ref exact
(phase-ordering + consistent-options + auth-before-IPCP negative), generated MC parity,
production rust+c.

### A14. `iscsi_login` - iSCSI login & session state machine
**Description.** iSCSI (RFC 7143 / SPC-4): login nop-in/out exchange -> full-feature
phase (negotiate) -> operational phase; logout + session recovery. Vars: session/target
IDs, negotiation keys. Outputs: SCSI PDU login operations.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no I/O is issued before the
operational phase (ordering safety), a clean single logout leaves no dangling session, and
recovery re-establishes the same session (idempotence). *Negative:* allow I/O in the
full-feature phase by removing the phase guard - pinned property-violation CE on "no I/O
before operational"; fix re-adds the guard. *Production:* rust+c production; a storage-
protocol analog of the PPP staged-negotiation pattern.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the login phases; RFS-mode sections for mc-ref exact
(no-IO-before-operational + clean-logout + recovery negative), generated MC parity,
production rust+c.

### A15. `fibrechannel_login` - Fibre Channel FLOGI/PLOGI login machine
**Description.** Fibre Channel (FC-FS): NOP -> F_LOGI -> P_LOGI -> logged-in, plus RSCN
registration and logout. Vars: domain/node/port IDs. Outputs: the login frames.
**RFS usage scenarios.** *Positive:* mc-ref exact proves at most one logged-in state per
port (mutual exclusion), no duplicate concurrent logins, and logout returns cleanly to NOP.
*Negative:* allow a second P_LOGI while already logged-in by removing the state guard -
pinned property-violation CE on "single logged-in"; fix re-adds the guard. *Production:*
rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the FLOGI/PLOGI machine; RFS-mode sections for mc-ref
exact (single-logged-in + no-duplicate + clean-logout negative), generated MC parity,
production rust+c.

### A16. `nvme_controller` - NVMe controller & queue lifecycle
**Description.** NVMe (NVM Express): controller reset -> initialized -> enabled; admin +
IO submission/completion queue lifecycle; asynchronous event requests. Vars: queue-pair
indices, CQ/SQ heads/tails. Outputs: admin/IO commands, completions.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no I/O is submitted before the
controller is enabled (ordering safety), every submission eventually gets a completion
(bounded liveness under a "device completes" assume), and a clean shutdown flushes all
queues. *Negative:* allow an IO submission in the initialized (not-enabled) state by
removing the enable guard - pinned property-violation CE on "no IO before enabled"; fix
re-adds the guard. *Production:* rust+c production; queue-index arithmetic is the
bit-vector showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `full.rfs` (large-state-space
twin), `run.sh`, `captures/`, `README.md`.
**Large-state-space variant (`full.rfs`).** The faithful model carries MANY IO queue pairs with deep submission/completion queues - many concurrent entities whose scheduling is the point; reducing it to a single queue loses that behavior. `full.rfs` is checked with `--mode smt-bounded --bound N` (k-step: no I/O before enabled, every submission completes within the bound) and `--mode fuzz` (random submit/complete interleavings across queues to hunt for a lost or out-of-order completion). Captures: the smt-bounded verdict + a pinned fuzz transcript. README gains the large-state-space subsection.
**README plan.** stateDiagram of the controller states + a queue-pair diagram; RFS-mode
sections for mc-ref exact (no-IO-before-enabled + completion-liveness + shutdown-flush
negative), generated MC parity, production rust+c.

### A17. `sata_link_power` - SATA link & power state machine
**Description.** SATA (ACS Link Power Management): Active / Partial / DevSlumber /
Slumber, with H2D/D2H cross-port power transitions and wake. Vars: link power state,
device power state. Outputs: SMP / SET-WRITE-FREQ commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no command is issued while in a
low-power state without a wake (safety), entry/exit are symmetric (no getting stuck in
Slumber - bounded liveness), and host/device power states stay consistent. *Negative:*
allow a command in Slumber by removing the wake-before-command guard - pinned
property-violation CE on "no command while asleep"; fix re-adds the guard. *Production:*
rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the link/power states + a host/device cross-port diagram;
RFS-mode sections for mc-ref exact (no-command-while-asleep + symmetric-entry-exit +
stuck-Slumber negative), generated MC parity, production rust+c.

### A18. `usb_device_link` - USB device & link state machine
**Description.** USB 2.0/3.x device: Default -> Addressed -> Configured -> Suspend ->
Recovery; link-level U0/U1/U2 with resume. Vars: device address, configuration value, LPM
state. Outputs: SETUP / Data / SO tokens. Distinct from the existing `usb_control_transfer`
(which models one transfer, not the device/link lifecycle).
**RFS usage scenarios.** *Positive:* mc-ref exact proves no data flows in Suspend (safety),
resume restores the prior configuration, and device addresses are unique across the bus
(mutual exclusion). *Negative:* allow a data token in Suspend by removing the state guard -
pinned property-violation CE on "no data while suspended"; fix re-adds the guard.
*Production:* rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the device states + a U0/U1/U2 link diagram; RFS-mode
sections for mc-ref exact (no-data-while-suspended + resume-restores + address-uniqueness
negative), generated MC parity, production rust+c.

### A19. `bt_gatt_session` - Bluetooth GATT client/server session machine
**Description.** Bluetooth GATT (Core Spec Vol 3): advertising -> connecting -> connected;
GATT services discovery, then read/write/notify per characteristic; L2CAP channels. Vars:
connection interval, attribute-handle table. Outputs: LL / GATT PDUs.
**RFS usage scenarios.** *Positive:* mc-ref exact proves notifications are sent only to
subscribed clients (safety), no write reaches an unmapped characteristic, and connection
parameters stay within spec bounds. *Negative:* allow a notification to an unsubscribed
client by removing the subscription guard - pinned property-violation CE on "notify only
when subscribed"; fix re-adds the guard. *Production:* rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the connection + a GATT read/write/notify diagram; RFS-
mode sections for mc-ref exact (notify-only-subscribed + no-unmapped-write + param-bounds
negative), generated MC parity, production rust+c.

### A20. `kerberos_auth` - Kerberos authentication state machine
**Description.** Kerberos (RFC 4120): AS-REQ/AS-REP (obtain TGT) -> TGS-REQ/TGS-REP (obtain
service ticket) -> AP-REQ/AP-REP; ticket renewal. Vars: timestamps, ticket expiry, replay
cache. Outputs: the KRB messages.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no service access without a valid
unexpired TGT (ordering + safety), clock skew is bounded (requests outside the window are
rejected), and no replayed authenticator is accepted (replay-cache safety). *Negative:*
accept a service request with an expired TGT by removing the expiry guard - pinned
property-violation CE on "no access with expired TGT"; fix re-adds the guard. *Production:*
rust+c production; timestamp arithmetic + staged credential chaining is the showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the AS/TGS/AP exchanges + a ticket-lifetime diagram; RFS-
mode sections for mc-ref exact (valid-TGT-only + clock-skew-bound + no-replay negative),
generated MC parity, production rust+c.

## Group B - Control FSMs (embedded / device / industrial control)

Control examples model the DISCRETE MODE layer of a real controller (the analog
loops are abstracted to threshold guards on sensor vars - RFS models state machines,
not continuous dynamics). Each demonstrates safety interlocks (the core value of
model checking a controller) plus bounded-liveness "the machine always reaches a safe
state" properties, with a negative scenario that disables one interlock.

### B1. `blc_motor_foc` - BLDC motor FOC mode machine
**Description.** The mode layer of a brushless-DC field-oriented controller:
standstill -> precharge -> rotor-alignment -> field-oriented-running -> braking ->
fault. The current loops are abstracted to threshold guards on electrical angle and
current magnitude. Vars: angle estimate, current magnitude, fault flags. Outputs: PWM
enable, phase commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves safe-stop latches on
overcurrent/overtemperature (safety), no phase drive during a fault, and alignment
completes before running (ordering). *Negative:* clear the overcurrent fault flag
immediately instead of latching it - pinned property-violation CE on "safe-stop
latches"; fix makes the fault latch until a reset. *Production:* rust+c production; a
power-electronics mode machine.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 6 modes + an angle/current threshold diagram; RFS-
mode sections for mc-ref exact (safe-stop-latch + no-drive-in-fault + alignment-ordering
negative), generated MC parity, production rust+c.

### B2. `bms_protection` - Battery Management System protection machine
**Description.** A battery pack manager: idle -> charging -> full -> discharging ->
balancing -> protection/fault. Vars: state-of-charge estimate, per-cell voltage, pack
temperature. Outputs: charge/discharge FET commands, balance enable.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the pack never charges above
Vmax or discharges below Vmin (hard safety), thermal shutdown latches until cooldown
(no re-entry while hot), and balancing runs only when cells are within a delta window.
*Negative:* disable the over-voltage guard so charging continues past Vmax - pinned
property-violation CE on "never above Vmax"; fix re-enables the guard. *Production:*
rust+c production; threshold-guard heavy - an ideal RFS safety showcase.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 6 states + a cell-voltage/temperature guard diagram;
RFS-mode sections for mc-ref exact (Vmax/Vmin bounds + thermal-latch + balancing-window
negative), generated MC parity, production rust+c.

### B3. `solar_inverter` - Solar inverter & anti-islanding machine
**Description.** A grid-tied inverter: off -> standby -> MPPT-tracking -> grid-tied ->
islanded -> fault; anti-islanding detection on grid loss. Vars: grid voltage/frequency,
DC bus level, MPPT operating point. Outputs: inverter enable, relay commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves anti-islanding trips and opens
the relay on grid loss (safety), no backfeed to the grid (invariant), and islanding
engages only after a confirmed grid fault. *Negative:* remove the anti-islanding trip so
the inverter keeps feeding a de-energized grid - pinned property-violation CE on "no
backfeed"; fix re-adds the trip. *Production:* rust+c production; an energy-system safety
example.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 6 states + a grid-detection diagram; RFS-mode sections
for mc-ref exact (anti-islanding-trip + no-backfeed + confirmed-fault-only negative),
generated MC parity, production rust+c.

### B4. `wind_turbine_ctrl` - Wind turbine controller mode machine
**Description.** A turbine controller: parked -> starting -> ramp-up -> rated ->
pitch-control -> cut-out -> emergency-stop, driven by wind-speed thresholds. Vars: rotor
speed, blade pitch angle, power output. Outputs: pitch/yaw commands, brake.
**RFS usage scenarios.** *Positive:* mc-ref exact proves blade pitch tracks the rotor-
speed bounds (no overspeed - safety), a gust triggers feather within a bounded reaction,
and cut-out is irreversible until re-permitted. *Negative:* let the rotor exceed rated RPM
by widening the pitch-response guard - pinned property-violation CE on "never above rated
RPM"; fix tightens the guard. *Production:* rust+c production; threshold hysteresis + a
strong overspeed invariant.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 7 modes + a wind-speed/pitch response diagram; RFS-
mode sections for mc-ref exact (no-overspeed + bounded-feather + irreversible-cut-out
negative), generated MC parity, production rust+c.

### B5. `hvac_thermostat` - HVAC thermostat control machine
**Description.** A thermostat: idle -> heating -> cooling -> fan-only -> defrost -> fault,
with hysteresis bands around setpoints. Vars: room temperature, setpoint, mode. Outputs:
compressor/fan/valve commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the unit never heats and cools
simultaneously (mutual exclusion), the temperature stays within the dead-band under an
actuator-delays assume (bounded liveness), and defrost engages only when frozen.
*Negative:* allow the heating and cooling valves to open together by removing the mode-
exclusion guard - pinned property-violation CE on "no simultaneous heat+cool"; fix re-adds
the exclusion. *Production:* rust+c production; a classic hysteresis-control teaching example.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`,
`README.md`.
**README plan.** stateDiagram of the 6 modes + a hysteresis-band diagram; RFS-mode sections
for mc-ref exact (no-simultaneous-heat-cool + dead-band-liveness + defrost-only-when-frozen
negative), generated MC parity, production rust+c.

### B6. `traffic_light_ctrl` - Traffic light controller FSM
**Description.** A signal controller: per-phase green -> yellow -> red with an all-red
clearance interval; pedestrian request handling; emergency-vehicle preemption; a flasher
fault mode. Vars: phase index, phase timer, request flags. Outputs: lamp states per approach.
**RFS usage scenarios.** *Positive:* mc-ref exact proves never two conflicting approaches are
green at once (the key safety invariant), an all-red interval is inserted before every phase
change, the cycle time is bounded, and preemption yields a safe gap. *Negative:* remove the
all-red clearance so two diagonal greens overlap on a phase change - pinned property-violation
CE on "no conflicting greens"; fix re-inserts the all-red. *Production:* rust+c production; a
rich mutual-exclusion + sequencing example.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the phase sequence + a signal-plan diagram showing approaches;
RFS-mode sections for mc-ref exact (no-conflicting-greens + all-red-before-change + bounded-
cycle + preemption negative), generated MC parity, production rust+c.

### B7. `vending_machine` - Vending machine control FSM
**Description.** A vending machine: idle -> coin-inserted -> product-selected -> dispensing
-> change-due; out-of-stock and jam branches. Vars: money-in, product price, stock counts.
Outputs: dispense, change, refund.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no product is dispensed without full
payment (safety), the correct change is returned (an arithmetic invariant), and a jam refunds
the money (no loss). *Negative:* dispense on partial payment by removing the price-meets-
money guard - pinned property-violation CE on "no product without full payment"; fix re-adds
the guard. *Production:* rust+c production; money-accounting guards are a great bit-vector
arithmetic exercise.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 5+2 states + a money-flow diagram; RFS-mode sections for
mc-ref exact (no-dispense-underpaid + correct-change + jam-refund negative), generated MC
parity, production rust+c.

### B8. `washing_machine` - Washing machine control FSM
**Description.** A washer: door-locked -> filling -> agitating -> draining -> spinning ->
rinsing -> done; fault branches. Vars: water level, spin speed (ramped), door state. Outputs:
valve/motor/door-lock commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the door unlocks only when the drum is
stopped AND the water is drained (a compound safety guard), no spin occurs with the door open,
and the spin speed ramps within bounds. *Negative:* unlock the door while water remains by
removing the drained-check from the unlock guard - pinned property-violation CE on "door
unlocks only when stopped and drained"; fix re-adds the check. *Production:* rust+c production;
compound-guard + interlock semantics.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 7 states + a water-level/spin-speed diagram; RFS-mode
sections for mc-ref exact (compound-unlock-guard + no-spin-door-open + spin-ramp-bounds
negative), generated MC parity, production rust+c.

### B9. `vehicle_power_modes` - Automotive electrical power mode FSM
**Description.** A vehicle power manager: ACCESSORY -> ON -> RUN -> SLEEP -> HIBERNATE, with
multiple wake sources and load-shedding on brown-out. Vars: ignition state, wake-source
bitmask, battery voltage. Outputs: rail enables, sleep commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no high-load rail is enabled in SLEEP
(safety), a wake from any low state returns to a consistent mode (no stuck state - bounded
liveness), and a brown-out sheds loads in priority order. *Negative:* enable the infotainment
rail in SLEEP by removing the load gate - pinned property-violation CE on "no high-load rail
in sleep"; fix re-adds the gate. *Production:* rust+c production; automotive power-management
interlocks.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 5 power states + a wake-source/rail diagram; RFS-mode
sections for mc-ref exact (no-high-load-in-sleep + consistent-wake + priority-shedding
negative), generated MC parity, production rust+c.

### B10. `atm_transaction` - ATM / POS transaction FSM
**Description.** An ATM: idle -> card-read -> PIN-entry -> authorized -> dispensing/printing;
timeout, jam, and security-lockout branches. Vars: PIN retry count, cash amount, card-present
flag. Outputs: dispense, print, card-return.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no cash is dispensed without
authorization (safety), lockout engages after N bad PINs (a counter invariant), and the card
is returned on every exit path (no swallowed card - a safety/liveness hybrid). *Negative:*
dispense before authorization by removing the auth guard - pinned property-violation CE on
"no cash without authorization"; fix re-adds the guard. *Production:* rust+c production.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the transaction + a PIN-retry/lockout diagram; RFS-mode
sections for mc-ref exact (no-cash-unauthorized + lockout-after-N + always-return-card
negative), generated MC parity, production rust+c.

### B11. `uav_flight_modes` - Drone / UAV flight mode FSM
**Description.** A drone: disarmed -> armed-idle -> takeoff -> hover -> mission -> RTL ->
landing -> failsafe, gated by GPS lock and battery level. Vars: GPS satellite count, battery
level, geofence distance. Outputs: motor throttle, mode commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no arming without GPS+IMU ready
(ordering safety), a signal loss triggers the RTL failsafe within a bounded reaction, and the
geofence is never crossed (invariant). *Negative:* allow arming with fewer than the required
GPS sats by lowering the arm guard - pinned property-violation CE on "no arming without GPS
ready"; fix restores the threshold. *Production:* rust+c production; safety-critical mode gating.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 8 modes + a GPS/battery/geofence guard diagram; RFS-mode
sections for mc-ref exact (no-arm-unready + bounded-failsafe-RTL + geofence-invariant negative),
generated MC parity, production rust+c.

### B12. `cnc_tool_change` - CNC tool-change cycle FSM
**Description.** A machining center: home -> move-to -> engage -> operate/cut -> retract ->
return; the M06 tool-change cycle with joint limits and collision guards. Vars: tool-in-spindle
index, axis positions. Outputs: spindle/axis/turret commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves never two tools are engaged at once
(mutual exclusion), the machine homes before any motion (ordering), and all axes stay within
soft limits (invariant). *Negative:* engage a second tool while one is already in the spindle by
removing the single-tool guard - pinned property-violation CE on "exactly one tool engaged";
fix re-adds the guard. *Production:* rust+c production; machine-tool interlocks + the one-tool
invariant.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the tool-change cycle + an axis-limit diagram; RFS-mode sections
for mc-ref exact (one-tool-only + home-before-motion + soft-limit-invariant negative), generated
MC parity, production rust+c.

### B13. `printer_thermal` - 3D printer print & thermal state machine
**Description.** A printer: idle -> heating bed/nozzle -> leveling -> printing -> paused ->
cooling -> done; a thermal-runaway fault. Vars: nozzle/bed temperatures, target temperatures,
deviation. Outputs: heater/motion commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves a thermal runaway aborts when the
temperature deviates beyond a bound (safety), no motion occurs while overheated (interlock), and
a pause preserves the print position. *Negative:* ignore a rising-temperature deviation by
removing the runaway guard - pinned property-violation CE on "thermal runaway aborts"; fix
re-adds the guard. *Production:* rust+c production; temperature-threshold guards + a hard-abort
invariant.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 7 states + a temperature/deviation diagram; RFS-mode sections
for mc-ref exact (runaway-abort + no-motion-overheated + pause-preserves negative), generated MC
parity, production rust+c.

### B14. `plc_conveyor_seq` - PLC conveyor / sequencing FSM
**Description.** A multi-station line: per station enable -> load-detect -> process -> unload,
with sensor interlocks between stations. Vars: per-station occupancy bits, upstream-clear flags.
Outputs: actuator commands per station.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no station processes while its downstream
is occupied (interlock safety), a load is never lost or duplicated across the line (a conservation
invariant), and a pinch-point interlock holds. *Negative:* let a station process into an occupied
downstream by removing the clear-check - pinned property-violation CE on "no process into
occupied downstream"; fix re-adds the check. *Production:* rust+c production; multi-station mutual
exclusion + conservation.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of one station + a line-topology diagram showing the interlocks; RFS-
mode sections for mc-ref exact (no-process-into-occupied + load-conservation + pinch-interlock
negative), generated MC parity, production rust+c.

### B15. `watchdog_reset` - Watchdog / reset controller FSM
**Description.** A watchdog: running -> window-violation -> timeout-reset; a brown-out reset path.
Vars: feed timestamp, window bounds, supply voltage. Outputs: reset assertion.
**RFS usage scenarios.** *Positive:* mc-ref exact proves reset asserts only on a genuine timeout or
brown-out (no spurious reset while feeding in-window - safety), and the feed window is enforced
(feeding too early or too late both fault). *Negative:* accept an out-of-window feed by widening the
window guard - pinned property-violation CE on "no spurious reset / window enforced"; fix restores
the window. *Production:* rust+c production; a precise valid-feed-window invariant.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the watchdog + a feed-window timing diagram; RFS-mode sections for
mc-ref exact (no-spurious-reset + window-enforced + brown-out-only negative), generated MC parity,
production rust+c.

## Group C - Other FSMs (consensus / workflow / hardware-digital / security / UI / AI)

These cover state machines that are not network protocols or physical controllers but
are central in software, hardware, and human-facing systems. They showcase RFS on
mutual-exclusion-over-time (consensus), atomicity (2PC), resilience (breakers/limiters),
digital coherence (caches/links/arbiters), security flows, deployment safety, and
behavior/UI machines.

### C1. `raft_election_log` - Raft leader election & log replication FSM
**Description.** Raft (Ongaro & Ousterhout): follower / candidate / leader roles; heartbeat,
request-vote, and append-entry RPCs. Vars: current term, vote-granted flag, last log index.
Outputs: the RPCs. A flagship distributed-safety machine.
**RFS usage scenarios.** *Positive:* mc-ref exact proves at most one leader per term (the core
safety property - mutual exclusion over time), a committed entry is never lost (log-append
invariant), and an available node eventually becomes leader (leader-completeness bounded
liveness under a quorum assumption). *Negative:* let two nodes both believe they won a term by
removing the majority-vote guard - pinned property-violation CE on "at most one leader per
term"; fix re-adds the majority check. *Production:* rust+c production; the strongest
distributed-safety teaching example in the set.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the three roles + a term/vote diagram; RFS-mode sections for
mc-ref exact (single-leader-per-term + committed-never-lost + leader-completeness negative),
generated MC parity, production rust+c.

### C2. `two_phase_commit` - Two-phase commit coordinator/participant FSM
**Description.** 2PC: coordinator + participant states in-doubt -> prepared -> commit/abort,
with timeout recovery. Vars: decision, prepared flag, timeout counter. Outputs: prepare /
commit / abort messages.
**RFS usage scenarios.** *Positive:* mc-ref exact proves all-or-nothing (no participant commits
unless the coordinator commits - atomicity), and an in-doubt participant always resolves
(bounded liveness under a "coordinator recovers" assume). *Negative:* let a participant commit
on its own after a coordinator timeout by removing the coordinator-decision guard - pinned
property-violation CE on "no commit without coordinator commit"; fix re-adds the guard.
*Production:* rust+c production; classic atomic-commit safety.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of coordinator + participant + a prepare/commit message diagram;
RFS-mode sections for mc-ref exact (all-or-nothing + in-doubt-resolves negative), generated MC
parity, production rust+c.

### C3. `circuit_breaker` - Circuit breaker resilience FSM
**Description.** A circuit breaker: closed -> open (on a failure threshold) -> half-open (after a
cooldown) -> closed (on a successful probe) / back to open (on a failed probe). Vars: failure
count, cooldown timer, probe state. Outputs: allow/block decision.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no request passes while open (safety),
half-open admits only a bounded number of probes, and the breaker closes only on a successful
probe (no flapping without evidence). *Negative:* let requests pass while open by removing the
state gate - pinned property-violation CE on "no request while open"; fix re-adds the gate.
*Production:* rust+c production; a clean threshold+timer resilience machine.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 3 states + a failure-count/cooldown diagram; RFS-mode
sections for mc-ref exact (no-pass-while-open + bounded-probes + close-only-on-success negative),
generated MC parity, production rust+c.

### C4. `token_bucket_limiter` - Token-bucket rate limiter FSM
**Description.** A rate limiter: refill -> consume -> burst -> throttle; token accounting over
time. Vars: token count, bucket capacity, last-refill timestamp. Outputs: admit/delay decision.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the admitted rate never exceeds the
configured rate over any window (the core invariant), the burst is bounded by capacity, and no
client starves under a fairness assume (bounded liveness). *Negative:* allow consumption beyond
the token count by removing the balance guard - pinned property-violation CE on "never exceed
rate"; fix re-adds the guard. *Production:* rust+c production; arithmetic + window reasoning.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 4 states + a token-refill/consume diagram; RFS-mode sections
for mc-ref exact (rate-bound + burst-capacity + no-starvation negative), generated MC parity,
production rust+c.

### C5. `cache_writeback` - Cache controller write-back FSM
**Description.** A cache line controller: idle -> read-hit / read-miss-fetch / write-allocate /
writeback-dirty / invalidate; coherence actions. Vars: line state (M/E/S/I), dirty bit, tag.
Outputs: bus read/write/invalidate.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no stale read is served after an
invalidate (coherence safety), a dirty line is written back before it is replaced (no data loss),
and state transitions respect the coherence rules. *Negative:* replace a dirty line without
writing it back by removing the writeback-before-replace guard - pinned property-violation CE on
"dirty line written back before replacement"; fix re-adds the guard. *Production:* rust+c
production; a digital-cache coherence model.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the line states + a coherence-action diagram; RFS-mode sections
for mc-ref exact (no-stale-read-after-invalidate + writeback-before-replace + rule-respecting
negative), generated MC parity, production rust+c.

### C6. `pcie_ltssm` - PCIe link training state machine (LTSSM)
**Description.** The PCIe LTSSM (PCIe Base Spec): Detect -> Polling -> Configuration -> L0, with
Recovery and L1/L2 low-power; width/speed negotiation. Vars: link speed, link width, training-
token count. Outputs: TS1/TS2 ordered sets.
**RFS usage scenarios.** *Positive:* mc-ref exact proves the link reaches L0 or disables cleanly
(no stuck training - bounded liveness), no TLP data flows during Recovery (safety), and the
negotiated speed is symmetric on both directions. *Negative:* allow data TLPs during Recovery by
removing the state gate - pinned property-violation CE on "no data in recovery"; fix re-adds the
gate. *Production:* rust+c production; a real, well-specified digital protocol FSM.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the LTSSM + a speed/width negotiation diagram; RFS-mode sections
for mc-ref exact (reach-L0-or-disable + no-data-in-recovery + symmetric-speed negative), generated
MC parity, production rust+c.

### C7. `uart_i2c_line` - UART / I2C bit-level line FSM
**Description.** A UART TX/RX line machine: idle -> start-bit -> data-bits -> stop-bit, with
framing-error detection; plus an I2C controller start/ack/data/stop sequence. Vars: bit index,
shift register, ack flag. Outputs: line levels / assembled bytes.
**RFS usage scenarios.** *Positive:* mc-ref exact proves a framing error is detected on a bad stop
bit (safety), no byte is emitted mid-transmission (frame atomicity), and I2C ack/nak is handled
correctly. *Negative:* emit a byte before the stop bit by removing the frame-complete guard -
pinned property-violation CE on "no byte mid-frame"; fix re-adds the guard. *Production:* rust+c
production; bit-level protocol sequencing.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the UART line states + a bit-timing diagram; RFS-mode sections for
mc-ref exact (framing-error-detect + frame-atomicity + i2c-ack negative), generated MC parity,
production rust+c.

### C8. `bus_arbiter` - Bus arbiter / interrupt grant FSM
**Description.** An arbiter: request -> grant -> service -> deassert, with priority arbitration
across requestors. Vars: request bitmask, grant index, priority level. Outputs: the grant signal.
**RFS usage scenarios.** *Positive:* mc-ref exact proves at most one grant is active at any time
(the key mutual-exclusion invariant), a higher priority is granted fairly, and no requestor
starves under a bounded-request assume (liveness). *Negative:* grant two requestors at once by
removing the single-grant guard - pinned property-violation CE on "at most one grant"; fix re-adds
the guard. *Production:* rust+c production; a canonical arbiter safety+liveness pair.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the arbiter + a priority-request diagram; RFS-mode sections for
mc-ref exact (single-grant + fair-priority + no-starvation negative), generated MC parity,
production rust+c.

### C9. `oauth2_authcode` - OAuth2 authorization-code flow FSM
**Description.** OAuth2 (RFC 6749) authorization-code grant: redirect -> authorize -> code-
exchange -> token -> refresh -> revoke; nonce/state anti-CSRF. Vars: state/nonce, authorization
code, token expiry. Outputs: redirects, tokens.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no token is issued without a valid code
AND a matching state (CSRF safety), a refresh rotates the token (no reuse), and revocation is
honored on subsequent use. *Negative:* issue a token on a mismatched state by removing the state-
check guard - pinned property-violation CE on "no token without matching state"; fix re-adds the
check. *Production:* rust+c production; staged credential exchange with a CSRF/replay invariant.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the authorization-code flow + a state/nonce diagram; RFS-mode
sections for mc-ref exact (no-token-without-state + refresh-rotates + revocation-honored negative),
generated MC parity, production rust+c.

### C10. `fw_conntrack` - Stateful firewall connection-tracking FSM
**Description.** A conntrack entry: NEW -> ESTABLISHED -> RELATED, plus INVALID; per-protocol
state and NAT. Vars: the 5-tuple, connection state, idle timeout. Outputs: allow/drop decision.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no reply is accepted for a NEW flow from
outside (stateful-filtering safety), RELATED connections (e.g. ICMP errors) are tracked to their
parent, and idle connections are reaped by timeout (bounded liveness). *Negative:* accept an
outside-initiated reply to a NEW flow by removing the direction guard - pinned property-violation
CE on "no outside reply to NEW"; fix re-adds the guard. *Production:* rust+c production; stateful-
firewall semantics.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the connection states + a 5-tuple/NAT diagram; RFS-mode sections
for mc-ref exact (no-outside-reply-to-NEW + related-tracked + idle-reaped negative), generated MC
parity, production rust+c.

### C11. `login_lockout` - Authentication / login lockout FSM
**Description.** An auth machine: idle -> authenticating -> success / failure-count -> locked ->
session-active -> token-refresh; an MFA step. Vars: failure count, lockout timer, session expiry.
Outputs: accept/reject/lock decision.
**RFS usage scenarios.** *Positive:* mc-ref exact proves lockout engages after N failures (a counter
invariant) and releases only after the cooldown, a session expires and cannot be refreshed past its
window (bounded validity), and MFA must complete before success. *Negative:* allow a refresh past
the session window by removing the expiry guard - pinned property-violation CE on "no refresh past
expiry"; fix re-adds the guard. *Production:* rust+c production; auth-state safety + timers.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the auth states + a failure-count/lockout diagram; RFS-mode sections
for mc-ref exact (lockout-after-N + no-refresh-past-expiry + mfa-before-success negative), generated
MC parity, production rust+c.

### C12. `update_rollout` - Software update / deployment rollout FSM
**Description.** An updater: idle -> download -> verify-signature -> stage -> canary -> promote/
rollback -> reboot. Vars: image hash, canary health, stage index. Outputs: flash/boot commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no boot from an unverified (bad-signature)
image (integrity safety), a canary failure triggers rollback (no bad promote), and the A/B switch is
atomic (no bricked state). *Negative:* boot an image with a bad signature by removing the verify gate -
pinned property-violation CE on "no boot from unverified image"; fix re-adds the gate. *Production:*
rust+c production; integrity + safe-rollback invariants.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the rollout stages + a canary/promote-rollback diagram; RFS-mode
sections for mc-ref exact (no-unverified-boot + rollback-on-canary-fail + atomic-switch negative),
generated MC parity, production rust+c.

### C13. `media_player` - Media player UI state machine
**Description.** A player: stopped -> buffering -> playing -> paused -> seeking -> ended -> error,
with buffer-underrun handling. Vars: buffer level, play position, target position. Outputs: render /
pause / seek commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no seek is honored during a buffer underrun
(safety), a pause preserves the exact position (a state-preservation invariant), and an error is always
recoverable to stopped (no dead state). *Negative:* honor a seek during an underrun by removing the
buffer guard - pinned property-violation CE on "no seek during underrun"; fix re-adds the guard.
*Production:* rust+c production; a familiar UI machine with real invariants.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the 7 states + a buffer-level diagram; RFS-mode sections for mc-ref
exact (no-seek-underrun + pause-preserves-position + error-recoverable negative), generated MC parity,
production rust+c.

### C14. `game_ai_fsm` - Game AI / character controller FSM
**Description.** A game AI: idle -> patrol -> alert -> chase -> attack -> flee -> dead, stimulus-
driven; plus a platformer controller (grounded / airborne / jumping / falling / wall-slide). Vars:
health, distance-to-target, on-ground flag. Outputs: move/attack/flee actions.
**RFS usage scenarios.** *Positive:* mc-ref exact proves never two mutually-exclusive behaviors are
active at once (mutual exclusion), the AI recovers from a stuck state (bounded liveness), and flee
triggers below a health threshold (a guard invariant). *Negative:* allow attack and flee simultaneously
by removing the behavior-exclusion guard - pinned property-violation CE on "no two exclusive behaviors";
fix re-adds the exclusion. *Production:* rust+c production; discrete behavior-machine mutual exclusion +
recovery.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the AI states + a stimulus/response diagram; RFS-mode sections for
mc-ref exact (no-two-exclusive-behaviors + stuck-recovery + flee-threshold negative), generated MC
parity, production rust+c.

### C15. `approval_workflow` - Approval / business-process workflow FSM
**Description.** A workflow: draft -> submitted -> in-review -> approved/rejected -> escalated ->
executed; delegation and SLA timers. Vars: approver, SLA deadline, delegation depth. Outputs: approve /
reject / execute commands.
**RFS usage scenarios.** *Positive:* mc-ref exact proves no execution without an approval (ordering
safety), escalation fires on an SLA breach (bounded liveness), and exactly one terminal state is reached
(no double-execution). *Negative:* execute a draft that was never approved by removing the approval gate -
pinned property-violation CE on "no execution without approval"; fix re-adds the gate. *Production:* rust+c
production; business-process invariants + timers.
**Files layout:** `pass.rfs`, `fail.rfs`, `fixed.rfs`, `run.sh`, `captures/`, `README.md`.
**README plan.** stateDiagram of the workflow states + an SLA/delegation diagram; RFS-mode sections for
mc-ref exact (no-execution-unapproved + escalation-on-sla-breach + single-terminal negative), generated MC
parity, production rust+c.

## Deliverables
- 50 new example projects under `crates/examples/` (one per FSM above), each with a
  `pass.rfs`, a `fail.rfs` (the introduced defect), a `fixed.rfs`, a `run.sh` driving the
  demonstrated RFS modes, pinned `captures/` (text + JSON counterexamples / diagnostics), and a
  template-conformant `README.md`. A curated subset is additionally a cargo-member crate to
  showcase production integration (adapters / no_std / property monitors).
- `tests/example_projects_test.rs`: new legs driving every new example (positive PASS, negative
  pinned CE/diagnostic + fix PASS, both-backends production compile warning-free).
- `docs/user/EXAMPLES.md`: the new "Other examples" section - a 50-row table (title | short
  description + rationale | README link).
- Training-set update: new entries covering real-FSM modelling breadth (protocol/control/other),
  regenerated snippets + aggregate.
- `Cargo.toml` workspace members updated for any cargo-member examples.
- `docs/roadmap/phase_18/PHASE_18_STATUS.md`: the phase completion report.

## Exit criteria
- [ ] All 50 example projects exist under `crates/examples/`, each with pass/fail/fixed specs,
      a run script, pinned captures, and a template-conformant README.
- [ ] Every example's POSITIVE model-checking run is PASS complete (mc-ref exact at minimum) and
      its NEGATIVE run produces the pinned counterexample trace or diagnostic; the fix re-runs to
      PASS. Pinned by test legs (the CE/diagnostic excerpt is asserted, not just a non-zero exit).
- [ ] Every example's production code generation compiles warning-free in BOTH backends
      (`rfs codegen rust production` + `rfs codegen c production`).
- [ ] The five large-state-space twins (`tcp_full_states`, `quic_loss_recovery`,
      `dns_resolver`, `mqtt_qos_session`, `nvme_controller`) each ship a `full.rfs` checked via
      `rfs mc-ref --mode smt-bounded` (k-step verdict pinned) and `--mode fuzz` (pinned
      transcript) - NOT exact MC - with a README subsection explaining why exact MC is
      infeasible for that machine.
- [ ] The curated cargo-member subset builds and tests green as workspace members.
- [ ] `docs/user/EXAMPLES.md` carries the "Other examples" section with all 50 rows, each linking
      to a resolvable README.
- [ ] Training set updated for the new breadth; snippets + aggregate regenerated; `cargo
      test-training` green.
- [ ] Doc gates green: `cargo test --test docs_md_test` (now covering the 50 new READMEs) and
      `cargo test --test dev_docs_test`.
- [ ] Full verification at or above the Phase 17 close-out baseline: `cargo test` >= 584,
      `--workspace`, `--features smt`, generated-code (moves only if new valid snippets are added -
      recorded, not a regression) - fmt clean, clippy at baseline.
- [ ] `TASKS.md` entry appended; `CURRENT_STATE.md` phase-18 row updated; `PHASE_18_STATUS.md`
      written per FORMATTING.md section 4.

## Non-goals
- No language-semantics, grammar, IR, codegen or MC behaviour change - this is an examples +
  docs + training-data phase (the new specs exercise the EXISTING surface; they add no features).
- No continuous-dynamics modelling: control machines are modelled at their discrete MODE layer
  with threshold guards; the analog loops are abstracted away.
- No restructuring of existing `examples/valid/` teaching specs or Phase 17 integration examples.
- No GPU training run - `training.py` is untouched; only the data it consumes changes.
