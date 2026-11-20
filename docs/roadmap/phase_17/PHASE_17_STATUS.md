# Phase 17 - Implementation status

> Written once when all exit criteria of `PHASE_17.md` are reached. Not rewritten
> afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.

## Added source files

The three new cargo-member example crates (each a build.rs that generates a real
spec via rfs-build + a lib that drives the artifact):

| Name | Description |
|---|---|
| `crates/examples/no_std_fsm/{Cargo.toml,build.rs,src/lib.rs}` | genuinely `#![no_std]` + `extern crate alloc` lib; build.rs generates spec 03 with `.no_std(true)`; identical public API to the std build |
| `crates/examples/monitors_production/{Cargo.toml,build.rs,src/lib.rs}` | production build with `.with_property_monitors(true)` on spec 41; debug-gated monitor catches an injected violation at the step API, release twin is zero-cost |
| `crates/examples/callback_minimal_adapters/{Cargo.toml,build.rs,src/lib.rs}` | the two rfs-adapters never demonstrated before - the callback driver (completion/failure/step-limit) and the no_std minimal wrapper (forwarding/composition) over a real `.with_protocol_fsm(true)` generated FSM |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml` | changed | workspace members list gains the three example crates above |
| `tests/example_projects_test.rs` | changed | +11 legs: 3 negative-MC (pinned CE traces/diagnostics + fix proofs), 3 nested-cargo build+test for the member crates, 4 plain-dir walkthroughs, 1 clang-gated `make CC=clang` leg for c_sync_handler |
| `src/cli/args.rs` | changed | the `usage()` string literal ONLY - the mc-ref signature line + help paragraph now show the three-mode CLI (matching `rfs mc-ref`'s own usage text); every other command's line is byte-identical. No parsing/dispatch/exit-code behaviour changed |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/user/USAGE.md` | new | the exhaustive CLI usage reference: exit-code convention, command map, the RFS_* tool-resolution contract (nine-tool table + precedence model + clang/z3 notes + a real `--print-script` `# tools:` proof), one section per command set (parse / validate / info+info format / codegen rust\|c / test / diagram / mc-ref / gen-spec) each with a four-column pattern table + naming-and-placement rules + error-details-and-fixes; the codegen take-or-emit contract stated in BOTH the table rows and a dedicated subsection; the mc-ref section with all three modes' flag tables + exit codes and field-by-field CE-trace walkthroughs using task 6's REAL captured traces; and the 15-row example-projects table |
| `docs/user/MODEL_CHECKING.md` | changed | section 2 rewritten for mode dispatch (per-mode flags/defaults/exit codes, `--nondet` under exact, not-applicable reduced to `--threads`/`--fast`, parity vs BOTH backends in all three modes); section 1's mode table + exit-code paragraph corrected (fuzz has no 2; smt-bounded 0/1/2) |
| `docs/user/INTEGRATION.md` | changed | one-line Nix installation pointer (`nix build`, BUILD.md section 8) next to the from-source note |
| `docs/user/README.md` | changed | USAGE.md row in "Which doc for which question?" + a pointer in the toolchain-at-a-glance section |
| `docs/dev/cli_commands.md` | changed | four stale spots rewritten: Overview + Dispatch to the plugin-registry architecture (main.rs a pure dispatcher over ALL_COMMANDS), Library API surface to the current module layout, mc-ref section to mode dispatch + per-mode exit codes |
| `docs/dev/workspace_crates.md` | changed | rows for the three new member crates |
| `docs/dev/ir_json_rw.md`, `docs/dev/validator.md` | changed | `frontend_test` -> `utils_test` command-name fixes |
| `docs/llm/SRC_STRUCTURE.md` | changed | budget header 1786->1830; five line-neutral stale-name fixes (test_cmd, frontend.rs->utils.rs, tree legend, PHANTOM row, mermaid label) |
| `docs/llm/TEST_STRUCTURE.md` | changed | budget header 1379->1430; thirteen line-neutral stale-name fixes (gen_spec_cmd/mc_ref_cmd/cli_log + ten frontend_test->utils_test) |
| `docs/llm/CHANGE_IMPACT.md` | changed | four stale-name fixes (gen_spec_cmd, test_cmd, frontend_test x3) |
| `docs/llm/TASK_MAP.md` | changed | row 31: mc-ref CLI row names the mode dispatch + all three engine entry points + links the three engine fragments |
| `docs/llm/cli_commands.md` | changed | row 33: ALL eight commands are registry plugins; row 39: exit_code_for is exact-mode-only with fuzz/smt-bounded codes derived inline |
| `docs/llm/ir_json_rw.md`, `docs/llm/validator.md` | changed | frontend_test -> utils_test fixes |
| `docs/llm/workspace_crates.md` | changed | rows for the three new member crates |
| `KB.md` | changed | section 4.2: three NEW rows for src/mc/ref_mc_exact.rs / ref_mc_fuzz.rs / ref_mc_smt.rs; the ref_mc.rs row updated to common-core + entry-points; the cli/commands/mc_ref.rs row updated to three-mode dispatch with per-mode exit codes |
| `TOOLCHAIN.md` | changed | section 6.4 gains the "mmdc present is NOT mmdc working" gotcha + the verified chrome-headless-shell install recipe (persistent PUPPETEER_CACHE_DIR, the mermaid-cli-bundled puppeteer 25.16.0) |
| `crates/examples/{mc_walkthrough,json_schema_ci,specgen_mutation_loop,tool_resolution,mc_deadlock_fail,mc_property_violation_fail,invalid_spec_diagnosis,no_std_fsm,monitors_production,callback_minimal_adapters}/README.md` | new | ten example READMEs, each following the template (What it demonstrates / The machine with mermaid diagrams / Files layout / Build-run-test / Expected results with pinned outputs) |
| `crates/examples/{sync_handler,async_handler,c_sync_handler,fuzz_rust,fuzz_c}/README.md` | changed | the five existing READMEs upgraded to the same template |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `crates/examples/mc_walkthrough/` (pass.rfs, fail.rfs, run.sh, captures/) | new | both-backends MC walkthrough with real text+JSON CE captures |
| `crates/examples/json_schema_ci/` (pass.rfs, fail.rfs, invalid_always_zero_bound.rfs, validate.sh, work/) | new | schema emission + CI-style validation of REAL tool output against the five JSON schemas |
| `crates/examples/specgen_mutation_loop/` (loop.sh, work/) | new | the gen-spec random+mutation -> validate -> mc-ref -> fix loop with a pinned real transcript |
| `crates/examples/tool_resolution/` (pass.rfs, run.sh, Makefile, captures/) | new | explicit RFS_* path passing + `--print-script` `# tools:` proof of the precedence model + gcc/clang Makefile choice |
| `crates/examples/mc_deadlock_fail/` (04_tcp_handshake_like.rfs, fixed.rfs) | new | the deadlock negative example + its one-transition fix |
| `crates/examples/mc_property_violation_fail/` (47_http2_stream_rst_fail.rfs, fixed.rfs) | new | the property-violation negative example + its fix |
| `crates/examples/invalid_spec_diagnosis/` (broken.rfs, fixed.rfs) | new | the invalid-spec diagnosis example + its corrected spec |
| `crates/examples/c_sync_handler/Makefile` | changed | documented `make CC=clang` option (the same source builds warning-free under both gcc and clang) |
| `training/frontend_cli/rfs_training.jsonl` | changed | 2 stale mc-ref entries rewritten + 1 new (RFS_* tool resolution) |
| `training/model_checking/rfs_training.jsonl` | changed | 1 stale mc-ref entry rewritten + 1 new (CE-trace reading) |
| `training/codegen_rust/rfs_training.jsonl` | changed | 1 new entry (artifact take-or-emit semantics) |
| `training/project_tooling/rfs_training.jsonl` | changed | 1 new entry (the ten new example projects) |
| `training/<cat>/snippets/` | regenerated | via cargo extract-training-snippets: 138 -> **140 valid** (+2 = the two intentionally-fenced complete specs), 128 invalid unchanged |
| `all_rfs_training.jsonl` | regenerated | via cargo build-training: 267 -> **271 lines** (+4 new entries) |

