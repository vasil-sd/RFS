# Phase 8: Detailed developer documentation and code↔doc traceability

## Goal

Make the RFS source tree self-documenting in BOTH directions: every technical
statement in the documentation traces down to the source code that implements
it, and every source file/function carries a pointer to the documentation
fragment that describes it. The phase delivers two documentation classes -
(a) developer/engineer documentation (rich, diagram-heavy, rationale- and education-focused)
and (b) LLM-assistant documentation (compact, task-oriented, context-budgeted)
- plus a machine-checked sync gate so docs and code cannot drift apart as the
project grows.

## Scope

- New developer documentation set under `docs/dev/` (Class A): architecture and component diagrams, functionality hierarchy, file-level and function-level dependency graphs, design-decision rationale index, multi-level main-algorithm write-ups with external references (articles/books/publications), per-subsystem deep dives (parser, validator, IR + json_rw, codegen core + Rust/C backends, MC engine + ref_mc, monitor compiler, CLI commands, workspace crates).
- New LLM-assistant documentation set under `docs/llm/` (Class B): functionality→source map, change-impact tables (change type → affected files/tests/docs), compact per-module reference cards with stable fragment IDs and machine-checked line budgets.
- Bidirectional traceability: doc→code links (file + item anchors) in every technical statement; code→doc pointers (module/function doc comments referencing doc fragment IDs) in EVERY source file under `src/`, `crates/*/src/`, `fuzz/fuzz_targets/`.
- Diagrams (mermaid, machine-checked by the existing docs gate): component diagram, functionality hierarchy, file dependency graph, function-level dependency graphs for core modules, spec→IR→targets data-flow, MC engine flow, monitor compilation flow.
- Machine-checked sync gate: new test suite verifying link resolution (doc→code), source coverage (every source file has a doc pointer and appears in the code map), no orphaned doc fragments, Class B line budgets; part of plain `cargo test`.
- Both backends documented with equal depth (the backend parity principle applies to documentation coverage).
- Pointer updates: `START.md` (documentation map + task→read table), `KB.md`, `BUILD.md` (new suite row + expected counts), `docs/roadmap/INTRO.md` phase-state table; cross-links from `docs/user/README.md` to the developer set.
- Training-set update: `project_tooling` entries describing the new documentation layout and how an LLM session should use Class B docs; snippets + aggregate regeneration.

## Bidirectional traceability principle

Doc → code (downward trace):

- Every technical statement in Class A/B docs that describes implemented behavior, an algorithm, a data structure, or a CLI rule carries a link to the implementing source: file path + item anchor where stable (e.g. `src/mc_plan.rs::plan_liveness_obligations`).
- Anchors use stable identifiers (function/const names), NEVER line numbers - line numbers drift; the sync gate re-resolves them on every test run.

Code → doc (upward trace):

- Every source file starts with a module doc comment naming its documentation fragment(s) (stable fragment IDs like `dev:codegen-core`, `llm:mc-engine`).
- Key public functions/constants carry a one-line doc pointer to the specific fragment section that specifies their behavior.
- The sync gate fails if a source file lacks a doc pointer or points at a missing fragment.

Sync guarantee:

- A new test suite (`tests/dev_docs_test.rs`) runs in plain `cargo test` and checks BOTH directions: (1) every doc→code link resolves to an existing file + item; (2) every source file has ≥1 code→doc pointer and appears in the Class B functionality map; (3) no doc fragment is unreferenced by any source file (no orphans); (4) Class B per-file line budgets hold.
- After any source change, the gate reports exactly which doc fragments are stale or missing - this is the mechanism that keeps docs and code in sync without manual discipline: altering source code makes the corresponding documentation changes identifiable from the test output alone.

## Documentation classes

### Class A - developers/engineers (`docs/dev/`)

Audience: engineers driving project development. Rich, diagram-heavy,
rationale-focused and educational. After reading it a developer must be able to:

