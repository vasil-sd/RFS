# Phase 8 Task 12 - CLI commands deep dive (`docs/dev/cli_commands.md`)

Plan rows: 12.1 (all CLI commands + library API surface), 12.2 (workspace
crates: `rfs-build`, `rfs-adapters`, examples), 12.3 (fuzz targets + toolchain
fuzz layer). One task file per plan group; subtask sections are appended at
execution.

## Subtask 12.1 - All CLI commands + library API surface deep dive

### Goal

Fill in `docs/dev/cli_commands.md` (Class A deep dive) for the whole CLI:
eight subcommands, the dispatch in `src/main.rs`, the library API surface in
`src/lib.rs`, and the shared external-tool layer - plus the Class B card
`docs/llm/cli_commands.md` (its TBD is assigned to 12.1; per the confirmed
scope decision each deep-dive task fills both files). The dev doc keeps its
existing Overview anchor byte-stable and gains eight new anchored sections,
one per command family + dispatch + library + ext-tool.

### Context

- **Doc state**: `docs/dev/cli_commands.md` was a 15-line skeleton - one
  Overview section (`dev:cli-commands-overview`) whose TBD (task 12.1)
  described the whole CLI at a high level; no other sections existed.
  `docs/llm/cli_commands.md` was a 12-line card skeleton with budget 200 and
  a single TBD (task 12.1) naming "the 13 CLI/diagram files" (actual count of
  the files covered: 15 - ten command modules + lib.rs + four diagram format
  plugins; diagrams/mod.rs helpers are described inside the diagram section).
- **Fragment structure** (house pattern: one fragment per major `##` section):
  keep `dev:cli-commands-overview`; add eight new sub-fragments:
  `dev:cli-commands-dispatch` (main.rs), `dev:cli-commands-library`
  (lib.rs), `dev:cli-commands-frontend` (parse/validate, frontend.rs),
  `dev:cli-commands-info` (info + info format), `dev:cli-commands-test`
  (test_cmd.rs), `dev:cli-commands-mc-ref-gen-spec` (mc_ref_cmd.rs +
  gen_spec_cmd.rs + specgen.rs), `dev:cli-commands-diagram` (diagram.rs +
  diagrams/), `dev:cli-commands-ext-tool` (ext_tool.rs).
- **No duplication with existing fragments**: docs/dev/codegen_core.md owns
  the codegen command registry (the `rfs codegen` section links to it, no
  restatement); docs/dev/mc_engine.md owns the MC engines (mc-ref links to
  dev:mc-engine-exact for the in-process engine + the 0/1/2 taxonomy);
  docs/dev/backend_rust.md / backend_c.md own the generated artifacts that
  `rfs test` compiles and runs; SEMANTICS.md owns semantics - cite, never
  restate; KB.md section 4.5 owns CLI behavior rules at file granularity;
  docs/user/ is the external user documentation (linked, not duplicated).
