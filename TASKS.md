# LLM ASSISTANT INSTRUCTIONS — READ THIS SECTION ONLY, THEN STOP READING THIS FILE

This file is an append-only history of user requests for RFS project updates: each entry records the assistant's interpretation/plan and a compact report of what was actually done. It exists so future sessions can recover context cheaply.

**Token-saving rule (mandatory):** Do NOT read this whole file into your context. Read only this first section (instructions + template). If you need history, locate entries surgically instead, for example:

```sh
grep -n "^## " TASKS.md | tail -5          # recent entry headings only
sed -n "<start>,<end>p" TASKS.md            # one specific entry by line range
```

**When to append:** after completing any user request that updates the RFS project (code, tests, docs, training sets). Do not create entries for TASKS.md maintenance itself.

**How to append:** add ONE new section at the END of the file using the template below. Never rewrite, reorder, or delete existing entries. Keep entries compact: short bullets, exact numbers (test counts, file/line facts), names of changed files; no prose padding. Date/time in local time when the task was completed.

**Entry template:**

```markdown
## YYYY-MM-DD HH:MM — <one-line short description>

### Interpretation & plan

- bullet: how the request was understood (scope, ambiguities resolved)
- bullet: planned work steps

### Report

- bullet: what was actually changed (files, APIs, commands, counts)
- bullet: key findings discovered during the task
- bullet: verification results (exact test/exit-code outcomes)
```

---

## 2026-10-14 20:13 — Actualized SFT training sets to current project state; added frontend_cli category

### Interpretation & plan

- Requested: update/actualize all `training/*.jsonl` files against the current state of docs, source, and tests; add new categories if needed; refresh code snippets; run affected tests; update KB/DESIGN/ROADMAP.
- Plan: (1) read training docs (`KB.md` §4.6, `src/training.rs`, `tests/training_test.rs`) to learn format rules — 2-message lines, `< think> ... < /think>` assistant format, snippets auto-extracted from rfs-tagged code blocks and classified by parser+validator; (2) inventory all 10 categories (~209 examples) and scan every line for stale references; (3) fix stale content, add missing coverage for recent work areas; (4) regenerate snippets + aggregate; (5) run affected tests; (6) update docs.

### Report

- Findings: `project_tooling` held a stale "current status" example describing Phase 0 ("no validator/codegen/MC yet", "30 valid examples"); zero training coverage of the Phase 1 frontend CLI (`rfs parse`/`rfs validate`) and of the codegen plugin system; `codegen_rust` line 4 described an idealized generated shape (e.g. `pub struct UnexpectedEvent`) that does not match actual output; all other categories (design, syntax_semantics, properties_temporal_logic, model_checking*, examples_explain, tla_adaptation) are language-level and remained accurate.
- `training/project_tooling/rfs_training.jsonl` 2→3 lines: rewrote status example to current state (parser/IR/validator/frontend CLI/codegen plugin system/training pipeline with correct paths and commands); added training-pipeline maintenance example (`cargo extract-training-snippets` → `cargo build-training` → tests).
- New category `training/frontend_cli/rfs_training.jsonl` (5 examples): `rfs parse` file-handling rules (extensions, target-dir structure, collision guard), `rfs validate` error reporting (stderr vs `<name>.rfs-validation.err`), RFS↔JSON round-trip with working example module, frontend/code split (`src/frontend.rs` logic vs `src/main.rs` argument parsing) and testability rationale, CLI exit-code conventions across parse/validate/codegen.
- `training/codegen_rust/rfs_training.jsonl` 11→15 lines: rewrote generated-code-shape example to match real `rfs codegen rust production` output (flat Input/Output enums, Trace runtime, AssumptionHandler, StepError); added examples for the backend plugin system (`Backend`, `BACKENDS`, dispatch), `rfs codegen` CLI usage, shared-vs-Rust-specific placement rule, and a C-backend addition checklist.
- Regenerated artifacts: snippets 120 valid / 125 invalid (+2 new valid: `ping_pong`, `simple_fsm`); `all_rfs_training.jsonl` 209→219 examples.
- Verification: `cargo test` 95/95 pass (training tests validate every line, all snippets, aggregate freshness); `cargo test-generated-code` 486/486 pass (was 480; +2 new valid snippet modules × 3 generated tests).
- Docs: `KB.md` §1 state table (plugin-system codegen row, 95-test breakdown, 11 categories/219 examples) and state diagram, §4.6 category tree + coverage notes, §5.2 command table (+`rfs codegen` rows) and expected results (95 tests, 120/125 snippets, 219 examples, 486 generated tests); `ROADMAP.md` Phase 1 status bullet on the training-set actualization; added `codegen = "run -- codegen"` cargo alias to `.cargo/config.toml` (verified working).

---

## 2026-10-14 22:41 — Added training.py: SFT+GRPO training of Qwen3.8-27B on all_rfs_training.jsonl

### Interpretation & plan

