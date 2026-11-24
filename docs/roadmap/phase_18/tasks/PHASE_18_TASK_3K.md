# Phase 18, Task 3K - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Root-cause and stabilize the recurring `/dev/null` corruption that breaks long test runs
in this environment. Find the ROOT of the issue if it lies in the toolchain or RFS scripts
(e.g. the example `run.sh` compile-checks writing to `/dev/null`, the test-suite's shared
binary-cache mutex cascading one environment blip into dozens of instant failures); or,
if the issue is a quirk of the unsloth-studio sandboxing that cannot be fixed from inside,
install a STABLE, durable work-around (repo-committed, never lost when `/tmp` is cleared)
so long suites stop being force-re-run. Rationale: this incident has repeatedly forced
re-runs of the large 612/752/634-test matrix, wasting hours; every occurrence so far was
patched ad hoc (truncate, mknod, ephemeral /tmp wrapper) and the next session re-learned it
from scratch.

## Context

### The incident history (gathered from TASKS.md + this conversation's earlier turns)
All occurrences share one signature: `/dev/null` is a REGULAR FILE instead of the char
device, so rustc's target-probe (`rustc - ...`, whose source stdin is redirected off
`/dev/null`) parses the file's content as Rust and dies. Documented occurrences:
- **Stale-stdin probe variant** (TASKS.md ~line 524): "cargo's target-info probe read stale
  session text even with cargo's stdin on `/dev/null` (the probe's stdin is not reliably
  cargo's fd 0 in this sandbox)" - a PERSISTENT variant; worked around with a RUSTC_WRAPPER
  forcing `< /dev/null` for `-` probes, handling both arg shapes (with/without the extra
  compiler-name arg prepended when RUSTC_WORKSPACE_WRAPPER is set).
- **ELF overwrite** (TASKS.md ~lines 2883, 2898): "/dev/null is a regular file that was
  transiently overwritten with an ELF binary, corrupting cargo's clippy probe stdin until
  truncated" - hit TWICE during Phase 18 task 2/3 execution.
- **git-status text** (TASKS.md ~line 2915): "/dev/null was again a regular file holding
  git-status text, breaking the rustc target-probe (E0762); fixed by truncating it +
  RUSTC_WRAPPER=/tmp/rustc-wrap.sh + deleting corrupted target/.rustc_info.json".
- **MID-RUN re-corruption** (observed 2026-11-23, documented in TOOLCHAIN.md §4): long test
  runs corrupt `/dev/null` AGAIN while executing - the example `run.sh` scripts compile
  with `-o /dev/null`, and the sandbox quirk turns the device into a regular file that then
  accumulates written data. The rustc target-probe of any cargo subprocess started after
  that point fails with "couldn't read from stdin, as it did not contain valid UTF-8", and
  in `tests/ref_mc_smt_test.rs` the first such failure POISONS the shared bin-cache mutex,
  so every remaining parity test dies with PoisonError (14 instant failures, 0.11 s).

### Current state at planning time (2026-11-24 baseline)
`/dev/null` is a healthy char device right now (`crw-r--r-- 1 root root 1, 3`,
`file /dev/null` = "character special (1/3)"). The corruption is INTERMITTENT - it appears
at session start sometimes and MID-RUN other times. That intermittency is exactly what
makes it costly: a suite can run for an hour and then cascade-fail at the end.

### The RFS-side contributors (verified by grep during planning)
1. **28 example `run.sh` scripts compile with `-o /dev/null`** - the documented mid-run
   trigger. Two forms, at ~lines 72-75 of each script:
   - `"$RUSTC" --edition 2021 --crate-type lib --emit=metadata "$BUILD/prod-rust-fsm.rs" -o /dev/null && echo "rust production: compiles clean"`
   - `"$GCC" -Wall -c "$BUILD/prod-c-fsm.c" -o /dev/null && echo "c production: compiles warning-free"`
   On a healthy system writing compiler output to `/dev/null` is harmless (data
   discarded); in this sandbox the write appears to be what materializes the device as a
   data-accumulating regular file. `grep -rln "\-o /dev/null" crates/examples/` returns 28
   files (every Group-A/B/C example that has a run.sh with production compile-checks).