- **Verified source material (measured 2026-10-31; line numbers for
  navigation only - doc references use item names)**:
  - `src/main.rs` (941 lines): usage() full text; main() dispatch match at
    66-73 - parse/validate/info/test/diagram/codegen to cmd_* wrappers in this
    file, mc-ref/gen-spec to their run_cli shells; manual parsing loops with
    take_format-style value helpers; cmd_info dispatches on the BARE token
    `format` before path handling (D-P7-6); cmd_codegen stdout mode refuses
    commands with extra files.
  - `src/frontend.rs` (746 lines): Format (Rfs|Json), normalize_abs, collect_
    files_with_ext, plan_conversion (extension replaces never stacks; collision
    guard on normalized absolute paths), run_conversion, validate_one(_with_
    warnings) + ValidateResult/FailedFile, .err side files in directory mode,
    StructuredError (parse = real 1-based line/col, semantic pinned 0/0),
    validate_json_doc / render_validate_json (pinned shapes: file = one
    object, directory = array of failing files).
  - `src/info.rs` (498 lines): InfoOptions{json}, run_info (shared file
    contract; per-module static view; monitor_status via compile_formula -
    identical R1-R4 classification to the backends), render_text / render_json
    (stable key order, all keys always present).
  - `src/info_format.rs` (1623 lines): FormatInfo{name, summary, outputs,
    render}; static FORMATS with TEN entries (spec-rfs from the in-binary EBNF
    model in src/ebnf_grammar.rs; spec-json/ce-trace/mc-report/validate-json/
    info-json via the shared draft-07 emitter in src/json_schema.rs - mc-
    report reuses the ce-trace counterexample root through
    counterexample_model(); diagram-dot/puml/mmd line grammars; artifact-
    header); find/list_markdown/render. D-P7-9 sync tests:
    tests/info_format_test.rs.
  - `src/test_cmd.rs` (890 lines): TestSuite(Unit|Fuzz), TestOptions (default
    backend rust, suites [unit], 30 s fuzz budget), resolved_fuzz_flags (env
    first then --fuzz-flags); run_test pipeline (load+validate -> get_backend
    + compiler preset via ExtTool::resolve -> codegen registry dispatch ->
    build_fuzz_context BEFORE --print-script -> sequential suites, every
    SELECTED suite runs); run_unit_suite; run_fuzz_c (clang -O1
    -fsanitize=fuzzer,address over the skeleton ONLY - #include pattern makes
    compiling both files a multiple-definition error; c_asan_options =
    detect_leaks=0); run_fuzz_rust (temp cargo-fuzz scaffold, lib root =
    production source, RUST_FUZZ_TOOLCHAIN nightly pin at 598, non-nightly
    RUSTUP_TOOLCHAIN stripped via run_in_without); run_fuzz_binary (-max_
    total_time + resolved flags); render_script/render_fuzz_steps/heredoc
    (byte-pinned in tests/frontend_test.rs).
  - `src/mc_ref_cmd.rs` (390 lines): CeFormat(Text|Json), McRefOptions with
    Default from src/codegen_mc.rs::EXACT_DEFAULT_MAX_STATES (= 1_000_000,
    line 586) / EXACT_DEFAULT_TIMEOUT_SEC (= 0, line 589); run_mc_ref in-
    process over ref_mc (unified report rendering with the generated exact
    binaries); exit_code_for mirrors 0/1/2; NOT-applicable flags documented in
    usage (--seed/--steps/--violate, --threads/--fast, --nondet).
  - `src/gen_spec_cmd.rs` (409 lines): DEFAULT_SEED = 12345 (line 22);
    generate_random / mutate_base; verify_rendered re-parses + validates the
    rendered text before writing (exit 1 on failure); run_cli (fully random
    vs --base/--mutations; K=0 re-renders unchanged; deterministic from seed).
  - `src/specgen.rs` (2094 lines): Rng LCG with the SAME constants as the fuzz
    wire format (6364136223846793005 / 1442695040888963407, lines 24-38),
    next_f64 = top-53-bit double mapping; Bounds{...} + small()/medium();
    SpecGen::generate (validity by construction - raw IR types deliberately
    NOT Arbitrary); MutOp with SEVENTEEN variants; mutate_traced (candidate-
    clone validation gate, 100 redraws/step then skip) / mutate. Consumers:
    rfs gen-spec, fuzz/ libFuzzer targets, tests/toolchain_fuzz_test.rs.
  - `src/diagram.rs` (413 lines): Renderer{Internal (default since Phase 7,
    D-P7-1), External} at line 43; DiagramPlugin{name, description, ext,
    generate, svg_tool, plan_render, render_svg, internal_render_svg,
    internal_renderer_name}; ALL_DIAGRAMS/DIAGRAMS registry at 97-105 with
    FIVE plugins (dot/puml/mmd/table/md); list_diagrams/get_diagram;
    ToolOverride/DiagramOptions; run_diagram (--render svg preconditions
    validated UP FRONT for both execution and --print-script; source written
    next to the SVG with the same stem); render_script.
  - `src/diagrams/` (mod.rs 66, dot.rs 130, plantuml.rs 132, mermaid.rs 202,
    tables.rs 383): shared label helpers (event_label/guard_label/
    emit_labels/states); render_check STRICT semantics (user decision
    2026-10-21 - any spawn error or non-zero exit = Err with captured stderr;
    plantuml exits 200 on syntax errors and still writes an SVG, so the gate
    is on exit status); dot generate + render_svg_internal (layout-rs);
    plantuml generate (bare note-on-link form) + render_svg_internal
    (plantuml-core; known gap: note text not drawn); mermaid generate +
    puppeteer_args/puppeteer_json (--no-sandbox config, overridable via
    RFS_MERMAID_PUPPETEER_ARGS) + script_pup_path + render_svg_internal
    (mermaid-rs-renderer render_strict); tables generate_table/generate_md
    (state x event matrix; no SVG renderer in either mode).
  - `src/ext_tool.rs` (622 lines): ExtTool{name, env_path_var, env_args_var,
    subcommand} with SEVEN presets (RUSTC/GCC/DOT/PLANTUML/MERMAID/CLANG/
    CARGO_FUZZ at 48-90; CARGO_FUZZ = cargo SUBCOMMAND); resolve (CLI flag >
    env var > bare name; args env-first then CLI); ResolvedTool{...} with
    display_name/render/run/run_in/run_in_without; ToolOutput; sh_quote;
    Script (deterministic POSIX sh builder behind every --print-script).
  - `src/lib.rs` (113 lines): all modules public; re-exports (parse_source/
    parse_file, print_ir, ParseError/Span, ir::*, frontend API); collect_rfs_
    files (recursive + sorted - makes the file-driven example suite
    extensible).

