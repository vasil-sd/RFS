# Phase 6 - Implementation status

> Written once when all exit criteria of `PHASE_6.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-25

Phase 6 delivered production hardening and the v1.0 release: a formal semantics
document (`docs/SEMANTICS.md`), the v1.0 versioning surface (syntax version
header, IR_VERSION 1 with read policy {0,1}, spec-hash-stamped generated
artifacts, crate 1.0.0, MIGRATION.md), independent reference infrastructure
(specgen + ref_mc exposed as `rfs mc-ref` / `rfs gen-spec`), five differential/
parity test suites (incl. the no-false-PASS negative control and cross-backend
Rust-vs-C byte parity), two-layer toolchain fuzzing (root cargo-fuzz workspace
+ slow compile-and-run CI suite), v1.0 benchmark specs 47/48/49 with extended
MC report metrics (`peak_rss_kb`, `ce_steps`) and a CI-runnable benchmark
suite, an external user documentation set (`docs/user/`, machine-checked via
mdr's parser stack), and the training-data update to the v1.0 surface. Tasks
1-9 of 10; close-out task 10 = this file + tag `v1.0.0`. Seven real bugs were
found by the phase's own verification infrastructure and fixed, each recorded
as a plan/task correction (2a, 4b, 5a, 5b, 6a, 6b, 6c).

## Exit criteria (`PHASE_6.md`) - verbatim check

Checked against this close-out run (2026-10-25) + the task records; all six hold.

1. **"v1.0 syntax is stable"** - SATISFIED. Task 2 shipped the versioning
   surface: formal `version "1.0"` header with validator format check (pinned
   error example `examples/invalid/invalid_bad_version_format.rfs`),
   `IR_VERSION = 1` with read policy {0,1} (v2+ rejected with pinned diagnostic),
   crate bumped to 1.0.0, `docs/user/MIGRATION.md` with the v0 -> v1 entry.
   This run: full suite green incl. `46_version_header.rfs` round-trips and all
   version pins; sealed by tag `v1.0.0` on this close-out commit.
2. **"toolchain passes fuzzing and differential testing (including cross-backend Rust-vs-C comparisons)"** - SATISFIED. This run, all green:
   `ref_mc_diff_test` 4/0 (ref-MC vs generated MC incl. counterexample replay +
   no-false-PASS negative control), `backend_parity_test` 11/0 (forced-violation
   parity, both backends fuzz mode, byte-identical stdout+exit),
   `prod_parity_test` 1/0 (EVERY valid example, cross-backend random-input
   byte parity), `monitor_ref_test` 2/0 (hand-written reference monitors vs
   generated), `psl_translation_test` 1/0 (independent PSL window evaluator vs
   generated), `toolchain_fuzz_test` 2/0 (compile-and-run fuzz layer). Bounded
   libFuzzer smokes this run: C-side spec-43 target 1,457,093 runs / 11 s /
   exit 0; Rust-side four toolchain targets rebuilt on nightly against the
   CURRENT tree (parser 637,562 / validator 11,541 / ir_roundtrip 2,013 runs @
   10 s each, codegen 1,758 @ 60 s) all exit 0.
3. **"model checker reports are trustworthy and clearly labeled"** - SATISFIED.
   The no-false-PASS negative control (`ref_mc_diff_test::tiny_state_limit_never_pass_complete`)
   is green in this run; the labeling guarantees (status vocabulary incl.
   `unknown-state-limit`, assumption-stop accounting, monitor-subset
   warnings) are specified in `docs/SEMANTICS.md` section 9 and documented for
   external users with real captured reports in `docs/user/MODEL_CHECKING.md`
   (PASS / FAIL+CE / deadlock / liveness-lasso anatomy).
4. **"production code generation is used in at least one real or realistic protocol project"** - SATISFIED, recorded explicitly: the Phase 5 example-crate matrix was re-verified GREEN in this run and drives the spec-43 sliding-window ARQ protocol end to end from GENERATED production code on both ecosystems - `sync_handler` (std mpsc channels) and `async_handler` (futures Stream) built via `rfs-build` + `rfs-adapters` (`cargo run -p <crate>` exit 0; their session tests are part of the 477/0 workspace run), and `c_sync_handler` rebuilt from clean via its Makefile (`gcc -Wall`, scenario data lines byte-identical to the Rust side: 19 ticks, frames/acks [0..7], 5 out-of-band drops, max window 4). A sliding-window ARQ sender with property monitors is a realistic protocol project; this is not a toy demo.
5. **"documentation is sufficient for external users"** - SATISFIED. `docs/user/` ships eight files (README, LANGUAGE, PROPERTIES, MODEL_CHECKING, INTEGRATION, EXAMPLES, FAQ + MIGRATION; 2135 lines across the task-7 set) plus `docs/SEMANTICS.md` (923 lines); every quoted tool output was captured live from v1.0.0 and every command example verified runnable (task 7 record); cross-links are machine-checked - this run `docs_md_test` 5/5 over all repo `.md` files: 148 at matrix-run time, re-run green after this status file's addition (set = 149 at the tag point); checks: comrak GFM parse + fence closure + full mermaid-rs-renderer pipeline for the 24 diagrams + relative-link resolution.
6. **"benchmark suite runs in CI"** - SATISFIED. Single command `cargo test --test benchmark_mc_test` green in this run (5/5, ~15 s): six v1.0 specs x both backends with deterministic-metric parity, FAIL variants pinned by label + step + ce_steps, parse/codegen timing informational, JSON report written to `target/mc_benchmarks.json` + per-backend table printed; `BUILD.md` section 7 "Benchmarks in CI" documents the command and carries the GitHub Actions snippet.

## Added source files

| Name | Description |
|---|---|
| `src/spec_hash.rs` | Hand-rolled SHA-256 spec hash (task 2): canonical IR digest, no new dependency; stamped into line 1 of EVERY generated artifact for stale-artifact detection. |
| `src/specgen.rs` | Seeded random spec generator + incremental mutation (task 3): LCG lineage shared with `codegen_test`, per-step validator gate with roll-back/redraw, validity by construction; feeds the fuzz targets and differential suites. |
| `src/ref_mc.rs` | Brute-force reference model checker (task 3, ~5k lines): independent representative-input rule, own monitor compilers, level-synchronous BFS, liveness end-component analysis, deadlock detection; public APIs `step_trace` / `verify_counterexample` / `RefResult.deadlock_mons` let external suites replay generated engines' JSON counterexamples against an independent transition relation (task 4). |
| `src/mc_ref_cmd.rs` | `rfs mc-ref` command (task 3): in-process ref_mc run WITHOUT compiling generated code; exit codes 0/1/2 mirror the generated binaries. |
| `src/gen_spec_cmd.rs` | `rfs gen-spec` command (task 3): fully random or base-mutated spec generation, output ALWAYS parsed + validated before writing, deterministic from the seed. |
| `fuzz/Cargo.toml`, `fuzz/Cargo.lock`, `fuzz/README.md` | Detached cargo-fuzz workspace `rfs-fuzz` (task 5): fuzzes the TOOLCHAIN itself; offline-resolvable deps (libfuzzer-sys 0.4.13 / arbitrary 1.5.0 from cache). |
| `fuzz/fuzz_targets/{common,parser,validator,ir_roundtrip,codegen}.rs` | Four libFuzzer targets + shared specgen-backed `Arbitrary` wrapper (raw IR types deliberately NOT made `Arbitrary`). |
| `fuzz/regression_seeds/{codegen_empty_input, ir_roundtrip_short_rest}` | Regression seeds for the two harness-side bugs found while building task 5. |

## Changed/removed source files

No source files were removed (`git diff --diff-filter=D` empty over the whole phase).

| Name | Operation | Description |
|---|---|---|
| `src/ir.rs` | changed | IR_VERSION 0 -> 1; read policy accepts {0,1}, rejects >=2 with pinned diagnostic (task 2). |
| `src/validator.rs` | changed | Version-header format check + major-version policy; new pinned error for malformed headers (task 2). |
| `src/printer.rs` | changed | Prints/round-trips the version header in rfs output (task 2). |
| `src/codegen.rs` | changed | Line-1 artifact header: toolchain version + spec hash on every command of both backends incl. fuzz skeletons (task 2). |
| `src/codegen_mc.rs` | changed | Report alignment to ref_mc's prefixed value forms in text+JSON via shared renderers (task 3, D-P6-10); JSON report gains `statistics.peak_rss_kb` + `counterexample.ce_steps` (task 6). |
| `src/codegen_test.rs` | changed | Planner binding values keyed by pattern-binding name (correction 5a); exact `expect_progress_from_initial` replacing the over-approximation (correction 6b) + pins. |
| `src/backends/c_mc.rs` | changed | Safe `trace_vars()` helper for fuzz-mode obligation rendering at sample 0 (correction 5b, was a segfault); ce_steps + VmHWM self-report (task 6); well-defined wide shifts/mul (2a). |
| `src/backends/rust_mc.rs` | changed | Post-level stop guard in `run_exact` - the BFS expanded one extra level after violation/state-limit (correction 4b, counters only); ce_steps + peak-RSS self-report (task 6). |
| `src/backends/{c,c_fsm,rust,rust_fsm}.rs` | changed | `<name>__rfsb` binding-identifier mangling so pattern bindings can never shadow generated-body identifiers (correction 6a, was an out-of-bounds write in C); well-defined wide bitvector ops on the C side (2a). |
| `src/backends/{c_test,rust_test}.rs`, `src/backends/{C,RUST}.md` | changed | Re-pins + doc updates for the report alignment and the fixes above (tasks 3-6). |
| `src/lib.rs`, `src/main.rs` | changed | New public modules (`ref_mc`, `specgen`, `spec_hash`) + the two new CLI commands wired in. |
| `src/ext_tool.rs`, `src/test_cmd.rs` | changed | Small adjustments surfaced by the phase's clippy pass (pre-existing warning baseline, see findings). |
| `Cargo.toml`, `Cargo.lock` | changed | Crate version 0.1.0 -> 1.0.0; dev-deps `comrak` 0.55 + `mermaid-rs-renderer` 0.3 (mdr's parser stack behind the doc-quality gate; mdr itself is binary-only and cannot be a library dep). |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/SEMANTICS.md` | new | Formal semantics, 10 sections / 923 lines (task 1): every claim cross-checked against the implementation; section 9 = model-checking report guarantees incl. labeling and the no-false-PASS invariant. |
| `docs/user/{README,LANGUAGE,PROPERTIES,MODEL_CHECKING,INTEGRATION,EXAMPLES,FAQ}.md` | new | External user documentation set (task 7): 2135 lines, 24 mermaid diagrams, every quoted output captured live from v1.0.0, every command verified runnable. |
| `docs/user/MIGRATION.md` | new | v1.0 compatibility policy + migration process/template + the v0 -> v1 entry (task 2). |
| `DESIGN.md` | changed | New section 24 "Phase 6 decisions as shipped" D-P6-1..D-P6-11 with execution notes (task 9); earlier pointer updates from tasks 1/3. |
| `KB.md` | changed | Re-measured counts/baselines + new-surface knowledge entries, stale-swept to final state (task 9). |
| `BUILD.md` | changed | Section 6 "Toolchain fuzzing" (task 5), section 7 "Benchmarks in CI" with Actions snippet (task 6), command rows for mc-ref/gen-spec, all counts re-pinned to final (task 9). |
| `TOOLCHAIN.md` | changed | Section 7.5 root-level fuzz workspace conventions (task 5); clang/libclang-rt install notes current. |
| `CODEGEN.md`, `src/backends/{C,RUST}.md` | changed | Generated-artifact contract updated: line-1 header, wide-op well-definedness, report fields (tasks 2/3/6, synced in 9). |
| `START.md` | changed | File-set rows + reading guidance for the new docs/suites; dev-deps note (task 9). |
| `docs/roadmap/PHASE_6_PLAN.md`, `PHASE_6_TASK_{1..10}.md` (+ `_STATUS` files) | new | Two-stage protocol artifacts: plan D-P6-1..D-P6-11 with corrections 2a/4b/5a/5b/6a/6b/6c/6d, per-task specs + status reports. |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `examples/valid/46_version_header.rfs` | new | v1.0 version-header teaching example (task 2). |
| `examples/valid/{47_http2_stream,48_usb_control_transfer,49_can_arbitration}.rfs` | new | v1.0 benchmark specs (task 6): measured 434/21857/7, 104/340/15, 240/448/41 states/transitions/max_depth - far under the ~150k budget. |
| `examples/valid/50_wide_ops.rfs` | new | Wide bitvector-op boundary example pinning correction 2a's well-defined behaviour (task 2a). |
| `examples/invalid/invalid_bad_version_format.rfs` | new | Pinned malformed-version-header rejection (task 2). |
| `training/{syntax_semantics,design,model_checking,examples_explain,frontend_cli}/rfs_training.jsonl` | changed | 17 new SFT entries across the five v1.0 categories, all with real captured output (task 8); append-only. |
| `training/*/snippets/*` (+3 valid, +1 invalid), `all_rfs_training.jsonl` | changed | Snippet regeneration: 135/127 -> 138/128; aggregate 244 -> 261 lines (task 8). |
| `.gitignore` | changed | += `fuzz/target`, `fuzz/corpus/`, `fuzz/artifacts/` (task 5). |

