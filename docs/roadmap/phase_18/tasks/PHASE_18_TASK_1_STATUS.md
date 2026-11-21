# Phase 18, Task 1 - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_1.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (example specs + test legs only; no `src/` change).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/llm/TEST_STRUCTURE.md` | changed | 10 new inventory rows (one per FSM test leg), same-commit correspondence rule |
| `docs/roadmap/phase_18/PHASE_18.md` | changed | the EXAMPLES.md table template's `[README](.../<fsm_title>/README.md)` rendered as an inline code span (link-pattern illustration) so the docs_md_test link resolver no longer chokes on the literal `<fsm_title>` placeholder |
| `docs/roadmap/phase_18/tasks/PHASE_18_TASK_8.md` | changed | same placeholder-link fix in its table-format description |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/{tcp_full_states,tls13_handshake,quic_loss_recovery,dhcp_client,dns_resolver,arp_resolution,stp_port_states,bgp_fsm,ospf_neighbor,sip_call_signaling}/` | new | 10 protocol-FSM example projects: `pass.rfs` + `fail.rfs` + `fixed.rfs` + `run.sh` + `captures/` (pinned text + JSON CE/diagnostic) + template `README.md` each; the three twins (`tcp_full_states`, `quic_loss_recovery`, `dns_resolver`) additionally ship `full.rfs` + `captures/full-smt.txt` + `captures/full-fuzz.txt` |

## Changed functionality

None - no command behaviour changed; new example projects + test legs only.

## Removed functionality

None.

## New functionality

- 10 real-world protocol FSMs (transport / routing / signaling) modelled at their
  discrete level, each demonstrating the full RFS workflow: specify -> analyze
  (`rfs validate`/`rfs info`) -> model-check positive (PASS complete) and negative
  (a pinned counterexample) -> generate production code in both backends.
- Three large-state-space twins (`tcp_full_states`, `quic_loss_recovery`,
  `dns_resolver`) checked via `--mode smt-bounded --bound 12` (k-step safety proof,
  PASS WITHIN BOUND) + `--mode fuzz --runs 200`, NOT exact MC - demonstrating that
  very large specifications can be analyzed where adequate simplification is impossible.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` (10 new fns) | changed | one pinned test leg per FSM: PASS-complete state count, the pinned FAIL reason/label/step + CE defect step (copied from the real capture), the JSON label, both-backend parity, both-backends production compile; the three twins additionally pin the smt-bounded verdict + fuzz transcript |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (24) | OK |
| `cargo test --test docs_md_test` (5) | OK |
| `cargo test --test dev_docs_test` (9) | OK |
| `cargo test` (root workspace, all binaries) | OK |
| `cargo clippy --all-targets` | OK - no errors; warnings at the pre-phase dead-code baseline (my legs reuse existing helpers) |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- The committed `dns_resolver` twin capture `full-smt.txt` held an error (`unknown
  identifier 'k' in expression`): the twin renamed its event parameter in the pattern
  (`R(k)` for field `v`) and used `output_has(...)` in the property - both unsupported by
  smt-bounded. Fixed by using field-name parameters and a plain-boolean snapshot invariant
  (`served`/`last_serve_ttl`); regenerated the capture to PASS WITHIN BOUND 12. Recorded as
  task correction 1a.
- Pre-existing: `docs_md_test` failed on two `<fsm_title>` placeholder links in
  PHASE_18.md / PHASE_18_TASK_8.md (the EXAMPLES.md table template). Fixed by rendering
  them as inline code spans, which the link resolver correctly ignores.
- Pre-existing flakiness noted (not fixed here): `mc_walkthrough`'s pinned
  `captures/fail-rust.json` `peak_rss_kb` drifts by a few KB on each run.sh re-execution
  (non-deterministic RSS). Its test leg does not assert that field, so it is harmless; the
  working-tree drift was reverted to keep this task's change set focused.

## Other important findings/information
- RFS event parameters must be nested record constructors (`Reply(R(mac: bit<8>))`), not
  bare typed fields (`Reply(mac: bit<8>)`) - a bare field triggers "trailing input after
  RFS source". All 10 specs use the record form.
- In a property, `state` refers to the TARGET state of the step (confirmed while authoring
  `bgp_fsm`: an UPDATE echoed on a transition INTO Established does not violate
  `output_has(SendUpdate) -> state == Established`; the defect must emit while staying in a
  non-Established target).
- The smt-bounded twin-authoring constraints (field-name event params + plain-boolean
  properties, no `output_has`) apply to every remaining Phase 18 twin task.