2. **`tests/ref_mc_smt_test.rs` BIN_CACHE mutex cascade** (~lines 313-325):
   ```rust
   static BIN_CACHE: std::sync::LazyLock<
       std::sync::Mutex<std::collections::HashMap<String, PathBuf>>,
   > = std::sync::LazyLock::new(|| std::sync::Mutex::new(std::collections::HashMap::new()));

   fn cached_binary(key: &str, build: impl FnOnce() -> PathBuf) -> PathBuf {
       let mut cache = BIN_CACHE.lock().expect("bin cache lock");
       ...
   }
   ```
   If the `build` closure panics while the lock is held (e.g. rustc dies on the corrupted
   probe), the Mutex is POISONED and every subsequent `.lock().expect(...)` in ANY test
   thread panics with PoisonError - one environment blip becomes 14 instant failures.
   The cache itself is sound; only the poison handling is fragile.
3. **Ephemeral work-around** (TOOLCHAIN.md §4): the RUSTC_WRAPPER shim lives in
   `/tmp/rustc-wrap.sh` - lost whenever /tmp is cleared, and NOT exported by any
   persistent mechanism. The activation prefix in START.md §2 / TOOLCHAIN.md §2 heals
   `/dev/null` at shell start but does NOTHING about mid-run re-corruption.

### Root-cause hypotheses to test (the investigation proper)
- **H1 - sandbox /dev handling (unsloth-studio quirk, unfixable from inside):** the
  sandbox's `/dev` is a tmpfs or overlay without persistent device nodes; something in the
  session lifecycle (or a write to the path) materializes `/dev/null` as a regular file.
  Probes: `mount | grep -E " /dev "`, `stat -f /dev` (filesystem type), in a FRESH shell
  run `echo probe > /dev/null && file /dev/null` (does a plain write convert it?),
  `rm /dev/null; mknod /dev/null c 1 3; sleep 1; file /dev/null` (does the mknod'd device
  persist across tool calls? across a new shell?). If writes materialize files and/or the
  device does not persist, H1 is confirmed and the durable work-around (action 3K.4) is
  the deliverable.
- **H2 - RFS scripts are the trigger (fixable from inside):** the `-o /dev/null`
  compile-checks are what convert the device mid-run. Test: after healing `/dev/null`,
  run ONE example's run.sh production-check section and immediately `file /dev/null`.
  If it converts, H2 is confirmed and action 3K.2 (temp-file outputs) removes the trigger
  entirely - possibly making the wrapper unnecessary for suite runs.
- **H3 - toolchain probe behavior (partially fixable):** cargo's target-probe reads source
  from stdin and in this sandbox the probe's stdin is not reliably cargo's fd 0 (TASKS.md
  line 524). Check whether `CARGO_BUILD_JOBS=1` or running cargo with stdin explicitly on
  a regular empty file (`cargo test < /tmp/empty`) changes the failure mode; check whether
  deleting `target/.rustc_info.json` before each long run avoids the stale-probe-cache
  variant. These are work-around refinements, not root fixes.
- **H4 - test-infrastructure cascade (fixable from inside, independent of H1-H3):** even if
  the corruption cannot be prevented, the PoisonError cascade can be - action 3K.3 makes
  one blip degrade ONE test instead of 14. This should be done regardless of which
  hypothesis wins.

### What a "stable work-around" must satisfy (the acceptance bar for H1)
- Repo-committed (NOT /tmp): survives session resets and /tmp clears.
- Zero-effort to activate: exported by the BUILD.md verification checklist and the
  TOOLCHAIN.md §2 activation prefix, so every long run gets it without a human remembering.
