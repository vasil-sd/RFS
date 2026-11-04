# Phase 10 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation immediately without re-deriving scope.

## Context

- Phase 10 is a DOCS-ONLY analysis phase. It produces two inventory documents
  (`SRC_STRUCTURE.md`, `TEST_STRUCTURE.md` at the repo root) that a later
  refactoring phase will consume to actually reorganize the source tree.
  Nothing under `src/`, `crates/`, `fuzz/`, or `tests/` is modified in this
  phase (exit criterion enforces this via `git diff --stat`).
- Scale measured at planning time (2026-11-02), re-verify with the same
  commands at each task start - if source changed, record a plan correction:
  - Non-test source (test-aware counting per plan correction 2a): ~954
    functions + ~215 types in `src/` (42 files, ~43.8k lines incl.
    `backends/`, `diagrams/`, `bin/`), plus ~45 functions + ~18 types across
    `crates/rfs-build/src`, `crates/rfs-adapters/src`,
    `crates/examples/*/src` + `build.rs` (the 5 `fuzz/fuzz_targets/*.rs`
    files - 4 fns / 1 type - belong to the TEST inventory per the boundary
    rule below). Total non-test: ~999 functions / ~233 types. Largest files:
    `ref_mc.rs` 4,971 lines / 100 non-test fns, `parser.rs` 2,091 / 76,
    `codegen_test.rs` 2,320 / 36, `specgen.rs` 2,094 / 53, `ir.rs` 1,432 / 63.
  - Test source: ~616 functions in `tests/*.rs` + `tests/common/*` (27 files,
    ~20.8k lines; largest `frontend_test.rs` 4,148 / 137 fns), ~255 inline
    `#[cfg(test)]` functions inside `src/` + crates files (test-aware count per
    plan correction 2a - the awk-era 311 included template-fn phantoms in
    rust_test.rs/test_cmd.rs/smt_emit.rs), 26 in `crates/*/tests/*`
    (rfs-build integration 6, rfs-adapters adapters 12 + nostd_minimal 8,
    14 types), and the 5 `fuzz/fuzz_targets/*.rs` harnesses (macro-based,
    0-3 fns each, 4 fns / 1 type total). Total test: ~901 functions.
  - Counting commands: functions
    `grep -cE '^[[:space:]]*(pub(\(crate\))? )?fn ' <file>`, types
    `grep -cE '^[[:space:]]*(pub )?(struct|enum|trait|type) ' <file>`;
    inline test fns via REAL-attribute + brace-depth detection over
    `#[cfg(test)]` regions (see task 29/30) - WARNING: doc comments and string
    templates that MENTION `#[cfg(test)]` (rust_test.rs, test_cmd.rs,
    smt_emit.rs) create phantoms for naive line-based counters.
- Purpose of the dependency edges (user clarification at plan review,
  2026-11-02): the inventory tables must enable fine-grained impact analysis
  of future source changes and precise scoping of refactoring/editing tasks.
  Consequences baked into every task file: (a) the All types table's
  identifier column is named `Type` (not `Function`) in BOTH documents; (b)
  Depends is recorded in BOTH directions across BOTH tables - function rows
  reference called functions AND used/constructed types, type rows reference
  contained types AND implementation functions (trait impls/methods).
- Boundary decision (binding): SRC_STRUCTURE.md covers `src/**`,
  `crates/*/src/**`, example-crate `src/` + `build.rs`. TEST_STRUCTURE.md
  covers `tests/**` (incl. `tests/common/` and the three
  `tests/fixtures/dev_docs/*.rs` negative-control fixtures, which are
  enumerated but flagged as fixtures), inline `#[cfg(test)]` modules in
  `src/`, `crates/*/tests/**`, and `fuzz/fuzz_targets/**`. The generated
  artifacts under `crates/examples/fuzz_rust/fuzz/target/` are build output -
  NEVER enumerated.
- Planning decision: the user's steps p1 (functions) and p2 (types) are
  executed TOGETHER per file-cluster chunk (one read pass per file fills both
  tables). Rationale: dependencies and descriptions are derived from the same
  body reading; separate passes would double the source reads for no benefit.
  If the user objects at plan review, split each chunk task into a function
  half and a type half (plan correction).
