# Phase 6, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_6_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

None (documentation-only task per the task non-goals).

## Changed/removed source files

None.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/user/README.md` | new | Index + "which doc for which question" routing table, toolchain component diagram (mermaid), two-minute tour with real v1.0.0 outputs, path map to examples/SEMANTICS/ide; the entry point of the external user doc set |
| `docs/user/LANGUAGE.md` | new | Language reference: quick start, module + v1.0 version header (D-P6-1), states/enums, events/outputs + flattening rules + anonymous-payload limitation, variables + C width limits, transitions (patterns table, guards, simultaneous updates, priorities + validator rules, `on_error`), Tick/timers, expression language (operators/wrapping/precedence/typing), and a complete beacon example with line-by-line walkthrough + state diagram; all semantics cross-referenced to SEMANTICS.md sections |
| `docs/user/PROPERTIES.md` | new | Property reference: properties vs assumptions, sampling model (sample 0 = initial configuration), atoms, every operator with one-line meaning + tiny example, the bounded-eventually obligation model with the exact stepping algorithm + lifecycle diagram, liveness (`eventually`/weak `until`, MC-only), monitor subset R1-R4 + skip reasons with REAL `rfs info` monitored/skipped output (spec 43), assumption handler policies (Continue/Stop/Panic) + selection rules + sample-0 panic, strong vs weak liveness + fairness, and worked examples 13/15/16/42 with ACTUAL `rfs mc-ref` outputs |
| `docs/user/MODEL_CHECKING.md` | new | MC modes reference: exact/fuzz/SMT when-to-use table, `rfs mc-ref` (unified flags + exit codes 0/1/2 + verified byte-parity with both generated binaries), FULL flag tables for the generated binary (exact: --max-states/--nondet/--timeout-sec/--threads/--fast/--counterexample-format/--assumes; fuzz: --mode/--bound/--runs/--seed/--violate/--unexpected-prob) with the phase-order diagram, status taxonomy + the no-false-PASS labeling rules with a decision diagram, counterexamples (real text anatomy from spec 13, real JSON document, real liveness lasso from spec 45 `--assumes none`), forced violations explained with a real spec-48 run (161/200 assumption stops), step-by-step report reading, and abstraction caveats pointing to FAQ |
| `docs/user/INTEGRATION.md` | new | Production integration guide: build.rs via rfs-build (full builder-options table + RFS_BIN resolution + include!/OUT_DIR pattern from sync_handler's real build.rs), ProtocolFsm trait + the four adapters (ChannelFsm/AsyncFsm/drive callback/Minimal) with a minimal channel-driver sample + session sequence diagram, `--no-std`, C Makefile pattern (verbatim from c_sync_handler), `rfs test` unit/fuzz pipelines with exit-code diagram, external-tool parameterization table (RFS_* env vars, CLI precedence, `--print-script` with real header), checked-in-artifact verification (real line-1 header with spec hash + toolchain version, MIGRATION.md pointer), and the `rfs gen-spec` random/mutation prototyping workflow with real outputs |
| `docs/user/EXAMPLES.md` | new | Curated walkthroughs of specs 01/03/04/43 (deep dive incl. input abstraction + priorities + terminal-state convention)/44/45/47/48/49 with a mermaid state diagram per FSM (+ sequence diagrams for the spec-04 handshake and spec-43 ARQ session), a real-result MC summary table (all numbers from live `rfs mc-ref` runs, matching the benchmark pins), verified runnable session commands for both example ecosystems, and the TLA+ adaptation strategy note |
| `docs/user/FAQ.md` | new | FAQ: representative inputs + soundness argument, assumption policies and their MC effect (incl. the two-assumption spurious-deadlock trap), nondeterministic vs deterministic modes, the "working spec fails with deadlock" anatomy (unexpected termination vs deadlock; why 04/15/16/42 fail and 43/47/48 absorb), when a PASS is trustworthy (labeling guarantees + trust checklist diagram), C bit-width limits (+ task-2a UB fix note), version headers/migration pointer, and the v1.0 known-limitations table from KB.md section 7 |

Total: 2135 lines across the seven files, 24 mermaid diagrams (flowchart,
stateDiagram-v2, sequenceDiagram) - every one rendered through mdr's exact
pipeline by `tests/docs_md_test.rs` as part of plain `cargo test`.

## Other file changes

| Name | Operation | Description |
|---|---|---|
| `Cargo.toml`, `Cargo.lock` | changed (pre-task prep, 2026-10-25) | dev-deps `comrak = { version = "0.55", default-features = false }` + `mermaid-rs-renderer = { version = "0.3", default-features = false }` - mdr's parser parts at the exact versions mdr 0.6.1 uses (mdr itself is binary-only and cannot be a library dependency); see the task file's Task corrections |
| `tests/docs_md_test.rs` | new (pre-task prep, 2026-10-25) | the mdr-stack doc-quality suite (5 tests): comrak parse under mdr's GFM extension set, fenced-block closing, full-pipeline mermaid rendering of every ```mermaid block, relative-link resolution - now covering all seven new docs |
| `docs/roadmap/PHASE_6_TASK_7.md` | changed | Task corrections section (2026-10-25 rule change) + Other information updates (mdr findings, verified mermaid syntax notes, verified-runnable sample list) |

