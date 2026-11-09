<!-- llm-budget: 250 -->
# RFS Change Impact (Class B)

Change type to affected source files + tests + doc fragments, at file
granularity. Mirrors and extends the `BUILD.md` section 5 checklist.

<a id="llm:change-impact"></a>
## Impact table

**How to use.** Pick the row matching your change -> edit the listed source
files -> run the listed tests -> update the listed doc fragments (the sync
gate's output after `cargo test --test dev_docs_test` names exactly which
ones). Every change also owes the universal row at the bottom. "Fragments" =
Class A sections under docs/dev/ + the Class B cards; repo-root docs
(DESIGN.md / KB.md / BUILD.md) are listed where their content must move with
the change.

| Change type | Affected source files | Tests to run | Doc fragments to update |
|---|---|---|---|
| RFS syntax/grammar | `src/lang/parser.rs`, `src/cli/info_format/ebnf_grammar.rs` (grammar model), `src/lang/validator.rs` if typing changes; add valid+invalid examples FIRST (`examples/valid|invalid`) | `cargo test --test example_corpus_test` + full checklist (BUILD.md section 5) + parser fuzz target | dev:parser-grammar / -expressions, dev:validator-overview, llm:parser, llm:validator; DESIGN.md sections 5-6 + KB.md sections 3/6.5 |
| IR schema / wire format | `src/lang/ir.rs` (`to_json`/`from_json`, `IR_VERSION` - bump if incompatible, read policy {0,1}) + EVERY consumer: parser, printer, validator, codegen_*, backends/*, mc_plan, ref_mc, smt_*, monitor, info, specgen | ir_roundtrip fuzz target + `cargo test --test frontend_test` + `codegen_test` + `toolchain_fuzz_test`; full checklist | dev:ir-model, dev:ir-wire-format, llm:ir-json-rw; DESIGN.md section 7 |
| Parser internals (no grammar change) | `src/lang/parser.rs`, `src/lang/error.rs` (spans) | `example_corpus_test` + `frontend_test` + parser fuzz target | dev:parser-entry-error / -expressions, llm:parser |
| Validator checks | `src/lang/validator.rs`; add invalid examples FIRST | `example_corpus_test` + validator fuzz target; full checklist | dev:validator-fatal-pass / -warnings, llm:validator; DESIGN.md sections 19-20 if a decision table moves |
| Codegen core / plugin API | `src/codegen/mod.rs`, `src/codegen/fsm.rs`, `src/codegen/mc_concepts.rs`, `src/codegen/test_plan.rs` + both backends when the API changes | `codegen_test` + `toolchain_fuzz_test` + `cargo test-generated-code`; full checklist | dev:codegen-plugin-api / -fsm-helpers / -mc-concepts / -test-plan, llm:codegen-core; src/codegen/CODEGEN.md |
| Rust backend | `src/codegen/backends/rust/mod.rs`, `rust_fsm.rs`, `rust_mc.rs`, `rust_test.rs` | `codegen_test` + `prod_parity_test` + `backend_parity_test` (+ `benchmark_mc_test` / `liveness_mc_test` for MC changes) + `cargo test-generated-code` | dev:backend-rust-plugin / -fsm / -mc / -test, llm:backend-rust |
| C backend | `src/codegen/backends/c/mod.rs`, `c_fsm.rs`, `c_mc.rs`, `c_test.rs` | same parity suites (they compile with gcc -Wall) + `toolchain_fuzz_test`; full checklist | dev:backend-c-plugin / -fsm / -mc / -test, llm:backend-c |
| MC engine (exact / liveness) | `src/mc/plan.rs`, `src/mc/ref_mc.rs` AND the generated twins in `src/codegen/backends/{rust,c}/mc.rs` must mirror (parity by construction) | `ref_mc_sweep_test` + `ref_mc_diff_test` + `ref_mc_diag_test` + `benchmark_mc_test` / `liveness_mc_test` + parity suites; full checklist | dev:mc-engine-plan / -exact / -liveness / -verify, llm:mc-engine; SEMANTICS.md section 9 if semantics move |
| SMT bounded mode (feature-gated) | `src/mc/smt_plan.rs`, `src/mc/smt_emit.rs` | `smt_bounded_test` with `--features smt` + z3 env vars (TOOLCHAIN.md section 5.2); spot-compile generated C SMT output per section 5.3 | dev:mc-engine-smt, llm:mc-engine |
| Monitor compiler | `src/codegen/monitor.rs` (+ backends render the SAME plan - parity) | `monitor_integration_test` + `monitor_ref_test` + `psl_translation_test`; full checklist | dev:monitor-compiler-signals / -nodes / -compile / -reference, llm:monitor-compiler |
| CLI command (new / changed) | `src/main.rs` (dispatch + usage text), the command module, shared infra as needed (`src/cli/frontend.rs`, `src/cli/ext_tool.rs`, `src/cli/log.rs`) | `frontend_test` (byte pins) + `dev_docs_test` (ORPHAN RULE: new fragments need //! Docs: owners); full checklist | dev:cli-commands-* + llm:cli-commands; KB.md section 4.5 |
| Diagram format | `src/cli/commands/diagram/mod.rs` (registry), `src/cli/commands/diagram/<fmt>.rs`, ext_tool preset if a new tool | `frontend_test` (byte pins) + `diagram_render_parity_test`; full checklist | dev:cli-commands-diagram, llm:cli-commands |
| External-tool layer | `src/cli/ext_tool.rs` (+ consumers: test_cmd, diagram, rfs-build resolution docs) | `frontend_test` (script byte pins) + `e2e_fuzz` in frontend_test (availability-gated); full checklist | dev:cli-commands-ext-tool, llm:cli-commands; TOOLCHAIN.md sections 6-7 for install facts |
| Workspace crates | `crates/rfs-build/*`, `crates/rfs-adapters/*`, `crates/examples/*` | `cargo test --workspace` + `cargo test -p rfs-adapters --all-features` + run sync/async_handler (need toolchain built or RFS_BIN) + `example_projects_test`; full checklist | dev:workspace-crates-build / -adapters / -examples, llm:workspace-crates |
| Fuzz targets / training / code-test harness | `fuzz/fuzz_targets/*`, `src/training.rs`, `src/bin/*` | `toolchain_fuzz_test` + `training_test` + `cargo test-generated-code`; training trio (`extract-training-snippets && build-training && test-training`); bounded fuzz smoke per BUILD.md section 6 (never weaken a target) | dev:toolchain-fuzz-targets / -training / -code-tests, llm:toolchain-fuzz |
| Spec generation | `src/lang/specgen.rs` (+ consumers: gen_spec_cmd, fuzz targets, toolchain_fuzz_test seeds) | in-module specgen tests + `toolchain_fuzz_test` (pinned seeds); full checklist | dev:cli-commands-mc-ref-gen-spec, llm:cli-commands |
| **Universal (every change)** | - | `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` (+ `--workspace` when crates touched) + `cargo test --test dev_docs_test` after any doc-affecting edit; append a TASKS.md entry | whatever the sync gate names; plan/roadmap rows for phase work |
