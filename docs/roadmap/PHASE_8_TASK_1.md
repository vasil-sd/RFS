# Phase 8, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

Establish the machine-checked documentation sync gate and the traceability
conventions it enforces (subtask 1.1), then grow the documentation skeleton
and the code→doc pointer coverage chunk by chunk (subtasks 1.2-1.10) so that
at every step the repo is green under the gate, and at the end ALL 72
hand-written `.rs` files carry pointers to stable doc fragment IDs and all 32
skeleton docs exist with their fragment IDs fixed for the content tasks 2-13.

## Context

- **File scope (plan correction #1)**: the pointer scope is ALL 72 hand-written
  `.rs` files = `src/` (49, incl. `src/bin/` x4 and `src/training.rs`) +
  `crates/` (18: rfs-build 2, rfs-adapters 7 incl. tests, examples 9) +
  `fuzz/fuzz_targets/` (5). The root `tests/` directory (27 files) is OUT of
  scope - each integration suite is documented in the `BUILD.md` suite table
  and self-describes in its module doc. Generated example bodies live in
  `OUT_DIR` (outside the tree), so no in-tree file is machine-generated.
- **No new dependencies** (PHASE_8.md): the gate reuses `comrak` 0.55
  (existing dev-dependency, mdr's parser) + std only. Markdown quality of all
  new docs is already covered by `tests/docs_md_test.rs` (it walks every repo
  `.md`), so the new suite focuses on traceability only.
- **Fixture safety**: `tests/fixtures/dev_docs/*.rs` are NOT cargo targets
  (cargo auto-detects only `tests/*.rs` and `tests/*/main.rs`), are never
  compiled, and are outside the training snippet extractor's scope (it scans
  `training/` categories only). The fixture `.md` files DO enter the
  `docs_md_test` checked set - they must stay valid Markdown with no relative
  links.
- **Mechanism**: doc checks (doc→code refs, fragment well-formedness, orphans,
  budgets) are always active over whatever exists under `docs/dev/` +
  `docs/llm/`; the code→doc pointer scope is driven by
  `tests/dev_docs_manifest.txt` (empty at 1.1, grows per chunk); the
  completeness test (manifest == filesystem scan of all 72) is added in 1.10.
- **Conventions pinned by subtask 1.1** (canonical home: the module doc of
  `tests/dev_docs_test.rs` until task 1.2 adds the READMEs):
  - Fragment IDs: `dev:<name>` / `llm:<name>`, `<name>` kebab-case
    `[a-z0-9]+(-[a-z0-9]+)*`; defined by an anchor line
    `<a id="dev:name"></a>` immediately followed (blank lines allowed) by a
    Markdown heading; unique across the doc set.
  - doc→code references: inline code spans `path/to/file.rs` or
    `path/to/file.rs::item` - repo-rooted, ≥1 path separator, `.rs`/`.c`/`.h`
    only, `::item` for `.rs` only; stable identifiers, NEVER line numbers.
    Any backticked string matching this shape MUST resolve (fenced code
    blocks are exempt).
  - code→doc pointers: exactly one `//! Docs: dev:a, llm:b` line per
    manifest-listed file; key items may carry one-line `/// Docs:` pointers
    (convention, not gate-checked).
  - Class B budget header: `<!-- llm-budget: N -->` within the first 10 lines
    of each `docs/llm/*.md`; total line count ≤ N.
  - Orphan rule: every defined fragment must be named by ≥1 source file's
    `//! Docs:` line (every fragment has an owner).
- **Planned primary fragment IDs per doc** (fixed when the skeleton is written;
  sub-fragments may be added later by the content tasks, each owned by the
  doc's source files). "Owner" = a file whose `//! Docs:` line names the ID:

| Doc | Planned primary fragment(s) | Owner(s) |
|-----|------------------------------|----------|
| `docs/dev/README.md` | `dev:index` | `src/lib.rs` |
| `docs/llm/README.md` | `llm:index` | `src/lib.rs` |
| `docs/dev/ARCHITECTURE.md` | `dev:architecture-pipeline`, `dev:architecture-components`, `dev:architecture-hierarchy` | `src/lib.rs` |
| `docs/dev/CODE_MAP.md` | `dev:code-map-frontend`, `dev:code-map-codegen`, `dev:code-map-cli`, `dev:code-map-workspace` | frontend: the 6 root files; codegen/cli/workspace: `src/lib.rs` (co-owners added in later chunks) |
| `docs/dev/DEPENDENCIES.md` | `dev:deps-file-level`; function-level: `dev:deps-fn-parser`, `dev:deps-fn-validator`, `dev:deps-fn-codegen-rust`, `dev:deps-fn-backend-c`, `dev:deps-fn-mc-engine`, `dev:deps-fn-monitor` | file-level: `src/lib.rs`; fn-*: `src/parser.rs`, `src/validator.rs`, `src/codegen.rs`, `src/backends/c.rs`, `src/mc_plan.rs`, `src/monitor.rs` (anchors added in chunks 4.2-4.5) |
| `docs/dev/DECISIONS.md` | `dev:decisions-early`, `dev:decisions-p2-p3`, `dev:decisions-p4-p5`, `dev:decisions-p6-p7` | `src/lib.rs` (anchors added in chunks 5.1-5.4) |
| `docs/dev/ALGORITHMS.md` | `dev:algo-parser-validation`, `dev:algo-ir-codegen`, `dev:algo-mc-engine`, `dev:algo-monitor` | `src/parser.rs`, `src/codegen.rs`, `src/mc_plan.rs`, `src/monitor.rs` (anchors added in chunks 6.1-6.4) |
| `docs/dev/REFERENCES.md` | `dev:references-theory`, `dev:references-model-checking` | `src/parser.rs`, `src/ref_mc.rs` (anchors added in chunks 7.1-7.2) |
| `docs/llm/TASK_MAP.md` | `llm:task-map` | `src/lib.rs` |
| `docs/llm/CHANGE_IMPACT.md` | `llm:change-impact` | `src/lib.rs` |
| `docs/dev/parser.md` / `docs/llm/parser.md` | `dev:parser-overview` / `llm:parser` | `src/parser.rs`, `src/ebnf_grammar.rs` |
| `docs/dev/validator.md` / `docs/llm/validator.md` | `dev:validator-overview` / `llm:validator` | `src/validator.rs` |
| `docs/dev/ir_json_rw.md` / `docs/llm/ir_json_rw.md` | `dev:ir-json-rw-overview` / `llm:ir-json-rw` | `src/ir.rs`, `src/json_rw.rs`, `src/json_schema.rs`, `src/spec_hash.rs` |
| `docs/dev/codegen_core.md` / `docs/llm/codegen_core.md` | `dev:codegen-core-overview` / `llm:codegen-core` | the 4 codegen files |
| `docs/dev/backend_rust.md` / `docs/llm/backend_rust.md` | `dev:backend-rust-overview` / `llm:backend-rust` | the 5 Rust-backend files |
| `docs/dev/backend_c.md` / `docs/llm/backend_c.md` | `dev:backend-c-overview` / `llm:backend-c` | the 4 C-backend files |
| `docs/dev/mc_engine.md` / `docs/llm/mc_engine.md` | `dev:mc-engine-overview` / `llm:mc-engine` | `src/mc_plan.rs`, `src/ref_mc.rs`, `src/smt_plan.rs`, `src/smt_emit.rs` |
| `docs/dev/monitor_compiler.md` / `docs/llm/monitor_compiler.md` | `dev:monitor-compiler-overview` / `llm:monitor-compiler` | `src/monitor.rs` |
| `docs/dev/cli_commands.md` / `docs/llm/cli_commands.md` | `dev:cli-commands-overview` / `llm:cli-commands` | the 13 CLI/diagram files (primary: `src/main.rs`) |
| `docs/dev/workspace_crates.md` / `docs/llm/workspace_crates.md` | `dev:workspace-crates-overview` / `llm:workspace-crates` | the 18 crate files (primary: `crates/rfs-build/src/lib.rs`) |
| `docs/dev/toolchain_fuzz.md` / `docs/llm/toolchain_fuzz.md` | `dev:toolchain-fuzz-overview` / `llm:toolchain-fuzz` | the 5 fuzz targets + `src/training.rs` (primary: `fuzz/fuzz_targets/parser.rs`) |

## Scope

### Depends on tasks

(none - first task of Phase 8)

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1. | 2-13 (all content tasks) | every doc written later must pass the gate; fragment IDs are stable from this task |
| 2. | 14 | `BUILD.md` suite row + expected counts reference this suite's test count |

### Affected source files

All 72 hand-written `.rs` files get a `//! Docs:` pointer line, chunk by chunk:

| Chunk | Files (count) |
|-------|---------------|
| 1.2 | `src/lib.rs`, `main.rs`, `error.rs`, `cli_log.rs`, `frontend.rs`, `printer.rs` (6) |
| 1.3 | `src/parser.rs`, `ebnf_grammar.rs` (2) |
| 1.4 | `src/validator.rs`, `ir.rs`, `json_rw.rs`, `json_schema.rs`, `spec_hash.rs` (5) |
| 1.5 | `src/codegen.rs`, `codegen_fsm.rs`, `codegen_mc.rs`, `codegen_test.rs` (4) |
| 1.6 | `src/backends/mod.rs`, `rust.rs`, `rust_fsm.rs`, `rust_mc.rs`, `rust_test.rs` (5) |
| 1.7 | `src/backends/c.rs`, `c_fsm.rs`, `c_mc.rs`, `c_test.rs` (4) |
| 1.8 | `src/mc_plan.rs`, `smt_plan.rs`, `smt_emit.rs`, `ref_mc.rs`, `monitor.rs` (5) |
| 1.9 | `src/info.rs`, `info_format.rs`, `gen_spec_cmd.rs`, `test_cmd.rs`, `mc_ref_cmd.rs`, `diagram.rs`, `diagrams/{mod,dot,mermaid,plantuml,tables}.rs`, `specgen.rs`, `ext_tool.rs` (13) |
| 1.10 | all 18 files under `crates/` (rfs-build 2, rfs-adapters 7, examples 9), `fuzz/fuzz_targets/{parser,validator,ir_roundtrip,codegen,common}.rs` (5), `src/bin/{build_training,extract_training_snippets,generate_code_tests,test_generated_code}.rs` (4), `src/training.rs` (1) = 28 |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/dev_docs_test.rs` | new | sync gate suite: 5 check types + 8 tests (main gate, positive control, 6 negative controls) |
| `tests/dev_docs_manifest.txt` | new | pointer-scope manifest; empty at 1.1, grows to exactly 72 entries by 1.10 |
| `tests/fixtures/dev_docs/` (7 files) | new | pinned fixtures: `good_doc.md`, `good_llm_doc.md`, `good_code.rs` (positive control), `broken_ref.md`, `missing_pointer.rs`, `dangling_pointer.rs`, `orphan_frag.md`, `budget_overflow.md` (negative controls) |

### Affected documentation files

32 skeletons added across chunks 1.2-1.10: `docs/dev/` = README, ARCHITECTURE,
CODE_MAP, DEPENDENCIES, DECISIONS, ALGORITHMS, REFERENCES, parser, validator,
ir_json_rw, codegen_core, backend_rust, backend_c, mc_engine,
monitor_compiler, cli_commands, workspace_crates, toolchain_fuzz (18); `docs/llm/`
= README, TASK_MAP, CHANGE_IMPACT + 11 module cards' files created in 1.2-1.9
as skeletons (parser, validator, ir_json_rw, codegen_core, backend_rust,
backend_c, mc_engine, monitor_compiler, cli_commands, workspace_crates,
toolchain_fuzz) (14). Skeleton = title + one-line purpose + stable fragment-ID
anchor(s) + `TBD (task N)` placeholders; Class B skeletons carry their budget
header from the start.

## Planned actions

### Action 1.1: sync gate suite + conventions + smoke tests (DONE 2026-10-28)

Built `tests/dev_docs_test.rs` with all five check types as reusable functions
(`check_doc_code_refs`, `check_code_pointers`, `check_task_map_coverage`,
`check_llm_budgets`, `check_no_orphans` + fragment-definition collection and
manifest validation), the main gate test over the real tree, and 7 fixture-
based control tests. doc→code references are extracted from comrak's AST
(inline code spans only - fenced blocks exempt); item existence is checked by
a line-anchored declaration scan (fn/struct/enum/trait/type/const/static/mod,
`async`/`const`/`unsafe fn`, impl headers) - no new dependencies. The manifest
starts empty, so the gate is green on the current tree; every check's failure
class is proven by a pinned fixture under `tests/fixtures/dev_docs/`.
Verification: `cargo test --test dev_docs_test` 8/8 pass; `docs_md_test`
still green (fixtures are valid Markdown); clippy + fmt clean.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/dev_docs_test.rs` | new | the gate suite + conventions in its module doc |
| `tests/dev_docs_manifest.txt` | new | empty manifest with format comments |
| `tests/fixtures/dev_docs/good_doc.md` | new | positive control: resolving refs + 2 owned fragments |
| `tests/fixtures/dev_docs/good_llm_doc.md` | new | positive control: budget header within budget + owned fragment |
| `tests/fixtures/dev_docs/good_code.rs` | new | positive control: valid `//! Docs:` line owning all 3 fixture fragments |
| `tests/fixtures/dev_docs/broken_ref.md` | new | negative control: missing file + missing item refs |
| `tests/fixtures/dev_docs/missing_pointer.rs` | new | negative control: no `//! Docs:` line |
| `tests/fixtures/dev_docs/dangling_pointer.rs` | new | negative control: pointer to undefined fragment |
| `tests/fixtures/dev_docs/orphan_frag.md` | new | negative control: defined fragment with no owner |
| `tests/fixtures/dev_docs/budget_overflow.md` | new | negative control: 6 lines vs budget 5 |

### Action 1.2: index + cross-cutting skeletons

Add `docs/dev/README.md` + `docs/llm/README.md` (conventions, fragment-ID
scheme, context-budget rules - restating the gate's module doc) and skeletons
for ARCHITECTURE, CODE_MAP, DEPENDENCIES, DECISIONS, ALGORITHMS, REFERENCES,
TASK_MAP, CHANGE_IMPACT with the planned primary fragment anchors from the
Context table (only fragments whose owners are in this chunk's manifest get
anchors now; later chunks add their anchors). Pointers in the 6 root files per
the ownership table. Extend the manifest with the 6 files; run the gate.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/dev/README.md`, `docs/llm/README.md` | new | index + conventions for both classes |
| 8 skeleton docs (ARCHITECTURE, CODE_MAP, DEPENDENCIES, DECISIONS, ALGORITHMS, REFERENCES under `docs/dev/`; TASK_MAP, CHANGE_IMPACT under `docs/llm/`) | new | stable fragment-ID headers + TBD placeholders; Class B with budget headers |
| 6 root `.rs` files | change | `//! Docs:` pointer lines per the ownership table |
| `tests/dev_docs_manifest.txt` | change | +6 entries |

### Action 1.3: parser area chunk

Skeletons `docs/dev/parser.md`, `docs/llm/parser.md` (fragments
`dev:parser-overview`, `llm:parser`); pointers in `src/parser.rs`,
`src/ebnf_grammar.rs`; manifest +2; run the gate.

### Action 1.4: validator + IR area chunk

Skeletons `docs/dev/validator.md`, `docs/dev/ir_json_rw.md`,
`docs/llm/validator.md`, `docs/llm/ir_json_rw.md`; pointers in `src/validator.rs`,
`ir.rs`, `json_rw.rs`, `json_schema.rs`, `spec_hash.rs`; manifest +5; run the gate.

### Action 1.5: codegen core chunk

Skeletons `docs/dev/codegen_core.md`, `docs/llm/codegen_core.md`; pointers in
`src/codegen.rs`, `codegen_fsm.rs`, `codegen_mc.rs`, `codegen_test.rs`; add the
`dev:code-map-codegen` anchor to CODE_MAP (co-owner `src/codegen.rs`);
manifest +4; run the gate.

### Action 1.6: Rust backend chunk

Skeletons `docs/dev/backend_rust.md`, `docs/llm/backend_rust.md`; pointers in
the 5 files under `src/backends/` (mod, rust, rust_fsm, rust_mc, rust_test);
manifest +5; run the gate.

### Action 1.7: C backend chunk

Skeletons `docs/dev/backend_c.md`, `docs/llm/backend_c.md`; pointers in the 4
files under `src/backends/` (c, c_fsm, c_mc, c_test); manifest +4; run the gate.

### Action 1.8: MC engine + monitor chunk

Skeletons `docs/dev/mc_engine.md`, `docs/dev/monitor_compiler.md`,
`docs/llm/mc_engine.md`, `docs/llm/monitor_compiler.md`; pointers in
`src/mc_plan.rs`, `smt_plan.rs`, `smt_emit.rs`, `ref_mc.rs`, `monitor.rs`;
manifest +5; run the gate.

### Action 1.9: CLI + diagrams chunk

Skeletons `docs/dev/cli_commands.md`, `docs/llm/cli_commands.md`; pointers in
the 13 files (`src/info.rs`, `info_format.rs`, `gen_spec_cmd.rs`, `test_cmd.rs`,
`mc_ref_cmd.rs`, `diagram.rs`, `diagrams/{mod,dot,mermaid,plantuml,tables}.rs`,
`specgen.rs`, `ext_tool.rs`); add the `dev:code-map-cli` anchor (co-owner
`src/main.rs` - extend its existing Docs line); manifest +13; run the gate.

### Action 1.10: workspace crates + fuzz + bins chunk (final)

Skeletons `docs/dev/workspace_crates.md`, `docs/llm/workspace_crates.md`,
`docs/dev/toolchain_fuzz.md`, `docs/llm/toolchain_fuzz.md`; pointers in all 28
remaining files; add the `dev:code-map-workspace` anchor (co-owner
`crates/rfs-build/src/lib.rs`); extend the manifest to all 72 entries and ADD
the completeness test (manifest == filesystem scan of `src/` + `crates/` +
`fuzz/fuzz_targets/`, excluding generated artifacts); run the full gate.

## Task deliverables

- `tests/dev_docs_test.rs` sync gate suite (5 check types, 8 tests at 1.1;
  completeness test added in 1.10) + pinned fixtures under
  `tests/fixtures/dev_docs/`.
- `tests/dev_docs_manifest.txt` with exactly the 72 hand-written `.rs` files.
- 32 skeleton docs under `docs/dev/` (18) + `docs/llm/` (14) with stable
  fragment IDs; Class B skeletons within their declared budgets.
- `//! Docs:` pointer lines in all 72 `.rs` files.

## Task exit criteria

- [ ] `cargo test --test dev_docs_test` green: main gate + positive control +
      all negative controls (8 tests at 1.1; the completeness test is added in
      1.10 and brings the count up).
- [ ] All 32 skeleton docs exist; every defined fragment is owned by ≥1 source
      file (no orphans); fragment IDs match the Context table (deviations
      recorded as task corrections).
- [ ] All 72 `.rs` files carry exactly one `//! Docs:` line naming only defined
      fragments; manifest == filesystem scan of the 72 files.
- [ ] Full verification green: `cargo build && cargo test && cargo clippy
      --all-targets && cargo fmt --check` (+ `--workspace`), docs gate
      (`docs_md_test`) - new docs enter its checked set automatically.

## Non-goals

- No doc CONTENT beyond skeletons (that is tasks 2-13); no changes to parser,
  validator, codegen, MC algorithms, CLI behavior, or generated output - the
  only source edits are `//!`/`///` doc comments.
- No item-level (`/// Docs:`) pointer gate-checking (convention only).
- No completeness test before 1.10 (the manifest is intentionally partial in
  between, so the repo stays green at every step).
