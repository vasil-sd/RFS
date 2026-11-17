# Phase 16, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope. The source-subdivision analysis with rationales lives in
> `PHASE_16.md` ("Source subdivision analysis") - read it first; the shared context
> block (baselines, structure map, conventions) lives in `PHASE_16_PLAN.md`
> ("Context").

## Goal
Split `src/mc/ref_mc.rs` (4971 lines) into the common core + entry points that stay
in `ref_mc.rs` and a new `src/mc/ref_mc_exact.rs` holding the exact engine, as a
PURE MOVE with zero behavior change. This is the safety net for the whole phase:
every later task (nondet, fuzz, smt) starts from a layout proven byte-safe by
unchanged canonical test counts.

## Context
- **What moves (measured 2026-11-17; line numbers drift - grep, don't trust
  ranges):** the exact-engine block `BNode`/`CfgKey`/`val_key`/`bnode_key`
  (~1991-2053), the `run()` BFS body (~2055-2465), `path_to`/`state_name`/
  `io_in`/`io_out`/`trace_steps` (~2467-2546), the liveness analysis
  `LivWitness`/`LeafSig`/`leaf_signals`/`skel_eval`/`liv_analyze`/`prod_delta`/
  `leaf_good`/`leaf_bits`/`liv_note` (~2548-3062), and
  `minimal_violation_labels`/`collect_min_labels` (~3416-3618). Their unit tests
  move with them (the test module starts ~4300; the exact-engine tests are the
  ones exercising BFS/liveness/minimal-labels - e.g. the `minimal_violation_labels`
  call sites at ~4642/4650/4704).
- **What stays in `ref_mc.rs`:** values/typing (~56-244), evaluator (~246-409),
  signals/inputs/outputs (~411-617), monitors (~619-1076), representative inputs
  (~1078-1283), effects `Effect`/`pattern_bindings`/`match_effect`/
  `input_matches_variant` (~1285-1497), liveness decomposition `Leaf`/`Skel`/
  `LivProp`/`decompose_top`/`decompose_properties` (~1499-1640), formula/expression
  printing (~1642-1882), config/result types `RefConfig`/`IoRec`/`StepRec`/
  `CeData`/`RefResult` (~1884-1989), rendering `render_*`/`to_json_report`
  (~3064-3414), counterexample verification (~3620-4296), and the independence
  audit test (stays in `ref_mc.rs`, extended - action 3).
- **Entry-point contract:** `rfs::mc::ref_mc::run(module, cfg)` must keep working
  unchanged for `src/cli/commands/mc_ref.rs` and `tests/ref_mc_diff_test.rs`. Move
  the engine's `run()` into `ref_mc_exact.rs` as `pub(crate) fn run(...)` and leave
  a one-line delegating `pub fn run(...)` in `ref_mc.rs` (D-P16-1: entry points
  stay in ref_mc.rs).
- **Visibility:** the moved code calls common-core items that are private today -
  `eval_expr`, `state_variants`, `compile_formula`, `contains_liveness`,
  `decompose_properties`, `mon_init`, `mon_step`, `mon_key`, `mon_bits`,
  `all_inputs`, `flatten`, `match_effect`, `ceil_log2`, and the types `Mon`/
  `MonSt`/`Sample`/`InputVal`/`OutputVal`/`Val`/`LivProp`. Widen exactly those to
  `pub(crate)`; nothing else. (Re-derive the list from the moved code's actual
  call sites - this is the measured starting set.)
- **Independence audit:** `independence_import_audit` (ref_mc.rs ~4956) currently
  self-reads only `ref_mc.rs`. Extend it to read EVERY file matching
  `src/mc/ref_mc_*.rs` and enforce the forbidden list (`mc::plan`, `monitor`,
  `codegen::fsm`, `printer`, `backends`) on all of them, with the documented
  per-file exceptions: `ref_mc_smt.rs` MAY import `crate::mc::plan` AND
  `crate::mc::smt_plan` (D-P16-3), `ref_mc_fuzz.rs` MAY import
  `crate::utils::rng`. Encode the exceptions now even though those files do not
  exist yet - the audit must be final-form after task 1.