## Changed functionality
None - documentation + example-projects + training-data only. The single source
edit (`src/cli/args.rs::usage()`) changes PRINTED text, never parsing, dispatch
or exit codes (verified: behaviour-identical test results).

## Removed functionality
None.

## New functionality
- An exhaustive external CLI reference (`docs/user/USAGE.md`): every command set's pattern table (pattern | options | generated-or-reused artifacts | errors + exit codes), the RFS_* tool-resolution contract, the codegen take-or-emit artifact semantics, and real CE-trace walkthroughs.
- A substantially expanded example set covering almost all usage scenarios:
  - negative model-checking diagnosis (deadlock CE reading, property-violation CE reading, invalid-spec diagnostic reading) - each with the fix + re-run proof pinned by a test leg;
  - cargo-member integrations (no_std production, production property monitors, the callback + minimal rfs-adapters);
  - plain-dir walkthroughs (both-backends MC report reading, JSON-schema CI validation, the gen-spec mutation loop, explicit external-tool resolution).
- A training set aligned with the unified three-mode mc-ref and the new USAGE.md semantics: CE-trace reading, artifact take-or-emit, RFS_* tool-resolution precedence, and the ten new example projects.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `example_projects_test` - 3 negative-MC legs | new | pin the REAL counterexample/diagnostic excerpts (reason=deadlock CE ending in Established; label=rst_goes_closed step=2 CE through HalfClosedLocal; the duplicate-priority diagnostic in text AND --json) + each fix's PASS/valid exit |
| `example_projects_test` - 3 cargo-member legs | new | nested-cargo build+test of no_std_fsm / monitors_production (incl. its release suite) / callback_minimal_adapters |
| `example_projects_test` - 4 plain-dir legs | new | mc_walkthrough (both backends, text+JSON CE), json_schema_ci (schema validation of real output), specgen_mutation_loop (pinned transcript), tool_resolution (`# tools:` precedence proof + gcc/clang Makefile) |
| `example_projects_test` - 1 clang leg | new | c_sync_handler builds the same source warning-free under `make CC=clang` (availability-gated) |
| no_std_fsm / monitors_production / callback_minimal_adapters inline tests | new | the member crates' own unit/integration tests (run under `--workspace`) |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` | OK |
| `cargo test` (root) | **584 pass** (Phase 16 baseline 573 + 11 new example_projects legs) |
| `cargo test --workspace` | **720 pass** (baseline 701 + 11 legs + 8 inline tests from the three new member crates) |
| `cargo test --features smt` (z3 5.3.1 per TOOLCHAIN.md section 5.2) | **594 pass** (baseline 583 + 11 legs) |
| `cargo test-generated-code` | **1720 pass / 194 modules** - see Found problems: the +2 modules / +37 tests are task 11's two new valid-spec fences; all pre-existing modules regenerate byte-identical (no codegen behaviour change) |
| `cargo test --test docs_md_test` | **5 pass** (user-set quality gate now covering USAGE.md + all 15 example READMEs: mermaid renders, cross-links resolve) |
| `cargo test --test dev_docs_test` | **9 pass** (sync gate + Class B budgets - the extended 1830/1430 headers hold with the appended rows inside them) |
| `rfs-dev inventory audit` | **EXACT** - SRC 1013 fns / 213 types, TEST 1055 fns / 35 types, zero per-file diffs (no fn/type/file in src/ added, removed or renamed) |
| `cargo clippy --all-targets` | 0 errors (only pre-existing duplicate warnings in untouched test helpers) |
| `cargo fmt --check` | clean |

