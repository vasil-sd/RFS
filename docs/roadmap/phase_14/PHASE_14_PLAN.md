# Phase 14 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

**Goal 1 - where `cargo run --` actually lives (verified by grep, 2026-11-13).**
In source, ONLY the global `usage()` in `src/cli/args.rs` carries the pattern -
four example lines at the end of the help text. Every per-command `usage()`
(parse/validate/info/test/diagram/mc-ref/gen-spec) and the codegen help
(`common_help`, `backend_help`, all six `BackendCommandInfo` usage/options
strings) already reference only `rfs`. Living docs with the pattern: `KB.md`
section 4.5 (~20 lines), `BUILD.md` command tables + quick-start (~14 lines),
`README.md` (mermaid node labels + text), `START.md` section 7 (one sanity line),
and `docs/user/{README,INTEGRATION,LANGUAGE,MODEL_CHECKING,PROPERTIES,EXAMPLES}.md`
(~32 lines). Training data: 14 entries teach the pattern -
`training/frontend_cli/rfs_training.jsonl` (10), `training/codegen_rust/`
rfs_training.jsonl (3), `training/project_tooling/rfs_training.jsonl` (1).
Legitimate cargo usages that must NOT change: workspace build/test commands,
`cargo run -p sync_handler` / `-p async_handler` (example crates), the
cargo-fuzz pipeline (`cargo fuzz build/run`, `RFS_CARGO_FUZZ`). Historical
records (`docs/roadmap/phase_0..13/*`, `TASKS.md` history, `commands.md`) are
frozen - never rewritten.

**Goal 2 - current artifact structure (verified by reading the generators).**
- C `production` (`src/codegen/backends/c/fsm.rs`): ONE self-contained .c -
  `emit_header` (line-1 artifact header, std includes, `-Wunused-function`
  pragma, `RFS_SPEC_VERSION` / `RFS_PROPERTY_MONITORS` defines) + `emit_types`
  (enums, input/output unions, `step_result_t`, `fsm_t`) + monitors +
  `emit_fsm_impl`, all into a single `code` string.
- C `mc` (`c/mc.rs`): inlines the WHOLE production FSM by calling
  `c::fsm::generate_core(...)` (line 66) plus `render_mc_step_all_c` /
  `render_liv_sample_evaluator_c` from c/fsm.rs, then its own engine + main -
  one self-contained executable .c.
- C `test` (`c/test.rs`): self-contained test .c (FSM inlined + tests + main)
  + a separate `fuzz_target.c` skeleton that does
  `#include "rfs_generated_fsm.c"` (line 627) - source-level inclusion of the
  whole implementation, the pattern this phase eliminates.
- Rust `production` (`rust/fsm.rs`): one self-contained .rs.
- Rust `test` (`rust/test.rs`): `generate_core(module, opts, with_tests=true,
  Always)` fuses FSM + tests into ONE string; the separate `fuzz_target.rs`
  skeleton is wired by the user into their own cargo-fuzz project via
  `use crate::{...}` (lib root = the production source).
- Rust `mc` (`rust/mc.rs`): self-contained executable .rs with the FSM inlined.
- `CodegenOutput { code, warnings, extra_files }` is the shared output model;
  the CLI write loop in `src/cli/commands/codegen.rs` writes `code` to `--out`
  (or stdout) and `extra_files` next to it, refusing stdout when extras exist.
- `rfs test` (`src/cli/commands/test.rs`) consumes the LIBRARY API directly
  (`CodegenCommandKind::run`), reads `extra_files.first()` as the fuzz skeleton
  (line 338) and calls `Production` for the C fuzz pipeline (line 341); the C
  unit pipeline compiles the single self-contained test .c with `gcc -Wall`,
  the C fuzz pipeline writes `rfs_generated_fsm.c` next to the skeleton
  (hardcoded name, lines 375-384, 849, 864) and clang-compiles ONLY the
  skeleton; the Rust unit pipeline runs `rustc --edition 2021 --test` on the
  fused artifact. The `--print-script` heredocs pin these names
  (`tests/utils_test.rs` lines ~3490-3518).
- The 1683-test generated-code suite (`src/bin/generate_code_tests.rs`) uses
  `rust::fsm::generate_core(..., with_tests=true, ...)` DIRECTLY and wraps each
  module as `pub mod mNNN_... { <fused code> }` in an aggregate crate - it
  never touches the CLI or the artifact-file layout. As long as the fused
  `generate_core` contract is preserved, that suite is UNTOUCHED by this phase.