- Documented-in column: every one of the 71 manifest `.rs` files carries
  exactly one `//! Docs:` line naming its `dev:`/`llm:` fragments (enforced by
  dev_docs_test) - copy those fragment names verbatim. Test files have none
  except `tests/dev_docs_test.rs`; for suites, note the matching
  `docs/llm/TASK_MAP.md` entry in the row if one exists, else leave blank.
- Both new .md files are parsed by `tests/docs_md_test.rs` (comrak GFM +
  mermaid-rs-renderer full pipeline + relative-link resolution) from the moment
  they exist - every task that adds content must keep that gate green. The
  mermaid layout diagrams (tasks 18, 33) must use syntax mermaid-rs-renderer
  accepts (verify by running the gate, not by assumption).
- Wide markdown tables: the inventory tables have long cells; keep each row on
  ONE physical line (comrak parses either way, but one-line rows stay
  diff-friendly and grep-able - a future session will grep these tables).
- Commit discipline: one commit per completed task (Phase 9 user rule),
  message starting with `Phase 10 task M (complete): ...`.
- Source freeze assumption: because the phase is docs-only, the inventory
  cannot drift mid-phase unless someone else edits source; each enumeration
  task re-runs its chunk's counting command first and records a plan
  correction if counts moved.
- Seed proposal for task 14 (refine after seeing all rows): Components
  (structural) - parser, IR model, JSON I/O, printer, validator, monitor
  compiler, codegen core (registry/options), shared test planning, shared MC
  concepts, MC plan, SMT plan+emitter, Rust backend (fsm/test/mc), C backend
  (fsm/test/mc), CLI dispatch, frontend CLI logic, external-tool resolution,
  rfs-test command, info command, info-format registry, EBNF grammar model,
  JSON-schema emitter, diagram core + plugins, reference MC, spec generation,
  spec hashing, training pipeline, rfs-build crate, rfs-adapters crate,
  example crates, toolchain fuzz targets. Functionality (cross-cutting) -
  parsing, semantic validation/typing, IR serialization, codegen-production,
  codegen-tests, model-checking, property monitoring, PSL translation,
  diagnostics, CLI surface, self-describing formats/documentation, provenance
  (version stamp + spec hash), fuzzing, training data, build integration,
  protocol adapters/integration.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | SRC_STRUCTURE.md skeleton + conventions | `SRC_STRUCTURE.md` (new) | Create the file at repo root with all six sections as empty tables carrying the exact column headers from PHASE_10.md, plus a short "Conventions" preamble pinning the row format, both identifier formats (`<file>::<function>`, `<file>::<Type>`), the two table headers (`Function` / `Type` identifier columns), bidirectional Depends semantics, multi-value comma separation, and the Documented-in source. Rationale: conventions must be frozen before any chunk task runs so all 12 enumeration tasks fill rows identically. |
