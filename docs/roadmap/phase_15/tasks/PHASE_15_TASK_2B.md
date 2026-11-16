# Phase 15, Task 2b - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope. Added as plan correction 2b during task 6 (close-out).

## Goal
Make the Rust and C SMT-bounded backends agree across z3 solver versions. Task 2a's
boolean-folding change (`simplify_expr` in `src/lang/ir.rs`, applied in both backends'
guard rendering) introduced a divergence that only manifests with the host z3 5.3.1:
on the fail variant of `smt_bounded_backend_parity` the C backend correctly reports
FAIL (property violated at the initial state) but the Rust backend reports PASS WITHIN
BOUND. Fix it so both backends give identical status/label/step for every spec under
ANY z3 version, then re-run the full canonical + nix sweeps to close out Phase 15.

## Context
- **The failing case (confirmed by task 6):** `tests/smt_bounded_test.rs`
  `smt_bounded_backend_parity`, the "fail" variant (`SMT_FAIL_SRC`). Property
  `flag_or_counter : always (flag == true || counter > 0)` is violated in the INITIAL
  configuration (flag=false, counter=0). Expected: both backends exit 1 with
  `status=fail ... label=flag_or_counter step=0`. Actual at current source + host z3
  5.3.1: C exits 1 (correct), Rust exits 0 (`PASS WITHIN BOUND 3`) - the assertion
  `assert_eq!(rr, rc)` at tests/smt_bounded_test.rs:291 fails.
- **It is a task-2a regression, z3-version-sensitive:** the test PASSES at pre-task-2a
  source (commit 17d22bc) under host z3 5.3.1, and PASSES at current source under nix
  z3 5.3.2 (task 2a's doCheck was 42/42). So task 2a's fold changed the encoding such
  that with z3 5.3.1's solver behaviour the two backends diverge, while with 5.3.2 they
  still agree. Task 2a only verified under nix (5.3.2), missing the host-z3-5.3.1 case.
- **Diagnosis starting points (task 6 did these):** the generated Rust FSM
  (`rfs codegen rust production`) and MC driver (`rfs codegen rust mc --mode
  smt-bounded`) are byte-IDENTICAL between pre-2a and current source for this spec, so
  the divergence is NOT in the rendered Rust text - it is in how the fold (or a shared
  IR-level rewrite) changes the SMT encoding/evaluation that only surfaces with a
  particular solver version. Investigate: (1) whether `simplify_expr` rewrites the
  property or an initial-state guard into a form z3 5.3.1 evaluates differently than
  5.3.2; (2) whether the fold is applied asymmetrically between the Rust and C SMT
  encoders (one folds a term the other leaves, so their z3 queries differ); (3) diff the
  actual z3 assertions each backend emits (add a debug dump or use `--print-script`-style
  introspection) for the fail spec under both backends.
- **The fold must stay a boolean equivalence** (task 2a's invariant - every rewrite
  truth-preserving). The fix is to make the two backends' SMT encodings agree, NOT to
  weaken the test or pin a z3 version. Do not suppress the warning gate or change the
  test's pinned expectations.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2a. | generated C warning-free under nixpkgs | introduced the fold this task fixes; its doCheck (nix z3 5.3.2) is green, so the fix must keep that green too |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Phase close-out | PENDING on this task - the canonical `cargo test --features smt` must reach 559/0 before the phase is COMPLETE |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/lang/ir.rs` or the SMT encoders (`src/codegen/backends/{rust,c}/...`) | change | make both backends' SMT-bounded encoding agree across z3 versions (the exact file depends on the diagnosis) |

## Planned actions

### 1. Diagnose the divergence
Reproduce under the host toolchain (START.md section 4 activation): `cargo test
--features smt --test smt_bounded_test` -> the fail variant fails (rust=0 c=1). Dump
the z3 assertions each backend emits for `SMT_FAIL_SRC` (Rust vs C) and diff them to
find the term where they differ. Confirm the same dump AGREES at pre-task-2a source
(17d22bc worktree) to isolate task 2a's fold as the cause.

### 2. Fix the encoding so both backends agree
Apply the minimal change that makes the Rust and C SMT-bounded encodings emit
equivalent z3 assertions for the initial-state property check, preserving task 2a's
boolean-equivalence invariant and the zero-warning gate. Keep the fix backend-symmetric
(so a future fold cannot re-introduce the skew).

### 3. Re-verify both toolchains
- Host (canonical): `cargo test --features smt` -> **559/0** (the fail variant now
  agrees: both exit 1 with the pinned label/step). Full canonical sweep still green:
  `cargo test` 557/0, `--workspace` 685/0, generated-code 1683/1683, fmt clean, clippy
  at baseline (55 lines / 34 unique).
- Nix: `nix build --impure --expr 'import ./nix/package.nix { }'` (doCheck=true) still
  42/42; `cargo test --features smt` inside `nix develop -f shell.nix` green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| the diagnosed source file(s) | change | restore cross-version backend parity for SMT-bounded mode |

## Task deliverables
- Both backends agree on status/label/step for every `smt_bounded_test` spec under both
  host z3 5.3.1 and nix z3 5.3.2; the zero-warning gate and task 2a's fold invariant hold.

## Task exit criteria
- [ ] `cargo test --features smt` = 559/0 under the canonical (host) toolchain.
- [ ] `smt_bounded_backend_parity` fail variant: both backends exit 1 with
      `status=fail ... label=flag_or_counter step=0`.
- [ ] Full canonical sweep green (557/685/559/1683, fmt clean, clippy at baseline).
- [ ] Nix doCheck build still 42/42; nix `cargo test --features smt` green.
- [ ] Committed per FORMATTING.md §7; task 6 then completes the phase close-out.

## Non-goals
- No weakening of the parity test's pinned expectations.
- No z3 version pinning to mask the divergence.
- No changes to the nix packaging (tasks 2-4's work stays as-is).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- This task unblocks the Phase 15 close-out (task 6), which is otherwise complete:
  tasks 1-5 are committed, the nix-side exit criteria are met, and only the canonical
  `--features smt` count is blocked on this fix.