### Planned actions

1. **Class A deep dive - `docs/dev/cli_commands.md`**: replace the Overview
   TBD in place (anchor stays byte-stable) and add eight new anchored sections
   (dispatch / library / frontend / info / test / mc-ref-gen-spec / diagram /
   ext-tool), each with a Code: line of resolving item references.
2. **Class B card - `docs/llm/cli_commands.md`**: fill the TBD in place
   (budget 200): responsibility, command-to-module map, key-items table,
   invariants (exit codes, --print-script agreement, determinism, file-input
   contract, external-tool strictness), change impact + deep-dive pointers.
3. **Fragment owners (ORPHAN RULE)**: add the eight new fragment IDs to the
   `//! Docs:` lines of the 15 owning files - src/main.rs (dispatch),
   src/lib.rs (library), src/frontend.rs (frontend), src/info.rs +
   src/info_format.rs (info), src/test_cmd.rs (test), src/mc_ref_cmd.rs +
   src/gen_spec_cmd.rs + src/specgen.rs (mc-ref-gen-spec), src/diagram.rs +
   src/diagrams/{dot,plantuml,mermaid,tables}.rs (diagram), src/ext_tool.rs
   (ext-tool). Doc comments only.
4. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (fmt/clippy/build/test root + workspace; counts unchanged:
   535/535 root, 559/559 workspace); plan row 12.1 -> DONE; check off this
   subtask's boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 12.1)

- [x] `docs/dev/cli_commands.md`: Overview TBD replaced in place + eight new
      anchored sections with full content; no TBD remains for 12.1's scope;
      every technical statement carries a resolving doc-to-code reference
      (gate check 1); the existing anchor is byte-stable.
- [x] `docs/llm/cli_commands.md`: card filled in place within the 200-line
      budget (command map + key items for all covered files; exit-code and
      external-tool strictness rules as the critical constraints).
- [x] Source edits limited to the 15 `//! Docs:` owner lines (doc comments
      only); all cargo counts unchanged from the task-11.3 tree (535/535
      root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      fmt --check clean, clippy --all-targets exit 0, cargo test (+
      --workspace).
- [x] Plan row 12.1 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 12.1)

- No CLI behavior changes of any kind; no new dependencies; no flag renames
  or aliases.
- No codegen registry content in the dev doc (docs/dev/codegen_core.md owns
  it - link only); no MC engine internals (dev:mc-engine-* owns them);
  no backend artifact internals (backend_rust/backend_c own them).
- No workspace-crate content (12.2) or fuzz-target/toolchain-fuzz content
  (12.3) - the test section describes the rfs test pipelines at CLI level and
  links forward.
- No restatement of SEMANTICS.md, KB.md, or docs/user/ content (link only);
  no edits to other docs (START.md/KB.md/BUILD.md pointer sync = task 14).
- No DESIGN.md edits; no new example `.rfs` files.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Subtask 12.2 - Workspace crates deep dive (added at execution, see Task corrections)