## Changed functionality

- Syntax versioning: the optional `version "1.0"` header is now a formal, validated part of the language - malformed headers are rejected with a pinned diagnostic; legacy v0.x specs without a header remain valid.
- IR read policy: documents with `ir_version` in {0,1} load; >=2 is rejected with a pinned "newer IR version" error (was: no version check).
- Generated artifact header: line 1 of EVERY generated file (both backends, all commands incl. fuzz skeletons) now carries the toolchain version + spec hash - stale artifacts are detectable by inspection.
- MC report shape: JSON reports gain `statistics.peak_rss_kb` and `counterexample.ce_steps` on both backends; text+JSON value rendering unified with ref_mc's prefixed forms (D-P6-10); monitor obligation "remaining" bits are packed only while pending so config identity matches ref_mc byte-for-byte.
- C codegen for wide bitvector ops: variable shifts render a uint64-wide guarded form (`amount >= width -> 0`) and multiplication computes in a uint64_t intermediate - previously the emitted C was UB/divergent vs Rust (correction 2a).
- Crate version: `rfs` 0.1.0 -> 1.0.0.

## Removed functionality

None - verified: `git diff --diff-filter=D db4a1df..HEAD` is empty; no command, flag, format or behaviour was dropped by the phase (the v0 read path remains for compatibility).

