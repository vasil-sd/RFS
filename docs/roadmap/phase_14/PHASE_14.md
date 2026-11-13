# Phase 14: CLI help correctness and separated codegen artifacts

## Goal
Make the `rfs` CLI self-describing without assuming a Rust toolchain - every help
message and all living documentation reference only the `rfs` binary (the tool is
usable on machines with no cargo/rustc at all, e.g. C-only projects) - and restructure
codegen output so that each artifact (FSM source, model checker, unit tests, fuzz
target) is generated as its own file that references the FSM code from a separate
file instead of duplicating it; for the C backend this means idiomatic
header/implementation separation.

```
spec.rfs -> IR -> rfs codegen <backend> <production|test|mc>
                 -> BASE-fsm.{h,c} | BASE-fsm.rs          (the FSM, exactly one copy)
                 -> BASE-mc.{h,c}    | BASE-mc.rs         (references the fsm file)
                 -> BASE-test.c/.rs  + BASE-fuzz-target.c/.rs (reference the fsm file)
```

## Scope
1. Help-text fix: the global `usage()` in `src/cli/args.rs` carries four
   `cargo run -- ...` example lines; every help surface (global usage, per-command
   `usage()` in the command plugins, `codegen`'s `common_help` / `backend_help`,
   backend `BackendCommandInfo` usage lines) is audited and rewritten so that ONLY
   the `rfs` binary is referenced - no cargo/rustc invocation examples anywhere.
2. Living-documentation sync for the same rule: `KB.md` (section 4.5 CLI behavior,
   ~20 example lines), `BUILD.md` (command tables + quick-start, ~14 lines),
   `README.md`, `START.md` (section 7 sanity line), and the user set
   `docs/user/{README,INTEGRATION,LANGUAGE,MODEL_CHECKING,PROPERTIES,EXAMPLES}.md`
   (~32 lines). Legitimate cargo usages stay untouched: workspace build/test
   commands, `cargo run -p <example-crate>`, and the cargo-fuzz pipeline.
3. Training-set update: 14 entries across `training/frontend_cli/` (10),
   `training/codegen_rust/` (3) and `training/project_tooling/` (1) teach the
   `cargo run -- <command>` pattern; they are rewritten to `rfs <command>`, then
   snippets + the aggregate set are regenerated per the training workflow.
4. New codegen output model: a per-artifact output-flag family
   (`--out-fsm-code`, `--out-fsm-header`, `--out-mc-code`, `--out-mc-header`,
   `--out-test-code`, `--out-fuzz-target-code`), the fsm-reference flags
   (`--fsm-file` / `--fsm-file-header` / `--fsm-file-source`, take-existing-or-
   emit semantics), the `--out BASE` shortcut that expands to backend-specific
   default names, and the spec-stem default naming when `--out` is absent (see
   "Proposed CLI" below). The
   `CodegenOutput` model is restructured from (single `code` string + unnamed
   `extra_files`) to named artifact slots; `parse_codegen_args` and the codegen
   help texts are updated accordingly.
5. C header/source separation: the C `production` command emits a header
   (types + public API) and an implementation file; the C `mc`, `test` and fuzz-
   target artifacts reference the FSM through the header include + object linking -
   no generated C ever does `#include "...fsm.c"` again, and no artifact carries a
   second copy of the FSM implementation.
6. Rust equivalent: single-file artifacts (no headers); the `mc`, `test` and fuzz-
   target artifacts reference the separately generated fsm file instead of inlining
   a duplicate of the FSM code.
7. Compiler-invocation changes: the `rfs test` C pipelines (unit suite: compile
   fsm + test sources to objects and link; fuzz suite: clang over both sources
   instead of one self-including file) and every test-suite leg that compiles
   generated C MC artifacts (`backend_parity_test`, `benchmark_mc_test`,
   `c_compiler_agnostic_test`, liveness/monitor suites as applicable).
8. Example fixes: `crates/examples/c_sync_handler` (handler.c includes ONLY the
   generated header; the Makefile generates explicit object files - arq_fsm.o and
   handler.o - and links them into the final executable) and
   `crates/examples/fuzz_c` (clang builds the fuzz target together with the
   separately generated fsm source).
