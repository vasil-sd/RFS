# Phase 18 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation immediately without re-deriving scope.

## Context

- The 50-FSM catalogue with per-FSM detail (description, RFS usage scenarios -
  positive AND negative with an introduced error + fix, files layout, README plan)
  lives in `PHASE_18.md` under Groups A/B/C. This plan only breaks that into tasks;
  it does not re-derive the per-FSM content.
- **Reuse the Phase 17 plain-dir example machinery.** `tests/example_projects_test.rs`
  already has a `run_rfs`-style helper and the flat-dir + plain-filename compilation
  recipe (the generated MC artifacts reference their fsm by codegen-time name, so they
  must be compiled from the example dir with bare filenames). New legs follow that exact
  pattern - no new helper infrastructure is needed.
- **The negative-example discipline (Phase 17 task 6) applies to every FSM.** Each
  `fail.rfs` introduces ONE specific, realistic defect (a dropped guard, a retargeted
  transition, a missing timer/limit). The test leg pins the REAL counterexample trace or
  diagnostic excerpt (reason/label/step + the CE steps, or the validate error kind/line),
  NOT just a non-zero exit. `fixed.rfs` is the one-line correction; the leg asserts it
  re-runs to PASS / validates.
- **The production-compile bar.** Every FSM's spec must compile warning-free in BOTH
  backends via `rfs codegen rust production` and `rfs codegen c production` (the
  generated-C compiler-agnosticism guarantee: `-Wall` clean under gcc, and the clang leg
  where availability-gated). This is asserted by a test leg per FSM.
- **Control machines are modelled at their discrete MODE layer.** The analog loops
  (current control, PID, temperature dynamics) are abstracted to threshold guards on
  sensor vars. Do NOT attempt continuous-dynamics modelling - RFS is a state-machine
  tool. This is stated in each control FSM's README.
- **generated-code caveat.** If any new example spec is also fenced into the training set
  as a valid snippet, `cargo test-generated-code` will grow (the generator path-dedups,
  so every `snippets/valid/*.rfs` is its own module). That is expected and recorded - it is
  NOT a codegen regression (all pre-existing modules stay byte-identical). The Phase 17
  close-out established this exact reasoning.
- **Cargo-member subset.** Three FSMs are additionally built as workspace-member crates to
  showcase production integration on a real protocol/controller: `tcp_full_states` (rfs-
  adapters: channel/async/callback/minimal over a TCP-like session), `bms_protection`
  (`--with-property-monitors`: the debug-gated monitor catches an injected over-voltage
  violation; release is zero-cost), and `uart_i2c_line` (`--no-std`: compiled in a genuinely
  `#![no_std]` crate). These reuse the plain-dir spec content via their `build.rs`.
- **Large-state-space twins.** Five FSMs whose faithful model is too large for exact MC and
  has no meaningful abstraction (`tcp_full_states`, `quic_loss_recovery`, `dns_resolver`,
  `mqtt_qos_session`, `nvme_controller`) ALSO ship a `full.rfs` checked via
  `rfs mc-ref --mode smt-bounded --bound N` (a k-step safety proof) and `--mode fuzz`
  (random testing), NOT exact MC - showing RFS can analyze very large specifications where
  adequate simplification is impossible. Covered in tasks 1 and 2; the per-FSM detail is in
  PHASE_18.md.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Protocol FSMs A1-A10 (transport/routing/signaling) | `crates/examples/{tcp_full_states,tls13_handshake,quic_loss_recovery,dhcp_client,dns_resolver,arp_resolution,stp_port_states,bgp_fsm,ospf_neighbor,sip_call_signaling}/` (new: pass.rfs, fail.rfs, fixed.rfs, run.sh, captures/, README.md each); `tests/example_projects_test.rs` (change: +10 legs) | Write the 10 specs (real state machines per PHASE_18.md Group A), the defect variants + fixes, run scripts, pinned captures, template READMEs; add a test leg per FSM (positive PASS + negative pinned CE + fix PASS + both-backends production compile). Rationale: the networking/transport core of the catalogue. |
