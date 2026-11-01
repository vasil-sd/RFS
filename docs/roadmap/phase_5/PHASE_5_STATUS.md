# Phase 5 - Implementation status

> Written once when all exit criteria of `PHASE_5.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

**State:** COMPLETE (all exit criteria met)
**Date:** 2026-10-23

Phase 5 delivered the full toolchain and production integration: a six-command
CLI (`parse`, `validate`, `info`, `test`, `diagram`, `codegen`), a cargo
workspace with the `rfs-build` build.rs helper and the `rfs-adapters`
integration crate, hand-rolled JSON everywhere (serde removed), external-tool
parameterization + `--print-script` on every tool-invoking command, `--no-std`
production codegen, MC counterexample JSON, extended cross-backend test
generation with libFuzzer skeletons for both ecosystems, the example-crate
matrix, and in-repo IDE support. Tasks 1-14 of the planned 16 + user-added
task 17 (C4); close-out task 16 = this file.

## Added source files

| Name | Description |
|---|---|
| `src/json_rw.rs` | Hand-rolled JSON reader-writer (task 2): ordered-object value model, pinned compact/pretty writer, strict reader with byte-offset errors, typed extraction. ALL JSON in the toolchain; an embedded copy of the writer ships inside generated MC binaries. |
| `src/info.rs` | `rfs info` command (task 3): static per-module introspection, text + `--json` document (stable key order, all keys present). |
| `src/ext_tool.rs` | Shared external-tool resolver + invocation helper (tasks 3/17): presets RUSTC/GCC/DOT/PLANTUML/MERMAID/CLANG/CARGO_FUZZ with `RFS_*` env vars (+ `*_ARGS`), CLI-flag override, PATH default; fixed-subcommand extension (cargo-fuzz); renders command lines for `--print-script` and executes them otherwise. |
| `src/test_cmd.rs` | `rfs test` command (tasks 7/17): suite selection (`unit`/`fuzz`), unit pipeline (codegen into temp dir + compile with resolved rustc/gcc + run), C/Rust libFuzzer fuzz pipelines, missing-tool diagnostics only for selected suites, `--print-script` coverage. |
| `src/diagram.rs` + `src/diagrams/{mod,dot,plantuml,mermaid,tables}.rs` | `rfs diagram` command (task 8): plugin registry mirroring the codegen backend pattern; v1 formats dot/puml/mmd/table/md; pure-text primary behaviour, strict `--render svg`. |
| `src/cli_log.rs` | stderr logging helper + `--log-level` filtering (task 3). |
| `crates/rfs-build/{Cargo.toml, src/lib.rs, tests/integration.rs}` | build.rs helper crate (tasks 1/9/11/12): `RFS_BIN` -> PATH resolution, pinned cargo rerun directives, builder options `.backend()/.production()/.test_command()/.with_property_monitors()/.no_std()/.with_protocol_fsm()`; dependency-free. |
| `crates/rfs-adapters/{Cargo.toml, src/{lib,channel,async_adapter,callback,minimal}.rs, tests/{adapters,nostd_minimal}.rs}` | Rust-ecosystem integration layer (task 11): `ProtocolFsm` trait + four generic adapters; `#![no_std]` core with std/async features; futures optional behind async. |
| `crates/examples/sync_handler/*`, `crates/examples/async_handler/*` | Example crates (task 11): spec-43 sliding-window ARQ driven end to end over the channel adapter / futures stream adapter; session as test + demo binary. |
| `crates/examples/c_sync_handler/*`, `crates/examples/fuzz_c/*` (Makefile-driven, NOT cargo members), `crates/examples/fuzz_rust/*` | Example projects (task 12): plain-C mirror of sync_handler (`gcc -Wall`, scenario data byte-identical to the Rust side); clang+libFuzzer and cargo-fuzz harnesses wiring the emitted skeletons. |
| `ide/{textmate/rfs.json, snippets.rfs, README.md}` | IDE support (task 13): TextMate grammar for `.rfs` (scope `source.rfs`, all 28 keywords), snippet set, VS Code/Sublime install notes. |
| `tests/example_projects_test.rs` | Example-project verification suite (task 12): c_sync_handler rebuild + C/Rust scenario parity, fuzz_rust detached-workspace check, fuzz_c generation (build gated on clang). |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen.rs` | changed | New flags: `--no-std` (task 9), `--with-protocol-fsm` (task 11), `--counterexample-format json\|text` (task 10) + `BackendOptions` fields + pinned entry-point rejections (`NO_STD_PRODUCTION_ONLY_ERROR`, `PROTOCOL_FSM_MC_ERROR`, `COUNTEREXAMPLE_FORMAT_ERROR`). |
| `src/ir.rs` | changed | serde derives REMOVED; hand-written `to_json`/`from_json` (fallible for u64-range values - plan correction C3) reproducing the exact pre-migration wire format. |
| `src/frontend.rs` | changed | `.json` paths via json_rw; structured `validate --json` diagnostics (task 13: parse errors carry real 1-based line/col, semantic errors 0/0, no `.err` side-files in JSON mode). |
| `src/main.rs` | changed | Six subcommands; info/test/diagram dispatch; all new flags + usage text; exit-code convention. |
| `src/parser.rs` | changed | `pub const KEYWORDS` (28 RFS v0 keywords, single source of truth shared with the `ide/` grammar) + bidirectional drift-guard test (task 13). |
| `src/printer.rs`, `src/lib.rs`, `src/training.rs` | changed | json_rw round-trip paths; new module declarations; JSONL validation via json_rw. |
| `src/codegen_test.rs` | changed | Extended language-agnostic test planner (task 4): per-transition cases (static guard taken/not-taken analysis), invariant cases, monitor-violation cases, seeded random-trace case, fuzz decode plan. |
| `src/codegen_mc.rs`, `src/mc_plan.rs` | changed | MC JSON report model + key-ordering contract (task 10); `ValueDomain::Enum` carries the enum name (C10-2 pre-existing fix: bare variant names did not compile). |
| `src/backends/rust.rs`, `rust_fsm.rs`, `rust_test.rs`, `rust_mc.rs` | changed | no_std header variant (task 9); opt-in `impl ProtocolFsm for Fsm` emission (task 11); extended test rendering + separate `fuzz_target.rs` skeleton (task 5); JSON report renderer with embedded writer (task 10). |
| `src/backends/c.rs`, `c_fsm.rs`, `c_test.rs`, `c_mc.rs` | changed | C mirrors: extended tests + `fuzz_target.c` libFuzzer skeleton (task 6); no_std documented no-op comment (task 9); JSON renderer mirror (task 10); **task 16 close-out fix**: `fsm_free` lifecycle end in the generated core + single-exit skeleton decode calling it per input (see Found problems, T16-1). |
| `tests/backend_parity_test.rs`, `benchmark_mc_test.rs`, `codegen_test.rs`, `examples_test.rs`, `frontend_test.rs`, `smt_bounded_test.rs`, `training_test.rs` | changed | Per-task suites: extended-test parity (task 6), JSON report parity (task 10), no_std sweep + protocol-fsm pins (tasks 9/11), info/test/diagram e2e (tasks 3/7/8), fuzz pipeline e2e (task 17), validate --json suite (task 13), task 16 C-fuzz free regression. |

## Changed functionality

- `rfs codegen` flag surface grew (`--no-std`, `--with-protocol-fsm`,
  `--counterexample-format`); default output of every command is byte-identical
  to pre-Phase 5 except where a task explicitly changed it (extended test
  artifacts, C fuzz skeleton per T16-1).
- The generated C production core gained `fsm_free` (task 16 fix) - additive;
  short-lived consumers (unit-test/MC executables) are unaffected.
- IR JSON wire format: UNCHANGED (hand-written serialization reproduces the
  serde output byte-for-byte on all 45 valid examples, both layouts).

## Removed functionality

- `serde`/`serde_json` dependencies (task 2): removed from `Cargo.toml`; every
  source migrated to `src/json_rw.rs`. (chumsky remains as a dev-only
  comparison smoke test; z3 stays optional behind the `smt` feature.)

## New functionality

- **CLI**: three new stable commands - `rfs info` (static introspection,
  text/`--json`), `rfs test` (generate + compile + run unit AND libFuzzer fuzz
  suites for both backends in one invocation; the CI entry point for generated
  tests), `rfs diagram` (dot/puml/mmd/table/md plugins; pure-text primary
  behaviour, strict `--render svg` with no graceful degradation).
- **External-tool parameterization**: env-var-preferred `RFS_RUSTC/RFS_GCC/
  RFS_DOT/RFS_PLANTUML/RFS_MERMAID/RFS_CLANG/RFS_CARGO_FUZZ` (+ `*_ARGS`),
  overridden by per-command flags, bare name on PATH as default; shared
  resolver in `src/ext_tool.rs`.
- **`--print-script`**: deterministic POSIX sh emission (shebang, `set -e`,
  header with rfs version + spec + resolved tools, commented steps, self-
  contained heredocs) instead of execution, on every tool-invoking command.
- **Workspace + crates**: cargo workspace (explicit members); `rfs-build`
  build.rs helper (`RFS_BIN` resolution, rerun directives); `rfs-adapters`
  (`ProtocolFsm` trait + channel/async/callback/minimal generic adapters).
- **Codegen**: `--no-std` production codegen (`#![no_std]` + `extern crate
  alloc`, identical public API; C documented no-op); opt-in
  `--with-protocol-fsm` impl emission (the one documented exception to the
  self-contained invariant); MC `--counterexample-format json` (byte-identical
  across backends, exact mode only).
- **Test generation**: extended shared planner rendered by BOTH backends with
  identical coverage (per-transition, invariant, monitor-violation, seeded
  random-trace) + libFuzzer skeletons as separate extra files for both
  ecosystems; `rfs test --suite fuzz` pipelines (C: clang skeleton-only build +
  bounded run; Rust: temp cargo-fuzz scaffold with nightly pin).
- **Examples**: sync/async handler crates on spec 43 (identical pinned session
  data), plain-C mirror, fuzz harness examples for both ecosystems.
- **IDE**: in-repo TextMate grammar + snippets; `rfs validate --json`
  structured diagnostics for editor integrations.
- **Training data**: 13 new SFT entries covering the Phase 5 surface (task 14).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `tests/example_projects_test.rs` | new | Example-project verification (task 12): c_sync_handler gcc -Wall rebuild + C/Rust scenario parity with pinned values, fuzz_rust detached-workspace check, fuzz_c generation. |
| `crates/rfs-build/tests/integration.rs`, `crates/rfs-adapters/tests/{adapters,nostd_minimal}.rs` | new | Helper-crate suites (tasks 1/11): RFS_BIN precedence/stub e2e; adapters over a mock FSM + no_std compile checks. |
| lib unit tests (+64 over Phase 4: 125 -> 189) | added | json_rw (24), ext_tool resolver/script/stub pins (9, incl. task 17 presets), cli_log (2), codegen_mc JSON report suite (task 10), codegen-test-planner suite (task 4), planner guarantee (task 5). |
| `tests/codegen_test.rs` (26 -> 50) | extended | no_std suite (task 9: header byte-pins + 45-example x 2-modes compile sweep + body parity), protocol-fsm suite (task 11: emitted-block byte pin, mc rejection both levels, C ignore), Rust/C renderer suites (tasks 5/6: extended classes, `--steps` sensitivity, fuzz skeleton extra-file rules), JSON writer embedding + dependency-free marker (task 10), **task 16 C-fuzz per-input free regression** (`fsm_free` in core, single-exit skeleton decode, clang-gated bounded run on spec 01 - the OOM case). |
| `tests/frontend_test.rs` (41 -> 93) | extended | info suite (task 3), test suite (task 7: both backends, injected invariant failure, --print-script byte pin + no-execution proof, flag > env > PATH stub proofs), diagram suite (task 8: 5-format pins, 45-example sweep, strict render errors, real-tool acceptance with negative controls), validate --json suite (task 13), fuzz pipeline suite (task 17: suite selection, missing tools, stub argv proofs, byte-pinned script, real-tool gated e2e). |
| `tests/backend_parity_test.rs` (7 -> 10) | extended | Per-test result-list comparison across backends (task 6); JSON report byte-parity over ALL property-bearing examples + five status/reason cases + golden full-document pin (task 10). |
| `tests/benchmark_mc_test.rs`, `tests/smt_bounded_test.rs`, `tests/training_test.rs` | changed | JSON report in the benchmark suite (task 10); SMT suite unchanged-behavior re-verification; training tests cover task 14's grown data. |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` (workspace root) | OK |
| `cargo test` (root package) | OK - **376/376** (189 lib, 10 parity, 5 benchmark MC, 50 codegen, 3 examples, 93 frontend CLI, 4 parser-library, 9 liveness MC, 4 monitor integration, 4 training, 3 example projects; +2 with `--features smt`) |
| `cargo test --workspace` | OK - **400/400** (376 + rfs-build 10 + rfs-adapters 12 via feature unification + sync_handler 1 + async_handler 1) |
| `cargo test --features smt` (z3 env per TOOLCHAIN.md 5.2) | OK - **378/378** (Phase 4 optional mode unbroken) |
| `cargo clippy --all-targets` | OK - 13 pre-existing warnings = re-pinned baseline (12 lib + 1 codegen_test), zero new |
| `cargo fmt --check` | OK - clean |
| cargo aliases (`parse`/`validate`/`codegen`/`generate-code-tests`/`test-generated-code`/`extract-training-snippets`/`build-training`/`test-training`) | OK - all work from the repo root (task 1 guarantee re-verified) |
| `cargo generate-code-tests && cargo test-generated-code` | OK - **1501/1501** (184 modules = 179 examples + task 14's 5 new valid training snippets) |
| `cargo run -- validate --input rfs examples/valid` | OK - exit 0 (45 valid / 12 invalid examples) |
| Training chain (`extract-training-snippets && build-training && test-training`) | OK - 135 valid / 127 invalid snippets, 244 aggregate lines, 4/4 tests |
| serde removal check (`grep` over Cargo.toml + src/tests) | OK - zero dependencies; the only source mention is the dependency-free marker test asserting generated MC code contains no serde reference |
| IR JSON round-trip golden tests | OK - `ir_json_golden_compact_pins_key_order`, `ir_json_golden_pretty_pins_layout`, `ir_json_roundtrip_property_all_valid_examples` |
| Workspace build of all example crates (`cargo test -p` each) | OK - rfs-build 10, rfs-adapters 9 (default)/12 (--all-features), sync_handler 1, async_handler 1, fuzz_rust builds clean |
| `gcc -Wall` spot checks of NEW C output | OK - zero warnings: extended test artifacts (specs 01/13/43), c_sync_handler full Makefile rebuild + run (pinned session data: 19 ticks / [0..8] / 5 dropped acks / max window 4), fuzz_c generation + clang build via `example_projects_test` |
| no_std compile sweep (task 9 suite, re-run) | OK - 7/7 (45 examples x 2 monitor modes compile under `#![no_std]`, bodies byte-identical to std) |
| `--print-script` smoke (`rfs test --print-script`, `rfs diagram --render svg --out ... --print-script`) | OK - scripts emitted, exit 0, NO execution (no artifacts created) |
| Real-tool C fuzz bounded runs (post T16-1 fix) | OK - spec 01: 5,769,560 runs in 31 s exit 0; spec 43: 3,302,638 runs in 31 s exit 0 (pre-fix spec 01 OOM'd at ~218k runs) |

## Found problems/issues/mismatches

- **T16-1 (found by this task's verification, FIXED at its source):** the
  shipped `rfs test --backend c --suite fuzz` FAILED on specs where every input
  byte maps to one FSM step (e.g. spec 01): each libFuzzer iteration leaked the
  per-input trace buffer (the generated C FSM had no free API - C17-2), so RSS
  grew until libFuzzer's out-of-memory limit tripped (~218k runs, exit 1 with
  the misleading "libFuzzer found a crashing input"). Task 17's verification
  used spec 43, where each step consumes multiple bytes, so the per-run leak
  stayed small enough to pass. Fix: (1) the generated C core now emits an
  explicit `fsm_free(fsm_t*)` freeing the two per-fsm heap allocations (trace
  buffer + `active_assumes` array); (2) the fuzz skeleton uses a single-exit
  decode (truncated field runs / unknown variants set a `done` flag and break -
  mirroring the Rust skeleton's break + scope-drop) and calls `fsm_free(&fsm)`
  at the end of every input. Regression test:
  `c_fuzz_target_frees_per_input_and_bounded_run_stays_green` (content pins +
  clang-gated bounded run on spec 01). Post-fix: the fuzz binary is fully
  leak-free even with LeakSanitizer ENABLED (812k runs, exit 0); `detect_leaks=0`
  stays merged in the pipeline as pinned defense in depth. Docs updated: C.md
  (lifecycle API + skeleton), DESIGN.md D-P5-5 (correction note).
- **Count re-pins (verified numbers win, per task file):** the generated-code
  aggregate grew 1453 -> **1501** tests (179 -> **184** modules) because task
  14's five new valid training snippets each become an aggregate module; root
  suite grew 375 -> **376** (+1 T16-1 regression test), workspace 399 ->
  **400**, smt 377 -> **378**. BUILD.md/START.md/KB.md re-pinned in this task.

## Other important findings/information

- The C fuzz pipeline's `ASAN_OPTIONS=detect_leaks=0` merge (C17-2) is now
  belt-and-braces: with T16-1 fixed, the emitted fuzz target itself is
  leak-free. Follow-up candidate for Phase 6: drop the merge once every
  consumer path is proven leak-free, to restore exit-time leak detection.
- `rfs test --suite fuzz` (Rust) uses a TEMP cargo-fuzz scaffold with a pinned
  nightly toolchain + offline `libfuzzer-sys = "=0.4.13"` registry pin; the
  C pipeline needs clang with the libFuzzer runtime (TOOLCHAIN.md section 7).
  Both are availability-gated in tests and documented.
- Phase 5 baseline superseded: Phase 4 close-out recorded 230/230 main (232
  smt), 537/537 generated (179 modules), 231 training lines. Final Phase 5:
  376/376 main (400 workspace, 378 smt), 1501/1501 generated (184 modules),
  244 training lines (135 valid + 127 invalid snippets).
- Design decisions D-P5-1..D-P5-14 as shipped: `DESIGN.md` section 23 (with
  plan-correction pointers C1-C4 and the T16-1 correction note).