9. Test updates: existing suites that pin the old single-file output or the old
   `--out` semantics (`codegen_test`, `backend_parity_test`,
   `example_projects_test`, the CLI-parsing tests in `tests/utils_test.rs`) are
   migrated; new coverage is added for the flag family, the `--out BASE`
   default-name expansion (both backends), the header/source split, and the
   cross-artifact reference correctness (mc/test/fuzz compile against the fsm
   file alone).
10. Documentation of the new CLI: `src/codegen/CODEGEN.md` +
    `src/codegen/backends/{C,RUST}.md` (the artifact model), the codegen rows in
    `KB.md` section 4.5 and `BUILD.md`, and the user-facing walkthroughs in
    `docs/user/{README,INTEGRATION,EXAMPLES,MODEL_CHECKING}.md`.

## Proposed CLI

```
rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json> [output options] [generation options]
```

Output options - each command accepts the subset relevant to its artifacts:

- `production`: `--out-fsm-code FILE` (C additionally `--out-fsm-header FILE`),
  or the shortcut `--out BASE`. Emits only the fsm artifact.
- `mc`: `--out-mc-code FILE` (C additionally `--out-mc-header FILE`) plus the
  fsm-reference flags below, or the shortcut `--out BASE`.
- `test`: `--out-test-code FILE`, `--out-fuzz-target-code FILE`, plus the
  fsm-reference flags below, or the shortcut `--out BASE`.

Fsm-reference flags (`mc` and `test` only) - name the FSM artifact the generated
code references; the artifact is TAKEN if already present on disk, otherwise
EMITTED new (one invocation yields a complete set either way - the user can give
the fsm a specific name without a separate generation step):

- Rust: `--fsm-file NAME[.rs]` (the extension is appended when omitted).
- C, form 1: `--fsm-file STEM` -> the pair `STEM.h` + `STEM.c`.
- C, form 2: `--fsm-file-header FILE.h --fsm-file-source FILE.c` (independent
  names, e.g. in different directories).

Naming resolution (the default policy - shell-prototyping friendly):

- BASE := the `--out` value when given, otherwise the SPEC FILE STEM (file name
  without extension, no directory component); artifacts are written to the CWD.
  So `rfs codegen c mc examples/valid/43_sliding_window.rfs` with no output flags
  writes `43_sliding_window-fsm.h/.c` + `43_sliding_window-mc.h/.c` right where
  you stand.
- An explicit per-artifact flag overrides the corresponding BASE-derived name for
  that artifact only.
- The fsm-reference flags override the BASE-derived fsm name (whether or not
  `--out` is present). The referenced fsm artifact - named by a fsm-reference
  flag OR derived from BASE - is TAKEN if already present on disk, otherwise
  EMITTED new. The command's own artifacts (mc/test/fuzz-target) are written
  fresh (overwritten) on every run.
- Relative names resolve against the CWD; `--out DIR/BASE` places the whole set
  under DIR/. Presence is checked at the final resolved path(s); for C form 1
  BOTH files must exist (a partial pair is a usage error - no silent overwrite).
- An existing fsm artifact whose line-1 header spec hash (D-P6-3) differs from
  the current spec's hash produces a stderr warning (stale reuse) but is still
  referenced.

Default names derived from BASE (identical whether BASE comes from `--out` or
from the spec-stem default policy):

| Command    | C backend                                        | Rust backend                              |
|------------|--------------------------------------------------|-------------------------------------------|
| production | `BASE-fsm.h`, `BASE-fsm.c`                       | `BASE-fsm.rs`                             |
| mc         | `BASE-mc.h`, `BASE-mc.c` (includes `BASE-fsm.h`) | `BASE-mc.rs` (references `BASE-fsm.rs`)   |
| test       | `BASE-test.c`, `BASE-fuzz-target.c`, plus the co-emitted `BASE-fsm.h`/`BASE-fsm.c` | `BASE-test.rs`, `BASE-fuzz-target.rs`, plus the co-emitted `BASE-fsm.rs` |

Explicit per-artifact flags override individual names of the expanded set. The
generation options (`--assumes`, `--steps`, `--no-std`, `--with-property-monitors`,
the mc-mode family, `--with-protocol-fsm`) keep their current semantics unchanged.

Stdout mode: `--out -` prints the single artifact to stdout (Rust `production`
only - multi-artifact commands refuse it, consistent with the existing strict
refusal of stdout for commands with extra files). This is a deliberate behaviour
change: today BOTH backends' `production` command prints its single self-
contained file to stdout when no output option is given; under the new policy
no-output-flag invocations WRITE spec-stem-named files instead. All call sites
migrate in this phase.

