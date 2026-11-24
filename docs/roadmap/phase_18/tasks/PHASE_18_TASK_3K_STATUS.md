# Phase 18, Task 3K - Implementation status

> Written once when all exit criteria of `PHASE_18_TASK_3K.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| `scripts/rustc-wrap.sh` | The committed RUSTC_WRAPPER (executable): rewrites the rustc target-probe's bare `-` source arg to a guaranteed-empty file and redirects rustc's stdin from that same empty file - so a corrupted/missing `/dev/null` can never be parsed as source. No-op passthrough for normal file-argument compiles. Replaces the retired ephemeral `/tmp/rustc-wrap.sh` recipe. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `crates/examples/*/run.sh` (28 files) | changed | Both `-o /dev/null` compile-checks (rustc `--emit=metadata`, gcc `-c`) now write to `$BUILD/.compile-check.rmeta` / `.compile-check.o` and `rm -f` them after the check; success messages byte-identical. Removes the documented mid-run corruption trigger. |
| `tests/ref_mc_smt_test.rs` | changed | `cached_binary` lock acquisition: `.lock().expect("bin cache lock")` -> `.lock().unwrap_or_else(std::sync::PoisonError::into_inner)` with a comment citing this task + the 2026-11-23 incident. A poisoned mutex now degrades ONE test (rebuild) instead of cascading PoisonErrors through every remaining parity test. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TOOLCHAIN.md` | changed | §2 activation prefix: exports `RUSTC_WRAPPER=/workspace/host/RFS/scripts/rustc-wrap.sh` next to the /dev/null heal line. §4: both /dev/null bullets rewritten - bullet 1 now states the ROOT CAUSE (sparse tmpfs /dev, device deleted after image build, any write to the missing path recreates it as a regular file); bullet 2 records the stabilization (trigger removed, mutex resilient, committed wrapper as THE remedy) and RETIRES the ephemeral /tmp recipe + its code block. |
| `START.md` | changed | §4 activation block: same RUSTC_WRAPPER export, kept in lockstep with TOOLCHAIN.md §2. |
| `BUILD.md` | changed | §5 verification checklist: the minimum command block now carries the RUSTC_WRAPPER export with a pointer to TOOLCHAIN.md §4. |

## Other file changes

None (no examples/, training/, or build-system files touched beyond the above).

## Changed functionality
- The 28 example `run.sh` production compile-checks: output destination moved from `/dev/null` to a temp file inside `$BUILD` (deleted after the check; the scripts' existing EXIT trap also cleans `$BUILD`). Check semantics and success messages unchanged.
- `tests/ref_mc_smt_test.rs` binary cache: poison recovery instead of cascade failure.

## Removed functionality
- All in-repo writes of compiler output to `/dev/null` (the 56 `-o /dev/null` occurrences across the 28 scripts).
- The ephemeral `/tmp/rustc-wrap.sh` work-around recipe (retired from TOOLCHAIN.md §4; superseded by the committed `scripts/rustc-wrap.sh`).

## New functionality
- Root-caused the /dev/null incident: `/dev` is a sparse tmpfs baked at image build (sibling nodes dated Oct 31); the original `/dev/null` was deleted afterwards; any write to the missing path (redirect or `-o`) recreates it as a regular file holding that data - which rustc's target-probe then parses as source (E0762 / not-valid-UTF-8). The ELF-content signature in past incidents is exactly gcc/rustc `-o /dev/null` on the missing device (reproduced during this task); the git-status-text signature comes from session tool-call redirects, not repo scripts.
- A durable, zero-effort defense: the committed wrapper exported by every activation point (TOOLCHAIN.md §2, START.md §4, BUILD.md §5), proven by A/B test - with `/dev/null` corrupted, `cargo build` fails without the wrapper ("failed to run rustc to learn about target-specific information") and succeeds with it.
- One-blip-degrades-one-test semantics for the SMT parity suite's shared binary cache.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_smt_test.rs` (`cached_binary`) | changed | poison-resilient lock (no new test functions - the behavior change is exercised by every existing parity test on the next real incident) |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test example_projects_test` (plain build, RUSTC_WRAPPER exported) | OK - 42 passed / 0 failed in 2158.99 s; `/dev/null` still a char device at the end (the mid-run trigger is gone across the whole 28-script corpus) |
| `cargo test --features smt` (full smt matrix, RUSTC_WRAPPER + z3 env) | OK - 639 passed / 0 failed across all 34 test binaries; ZERO PoisonError/panic lines in the log; `/dev/null` still a char device at the end. The count is the 2026-11-23 baseline (634) + 5 legs added by task 3D after that measurement - this task adds no tests |
| `cargo test --no-run` (plain build compile check) | OK - all test executables build with the ref_mc_smt_test.rs change present |
| A/B probe test (corrupted `/dev/null`: regular file with garbage) | Without wrapper: `cargo build` exit 101 (probe failure). With wrapper: `cargo build` Finished in 2.40 s. Device healed afterwards |

## Found problems/issues/mismatches
- The original `/dev/null` device node is GONE from the sandbox tmpfs (all sibling nodes carry the Oct 31 image-build date; only a mknod'd replacement exists) - the deletion event itself is managed by unsloth-studio's sandboxing and is not visible from inside (its install dir is credential-protected). H1 confirmed as far as it can be from inside: the quirk is real, intermittent, and unfixable from within - hence the durable work-around is the deliverable, alongside removing every in-repo trigger.
- No other `-o /dev/null` compiler-output sites and no shell redirects to `/dev/null` exist anywhere in `crates/`, `tests/`, or the build files (grep-verified) - the 28 run.sh lines were the sole in-repo mid-run write-sites.

## Other important findings/information
- Deliberate deviation from the old /tmp recipe: the committed wrapper redirects rustc's stdin from the guaranteed-empty `/tmp/empty_src.rs` instead of from `/dev/null` - so even a corrupted device can never feed rustc's stdin (the old recipe was safe only because the `-` arg rewrite made stdin irrelevant; this removes that dependency).
- The wrapper self-heals its empty-file on every invocation (`printf '' > /tmp/empty_src.rs`), so /tmp clears are harmless; only the script itself is durable, and it now lives in the repo.
- Future sessions recover the whole incident understanding from this status + TOOLCHAIN.md §4 + the TASKS.md entry - no need to re-read the task file.
