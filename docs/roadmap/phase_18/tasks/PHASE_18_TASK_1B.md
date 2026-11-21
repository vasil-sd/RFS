# Phase 18, Task 1B - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Rewrite the `dns_resolver` large-state-space twin (`full.rfs`) to use the syntax that
task 1A made natively supported: state the safety property as
`always (output_has(ServeAnswer) -> ttl > 0)` instead of the `served`/`last_serve_ttl`
witness-variable workaround, and demonstrate renamed event parameters end-to-end
(`Reply(R(rk)) where rk == key`). Rationale: the twin is the flagship demonstration that
smt-bounded now checks exact-mode-style properties on a large spec; keeping the
workaround would misrepresent the new capability.

## Context
- Task 1A (this phase's correction) landed `output_has` support in smt-bounded mode and
  renamed-parameter binding aliases. Correction 1a's constraint ("twins must use
  field-name event parameters and plain-boolean invariants") is SUPERSEDED for all
  remaining twin work - but the frozen task-1 files are not rewritten; this supersession
  is recorded here and in 1A's status.
- The current `full.rfs` (written under correction 1a) carries two witness vars
  (`served: bit<1>`, `last_serve_ttl: bit<8>`) set by the serve transition, and the
  property `always (served == 0 || last_serve_ttl > 0)`. The rewrite drops both vars and
  uses the natural property. The state space SHRINKS (two fewer vars) - the smt verdict
  must still be PASS WITHIN BOUND 12, and the fuzz transcript (`runs=200 bound=5
  completed=200`, seed 7) must stay clean; re-verify and regenerate both captures.
- Renamed-parameter demonstration: the twin's `Reply` event declares field `rkey`; write
  the pattern as `Reply(R(rk))` and the guard as `rk == key` - proving the binding alias
  on a real, checked spec (this exact form was the one that failed under correction 1a).
- The test leg (`dns_resolver_no_stale_serve_and_the_large_twin` in
  `tests/example_projects_test.rs`) asserts `PASS WITHIN BOUND 12` and `runs=200 bound=5
  completed=200` from run.sh's output (which cats the regenerated captures). If either
  excerpt moves after the rewrite, re-pin it from the REAL capture (never hand-write).
- run.sh regenerates `captures/` from scratch; after running it, revert any
  non-deterministic drift (none expected here - smt verdicts and fuzz transcripts are
  deterministic for fixed bound/seed/runs).

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1A. | SMT-bounded property-language extension | provides output_has + renamed-parameter support that this rewrite uses |

### Affected source files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/dns_resolver/full.rfs` | change | drop the witness vars; natural `output_has` property; renamed `Reply(R(rk))` pattern + `rk == key` guard |
| `crates/examples/dns_resolver/captures/full-smt.txt` | change | regenerated smt-bounded verdict (PASS WITHIN BOUND 12) |
| `crates/examples/dns_resolver/captures/full-fuzz.txt` | change | regenerated fuzz transcript (runs=200 bound=5 completed=200, seed 7) |

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/dns_resolver/README.md` | change | large-state-space subsection: output_has now checked directly (witness encoding retired); renamed-parameter note; the mermaid diagram + Files layout updated if the var list changed |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/example_projects_test.rs` | change | re-pin the dns_resolver leg's twin excerpts ONLY if they moved (from the real capture) |

## Planned actions

### 1B.1 - Rewrite `full.rfs`
Remove `served`/`last_serve_ttl` and their assignments; replace the property with
`always (output_has(ServeAnswer) -> ttl > 0)`; rename the Reply pattern binding to `rk`
(`Reply(R(rk)) where rk == key`). Keep everything else (16-bit key, 8-bit TTL, the
simplified-state machine) identical so the twin remains the same large-state-space model.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/dns_resolver/full.rfs` | change | natural property + renamed parameter; witness vars dropped |

### 1B.2 - Regenerate captures + verify verdicts
Run `./run.sh` (needs the smt-enabled binary: `target/debug/rfs-smt` or `RFS_SMT_BIN`).
Confirm: smt-bounded still reports PASS WITHIN BOUND 12 on the natural property; fuzz
still completes 200/200 clean with seed 7. Regenerate `captures/full-smt.txt` +
`full-fuzz.txt`; revert any non-deterministic drift.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/dns_resolver/captures/full-smt.txt` | change | regenerated verdict capture |
| `crates/examples/dns_resolver/captures/full-fuzz.txt` | change | regenerated fuzz transcript |

### 1B.3 - README + test-leg sync
Update the README's large-state-space subsection (output_has checked directly; witness
encoding retired; renamed-parameter demonstration noted) and its Files-layout/var
summary. Re-run `cargo test --test example_projects_test dns_resolver`; re-pin any moved
excerpt from the real capture.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/dns_resolver/README.md` | change | subsection + layout updates |
| `tests/example_projects_test.rs` | change | re-pin moved twin excerpts only if needed |

## Task deliverables
- `dns_resolver/full.rfs` using the native `output_has` property + a renamed event
  parameter, checked by smt-bounded (PASS WITHIN BOUND 12) and fuzz (clean transcript).
- Regenerated captures, updated README, test leg green.

## Task exit criteria
- [ ] `full.rfs` carries NO witness vars; the property is `always (output_has(ServeAnswer) -> ttl > 0)`; the Reply pattern uses a renamed binding (`rk`) with a matching guard.
- [ ] `rfs mc-ref full.rfs --mode smt-bounded --bound 12` reports PASS WITHIN BOUND 12 (capture regenerated).
- [ ] `rfs mc-ref full.rfs --mode fuzz --runs 200 --seed 7` completes clean (capture regenerated).
- [ ] README large-state-space subsection reflects the native property + renamed parameter; Files layout accurate.
- [ ] `cargo test --test example_projects_test` green (dns leg re-pinned from real captures if anything moved); `cargo test --test docs_md_test` green.
- [ ] Full checklist: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Non-goals
- No changes to `pass.rfs`/`fail.rfs`/`fixed.rfs` (the exact-MC trio is untouched).
- No changes to the other twins (tcp/quic) - they may adopt the native syntax in a later
  task if desired; this task scopes to dns_resolver only.
- No test-leg structural change beyond re-pinning moved excerpts.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- If the natural property's smt verdict or the fuzz transcript differs from the pinned
  excerpts, investigate BEFORE re-pinning: a changed verdict would indicate an encoding
  bug in task 1A, not a spec problem.