- understand the source layout and identify which files implement which functionality/component;
- follow any feature from spec syntax through IR to generated artifacts;
- understand WHY each major design decision was made (not just what), with links to the decision tables in `DESIGN.md` sections 19-25;
- learn the main algorithms behind RFS at several levels of detail - from an abstract high-level view down to optional views that include important implementation details (emphasized where they matter) - and follow links to related articles/books/publications for deeper study.

Required content:

- `ARCHITECTURE.md`: pipeline overview + component diagram (mermaid) + functionality hierarchy tree + relationship map between components;
- `CODE_MAP.md`: file-by-file map of `src/`, `crates/`, `fuzz/` - each file's responsibility, key items, doc links both ways;
- `DEPENDENCIES.md`: file-level dependency graph (mermaid) + function-level dependency/call graphs for the core modules (parser, validator, codegen core, mc_plan, monitor, ref_mc, backends rust*/c*);
- `DECISIONS.md`: rationale index - every decision ID (D1-D10, D-P3-1..6, D-P4-1..14, D-P5-1..14, D-P6-1..11, D-P7-1..13) with a 2-4 line "why" summary + link to the full table in `DESIGN.md` and to the implementing code;
- `ALGORITHMS.md`: main-algorithm write-ups (LL(1) parser construction, validation passes, IR normalization, codegen pipeline, MC engine - exact/liveness/SMT -, monitor compilation): each algorithm described at several levels of detail - L1 abstract high-level view, L2 key data structures and invariants, L3 optional implementation details with the important ones emphasized - every level linked to the implementing code;
- `REFERENCES.md`: consolidated index of external articles/books/publications (parsing theory, automata theory, SMT/SAT solving, model checking, compiler construction) with one-line annotations on what each covers and which RFS component uses it; algorithm sections link into it;
- per-subsystem deep dives (one file each): parser, validator, IR + json_rw, codegen core + plugin API, Rust backend, C backend, MC engine (exact/liveness/SMT) + ref_mc, monitor compiler, CLI commands, workspace crates (rfs-build/rfs-adapters/examples), toolchain-fuzz layer.

### Class B - LLM assistants (`docs/llm/`)

Audience: LLM assistants executing tasks in this repo. Compact, task-oriented,
context-budgeted. After reading the relevant fragment(s) an assistant must be
able to:

- identify the source parts related to a given functionality/component without exploring the tree;
- identify every place that needs changes/corrections for the task at hand (code + tests + docs);
- do so with minimal context usage - stable anchors, short fragments, machine-checked line budgets, no quality compromise.

Required content:

- `TASK_MAP.md`: functionality → files/functions table covering EVERY CLI command, subsystem, and test suite; the "read this fragment for this task" index that extends `START.md` section 6;
- `CHANGE_IMPACT.md`: change type → affected source files + tests + doc fragments (mirrors and extends the `BUILD.md` section 5 checklist at file granularity);
- per-module compact cards (one short file per core module): responsibility, key items with one-line roles, invariants that must not break, doc links; each card within its line budget.

Context-budget rules (machine-checked by the sync gate):

- every Class B file declares a line budget in its header; the gate fails on overflow;
- fragments are addressable by stable IDs (`llm:<module>`) so an assistant reads exactly one fragment instead of a whole file;
- no duplicated content between Class A and Class B - Class B links to Class A for rationale and algorithm detail, never restates it.

## Machine-checked sync gate

New suite `tests/dev_docs_test.rs` (part of plain `cargo test`, no new
dependencies beyond the existing mdr-stack):

1. doc→code: every link in `docs/dev/` + `docs/llm/` resolves to an existing file and (where given) an existing item name;
2. code→doc: every `.rs` file under `src/`, `crates/*/src/`, `fuzz/fuzz_targets/` carries a doc pointer naming ≥1 existing fragment ID;
3. coverage: every source file appears in `docs/llm/TASK_MAP.md`; no Class A/B fragment is unreferenced by any source file;
4. budgets: Class B line budgets hold;
5. negative controls: a broken link / missing pointer / budget overflow each fail the suite (pinned fixtures).