- Test legs that compile generated C MC artifacts: `tests/backend_parity_test.rs`
  (gcc -Wall, lines ~146-163), `tests/benchmark_mc_test.rs` (~180-197),
  `tests/c_compiler_agnostic_test.rs` (gcc AND clang legs at -O2, real `-c`
  compiles), plus the liveness/monitor suites as applicable.
- Examples: `crates/examples/c_sync_handler/src/handler.c` does
  `#include "arq_fsm.c"` (whole source); its Makefile generates
  `build/arq_fsm.c` via `rfs codegen c production $(SPEC) --out $@` and compiles
  only handler.c with `-I$(BUILD_DIR)`. `crates/examples/fuzz_c/Makefile`
  generates `build/rfs_generated_fsm.c` + the skeleton and clang-builds only the
  skeleton. `tests/example_projects_test.rs` pins both (line 182 asserts the
  skeleton's `#include "rfs_generated_fsm.c"`; line 184 reads the generated .c;
  `has_gcc_diagnostic` greps for `handler.c:` / `arq_fsm.c:`).

**Confirmed design decisions (user-reviewed 2026-11-13, recorded in
`PHASE_14.md` "Proposed CLI" incl. the 23-row CLI examples table):**
- D14-1: each command accepts only its own output-flag subset; the flag family
  is `--out-fsm-code/--out-fsm-header` (production), `--out-mc-code/
  --out-mc-header` (mc), `--out-test-code/--out-fuzz-target-code` (test);
  header flags exist for the C backend only.
- D14-2: fsm-reference flags (`mc`/`test` only) - Rust `--fsm-file NAME[.rs]`
  (extension appended when omitted); C form 1 `--fsm-file STEM` -> the pair
  `STEM.h` + `STEM.c` (NO `-fsm` suffix); C form 2 `--fsm-file-header FILE.h
  --fsm-file-source FILE.c`. Semantics: TAKE the artifact if present on disk,
  otherwise EMIT it new - applies whether the fsm name comes from a flag or is
  derived from BASE. The command's own artifacts are always written fresh
  (overwritten). A partial C pair (only one of .h/.c) is a usage error - no
  silent overwrite.