### CLI examples

The contract table (reviewed and confirmed 2026-11-13): one row per invocation
shape, both backends plus the edge rules. "present" means the file exists on
disk at the resolved path before the invocation.

| # | CLI example | Description of args/options | Expected inputs | Expected outputs | Rationale |
|---|-------------|------------------------------|-----------------|------------------|-----------|
| 1 | `rfs codegen rust production spec.rfs` | No output options -> BASE = spec stem `spec` | `spec.rfs` | writes `spec-fsm.rs` to CWD, exit 0 | The default policy names artifacts from the spec - shell-prototyping convenience |
| 2 | `rfs codegen rust production spec.rfs --out -` | Explicit stdout marker, single-artifact command only | `spec.rfs` | FSM source on **stdout**, exit 0 | Keeps the pipe workflow (`\| head -1`) alive under the new write-by-default policy; multi-artifact commands refuse `-` |
| 3 | `rfs codegen rust production spec.rfs --out arq` | `--out BASE` shortcut -> `arq-fsm.rs` | `spec.rfs` | writes `arq-fsm.rs`, exit 0 | Explicit base name |
| 4 | `rfs codegen rust production spec.rfs --out-fsm-code my_fsm.rs` | Explicit per-artifact flag overrides the derived name | `spec.rfs` | writes `my_fsm.rs`, exit 0 | User-chosen file name |
| 5 | `rfs codegen rust mc examples/valid/43_sliding_window.rfs` | No output options -> BASE = spec stem; fsm derived, absent -> co-emitted | the spec file | writes `43_sliding_window-fsm.rs` + `43_sliding_window-mc.rs` to CWD, exit 0 | The flagship prototyping case - one command, predictable names, nothing to type |
| 6 | `rfs codegen rust mc spec.rfs --out arq` | BASE shortcut; fsm derived from BASE, absent -> co-emitted | `spec.rfs` | writes `arq-fsm.rs` + `arq-mc.rs`; mc references `arq-fsm.rs`, exit 0 | Complete self-consistent set in one invocation |
| 7 | `rfs codegen rust mc spec.rfs --out arq --fsm-file existing_fsm.rs` | fsm name WITH extension; present on disk -> **taken** | `spec.rfs` + `existing_fsm.rs` (present) | writes only `arq-mc.rs`, exit 0 | Reuse a named fsm without a separate step |
| 8 | `rfs codegen rust mc spec.rfs --out-mc-code checker.rs --fsm-file my_fsm` | per-artifact flag for the mc file; fsm name WITHOUT extension -> `my_fsm.rs`; absent -> **emitted** | `spec.rfs` | writes `my_fsm.rs` + `checker.rs`, exit 0 | Specific fsm name, single invocation |
| 9 | `rfs codegen rust mc spec.rfs --out-mc-code checker.rs` | No `--out`, no fsm flag -> BASE = spec stem; fsm derived `spec-fsm.rs`, absent -> co-emitted | `spec.rfs` | writes `spec-fsm.rs` + `checker.rs`, exit 0 | The default policy supplies the missing name - per-artifact flags alone are sufficient |
| 10 | `rfs codegen rust test spec.rfs --out arq` | BASE shortcut; fsm co-emitted (absent) | `spec.rfs` | writes `arq-fsm.rs` + `arq-test.rs` + `arq-fuzz-target.rs`, exit 0 | Full test set, no FSM duplication |
| 11 | `rfs codegen rust test spec.rfs --out arq --fsm-file existing_fsm.rs` | fsm present -> taken | `spec.rfs` + `existing_fsm.rs` (present) | writes only `arq-test.rs` + `arq-fuzz-target.rs`, exit 0 | Reuse semantics for the test command |
| 12 | `rfs codegen c production spec.rfs` | No output options -> BASE = spec stem | `spec.rfs` | writes `spec-fsm.h` + `spec-fsm.c` to CWD, exit 0 | The header/source pair gets spec-stem names by default |
| 13 | `rfs codegen c production spec.rfs --out arq` | BASE shortcut -> pair | `spec.rfs` | writes `arq-fsm.h` + `arq-fsm.c`, exit 0 | Idiomatic C separation, each file with its own line-1 artifact header (D-P6-3) |
| 14 | `rfs codegen c production spec.rfs --out-fsm-header arq_fsm.h --out-fsm-code arq_fsm.c` | Explicit pair, independent names/locations | `spec.rfs` | writes exactly those two files, exit 0 | Full control (e.g. header into an `include/` dir) |
| 15 | `rfs codegen c mc spec.rfs --out arq` | BASE shortcut; fsm pair co-emitted (absent); `arq-mc.c` includes `"arq-fsm.h"` | `spec.rfs` | writes `arq-fsm.h`, `arq-fsm.c`, `arq-mc.h`, `arq-mc.c`, exit 0 | Complete set, strict separation |
| 16 | `rfs codegen c mc spec.rfs --out arq --fsm-file arq_fsm` | C form 1: stem -> pair `arq_fsm.h` + `arq_fsm.c` (no `-fsm` suffix); absent -> emitted; mc includes `"arq_fsm.h"` | `spec.rfs` | writes `arq_fsm.h`, `arq_fsm.c`, `arq-mc.h`, `arq-mc.c`, exit 0 | Specific fsm name without a separate step |
| 17 | `rfs codegen c mc spec.rfs --out arq --fsm-file-header include/arq.h --fsm-file-source src/arq.c` | C form 2: explicit pair; BOTH present -> taken | `spec.rfs` + both files (present) | writes only `arq-mc.h`, `arq-mc.c` (mc includes `"include/arq.h"`), exit 0 | Reuse an fsm installed elsewhere in the tree |
| 18 | `rfs codegen c test myspec.rfs --fsm-file arq_fsm` | No `--out` -> BASE = spec stem `myspec`; fsm flag overrides the derived `myspec-fsm.*` name; pair present -> taken | `myspec.rfs` + `arq_fsm.h`, `arq_fsm.c` (present) | writes only `myspec-test.c` + `myspec-fuzz-target.c` (both include `"arq_fsm.h"`), exit 0 | `--fsm-file` overwrites the spec-name default when `--out` is absent |
| 19 | `rfs codegen c test spec.rfs --out arq` | BASE shortcut; fsm pair co-emitted; test + fuzz target include `"arq-fsm.h"` (never the .c) | `spec.rfs` | writes `arq-fsm.h`, `arq-fsm.c`, `arq-test.c`, `arq-fuzz-target.c`, exit 0 | Kills the `#include "...fsm.c"` whole-source inclusion |
| 20 | `rfs codegen c test spec.rfs --out arq --fsm-file arq_fsm` | C form 1; pair present -> taken | `spec.rfs` + `arq_fsm.h`, `arq_fsm.c` (present) | writes only `arq-test.c` + `arq-fuzz-target.c`, exit 0 | Reuse semantics for the test command |
| 21 | `rfs codegen rust mc spec.rfs --out arq --fsm-file other.rs` | `other.rs` present but its line-1 header spec hash != current spec's hash | `spec.rfs` + stale `other.rs` (present) | **warning** on stderr (stale reuse), still referenced; writes `arq-mc.rs`, exit 0 | The D-P6-3 artifact header makes drift detectable without blocking legitimate reuse |
| 22 | `rfs codegen c mc spec.rfs --out arq --fsm-file arq_fsm` | C form 1; only `arq_fsm.h` exists, no `arq_fsm.c` | `spec.rfs` + partial pair | **usage error**, exit 1 (no silent overwrite of the lone file) | A partial pair is ambiguous - strict refusal |
| 23 | `rfs codegen rust test spec.rfs --out arq --out-test-code custom_test.rs` | Explicit per-artifact flag overrides the BASE default for that artifact only | `spec.rfs` | writes `arq-fsm.rs`, `custom_test.rs`, `arq-fuzz-target.rs`, exit 0 | Shortcut + targeted override compose |

