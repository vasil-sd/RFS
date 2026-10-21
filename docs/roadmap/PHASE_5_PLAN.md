# Phase 5 - Implementation plan

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

Review of `PHASE_5.md` (frozen phase description) against the shipped state
after Phase 4 close-out (2026-10-20). Findings that shape this plan:

- **Current CLI**: exactly three commands exist (`parse`, `validate`,
  `codegen`). `rfs test`, `rfs diagram`, `rfs info` are NEW and need concrete
  semantics defined here (see tasks 2, 6, 7). There is no shared diagnostics/
  logging helper yet - task 2 introduces one (`--log-level`, structured
  `--json` plumbing reused by later tasks).
- **Flag-name mismatch (correction)**: `PHASE_5.md` lists `--state-limit` /
  `--time-limit`, but Phase 4 SHIPPED `--max-states` / `--timeout-sec` (pinned
  in tests, docs, training data, and the generated binaries' wire format).
  Decision: keep the shipped names; no aliases (two names for one flag would
  break the parity-pinned surface). Recorded as plan correction C1.
- **`--features` option dropped (correction)**: its intent (select codegen
  features) is covered by the explicit `--no-std` / `--with-property-monitors`
  flags; rfs-build exposes cargo feature passthrough separately. Correction C2.
- **`--counterexample-format json|text`**: the text report format is
  PARITY-PINNED (byte-identical across backends). JSON must be an ADDITIONAL
  mode with `text` as default, so no pinned output changes; parity extends to
  byte-identical JSON. (Task 9.)
- **no_std (D9)**: explicitly deferred to Phase 5 in DESIGN.md section 20.
  Generated Rust currently uses std heavily (Vec/String/Box/format!). Scope
  decision: `--no-std` applies to the PRODUCTION command only (test/mc stay
  std - they need allocation-heavy test/MC machinery); generated code becomes
  `#![no_std]` + `alloc` with an IDENTICAL public API shape. The C backend is
  already dependency-free plain C: the flag is accepted for CLI symmetry and
  documented as a no-op there. (Task 9.)
- **Repo layout**: currently a single cargo package (`rfs`). `rfs-build`
  requires converting to a WORKSPACE (members: `rfs`, `rfs-build`, plus later
  adapter/example crates). Task 1 does this first; all other tasks depend on
  nothing from it except the crate-plumbing ones.
- **Self-contained generated code is a design invariant** (RUST.md section 2):
  every artifact compiles with no dependencies. Therefore the `ProtocolFsm`
  trait + adapters CANNOT be linked into generated code; instead codegen
  optionally emits an `impl ProtocolFsm for Fsm` block, and a separate adapter
  crate provides GENERIC wrappers over the trait. (Task 10.)
- **C parity bar**: every generated C artifact must stay `gcc -Wall` clean;
  cross-backend test coverage must be identical (same cases, same expected
  outcomes). libFuzzer targets need CLANG to build - the skeleton is generated
  for both backends but building/running the C fuzz target is documented as a
  clang requirement (Rust side uses cargo-fuzz/libfuzzer_sys). (Tasks 5, 11.)
- **Diagram formats as plugins (user requirement, 2026-10-21)**: `rfs diagram`
  is built on a plugin registry mirroring the codegen backend pattern
  (`src/diagrams/` + `DiagramPlugin` in `src/diagram.rs`) so new formats are
  added as one module + one registry line. v1 plugins: `dot` (Graphviz),
  `puml` (PlantUML state diagrams - edge label `event \n guard`, emitted data
  via `note on link`), `mmd` (Mermaid `stateDiagram-v2` - all info in
  multi-line edge labels, since mermaid has no notes-on-links), plus `table`/
  `md` transition tables + variable/event summaries. PRIMARY behaviour is pure
  text generation with NO tool calls; `--render svg` invokes the format's tool
  (`dot`/`plantuml`/`mmdc`) with STRICT error semantics: missing/failing render
  tool -> exit 1 + diagnostic, NO graceful degradation (user decision 2026-10-21:
  RFS runs in CI/CD pipelines - a command either does what was requested or
  throws an error; users compose their own fallbacks, e.g. via `--print-script`
  or try-render-then-generate wrappers). All three tools are optional system
  dependencies: graphviz/plantuml via apt, mermaid-cli via npm. (Task 8.)
- **Test generation today**: the shared planner (`codegen_test.rs`) produces
  module-level tests only (positive drive, negative unexpected-input probe,
  deadlock-style). Phase 5 extends it with per-transition unit tests,
  invariant tests, property-monitor debug tests, seeded random-trace tests, and
  fuzz-target skeleton planning - language-agnostic, rendered by BOTH backends
  with identical coverage. (Tasks 4-7.)
- **Baseline to build on** (Phase 4 close-out): 230/230 main tests (232 with
  `--features smt`), 537/537 generated-code tests (179 modules), 45 valid /
  12 invalid examples, training aggregate 231 lines. The extended test
  generation will grow the aggregate crate - counts are re-pinned in task 15.
- **External-tool parameterization (user requirement, 2026-10-20)**: every CLI
  command that invokes external tools/compilers/build systems must be
  parameterizable by tool PATHS and additional tool PARAMETERS. Design:
  environment variables are the PREFERRED mechanism - `RFS_RUSTC` / `RFS_GCC`
  / `RFS_DOT` / `RFS_PLANTUML` / `RFS_MERMAID` (paths) and
  `RFS_RUSTC_ARGS` / `RFS_GCC_ARGS` / `RFS_DOT_ARGS` / `RFS_PLANTUML_ARGS` /
  `RFS_MERMAID_ARGS` (space-separated extra parameters); rfs-build uses
  `RFS_BIN` to locate the rfs executable. Per-command CLI flags (`--rustc
  PATH`, `--gcc PATH`, `--dot PATH`, `--plantuml PATH`, `--mermaid PATH`,
  `--*-args "..."`, etc.) override the env vars; without either, the bare tool
  name is looked up on PATH. A shared resolver + invocation helper (task 3)
  implements the precedence and renders/executes command lines identically for
  all commands.
- **Script-generation mode (user requirement, 2026-10-20)**: commands that
  invoke external tools accept `--print-script`: instead of executing, they
  print a deterministic POSIX sh script to stdout (shebang, `set -e`, header
  comment with rfs version + spec + resolved tool paths/args, one commented
  step per invocation) and exit 0. Applies to `rfs test` (compile + run steps)
  and `rfs diagram --render svg` (self-contained: heredoc writing the generated
  source, then the dot/plantuml/mmdc render invocation; for mermaid an extra
  step writes the temp puppeteer config; without `--render`, a no-external-tool
  script). Output is unit-test-pinned.
- **Hand-rolled JSON reader-writer, ALL sources (user requirement, 2026-10-20)**: a
  full hand-rolled JSON READER+WRITER (`src/json_rw.rs`) replaces serde/serde_json
  EVERYWHERE in the toolchain - IR serialization/round-trip, all `--json` CLI
  outputs (info/validate/MC counterexample), and the generated MC JSON via an
  embedded copy of the same module. Rationale (must be recorded with full detail
  in DESIGN.md section 23, KB.md, and the module's source comments): guarantee
  KEY ORDERING (declaration order, pinned) and ALWAYS byte-identical text output
  across platforms, Rust versions, and dependency updates - external crates give
  no such contract; it also keeps the toolchain dependency-light and consistent
  with the generated-code constraint. New task 2 implements it BEFORE any
  JSON-emitting command exists.
- **Protocol precedent**: phases end with a training-data task, a docs-sync
  task (DESIGN gets a new "Phase N decisions" section), and a close-out task
  (full verification + PHASE_N_STATUS.md per template 2.3). Tasks 13-15 below.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | Workspace conversion + `rfs-build` crate | `Cargo.toml` (change - workspace root), `crates/rfs-build/Cargo.toml` + `crates/rfs-build/src/lib.rs` (new), `crates/rfs-build/tests/` (new), `BUILD.md` (change - workspace build notes) | Convert the single package to a cargo workspace (`rfs` + `rfs-build`). `rfs-build` is the build.rs helper: a small Rust API that locates/invokes the `rfs` toolchain (path overridable via the `RFS_BIN` environment variable, default: PATH lookup), runs `codegen <backend> production`, writes artifacts to `OUT_DIR`, and emits correct `cargo:rerun-if-changed` directives so specs rebuild on change. Rationale: exit criterion "a team can add RFS to an existing Rust project with build.rs". Tests: unit tests for the API (incl. RFS_BIN override) + an integration test crate that builds a real spec through build.rs. |
| 2. | Hand-rolled JSON reader-writer + migration off serde | `src/json_rw.rs` (new), `src/ir.rs` (change - hand-written to_json/from_json for all IR types, serde derives removed), `src/frontend.rs` (change - .json paths via json_rw), `Cargo.toml` (change - serde/serde_json removed), `tests/frontend_test.rs` + lib unit tests (change) | Full JSON value model with ORDERED objects + writer (compact & pretty layouts, pinned escaping/number formatting) + reader (parse + typed extraction). ALL sources switch to it: IR round-trip, parse/validate/codegen .json input-output paths, test assertions; generated MC JSON (task 10) embeds a copy of the writer. serde/serde_json removed from dependencies. The module doc carries the detailed rationale (why hand-rolled instead of external crates: key-ordering guarantee + always byte-identical text output). Golden round-trip tests pinned; existing IR JSON text may change key order - affected tests re-pinned. |
| 3. | `rfs info` command + shared CLI infrastructure (diagnostics, external-tool resolution, script emission) | `src/main.rs` (change), `src/info.rs` (new), `src/ext_tool.rs` (new), `tests/frontend_test.rs` (change) | New stable command: static spec introspection - per module: states, events/outputs with variants+fields, variables with types, transition count, properties/assumptions with monitor-subset status (in-subset / skipped + reason). Options: `--json` (machine-readable document for tooling/IDE reuse) and `--log-level off\|error\|warn\|info\|debug`. ALSO introduces the shared CLI infrastructure reused by tasks 7/8/10/13: (a) stderr logging helper, (b) JSON plumbing, (c) external-tool resolver + invocation helper (`src/ext_tool.rs`): precedence CLI flag > env var (`RFS_RUSTC`/`RFS_GCC`/`RFS_DOT` + `RFS_*_ARGS`) > PATH default; renders command lines for `--print-script` mode and executes them otherwise; (d) `--print-script` emission (deterministic POSIX sh: shebang, `set -e`, header comment with rfs version/spec/resolved tools, commented steps). Unit-test the resolver precedence and pin the script output. |
| 4. | Extended shared test planner | `src/codegen_test.rs` (change), lib unit tests (change) | Extend `TestPlan`/`build_test_plan` with language-agnostic plans for: per-transition unit tests (drive each transition from a reachable state with representative inputs; assert next-state + emitted outputs), invariant tests (safety property atoms re-checked at every driven step), property-monitor debug tests (monitor must fire on an injected violating trace, mirroring Phase 3's monitor integration approach), seeded random-trace tests (deterministic LCG, fixed seed constant, K steps, assert no panic + report coverage counts), and fuzz-target skeleton planning (input decode strategy placeholder per event variant). Unit-test the planner exhaustively. Rationale: one plan rendered by both backends guarantees identical coverage (parity principle). |
| 5. | Rust test renderer + libfuzzer_sys skeleton | `src/backends/rust_test.rs` (change), generated aggregate crate (regenerated) | Render the extended plan into the `test` command output: per-transition tests, invariant checks, monitor debug tests, random-trace test, plus a fuzz skeleton file using `libfuzzer_sys` (as in PHASE_5.md's example). Regenerate the aggregate crate; all generated-code tests green; re-pin counts. |
| 6. | C test renderer + libFuzzer skeleton | `src/backends/c_test.rs` (change), `tests/backend_parity_test.rs` (change) | Mirror of task 4 in C: SAME cases, SAME expected outcomes (identical coverage per PHASE_5.md); plain-assert harness extended accordingly; libFuzzer C skeleton generated with a documented clang build recipe. `gcc -Wall` stays clean on every generated artifact. Extend the parity suite: both backends' generated test executables must agree on every example, including the new test classes. |
| 7. | `rfs test` command | `src/main.rs` (change), `src/test_cmd.rs` (new), `tests/frontend_test.rs` (change) | New stable command: generate + compile + run the extended test suite for a spec in one invocation. Options: `--backend rust\|c` (default rust), `--steps N`, `--log-level`; external-tool parameterization via task 3's resolver - `--rustc PATH` / `--gcc PATH` + `--rustc-args "..."` / `--gcc-args "..."` overriding `RFS_RUSTC`/`RFS_GCC` (+ `*_ARGS`); and `--print-script` to emit the compile+run steps as a shell script instead of executing. Exit 0 on all-pass, 1 on failure/compile error, with per-test results on stdout and diagnostics on stderr. Rationale: "CI can run model checking and generated tests" exit criterion - CI calls `rfs test` (or its generated script) instead of wiring codegen+compiler by hand. |
| 8. | `rfs diagram` command (EXTENDED 2026-10-21: plugin architecture + dot/plantuml/mermaid) | `src/main.rs` (change), `src/diagram.rs` (new), `src/diagrams/` (new - mod.rs, dot.rs, plantuml.rs, mermaid.rs, tables.rs), `src/ext_tool.rs` (change - RFS_PLANTUML/RFS_MERMAID presets), `tests/frontend_test.rs` (change), `BUILD.md` + `TOOLCHAIN.md` (change) | New stable command on a plugin registry mirroring the codegen backend pattern: every format is a `DiagramPlugin` (name, ext, pure-Rust `generate` fn, optional svg tool preset + `render_svg` fn). v1 formats: `dot` (Graphviz, pure Rust), `puml` (PlantUML state diagram - edge label `event \n guard`, emitted data via `note on link`, `-[dashed]->` on_error), `mmd` (Mermaid stateDiagram-v2 - all info in multi-line edge labels; `[on_error]` prefix; no notes-on-links/dotted edges exist), `table`/`md` (state x event matrix + variable/event summaries). PRIMARY behaviour = text generation with NO tool calls. `--render svg` (requires `--out FILE.svg`; source written next to it) invokes the resolved `dot -Tsvg` / `plantuml -tsvg` / `mmdc` (+auto temp puppeteer JSON with --no-sandbox for root); STRICT: missing/failing render tool -> exit 1 + diagnostic incl. captured tool stderr, no graceful degradation (CI/CD strictness, user decision 2026-10-21). Options: `--format dot\|puml\|mmd\|table\|md` (default dot), `--out FILE`, `--list`, `--render svg`, `--log-level`; parameterization via task 3's resolver - `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID` + `*_ARGS` env vars, overriding `--dot/--plantuml/--mermaid PATH` + `--*-args "..."` flags; `--print-script` (self-contained render script incl. heredoc + puppeteer step for mmd; no-tool script otherwise). Tests: pinned output for all 5 formats, 45-example sweep, real-tool acceptance tests (plantuml exit 0 / negative control exit 200; mmdc exit 0 / negative control exit 1 `Parse error`), stub-tool argv proofs, strict render-error tests (missing tool + failing renderer, all formats incl. DOT). BUILD.md/TOOLCHAIN.md: install/run instructions for dot/plantuml/mermaid. Full details: PHASE_5_TASK_8.md (extended). |
| 9. | no_std production codegen (`--no-std`) | `src/codegen.rs` (change - flag), `src/backends/rust_fsm.rs` (change), `src/backends/c_fsm.rs` (change - no-op note), `tests/codegen_test.rs` (change) | Per Context: PRODUCTION command only; generated Rust becomes `#![no_std]` + `extern crate alloc` with identical public API shape (Vec/Box via alloc; no std-only items). test/mc stay std. C accepts the flag as a documented no-op (plain C already dependency-free). Verification: every valid example's production output compiles under `#![no_std]` (compile check in the test suite); behavior parity unchanged. |
| 10. | MC counterexample JSON format (`--counterexample-format json\|text`) | `src/codegen.rs` (change - flag), `src/codegen_mc.rs` (change - JSON doc types), `src/backends/rust_mc.rs` + `src/backends/c_mc.rs` (change - renderers), `tests/backend_parity_test.rs` (change) | Codegen-time + runtime-overridable flag, default `text` (the pinned format is UNCHANGED). `json` emits a machine-readable document mirroring the text report: status, metrics (states/transitions/max_depth), reason/label/step, statistics block, and the counterexample (trace or lasso) as structured data. Parity extension: byte-identical JSON across backends for the same spec/mode. Rationale: CI + IDE integration need machine-readable MC results (exit criteria). |
| 11. | `ProtocolFsm` trait + adapter crate + Rust example crates | `crates/rfs-adapters/Cargo.toml` + `src/` (new), `src/backends/rust_fsm.rs` (change - optional impl emission), `src/codegen.rs` (change - `--with-protocol-fsm` flag), `crates/examples/sync_handler/`, `crates/examples/async_handler/` (new) | Per Context: codegen optionally emits `impl ProtocolFsm for Fsm` (flag `--with-protocol-fsm`, off by default to keep the plain surface); `rfs-adapters` provides the trait + GENERIC wrappers: sync channel-based adapter, futures-based async adapter, callback-based adapter - all parameterized over `F: ProtocolFsm`, no dependency on generated code. Example crates demonstrate a real protocol (reuse example 43 sliding-window ARQ): sync handler and async (futures) handler compiled against generated code + adapters. Rust-ecosystem-only by design (documented per parity principle). |
| 12. | C example projects + fuzzing harness examples | `crates/examples/c_sync_handler/` (new), `crates/examples/fuzz_rust/`, `crates/examples/fuzz_c/` (new), README in each crate (new) | Language-neutral mirrors of the Rust examples per the parity principle: a plain-C sync-style handler project built from generated C code (gcc -Wall clean, documented build). Fuzzing harness examples for BOTH ecosystems wiring the generated fuzz skeletons (task 4/5): Rust via cargo-fuzz/libfuzzer_sys, C via clang libFuzzer with the documented recipe. Each example: README with build/run instructions + expected first-run behavior. |
| 13. | IDE support (syntax highlighting + diagnostics integration) | `ide/textmate/rfs.json` + `ide/README.md` (+ snippets) (new), `src/frontend.rs` (change - `validate --json`) | Initial IDE support per PHASE_5.md: TextMate grammar for `.rfs` (keywords, temporal operators incl. Phase 4's eventually/until, comments, strings, numbers) + snippet set, shipped in-repo under `ide/` with install notes for VS Code/Sublime. CLI diagnostics integration: `rfs validate --json` emits structured parse/semantic errors (reusing task 2's JSON plumbing) so editors can ingest them directly. Full LSP stays a non-goal. |
| 14. | Training data update | `training/frontend_cli/rfs_training.jsonl`, `training/project_tooling/rfs_training.jsonl`, `training/codegen_rust/rfs_training.jsonl` (change), snippets + `all_rfs_training.jsonl` (regenerated) | New user-facing surface needs training coverage (Phase 4 task 12 precedent): new CLI commands (`info`/`test`/`diagram` with options and exit codes), external-tool parameterization (`RFS_RUSTC`/`RFS_GCC`/`RFS_DOT` + `*_ARGS`, `RFS_BIN`, the overriding flags) and `--print-script` mode, rfs-build build.rs usage, ProtocolFsm/adapters, no_std flag, counterexample JSON, diagram output. Follow the established format (think block + verbatim tool-output quotes); regenerate snippets + aggregate; training tests green. |
| 15. | Docs sync to final Phase 5 state | `DESIGN.md` (change - sections 5/8/9/10 updates + NEW section 23 "Phase 5 toolchain decisions" D-P5-1..N), `KB.md`, `BUILD.md`, `CODEGEN.md`, `src/backends/RUST.md`, `src/backends/C.md`, `START.md` (change) | Sync all design-relevant docs to the shipped Phase 5 state: new CLI surface + option table, workspace layout, external-tool parameterization + `--print-script` semantics (env vars, flags, precedence), rfs-build/adapters/no_std/diagram/JSON decisions as D-P5-x, updated source map, final test counts and baseline. Stale sweep per the established checklist. |
| 16. | Close-out: full verification + status file | `docs/roadmap/PHASE_5_STATUS.md` (new), `docs/roadmap/INTRO.md` (change - row 5 COMPLETE), `TASKS.md` (change) | Full BUILD.md section 5 checklist (+ gcc -Wall spot checks of new C output, no_std compile sweep, workspace build of all example crates); write PHASE_5_STATUS.md per template 2.3 with exact final counts; INTRO state-table row 5 -> COMPLETE; TASKS.md entry; single close-out commit, clean tree. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| C1 | (phase-wide) | change | `--state-limit`/`--time-limit` from PHASE_5.md are NOT introduced: Phase 4 shipped `--max-states`/`--timeout-sec` (parity-pinned in tests, docs, training data, generated-binary wire format). The shipped names stay canonical; no aliases. |
| C2 | (phase-wide) | remove | `--features` from PHASE_5.md's option list is dropped: its intent is covered by the explicit `--no-std`/`--with-property-monitors` flags; rfs-build exposes cargo feature passthrough separately. |
| C3 | 2. | change | The task sketch's infallible `to_json(&T) -> Json` became FALLIBLE `Ir::to_json(&self) -> Result<Json, String>`: unsigned IR fields (`u64` bounds/int literals, `usize` ids) can hold values up to `u64::MAX` (the parser accepts them), which the i64 value model cannot represent - failing with a clear error beats silent clamping. `from_json` rejects negative integers for those fields. Also confirmed during execution: output is byte-identical to the pre-migration format on all 45 valid examples (compact AND pretty), so no test re-pinning was needed. |

## Other information

- **Execution order / dependencies** (see the task-list summary presented with
  this plan): tasks 1, 2, 4, 9 can start immediately in parallel tracks;
  3 <- 2; 5 <- 4; 6 <- 4+5; 7 <- 5+6; 8 <- 3; 10 <- 2; 11 <- 1+9; 12 <- 6+11;
  13 <- 3; 14 <- 1..13; 15 <- 1..14; 16 <- 14+15. Critical paths: 2 -> 3 -> 8
  (CLI infrastructure track) and 4 -> 5 -> 6 -> 7 (test-generation track);
  11/12 (adapters + examples) run after 1+9.
- **External-tool + script-mode testing**: the resolver precedence (flag > env
  var > PATH) and `--print-script` output are unit-test-pinned with stub tools
  (no real rustc/gcc/dot needed); integration coverage comes from tasks 6/7.
  If `dot` or clang is absent in this environment, the corresponding paths are
  still implemented but their runtime tests gate on availability (documented).
- **Workspace risk**: converting to a workspace changes `cargo test`/alias
  behavior for the root package - task 1 must re-verify every cargo alias in
  `.cargo/config.toml` still works and update BUILD.md accordingly.
- **futures dependency** (async adapter + async example): new external dep,
  confined to `crates/rfs-adapters` / example crates - the `rfs` toolchain
  crate itself stays dependency-light (nom/serde only + optional z3).
- **Graphviz**: check whether `dot` exists in this environment before task 7;
  if absent, the SVG/PNG path is still implemented but its tests use a stub or
  are gated on availability (documented).
- **clang**: check availability for the C libFuzzer example build docs; the
  skeleton generation itself does not require clang.