## Changed functionality

None - documentation-only; no source, example, or behavior change.

## Removed functionality

None.

## New functionality

- External user documentation set (`docs/user/`, 7 files):
  - self-contained for people who have never seen the repository (no internal jargon without a one-line definition);
  - every command example copy-paste runnable from the repository root and verified by actually running it (sample list in "Other important findings/information");
  - all quoted tool outputs captured live from this repository's v1.0.0 toolchain (machine-dependent values explicitly marked as such);
  - 24 mermaid diagrams illustrating project structure, the codegen pipeline, FSMs as state diagrams, protocols as sequence diagrams, and MC decision flows - plain text, versionable, machine-checked;
  - cross-linked with `docs/SEMANTICS.md` (normative), `examples/`, `ide/README.md`, and within the set; each page carries a table of contents.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none added by this task) | - | the mdr-stack doc-quality suite (`tests/docs_md_test.rs`, 5 tests) was added during pre-task prep per the 2026-10-25 user rule change and now validates the seven new docs (parse, fence closing, mermaid rendering, link resolution) as part of plain `cargo test` |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (453) | OK |
| `cargo test --test docs_md_test` (5) | OK |
| `cargo fmt --check` | OK |
| doc-example verification runs (see findings section for the list) | OK - all exit 0 with the documented outputs |

## Found problems/issues/mismatches

- The task outline's atom list mentions a `state_is` atom; NO such atom exists
  on the shipped surface - state checks are expression atoms (`state == X`)
  per SEMANTICS.md section 5.1 (verified against the parser/validator and all
  50 valid examples). Documented the ACTUAL atom set (expression /
  `input_is` / `output_has`) per the normative document; did not document
  fiction.
- Bare `make -C crates/examples/c_sync_handler run` fails in this environment
  (`rfs` is not on PATH; the Makefile resolves RFS_BIN > bare rfs). The doc
  shows the working form `make ... RFS_BIN=$PWD/target/debug/rfs` and notes
  that a bare `rfs` on PATH works for installed toolchains.
- Educational specs 04/15/16/42 FAIL exact MC with deadlock (terminal states
  without absorbing arms) even though their properties hold - expected per
  SEMANTICS 8.4, not a bug; documented as the canonical teaching case in FAQ
  section 4 and EXAMPLES.md instead of hiding it.
- `rfs mc-ref` reports `peak_rss_kb: 0` in its JSON (in-process run) while
  generated binaries self-report real values - the docs state this explicitly
  rather than claiming full byte-identity for the JSON documents (parity is
  modulo that informational field, exactly as the test suites normalize it).

