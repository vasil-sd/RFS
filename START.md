# START.md - Entry point for LLM-assisted RFS development

## 0. System prompt (read this first, then stop reading other files until you have a task)

You are an expert Rust and domain-specific-language research assistant continuing the
development of the **RFS** project: a finite-state protocol specification language
that compiles to one versioned IR and multiple targets (production Rust FSMs, unit
tests, bounded model checkers; C backend planned).

Project root: `/workspace/host/RFS`. Core principle: *one constrained specification
-> one versioned IR -> multiple Rust targets.* The DSL stays finite and
model-checkable (no unbounded data, floats, or arbitrary functions in the core).

**Operating protocol (token economy - follow it strictly):**

1. This file is the only document you read fully at session start. Everything else is
   read **on demand**, in targeted chunks, when a task requires it (see section 6).
2. Never load whole large documents into context. For `KB.md` / `DESIGN.md` use
   `grep -n "^## \|^### " <file>` to get current line numbers, then
   `sed -n '<a>,<b>p'` for just the needed section. Roadmap files in
   `docs/roadmap/` are small by design: read `INTRO.md` once per phase task and
   only the specific `PHASE_N*.md` file you need (never all of them).
3. Toolchain: do not search for cargo/rustc. Use the activation block in section 4.
   If a build fails with a lock-file or version error, read `TOOLCHAIN.md` (it is
   short) and follow it - do not try other toolchains first.
4. Before building/testing anything, check `BUILD.md` section 5 (verification
   checklist) for what your change type requires.
5. When a task is finished, append one entry to `TASKS.md` using the template in its
   FIRST paragraph only (the file contains an instruction block telling you exactly
   how; never read the whole history into context).
6. Keep `cargo test`, `cargo clippy --all-targets`, and `cargo fmt` green, and keep
   the docs synchronized with design-relevant changes (section 8 rules).
7. Images are PROHIBITED by default in all answers and reports. Do NOT run image
   searches (e.g. `web_search` with `image_queries`) or download/embed images to
   illustrate your work - for this project's subject matter such searches almost
   always return irrelevant results (unrelated acronyms, stock templates, unrelated
   diagrams) and pollute the answer with misleading visuals. Include an image ONLY
   when a task strictly requires one that cannot be conveyed in text or code (for
   example an important math equation or an essential diagram). When in doubt: no
   image at all.
8. Phase work (planning or implementing any project phase) follows the two-stage
   protocol in `docs/roadmap/INTRO.md`: the planning stage ends with a written
   `PHASE_N_PLAN.md` plus per-task `PHASE_N_TASK_M.md` files, each followed by a stop
   for user review; the implementation stage requires that plan file to exist before
   any code is touched and executes tasks one at a time - every task starts only
   after explicit user confirmation.

## 1. Project state snapshot (2026-10-19)

- Phase 0 (parser/IR baseline), Phase 1 (validator, plugin-based Rust codegen,
  frontend CLI `parse`/`validate`/`codegen`), Phase 2 (core DSL + C backend) and
  **Phase 3 (PSL-like properties and monitors)** are COMPLETE, and **Phase 4
  (Scalable model checking)** is COMPLETE at implementation level: tasks 1-13
  done; close-out task 14 records the final verification run +
  `PHASE_4_STATUS.md`. Phase 4 added: the EXACT reachable-state MC engine as
  the default `mc` mode (seeded fuzz retained behind `--mode fuzz`; shared
  codegen-time plan `src/mc_plan.rs` embedded in BOTH backends), liveness
  operators `eventually`/`until` (weak until) with lasso counterexamples, the
  three-status taxonomy `pass-complete`/`fail`/`unknown-state-limit` with a
  pinned summary line, input abstraction for unused payload fields, benchmark
  specs 43/44/45 + the benchmark parity suite, level-parallel exact BFS
  (`--threads N`, byte-identical to N=1; runtime-only non-parity `--fast`), and
  an optional SMT bounded mode (`--mode smt-bounded`, feature-gated `smt`,
  shared plan + emitter). Design decisions D-P4-1..D-P4-14: `DESIGN.md`
  section 22.
- Next work: execute Phase 4 task 14 (close-out) per
  `docs/roadmap/PHASE_4_PLAN.md`; phase planning/implementation protocol:
  `docs/roadmap/INTRO.md`.
- Known-good baseline: 230/230 main tests (232 with `--features smt`),
  537/537 generated-code tests (179 modules), clippy without errors, fmt clean;
  45 valid + 12 invalid examples; 11 training categories / 231 aggregated SFT
  examples (130 valid + 127 invalid snippets).

## 2. Documentation map (what each file is for)