- **Inventory + docs:** ~60 SRC_STRUCTURE.md rows for moved items change their
  file column to `src/mc/ref_mc_exact.rs` (the file has 139 ref_mc mentions in
  total - only the moved items' rows change); the `//! Docs:` pointer lines move
  with the code; the docs/dev MC-engine fragments (`dev:mc-engine-exact` at
  mc_engine.md ~122, `dev:mc-engine-liveness` at ~225) reference the moved items -
  update their file references. Same-commit rule (PROTOCOL.md section 5); finish
  with `rfs-dev inventory audit` EXACT + `rfs-dev sync-check` clean.
- **Verification bar:** full canonical sweep with UNCHANGED counts - `cargo test`
  557/0, `--workspace` 685/0, `--features smt` 559/0, generated-code 1683/1683,
  fmt clean, clippy at baseline (55 warning lines / 34 unique). Any count movement
  is a defect in the move, not a new test.

## Scope

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | --nondet mode | extends the moved BFS + liveness in ref_mc_exact.rs; needs the verified layout first |
| 3. | Seeded fuzz mode | new ref_mc_fuzz.rs built on the common core that task 1 isolates; the mode-dispatch entry point exists after the split |
| 4. | SMT-bounded mode | new ref_mc_smt.rs; the extended independence audit (task 1) already carries its import exceptions |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/mc/ref_mc.rs` | change | shrinks to common core + entry points; moved items removed; private fns/types used by the engine widened to pub(crate); delegating run(); audit extended |
| `src/mc/ref_mc_exact.rs` | new | exact engine: keying, BFS run(), path/trace, liveness analysis, minimal violation labels + their unit tests |
| `src/mc/mod.rs` | change | + `pub mod ref_mc_exact;`, doc comment updated (independence contract scoped per file) |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/llm/SRC_STRUCTURE.md` | change | moved items' rows: file column -> src/mc/ref_mc_exact.rs (~60 rows) |
| `docs/dev/mc_engine.md` | change | dev:mc-engine-exact / dev:mc-engine-liveness fragments: file references for the moved items |

## Planned actions

### 1. Move the exact engine into ref_mc_exact.rs
Cut the exact-engine block (Context bullet 1) verbatim into a new
`src/mc/ref_mc_exact.rs` with its own header comment (module role + independence
scope + the `//! Docs:` pointers of the moved fragments). Widen the common-core
items it calls to `pub(crate)` (exact list in Context - re-derive from actual call
sites). In `ref_mc.rs` replace the removed block with the delegating
`pub fn run(module, cfg) -> Result<RefResult, String> { ref_mc_exact::run(module, cfg) }`.
Move the exact-engine unit tests into a `#[cfg(test)] mod tests` in the new file;
common-core tests stay. Rationale: D-P16-1 - one mode per file; the move must be
textually verifiable (git diff shows only relocation + visibility keywords), which
is what makes "zero behavior change" provable by unchanged test counts rather than
by re-reading the code.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc_exact.rs` | new | the moved engine + tests; header documents that this file is exact-mode-only and imports nothing beyond std / lang::ir / mc_concepts + the common core |
| `src/mc/ref_mc.rs` | change (~1991-2546, ~2548-3062, ~3416-3618) | engine block removed; delegating run() added at the same position |
| `src/mc/ref_mc.rs` | change (visibility sites) | pub(crate) widening of the exact list from Context - no other visibility changes |

### 2. Register the module + update the cluster docs
Add `pub mod ref_mc_exact;` to `src/mc/mod.rs` and rewrite its doc comment: the
independence contract is now scoped per file (exact/fuzz: no intra-`mc/` imports
beyond the common core; smt: plan + smt_plan model only). Rationale: keeps the
cluster's self-description true for the audit and for future readers.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/mod.rs` | change | module declaration + doc comment |

### 3. Extend the independence import audit to all ref_mc_*.rs
Rewrite `independence_import_audit` to read every file in `src/mc/` whose name
starts with `ref_mc_` and ends with `.rs` (via `std::fs::read_dir` on the manifest
`src/mc` directory), assert the forbidden list on each file, and encode the
per-file exceptions (Context bullet 5). Keep the test in `ref_mc.rs`. Rationale:
the audit is the enforcement mechanism of D-P16-2; making it final-form now means
tasks 3-4 cannot accidentally import a forbidden module without a red test.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/mc/ref_mc.rs` | change (~4956-4971) | audit reads all ref_mc_*.rs with the per-file exception table (smt: mc::plan + smt_plan; fuzz: utils::rng) |

### 4. Sync the inventories and dev docs, then run the full verification sweep
Update SRC_STRUCTURE.md rows (file column for every moved item) and the
docs/dev/mc_engine.md fragment references in the SAME commit; move `//! Docs:`
pointer lines with the code. Then run: `rfs-dev inventory audit` (must be EXACT),
`rfs-dev sync-check` (clean), fmt, clippy, and the full canonical sweep (Context
verification bar). Rationale: PROTOCOL.md section 5 same-commit rule; the
unchanged-counts sweep is the proof that the move changed nothing.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/SRC_STRUCTURE.md` | change | ~60 rows' file column -> src/mc/ref_mc_exact.rs |
| `docs/dev/mc_engine.md` | change | fragment file references for the moved items |

## Task deliverables
- `src/mc/ref_mc_exact.rs` - the exact engine (keying, BFS run, path/trace,
  liveness analysis, minimal violation labels) + its unit tests.
- `src/mc/ref_mc.rs` shrunk to the common core + entry points, with a delegating
  `run()` and the extended independence audit.
- Updated `src/mc/mod.rs`, SRC_STRUCTURE.md rows, docs/dev/mc_engine.md references
  - all in one commit.
- A green full canonical sweep with byte-unchanged counts (557/685/559/1683), fmt
  clean, clippy at baseline, inventory EXACT, sync-check clean.

## Task exit criteria
- [ ] `src/mc/ref_mc_exact.rs` exists and contains the exact engine + its tests; `ref_mc.rs` no longer defines BNode/CfgKey/the BFS run body/liv_analyze/minimal_violation_labels (only the delegating run()).
- [ ] `rfs::mc::ref_mc::run`, `RefConfig`, `render_text`, `to_json_report` have unchanged signatures and behavior (CLI + diff test compile without changes).
- [ ] The independence audit reads all `src/mc/ref_mc_*.rs` files and passes, with the smt/fuzz exceptions encoded.
- [ ] SRC_STRUCTURE.md inventory EXACT (`rfs-dev inventory audit`) and `rfs-dev sync-check` clean, in the same commit as the code move.
- [ ] Full canonical sweep: `cargo test` 557/0, `--workspace` 685/0, `--features smt` 559/0, generated-code 1683/1683 - UNCHANGED; fmt clean; clippy at baseline (55 warning lines / 34 unique).

## Non-goals
- No behavior change of any kind - this task moves code, it does not touch algorithms, formats, or the CLI.
- No `--nondet`, no fuzz mode, no SMT mode (tasks 2-4).
- No restructuring of the common core's internal section order beyond removing the moved blocks.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- The move is mechanical but wide; budget time for the ~60 inventory rows and the
  visibility widening, not for design decisions.
- If clippy reports new warnings after the move (e.g. unused-import relocations),
  fix them as part of the move - the baseline must hold exactly.
- Re-check with `git diff --stat` that only the four expected source files changed
  before committing.