- D14-3: naming resolution - BASE := `--out` value when given, otherwise the
  SPEC FILE STEM (file name without extension, no directory component);
  artifacts land in the CWD (`--out DIR/BASE` places the set under DIR/).
  Explicit per-artifact flags override the corresponding BASE-derived name.
  Stdout: `--out -` prints the single artifact (Rust `production` only;
  multi-artifact commands refuse it). Behaviour change: no-output-flag
  invocations now write spec-stem-named files instead of printing to stdout
  (both backends' production today print to stdout).
- D14-4: an existing fsm artifact whose line-1 header spec hash (D-P6-3) differs
  from the current spec's hash produces a stderr WARNING (stale reuse) but is
  still referenced.
- D14-5: Rust reference mechanism - `include!("BASE-fsm.rs")` at the top of the
  mc and test artifacts (zero body changes; works with single-file rustc
  compilation in temp dirs; both files land in the same directory). The Rust
  fuzz target KEEPS its crate-wiring model (lib root = production source,
  `use crate::{...}`) - only its wiring comments are updated.
- D14-6: C mc artifact split - the header declares the MC engine's public API
  (run entry point + report types, the structs already used for the text/JSON
  reports) and carries the line-1 artifact header; the source implements the
  engine + main() and includes BOTH its own header and the fsm header. The
  inlined `generate_core` call is removed from the executable.

**Risks / things to re-check during implementation:**
- The `c_compiler_agnostic_test.rs` gcc/clang -O2 parity legs must keep passing
  under object+link compilation (the `-Wunused-function` pragma moves into the
  fsm .c; verify clang stays quiet on both files).
- `include!` of the Rust production artifact: confirm the artifact contains no
  crate-root-level items that would collide when inlined at the mc/test crate
  root (it is flat items today - re-verify against a property-bearing spec).
- The C fuzz pipeline's `ASAN_OPTIONS=detect_leaks=0` merge and the "compile
  ONLY the skeleton" gotcha (TOOLCHAIN.md 7.4) change shape: clang now compiles
  fsm.c + fuzz_target.c together - re-verify no multiple-definition errors
  (there are none expected: the skeleton no longer includes the .c).
- `docs_md_test.rs` renders every mermaid block in `docs/user/*` - the task-7
  edits must keep all diagrams syntactically valid.
- Pre-existing staleness OUT of scope: `DESIGN.md` section 14's codegen examples
  (`rfs codegen production ...`) predate the backend registry (Phase 2) and are
  not re-synced by this phase.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Codegen output model + CLI flag family + naming resolution | `src/codegen/mod.rs` (change), `src/cli/commands/codegen.rs` (change), `src/codegen/backends/c/{fsm,test,mc}.rs` (change - adapt to the new output struct only), `src/codegen/backends/rust/{fsm,test,mc}.rs` (change - same), `tests/codegen_test.rs` (change - CLI parsing tests + new flag-family coverage), `tests/utils_test.rs` (change - CLI-parsing legs) | The atomic CLI change: restructure `CodegenOutput` from (`code` + unnamed `extra_files`) to named artifact slots (fsm-header, fsm-code, mc-header, mc-code, test-code, fuzz-target; each an optional content string); extend `CodegenRequest::Generate` + `parse_codegen_args` with the full flag family, BASE resolution (`--out` \| spec stem), per-artifact overrides, fsm-reference flags (D14-2 forms) and the `--out -` marker; rewrite the codegen.rs write loop (path resolution vs CWD / `--out DIR`, dir creation, take-or-emit presence check for the fsm slots incl. partial-pair error + D14-4 hash-mismatch warning, fresh-write for own artifacts, stdout mode). Generators adapt minimally: current single-file content goes into the primary slot (C production -> fsm-code only, no header yet; mc/test self-contained as today), so ALL existing behaviour is preserved except file NAMING - and every call site + test using the old `--out FILE` semantics migrates to the new naming in this same task. Rationale: the parsing layer is shared by both backends, so it lands atomically; keeping generators unchanged here keeps the tree green between tasks. | 
| 1.1 | Output struct + request fields + parser | `src/codegen/mod.rs` (change) | New `CodegenOutput` slot model; new `CodegenRequest::Generate` fields (per-artifact names, fsm-reference forms, stdout marker); `parse_codegen_args` implements D14-1..D14-3 incl. all error conventions (unknown option / unexpected argument / missing value via the existing helpers in `src/cli/args.rs`). |
| 1.2 | Write loop + take-or-emit + stdout | `src/cli/commands/codegen.rs` (change) | Resolve each requested artifact's final path; presence check for fsm slots (C form 1 requires BOTH files - partial pair is a usage error); parse the line-1 header of a taken fsm file for the D14-4 spec-hash comparison (warn on mismatch); `create_dir_all` for parent dirs; `--out -` prints the single artifact and is refused for multi-artifact commands. |
| 1.3 | Generator adaptation (content unchanged) | `src/codegen/backends/c/{fsm,test,mc}.rs`, `src/codegen/backends/rust/{fsm,test,mc}.rs` (change) | Each entry point returns its current content in the primary slot(s); C production's single .c goes into the fsm-code slot; the rust test command's fuzz skeleton moves from `extra_files` into the fuzz-target slot. No emitted bytes change in this task. |
| 1.4 | Call-site + test migration to the new naming | `tests/codegen_test.rs`, `tests/utils_test.rs` (change) | Migrate every `--out FILE` usage to `--out BASE` or explicit flags; extend the CLI-parsing tests with the flag family, spec-stem default, per-artifact override, partial-pair error and `--out -` refusal cases (pinning the 23-row contract table's parse-level rows). |
| 2. | C production header/source split | `src/codegen/backends/c/fsm.rs` (change), `tests/codegen_test.rs` (change) | Split the single-string emission into a header part (include guard, line-1 artifact header, `RFS_SPEC_VERSION` / `RFS_PROPERTY_MONITORS` defines, ALL type definitions, public API declarations - `fsm_new`, `fsm_step`, policy/monitor surface) and a source part (`#include` of its own header, the `-Wunused-function` pragma, static helpers, all function bodies); `generate_production` fills both slots. Tests pin the pair, compile EACH file with `gcc -Wall -c` (header must be self-contained), and keep the existing content assertions re-pointed at the right slot. Rationale: idiomatic C separation is the point of the phase for the C backend; doing it before the mc/test tasks means those artifacts can reference a real header immediately. |
| 3. | C mc artifact pair + fsm reference | `src/codegen/backends/c/mc.rs` (change), `tests/backend_parity_test.rs` (change), `tests/benchmark_mc_test.rs` (change), `tests/c_compiler_agnostic_test.rs` (change), liveness/monitor test suites as applicable (change) | Remove the inlined `generate_core` call from the executable: mc.c includes the fsm header (name resolved by the CLI layer and passed to the generator); split the MC engine into header (public run entry + report types per D14-6) and source (engine + main, including its own header). The mc command co-emits/takes the fsm pair via take-or-emit. Migrate every MC-compiling test leg to compile fsm.c + mc.c to objects and link, keeping the `gcc -Wall` zero-warning bar and the gcc/clang -O2 parity legs of c_compiler_agnostic_test. Rationale: strict separation - no FSM implementation inside the mc artifact; object+link is the idiomatic C build shape the examples will mirror. |
| 4. | C test + fuzz-target reference the fsm header; `rfs test` C pipelines | `src/codegen/backends/c/test.rs` (change), `src/cli/commands/test.rs` (change), `tests/utils_test.rs` (change) | The test artifact includes the fsm header instead of inlining the FSM; the `fuzz_target.c` skeleton replaces `#include "rfs_generated_fsm.c"` with the fsm-header include and its wiring comments are updated to the new generation commands. The `rfs test` C unit pipeline writes the fsm pair + test.c into the temp dir, compiles each with `gcc -Wall -c` and links; the C fuzz pipeline clang-compiles fsm.c + fuzz_target.c together (the "compile ONLY the skeleton" gotcha no longer applies - re-verify no multiple definitions); the `--print-script` heredocs are regenerated accordingly and their pins in utils_test updated. Rationale: kills the whole-source inclusion pattern end-to-end, including the CI entry point. |
| 5. | Rust mc/test/fuzz-target reference the fsm file; `rfs test` Rust pipeline | `src/codegen/backends/rust/{fsm,mc,test}.rs` (change), `src/cli/commands/test.rs` (change) | Refactor `rust/fsm.rs`'s Gen to expose the fsm-core and test-section renderings separately so the CLI test artifact can be `include!("BASE-fsm.rs")` + test section ONLY - while `generate_core(with_tests=true)` keeps returning the fused self-contained string for the aggregate-crate path (the 1683-test suite stays untouched). rust/mc.rs: mc artifact = include! + engine, no inlined FSM; co-emits/takes the fsm .rs. The fuzz_target.rs skeleton keeps its crate-wiring model; only its comments are updated to the new naming. The `rfs test` Rust unit pipeline writes fsm.rs + test.rs into the temp dir and runs `rustc --edition 2021 --test` on the test file (include! resolves relative to it). Rationale: D14-5 - zero body changes, single-file compilation preserved, no FSM duplication across files. |
| 6. | Help-text fix (goal 1, source side) | `src/cli/args.rs` (change), per-command `usage()` sites in `src/cli/commands/{parse,validate,info,test,diagram/mod,mc_ref,gen_spec}.rs` (change only if the audit finds cargo/rustc assumptions or stale `--out` semantics), `src/codegen/mod.rs` (change - `common_help` / `backend_help` / the six `BackendCommandInfo` usage+options strings rewritten to the new flag family + default policy), `tests/codegen_test.rs` (change - new regression test) | Replace the four `cargo run -- ...` example lines in the global `usage()` with `rfs ...`; update the codegen synopsis line(s) to the new flag family, spec-stem default and `--out -`; audit EVERY help surface so only the `rfs` binary is referenced (no cargo/rustc invocation examples anywhere); add a regression test asserting no help output (global + all eight commands + codegen --list/--help/<backend> --help) contains `cargo run`. Rationale: goal 1's source half - the rfs binary must be usable and self-describing on machines without any Rust toolchain. |
| 7. | Living-documentation sync (goal 1, docs side) | `KB.md` (change - section 4.5 ~20 lines + codegen rows for the new flags), `BUILD.md` (change - ~14 command-table lines + quick-start + codegen rows), `README.md` (change - mermaid node labels + text), `START.md` (change - section 7 sanity line), `docs/user/README.md` (change), `docs/user/INTEGRATION.md` (change), `docs/user/LANGUAGE.md` (change), `docs/user/MODEL_CHECKING.md` (change), `docs/user/PROPERTIES.md` (change), `docs/user/EXAMPLES.md` (change) | Rewrite every `cargo run -- <rfs-subcommand>` example to `rfs <subcommand>` (legitimate cargo usages stay: workspace builds, `cargo run -p <example>`, cargo-fuzz); update the codegen walkthroughs to the new CLI - the two-minute tour, integration examples, and the MC workflow showing the fsm/mc file pair + object linking. Keep every mermaid block valid (docs_md_test renders them all). Rationale: goal 1's docs half - one rule across all living documentation. |
| 8. | Codegen architecture docs + dev/llm re-sync | `src/codegen/CODEGEN.md` (change - the artifact model, flag family, naming resolution, take-or-emit, stdout mode), `src/codegen/backends/C.md` (change - header/source split details), `src/codegen/backends/RUST.md` (change - the include! mechanism), `docs/dev/*` (change via `rfs-dev sync-fix` - cli_commands, codegen-core, backend-c, backend-rust and any file whose references go stale), `docs/llm/TASK_MAP.md` / `CHANGE_IMPACT.md` / `SRC_STRUCTURE.md` / `TEST_STRUCTURE.md` (change via `rfs-dev inventory` + sync-fix), `tests/dev_docs_manifest.txt` (change only if the source file set changes) | Document the new artifact model where codegen is documented; re-sync every doc<->source pointer and inventory row touched by tasks 1-6 with the dev_tools (sync-check/sync-fix/inventory audit per SKILL.md). Rationale: the docs<->source correspondence rule (PROTOCOL.md section 5) - inventories describe the tree as it is, in the same commit. |
| 9. | Example fixes | `crates/examples/c_sync_handler/Makefile` (change), `crates/examples/c_sync_handler/src/handler.c` (change), `crates/examples/c_sync_handler/README.md` (change), `crates/examples/fuzz_c/Makefile` (change), `crates/examples/fuzz_c/README.md` (change), `tests/example_projects_test.rs` (change) | c_sync_handler: handler.c includes ONLY the generated header; the Makefile generates the fsm pair under new flags, builds EXPLICIT object files (arq_fsm.o from arq_fsm.c, handler.o from handler.c) and links them into the final executable; scenario output must stay byte-identical. fuzz_c: generation step uses the new flags; clang builds the fuzz target together with the separately generated fsm source (or objects). Migrate example_projects_test's pins (the skeleton-include assertion, the generated-file reads, `has_gcc_diagnostic`). Rationale: the user's concrete complaint - handler.c including the whole arq_fsm.c - fixed end-to-end with explicit object files. |
| 10. | Training-set update | `training/frontend_cli/rfs_training.jsonl` (change - 10 entries), `training/codegen_rust/rfs_training.jsonl` (change - 3 entries), `training/project_tooling/rfs_training.jsonl` (change - 1 entry), `training/*/snippets/` + `all_rfs_training.jsonl` (regenerated) | Rewrite the 14 entries teaching `cargo run -- <command>` to `rfs <command>`; audit the codegen entries for old single-file `--out` semantics and update any that teach generation to the new flag family; regenerate snippets + the aggregate set per the training workflow (`cargo extract-training-snippets && cargo build-training && cargo test-training`). Rationale: the training set must not teach the CLI pattern this phase removes. |
| 11. | Close-out | `docs/roadmap/CURRENT_STATE.md` (change - phase 14 row -> COMPLETE), `TASKS.md` (change - append entry per its first-paragraph template) | Run the full verification (`cargo test`, `cargo test-generated-code`, `cargo fmt --check`, clippy at the pre-phase baseline, the example-project suite), flip the CURRENT_STATE row, write the TASKS.md entry and commit per FORMATTING.md section 7. Rationale: PROTOCOL.md section 3 close-out - the phase exits only with docs synchronized and all tests green. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 2a | 2 - C production header/source split | change | The split makes the generated source non-self-contained (it `#include`s its own header), so the two C example Makefiles (`c_sync_handler`, `fuzz_c`) - which `#include` the whole `.c` - broke until they also generate the header pair. A MINIMAL fix landed in task 2 (generate `--out-fsm-header`+`--out-fsm-code`, add `-I.`; fuzz_c's `test` invocation now references the explicit pair). This pulls forward a small piece of task 9's scope; task 9 RETAINS the deeper restructure (handler.c includes only the header, explicit `.o` generation, link `arq_fsm.o` with `handler.o`). Rationale: task 2's exit criterion requires `cargo test` (root) green, and the examples are part of that suite - they cannot stay red until task 9. |
| 4a | 4 - C test + fuzz-target reference; `rfs test` C pipelines | change | The `--print-script` shape change (a THIRD heredoc for the production header + renumbered build/run/clean steps) landed in task 2 via the fuzz-pipeline groundwork (`FuzzContext` already carried the header), not in task 4 as originally assumed. The `tests/utils_test.rs` pins were regenerated against the new three-heredoc script in task 2. Task 4's remaining scope is the test/fuzz-target REFERENCE of the split fsm (they still emit self-contained content in tasks 2-3). |
| 4b | 4 - C test + fuzz-target reference; `rfs test` C pipelines | change | Task 4's change to the fuzz skeleton (header-reference instead of `#include "...fsm.c"`) breaks the `fuzz_c` example's build rule, which compiled ONLY the skeleton (relying on the source-include). A MINIMAL fix landed in task 4: the clang line now compiles `$(GEN_FSM_C)` + the skeleton together, and `example_projects_test.rs`'s skeleton-include assertion was re-pointed from `rfs_generated_fsm.c` to `build/rfs_generated_fsm.h`. This pulls forward a small piece of task 9's scope (the fuzz_c both-source build); task 9 RETAINS the deeper example restructure. Rationale: task 4's exit criterion requires `cargo test` (root) green, and the fuzz_c example is part of that suite - it cannot stay red until task 9. |
| 3a | 3 - C mc artifact pair + fsm reference | change | The plan scoped task 3 to `c/mc.rs` only, on the assumption (see "Other information") that the MC engine's report types were already header-shareable. In reality the MC engine is coupled to FSM internals that were source-local statics (`rfs_commit`, `trace_vars`, `assume_monitors_step`, `prop_monitors_step`, `ASSUME_LABELS`/`N_ASSUME_LABELS`) - visible only because the FSM core used to be inlined into the same translation unit. A true object+link split therefore required exposing them as EXTERNAL symbols declared in the fsm header, which expanded task 3's footprint to `c/fsm.rs` (de-staticise + emit prototypes/externs; `prop_monitors_step` gated by `prop_emit`), `mod.rs` (`BackendOptions.mc_header_name`; `primary_code()` re-prioritised mc > test > fsm because C mc now fills both the referenced fsm slot and its own), and `cli/commands/codegen.rs` (name-stamping extended from C production to C mc, sharing the write loop's `resolve_fsm_reference`). Rationale: cross-translation-unit visibility is a hard requirement of the split, not an optional refactor - the MC cannot link against the fsm object unless the symbols it calls are exported. |

## Other information

- Verification commands per task: after tasks 1-5 `cargo test` (root) must stay green at every step - the plan is ordered so each task lands behaviour-complete for what it touches; task 6 adds the help regression test; task 9 runs the example-project suite (`cargo test --test example_projects_test`); task 10 runs the training trio; task 11 runs everything incl. `cargo test-generated-code` (the 1683-test suite must be UNCHANGED - it consumes `generate_core` directly).
- Assumption to re-check at task 5: the Rust production artifact is flat crate-root items (no `mod` declarations, no inner `include!`) so `include!` at the mc/test crate root cannot collide - verify against a property-bearing spec (e.g. examples/valid/16_next_property.rfs) before relying on it.
- Assumption to re-check at task 3: the MC engine's report structs are already shared types usable in a header without pulling implementation code - if any struct is currently defined inside a function body or behind statics, hoist it into the header as part of the split.
- The `--print-script` output of `rfs test` changes shape (task 4) - its pins live in `tests/utils_test.rs` (~lines 3490-3518) and must be regenerated from the NEW script, never hand-edited to match.
- clippy baseline: the pre-phase count is 46 warning lines / 35 unique (dead-code in shared test modules + pre-existing patterns in `src/codegen/backends/c/fsm.rs` etc.); the header/source split may MOVE existing warnings between files but must not add new ones.