## Other important findings/information

- **Verified-runnable sample (recorded per the task file's Other information)**
  - every command below was executed in this repository during task execution
    and produced the output quoted in the docs:
  - `cargo run -- validate examples/valid/01_trivial_two_state.rfs` (exit 0)
  - `cargo run -- validate --input rfs examples/valid` (directory mode, exit 0, all 50 valid)
  - `cargo run -- validate /tmp/beacon.rfs` + `mc-ref /tmp/beacon.rfs` (the LANGUAGE.md line-by-line example spec, written via the doc's self-contained heredoc: PASS complete, 7 states / 28 transitions / max depth 5)
  - `cargo run -- mc-ref <spec>` on specs 01, 13, 15, 16, 42, 43, 46 and all nine EXAMPLES.md walkthrough specs (01/03/04/43/44/45/47/48/49) - every quoted summary line is the live output
  - `cargo run -- codegen rust|c mc` + `rustc --edition 2021 -O` / `gcc -Wall` compile + run for specs 13, 04, 45, 48; text reports diffed BYTE-IDENTICAL between `rfs mc-ref`, the Rust binary, and the C binary (spec 13); JSON identical modulo `peak_rss_kb`
  - `./mc45 --assumes none` (real liveness lasso quoted in MODEL_CHECKING.md section 6)
  - `./mc48 --mode fuzz --seed 12345 --bound 20 --runs 200 --violate env_progress:1.0:100%` (161 assumption stops quoted in section 7) and `./mc13 --mode fuzz --seed 12345` (exit 1, violation trace quoted in section 4)
  - `cargo run -- codegen rust|c production examples/valid/46_version_header.rfs` (real line-1 headers with spec hash quoted in INTEGRATION.md section 7) and `... production examples/valid/01_trivial_two_state.rfs --out /tmp/trivial_fsm.rs`
  - `cargo run -- info` on specs 04 and 43 (monitored/skipped sections quoted verbatim)
  - `cargo run -- diagram --format table examples/valid/03_timeout_retry.rfs` (pure text, no tools)
  - `cargo run -- test examples/valid/46_version_header.rfs` (full e2e: codegen + rustc compile + run, 8 generated tests pass, exit 0) and `cargo run -- test --backend rust --print-script ... | head` (real script header quoted)
  - `cargo run -- gen-spec --out /tmp/rand.rfs --seed 42 --size small` and `--base examples/valid/03_timeout_retry.rfs --mutations 3 --seed 7 --out /tmp/mut.rfs` (+ diff, abridged output quoted in INTEGRATION.md section 8)
  - `cargo run -p sync_handler` (exit 0; pinned scenario: 19 ticks, frames/acks [0..8], 5 dropped out-of-band acks, max window 4) and `make -C crates/examples/c_sync_handler run RFS_BIN=$PWD/target/debug/rfs` (byte-identical data lines)
- **Mermaid coverage**: 24 diagrams across the set - structure/pipeline flowcharts (README, LANGUAGE, MODEL_CHECKING x2, INTEGRATION), FSM state diagrams for every walked example (01/03/04/43/45/47/48/49 + LANGUAGE's beacon + PROPERTIES' obligation lifecycle + FAQ's trust checklist), protocol sequence diagrams (spec-04 handshake, spec-43 ARQ session, INTEGRATION's channel adapter). All render in the pinned mermaid-rs-renderer 0.3.1 (diamond decision nodes included - verified by the positive-control test).
- **Page sizes**: largest page is LANGUAGE.md at 487 lines; all far below the 3000-line limit set by the 2026-10-25 rule change (diagram code counted).
- **For task 8 (training data)**: the user docs' command examples and quoted outputs are now the canonical final-form material to quote in training entries; `docs/user/` is the pointer target for the START.md doc-map row that task 9 adds.
