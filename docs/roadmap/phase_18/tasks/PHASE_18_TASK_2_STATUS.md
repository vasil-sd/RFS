# Phase 18, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (example specs + test legs only; no `src/` change).

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | changed | 10 new pinned test legs (one per FSM A11-A20): run.sh exit 0, pinned PASS summary (state count), pinned FAIL reason/label/step + the CE defect step line (copied from the real capture), pinned JSON label, both-backend parity (3x pass summary), both-backends production compile proofs; mqtt + nvme additionally pin the smt-bounded verdict (PASS WITHIN BOUND 12) and the fuzz transcript (runs=200 bound=5 completed=200); coap additionally pins its --mode fuzz transcript |
| `docs/llm/TEST_STRUCTURE.md` | changed | +31 inventory rows: 10 for this task's legs + 21 repairing task 1A's missing rows (6 fns + SmtEmit in smt_plan.rs, 2 fns in smt_emit.rs, 2 fns + SfCtx in ref_mc_smt.rs, 9 fns in ref_mc_smt_test.rs); llm-budget raised 1430 -> 1432 |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `crates/examples/{mqtt_qos_session,coap_retransmit,ppp_lcp_ipcp,iscsi_login,fibrechannel_login,nvme_controller,sata_link_power,usb_device_link,bt_gatt_session,kerberos_auth}/README.md` | new | 10 template-conformant walkthroughs (PHASE_18.md shared template): What it demonstrates / The machine (mermaid stateDiagram-v2) / RFS modes demonstrated (one subsection per mode with rationale) / Files layout / Build-run-test / Expected results with the negative CE walkthrough + fix proof; mqtt + nvme carry the large-state-space subsection |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/{mqtt_qos_session,coap_retransmit,ppp_lcp_ipcp,iscsi_login,fibrechannel_login,nvme_controller,sata_link_power,usb_device_link,bt_gatt_session,kerberos_auth}/` | new | 10 protocol-FSM example projects: `pass.rfs` + `fail.rfs` (one defect) + `fixed.rfs` (= pass.rfs) + `run.sh` + `captures/` (pinned text + JSON CEs, smt verdicts, fuzz transcripts) each; mqtt_qos_session + nvme_controller additionally ship `full.rfs` (large-state-space twins) + `captures/full-smt.txt` + `captures/full-fuzz.txt`; coap_retransmit additionally ships `captures/fuzz-pass.txt` |

## Changed functionality

None - no command behaviour changed; new example projects + test legs only.

## Removed functionality

None.

## New functionality

- 10 real-world protocol FSMs (application / storage / wireless / security)
  modelled at their discrete level, each demonstrating the full RFS workflow:
  specify -> analyze (`rfs validate`/`rfs info`) -> model-check positive (PASS
  complete) and negative (a pinned counterexample) -> generate production code
  in both backends.
- Two large-state-space twins (`mqtt_qos_session`: full 16-bit packet-id space;
  `nvme_controller`: 4 IO queue pairs x bit<8> deep queues = 2^32 configs)
  checked via `--mode smt-bounded --bound 12` (PASS WITHIN BOUND) +
  `--mode fuzz --runs 200 --seed 7` (clean), NOT exact MC.
- `coap_retransmit` additionally demonstrates `rfs mc-ref --mode fuzz` on the
  passing spec (200 seeded runs, clean).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` (10 new fns) | changed | one pinned test leg per FSM - pins the failures so a regression in CE format or codegen is caught; suite grows 24 -> 34 legs |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root, all binaries) | OK - 604 pass / 0 fail (594 baseline + 10 new legs) |
| `cargo test --test example_projects_test` (34) | OK |
| `cargo test --test docs_md_test` (5) | OK - the 10 new READMEs are in its file set and parse/render clean |
| `cargo test --test dev_docs_test` (9) | OK - after the TEST_STRUCTURE budget raise |
| `rfs-dev inventory audit` | EXACT - SRC 1023 fns / 215 types, TEST 1084 fns / 35 types, zero per-file diffs |
| `cargo clippy --all-targets` | OK - no errors; 71 warning lines = the 64-line Phase 17 baseline + 7 from task 1A's smt_plan/smt_emit code (useless_format x4, type_complexity x3); zero in this task's files |
| `cargo fmt --check` | OK |

## Found problems/issues/mismatches
- Task 1A/1B protocol debt found and repaired: both tasks were committed
  (`b172fd4`, `5fc85fc`) without their `PHASE_18_TASK_{1A,1B}_STATUS.md` files,
  without TASKS.md entries, and with 21 inventory rows missing (the string-masked
  audit failed on them). The 21 rows were added in this commit; the STATUS files
  and TASKS.md entries remain to be written by a follow-up.
- `kerberos_auth` could not be modelled with bit<8> time vars: four independent
  bit<8> vars give ~2^34 reachable configurations, and the rfs binary in this
  sandbox runs under a seccomp filter (Seccomp: 2) that caps its address space at
  ~1 GB - exact MC died with "memory allocation of 34078736 bytes failed" at
  ~950 MB RSS. Rescaled to bit<3> (TGT valid 6 ticks, ST 4 ticks), noted in the
  README per the task file's simplify-and-note rule.
- RFS transition priorities: the HIGHER number wins (LANGUAGE.md line 258,
  spec 39). An early kerberos draft put accept at priority 1 / reject at 2, so
  the reject path masked the expiry-guard defect and fail.rfs passed; swapping
  the priorities fixed it.
- `always (input_is(E) -> next (f))` checks f at the sample AFTER the E step
  (the Delayed monitor carries prev_ant one sample forward) - a sata draft using
  this form for "Wake lands in Active" was violated by the transition following
  the Wake; the plain implication `input_is(Wake) -> state == Active` is the
  correct form for entry-state assertions.
- Sandbox environment: `/dev/null` is a regular file here and was transiently
  overwritten with an ELF binary, which corrupted cargo's clippy target-probe
  stdin ("unknown start of token" / "not valid UTF-8"); truncating it restored
  clippy. Also, the mmdc/puppeteer cache needed the documented `~/.cache/puppeteer`
  symlink to the persistent `/workspace/host/.cache/puppeteer` (TOOLCHAIN.md 6.4)
  before diagram_render_parity_test would pass.

## Other important findings/information
- Formula patterns do not bind variables in Phase 1 (validator: "formula input
  patterns do not bind variables") - `output_has(Deliver(P(i))) -> i == last`
  is illegal. Per-id exactly-once (mqtt) is therefore checked via a machine-
  maintained duplicate-detector var (`was_dup`) that the property asserts never
  fires; the dns_resolver witness-var pattern from task 1 correction 1a remains
  the established technique for this class of invariant.
- smt-bounded rejects specs with assumptions (v1 scope, shared error) - the
  nvme twin carries no assumption (the pass spec's env_tick is dropped in
  full.rfs), matching the task-1 twins.
- Every state must handle an assumed input or paths deadlock under the
  assumption (coap: Idle/Aborted needed Timeout self-loops for env_timeout).
- An overlapping-guard defect transition that shadows a cap transition by source
  order silently disables the cap (coap's first fail.rfs draft kept both Timeout
  transitions and passed) - remove, don't just unguard.