## Found problems/issues/mismatches
- **BUILD.md's root/workspace count lines were stale from prior phases.** They read "538 pass" / "666 pass", which is BELOW the Phase 16 close-out baseline (573/701) recorded in CURRENT_STATE.md - those lines were never synced in Phase 15 or 16. Corrected in this close-out to the measured **584** / **720** (and the example-projects sub-count 3 -> 14, the training snippet/aggregate totals 138/128/267 -> 140/128/271).
- **generated-code moved 1683 -> 1720**, not the "exactly 1683" the task context predicted. This is correct and expected: the generator turns every `training/<cat>/snippets/valid/*.rfs` file into a module (path-dedup, so no content-collision with examples), and task 11 intentionally added two valid-spec fences. BUILD.md documents "it moves when snippet sets change." The deeper invariant - no codegen behaviour change - holds: all 192 pre-existing modules regenerate byte-identically; only 2 new snippet modules were appended (+37 tests from the extended plan classes).

## Other important findings/information
- Every nuanced claim in USAGE.md and the training entries was verified by RUNNING the command against the built binary, not from memory: take-or-emit (an existing `--fsm-file` is reused, only the mc artifact rewritten), the C partial-pair usage error, the stale-reuse warning, the mc-ref per-mode exit codes, the fuzz per-run report (no summary line) vs no-violation summary line, and the `# tools:` precedence captures (task 8.4's real artifacts).
- All three engines (mc-ref, Rust MC binary, C MC binary) produced BYTE-IDENTICAL reports on the negative examples - the parity claim in the READMEs is verified, not assumed.
- The generated MC artifacts reference their fsm file by the CODEGEN-TIME name (`#[path]` / `#include "..."`), so compiling them from a different CWD fails with doubled or missing paths - the flat-dir + plain-filename pattern (what the test suites use) is the reliable capture recipe; noted in USAGE.md's codegen section.