## Artifact separation model

- C `production`: the header carries the include guard, the v1.0 line-1 artifact
  header, the type definitions (enums, input/output unions, `step_result_t`,
  `fsm_t`), the public API declarations (`fsm_new`, `fsm_step`, policy/monitor
  surface) and the `RFS_SPEC_VERSION` / `RFS_PROPERTY_MONITORS` defines; the
  implementation file includes its own header and carries the static helpers (with
  the `-Wunused-function` pragma) and all function bodies. Both files carry their
  own line-1 artifact header (D-P6-3).
- C `mc` / `test` / fuzz target: each `.c` includes the fsm HEADER (never the .c);
  builds compile every `.c` to an object file and link - explicit objects in the
  example Makefiles, object + link in the `rfs test` pipelines.
- Rust: `BASE-fsm.rs` content is unchanged; `BASE-mc.rs` / `BASE-test.rs` /
  `BASE-fuzz-target.rs` reference it via a path-based include instead of carrying
  a second copy (exact mechanism - `include!` vs `#[path] mod` - is a plan-stage
  design decision).
- Invariant: the FSM implementation exists in exactly ONE file per invocation;
  dependent artifacts reference it. No code duplication across artifacts.

## Deliverables
- Help texts (global usage + every per-command/codegen help) referencing only the
  `rfs` binary; zero `cargo run --` examples in any help output.