The existing `tests/docs_md_test.rs` gate already validates markdown quality +
mermaid rendering for every repo `.md` - the new files enter that checked set
automatically.

## Deliverables

- `docs/dev/README.md` (index), `docs/dev/ARCHITECTURE.md`, `docs/dev/CODE_MAP.md`, `docs/dev/DEPENDENCIES.md`, `docs/dev/DECISIONS.md`, `docs/dev/ALGORITHMS.md`, `docs/dev/REFERENCES.md` + per-subsystem deep-dive files under `docs/dev/`.
- `docs/llm/README.md` (index + context-budget rules), `docs/llm/TASK_MAP.md`, `docs/llm/CHANGE_IMPACT.md` + per-module compact cards under `docs/llm/`.
- Code→doc pointers in every source file under `src/`, `crates/*/src/`, `fuzz/fuzz_targets/` (module doc comments + key-item pointers).
- Mermaid diagrams: component diagram, functionality hierarchy, file dependency graph, function-level dependency graphs for core modules, spec→IR→targets data-flow, MC engine flow, monitor compilation flow.
- `tests/dev_docs_test.rs` sync gate suite (+ shared fixtures), wired into plain `cargo test`.
- Updated pointers: `START.md` (documentation map + task→read table row for the new sets), `KB.md` (doc map + section 8 roadmap row), `BUILD.md` (new suite row + expected counts), `docs/roadmap/INTRO.md` (phase-state table).
- Training-set update: `training/project_tooling/rfs_training.jsonl` entries on the new documentation layout + Class B usage; regenerated snippets + `all_rfs_training.jsonl`.

## Exit criteria

- Every technical statement in `docs/dev/` and `docs/llm/` that describes implemented behavior carries a doc→code link that resolves (file + item anchor); no untraced technical claims.
- Every source file under `src/`, `crates/*/src/`, `fuzz/fuzz_targets/` carries a code→doc pointer to an existing fragment and appears in `docs/llm/TASK_MAP.md`.
- Sync gate suite passes in plain `cargo test`: link resolution, source coverage, no orphaned fragments, Class B line budgets, negative controls.
- All required diagrams present as mermaid blocks and render through the docs gate (component diagram, functionality hierarchy, file dependency graph, function-level graphs for core modules, data-flow, MC engine flow, monitor compilation flow).
- Main algorithms (parser, validator, codegen, MC engine exact/liveness/SMT, monitor compilation) are documented in `docs/dev/ALGORITHMS.md` at several levels of detail (abstract view → key structures/invariants → optional implementation details with important ones emphasized), each level linked to the implementing code; external references are collected in `docs/dev/REFERENCES.md` and linked from the relevant algorithm sections.
- Both backends (Rust and C) are covered with equal depth in CODE_MAP / DEPENDENCIES / deep dives (backend parity principle applied to documentation).
- Class B docs satisfy the context-budget rules: stable fragment IDs, per-file budgets machine-checked, no content duplication with Class A.
- Full verification green: `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` (+ `--workspace`), `cargo test-generated-code`, docs gate (`docs_md_test`).
- Docs synchronized: `START.md`, `KB.md`, `BUILD.md`, `docs/roadmap/INTRO.md` updated; `TASKS.md` entry appended.

## Non-goals

- NO changes to parser, validator, codegen, or model-checking algorithms, CLI behavior, IR schema, or generated output (documentation-only phase - the only source edits are doc comments and the new test suite).
- No new RFS language features; no LSP.
- No automatic documentation generation from source code - docs are hand-written and kept in sync by the machine-checked gate (a generator is a post-v1 candidate if the gate proves insufficient).
- No visual/pixel rendering requirements for diagrams - mermaid text blocks validated by the existing docs gate only.
- No changes to the external user documentation set (`docs/user/`) beyond cross-links from its README - that set targets end users, not developers or assistants.
- No line-number-based anchors anywhere (they drift); stable item-name anchors only.
