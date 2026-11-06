# Phase 11, Task 6 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without re-deriving scope.

## Goal
Move the CLI surface root (log, frontend, ext_tool) and the four command modules
(test, gen_spec, mc_ref, info) into `src/cli/` + `src/cli/commands/`, updating
main.rs's dispatch.

## Context
- Renames per proposal rule 2: test_cmd.rs -> cli/commands/test.rs,
  gen_spec_cmd.rs -> cli/commands/gen_spec.rs, mc_ref_cmd.rs ->
  cli/commands/mc_ref.rs (the directory carries the "command" context, so the
  _cmd suffix is dropped); info.rs -> cli/commands/info.rs (no rename).
- cli_log.rs -> cli/log.rs (renamed - the level-filtered stderr logger shared by
  all commands; zero external import sites, internal churn only).
- frontend.rs / ext_tool.rs move unrenamed: frontend is the shared front half of
  ALL eight commands (not one command); ext_tool is the external-tool resolution
  leaf (61 incoming edges - script the rewrite).
- main.rs is the only kept-root file edited in this task (dispatch use-paths).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 1.-5. | all earlier moves | every import line in these eight files is already final |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 7. | diagram + info_format clusters | nest under cli/commands/ and cli/ respectively |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/cli_log.rs` | remove (moved) | git mv to src/cli/log.rs |
| `src/frontend.rs`, `src/ext_tool.rs` | remove (moved) | git mv to src/cli/ (unrenamed) |
| `src/test_cmd.rs`, `src/gen_spec_cmd.rs`, `src/mc_ref_cmd.rs`, `src/info.rs` | remove (moved) | git mv to src/cli/commands/ per mapping |
| `src/cli/mod.rs`, `src/cli/commands/mod.rs` | new | module roots |
| `src/main.rs` | change | dispatch use-path updates |
| all import sites of the eight modules | change | scripted use-path rewrite |

## Planned actions

### Move the eight files + create both module roots
git mv per mapping; write src/cli/mod.rs (log, frontend, ext_tool, commands) and
src/cli/commands/mod.rs (test, gen_spec, mc_ref, info - diagram + info_format
join later in task 7); script every use-path rewrite incl. main.rs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli_log.rs` | remove | git mv to src/cli/log.rs |
| `src/frontend.rs`, `src/ext_tool.rs` | remove | git mv to src/cli/ |
| `src/test_cmd.rs`, `src/gen_spec_cmd.rs`, `src/mc_ref_cmd.rs`, `src/info.rs` | remove | git mv to src/cli/commands/ |
| `src/cli/mod.rs`, `src/cli/commands/mod.rs` | new | module roots (task 7 extends commands/mod.rs) |
| `src/main.rs` + all other import sites | change | scripted rewrite, one pass |

### Gate
cargo build && cargo test green.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

## Task deliverables
- `src/cli/` with log/frontend/ext_tool + `commands/` with the four modules;
  main.rs dispatch updated.

## Task exit criteria
- [ ] cargo build && cargo test green (535 baseline).
- [ ] No file references the old paths for the eight modules (grep-verified).
- [ ] File contents byte-identical to before (moves + use-line edits only).

## Non-goals
- No CLI behavior change; diagram/info_format clusters not touched (task 7).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Root-level docs included in the re-sync (retroactive sweep) | Tasks 1-5 updated only `docs/dev/` + `docs/llm/` (+ `docs/user/`) and missed the root-level `*.md` files by mistake (user-confirmed). This task therefore also rewrote every stale source-path reference in `KB.md`, `DESIGN.md`, `BUILD.md`, `CODEGEN.md` and `START.md` - both the seven task-6 paths AND the tasks 1-5 paths those files still carried (`src/parser.rs`, `src/ir.rs`, `src/json_rw.rs`, `src/backends/*`, ...). `TASKS.md` (append-only history) and `SRC_STRUCTURE.md` / `TEST_STRUCTURE.md` (tasks 11-12 actualize them wholesale) are deliberately untouched. |
| Pre-existing doc count bugs fixed | `docs/dev/CODE_MAP.md`'s "CLI commands (7)" header listed only 6 bullets; `docs/llm/TASK_MAP.md`'s "Coverage (all 71 manifest files)" header was stale (85 entries before this task). Both corrected while restructuring those files (now: CLI surface section with 9 bullets; coverage header 87). |
| Unused-import trim in `src/codegen/backends/rust/mod.rs` | Task 5's ident extraction left `rust_binding_ident` imported but unused there (only `fsm.rs` uses it) - a new clippy warning line against the documented baseline (46/35), invisible to the per-task build+test gate. Removed from the import list; output-neutral, restores the baseline. |
| `cargo fmt` normalization of accumulated path-length drift | Tasks 1-5's longer module paths pushed several lines past the 100-column limit and changed import sort order in ~20 files (e.g. `frontend.rs`'s three long `json_rw` lines, `rust/mc.rs`'s FUZZ_RUNTIME replace-chain). `cargo fmt` applied once in this commit - formatting-only, no semantic change; full suite re-run green afterwards. |
| Environment fix: chrome-headless-shell installed | `diagram_render_parity_test::parity_external_mmd_pinned_subset` failed on the committed tree (pre-existing, not caused by this task): `mmdc` 12.0.0 is present but its puppeteer 25 browser cache was empty. Ran mmdc's bundled `puppeteer/install.mjs`, which downloaded chrome + chrome-headless-shell 156.0.8078.47 into the sandbox puppeteer cache; the suite then passed 13/13. |