- Requested: a Python script `training.py` that trains qwen3.8-27B with Unsloth using SFT and GRPO on `all_rfs_training.jsonl`, converting samples as needed; do NOT run it (GPU busy) — only syntax check, lint, and thorough logic review.
- Plan: read the official Qwen3.8 training docs + both Kaggle notebooks (Conversational SFT, GRPO); pin down exact dataset formats per stage; write the script with lazy GPU imports so pure logic is verifiable without hardware; verify via py_compile + ruff + `--dry-run`.

### Report

- Read: unsloth.ai/docs/models/qwen3.8/train plus both notebooks (saved under `.ref_docs/` for future sessions).
- Formats pinned: SFT = `text` column rendered with `apply_chat_template(tokenize=False, add_generation_prompt=False)` + `train_on_responses_only`; GRPO = `prompt` (user turn) + extra reward columns, `GRPOConfig(loss_type="grpo", importance_sampling_level="sequence")`, `FastModel(..., text_only=True, fast_inference=False)`.
- `training.py` (554 lines): stages `sft|grpo|both`; JSONL validation mirroring the Rust validator; conversion to SFT `text` and GRPO prompt/reference (rows over token budget dropped); deterministic CRC32 train/eval split; three pure-Python rewards — think_channel (`< think>` structure), rfs_structure (brace balance + module grammar), reference_similarity (word-bigram Jaccard vs gold); SFT→GRPO chaining via the saved LoRA adapter dir (`--grpo-base auto`); adapters saved, optional 16-bit merge; `--dry-run` validates data + reward self-test without GPU.
- Verification: `python -m py_compile training.py` OK; ruff (E,W,F,I,N,UP,B,SIM @ line-length 100) clean; `python training.py --dry-run`: 219 rows loaded/validated, split 198/21, reward self-test OK. Not run on GPU per request.
- Docs: `KB.md` §5.2 command table (+training.py row).

---

## 2026-10-15 10:22 — Phase 2 preparation: removed legacy/obsolete code, aligned docs and training sets

### Interpretation & plan

- Requested: prepare the repo for Phase 2 (core DSL + production codegen) by (1) removing all legacy/obsolete functionality from source, (2) aligning documentation with actual source, (3) removing training items tied to legacy code, (4) running tests and fixing errors, (5) logging this in TASKS.md.
- Plan: consult KB/ROADMAP/DESIGN for legacy markers; identify removal candidates (Phase 0 positional CLI mode, `validate_rfs` + `rfs_codegen` bins, pre-plugin monolithic codegen API); migrate remaining callers to the backend registry; update docs/training; full regression.

### Report

- Removed from source: `legacy_main` Phase 0 parser CLI mode in `src/main.rs` (usage text updated; unknown subcommand now prints usage + exit 1); deleted bins `src/bin/validate_rfs.rs` and `src/bin/rfs_codegen.rs` (superseded by `rfs validate` and `rfs codegen rust <cmd>`); removed their `.cargo/config.toml` aliases (`validate-rfs`, `rfs-codegen`); deleted the pre-plugin API section of `src/codegen.rs` (`CodegenOptions`, `CodegenResult`, `generate_module_code`, `generate_model_checker_code`, re-exports).
- Migrated callers to backend functions: `rust_test::generate_aggregate_test_crate` now takes `&BackendOptions` and calls `rust_fsm::generate_with_tests(..., true)` directly; `src/bin/generate_code_tests.rs`, `src/bin/test_generated_code.rs`, and `tests/codegen_test.rs` updated accordingly.
- Fixed a pre-existing bug found during verification: `generate_code_tests.rs` collected the whole `training/` tree (including `snippets/invalid`) and aborted on the first unparseable file; it now collects `examples/valid` + each category's `snippets/valid` only (mirrors `test_generated_code.rs`).
- Training sets: scanned all 11 categories for legacy references; only `training/project_tooling/rfs_training.jsonl` line 2 cited the removed positional CLI (`cargo run -- examples/valid`); rewritten to `cargo run -- validate --input rfs examples/valid`. Regenerated `all_rfs_training.jsonl` (219 examples) via `cargo build-training`; snippets unchanged (no snippet-bearing lines modified).
- Docs aligned: `KB.md` (state table CLI row, §4.3 API note, §4.5 CLI behavior = three subcommands without legacy section, §5.2 command table, expected results, §9 debug tips + historical-plan banner, §10 quick-start, §13.1/13.2/13.4/13.5 commands); `DESIGN.md` §14 implemented-CLI block; `CODEGEN.md` file-table row + §7 rewritten as "Pre-plugin API (removed)"; `ROADMAP.md` Phase 0 status bullets updated + new Phase 2-preparation cleanup bullet.
- Verification: `cargo build` clean; `cargo test` 95/95 pass (31 lib, 41 frontend, 12 codegen, 3 examples, 4 training, 4 comparison); `cargo generate-code-tests` now succeeds (162 modules into `target/rfs_codegen_tests`); `cargo test-generated-code` 486/486 pass; `cargo clippy --all-targets` no errors (2 pre-existing warnings: `from_str` naming in lib, `&PathBuf` arg in codegen_test — untouched, predating this task); `cargo fmt` applied; CLI smoke: `codegen rust production`, `validate --input rfs examples/valid` (exit 0), `codegen --list`, unknown-subcommand exit 1 all verified.

