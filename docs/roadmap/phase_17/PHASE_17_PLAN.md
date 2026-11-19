# Phase 17 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

Phase 17 is a documentation + example-projects + training-data phase. NO language,
IR, codegen or MC behaviour changes; the only `src/` edit is the printed usage
text in `src/cli/args.rs`. The staleness inventory (verified 2026-11-19) lives in
`PHASE_17.md` section "Staleness inventory" - fix every row of it. The proposed
USAGE.md layout, example README template and the list of 10 new example projects
are also in `PHASE_17.md` - they are BINDING for this plan.

Findings from preparation that constrain implementation:

- **BOTH structure inventories are EXACTLY at their Class B budgets**
  (SRC_STRUCTURE 1786/1786, TEST_STRUCTURE 1379/1379) - and this phase ADDS
  source files (task 7's three cargo-member crates: lib/main/build.rs each,
  ~15-20 inventory rows) and test functions (tasks 6-9's new
  `example_projects_test` legs + the members' inline tests, ~15-20 rows).
  Task 3 therefore EXTENDS both budgets first (SRC_STRUCTURE 1786 -> 1830,
  TEST_STRUCTURE 1379 -> 1430 - +44/+51 headroom), and tasks 6-9 append their
  rows in the same commit as the source they describe (PROTOCOL.md section 5).
  The stale-name fixes in TEST_STRUCTURE rows 1196-1197 stay line-neutral.
  The other LLM files have headroom (TASK_MAP 207/400, CHANGE_IMPACT 36/250,
  cli_commands 77/200). If the actual additions exceed the extended budgets,
  that is a plan correction - do not silently shrink descriptions.
- **No test pins the global usage string or mc-ref's "NOT applicable" text**
  (verified by grep over `tests/`): task 4 migrates nothing, but must still run
  the full suite. `mc_ref_flag_errors_are_usage_failures`
  (`tests/utils_test.rs`) pins flag-error BEHAVIOUR, which is preserved.
- **The current mc-ref CLI surface** (from the actual usage text, Phase 16):
  `rfs mc-ref <spec.rfs|spec.json> [--mode exact|fuzz|smt-bounded]` - exact
  (default): `--max-states N` (1000000), `--timeout-sec N` (0 = off), `--nondet`,
  `--counterexample-format json|text` (text); fuzz: `--bound N` (5), `--runs N`
  (20), `--seed N` (12345), `--violate label:prob:percent%,...`,
  `--unexpected-prob P` (0.0); smt-bounded: `--bound N` (10) is the ONLY
  accepted flag, specs with assumptions/on_error are rejected, requires the
  `smt` cargo feature (non-smt build exits with a clear diagnostic). Shared:
  `--assumes all|none|labels`, `--log-level`. Exit codes: exact 0 pass-complete /
  1 fail / 2 unknown-state-limit; fuzz 0 no violation (summary line) / 1
  violation (per-run report, NO summary line); smt-bounded 0 pass-within-bound /
  1 fail / 2 unknown. Exact-only flags are usage errors under `--mode fuzz`;
  every flag but `--bound` is a usage error under `--mode smt-bounded`.
  Not applicable: `--threads`/`--fast` only.
- **Spec selections for the negative examples:** deadlock =
  `examples/valid/04_tcp_handshake_like.rfs` (known FAIL: deadlock at terminal
  `Established`, the teaching case in docs/user/EXAMPLES.md); property violation
  = ONE of the pinned FAIL variants in `tests/common/fail_variants.rs`
  (`http2_rst_fail_src()` or `can_passive_fail_src()`) - extract the chosen
  variant's source into the example dir as a `.rfs` file and pin it; invalid-
  spec diagnosis = `examples/invalid/invalid_duplicate_priority.rfs` (a semantic
  error with an obvious fix - change one priority) unless implementation finds a
  better-instructive one among the 13.
- **Monitor reference material:** `examples/valid/41_property_monitors.rfs` +
  the buggy-variant pattern from `tests/monitor_integration_test.rs` (drop the
  Ack-handling transition so `fin_acked_bounded` must expire).
- **Example-project conventions:** cargo members follow
  `crates/examples/sync_handler/Cargo.toml` (path dep on `rfs-adapters`,
  build-dep on `rfs-build`, build.rs generates the artifact); plain dirs follow
  `c_sync_handler` / `fuzz_c` (Makefile-driven, NOT workspace members). New test
  legs go into `tests/example_projects_test.rs` with availability gating
  (skip-with-note, never fail, mirroring `tests/common/toolchain.rs`).
- **`tests/docs_md_test.rs` collects EVERY repo `.md`** - all new example
  READMEs and USAGE.md are automatically gated (mermaid must parse, relative
  links must resolve; from `crates/examples/<name>/` the user docs are at
  `../../../docs/user/...`).
- **`python3 jsonschema` 4.26.0 is available** in this environment -
  `json_schema_ci` uses real draft-07 validation, availability-gated like other
  real-tool tests.
- **The workspace test count GROWS** with the three new cargo members - the
  Phase 16 baseline (573 root / 701 workspace / 583 smt / 1683 generated) is a
  FLOOR, not a target; generated-code stays exactly 1683.
- **mmdc/puppeteer state:** installed during preparation -
  `chrome-headless-shell` + `chrome` live in `/workspace/host/.cache/puppeteer`
  (persistent), symlinked into the session's `~/.cache/puppeteer`; mmdc renders
  end-to-end with the `--no-sandbox` puppeteer config rfs already passes. Task 5
  records this in TOOLCHAIN.md.
- **The RFS_* table source of truth** is TOOLCHAIN.md section 8.2 (nine tools,
  precedence CLI flag > RFS_* env var > standard Unix var > auto-discovery);
  USAGE.md section 2 mirrors it and adds the used-by column + the special vars
  (`RFS_BIN`, `RFS_FUZZ_FLAGS`, `RFS_MERMAID_PUPPETEER_ARGS`).
- **Training format rules** (KB.md section 4.6): 2 messages per entry, assistant
  answer wrapped in `< think> ... < /think>`; snippets auto-extracted from fenced
  ```rfs blocks by `cargo extract-training-snippets`; aggregate via
  `cargo build-training`; verify with `cargo test-training`.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Fix stale user docs (mc-ref unification + Nix pointer) | `docs/user/MODEL_CHECKING.md` (change), `docs/user/INTEGRATION.md` (change) | Rewrite section 2 for mode dispatch: per-mode flag tables, per-mode exit codes, `--nondet` now applicable, not-applicable reduced to `--threads`/`--fast`, parity statement covering all three modes vs BOTH backends. Fix section 1's mode table (mc-ref available in all three modes) + the exit-code paragraph (fuzz: 0/1 with no 2; smt-bounded: 0 pass-within-bound / 1 fail / 2 unknown). INTEGRATION.md: one-line Nix installation pointer (`nix build` per BUILD.md section 8) next to the existing from-source note. Rationale: the user set is the primary Phase 17 deliverable's foundation; USAGE.md (task 10) must not contradict it | |
| 2. | Fix stale developer docs (`docs/dev/cli_commands.md`) | `docs/dev/cli_commands.md` (change) | Overview + Dispatch: describe the plugin-registry architecture - `main.rs` is a ~30-line pure dispatcher over `ALL_COMMANDS`/`find()` in `src/cli/commands/mod.rs`, ALL eight commands are plugins with `run_cli`, shared argument handling in `src/cli/args.rs`. Library API surface: replace the pre-Phase 11 flat module list with the current layout (`pub mod cli/codegen/json/lang/mc/training/utils` + the re-exports actually in `src/lib.rs`). mc-ref section: mode dispatch (`run()` delegates to `ref_mc_exact/fuzz/smt`), the ACTUAL current options model (read `src/cli/commands/mc_ref.rs` first - do not trust this plan's names over the source), per-mode exit codes, updated not-applicable list. Rationale: developer docs must match the shipped architecture or they mislead every future session | |
| 3. | Fix stale LLM set + KB.md section 4.2 + extend inventory budgets | `docs/llm/TASK_MAP.md` (change), `docs/llm/CHANGE_IMPACT.md` (change), `docs/llm/SRC_STRUCTURE.md` (change - budget header only), `docs/llm/TEST_STRUCTURE.md` (change), `docs/llm/cli_commands.md` (change), `KB.md` (change) | TASK_MAP row 31: mc-ref CLI row gains the mode entry points (`ref_mc_exact/fuzz/smt::run`, D-P16-1 delegation). CHANGE_IMPACT row 35: `gen_spec_cmd` -> `src/cli/commands/gen_spec.rs`. TEST_STRUCTURE rows 1196-1197: stale names fixed - LINE-NEUTRAL. llm/cli_commands.md rows 33 + 39: dispatch description (ALL commands delegate) + `exit_code_for` per-mode. KB.md section 4.2: NEW rows for `src/mc/ref_mc_exact.rs`, `ref_mc_fuzz.rs`, `ref_mc_smt.rs`; update the `ref_mc.rs` row (common core + entry points, D-P16-1) and the `cli/commands/mc_ref.rs` row (mode dispatch, per-mode exit codes). BUDGET EXTENSION (user instruction 2026-11-19): SRC_STRUCTURE `llm-budget` 1786 -> 1830, TEST_STRUCTURE 1379 -> 1430 - both are exactly at their old budgets and tasks 6-9 will append rows. Rationale: the LLM set is the context-budgeted navigation layer - stale names send future sessions to files that do not exist; the extension must land BEFORE any row-adding task | |
| 4. | Fix the global usage text (`src/cli/args.rs`) | `src/cli/args.rs` (change) | The mc-ref usage line becomes the three-mode signature (matching `rfs mc-ref`'s own usage in `src/cli/commands/mc_ref.rs`); the mc-ref help paragraph describes per-mode flags + exit codes and reduces not-applicable to `--threads`/`--fast`. No test pins this string (verified) - run the full suite anyway. Rationale: the global usage is user-facing documentation embedded in the binary; it currently contradicts the command's own usage text | |
| 5. | Record the mmdc/puppeteer finding in TOOLCHAIN.md | `TOOLCHAIN.md` (change) | Section 6.4 new gotcha: the mermaid-cli npm package does NOT bundle its browser - `chrome-headless-shell` must be installed separately into a PERSISTENT `PUPPETEER_CACHE_DIR` (the sandbox `$HOME` is ephemeral per session); Node 18 cannot run the latest npx-fetched puppeteer (needs Node 20+) - install the version mermaid-cli bundles (25.16.0 here) and run its installer; root needs the `--no-sandbox` config rfs already passes. Include the verified recipe + the symlink approach used in preparation. Rationale: a future session that sees `mmdc` on PATH will assume it works - this gotcha cost real debugging time | |
| 6. | Negative example projects (3) | see subtasks | Parent task: the three negative examples that teach diagnosis. Each is a plain dir (NOT a workspace member) with spec file(s) + README per the template + a test leg in `tests/example_projects_test.rs` that runs the REAL commands and asserts the pinned failure excerpts + exit codes (1 for MC fail, 1 for validation fail) AND that the fixed variants pass (exit 0). Rationale: negative examples are the highest-value user documentation after the happy path | |
| 6.1 | `mc_deadlock_fail` | `crates/examples/mc_deadlock_fail/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | Copy of `examples/valid/04_tcp_handshake_like.rfs` + README: real CE traces from `rfs mc-ref` AND both generated backends (text + JSON), what to look for in a deadlock lasso, the one-transition fix as a second spec file + re-run proof (PASS) | |
| 6.2 | `mc_property_violation_fail` | `crates/examples/mc_property_violation_fail/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | Extract ONE pinned FAIL variant from `tests/common/fail_variants.rs` (`http2_rst_fail_src()` or `can_passive_fail_src()`) into a `.rfs` file; step-by-step CE walkthrough (which property, which step, why); the fix + re-run proof | |
| 6.3 | `invalid_spec_diagnosis` | `crates/examples/invalid_spec_diagnosis/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | Copy of `examples/invalid/invalid_duplicate_priority.rfs`; `rfs validate` text + `--json` diagnostic explained field by field; the corrected spec that validates | |
| 7. | Positive integration examples - cargo members (3) | see subtasks | Parent task: three new WORKSPACE MEMBER crates following the `sync_handler` conventions (path dep on `rfs-adapters`, build-dep on `rfs-build`, build.rs generates the artifact); each README per the template (automaton description + mermaid diagram + files layout + build/run/test + expected results); `Cargo.toml` members list updated; test legs in `tests/example_projects_test.rs`. Rationale: completes the adapter coverage and demonstrates the two production-only flags (`--no-std`, `--with-property-monitors`) that no example currently shows | |
| 7.1 | `no_std_fsm` | `crates/examples/no_std_fsm/` (new), `Cargo.toml` (change), `tests/example_projects_test.rs` (change), `docs/llm/SRC_STRUCTURE.md` + `docs/llm/TEST_STRUCTURE.md` (change - new rows) | build.rs generates production with `.no_std(true)`; the crate lib is `#![no_std]` + `extern crate alloc` including the artifact (identical public API to the std build); a std test drives it - pattern: `crates/rfs-adapters/tests/nostd_minimal.rs` | |
| 7.2 | `monitors_production` | `crates/examples/monitors_production/` (new), `Cargo.toml` (change), `tests/example_projects_test.rs` (change), `docs/llm/SRC_STRUCTURE.md` + `docs/llm/TEST_STRUCTURE.md` (change - new rows) | spec `41_property_monitors.rfs` + the buggy variant (Ack transition dropped, per `tests/monitor_integration_test.rs`); production build with `--with-property-monitors`; debug run surfaces the violation at the step API, release build documented as zero-cost; C leg note (`-DRFS_PROPERTY_MONITORS`) | |
| 7.3 | `callback_minimal_adapters` | `crates/examples/callback_minimal_adapters/` (new), `Cargo.toml` (change), `tests/example_projects_test.rs` (change), `docs/llm/SRC_STRUCTURE.md` + `docs/llm/TEST_STRUCTURE.md` (change - new rows) | drive a real generated spec through `CallbackFsm` and the no_std `MinimalFsm` wrapper - the two rfs-adapters never demonstrated in an example | |
| 8. | Positive integration examples - plain dirs (4) | see subtasks | Parent task: four script-driven plain dirs (NOT workspace members) with READMEs per the template + test legs in `tests/example_projects_test.rs` (availability-gated where a real tool is needed). Rationale: these scenarios need no build files beyond scripts - plain dirs keep them out of the workspace while staying testable | |
| 8.1 | `mc_walkthrough` | `crates/examples/mc_walkthrough/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | generate MC binaries for BOTH backends on a passing spec, run exact mode; then a failing variant -> text + JSON CE documents; README walks a report end to end (status line, summary fields, CE structure) | |
| 8.2 | `json_schema_ci` | `crates/examples/json_schema_ci/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | script emitting `rfs info format <name> --output schema` for the five JSON document formats and validating REAL tool output (parse JSON, mc report, validate --json, info --json, CE trace) against them with `python3 -m jsonschema` (availability-gated; 4.26.0 present in this environment) | |
| 8.3 | `specgen_mutation_loop` | `crates/examples/specgen_mutation_loop/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | the rapid-prototyping loop as a runnable script - gen-spec random + mutation, validate, mc-ref, fix, repeat - with pinned transcripts in the README | |
| 8.4 | `tool_resolution` | `crates/examples/tool_resolution/` (new), `tests/example_projects_test.rs` (change), `docs/llm/TEST_STRUCTURE.md` (change - new rows) | run script setting explicit `RFS_GCC`/`RFS_RUSTC`/`RFS_CLANG` paths and showing the `--print-script` "# tools:" lines proving precedence (flag > env var > standard var > PATH); Makefile with a gcc/clang choice | |
| 9. | Upgrade existing example projects to the template | `crates/examples/sync_handler/README.md` (change), `crates/examples/async_handler/README.md` (change), `crates/examples/fuzz_rust/README.md` (change), `crates/examples/fuzz_c/README.md` (change), `crates/examples/c_sync_handler/Makefile` (change), `crates/examples/c_sync_handler/README.md` (change), `tests/example_projects_test.rs` (change - clang leg), `docs/llm/TEST_STRUCTURE.md` (change - new row) | Bring the five existing READMEs to the template (What it demonstrates / The machine with mermaid diagrams / Files layout / Build-run-test / Expected results) - content preserved, structure + diagrams added. `c_sync_handler` Makefile: the existing `CC ?= gcc` becomes a DOCUMENTED explicit option - `make CC=clang` builds the same scenario warning-free under clang (availability-gated in the test leg), mirroring the generated-C compiler-agnosticism guarantee; README shows both builds + the `RFS_GCC`/`RFS_CLANG` equivalents when driving rfs. Rationale: one template across all 15 example projects is what makes the USAGE.md examples table trustworthy | |
| 10. | Write `docs/user/USAGE.md` + wire it into the user index | `docs/user/USAGE.md` (new), `docs/user/README.md` (change) | The full layout from PHASE_17.md "Proposed USAGE.md layout": section 1 command map; section 2 external-tool resolution (the RFS_* table with used-by column, precedence model stated once, clang-no-standard-tier + z3 discovery-chain notes, special vars); sections 3-10 one per command set, each with the pattern table (CLI pattern | options - detailed description | generated files / output | possible errors (exit codes)) + naming/placement rules + error details and fixes; the codegen section states take-or-emit EXPLICITLY (`--out BASE` derivation, `--fsm-file` taken-if-present / emitted-if-missing, partial-pair usage error, stale-reuse warning); the mc-ref section carries real CE-trace walkthroughs (reuse task 6's captured traces - deadlock lasso + property violation, text and JSON, field by field); section 11 the example-projects table (`| title | description + rationale | ref to README.md |`) covering ALL 15 projects. README.md: USAGE.md row in "Which doc for which question?" + toolchain-at-a-glance mention where it fits. Rationale: the exhaustive CLI reference is the phase's central new deliverable; it depends on tasks 6-9 (real traces + real example READMEs to reference) and task 1 (no contradictions with MODEL_CHECKING.md) | |
| 11. | Training-set update | see subtasks | Parent task: bring the SFT set in line with the current toolchain - rewrite the three stale mc-ref entries, add coverage for the new USAGE.md material and the new examples, then regenerate snippets + aggregate per KB.md section 4.6. Rationale: entries claiming "mc-ref has no fuzz mode" actively mis-train the model | |
| 11.1 | Rewrite the three stale mc-ref entries | `training/frontend_cli/rfs_training.jsonl` (change), `training/model_checking/rfs_training.jsonl` (change) | frontend_cli #11 (flags + exit codes per mode), frontend_cli #14 (mc-ref vs generated binary - fuzz/smt-bounded now in-process; the generated binary's unique value is `--threads`/`--fast` + production-identical engine), model_checking #29 (quick MC without compilation - all three modes). Same user questions, rewritten assistant answers | |
| 11.2 | New entries for USAGE.md material + new examples | `training/model_checking/rfs_training.jsonl` (change), `training/codegen_rust/rfs_training.jsonl` (change), `training/frontend_cli/rfs_training.jsonl` (change), `training/project_tooling/rfs_training.jsonl` (change) | CE-trace reading (model_checking - deadlock lasso + violation trace from task 6's real captures); artifact take-or-emit semantics (codegen_rust - `--out BASE` / `--fsm-file` taken-vs-emitted / stale-reuse warning); RFS_* tool-resolution precedence (frontend_cli - flag > env > standard var > PATH, `--print-script` inspection); new example projects walkthrough (project_tooling - what each of the 10 new examples demonstrates + how to run it) | |
| 11.3 | Regenerate snippets + aggregate | `training/<cat>/snippets/` (regenerated), `all_rfs_training.jsonl` (regenerated) | `cargo extract-training-snippets && cargo build-training && cargo test-training` - the snippet counts may shift if new entries carry new ```rfs fences; record the new totals in the task status | |
| 12. | Phase close-out | `docs/roadmap/phase_17/PHASE_17_STATUS.md` (new), `TASKS.md` (change - append), `docs/roadmap/CURRENT_STATE.md` (change), `START.md` (change), `BUILD.md` (change) | Full verification: `cargo test` (>= 573 - new example-project + member tests add to it), `--workspace`, `--features smt`, `cargo test-generated-code` (exactly 1683), `cargo clippy --all-targets` at baseline, `cargo fmt --check` clean; doc gates `docs_md_test` + `dev_docs_test` green. Write PHASE_17_STATUS.md per FORMATTING.md section 4 (files added/changed, functionality deltas = none, tests, findings). Append the TASKS.md entry (first-paragraph template only). CURRENT_STATE.md: phase-17 row + refresh the stale header date. START.md: the docs/user doc-map row gains USAGE.md. BUILD.md: the example-projects test row updated with the new legs' counts. Rationale: protocol step s-t - the phase is not done until the record is complete | |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|

## Other information

- Execution order: tasks 1-5 are independent (any order); 6-9 independent of each
  other and of 1-5 (all append to `tests/example_projects_test.rs` - sequential
  execution avoids conflicts); 10 requires 1 + 6-9 done; 11 requires 10 (+ 6 for
  the real traces); 12 last.
- Every task that touches docs runs `cargo test --test docs_md_test` (user set)
  and/or `cargo test --test dev_docs_test` (dev/llm sets + KB.md) as its gate;
  every task that touches source or tests runs the BUILD.md section 5 checklist
  minimum (`cargo build && cargo test && cargo clippy --all-targets &&
  cargo fmt --check`).
- New example READMEs and USAGE.md use mermaid `stateDiagram-v2` / `flowchart`
  only (the subset rfs itself generates - guaranteed to parse under the gate).
- The negative examples' pinned excerpts must come from REAL runs in this
  environment (capture during implementation, paste into README + test
  assertions) - never hand-written approximations.
- Risk: `example_projects_test` runtime grows with four plain-dir script legs -
  keep each script's work bounded (small specs, no fuzzing, `--max-states`
  caps where relevant).
- Risk: task 7's new workspace members change `cargo test --workspace` counts -
  record the exact new totals in PHASE_17_STATUS.md.