| 2. | Protocol FSMs A11-A20 (application/storage/wireless/security) | `crates/examples/{mqtt_qos_session,coap_retransmit,ppp_lcp_ipcp,iscsi_login,fibrechannel_login,nvme_controller,sata_link_power,usb_device_link,bt_gatt_session,kerberos_auth}/` (new, same file set); `tests/example_projects_test.rs` (change: +10 legs) | Same pattern for the application/storage/wireless/security protocols. Rationale: completes Group A; mqtt exactly-once and kerberos credential-chaining are flagship safety examples. |
| 3. | Control FSMs B1-B8 (power/energy/building/consumer) | `crates/examples/{blc_motor_foc,bms_protection,solar_inverter,wind_turbine_ctrl,hvac_thermostat,traffic_light_ctrl,vending_machine,washing_machine}/` (new, same file set); `tests/example_projects_test.rs` (change: +8 legs) | Same pattern for the power/energy/building/consumer controllers (discrete-mode layer, threshold guards). Rationale: the industrial-control half; bms_protection and traffic_light_ctrl are the safety-interlock showcases. |
| 4. | Control FSMs B9-B15 (automotive/financial/aerospace/machining/safety) | `crates/examples/{vehicle_power_modes,atm_transaction,uav_flight_modes,cnc_tool_change,printer_thermal,plc_conveyor_seq,watchdog_reset}/` (new, same file set); `tests/example_projects_test.rs` (change: +7 legs) | Same pattern for the remaining controllers. Rationale: completes Group B; uav failsafe and watchdog window are bounded-liveness showcases. |
| 5. | Other FSMs C1-C8 (consensus/resilience/hardware-digital) | `crates/examples/{raft_election_log,two_phase_commit,circuit_breaker,token_bucket_limiter,cache_writeback,pcie_ltssm,uart_i2c_line,bus_arbiter}/` (new, same file set); `tests/example_projects_test.rs` (change: +8 legs) | Same pattern for the consensus/resilience/hardware machines. Rationale: raft single-leader-per-term and 2PC atomicity are the strongest distributed-safety examples; pcie_ltssm/uart_i2c_line/bus_arbiter cover digital hardware. |
| 6. | Other FSMs C9-C15 (security/deployment/UI/AI/workflow) | `crates/examples/{oauth2_authcode,fw_conntrack,login_lockout,update_rollout,media_player,game_ai_fsm,approval_workflow}/` (new, same file set); `tests/example_projects_test.rs` (change: +7 legs) | Same pattern for the security/deployment/UI/AI/workflow machines. Rationale: completes Group C; oauth2 CSRF and update_rollout integrity are the security showcases. |
| 7. | Cargo-member production-integration subset | `crates/examples/{tcp_full_states,bms_protection,uart_i2c_line}/{Cargo.toml,build.rs,src/lib.rs}` (new); `Cargo.toml` (change: +3 workspace members); `tests/example_projects_test.rs` (change: +3 nested-cargo legs) | Promote the three chosen FSMs to workspace-member crates: tcp_full_states drives a TCP-like session over rfs-adapters (channel/async/callback/minimal via `--with-protocol-fsm`); bms_protection builds with `--with-property-monitors` and its debug-gated monitor catches an injected over-voltage violation (release twin zero-cost); uart_i2c_line compiles in a genuinely `#![no_std]` crate. Rationale: demonstrates production codegen + integration on REAL machines, not just the Phase 17 teaching specs. |
| 8. | `docs/user/EXAMPLES.md` "Other examples" section | `docs/user/EXAMPLES.md` (change: new section + 50-row table); `docs/user/README.md` (change: pointer if it indexes EXAMPLES.md content) | Add the "Other examples" section with one row per FSM (title \| short description + rationale \| README link), per PHASE_18.md. Rationale: users discover the catalogue from the user docs; the links must resolve (docs_md_test gate). |
| 9. | Training-set update for real-FSM breadth | `training/<cat>/rfs_training.jsonl` (change: new entries); `training/<cat>/snippets/` + `all_rfs_training.jsonl` (regenerated) | Add entries so the SFT data covers modelling REAL protocols/controllers (how to express sequence numbers/timers/windows as bit-vector vars, how to state the safety invariants, how to read the CEs on a real machine) - a few per cluster (protocol / control / other), referencing the new examples. Regenerate snippets + aggregate; `cargo test-training` green. Rationale: the training set must reflect the new breadth (KB.md section 4.6). |
| 10. | Close-out | `docs/roadmap/phase_18/PHASE_18_STATUS.md` (new); `TASKS.md` (change: append); `docs/roadmap/CURRENT_STATE.md` (change: phase-18 row + header date); `START.md` / `BUILD.md` (change: doc-map + test-count rows if shifted) | Run the full verification matrix, write the phase status report per FORMATTING.md section 4, sync every state record, commit. Rationale: the phase exits only on a fully green matrix + recorded counts. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 1A | SMT-bounded property-language extension + renamed event parameters | new | Extend `--mode smt-bounded` (in-process `ref_mc_smt` AND both generated SMT binaries, parity-pinned) toward exact-mode property support: (a) `input_is` / `output_has` / `next` enabled BY DEFAULT - they are cheap because the k-step unrolling already computes per-step first-match `fire_t` predicates and free input-class/payload step variables, so `output_has(X)(i) = OR_t fire_t(i) AND emit-matches(t,X)` reuses existing machinery (only `SmtTrans.emits`, currently dropped, must be carried); (b) the genuinely expensive class - `bounded_eventually[N]` under a universal anchor, `until`, and temporal-under-temporal nesting (O(k*N)/O(k^2) quantifier-free expansions) - goes behind a new `--full-properties` CLI flag on BOTH smt surfaces; without it such properties are OMITTED with a per-property warning naming them and the reason (greatly affects solver time/memory at large bounds), and the summary line gains `properties_omitted=M` only when M > 0 (existing pins stay byte-identical); (c) renamed event parameters (`R(k)` for IR field `rkey`) supported UNCONDITIONALLY - the parser records patterns positionally and the SMT encoder currently builds its guard context from IR field names only, discarding binding names; fix by aliasing each `Bind` name to its positionally-zipped declared field in `smt_plan.rs` (zero formula-size change, no flag). Assumptions/on_error stay rejected (v1 scope). Rationale: user-requested extension of smt-bounded up to exact-mode semantics with the performance-sensitive part flag-gated; unblocks natural property writing in all remaining Phase 18 twins (supersedes correction 1a's plain-boolean workaround for future tasks). |
| 1B | `dns_resolver` twin: use the natively supported syntax | new | After 1A lands, rewrite `crates/examples/dns_resolver/full.rfs` to drop the `served`/`last_serve_ttl` witness-variable workaround and state the safety property NATIVELY as `always (output_has(ServeAnswer) -> ttl > 0)`, and demonstrate renamed event parameters end-to-end (`Reply(R(rk)) where rk == key`) to prove the binding-alias path on a real twin. Regenerate `captures/full-smt.txt` + `full-fuzz.txt`, update the README's large-state-space subsection (output_has now directly supported; witness encoding retired), and re-pin the test leg if any asserted excerpt moved. Rationale: the twin is the flagship demonstration that smt-bounded now checks exact-mode-style properties on a large spec; keeping the workaround would misrepresent the new capability. |

## Other information

- **Execution order:** tasks 1-6 are independent of each other (each just adds examples +
  test legs) but are done sequentially; task 7 depends on the specs from tasks 1/3/5 existing;
  task 8 depends on all READMEs (tasks 1-7); task 9 depends on the examples being present;
  task 10 depends on everything.
- **Spec-authoring guidance for the implementing session:** each `pass.rfs` must be a COMPLETE,
  VALID module that model-checks to PASS complete under mc-ref exact (verify by running it before
  pinning). Keep state counts in the tens-to-low-thousands range so exact MC is fast (like the
  existing benchmarks: http2_stream ~434 states, sliding_window 94). Use bit-vector vars for
  sequence numbers/timers/windows/counters; use `assume` for environment contracts ("peer
  acknowledges", "network delivers", "responder replies"); use `bounded_eventually[N]` for the
  liveness properties. The `fail.rfs` defect must produce a DETERMINISTIC, pin-able counterexample
  (a specific reason/label/step + CE steps) - avoid defects that only show up under fuzz.
- **Captures:** store the pinned text AND JSON counterexample/diagnostic excerpts in each
  example's `captures/` dir (the Phase 17 pattern), and have the test leg assert against them so a
  future codegen/MC change that alters the report is caught.
- **Risk:** 50 specs is a large authoring surface. If a particular real protocol proves too large
  to model cleanly at the discrete level within the state budget, simplify it (fewer states / a
  coarser abstraction) and note the simplification in its README - do not drop the example.