### Goal

Fill in `docs/dev/workspace_crates.md` (Class A deep dive) for the cargo
workspace members: `crates/rfs-build` (the build.rs helper API),
`crates/rfs-adapters` (ProtocolFsm trait + four adapters), and the example
projects under `crates/examples/` (three Rust workspace members + two
Makefile-driven C projects) - plus the Class B card
`docs/llm/workspace_crates.md` (its TBD is assigned to 12.2; per the
confirmed scope decision each deep-dive task fills both files). The dev doc
keeps its existing Overview anchor byte-stable and gains three new anchored
sections, one per member family.

### Context

- **Doc state**: `docs/dev/workspace_crates.md` was a 15-line skeleton - one
  Overview section (`dev:workspace-crates-overview`) whose TBD (task 12.2)
  described the three members at a high level; no other sections existed.
  `docs/llm/workspace_crates.md` was a 12-line card skeleton with budget 200
  and a single TBD (task 12.2) naming "the 17 tracked crate files" - verified
  exact: git ls-files shows 17 tracked .rs files under crates/ (rfs-build 2,
  rfs-adapters 7, sync_handler 3, async_handler 3, fuzz_rust 2; the generated
  fuzz/fuzz_targets/arq.rs is git-ignored).
- **Fragment structure** (house pattern: one fragment per major `##` section):
  keep `dev:workspace-crates-overview`; add three new sub-fragments:
  `dev:workspace-crates-build` (rfs-build), `dev:workspace-crates-adapters`
  (rfs-adapters), `dev:workspace-crates-examples` (all example projects).
- **No duplication with existing fragments**: docs/dev/CODE_MAP.md
  dev:code-map-workspace owns the file-level entries for all 23 workspace +
  fuzz files (one-line roles - this doc goes deeper: contracts, invariants,
  test strategy); docs/dev/codegen_core.md owns the codegen commands the
  examples invoke (production/test, --with-protocol-fsm, --no-std - linked,
  not restated); dev:toolchain-fuzz-overview (task 12.3) owns the ROOT fuzz/
  targets - the example fuzz projects are described here at CLI level with a
  forward pointer; KB.md section 4.x owns toolchain behavior rules.
