# Phase 17: Documentation update for users and developers + training-set sync

## Goal
Bring every documentation surface - the external user set (`docs/user/`), the
developer set (`docs/dev/`, `docs/llm/`, `KB.md`), the CLI's self-documentation
embedded in the binary (`src/cli/args.rs::usage()`), and the SFT training set -
into alignment with the toolchain as it stands after Phases 13-16, AND make RFS
usage exhaustively documented for users and developers: a new `docs/user/
USAGE.md` reference covering every CLI command set with detailed pattern tables
(inputs, options, generated-or-reused artifacts, errors), a substantially
expanded `crates/examples/` set (positive AND negative) covering almost all usage
scenarios with per-example READMEs, and a training set updated to match. Two
drift sources dominate the staleness fix: Phase 16 unified `rfs mc-ref` over ALL
three model-checking modes (exact/fuzz/smt-bounded + `--nondet`), but the user
docs, developer docs, LLM set, KB.md, the global usage text and three training
entries still describe it as exact-only; Phase 13's CLI plugin refactor left
stale prose in the developer docs describing the pre-refactor dispatch
architecture, and stale pre-Phase 11 module names linger in the LLM set.

## Scope
1. User set (`docs/user/`): `MODEL_CHECKING.md` section 2 rewritten for mode
   dispatch (per-mode flags, per-mode exit codes, `--nondet` now applicable,
   not-applicable list reduced to `--threads`/`--fast`, parity statement covering
   all three modes); section 1's mode table + exit-code paragraph corrected
   (mc-ref available in all three modes; fuzz exits 0/1 with no 2).
2. Developer set (`docs/dev/cli_commands.md`): Overview + Dispatch prose updated
   to the plugin-registry architecture (`main.rs` is a pure dispatcher over
   `ALL_COMMANDS`); the Library API surface section updated to the current module
   layout; the mc-ref section updated for mode dispatch + per-mode exit codes.
3. LLM set: `TASK_MAP.md` mc-ref CLI row (mode entry points), `CHANGE_IMPACT.md`
   stale `gen_spec_cmd` name, `TEST_STRUCTURE.md` stale command-file names,
   `cli_commands.md` dispatch + `exit_code_for` descriptions.
4. `KB.md` section 4.2: rows added for `src/mc/ref_mc_exact.rs`,
   `ref_mc_fuzz.rs`, `ref_mc_smt.rs`; the `ref_mc.rs` and
   `cli/commands/mc_ref.rs` rows updated to the Phase 16 state.
5. Source-embedded documentation: `src/cli/args.rs::usage()` - the mc-ref usage
   line + help paragraph updated to the three-mode CLI (matching `rfs mc-ref`'
   own usage text); any test pinning affected substrings migrated.
