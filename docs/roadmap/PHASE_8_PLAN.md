# Phase 8 Plan

## Context

Phase 8 makes the RFS source tree self-documenting in both directions: every
technical documentation statement traces down to the implementing source code,
and every source file points back up to its documentation fragment, enforced by
a machine-checked sync gate. It matters now because after seven phases the
project's complexity (72 hand-written `.rs` files, ~46k lines, 23 integration
suites, two backends) outgrows what a developer or an LLM session can re-derive
by exploration - and context budgets for assistant work must be spent on
reasoning and edits, not on rediscovering where things live. Tasks are split
fine-grained (subtasks) so each one fits a single working session's context.

## Planned tasks

| # | Task | Files (add/change) | Depends on | Status |
|---|------|--------------------|------------|--------|
| 1.1 | Sync gate suite + conventions + smoke tests. Add `tests/dev_docs_test.rs`: all check types (doc→code reference resolution from inline code spans, code→doc pointer presence, TASK_MAP coverage, no orphaned fragments, Class B line budgets) + negative-control fixtures (broken reference, missing pointer, orphan fragment, budget overflow). Mechanism: doc checks are always active over whatever exists under `docs/dev/` + `docs/llm/`; the pointer-check scope is driven by a coverage manifest (initially empty); the completeness test (manifest == filesystem scan of all 72 `.rs`) is added in 1.10. Fix conventions: fragment IDs (`dev:<name>`, `llm:<name>`), doc→code reference syntax, Class B budget header format | add: `tests/dev_docs_test.rs` + fixtures | - | COMPLETE (2026-10-28) |
| 1.2 | Chunk: index + cross-cutting skeletons. Add `docs/dev/README.md`, `docs/llm/README.md` (conventions, fragment-ID scheme, context-budget rules) + skeletons for ARCHITECTURE, CODE_MAP, DEPENDENCIES, DECISIONS, ALGORITHMS, REFERENCES, TASK_MAP, CHANGE_IMPACT (stable fragment-ID headers, minimal valid content). Pointers in root files: `src/lib.rs`, `main.rs`, `error.rs`, `cli_log.rs`, `frontend.rs`, `printer.rs` (6; root pointers also name the cross-cutting fragments). Extend manifest; run gate on chunk | add: 10 `.md`; change: 6 `.rs` | 1.1 | DONE |
| 1.3 | Chunk: parser area. Skeletons `docs/dev/parser.md`, `docs/llm/parser.md`; pointers in `src/parser.rs`, `src/ebnf_grammar.rs` (2). Extend manifest; run gate on chunk | add: 2 `.md`; change: 2 `.rs` | 1.2 | DONE |
| 1.4 | Chunk: validator + IR area. Skeletons `docs/dev/validator.md`, `docs/dev/ir_json_rw.md`, `docs/llm/validator.md`, `docs/llm/ir_json_rw.md`; pointers in `src/validator.rs`, `ir.rs`, `json_rw.rs`, `json_schema.rs`, `spec_hash.rs` (5). Extend manifest; run gate on chunk | add: 4 `.md`; change: 5 `.rs` | 1.3 | DONE |
| 1.5 | Chunk: codegen core. Skeletons `docs/dev/codegen_core.md`, `docs/llm/codegen_core.md`; pointers in `src/codegen.rs`, `codegen_fsm.rs`, `codegen_mc.rs`, `codegen_test.rs` (4). Extend manifest; run gate on chunk | add: 2 `.md`; change: 4 `.rs` | 1.4 | DONE |
| 1.6 | Chunk: Rust backend. Skeletons `docs/dev/backend_rust.md`, `docs/llm/backend_rust.md`; pointers in `src/backends/mod.rs`, `rust.rs`, `rust_fsm.rs`, `rust_mc.rs`, `rust_test.rs` (5). Extend manifest; run gate on chunk | add: 2 `.md`; change: 5 `.rs` | 1.5 | DONE |
| 1.7 | Chunk: C backend. Skeletons `docs/dev/backend_c.md`, `docs/llm/backend_c.md`; pointers in `src/backends/c.rs`, `c_fsm.rs`, `c_mc.rs`, `c_test.rs` (4). Extend manifest; run gate on chunk | add: 2 `.md`; change: 4 `.rs` | 1.6 | DONE |
| 1.8 | Chunk: MC engine + monitor. Skeletons `docs/dev/mc_engine.md`, `docs/dev/monitor_compiler.md`, `docs/llm/mc_engine.md`, `docs/llm/monitor_compiler.md`; pointers in `src/mc_plan.rs`, `smt_plan.rs`, `smt_emit.rs`, `ref_mc.rs`, `monitor.rs` (5). Extend manifest; run gate on chunk | add: 4 `.md`; change: 5 `.rs` | 1.7 | DONE |
| 1.9 | Chunk: CLI + diagrams area. Skeletons `docs/dev/cli_commands.md`, `docs/llm/cli_commands.md`; pointers in `src/info.rs`, `info_format.rs`, `gen_spec_cmd.rs`, `test_cmd.rs`, `mc_ref_cmd.rs`, `diagram.rs`, `diagrams/{mod,dot,mermaid,plantuml,tables}.rs`, `specgen.rs`, `ext_tool.rs` (13). Extend manifest; run gate on chunk | add: 2 `.md`; change: 13 `.rs` | 1.8 | DONE |
| 1.10 | Chunk: workspace crates + fuzz + bins (final). Skeletons `docs/dev/workspace_crates.md`, `docs/llm/workspace_crates.md`, `docs/dev/toolchain_fuzz.md`, `docs/llm/toolchain_fuzz.md`; pointers in all 18 files under `crates/` (rfs-build, rfs-adapters incl. tests, examples), 5 files under `fuzz/fuzz_targets/`, `src/bin/*.rs` (4), `src/training.rs`. Add completeness test: manifest == filesystem scan of all 72 `.rs`; run full gate | add: 4 `.md`; change: 27 `.rs` (arq.rs is build-generated - see task corrections) | 1.9 | DONE |
| 2.1 | `docs/dev/ARCHITECTURE.md`: pipeline overview (spec → parse → validate → IR → codegen/MC/diagram) + component diagram (mermaid); every technical statement carries a doc→code reference | change: `docs/dev/ARCHITECTURE.md` | 1.2 | DONE |
| 2.2 | `docs/dev/ARCHITECTURE.md`: functionality hierarchy tree + relationship map between components | change: `docs/dev/ARCHITECTURE.md` | 2.1 | DONE |
| 3.1 | `docs/dev/CODE_MAP.md`: `src/` frontend area - root (6), parser (2), validator+IR (5) = 13 files: responsibility, key items, doc links both ways | change: `docs/dev/CODE_MAP.md` | 1.4 | DONE |
| 3.2 | `docs/dev/CODE_MAP.md`: codegen core (4) + both backends (9) + MC/monitor (5) = 18 files; Rust and C covered with equal depth | change: `docs/dev/CODE_MAP.md` | 1.8 | DONE |
| 3.3 | `docs/dev/CODE_MAP.md`: CLI/diagrams area (13) + bins/training (5) = 18 files | change: `docs/dev/CODE_MAP.md` | 1.9 | DONE |
| 3.4 | `docs/dev/CODE_MAP.md`: `crates/` (18) + `fuzz/` (5) = 23 files | change: `docs/dev/CODE_MAP.md` | 1.10 | DONE |
| 4.1 | `docs/dev/DEPENDENCIES.md`: file-level dependency graph (mermaid, split into per-area subgraphs to stay within the renderer's layout limits) | change: `docs/dev/DEPENDENCIES.md` | 1.2 | DONE |
| 4.2 | `docs/dev/DEPENDENCIES.md`: function-level graphs - parser, validator | change: `docs/dev/DEPENDENCIES.md` | 1.4 | DONE |
| 4.3 | `docs/dev/DEPENDENCIES.md`: function-level graphs - codegen core, Rust backend | change: `docs/dev/DEPENDENCIES.md` | 1.6 | DONE |
| 4.4 | `docs/dev/DEPENDENCIES.md`: function-level graphs - C backend, `mc_plan` + `ref_mc` (MC engine) | change: `docs/dev/DEPENDENCIES.md` | 1.8 | DONE |
| 4.5 | `docs/dev/DEPENDENCIES.md`: function-level graph - monitor compiler (+ SMT path where relevant) | change: `docs/dev/DEPENDENCIES.md` | 1.8 | DONE |
| 5.1 | `docs/dev/DECISIONS.md`: rationale index for `DESIGN.md` §9 (subsections 9.1-9.11) + §19 bullets - 2-4 line "why" each + link to the DESIGN.md section + implementing code | change: `docs/dev/DECISIONS.md` | 1.2 | DONE |
| 5.2 | `docs/dev/DECISIONS.md`: §20 D1-D10 + §21 D-P3-1..6 | change: `docs/dev/DECISIONS.md` | 1.2 | DONE |
| 5.3 | `docs/dev/DECISIONS.md`: §22 D-P4-1..14 + §23 D-P5-1..14 | change: `docs/dev/DECISIONS.md` | 1.2 | DONE |
| 5.4 | `docs/dev/DECISIONS.md`: §24 D-P6-1..11 + §25 D-P7-1..13 | change: `docs/dev/DECISIONS.md` | 1.2 | DONE |
| 6.1 | `docs/dev/ALGORITHMS.md`: parser construction (LL(1)/chumsky) + validation passes - L1 abstract / L2 key structures+invariants / L3 optional implementation details (important ones emphasized), every level linked to the implementing code | change: `docs/dev/ALGORITHMS.md` | 1.4 | DONE |
| 6.2 | `docs/dev/ALGORITHMS.md`: IR normalization + codegen pipeline (same L1/L2/L3 structure) | change: `docs/dev/ALGORITHMS.md` | 1.5 | DONE |
| 6.3 | `docs/dev/ALGORITHMS.md`: MC engine - exact / liveness / SMT paths | change: `docs/dev/ALGORITHMS.md` | 1.8 | DONE |
| 6.4 | `docs/dev/ALGORITHMS.md`: monitor compilation (PSL translation) | change: `docs/dev/ALGORITHMS.md` | 1.8 | PENDING |
| 7.1 | `docs/dev/REFERENCES.md`: entries for parsing theory, automata theory, compiler construction (one-line annotation + which RFS component uses it); wire links from ALGORITHMS parser/validation/codegen sections | change: `docs/dev/REFERENCES.md`, `docs/dev/ALGORITHMS.md` | 6.2 | PENDING |
| 7.2 | `docs/dev/REFERENCES.md`: entries for model checking, SMT/SAT solving; wire links from ALGORITHMS MC/monitor sections | change: `docs/dev/REFERENCES.md`, `docs/dev/ALGORITHMS.md` | 6.4 | PENDING |
| 8.1 | Deep dive: `docs/dev/parser.md` (full content) | change: `docs/dev/parser.md` | 1.3 | PENDING |
| 8.2 | Deep dive: `docs/dev/validator.md` (full content) | change: `docs/dev/validator.md` | 1.4 | PENDING |
| 8.3 | Deep dive: `docs/dev/ir_json_rw.md` (IR + `json_rw` + `json_schema` + `spec_hash`) | change: `docs/dev/ir_json_rw.md` | 1.4 | PENDING |
| 9.1 | Deep dive: `docs/dev/codegen_core.md` (`codegen.rs`, `codegen_fsm.rs`, `codegen_mc.rs`, `codegen_test.rs`, plugin API) - backend parity 1/2a | change: `docs/dev/codegen_core.md` | 1.5 | PENDING |
| 9.2 | Deep dive: `docs/dev/backend_rust.md` (`rust.rs`, `rust_fsm.rs`, `rust_mc.rs`, `rust_test.rs`) - backend parity 1/2b | change: `docs/dev/backend_rust.md` | 1.6 | PENDING |
| 10.1 | Deep dive: `docs/dev/backend_c.md` - core translation + FSM codegen (`c.rs`, `c_fsm.rs`) - backend parity 2/2a | change: `docs/dev/backend_c.md` | 1.7 | PENDING |
| 10.2 | `docs/dev/backend_c.md` - MC + test generation (`c_mc.rs`, `c_test.rs`) + parity notes vs the Rust backend - backend parity 2/2b | change: `docs/dev/backend_c.md` | 10.1 | PENDING |
| 11.1 | Deep dive: `docs/dev/mc_engine.md` - exact + liveness paths (`mc_plan.rs`, `ref_mc.rs`) | change: `docs/dev/mc_engine.md` | 1.8 | PENDING |
| 11.2 | `docs/dev/mc_engine.md` - SMT path (`smt_plan.rs`, `smt_emit.rs`) + diagnostics | change: `docs/dev/mc_engine.md` | 11.1 | PENDING |
| 11.3 | Deep dive: `docs/dev/monitor_compiler.md` (`monitor.rs`, PSL translation) | change: `docs/dev/monitor_compiler.md` | 1.8 | PENDING |
| 12.1 | Deep dive: `docs/dev/cli_commands.md` (all CLI commands + library API surface) | change: `docs/dev/cli_commands.md` | 1.9 | PENDING |
| 12.2 | Deep dive: `docs/dev/workspace_crates.md` (`rfs-build`, `rfs-adapters`, examples) | change: `docs/dev/workspace_crates.md` | 1.10 | PENDING |
| 12.3 | Deep dive: `docs/dev/toolchain_fuzz.md` (fuzz targets + toolchain fuzz layer) | change: `docs/dev/toolchain_fuzz.md` | 1.10 | PENDING |
| 13.1 | Class B: full `docs/llm/TASK_MAP.md` - functionality → files/functions for EVERY CLI command, subsystem, and test suite; "read this fragment for this task" index extending `START.md` section 6 | change: `docs/llm/TASK_MAP.md` | 1.10 | PENDING |
| 13.2 | Class B: `docs/llm/CHANGE_IMPACT.md` (change type → affected source files + tests + doc fragments at file granularity) + finalize `docs/llm/README.md` context-budget rules | change: `docs/llm/CHANGE_IMPACT.md`, `docs/llm/README.md` | 13.1 | PENDING |
| 13.3 | Class B: 11 per-module compact cards within their line budgets (do after tasks 2-12 so one-line roles match final Class A content) | change: 11 module cards under `docs/llm/` | 13.2 | PENDING |
| 14 | Pointer/doc sync: `START.md` (section 2 documentation map + section 6 task→read table rows for `docs/dev/` and `docs/llm/`), `KB.md` (documentation map + section 8 roadmap row), `BUILD.md` (new suite row in the test-suite table + expected counts + section 5 docs-sync checklist update), cross-link from `docs/user/README.md` ("Where things live") to the developer set | change: `START.md`, `KB.md`, `BUILD.md`, `docs/user/README.md` | 2-13 | PENDING |
| 15 | Training-set update: `training/project_tooling/rfs_training.jsonl` entries on the new documentation layout and how an LLM session uses Class B docs (TASK_MAP fragment → locate code → CHANGE_IMPACT → update docs; sync-gate behavior); regenerate snippets (`cargo extract-training-snippets`) + aggregate (`cargo build-training` → `all_rfs_training.jsonl` at repo root) | change: `training/project_tooling/rfs_training.jsonl`, `training/project_tooling/snippets/`, `all_rfs_training.jsonl` | 14 | PENDING |
| 16 | Full verification + close-out: full verification suite (`cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` + `--workspace`, `cargo test-generated-code`, docs gate), write `docs/roadmap/PHASE_8_STATUS.md`, update `INTRO.md` phase-state row to COMPLETE, append `TASKS.md` entry | add: `docs/roadmap/PHASE_8_STATUS.md`; change: `docs/roadmap/INTRO.md`, `TASKS.md` | 2-15 | PENDING |

Notes on the explicit-update rule: task 1.1 is the tests-update task (new sync
gate suite) and the examples/fixtures task (pinned negative-control fixtures);
task 15 is the training-set update task. Backend parity is carried by tasks
9.1/9.2 (Rust) and 10.1/10.2 (C, equal depth), plus both-backend coverage in
tasks 3.2 and 4.3/4.4. Task 13 was split as well for the same context-budget
reason as tasks 1-12. Environment note: external renderers (dot 2.43.0,
plantuml 1.2020.02, mmdc 12.0.0) and clang 18.1.3 are installed and smoke-
tested per TOOLCHAIN.md §6.1, so `--renderer external` paths and C fuzzing are
available; the docs gate itself needs no external tools (in-process
mermaid-rs-renderer). Toolchain environment note: in this container cargo is
NOT on the default PATH - run with
`export RUSTUP_HOME=/workspace/host/rustup CARGO_HOME=/workspace/host/.cargo PATH=/workspace/host/.cargo/bin:$PATH`
(default toolchain: stable, per `/workspace/host/rustup/settings.toml`).

## Corrections

| # | Date | What changed in the plan | Why |
|---|------|--------------------------|-----|
| 1 | 2026-10-28 | Code→doc pointer scope extended from "`src/`, `crates/*/src/`, `fuzz/fuzz_targets/`" (PHASE_8.md wording) to ALL 72 hand-written `.rs` files, including crate-root `build.rs` files and workspace test files (`crates/rfs-adapters/tests/`, `crates/rfs-build/tests/`) | User principle: all source code should have pointers to corresponding documentation. Verified that generated example bodies live in `OUT_DIR` (outside the tree), so no in-tree file is machine-generated and none needs exclusion. |
| 2 | 2026-10-28 | "D1-D10" from PHASE_8.md refers to the D-numbered decisions in `DESIGN.md` §20 (Phase 2 core DSL); `DECISIONS.md` additionally indexes the unnumbered entries of §9 (subsections 9.1-9.11) and §19 (bullets) | The actual decision-ID landscape in `DESIGN.md` differs from the phase-description shorthand; the rationale index must cover every decision entry, not only D-labeled ones. |