- **Verified source material (measured 2026-11-01; line numbers for
  navigation only - doc references use item names)**:
  - Root Cargo.toml: workspace members = EXPLICIT list of five (rfs-build,
    rfs-adapters, sync_handler, async_handler, fuzz_rust); crates/examples has
    no manifest; the C projects are Makefile-driven and NOT members.
  - `crates/rfs-build/src/lib.rs` (621 lines): RfsBuildError{message};
    resolve_rfs_bin (RFS_BIN preferred - empty/whitespace = unset, nonexistent
    file = error; else bare PATH - documented rebuild limitation); RfsBuild
    builder { spec, backend (default rust), command (default production),
    with_property_monitors, no_std, with_protocol_fsm } with new/backend/
    production/test_command/with_property_monitors/no_std/with_protocol_fsm/
    spec; compile (validation BEFORE environment: backend in {rust,c}, ext
    .rfs/.json, spec exists, OUT_DIR set; artifact = OUT_DIR/<stem>.<rs|c>;
    cargo directives via pure cargo_directives - rerun-if-changed spec + env
    RFS_BIN + bin when pinned + cargo:include; PATH note line; invokes rfs
    codegen <backend> <command> <spec> --out + optional flags; failure = exit
    code + stderr (+ stdout when non-empty); success without artifact =
    error). In-module hermetic tests (stub rfs script records argv, honors
    --out, fails on RFS_STUB_FAIL; ENV_LOCK serializes env-mutating tests;
    cargo_directives_pinned).
  - `crates/rfs-build/tests/integration.rs` (112 lines): the one REAL end-to-
    end check - real spec + workspace rfs binary; rust artifact must COMPILE,
    c backend generates a .c file.
  - `crates/rfs-adapters/src/lib.rs` (54 lines): #![no_std] + alloc;
    ProtocolFsm trait { type Input/Output/Error, new(), step() ->
    Result<Vec<Output>, Error> } - mirrors every generated Rust FSM; codegen
    emits the impl opt-in via --with-protocol-fsm (mechanical forward to the
    inherent Fsm methods). Features: std (default) / async = ["std",
    "dep:futures"] (futures 0.3 optional - the crate's only external dep);
    dev-dep on root rfs (test-only, no_std generation check).
  - `crates/rfs-adapters/src/minimal.rs` (50): Minimal<F>(pub F) - new/step/
    into_inner; itself a ProtocolFsm so adapters compose.
  - `crates/rfs-adapters/src/callback.rs` (64): drive(fsm, decide, max_steps)
    -> CallbackResult { Completed(steps), Failed(e, steps done), StepLimit(
    steps) }; closure gets the PREVIOUS step's outputs (empty first).
  - `crates/rfs-adapters/src/channel.rs` (97): ChannelFsm<F> spawn/
    send_input/recv_outcome - FSM on its own thread, std mpsc FIFO; failed
    step does NOT stop the thread; dropping the handle does not stop it (exits
    when the input channel closes / receiver drops); ChannelOutcome { Outputs,
    Error } with manual Debug/PartialEq bounded on the ASSOCIATED types (the
    generated Fsm derives neither).
  - `crates/rfs-adapters/src/async_adapter.rs` (107): AsyncFsm<F> new/
    submit/close; Stream impl steps SYNCHRONOUSLY in poll_next (generated step
    does no I/O - any executor works, no tokio); one queued input per poll,
    Pending until close then Ready(None); Unpin bounds keep it unsafe-free;
    AsyncOutcome { Outputs, Error }.
  - `crates/rfs-adapters/tests/adapters.rs` (234): in-crate MOCK FSM (no
    generated code needed) - minimal forwards + is itself a ProtocolFsm;
    callback stops/errors/limits; channel_tests + async_tests modules.
  - `crates/rfs-adapters/tests/nostd_minimal.rs` (168): no_std exit criterion
    - core-only adapters compile inside #![no_std] crates, with a hand-written
    mock AND against a REAL task-9 no_std generated FSM (opt-in ProtocolFsm
    impl); gated on rustc availability, generation unconditional.
  - Example build.rs files (42/42/61 lines): shared pattern - RFS_BIN >
    workspace target/debug/rfs (set_var when not pinned; cold checkout needs
    cargo build first) > PATH; spec = examples/valid/43_sliding_window.rfs;
    strip `#!` lines from the artifact (include! cannot carry inner
    attributes), byte-identical body to OUT_DIR/43_sliding_window_body.rs;
    module-level #[allow(dead_code, unused_variables, unused_mut,
    unused_parens, irrefutable_let_patterns, non_camel_case_types,
    clippy::all)] mirrors the artifact header exactly; body outside the source
    tree so cargo fmt never sees generated code. sync/async_handler use
    --with-protocol-fsm; fuzz_rust additionally runs the TEST command and
    adapts the emitted skeleton (`use crate::{` -> `use fuzz_rust::{`,
    asserted) into fuzz/fuzz_targets/arq.rs (git-ignored build artifact).
  - Example libs/mains: sync_handler run() = ChannelFsm::<Fsm>::spawn() +
    scripted peer (wake tick, ticks until all 8 frames sent+acked, out-of-band
    Ack every third tick must be dropped) -> ArqReport { frames_sent,
    acks_received, out_of_band_acks_dropped, ticks, max_window }; async_handler
    run() = the SAME scenario over AsyncFsm with futures::executor::block_on
    (no tokio); fuzz_rust lib = include! body + pub use arq::* at the root.
  - C projects: c_sync_handler (Makefile: rfs codegen c production ->
    build/arq_fsm.c; handler.c #includes it DIRECTLY - single translation
    unit, same wiring as the fuzz skeleton; gcc -Wall zero-warning bar; data
    lines byte-identical to the Rust sync_handler); fuzz_c (generation needs
    only rfs - production under the exact name the skeleton's #include expects
    + test-command skeleton in build/ so no -I needed; building needs a
    libFuzzer-enabled clang: clang -O1 -fsanitize=fuzzer,address over the
    skeleton only; run = bounded smoke -max_total_time=10).
  - tests/example_projects_test.rs pins all of it: c_sync_handler wall-clean +
    Rust scenario parity; fuzz_rust skeleton compiles in its cargo-fuzz layout;
    fuzz_c generation without clang (build gated on clang availability).

