# Phase 5, Task 14 - Implementation status

Completed 2026-10-23. Training data for the Phase 5 user-facing surface: 13 new SFT entries across `frontend_cli` (+6), `project_tooling` (+3) and `codegen_rust` (+4) covering `rfs info`/`test`/`diagram` with options + exit codes, `validate --json`, external-tool parameterization + `--print-script`, rfs-build/build.rs/RFS_BIN, workspace layout, example-crate walkthroughs, `--no-std`, `--with-protocol-fsm`, the adapters crate, and MC counterexample JSON. Every quoted output was captured verbatim from the shipped binary in this environment; snippets + aggregate regenerated; training tests green.

## Added source files

| File | Description |
|---|---|
| `docs/roadmap/PHASE_5_TASK_14_STATUS.md` | this file |

## Changed/removed source files

None (training-data task; no `src/` changes).

## Other file changes

| File | Operation | Description |
|---|---|---|
| `training/frontend_cli/rfs_training.jsonl` | changed | +6 entries: (1) `info` text output on spec 04 - sections incl. monitor-subset status, file-only input + the real directory-rejection error, exit codes; (2) `info --json` full document on spec 01 - stable key order, all keys present (`outputs: []`, `on_error: null`); (3) `validate --json` - ok/parse/semantic documents verbatim, parse line/col semantics incl. the 1:1 whole-module case, semantic 0/0, directory-mode array over a small reproducible 2-file dir; (4) `rfs test` unit suite - real rust output on spec 13 with all five test classes named, C-backend output shape, `--suite bogus` usage error; (5) `--print-script` + tool parameterization - script header/steps verbatim (heredoc body described in prose), env > PATH precedence, `*_ARGS`, flag overrides, real missing-clang diagnostic via `RFS_CLANG`; (6) `diagram` - `--list`, dot + table outputs, pure-text primary behaviour, strict missing-render-tool error (exit 1), unknown-format error, self-contained `--print-script` render script |
| `training/project_tooling/rfs_training.jsonl` | changed | +3 entries: (7) rfs-build build.rs wiring - API sketch, RFS_BIN resolution order, cargo rerun directives, builder options, the shipped sync_handler build.rs quoted verbatim incl. the include!/inner-attribute-stripping pattern; (8) workspace layout - explicit member list quoted from Cargo.toml, C projects as non-members, per-suite results selected from a real `cargo test --workspace` run; (9) example-crate walkthrough - sync/async handlers on spec 43, both demo outputs verbatim (identical session data: 19 ticks / [0..8] / 5 dropped acks / max window 4) |
| `training/codegen_rust/rfs_training.jsonl` | changed | +4 entries: (10) `--no-std` - real generated header, the exact 5-line diff vs std output, production-only rejections verbatim (mc + test), C no-op comment line; (11) `--with-protocol-fsm` - emitted impl block verbatim, off-by-default invariant, mc rejection verbatim, C byte-identical ignore; (12) rfs-adapters - ProtocolFsm trait quoted from the crate source, the four generic adapters with real signatures, no_std core + futures confinement; (13) MC `--counterexample-format json` - real JSON document on a one-step failing trace, field-by-field guide, default text report verbatim, exact-mode-only + cross-backend byte parity |
| `training/frontend_cli/snippets/valid/*`, `training/codegen_rust/snippets/valid/*` | regenerated | via `cargo extract-training-snippets`; +5 valid snippets (frontend_cli 1->4: specs 04/01/13; codegen_rust 1->3: specs 04/13); invalid buckets unchanged; no drift in the other 8 categories |
| `all_rfs_training.jsonl` | regenerated | via `cargo build-training`; 231 -> 244 lines |

## Changed functionality

None - no tool behavior changed; training data only.

## Removed functionality

None.

## New functionality