- No-op when healthy: must not change compile behavior or outputs when `/dev/null` is a
  proper char device (the existing /tmp shim already has this property - it only rewrites
  the probe's bare `-` source arg and redirects rustc stdin).
- Documented in TOOLCHAIN.md §4 as THE remedy, replacing the "recreate it if /tmp was
  cleared" instruction.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| (none) | - | independent of 3F-3J (disjoint files); no unexecuted dependency |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3I. | Tests for the 3F-3H changes + test-suite adoption | runs the large suite matrix - benefits directly from this task's stabilization (no mid-run corruption, no PoisonError cascade) |
| 10. | Phase close-out | its full-verification matrix run is the longest single suite execution in the phase - most exposed to the incident |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `crates/examples/*/run.sh` (28 files) | change | replace `-o /dev/null` compile-checks with temp-file outputs inside `$BUILD` (deleted after the check) |
| `tests/ref_mc_smt_test.rs` | change | `cached_binary` recovers from a poisoned mutex instead of cascading PoisonErrors |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `TOOLCHAIN.md` | change | §4: record the investigation findings (which hypothesis won) + replace the ephemeral-/tmp-wrapper instruction with the durable remedy; §2 activation prefix: export the wrapper if H1 confirmed |
| `BUILD.md` | change | verification checklist: add the wrapper export (if H1 confirmed) so every long run gets it automatically |
| `START.md` | change (conditional) | §4 activation block: same wrapper export, keeping START/TOOLCHAIN in lockstep |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `scripts/rustc-wrap.sh` (or similar repo path) | new (if H1 confirmed) | the committed RUSTC_WRAPPER shim - the /tmp recipe from TOOLCHAIN.md §4, verbatim logic, stable location |

## Planned actions

### 3K.1 - Run the root-cause probes
Execute the H1-H3 probes from Context in a fresh shell sequence: filesystem type of /dev
(`mount`, `stat -f`), plain-write conversion test (`echo probe > /dev/null; file
/dev/null`), mknod persistence across tool calls and across a new shell, then the H2
test (heal /dev/null, run one example's production compile-checks, re-check
`file /dev/null`). Record each probe's exact output in the task status - this is the
evidence base for which hypothesis won. If `/dev/null` is already corrupted when the task
starts, heal it first (`rm -f /dev/null; mknod /dev/null c 1 3`) and note that the
corruption was present at session start (an H1 data point).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | investigation only; probe outputs recorded in the status file |

### 3K.2 - Remove the RFS-side trigger: temp-file compile-checks in run.sh
In all 28 `crates/examples/*/run.sh`, replace the two `-o /dev/null` forms with
temp-file outputs inside the script's existing `$BUILD` dir, deleted after the check:
- rustc: `--emit=metadata "$BUILD/prod-rust-fsm.rs" -o "$BUILD/.compile-check.rmeta"` ... `rm -f "$BUILD/.compile-check.rmeta"`
- gcc: `-Wall -c "$BUILD/prod-c-fsm.c" -o "$BUILD/.compile-check.o"` ... `rm -f "$BUILD/.compile-check.o"`
Keep the `&& echo "... compiles clean"` success messages byte-identical (the test legs
pin those strings). Verify with `grep -rn "\-o /dev/null" crates/examples/` returning
nothing, and run 2-3 example run.sh scripts end-to-end to confirm the compile-checks still
pass and leave no artifacts. This action is done REGARDLESS of which hypothesis wins -
writing compiler output to a device path is fragile in any sandbox.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `crates/examples/*/run.sh` (28 files) | change (~lines 72-75 each) | `-o /dev/null` -> temp file in $BUILD + rm; removes the documented mid-run corruption trigger |

### 3K.3 - Make the SMT test cache poison-resilient
In `tests/ref_mc_smt_test.rs`, change `cached_binary`'s lock acquisition from
`.lock().expect("bin cache lock")` to
`.lock().unwrap_or_else(std::sync::PoisonError::into_inner)` so a poisoned mutex (from a
build-closure panic caused by an environment blip) degrades to "rebuild the binary for
this one test" instead of cascading PoisonErrors through every remaining parity test.
Add a comment citing this task + the 2026-11-23 incident. Verify: `cargo test --test
ref_mc_smt_test` green (the smt feature build), and a targeted check that a poisoned lock
no longer kills the suite (a small inline unit test of `cached_binary` after manually
poisoning a throwaway mutex is NOT possible on the static - instead, document the behavior
change and rely on the next real incident to prove it; optionally add a `#[cfg(test)]`
helper that exercises the `unwrap_or_else` path on a local Mutex).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/ref_mc_smt_test.rs` | change (~line 318) | poison-resilient lock acquisition - one blip degrades one test, not 14 |

### 3K.4 - Install the durable work-around (if H1 confirmed)
If the probes confirm the sandbox materializes `/dev/null` as a regular file and/or the
mknod'd device does not persist (H1), commit the wrapper: create `scripts/rustc-wrap.sh`
(the exact /tmp recipe from TOOLCHAIN.md §4 - it rewrites the probe's bare `-` source arg
to an empty file and redirects rustc stdin from /dev/null; no-op for normal compiles),
make it executable, and export `RUSTC_WRAPPER=$PWD/scripts/rustc-wrap.sh` in (a) the
TOOLCHAIN.md §2 activation prefix (next to the existing /dev/null heal line) and (b) the
BUILD.md verification checklist's long-run commands, and (c) START.md §4 if it carries its
own copy of the activation block. Update TOOLCHAIN.md §4: record the findings, mark the
wrapper as THE remedy, and retire the "recreate it if /tmp was cleared" instruction. If H1
is NOT confirmed (e.g. H2 alone explains every occurrence), skip the wrapper and instead
record in TOOLCHAIN.md §4 that the run.sh fix (3K.2) removed the trigger - keeping the
heal-line as a session-start safety net.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `scripts/rustc-wrap.sh` | new (conditional on H1) | committed wrapper - survives /tmp clears and session resets |
| `TOOLCHAIN.md` | change (§2, §4) | activation prefix exports the wrapper; §4 records findings + durable remedy |
| `BUILD.md` | change (verification checklist) | long-run commands carry the export automatically |
| `START.md` | change (conditional, §4) | activation block stays in lockstep with TOOLCHAIN.md |

### 3K.5 - Prove the stabilization on a long run
Run the longest available suite execution (`cargo test --features smt` - the 634-test
matrix, or the full `cargo test && cargo test --workspace` pair) and confirm: (a)
`file /dev/null` is still a char device at the END of the run (the mid-run trigger is
gone), (b) zero PoisonError failures, (c) the full matrix green with the same counts as
the 2026-11-23 baseline (612 root / 752 workspace / 634 smt). This is the acceptance proof
that long runs no longer cascade-fail. Record the exact counts + the end-of-run
`file /dev/null` output in the status.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- A recorded root-cause determination (which of H1-H4 won, with probe evidence) in the
  task status file.
- 28 run.sh scripts without `-o /dev/null` (temp-file compile-checks, artifacts cleaned).
- `ref_mc_smt_test.rs` bin-cache that survives a poisoned mutex.
- If H1: a committed `scripts/rustc-wrap.sh` + the activation/checklist exports + the
  TOOLCHAIN.md §4 rewrite. If not: the TOOLCHAIN.md §4 finding that the trigger is removed.
- A long-suite run proving no mid-run corruption and no cascade (exact counts recorded).

## Task exit criteria
- [ ] The status file records each H1-H3 probe's exact output and names the winning
      hypothesis (or combination).
- [ ] `grep -rn "\-o /dev/null" crates/examples/` returns nothing; 2-3 example run.sh
      scripts still pass their compile-checks end-to-end with byte-identical success
      messages and no leftover artifacts.
- [ ] `tests/ref_mc_smt_test.rs` uses `unwrap_or_else(PoisonError::into_inner)` (or
      equivalent) for the BIN_CACHE lock, with a comment citing this task.
- [ ] If H1 confirmed: `scripts/rustc-wrap.sh` exists, is executable, is exported by the
      TOOLCHAIN.md §2 prefix + BUILD.md checklist (+ START.md §4 if applicable), and
      TOOLCHAIN.md §4 no longer says "recreate it if /tmp was cleared".
- [ ] A long suite run (`cargo test --features smt` or fuller) finishes with
      `file /dev/null` = char device, zero PoisonError failures, and the baseline counts
      (612/752/634) green.
- [ ] `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`
      green after all changes.

## Non-goals
- No changes to the rustc/cargo toolchain itself (it is a pinned external; the probe
  behavior is worked around, not patched).
- No changes to unsloth-studio's sandboxing (out of reach from inside; H1 is confirmed or
  refuted by probes, not fixed).
- No restructuring of the example test legs or the SMT parity architecture - only the
  two fragile points (run.sh outputs, mutex poison handling) are touched.
- No changes to 3F-3J scope or ordering (this task is independent; it is simply best done
  before 3I's long runs).
- No new /dev/null healing logic in Rust source - the shell-level heal line in the
  activation prefix stays the session-start safety net.

## Other information
- Probe hygiene: run H1 probes in a FRESH shell (the activation prefix heals /dev/null at
  start, which would mask a session-start corruption - comment out or skip the heal line
  for the probe sequence, and record the pre-heal state FIRST).
- The 28-file run.sh edit is mechanical but must keep each script's success-message strings
  byte-identical (test legs pin them) - do it with a careful per-file edit or a reviewed
  scripted replacement, then re-run the affected example test legs.
- If the investigation finds the corruption ALSO hits non-cargo tools (gcc writing to
  /dev/null via the same run.sh lines), the 3K.2 fix covers it automatically - note that
  in the status as a bonus finding.
- Keep the TASKS.md entry for this task factual: which hypothesis won, what was changed,
  the long-run proof counts - future sessions recover the whole incident understanding
  from that entry + TOOLCHAIN.md §4 without re-reading this file.