| File | Content | Read it when... | How to read |
|---|---|---|---|
| `START.md` | This file: orientation + protocol | Always, at session start | Fully (it is short) |
| `BUILD.md` | Build files, all commands/aliases, test suites, expected results, post-change checklist | Before any build/test/run; when a command or its expected output is needed | Fully (~90 lines, safe) |
| `TOOLCHAIN.md` | Rust toolchain paths/versions, activation, reinstall, env corner cases | Only when cargo/rustc fails or exact versions are needed | Fully (~70 lines, safe) |
| `KB.md` | Design decisions, RFS v0 syntax reference, source-code map, CLI behavior, training-set organization, agent workflow experience | For language semantics questions, source layout, CLI rules, "how is X implemented" | **Never fully** - section map in section 5; grep + sed ranges |
| `DESIGN.md` | RFS v0 design: architecture, core semantics, EBNF grammar, IR schema, parser/codegen/MC/PSL plans, design-choice tables | When changing language semantics, grammar, IR, or codegen/MC design | **Never fully** - section map in section 5 |
| `docs/roadmap/INTRO.md` | Roadmap overview, phase-state table, formatting templates, two-stage planning/implementation protocol | When scoping, planning, or implementing any phase; checking next priorities | Fully (short) |
| `docs/roadmap/PHASE_N.md` / `PHASE_N_PLAN.md` / `PHASE_N_TASK_M.md` / `PHASE_N_*_STATUS.md` | Phase N description (goal/scope/deliverables/exit criteria/non-goals) / implementation plan / per-task implementation details / phase or task completion status report | Description: when working on phase N. Plan: fully, before implementing. Task file: fully, before executing that task. Status: for completed phases/tasks | Fully (each is short by design) |
| `CODEGEN.md` | Codegen backend plugin API, registry, how to add a new target language, testing conventions | When touching `src/codegen.rs`, `src/backends/*`, or adding a backend | Fully (~280 lines, safe) |
| `src/backends/RUST.md` / `src/backends/C.md` | Backend internals: module split, type mapping, value representation, MC wire format | When changing generated-code output for that language | Fully (~90-150 lines, safe) |
| `TASKS.md` | Append-only history of LLM-assisted tasks (interpretation + report per task) | To recall how a similar task was done: `grep -n "<keyword>" TASKS.md` then read that section; to append: read ONLY its first paragraph (template + rules) | Never fully |
| `training.py` docstring | SFT+GRPO training pipeline for Qwen3.8-27B on the RFS data | When running/adjusting LLM training | Read only the module docstring (`head -60 training.py`) |

## 3. Source tree (one level; details in KB.md section 4.2)

```text
src/parser.rs        nom-based RFS v0 parser          src/ir.rs         versioned serde IR
src/frontend.rs      parse/validate CLI logic         src/main.rs       CLI arg parsing + dispatch
src/validator.rs     semantic validation + typing     src/printer.rs    IR -> formatted RFS
src/codegen.rs       backend plugin registry + CLI    src/backends/     rust*.rs + c*.rs backends (+ RUST.md, C.md)
src/codegen_fsm.rs   shared production-FSM analysis   src/codegen_test.rs  unit-test planning
src/codegen_mc.rs    shared exact-mode concepts (statuses, summary line, flag defaults)
src/mc_plan.rs       shared MC plan: abstraction, encoding, traversal, liveness obligations
src/monitor.rs       shared monitor compiler (Phase 3 MonitorPlan)
src/smt_plan.rs + src/smt_emit.rs   shared SMT spec + emitter (optional smt mode)
src/training.rs      snippet extraction + aggregation
src/error.rs         ParseError/Span                  src/bin/          build_training, extract_training_snippets,
                                                           generate_code_tests, test_generated_code
tests/               examples_test, frontend_test, codegen_test, integration_example_test,
                   library_comparison, training_test
examples/valid       40 .rfs files (file-driven suite)   examples/invalid  11 .rfs files (must fail)
training/<cat>/      11 categories: rfs_training.jsonl + snippets/{valid,invalid}/
all_rfs_training.jsonl   aggregated SFT set (219 lines) at repo root
```

## 4. Toolchain essentials (do not search - use this)

Canonical toolchain (cargo/rustc 1.99.0):
`/workspace/host/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin`.
Activate in every shell command that needs cargo (env does not persist between calls):

```sh
export RUSTUP_HOME=/workspace/host/rustup
export CARGO_HOME=/workspace/host/.cargo
export PATH="$CARGO_HOME/bin:$PATH"
cd /workspace/host/RFS
```

Do NOT use system `/usr/local/bin/cargo` (1.57.0, cannot read this `Cargo.lock`).
Paths, reinstall, and failure recovery: `TOOLCHAIN.md`.

## 5. Large-doc section maps (locate with grep, read only the needed range)

Line numbers drift as docs evolve - always run `grep -n "^## \|^### " <file>` first,
then `sed -n '<start>,<end>p'`. Sections:

- **KB.md**: 1 current project state / 2 design decisions / 3 RFS v0 syntax reference
  (3.1-3.9) / 4 Rust source code map (4.1 deps, 4.2 files, 4.3 public API, 4.4 parser
  internals, 4.5 CLI behavior, 4.6 training-set organization, 4.7 TLA+ adaptation) /
  5 build-test-run pointers / 6 agent workflow experience (6.1 env corner cases, 6.2
  editing workflow, 6.3-6.5 parser findings, 6.6 doc workflow) / 7 known limitations /
  8 updated roadmap / 9 historical R&D plan (superseded) / 10 quick-start pointer /
  11 rules for future work / 12 what to work on next / 13 Phase 1 codegen + bounded MC.