### Planned actions

1. **Class A deep dive - `docs/dev/workspace_crates.md`**: replace the
   Overview TBD in place (anchor stays byte-stable) and add three new anchored
   sections (rfs-build / rfs-adapters / examples), each with a Code: line of
   resolving item references.
2. **Class B card - `docs/llm/workspace_crates.md`**: fill the TBD in place
   (budget 200): responsibility, crate-to-file map for the 17 tracked files,
   key-items table, invariants (build-script contract, dependency hygiene,
   adapter generics, no_std, example parity, cold checkouts), change impact +
   deep-dive pointers.
3. **Fragment owners (ORPHAN RULE)**: add the three new fragment IDs to the
   `//! Docs:` lines of all 17 tracked crate files - rfs-build x2 (build),
   rfs-adapters x7 (adapters), examples x8 (examples). Doc comments only.
4. **Verification + bookkeeping**: dev_docs_test 9/9, docs_md_test 5/5, full
   verification (fmt/clippy/build/test root + workspace; counts unchanged:
   535/535 root, 559/559 workspace); plan row 12.2 -> DONE; check off this
   subtask's boxes; TASKS.md entry; commit per action.

### Exit criteria (subtask 12.2)

- [x] `docs/dev/workspace_crates.md`: Overview TBD replaced in place + three
      new anchored sections with full content; no TBD remains for 12.2's
      scope; every technical statement carries a resolving doc-to-code
      reference (gate check 1); the existing anchor is byte-stable.
- [x] `docs/llm/workspace_crates.md`: card filled in place within the 200-line
      budget (crate-to-file map for all 17 tracked files; build-script
      contract + ProtocolFsm surface as the critical constraints).
- [x] Source edits limited to the 17 `//! Docs:` owner lines (doc comments
      only); all cargo counts unchanged from the task-12.1 tree (535/535
      root, 559/559 workspace).
- [x] Full verification green: dev_docs_test 9/9, docs_md_test 5/5, cargo
      fmt --check clean, clippy --all-targets exit 0, cargo test (+
      --workspace).
- [x] Plan row 12.2 marked DONE; this subtask's boxes checked; TASKS.md entry
      appended.

### Non-goals (subtask 12.2)

- No workspace-crate behavior changes of any kind; no new dependencies;
  no feature-gate changes.
- No codegen command internals in the dev doc (docs/dev/codegen_core.md owns
  them - link only); no root fuzz/ target content (task 12.3) - the example
  fuzz projects are described at CLI level with a forward pointer.
- No restatement of CODE_MAP file entries, KB.md rules, or SEMANTICS.md
  content (link only); no edits to other docs (START.md/KB.md/BUILD.md
  pointer sync = task 14).
- No DESIGN.md edits; no new example specs or crates.
- No line-number-based anchors anywhere; stable item-name anchors only.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Subtask 12.2 appended at execution (2026-11-01) | Per the correction above: subtask sections are written when executed, following the PHASE_8_TASK_9.md / PHASE_8_TASK_11.md pattern. Written with verified source material measured 2026-11-01 (item maps of all 17 tracked crate files + both C projects' Makefiles/READMEs; git ls-files confirms the card's "17 tracked crate files" count exactly - fuzz/fuzz_targets/arq.rs is a git-ignored build artifact), fragment plan with exact owner-line targets (all 17 files, one new ID each), no-duplication rule vs CODE_MAP dev:code-map-workspace + codegen_core + the upcoming toolchain-fuzz doc. |
| Task file created at execution (2026-10-31) for plan group 12 | User model: one task file per plan group ("execute 12.1" = subtask of task file 12). Written for subtask 12.1 only; subtasks 12.2 (workspace crates) and 12.3 (toolchain fuzz) will be appended at execution, following the PHASE_8_TASK_9.md / PHASE_8_TASK_11.md pattern. |
| Class B card in scope for 12.1 | The plan row names only `docs/dev/cli_commands.md`, but the card skeleton's TBD is explicitly assigned to task 12.1 and the confirmed scope decision (task 11) is that each deep-dive task fills both files - so the card was filled in the same action. |