---

## 2026-10-15 11:05 — Documentation refactor: created TOOLCHAIN.md, BUILD.md, START.md; slimmed KB/DESIGN/ROADMAP

### Interpretation & plan

- Requested: restructure docs so a future LLM assistant (1) immediately finds the Rust toolchain without searching, (2) has one place for all build/run/test instructions, (3) gets a token-economical session entry point with lazy on-demand reading of the rest.
- Plan: inventory all toolchain/build content in KB/DESIGN/ROADMAP; pin down actual toolchain facts by probing the environment; write TOOLCHAIN.md + BUILD.md; replace moved content in old docs with pointers; write START.md (system prompt + doc map + section maps + task->read table); verify cross-references.

### Report

- New `TOOLCHAIN.md` (~70 lines): pinned toolchain facts - canonical project-local toolchain at `/workspace/host/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin` (cargo/rustc 1.99.0), rustup shim dir `/workspace/host/.cargo/bin` (works only with RUSTUP_HOME/CARGO_HOME exports; fails "no default is configured" without them), identical toolchain under `$HOME/.rustup` (session-dependent), system `/usr/local/bin/cargo` 1.57.0 must not be used (Cargo.lock v4); activation block, reinstall procedure (incl. rustup-init env-var caveat), observed corner cases.
- New `BUILD.md` (~90 lines): main build files table (Cargo.toml/.cargo/config.toml/Cargo.lock/src/tests/target), toolchain activation + build/test, test-suite table with expected current results (95 main / 486 generated / 38+6 examples / 219 training), full command+alias table (parse/validate/codegen CLI, generate-code-tests, test-generated-code, extract-training-snippets, build-training, test-training, training.py GPU row), post-change verification checklist + known-good baseline date.
- New `START.md` (~165 lines): system-prompt section (role, project root, core principle, 6-rule token-economy operating protocol incl. "never read whole large docs - grep section headers then sed ranges", TASKS.md first-paragraph-only rule), project state snapshot, documentation map table (file -> content -> read-when -> how-to-read with line-budget guidance), one-level source tree, toolchain essentials block, KB/DESIGN/ROADMAP section maps for targeted reads, task->read/do/verify lazy-loading table (8 task types), condensed workflow rules + syntax-change and IR-change checklists.
- `KB.md`: section 5 body replaced by pointer stub to BUILD.md/TOOLCHAIN.md/START.md; section 6.1 toolchain items moved out (generic cwd/terminal tips kept); section 10 quick-start replaced by START.md pointer (its two workflow checklists migrated into START.md section 8); obsolete section 12 "Suggested first tasks" (recommended implementing the already-existing validator) replaced by ROADMAP Phase 2 pointer; section 13.5 now points to BUILD.md section 4; section 1 state table gained rows for START/BUILD/TOOLCHAIN and KB re-labeled "Maintained".
- `DESIGN.md` section 14: "Implemented so far" bash block replaced by prose + pointers to BUILD.md / KB 4.5 (planned-commands design list kept). `ROADMAP.md`: intro gained one-line pointers to BUILD.md/TOOLCHAIN.md/START.md (phase exit criteria left as acceptance statements, not how-to).
- Verification: no RUSTUP_HOME/CARGO_HOME or command tables remain in KB/DESIGN/ROADMAP/CODEGEN (grep clean); all three new files cross-referenced from the old docs; KB section numbering intact (1-13, no duplicates); no source code touched, so no test run needed.

---

## 2026-10-15 16:41 — Curated .gitignore and made the initial git commit

### Interpretation & plan

- Requested: add all unnecessary files/folders (temp, build, etc.) to `.gitignore`, then make a git commit of the current RFS project state.
- Plan: inventory untracked content; decide track-vs-ignore per item (build artifacts/caches ignored, everything else tracked); note that the repo had zero commits, so this is the initial commit; set a repo-local git identity (none configured).

### Report

- Rewrote `.gitignore`: ignores `/target` (2.3 GB build artifacts), `__pycache__/` + `*.pyc`, `.ruff_cache/`, editor/OS noise (`*~`, `.DS_Store`).
- Deliberate change: removed the previous `Cargo.lock` ignore entry - this is an application crate (CLI + 4 bins) and the lock file (version 4) is required for reproducible builds; it is now tracked.
- Tracked as project content (not ignored): `.cargo/config.toml` (cargo aliases used throughout docs), `.ref_docs/` (Unsloth Qwen3.8 training references backing `training.py`), all docs, `src/` (23 files), `tests/` (5), `examples/` (44: 38 valid + 6 invalid), `training/` (256 files: 11 categories + snippets), `all_rfs_training.jsonl`, `training.py`.
- Set repo-local identity `RFS Development <rfs-dev@localhost>` (no global/repo identity existed).
- Commit: `65a7749` "Initial commit: RFS Phase 1 complete + Phase 2 preparation" - 345 files; worktree clean afterwards (`git status` empty); verified ignored set is exactly `.ruff_cache/`, `__pycache__/`, `target/`.