- The per-artifact output-flag family, the fsm-reference flags, the `--out BASE`
  shortcut, the spec-stem default naming and the `--out -` stdout marker in
  `parse_codegen_args` / `CodegenRequest::Generate`, with the restructured
  `CodegenOutput` (named artifact slots).
- C backend: header/source split for production; mc/test/fuzz-target artifacts
  referencing the fsm header. Rust backend: dependent artifacts referencing the
  fsm file.
- Updated `rfs test` compiler invocations (C unit + fuzz pipelines) and updated
  MC-compiling test legs.
- Fixed examples: `crates/examples/c_sync_handler` (header-only include, explicit
  object files in the Makefile), `crates/examples/fuzz_c` (separate-source clang
  build).
- Migrated + new tests covering the flag family, default-name expansion, the
  split, and cross-artifact reference correctness.
- Synced living docs: `KB.md`, `BUILD.md`, `README.md`, `START.md`,
  `docs/user/*`, `src/codegen/CODEGEN.md`, `src/codegen/backends/{C,RUST}.md`;
  updated training entries + regenerated snippets/aggregate.

## Exit criteria
- [ ] `rfs --help` and every per-command / codegen help output contain no
      cargo/rustc invocation examples - only the `rfs` binary is referenced.
- [ ] No living document (`KB.md`, `BUILD.md`, `README.md`, `START.md`,
      `docs/user/*`) contains a `cargo run -- <rfs-subcommand>` example; legitimate
      cargo usages (workspace builds, `cargo run -p <example>`, cargo-fuzz) are
      unchanged.
- [ ] The 14 training entries teaching `cargo run --` are rewritten to `rfs`; the
      snippet + aggregate regeneration passes (`cargo test-training`).
- [ ] codegen accepts the per-artifact output flags, the fsm-reference flags
      (take-existing-or-emit), the `--out BASE` shortcut AND the spec-stem default
      naming when `--out` is absent - all expanding to the documented default
      names for BOTH backends; `--out -` prints to stdout for Rust production
      only, and the 23-row CLI examples table in "Proposed CLI" holds verbatim.
- [ ] C production emits a header + implementation pair; no generated C artifact
      contains a second copy of the FSM implementation and no generated C does
      `#include "...fsm.c"`.
- [ ] Rust mc/test/fuzz-target artifacts reference the separately generated fsm
      file instead of inlining a duplicate.
- [ ] `rfs test` compiles + runs green for both backends under the new multi-file
      layout (unit AND fuzz suites).
- [ ] `c_sync_handler` builds via explicit object files (arq_fsm.o + handler.o)
      with handler.c including only the header, and its scenario output is
      unchanged; `fuzz_c` builds with clang over the separate sources.
- [ ] Full verification green: `cargo test` all pass (main-test count at or above
      the 535 baseline, any delta explained by new tests), generated-code tests
      pass, `cargo fmt --check` clean, clippy at the pre-phase baseline (no new
      warnings).
- [ ] Docs synchronized: CODEGEN.md + backend docs describe the artifact model;
      user docs show the new CLI; the dev_docs sync gate is green.

## Non-goals
- No RFS language syntax, IR, validator or MC-semantics changes (exact/fuzz/smt
  modes and the byte-parity contracts are untouched).
- No change to generated-binary runtime flags or report formats (the parity-pinned
  surface stays byte-identical).
- No rewriting of historical records: `docs/roadmap/phase_0..13/*`, `TASKS.md`
  history entries and `commands.md` keep their frozen `cargo run --` text.
- No new build-system support (CMake/meson) - Makefile + direct compiler
  invocations only; no new codegen backends.
- No change to the Rust example crates' cargo-workspace build model beyond what
  artifact separation requires.