| 2. | Enumerate chunk A - parser core | `SRC_STRUCTURE.md` (change) | Fill All functions + All types rows for `src/parser.rs`, `src/error.rs`, `src/lib.rs` (~86 fns / ~4 types). Method per PHASE_10.md conventions: grep signatures, read each body for callees + one-line description, copy fragments from the `//! Docs:` line. Rationale: the single largest file (parser) gets its own chunk with the two small foundational files. |
| 3. | Enumerate chunk B - IR model + serialization | `SRC_STRUCTURE.md` (change) | Rows for `src/ir.rs`, `src/printer.rs`, `src/json_rw.rs` (~115 fns / ~27 types). Rationale: the IR data model, its RFS printer, and the hand-rolled JSON reader-writer form one serialization cluster; ir.rs's 63 fns are mostly per-type to_json/from_json pairs that share description patterns. |
| 4. | Enumerate chunk C - validation + monitor compilation | `SRC_STRUCTURE.md` (change) | Rows for `src/validator.rs`, `src/codegen_fsm.rs`, `src/monitor.rs` (~47 fns / ~13 types). Rationale: semantic validation, the legacy production-FSM helpers, and the shared MonitorPlan compiler are the pre-codegen analysis cluster. |
| 5. | Enumerate chunk D - codegen core + shared test planning | `SRC_STRUCTURE.md` (change) | Rows for `src/codegen.rs`, `src/codegen_mc.rs`, `src/codegen_test.rs` (~66 fns / ~40 types). Rationale: the Backend plugin registry, exact-mode MC concepts/wire format, and the language-agnostic TestPlan are the codegen-core cluster; codegen_test.rs is the largest planner (67 fns). |
| 6. | Enumerate chunk E - MC plan + SMT | `SRC_STRUCTURE.md` (change) | Rows for `src/mc_plan.rs`, `src/smt_plan.rs`, `src/smt_emit.rs` (~61 fns / ~29 types; smt_emit carries string-template C fns - read-through is authoritative). Rationale: abstraction/encoding/traversal planning plus the feature-gated SMT spec+emitter form the model-checking-planning cluster. |
| 7. | Enumerate chunk F - Rust backend | `SRC_STRUCTURE.md` (change) | Rows for `src/backends/rust.rs`, `rust_fsm.rs`, `rust_test.rs`, `rust_mc.rs` (~131 fns / ~26 types; rust_test EMITS test code as string templates - read-through is authoritative). Rationale: all four Rust-backend files in one chunk keeps the production/test/mc rendering trio comparable in a single pass. |
| 8. | Enumerate chunk G - C backend | `SRC_STRUCTURE.md` (change) | Rows for `src/backends/c.rs`, `c_fsm.rs`, `c_test.rs`, `c_mc.rs` (~73 fns / ~5 types). Rationale: mirror of chunk F; the C MC mirrors the Rust exact engine, so reading both chunks back-to-back makes cross-backend dependency notes consistent. |
| 9. | Enumerate chunk H - CLI entry + tooling commands | `SRC_STRUCTURE.md` (change) | Rows for `src/main.rs`, `src/frontend.rs`, `src/ext_tool.rs`, `src/cli_log.rs`, `src/test_cmd.rs`, `src/gen_spec_cmd.rs`, `src/mc_ref_cmd.rs` (~89 fns / ~20 types - the two command files were missing from every chunk and are added here per plan correction 2a). Rationale: subcommand dispatch, parse/validate CLI logic, the external-tool resolver, log-level handling, the generate+compile+run test command, and the mc-ref/gen-spec command modules are the CLI-execution cluster. |
| 10. | Enumerate chunk I - introspection + self-describing formats | `SRC_STRUCTURE.md` (change) | Rows for `src/info.rs`, `src/info_format.rs`, `src/ebnf_grammar.rs`, `src/json_schema.rs` (~75 fns / ~7 types). Rationale: the `rfs info` command, the 10-format registry with embedded models, the 61-rule EBNF model, and the draft-07 schema emitter are the Phase 5/7 introspection cluster. |
| 11. | Enumerate chunk J - diagram command + plugins | `SRC_STRUCTURE.md` (change) | Rows for `src/diagram.rs` + `src/diagrams/{mod,dot,plantuml,mermaid,tables}.rs` (~41 fns / ~12 types). Rationale: the plugin registry and all five v1 plugins (incl. Phase 7 in-process SVG hooks) form one small cluster. |
| 12. | Enumerate chunk K - independent reference infrastructure | `SRC_STRUCTURE.md` (change) | Rows for `src/ref_mc.rs`, `src/specgen.rs`, `src/spec_hash.rs` (~156 fns / ~31 types). Rationale: the brute-force reference MC, the seeded spec generator/mutator, and the hand-rolled SHA-256 are the Phase 6 soundness infrastructure; ref_mc.rs alone (124 fns) justifies a dedicated chunk. |
| 13. | Enumerate chunk L - training pipeline + workspace crates | `SRC_STRUCTURE.md` (change) | Rows for `src/training.rs`, `crates/rfs-build/src/lib.rs`, `crates/rfs-adapters/src/*.rs` (5 files), `crates/examples/{sync_handler,async_handler,fuzz_rust}/src/*.rs` + their `build.rs`, plus the four thin `src/bin/*.rs` wrappers (~59 fns / ~19 types). Rationale: everything outside the earlier clusters in one small chunk; the bins belong to the training/test-generation tooling. The fuzz targets are NOT here - per the boundary rule (user decision at plan review) they belong to the test inventory and are enumerated in task 30. |
| 14. | Propose Components + Functionality categories | `SRC_STRUCTURE.md` (change) | Fill `## Components` and `## Functionality` tables (name / depends on / description + rationale) by reviewing all enumerated rows; start from the seed proposal in Context, rename/merge/split as the actual dependency picture demands. Rationale: categories must be derived from the finished inventory, not imposed before it exists. |
| 15. | Fill classification - All functions table | `SRC_STRUCTURE.md` (change) | Set the Component + Functionality cells of every ~999 function rows (multi-valued allowed), working chunk-by-chunk in task 2-13 order. Rationale: separate from task 14 so the category set is frozen first; a dedicated pass keeps the classification decisions reviewable row by row. |
| 16. | Fill classification - All types table | `SRC_STRUCTURE.md` (change) | Same for the ~233 type rows. Rationale: types cluster differently from functions (e.g. wire-format structs vs plan models) and get their own pass. |
| 17. | Propose new naming scheme + target tree | `SRC_STRUCTURE.md` (change) | Write a `## Proposed naming scheme` section: naming conventions for files/modules (snake_case mirroring module paths, suffix rules for command files, backend file split) and the target directory tree in text form, each choice justified against the task-14 categories. Rationale: the scheme must be argued from the categories before any file is mapped onto it. |
| 18. | File mapping table + proposed layout mermaid | `SRC_STRUCTURE.md` (change) | Fill the per-file mapping table (one row per non-test source file: components, functionalities, description + rationale - including where each file lands under the proposed scheme) and draw the proposed tree as a mermaid diagram; run docs_md_test to prove it renders. Rationale: the mapping table is the bridge document the refactoring phase executes against. |
| 19. | TEST_STRUCTURE.md skeleton + conventions | `TEST_STRUCTURE.md` (new) | Same six-section skeleton as task 1, adapted: note that Documented-in is blank except `tests/dev_docs_test.rs`, fixtures are flagged, and inline `#[cfg(test)]` rows carry their host file in the identifier (`src/parser.rs::tests::<fn>`). Rationale: test inventory has different documentation provenance and one extra row class (inline tests). |
| 20. | Enumerate test chunk T0 - shared helpers | `TEST_STRUCTURE.md` (change) | Rows for `tests/common/{mod,prod_driver,fail_variants,mc_json}.rs` (~27 fns / ~8 types). Rationale: every suite depends on these helpers; enumerating them first makes later chunks' Depends columns resolvable. |
| 21. | Enumerate test chunk T1 - frontend CLI suite | `TEST_STRUCTURE.md` (change) | Rows for `tests/frontend_test.rs` + `tests/library_comparison.rs` (~146 fns / ~1 type). Rationale: the largest single suite (4,148 lines, 137 fns) gets its own chunk. |
| 22. | Enumerate test chunk T2 - doc/format gates | `TEST_STRUCTURE.md` (change) | Rows for `tests/info_format_test.rs` + `tests/docs_md_test.rs` (~72 fns). Rationale: the two documentation-quality suites form one cluster. |
| 23. | Enumerate test chunk T3 - codegen + examples suites | `TEST_STRUCTURE.md` (change) | Rows for `tests/codegen_test.rs` + `tests/examples_test.rs` (~67 fns / ~5 types). Rationale: registry/per-command generation tests plus the file-driven example suite. |
| 24. | Enumerate test chunk T4 - cross-backend parity suites | `TEST_STRUCTURE.md` (change) | Rows for `tests/backend_parity_test.rs` + `tests/wide_ops_parity_test.rs` + `tests/prod_parity_test.rs` (~52 fns / ~18 types). Rationale: the three parity-pinning suites share drivers and comparison idioms. |
| 25. | Enumerate test chunk T5 - MC benchmark/liveness/SMT suites | `TEST_STRUCTURE.md` (change) | Rows for `tests/benchmark_mc_test.rs` + `tests/liveness_mc_test.rs` + `tests/smt_bounded_test.rs` (~49 fns / ~23 types). Rationale: the model-checking test cluster. |
| 26. | Enumerate test chunk T6 - reference-MC soundness suites | `TEST_STRUCTURE.md` (change) | Rows for `tests/ref_mc_sweep_test.rs` + `ref_mc_diag_test.rs` + `ref_mc_diff_test.rs` + `toolchain_fuzz_test.rs` (~54 fns / ~10 types). Rationale: the Phase 6 differential-soundness cluster. |
| 27. | Enumerate test chunk T7 - monitor/PSL suites | `TEST_STRUCTURE.md` (change) | Rows for `tests/monitor_ref_test.rs` + `monitor_integration_test.rs` + `psl_translation_test.rs` (~73 fns / ~28 types). Rationale: hand-written monitor references, injected-bug integration, and the independent PSL evaluator. |
| 28. | Enumerate test chunk T8 - remaining suites | `TEST_STRUCTURE.md` (change) | Rows for `tests/dev_docs_test.rs` + `diagram_render_parity_test.rs` + `example_projects_test.rs` + `integration_example_test.rs` + `training_test.rs` (~69 fns / ~14 types), incl. the three `tests/fixtures/dev_docs/*.rs` files flagged as fixtures. Rationale: the leftover suites are all small; one chunk finishes the `tests/` tree. |
| 29. | Enumerate inline tests - frontend/IR cluster | `TEST_STRUCTURE.md` (change) | Rows for the `#[cfg(test)]` functions in `src/{parser,validator,json_rw,printer,monitor,codegen_test,ext_tool,cli_log,lib,frontend,spec_hash,codegen_fsm}.rs` (~180 fns), identifiers `src/<file>.rs::tests::<fn>`. Rationale: half of the 311 inline test functions; split by cluster so each chunk stays session-sized. |
| 30. | Enumerate inline tests - backends/MC/CLI cluster + crate tests + fuzz targets | `TEST_STRUCTURE.md` (change) | Rows for the remaining inline `#[cfg(test)]` functions (`src/{backends/*,specgen,codegen_mc,json_schema,test_cmd,info_format,smt_emit,ref_mc,mc_plan,mc_ref_cmd,gen_spec_cmd}.rs`, ~99 fns - rust_test.rs and test_cmd.rs have NO real test module per correction 2a) plus `crates/rfs-build/tests/integration.rs`, `crates/rfs-adapters/tests/{adapters,nostd_minimal}.rs` (26 fns / 14 types together), and ALL 5 `fuzz/fuzz_targets/*.rs` files - their SOLE home, moved out of task 13 per the user's plan-review decision. Rationale: completes the test inventory; fuzz targets are macro-thin (0-3 fns each) but must appear for completeness. |
| 31. | Propose test Components + Functionality categories | `TEST_STRUCTURE.md` (change) | Fill its `## Components` / `## Functionality` tables: reuse SRC_STRUCTURE categories where a suite tests that component, add test-specific ones (e.g. parity-pinning, doc-quality gates, soundness/differential, file-driven corpus). Rationale: test classification is derived the same way as source classification but needs its own cross-cutting axes. |
| 32. | Fill test classification columns | `TEST_STRUCTURE.md` (change) | Set Component + Functionality cells on every test function/type row, chunk-by-chunk in task 20-30 order. Rationale: mirror of tasks 15/16 for the test tables. |
| 33. | Test file mapping table + proposed test-tree mermaid | `TEST_STRUCTURE.md` (change) | Fill the per-test-file mapping table and draw the proposed test-tree layout as a mermaid diagram; run docs_md_test. Rationale: completes p7 - the refactoring phase gets one executable bridge document per source set. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 2a | Tasks 2.-13. chunk counts; tasks 29./30. inline counts | change | Discovered at task 2's count re-verification: the planned chunk counts came from raw line-based greps that (i) INCLUDED inline `#[cfg(test)]` functions which per the boundary rule belong to tasks 29-30, and (ii) MISSED `src/gen_spec_cmd.rs` + `src/mc_ref_cmd.rs` entirely (no chunk assigned them). Additionally, naive counters miscount emitter files: `rust_test.rs` and `test_cmd.rs` have NO real test module (their awk-era counts were template fns after doc-comment mentions of `#[cfg(test)]`), and `smt_emit.rs`'s real test module holds 6 fns, not 10. Corrected (test-aware counting): chunk totals T2 86/4, T3 115/27, T4 47/13, T5 66/40, T6 61/29, T7 131/26, T8 73/5, T9 89/20 (gen_spec_cmd + mc_ref_cmd added), T10 75/7, T11 41/12, T12 156/31, T13 59/19 - non-test total ~999 fns / ~233 types (was ~1,250/~237); inline test total ~255 (was 311); test inventory total ~901 (was ~957). `backends/mod.rs` has 0 fns/0 types - file-mapping-only (task 18). Emitter files (`rust_test.rs`, `smt_emit.rs`) carry template fn definitions inside string literals: the read-through during each task establishes the authoritative per-file count. PHASE_10.md's exit-criteria counts (~1,250/~237) are understood as superseded by this correction - the close-out re-verification uses test-aware counting. |
| 6a | Task 6 chunk count (smt_emit.rs) | change | Discovered at task 6's count re-verification: correction 2a's "T6 61/29" assumed smt_emit.rs holds 32 non-test fns, derived from its claim that "smt_emit.rs's real test module holds 6 fns, not 10" (4 template phantoms). The read-through shows the opposite: ALL 10 test-module fns are real (test_spec, bin_ops, cmp_widths_full_fits_and_widens, shift_amt_width_rules, normalize_rust, normalize_c, fn_body, rust_and_c_render_identical_op_sequences, oversized_literal_comparison_widens, shift_rendering_paths) and there are NO template phantoms at all - the generated `fn` text lives inside format!/push_str string literals and never starts a line, so line-based counters see only real definitions. Authoritative smt_emit.rs count: 28 non-test fns / 10 test fns / 2 types (RustRender, CRender). Chunk E total is therefore 57 fns / 29 types (mc_plan.rs 18/16 - its type grep's 2 extra matches are template enums inside the test region, smt_plan.rs 11/11, smt_emit.rs 28/2), not 61/29. Correction 2a's "T6 61/29" figure and task 6's Planned-tasks row (~61 fns / ~29 types) are understood as superseded by this correction; the non-test src/ total becomes ~995 fns (was ~999). The type total (~233) is unchanged. |
| 7a | Task 7 chunk count (Rust backend template phantoms) | change | Discovered at task 7's count re-verification: correction 2a's "T7 131/26" counted TEMPLATE PHANTOMS - fn and type definitions inside the raw-string code templates these emitters generate. Read-through (authoritative per correction 2a's own rule) found: rust_fsm.rs 41 non-test fns / 2 types (Gen, ExprCtx; 5 phantom fns + 13 phantom types in the ProtocolFsm/trace/Fsm raw strings), rust_test.rs 16 non-test fns / 1 type (GeneratedCrate; 9 phantom test-fn templates in format! strings, NO real test module - its awk-era "25 test fns" started counting at a DOC COMMENT mentioning #[cfg(test)]), rust_mc.rs 7 non-test fns / 0 types (46 phantom fns + 10 phantom types across the EXACT_ENGINE/LIV_ENGINE/JSON_WRITER_RUST/EXACT_RUN/MAIN_DISPATCH/FUZZ_RUNTIME/SMT_ENGINE_RUST raw strings - LevelOutcome included). Authoritative chunk F total: 71 fns / 3 types (was ~131/~26); rust.rs unchanged at 7/0. The non-test src/ total becomes ~935 fns (was ~995 per correction 6a); the type total drops from ~233 to ~210 (chunk F contributed 26 phantom-inflated types, actually 3). |
| 8a | Task 8 chunk count (C backend type phantoms) | change | Discovered at task 8's count re-verification: correction 2a's "T8 73/5" overcounted TYPES. Read-through found c_mc.rs holds NO Rust types - its three `struct` grep hits (`struct rusage ru;`, two `struct timespec`) are C declarations inside the generated-code raw strings, not toolchain definitions. Authoritative per-file counts: c.rs 6 non-test fns / 0 types (its 4 inline tests belong to task 30), c_fsm.rs 37 fns / 2 types (Gen, Ctx - including the two truly-nested fns push/walk, which get rows under the new `<file>::<parent_fn>::<child_fn>` identifier convention recorded in the SRC_STRUCTURE.md preamble), c_test.rs 15/0, c_mc.rs 15/0. Authoritative chunk G total: 73 fns / 2 types (was ~73/~5). The non-test src/ fn total (~935 per correction 7a) is unchanged; the type total drops from ~210 to ~207. |
| 9a | Task 9 chunk count (frontend.rs associated-type phantom) | change | Discovered at task 9's count re-verification: correction 2a's "T9 89/20" overcounted TYPES by one. frontend.rs's sixth type grep hit is `type Err = String;` - an associated-type binding inside the FromStr impl, not a standalone type (the same phantom class task 5's codegen.rs correction established for the two `type Err = String` bindings there). Authoritative per-file counts: main.rs 11/0, frontend.rs 21/5, ext_tool.rs 16/4, cli_log.rs 8/2, test_cmd.rs 19/5, gen_spec_cmd.rs 7/1, mc_ref_cmd.rs 7/2. Authoritative chunk H total: 89 fns / 19 types (was ~89/~20). The non-test src/ fn total becomes ~1,024 (was ~935 per correction 7a); the type total becomes ~225. |
| 13a | Task 13 chunk count (missed demo bins + associated-type phantoms) | change | Discovered at task 13's count re-verification: the cluster notes' "~59 fns / ~19 types incl. bins" was wrong in both directions. FUNCTIONS undercounted by 3: the two example-crate demo binaries (crates/examples/{sync_handler,async_handler}/src/main.rs, 1 fn each) were never mentioned in the notes though the Goal's `src/*.rs` covers them, and async_handler/src/lib.rs holds 4 non-test fns, not 3 (the async `step` helper was missed). TYPES overcounted by 7: the raw grep counted ASSOCIATED-TYPE bindings as types - the same phantom class correction 9a established for frontend.rs and task 5's read-through established for codegen.rs. The phantoms: lib.rs's ProtocolFsm trait declares `type Input;` / `type Output;` / `type Error;` (3), async_adapter.rs's Stream impl binds `type Item = AsyncOutcome<F>;` (1), minimal.rs's ProtocolFsm impl binds `type Input/Output/Error = F::...;` (3). Authoritative per-file counts: training.rs 10/1, the four src/bin wrappers 4/0, rfs-build 13/2, rfs-adapters lib 2/1 + channel 5/2 + async_adapter 6/2 + callback 3/1 + minimal 5/1 (= 21/7), sync_handler lib 5/1 + main 1/0 + build 1/0, async_handler lib 4/1 + main 1/0 + build 1/0, fuzz_rust lib 0/0 + build 1/0. Authoritative chunk L total: 62 fns / 12 types (was ~59/~19). The completeness sweep's authoritative final inventory total is therefore 938 fns / 197 types (chunks A-K 876/185 + chunk L 62/12) - note correction 2a's "T5 66/40" was itself phantom-inflated by the two codegen.rs `type Err = String` bindings; task 5's status recorded the authoritative 38 but the corrections table row was never updated, so A-K's true type total is 185, not the ~187 a naive sum of the corrections table would give. |
| 20a | Task 20 chunk count (test-helper type phantoms) | change | Discovered at task 20's count re-verification: the cluster notes' "~8 types" overcounted by 6. The read-through found only 2 real Rust types, both in prod_driver.rs (FlatVariant, ProductionArtifacts). The phantom "types": fail_variants.rs's two `enum State` grep hits are RFS spec declarations INSIDE the pinned-FAIL-variant raw strings (`r#"..."#`), not toolchain definitions - the same template-phantom class corrections 7a/8a established for the emitter files; mc_json.rs holds NO types (its only type-ish grep hit is a local `const MARKER: &str`, neither a fn nor a type). Authoritative per-file counts: mod.rs 0/0, prod_driver.rs 21 fns / 2 types, fail_variants.rs 5 fns / 0 types, mc_json.rs 1 fn / 0 types. Authoritative chunk total: 27 fns / 2 types (was ~27/~8). The fn count (~27) was already correct; only the type estimate was wrong. |
## Other information

- Open question for plan review: are p1/p2 merged-per-chunk (the planning
  decision in Context) acceptable, or does the user want separate function and
  type tasks per chunk?
- Resolved at plan review (2026-11-02, user decision): `fuzz/fuzz_targets/**`
  has a single home - TEST_STRUCTURE.md, enumerated in task 30 (removed from
  task 13; chunk counts re-derived: non-test total ~1,250 fns / ~237 types,
  test total ~957 fns). Rationale given by the user: they are fuzz harnesses,
  i.e. test infrastructure.
- Risk: the inventory tables are large (~1,500 rows total across both files);
  keep rows one-per-line and never reflow them - future grep-based tooling
  (and the refactoring phase) depends on stable row identity.
- Commands to run at every task: the chunk's counting greps (Context), then
  `cargo test --test docs_md_test` after any .md change; full
  `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`
  only at close-out (no source changes are possible mid-phase).