## New functionality

- Versioning surface (task 2): syntax version header + validator policy, IR_VERSION 1, hand-rolled SHA-256 spec hash, artifact stamping, MIGRATION.md.
- `rfs mc-ref` command (task 3): in-process reference model checking without compiling generated code; exit codes mirror the generated binaries.
- `rfs gen-spec` command (task 3): deterministic random / base-mutated spec generation, always validated before write.
- Reference infrastructure as libraries (tasks 3/4): `src/ref_mc.rs` (public `step_trace`, `verify_counterexample`, `RefResult.deadlock_mons`) + `src/specgen.rs` (shared LCG lineage, mutation ops) - independent of the generated-code path by construction.
- Differential + parity test layer (task 4): ref-MC vs generated-MC differential with counterexample replay and the no-false-PASS negative control; forced-violation cross-backend parity; production-FSM random-input byte parity over EVERY valid example; hand-written reference monitors (specs 43/44/45); independent PSL window-translation evaluator (+ `RFS_PSL_REF_TOOL` external-tool hook).
- Toolchain fuzzing layer (task 5): root detached cargo-fuzz workspace with four libFuzzer targets over parser/validator/IR-roundtrip/codegen + the slow seeded compile-and-run suite (`tests/toolchain_fuzz_test.rs`) that is CI-runnable without nightly.
- Benchmark metrics + suite extension (task 6): `peak_rss_kb` / `ce_steps` in MC reports; six v1.0 specs x both backends with FAIL variants pinned by label + step + ce_steps and cross-backend parity; informational parse/codegen timing; `target/mc_benchmarks.json` report; BUILD.md CI section.
- External user documentation (tasks 2/7): the `docs/user/` set (8 files) + formal `docs/SEMANTICS.md`, machine-checked by the new doc-quality gate.
- Doc-quality gate (task 7 prep, user rule change 2026-10-25): `tests/docs_md_test.rs` validates every repo `.md` with mdr's parser stack (comrak GFM + full mermaid-rs-renderer pipeline + link resolution); new `.md` files enter the checked set automatically. Mermaid diagrams are an allowed doc construct (page limit 3000 lines).
- v1.0 benchmark examples (task 6): HTTP/2 stream, USB control transfer, CAN arbitration - all `version "1.0"`, safety + bounded liveness.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/ref_mc_diff_test.rs` (4) | new | 4.1 differential: status agreement + CE replay validity on all eligible examples + 20 specgen random specs; negative control `tiny_state_limit_never_pass_complete`. |
| `tests/ref_mc_sweep_test.rs` (1) | new | Task-3 sweep harness pin: 45/45 byte-identical vs generated Rust MC, fail-fast ordering. |
| `tests/ref_mc_diag_test.rs` (1) | new | ref_mc diagnostics pins (timeout mid-level bug regression included). |
| `tests/prod_parity_test.rs` (1) + `tests/common/prod_driver.rs` | new | 4.3: every valid example, seeded 200-step scripts, byte-identical stdout+exit across backends; shared driver with the ASSUME/PROP echo contract. |
| `tests/monitor_ref_test.rs` (2) | new | 4.4: hand-written Rust + C reference monitors vs generated on random + crafted boundary traces (specs 43/44/45). |
| `tests/psl_translation_test.rs` (1) | new | 4.5: independent recursive window evaluator vs generated monitors, 19 examples x 28 properties; `RFS_PSL_REF_TOOL` hook. |
| `tests/toolchain_fuzz_test.rs` (2) | new | Slow CI fuzz layer: 3 seeds x 10 small specs -> six artifacts -> rustc/gcc -Wall compile -> MC exact+fuzz vs ref_mc ground truth + deterministic trigger-step-0 pin. |
| `tests/wide_ops_parity_test.rs` (2) | new | Cross-backend differential pins for correction 2a's well-defined wide-op boundary behaviour. |
| `tests/docs_md_test.rs` (5) | new | Doc-quality gate: all-repo sweep + three negative controls (unterminated fence, non-rendering mermaid, dead link). |
| `tests/common/{mod,mc_json}.rs` | new | Shared helpers: MC JSON report parsing/pinning used by the differential suites. |
| `tests/backend_parity_test.rs` | changed | Extended with 4.2 forced-violation parity (every assumption-bearing example x label + combined run; seed 777, byte-identical stdout+exit both backends). |
| `tests/benchmark_mc_test.rs` | changed | 3 -> 6 specs; FAIL variants 2 -> 5 with ce_steps pins + cross-backend parity; JSON report + table test; informational parse/codegen timing; threads gate 5 -> 8 workloads. |
| `tests/codegen_test.rs` | changed | Re-pins for header/hash/report changes + correction 5a/6b regression pins. |
| `tests/frontend_test.rs` | changed | e2e coverage for the two new commands (mc-ref/gen-spec) incl. pinned errors; count re-pins. |
| `tests/{example_projects,integration_example,liveness_mc,monitor_integration,smt_bounded}_test.rs`, `crates/rfs-adapters/tests/nostd_minimal.rs` | changed | Golden-output re-pins only (header line 1, report fields) - no behaviour change. |

No test files were removed.

## Tests run status

Close-out verification matrix, all executed 2026-10-25 on the final tree (tag `v1.0.0` commit):

| Name | Status |
|---|---|
| `cargo build` | OK (exit 0) |
| `cargo test` (root package) | OK - 453 passed / 0 failed |
| `cargo clippy --all-targets` | OK - exit 0; 36 warnings, ALL pre-existing in task 4-6 test files (`tests/common/prod_driver.rs`, `tests/common/mc_json.rs`, `src/test_cmd.rs`) - no new warnings |
| `cargo fmt --check` | OK - clean |
| `cargo test --workspace` (all members incl. example crates) | OK - 477 passed / 0 failed |
| `cargo test --features smt` (z3 5.3.1, env per TOOLCHAIN.md 5.2) | OK - 455 passed / 0 failed |
| `cargo generate-code-tests` | OK - 192 modules; regenerated tree byte-identical to committed (git clean) |
| `cargo test-generated-code` | OK - 1683 passed / 0 failed |
| `cargo test --test examples_test` | OK - 3/3 over 50 valid + 13 invalid example files (the task file's "49/13" parenthetical predates task 6's three benchmark specs) |
| `cargo test --test ref_mc_diff_test` | OK - 4/0 (~59 s) |
| `cargo test --test backend_parity_test` | OK - 11/0 (~38 s) |
| `cargo test --test prod_parity_test` | OK - 1/0 (~2 s) |
| `cargo test --test monitor_ref_test` | OK - 2/0 (~1 s) |
| `cargo test --test psl_translation_test` | OK - 1/0 (~5 s) |
| `cargo test --test toolchain_fuzz_test` | OK - 2/0 (~29 s) |
| `cargo test --test benchmark_mc_test` | OK - 5/0 (~15 s); `target/mc_benchmarks.json` written (suite/mode/specs/codegen_timing) + per-backend table printed, all six specs pass-complete with identical deterministic metrics across backends |
| gcc -Wall spot checks, NEW C artifacts 47/48/49 | OK - production objects compile warning-free; MC binaries build warning-free, run exit 0 `pass-complete` with exactly the pinned counts (434/21857/7, 104/340/15, 240/448/41) |
| `cargo run -p sync_handler` / `-p async_handler` | OK - both exit 0 with the pinned ARQ session (frames/acks [0..7], 5 OOB drops, max window 4) |
| c_sync_handler Makefile rebuild + run (`RFS_BIN`) | OK - clean rebuild from `make clean`, gcc -Wall, scenario parity byte-identical to the Rust side |
| fuzz_rust detached workspace `cargo check` | OK - exit 0 |
| fuzz_c generation without clang (`make generate`) | OK - production FSM + libFuzzer skeleton emitted with only the rfs toolchain |
| C-side libFuzzer smoke (spec-43, `-max_total_time 10`) | OK - 1,457,093 runs in 11 s, exit 0 (clang 18.1.3 + libclang-rt-18 installed during this run; see findings) |
| Rust-side toolchain fuzz smokes (rebuilt on nightly vs current tree) | OK - parser 637,562 / validator 11,541 / ir_roundtrip 2,013 runs @ 10 s each + codegen 1,758 @ 60 s; all exit 0 |
| `cargo test --test docs_md_test` | OK - 5/5 over all repo `.md` files: 148 at matrix-run time, re-run green after this status file's addition (set = 149 at the tag point); checks: comrak GFM parse, fence closure, mermaid pipeline for 24 diagrams, link resolution |

## Found problems/issues/mismatches

All found by the phase's own verification infrastructure; each fixed within its task and recorded as a plan/task correction in `PHASE_6_PLAN.md` / the task files. None open at close-out.

- **2a** (task 1, SEMANTICS cross-check): generated C for wide bitvector ops was UB/divergent vs Rust - variable shifts unguarded (`amount >= width`) and multiplication done in-width; fixed with well-defined uint64-wide forms + cross-backend differential pins.
- **4b** (task 4): the Rust exact-mode BFS expanded ONE EXTRA LEVEL after a property violation or state-limit hit (Phase-B break skipped the per-config stop check, SEMANTICS 9.5 "the search STOPS"); fixed with a post-level guard in `run_exact` - counters only, verdict/CE unchanged. Same task: ref_mc liveness DFS path-only pruning spun forever on diamond subgraphs (specgen seed 900_006) - fixed with global-visited; sample-0 minimal labels now collect every failing monitorable property.
- **5a** (task 5, found by the `codegen` fuzz target): valid specgen specs failed `test` codegen on BOTH backends - planner keyed binding values by declared field names while `eval_env` looks them up by pattern-binding name; fixed in `src/codegen_test.rs` + regression seed.
- **5b** (task 5, found by the slow suite): generated C MC SEGFAULTED in fuzz mode when a property obligation started at sample 0 (`steps[-1]`); fixed with the safe `trace_vars()` helper + deterministic micro-spec pin.
- **6a** (task 6): pattern-binding SHADOWING of generated-body identifiers - spec 48's binding `n` shadowed C MC's unqualified effect counter, so emitted outputs were written out-of-bounds and the counter never incremented (memory-safety + parity bug reachable from valid specs); fixed with `<name>__rfsb` mangling in both backends.
- **6b** (task 6): test planner's `expect_progress = !transitions.is_empty()` over-approximation generated a POSITIVE drive that fails at runtime on spec 47's guarded initial arm; fixed with exact `expect_progress_from_initial`.
- **6c** (task 6): spec 48's absorbing Error arm violated its own `no_double_setup` invariant once Error became reachable under relaxed assumptions (found by the new FAIL variant); fixed with the RESET-first convention.
- **6d** (task 6, design note): RFS priorities fire DESCENDING (SEMANTICS 3.4) - a first draft of the spec-49 FAIL variant relying on guard overlap silently passed; final variant mutates the band-entry arm's `set` instead.
- Task 7 doc findings: the task outline's `state_is` atom does not exist on the shipped surface (documented the actual atom set per SEMANTICS 5.1); bare `make -C crates/examples/c_sync_handler run` needs `RFS_BIN` in this environment (doc shows the working form); educational specs 04/15/16/42 fail exact MC with deadlock BY DESIGN (documented as teaching cases).
- This close-out run: NONE - every matrix item green on first execution; no code changes were needed.

## Other important findings/information

- **Release tag**: `v1.0.0` (annotated) on this close-out commit; working tree clean; `target/`, `fuzz/target`, `fuzz/corpus/`, `fuzz/artifacts/` are gitignored - no untracked files remain.
- **Post-v1.0 cleanup candidate**: the 36 clippy warnings in task 4-6 test files (`tests/common/prod_driver.rs` majority, `tests/common/mc_json.rs`, `src/test_cmd.rs`) - all pre-existing style lints, zero functional impact; deliberately left out of the release.
- **Environment note for future sessions**: this close-out container did NOT carry clang/libclang-rt (task 5's session had them); both were installed via apt during this run (`clang` 18.1.3 + `libclang-rt-18-dev`) to execute the C-side libFuzzer smoke. TOOLCHAIN.md section 7 documents the install; if a future environment lacks network, the C-side generated fuzz targets cannot be BUILT (generation still works without clang - re-verified), while everything else in the matrix runs on gcc + the project-local Rust toolchain.
- **mdr usage** (for anyone extending the doc gate): mdr is binary-only and cannot be a library dependency; the gate uses its parser stack at mdr 0.6.1's pinned versions (`comrak` 0.55, `mermaid-rs-renderer` 0.3). If a new mermaid construct fails `docs_md_test`, the error carries the renderer's typed diagnostic (line/col) - fix the diagram, never weaken the check.
- **Crates.io publishing** is NOT a phase deliverable and was not done; it is a natural post-v1.0 candidate (the crate is already at 1.0.0 with stable syntax/IR/codegen contract).
- **Count lineage for future phases**: root `cargo test` 453, workspace 477, smt 455, generated-code 1683/192 modules, examples 50 valid / 13 invalid, training aggregate 261 lines (snippets 138/128), repo docs 149 `.md` at v1.0.0 (all under the doc gate).