6. Training set: the three stale entries rewritten for the unified mc-ref
   (`frontend_cli` #11, `frontend_cli` #14, `model_checking` #29); new entries
   added where mc-ref's fuzz/smt-bounded modes have no coverage; snippets +
   aggregate regenerated (`cargo extract-training-snippets && cargo
   build-training`); `cargo test-training` green.
7. Small: a pointer in `docs/user/INTEGRATION.md` to Nix as an installation path
   (`nix build` per BUILD.md section 8) - confirm at planning; and the new
   mmdc/puppeteer environment finding recorded in `TOOLCHAIN.md` section 6.4
   (mmdc present is NOT mmdc working - the chrome-headless-shell browser binary
   must be installed into a persistent `PUPPETEER_CACHE_DIR`; verified
   2026-11-19).
8. NEW user doc `docs/user/USAGE.md`: the exhaustive CLI usage reference - a
   dedicated external-tool-resolution section (the full `RFS_*` env-var table
   with detailed descriptions and the resolution priority) plus one section per
   command set (`rfs parse`, `rfs validate`, `rfs info` + `info format`,
   `rfs codegen rust|c`, `rfs test`, `rfs diagram`, `rfs mc-ref`,
   `rfs gen-spec`), each carrying a pattern table (CLI pattern | options with
   detailed description | generated files / output incl. the take-or-emit
   artifact semantics | possible errors + exit codes) followed by detailed notes
   (artifact naming resolution, counterexample-trace walkthroughs, error fixes).
   Layout proposal: section below.
9. NEW example projects under `crates/examples/` covering almost all usage
   scenarios - positive (MC walkthrough with CE traces, no_std production,
   production property monitors, callback/minimal adapters, JSON-schema CI
   validation, gen-spec mutation loop, external-tool resolution with explicit
   tool-path passing) and negative (deadlock MC failure, property-violation MC
   failure, invalid-spec diagnosis), each with a detailed `README.md` per the
   template below; the five existing example READMEs are upgraded to the same
   template, and `c_sync_handler`'s Makefile gains an explicit gcc/clang choice
   (`make CC=clang`) documented alongside the `RFS_GCC`/`RFS_CLANG` equivalents.
   List + coverage matrix: sections below.
10. `docs/user/README.md`: reference to `USAGE.md` in the "Which doc for which
    question?" table (and the toolchain-at-a-glance diagram where it fits).
11. Training set: beyond the three stale rewrites of item 6, NEW entries covering
    the USAGE.md CLI-pattern semantics (artifact take-or-emit, CE-trace reading,
    error diagnosis) and the new example projects; snippets + aggregate
    regenerated.

## Staleness inventory (verified 2026-11-19 during preparation)

| Location | What is stale / missing | Drift source |
|---|---|---|
| `docs/user/MODEL_CHECKING.md` section 2 | mc-ref described as exact-only; `--seed`/`--violate`/`--nondet` listed "NOT applicable"; "use a generated binary when you need fuzz mode" | Phase 16 |
| `docs/user/MODEL_CHECKING.md` section 1 | mode table lists mc-ref only under Exact; "All three share exit codes 0/1/2" is wrong for fuzz (0/1, no 2) and smt-bounded (0 pass-within-bound / 1 fail / 2 unknown) | Phase 16 (+ pre-existing inaccuracy) |
| `docs/dev/cli_commands.md` Overview + Dispatch | "the first six are parsed in `src/main.rs` itself", cmd_* wrappers in main.rs - the pre-Phase 13 architecture (main.rs is now a 30-line pure dispatcher) | Phase 13 |
| `docs/dev/cli_commands.md` Library API surface | lists pre-Phase 11 flat module names (`frontend`, `gen_spec_cmd`, `mc_ref_cmd`, `test_cmd`, ...) instead of the current `cli`/`codegen`/`json`/`lang`/`mc`/`training`/`utils` layout + re-exports | Phases 11/13 |
| `docs/dev/cli_commands.md` mc-ref section | `McRefOptions { max_states, timeout_sec, assumes, ce_format }` exact-only; fuzz flags documented as NOT applicable | Phase 16 |
| `docs/llm/TASK_MAP.md` row 31 | mc-ref CLI row names only `src/mc/ref_mc.rs::run` as the engine - no mode entry points (`ref_mc_exact/fuzz/smt`) | Phase 16 |
| `docs/llm/CHANGE_IMPACT.md` row 35 | stale consumer name `gen_spec_cmd` (file is `src/cli/commands/gen_spec.rs`) | Phase 11 |
| `docs/llm/TEST_STRUCTURE.md` rows 1196-1197 | stale names "the gen_spec_cmd inline tests" / "the mc_ref_cmd inline tests" | Phase 11 |
| `docs/llm/cli_commands.md` rows 33, 39 | dispatch described as "mc-ref/gen-spec delegate to their run_cli shells" (pre-Phase 13 - now ALL commands do); `exit_code_for` described as exact-only (0/1/2) | Phases 13/16 |
| `KB.md` section 4.2 | no rows for `ref_mc_exact.rs` / `ref_mc_fuzz.rs` / `ref_mc_smt.rs`; the `ref_mc.rs` row still describes the monolithic engine; the `cli/commands/mc_ref.rs` row still lists exact-only flags + "exit codes 0/1/2 mirrored" | Phase 16 |
| `src/cli/args.rs::usage()` | mc-ref usage line + help paragraph are exact-only (list `--seed`/`--violate`/`--nondet` as not applicable) while `rfs mc-ref`'s own usage shows the three-mode CLI | Phase 16 |
| `training/frontend_cli/rfs_training.jsonl` #11, #14; `training/model_checking/rfs_training.jsonl` #29 | mc-ref exact-only claims ("no fuzz mode at all", "the only way to get fuzz mode is a generated binary") | Phase 16 |
| `TOOLCHAIN.md` section 6.4 | missing the mmdc/puppeteer finding: the mermaid-cli npm package does NOT bundle its browser - `chrome-headless-shell` must be installed separately (Node 18 here cannot run the latest npx-fetched puppeteer; use the version mermaid-cli bundles, 25.16.0) into a persistent `PUPPETEER_CACHE_DIR`, and root needs the `--no-sandbox` puppeteer config rfs already passes | new finding (2026-11-19) |

## Proposed USAGE.md layout

File: `docs/user/USAGE.md`. It is machine-checked by `tests/docs_md_test.rs`
like the rest of `docs/user/` (the gate collects every repo `.md`; mermaid must
parse, cross-links must resolve). Structure:

```markdown
# RFS CLI Usage Reference

<2-3 sentence intro: what this file is and how it complements MODEL_CHECKING.md
(mode semantics + report reading), INTEGRATION.md (build.rs/adapters/Makefiles)
and EXAMPLES.md (spec walkthroughs); the one exit-code convention - 0 success,
1 any error for CLI commands, the 0/1/2 status taxonomy for MC binaries.>

## 1. Command map
| Command | One-line purpose | Section |
|---|---|---|
| `rfs parse` | format conversion rfs<->json through the IR | 3 |
| `rfs validate` | parse + semantic validation, text or --json diagnostics | 4 |
| `rfs info` / `rfs info format` | static introspection; self-describing format docs | 5 |
| `rfs codegen rust\|c` | production/test/mc artifact generation (both backends) | 6 |
| `rfs test` | generate + compile + run unit/fuzz suites | 7 |
| `rfs diagram` | dot/puml/mmd/table/md sources; in-process SVG by default | 8 |
| `rfs mc-ref` | in-process reference MC, all three modes | 9 |
| `rfs gen-spec` | random / base-mutated spec generation | 10 |

## 2. External tool resolution - the RFS_* environment contract
<the one precedence model, stated once: CLI flag > RFS_* env var > standard
Unix var > auto-discovery (bare name on PATH, pkg-config for z3); --print-script
prints the resolved invocation so any run can be inspected without executing>

| Tool | RFS_* variable (+ `*_ARGS`) | Standard var | CLI override flag(s) | Auto-discovery fallback | Used by |
|---|---|---|---|---|---|
| C compiler (gcc) | `RFS_GCC` (+`RFS_GCC_ARGS`) | `CC` | `--gcc PATH`, `--gcc-args "..."` | bare `gcc` on PATH | codegen c, test (c), example Makefiles |
| Rust compiler | `RFS_RUSTC` (+`RFS_RUSTC_ARGS`) | `RUSTC` | `--rustc PATH`, `--rustc-args "..."` | bare `rustc` on PATH | codegen rust, test (rust) |
| clang (libFuzzer host) | `RFS_CLANG` (+`RFS_CLANG_ARGS`) | *(none - deliberate)* | `--clang PATH`, `--clang-args "..."` | bare `clang` on PATH | test --suite fuzz (c) |
| cargo (fuzz scaffold) | `RFS_CARGO_FUZZ` (+`_ARGS`) | `CARGO` | `--cargo-fuzz PATH`, `--cargo-fuzz-args "..."` | bare `cargo` on PATH | test --suite fuzz (rust) |
| graphviz dot | `RFS_DOT` (+`RFS_DOT_ARGS`) | *(none)* | `--dot PATH`, `--dot-args "..."` | bare `dot` on PATH | diagram --renderer external |
| PlantUML | `RFS_PLANTUML` (+`RFS_PLANTUML_ARGS`) | *(none)* | `--plantuml PATH`, `--plantuml-args "..."` | bare `plantuml` on PATH | diagram --renderer external |
| Mermaid CLI (mmdc) | `RFS_MERMAID` (+`RFS_MERMAID_ARGS`) | *(none)* | `--mermaid PATH`, `--mermaid-args "..."` | bare `mmdc` on PATH | diagram --renderer external |
| make (C example projects) | `RFS_MAKE` | *(none)* | - | bare `make` on PATH | C example builds |
| z3 (SMT, feature-gated) | `RFS_Z3_DIR` | `Z3_SYS_Z3_DIR` | - | `pkg-config z3`, then the project-local install | mc-ref --mode smt-bounded, generated SMT binaries |

<notes: clang deliberately has NO standard tier (a CC=clang system must not
silently switch the fuzzer compiler); env args come first, CLI args appended;
special vars outside the table - `RFS_BIN` (the rfs-build build.rs helper's
binary override), `RFS_FUZZ_FLAGS` (extra libFuzzer runtime flags),
`RFS_MERMAID_PUPPETEER_ARGS` (the mmdc puppeteer config); a missing tool for a
SELECTED operation is an error with a diagnostic - no graceful degradation>

## 3. rfs parse - format conversion
<purpose + when to use, 1-3 lines>

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs parse --input rfs --output json spec.rfs` | ... | `spec.json` next to the input (or under `--target-dir DIR`, structure preserved) | parse/validation error -> stderr + exit 1; output would overwrite the input -> refused, exit 1 |
...

### Naming and placement rules (detailed notes)
- ...

### Error details and fixes
- ...
```

Sections 4-10 follow the same shape. The codegen section (6) states the artifact
take-or-emit semantics EXPLICITLY in the table rows AND in a dedicated naming-
rules subsection (BASE derivation, per-artifact flag override, `--fsm-file`
taken-if-present / emitted-if-missing, partial-pair usage error, stale-reuse
warning). The mc-ref section (9) carries counterexample-trace walkthroughs: real
text + JSON CE documents from this repository (a deadlock lasso and a
property-violation trace), field by field. Section 11 is the example-projects
table (below).

Sample filled-in row (the artifact nuance called out in the phase request):

| CLI pattern | Options - detailed description | Generated files / output | Possible errors (exit codes) |
|---|---|---|---|
| `rfs codegen c mc spec.rfs --out BASE` | `--out BASE`: base name for the whole artifact set; without it BASE = the spec file's stem and files land in the CWD. `--fsm-file STEM` (or `--fsm-file-header H --fsm-file-source C`): names the FSM artifact the generated MC code references - an EXISTING pair is taken as-is (a partial pair is a usage error), a missing one is emitted new, so one invocation always yields a complete set; a taken fsm whose line-1 spec hash differs from the current spec prints a stale-reuse warning but is still referenced | `BASE-mc.h` + `BASE-mc.c` (always written fresh); `BASE-fsm.h` + `BASE-fsm.c` (taken or emitted). Compile: `gcc -Wall -c BASE-fsm.c BASE-mc.c && gcc -Wall *.o -o mc` | usage error (partial fsm pair, unknown flag) exit 1; parse/validation error exit 1; stale-reuse warning on stderr (not an error) |

## Proposed example README.md template

Every `crates/examples/<name>/README.md` follows this layout (the five existing
examples are upgraded to it):

```markdown
# <name> - <one-line title>

## What it demonstrates
- RFS functionality exercised (bullets, each linking the relevant docs/user page)
- Rationale: why this example exists (which usage scenario it covers)
- The automaton itself: what it is used for; whether it is drawn from a real
  protocol (named) or constructed for teaching; states/events/outputs/vars summary

## The machine
- mermaid stateDiagram of the FSM (always)
- mermaid diagram of the property/monitor structure when properties are involved
- transition table when it adds clarity

## Files layout
| File | Role |
|---|---|

## Build / run / test
Exact commands + prerequisites (toolchain activation, RFS_BIN, external tools).

## Expected results
- Pinned output excerpts the user should see
- What to pay attention to (invariants held, window bounds, dropped inputs, ...)
- Negative examples only: what the counterexample trace shows, why the spec
  fails, and the fix (the corrected spec or flag change + the re-run result)
```

## Proposed list of new example projects

| # | Title (dir under `crates/examples/`) | Description + rationale | Kind |
|---|---|---|---|
| 1 | `mc_walkthrough` | Generate MC binaries for BOTH backends on a passing spec, run exact mode, then show the counterexample machinery (text + JSON) on a failing variant - teaches reading a report end to end | plain dir (specs + run script) |
| 2 | `mc_deadlock_fail` (negative) | A valid spec that deadlocks at a terminal state (the spec-04 teaching case): the CE trace, what to look for, and the one-transition fix with re-run proof | plain dir |
| 3 | `mc_property_violation_fail` (negative) | A valid spec with an unsatisfiable property (from the pinned FAIL variants in `tests/common/fail_variants.rs`): step-by-step CE walkthrough + the fix | plain dir |
| 4 | `invalid_spec_diagnosis` (negative) | One of the 13 invalid specs: `rfs validate` text + `--json` diagnostics, what each error means, and the corrected spec that validates | plain dir |
| 5 | `no_std_fsm` | Production codegen with `--no-std` compiled in a no_std context; identical public API to the std build | cargo member |
| 6 | `monitors_production` | Production build with `--with-property-monitors`: the debug-gated monitor catches an injected violation, the release build is zero-cost (C leg: `-DRFS_PROPERTY_MONITORS`) | cargo member (+ C note) |
| 7 | `callback_minimal_adapters` | The two rfs-adapters not yet demonstrated: the callback driver and the no_std minimal wrapper | cargo member |
| 8 | `json_schema_ci` | `rfs info format <name> --output schema` for the five JSON document formats; a CI-style script validating REAL tool output against the schemas | plain dir (script) |
| 9 | `specgen_mutation_loop` | The rapid-prototyping loop as a runnable example: gen-spec random + mutation, validate, mc-ref, fix, repeat - with real transcripts | plain dir (script) |
| 10 | `tool_resolution` | Explicit external-tool path passing end to end: a run script that sets `RFS_GCC`/`RFS_RUSTC`/`RFS_CLANG` to explicit paths and shows the `--print-script` "# tools:" lines proving the precedence (flag > env var > standard var > PATH), plus a Makefile with a gcc/clang choice (`make CC=clang`) - the user sees exactly which compiler rfs resolved and used | plain dir (script + Makefile) |

The five existing projects (`sync_handler`, `async_handler`, `c_sync_handler`,
`fuzz_rust`, `fuzz_c`) stay; their READMEs are upgraded to the template, and
`c_sync_handler`'s Makefile gains the explicit gcc/clang choice (`CC ?= gcc`
already exists - it becomes a documented, tested option: `make CC=clang` builds
the same scenario warning-free under clang, mirroring the generated-C compiler-
agnosticism guarantee).

Scenario coverage matrix: pure-CLI workflows (one-shot `parse` / `validate` /
`info` / `diagram` / `test` / `mc-ref` / `gen-spec` invocations) are documented
as USAGE.md pattern tables - they need no build files, so no example project is
created for them; `crates/examples/` covers the integration scenarios that DO
require build configuration (build.rs, Makefiles, fuzz layouts, adapters).
New cargo-member examples join the workspace (`Cargo.toml` members list) and are
driven by `tests/example_projects_test.rs`; plain-dir examples get their own test
legs there (or a dedicated suite - planning decision).

## Deliverables
- `docs/user/MODEL_CHECKING.md` - changed (sections 1-2).
- `docs/user/INTEGRATION.md` - changed (one-line Nix installation pointer, if confirmed at planning).
- `docs/dev/cli_commands.md` - changed (Overview, Dispatch, Library API surface, mc-ref section).
- `docs/llm/TASK_MAP.md`, `docs/llm/CHANGE_IMPACT.md`, `docs/llm/TEST_STRUCTURE.md`, `docs/llm/cli_commands.md` - changed (the rows in the inventory).
- `KB.md` - changed (section 4.2 rows).
- `src/cli/args.rs` - changed (usage text only; no behaviour change).
- `training/frontend_cli/rfs_training.jsonl`, `training/model_checking/rfs_training.jsonl` (+ any category gaining new entries) - changed; `training/<cat>/snippets/` + `all_rfs_training.jsonl` regenerated.
- `TOOLCHAIN.md` - changed (section 6.4 gotcha).
- `docs/user/USAGE.md` - new (the CLI usage reference, sections per command set
  + the example-projects table).
- `crates/examples/<9 new dirs>/` - new (specs, sources, Makefiles/scripts,
  READMEs per the template).
- `crates/examples/{sync_handler,async_handler,c_sync_handler,fuzz_rust,fuzz_c}/README.md`
  - changed (template upgrade).
- `crates/examples/c_sync_handler/Makefile` - changed (documented gcc/clang
  choice: `make CC=clang`).
- `Cargo.toml` - changed (new workspace members for the cargo-member examples).
- `tests/example_projects_test.rs` - changed (drives the new example projects;
  negative examples pin their expected failures).
- `docs/user/README.md` - changed (USAGE.md reference).
- `docs/roadmap/phase_17/PHASE_17_STATUS.md` - phase completion report.

## Exit criteria
- [ ] No documentation surface (user set, developer set, LLM set, KB.md, the
      global usage text, training data) still describes `rfs mc-ref` as
      exact-only or lists `--seed`/`--violate`/`--nondet` as not applicable to it.
- [ ] `docs/user/MODEL_CHECKING.md` section 1's mode table + exit-code paragraph
      are accurate for all three modes (fuzz: 0 no-violation / 1 violation, no 2;
      smt-bounded: 0 pass-within-bound / 1 fail / 2 unknown; exact: 0/1/2).
- [ ] `docs/dev/cli_commands.md` Overview + Dispatch describe the plugin-registry
      architecture matching `src/main.rs` (pure dispatcher over `ALL_COMMANDS`);
      the Library API surface section matches `src/lib.rs`'s current module list
      and re-exports.
- [ ] No stale pre-Phase 11/13 module or file names remain in `docs/dev/*` or
      `docs/llm/*` (`frontend` as a module, `gen_spec_cmd`, `mc_ref_cmd`,
      `test_cmd`).
- [ ] `KB.md` section 4.2 carries rows for `ref_mc_exact.rs` / `ref_mc_fuzz.rs`
      / `ref_mc_smt.rs` and updated `ref_mc.rs` + `cli/commands/mc_ref.rs` rows.
- [ ] The global usage text (no-argument and unknown-subcommand output) shows the
      three-mode mc-ref signature consistent with `rfs mc-ref`'s own usage; any
      test pinning affected substrings is migrated, not weakened.
- [ ] Training entries rewritten for the unified mc-ref; no entry claims mc-ref
      lacks fuzz/smt-bounded/`--nondet`; snippets + aggregate regenerated;
      `cargo test-training` green.
- [ ] `TOOLCHAIN.md` section 6.4 records the mmdc/puppeteer browser-binary finding
      with the verified install recipe.
- [ ] Doc gates green: `cargo test --test docs_md_test` (user-set quality gate -
      now covering USAGE.md) and `cargo test --test dev_docs_test` (sync gate).
- [ ] `docs/user/USAGE.md` exists with one section per command set; every
      section carries the pattern table (pattern | options | generated files /
      output | errors + exit codes); the codegen rows state the take-or-emit
      artifact semantics explicitly (`--out BASE` derivation, `--fsm-file`
      taken-if-present / emitted-if-missing, partial-pair usage error,
      stale-reuse warning); the mc-ref section includes real CE-trace
      walkthroughs (text + JSON).
- [ ] `docs/user/USAGE.md` carries the external-tool-resolution section: the
      full `RFS_*` table (variable, `*_ARGS`, standard var, CLI override,
      auto-discovery fallback, used-by) with the precedence model stated once
      and the clang-no-standard-tier + z3 discovery-chain notes.
- [ ] At least one example demonstrates explicit tool-path passing: the
      `tool_resolution` example's script shows `--print-script` "# tools:" lines
      under explicit `RFS_GCC`/`RFS_RUSTC`/`RFS_CLANG` values, and
      `c_sync_handler` builds warning-free under BOTH `make` (gcc) and
      `make CC=clang`, pinned by its test leg.
- [ ] Every new example project builds/runs/tests green per its README; negative
      examples' expected failures are PINNED (the CE trace / diagnostic excerpt
      is what the test asserts, not just a non-zero exit); the five existing
      examples still pass.
- [ ] Every `crates/examples/*/README.md` follows the template (What it
      demonstrates / The machine with diagrams / Files layout / Build-run-test /
      Expected results).
- [ ] `docs/user/README.md` references USAGE.md.
- [ ] Training set: new entries cover the CLI-pattern semantics + at least the
      negative-example diagnosis workflow; snippets + aggregate regenerated;
      `cargo test-training` green.
- [ ] Full verification at or above the Phase 16 close-out baseline: `cargo
      test` >= 573 (new example-project tests add to it), `--workspace`,
      `--features smt`, generated-code 1683 - fmt clean, clippy at baseline.
- [ ] `TASKS.md` entry appended; `CURRENT_STATE.md` phase-17 row updated;
      `PHASE_17_STATUS.md` written per FORMATTING.md section 4.

## Non-goals
- No language-semantics, grammar, IR, codegen or MC behaviour change - this is a
  documentation + embedded-usage-text + example-projects + training-data phase
  (the `usage()` edit changes printed text only, never parsing or exit codes;
  the new examples exercise the EXISTING surface, they add no features).
- No restructuring of the existing `docs/user/` pages beyond the README
  reference and the MODEL_CHECKING.md fixes - USAGE.md is additive.
- No change to `rfs-dev`, the sync gates, or the structure-inventories' row
  format (no fn/type/file in `src/` is added, removed or renamed, so the
  inventories need no row changes; new example-crate sources are test-side and
  follow the existing example-project convention).
- No GPU training run - `training.py` is untouched; only the data it consumes
  changes.
- No Nix packaging work (Phase 15's non-goals carry over: no flake.nix).