- **DESIGN.md**: 1 purpose / 2 architecture / 3 core semantics / 4 model-checking
  constraints / 5 RFS v0 language subset / 6 EBNF grammar / 7 IR v0 schema / 8 parser
  design / 9 major design choices / 10 production codegen plan / 11 MC plan /
  12 PSL-like property language plan / 13 diagnostics + validation plan /
  14 toolchain CLI plan / 15 testing strategy / 16 risks / 17 Phase 0 deliverables /
  18 example spec + Phase 1 codegen decisions.
- **docs/roadmap/**: split roadmap (replaces the old `ROADMAP.md`, removed
  2026-10-16): `INTRO.md` (overview + templates + protocol), `PHASE_0..6.md`
  (phase descriptions), `PHASE_N_PLAN.md` / `PHASE_N_STATUS.md` where they exist
  (status exists for completed phases 0-2). All small: read the specific file you
  need, fully.

## 6. Task -> read / do / verify table (lazy loading protocol)

| Task type | Read (on demand) | Do | Verify |
|---|---|---|---|
| Add a valid/invalid example | `BUILD.md` section 3 (suite expectations) | Drop `.rfs` into `examples/valid|invalid` - no test-code change | `cargo test --test examples_test` |
| Change RFS syntax/grammar | `DESIGN.md` sections 5-6 + `KB.md` sections 3, 6.5; add examples FIRST | Update parser/validator | Full checklist (`BUILD.md` section 5) + update DESIGN/KB |
| Change the IR | `DESIGN.md` section 7 + serde attrs in `src/ir.rs` | Decide backward compat; bump `IR_VERSION` if needed | JSON round-trip tests + full checklist |
| Add a codegen backend language | `CODEGEN.md` fully + `src/backends/RUST.md` / `src/backends/C.md` as references | Implement 3 command fns in `src/backends/<lang>*.rs`, register static in `BACKENDS` | per-command generation tests (see CODEGEN section 8) |
| Phase implementation (next: Phase 3 properties/monitors) | `docs/roadmap/INTRO.md` fully + `docs/roadmap/PHASE_N.md`; phase-specific docs (`DESIGN.md` section 12 PSL plan, `KB.md` section 2.6 for Phase 3) | Two-stage protocol from INTRO.md: planning stage writes `PHASE_N_PLAN.md`, then stops for user review; implementation stage executes the plan, tracks corrections, writes `PHASE_N_STATUS.md` at exit criteria | Full checklist + docs sync + TASKS.md entry |
| Improve validator/codegen checks | `DESIGN.md` sections 19-20 (decision tables D1-D10) + `KB.md` section 7 | Implement checks, add invalid examples first | Full checklist |
| Training-set work | `KB.md` section 4.6 (format rules: 2 messages, `< think> ... < /think>` assistant, snippets auto-extracted) | Edit `training/<cat>/rfs_training.jsonl` | `cargo extract-training-snippets && cargo build-training && cargo test-training` |
| LLM training run (GPU) | `head -60 training.py` (docstring only) + `BUILD.md` last command row | `python training.py --stage both` | `--dry-run` first for CPU-only data/reward validation |
| Recall how a past task was done | `grep -n "<keyword>" TASKS.md`, read that section only | - | - |

## 7. Build & test essentials (full table in BUILD.md)

```sh
cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check   # after any change
cargo test-generated-code        # 513 generated unit tests (after codegen changes)
cargo run -- validate --input rfs examples/valid    # quick sanity, exit 0 expected
```

Expected results and every command/alias: `BUILD.md` sections 3-4.

## 8. Workflow rules (condensed from KB.md section 11)

- Preserve the core principle: one constrained RFS spec -> one IR -> multiple Rust targets.
- Keep the DSL finite and model-checkable; no unbounded data, floats, arbitrary
  functions, or dynamic allocation in the core language.
- Prefer small, testable increments over large rewrites.
- Do not break the file-driven example suite (new examples work without test changes).
- Keep training data category-local: add SFT examples to
  `training/<category>/rfs_training.jsonl`, then regenerate snippets + aggregate.
- Keep `cargo test`, `cargo clippy --all-targets`, and `cargo fmt` green.
- Keep `DESIGN.md`, the `docs/roadmap/` phase files (incl. status reports), and
  `KB.md` (and `BUILD.md`/`TOOLCHAIN.md` when commands/toolchain change)
  synchronized with design-relevant changes.
- Append a `TASKS.md` entry after every completed task (template in its first paragraph).

Before changing language syntax:

1. Search `DESIGN.md`, the relevant `docs/roadmap/PHASE_N.md`, and `KB.md` for the
   affected decision.
2. Add valid and invalid examples first if possible.
3. Update parser or validator.
4. Run full tests.
5. Update documentation.

Before changing IR:

1. Check serde attributes in `src/ir.rs`.
2. Decide whether the change is backward compatible.
3. If not, consider bumping `IR_VERSION` or adding a migration note.
4. Update JSON round-trip tests.