- SFT coverage of the Phase 5 user-facing surface (13 new entries, all outputs verbatim from shipped-binary runs):
  - `rfs info` text + `--json` (monitor-subset status, stable key order with all keys present, file-only input, exit codes)
  - `validate --json` document shape (parse line/col incl. the 1:1 whole-module-failure case, semantic 0/0, directory array, no `.err` side files in JSON mode)
  - `rfs test` unit suite (five test classes by name on real output, rust + C result shapes, exit codes, `--suite` usage error)
  - `--print-script` (deterministic POSIX sh: shebang/`set -e`/header with resolved tools/self-contained heredoc) + external-tool parameterization (`RFS_*` env vars > PATH, `*_ARGS`, flag overrides, missing-tool diagnostics only for selected suites)
  - `rfs diagram` (`--list`, dot/table outputs, pure-text primary behaviour with no tool calls, STRICT missing/failing render tool = exit 1, self-contained render script)
  - rfs-build build.rs usage (RFS_BIN resolution, rerun directives, include! inner-attribute pattern, `.test_command()`/`.no_std()`/`.with_protocol_fsm()` options)
  - workspace layout (explicit members, Makefile-driven C projects as non-members, real per-suite test results)
  - example-crate walkthrough (sync_handler/async_handler on spec 43 with identical pinned session data)
  - `--no-std` (5-line header diff, identical public API, production-only rejections, C no-op comment)
  - `--with-protocol-fsm` (emitted impl block, off by default, mc rejection, C ignore)
  - rfs-adapters (ProtocolFsm trait + four generic adapters, feature confinement of futures)
  - MC `--counterexample-format json` (document fields, text default unchanged, byte-identical across backends)

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no test-code changes; the existing `training_test` suite (structure / snippet validity / aggregation / freshness) now covers the 13 new lines and their 5 new snippets |

## Tests run status

| Name | Status |
|---|---|
| `cargo extract-training-snippets` | OK (135 valid / 127 invalid snippets) |
| `cargo build-training` | OK (244 examples aggregated) |
| `cargo test-training` (4) | OK |
| `cargo test` (375) | OK (unchanged from the task 13 baseline - training data adds no Rust tests; snippet suites green on the new snippets) |

## Old -> new counts (record for task 16)

| Metric | Old | New |
|---|---|---|
| `all_rfs_training.jsonl` lines | 231 | 244 (+13) |
| snippets valid | 130 | 135 (+5) |
| snippets invalid | 127 | 127 |
| `frontend_cli` jsonl lines | 5 | 11 (+6) |
| `frontend_cli` snippets | 1 valid / 0 invalid | 4 valid / 0 invalid |
| `project_tooling` jsonl lines | 3 | 6 (+3) |
| `project_tooling` snippets | 0 / 0 | 0 / 0 (no rfs fences by design - invalid quotes stay inline) |
| `codegen_rust` jsonl lines | 15 | 19 (+4) |
| `codegen_rust` snippets | 1 valid / 4 invalid | 3 valid / 4 invalid |

New snippet bucketing: all five new snippets are VALID (specs 04_tcp_handshake_like, 01_trivial_two_state, 13_properties_safety - existing examples/valid files); no new invalid snippets.

## Task corrections / decisions

- **D14-1**: commands are quoted in `cargo run -- <cmd>` form to match the existing entries and the BUILD.md command table (it runs the same shipped binary; outputs captured from `target/debug/rfs` built at HEAD).
- **D14-2**: long outputs are scoped, never truncated mid-line: the `test --print-script` entry quotes the header + steps verbatim with the heredoc body described in prose (it is byte-for-byte the codegen output); the `validate --json` directory-mode quote uses a small 2-file reproducible directory (both files given inline) instead of the 3 KB `examples/invalid` array. Every quoted line is verbatim from a real run.
- **D14-3**: invalid spec quotes stay as indented inline text (not ```rfs fences) so the extractor does not turn them into snippet files - per the task file's "invalid quotes stay inline (not extracted)".

## Verification

- Exit criterion 1 spot-check, systematic: all 21 untagged output blocks in the new entries (244 non-empty lines) match the capture corpus byte-for-byte; every inline-quoted error string re-verified against a live run of `target/debug/rfs`.
- Exit criterion 2 - error/invalid entry per new command area: info (directory rejection), validate --json (parse + semantic documents), test (`--suite bogus` usage error + missing-clang diagnostic via `RFS_CLANG`), diagram (missing dot render tool exit 1 + unknown format), codegen flags (mc rejections for BOTH `--no-std` and `--with-protocol-fsm`).
- Live cross-checks while capturing: MC JSON byte-identical across rust/c backends on the failing spec; `--no-std` diff = exactly 5 header lines (C: 1 comment line); `--with-protocol-fsm` = default output + block exactly once.
- Exit criterion 3: `cargo extract-training-snippets && cargo build-training && cargo test-training` green; new aggregate count 244 recorded above.
- Exit criterion 4: all new valid snippets parse (training tests); no snippet drift in the other 8 categories (extractor output identical).

## Found problems/issues/mismatches

- None blocking. Observation taught in the entry: a parse error on a wholly-failing first module reports line/col 1:1 (the top-level `many0` matches zero modules, so the residual starts at the file beginning) - the training data teaches the real behavior rather than an idealized position.
