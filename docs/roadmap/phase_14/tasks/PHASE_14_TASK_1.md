# Phase 14, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Land the new codegen output model and CLI flag family atomically: restructure
`CodegenOutput` from (single `code` string + unnamed `extra_files`) to named
artifact slots, implement the full output-flag family with BASE resolution
(`--out` value or spec-file stem), take-or-emit fsm-reference semantics and the
`--out -` stdout marker in `parse_codegen_args`, rewrite the CLI write loop,
and migrate every call site and test to the new naming - while generator CONTENT
stays byte-identical (the actual file separation lands in tasks 2-5).

```
rfs codegen <backend> <production|test|mc> <spec.rfs|spec.json>
    [output options]            <- this task: parsing + naming + writing
    [generation options]        <- unchanged semantics
```

## Context

- Current output model (`src/codegen/mod.rs`): `CodegenOutput { code: String,
  warnings: Vec<String>, extra_files: Vec<ExtraFile> }`. The rust/c `test`
  commands put their fuzz skeleton into `extra_files`; everything else is a
  single `code` string.
- Current CLI parsing (`parse_codegen_args` in `src/codegen/mod.rs`):
  `args[0]` = backend name, `rest[0]` = command (`production|test|mc`),
  `rest[1]` = input file, flags from index 2. The only output option is
  `--out FILE`. `CodegenRequest::Generate` carries `out: Option<String>`.
- Current write loop (`run_cli` in `src/cli/commands/codegen.rs`): with `--out`,
  writes `result.code` to the path (creating parent dirs) and every
  `extra_files` entry next to it, printing `wrote <path>` per file; without
  `--out`, prints `result.code` to stdout and REFUSES to run when
  `extra_files` is non-empty (strict, no graceful degradation).
- The six generator entry points (`generate_production` / `generate_unit_tests`
  / `generate_model_checker` in `src/codegen/backends/{c,rust}/{fsm,test,mc}.rs`)
  all return `CodegenOutput`; they are registered as function pointers in the
  `Backend` struct and dispatched via `CodegenCommandKind::run` / `run_module`.
- Library consumers of the output model that must keep compiling: `rfs test`
  (`src/cli/commands/test.rs` lines ~180-200 read `out.extra_files`, line ~338
  takes `extra_files.first()` as the fuzz skeleton, line ~341 calls
  `Production` for the C fuzz pipeline) and the aggregate-crate path
  (`src/bin/generate_code_tests.rs` uses `rust::fsm::generate_core` directly -
  that function's fused contract is NOT touched by this task).
- Tests pinning the current CLI: `tests/codegen_test.rs` (`cli_parses_list_and_
help`, `cli_parses_backend_help`, `cli_parses_generate_with_options`,
`cli_generate_defaults`, `cli_parses_assume_labels`, `cli_reports_errors`, plus
compile legs that write generated output to fixed names such as
`rfs_generated_fsm.c` at lines ~2039/2121) and the CLI-parsing legs in
`tests/utils_test.rs`.
- Confirmed decisions this task implements (full text in `PHASE_14.md`
"Proposed CLI" + the 23-row CLI examples table): D14-1 per-command flag
subsets; D14-2 fsm-reference flag forms (Rust `--fsm-file NAME[.rs]`; C form 1
`--fsm-file STEM` -> `STEM.h`+`STEM.c`, NO `-fsm` suffix; C form 2
`--fsm-file-header FILE.h --fsm-file-source FILE.c`) with take-or-emit
semantics and the partial-pair usage error; D14-3 BASE := `--out` value | spec
file stem (file name without extension, no directory component), artifacts to
CWD, `--out DIR/BASE` places the set under DIR/, per-artifact flags override
BASE-derived names, `--out -` = stdout for Rust production only; D14-4 line-1
header spec-hash mismatch on a TAKEN fsm file = stderr warning, still
referenced.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | (none) | First task of the phase; no dependencies. |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | C production header/source split | Needs the fsm-header/fsm-code slots this task creates. |
| 3. | C mc artifact pair + fsm reference | Needs the slots + the resolved-fsm-name plumbing. |
| 4. | C test + fuzz-target reference; `rfs test` C pipelines | Needs the fuzz-target slot (replacing `extra_files`) + naming. |
| 5. | Rust mc/test/fuzz-target reference; `rfs test` Rust pipeline | Same, Rust side. |
| 6. | Help-text fix | Rewrites the codegen help to describe THIS task's flag family. |

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/codegen/mod.rs` | change | New `CodegenOutput` slot model; new `CodegenRequest::Generate` fields; `parse_codegen_args` flag family + BASE resolution + error conventions. |
| `src/cli/commands/codegen.rs` | change | Write loop: path resolution, take-or-emit presence check, D14-4 hash warning, dir creation, `--out -` stdout mode. |
| `src/codegen/backends/c/fsm.rs` | change | Adapt `generate_production` to the new output struct ONLY (current single .c content -> fsm-code slot); no emitted bytes change. |
| `src/codegen/backends/c/test.rs` | change | Adapt `generate_unit_tests`: test content -> test-code slot, fuzz skeleton -> fuzz-target slot (was `extra_files`). |
| `src/codegen/backends/c/mc.rs` | change | Adapt `generate_model_checker`: content -> mc-code slot. |
| `src/codegen/backends/rust/fsm.rs` | change | Adapt `generate_production`: content -> fsm-code slot. |
| `src/codegen/backends/rust/test.rs` | change | Adapt `generate_unit_tests`: test content -> test-code slot, fuzz skeleton -> fuzz-target slot. |
| `src/codegen/backends/rust/mc.rs` | change | Adapt `generate_model_checker`: content -> mc-code slot. |
| `src/cli/commands/test.rs` | change | Re-point the two `extra_files` reads (lines ~185-194, ~338) at the new fuzz-target slot; no pipeline behaviour change yet. |

### Affected test files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/codegen_test.rs` | change | Migrate CLI-parsing tests to the new flag family; migrate compile legs' fixed output names to the new naming; add parse-level coverage for the 23-row contract (spec-stem default, per-artifact override, partial-pair error, `--out -` refusal). |
| `tests/utils_test.rs` | change | Migrate CLI-parsing legs that touch codegen args. |

## Planned actions

### Action 1.1 - New output struct + request fields + parser (plan subtask 1.1)

Replace `CodegenOutput { code, warnings, extra_files }` with a slot model:
six optional content slots - `fsm_header`, `fsm_code`, `mc_header`, `mc_code`,
`test_code`, `fuzz_target` (each `Option<String>`) - plus the unchanged
`warnings`. Drop `ExtraFile` (its only consumer, the fuzz skeleton, becomes a
named slot). Extend `CodegenRequest::Generate` with: per-artifact output names
(`out_fsm_code`, `out_fsm_header`, `out_mc_code`, `out_mc_header`,
`out_test_code`, `out_fuzz_target` - all `Option<String>`), the fsm-reference
forms (`fsm_file: Option<String>` for Rust/C-form-1, `fsm_file_header` /
`fsm_file_source: Option<String>` for C form 2), and the raw `--out` value
(`base: Option<String>`, where `Some("-")` is the stdout marker). Rewrite
`parse_codegen_args`: keep the backend/command/input positional parsing, then
the flag loop accepts the new family with the existing error conventions
(unknown option / unexpected argument / missing value). Validation rules:
header flags rejected on the Rust backend; `--fsm-file*` rejected on
`production`; C form 2 requires BOTH header and source flags (one without the
other is a usage error); `--out -` accepted only where a single-artifact result
is possible (enforced at write time, not parse time - the parser cannot know
the backend's artifact count before generation).

Rationale: explicit named slots give exhaustiveness at every write site and
make "which artifacts does this command produce" a property of the filled
slots, not of side-channel `extra_files`.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/mod.rs` | change (`CodegenOutput`/`ExtraFile` definitions) | Slot model replaces code+extra_files; ExtraFile deleted. |
| `src/codegen/mod.rs` | change (`CodegenRequest::Generate`) | New fields for the flag family. |
| `src/codegen/mod.rs` | change (`parse_codegen_args` flag loop) | New flags + validation rules + error conventions. |

### Action 1.2 - Write loop: naming resolution, take-or-emit, stdout (plan subtask 1.2)

Rewrite the `Generate` arm of `run_cli` in `src/cli/commands/codegen.rs`:
(1) compute BASE := `base` if present else the spec file stem (`Path::
file_stem` of the input, no directory); (2) derive each artifact's default
name from BASE per the documented table (`BASE-fsm.h`/`BASE-fsm.c`,
`BASE-mc.h`/`BASE-mc.c`, `BASE-test.c`, `BASE-fuzz-target.c`; Rust: `BASE-fsm.
rs`, `BASE-mc.rs`, `BASE-test.rs`, `BASE-fuzz-target.rs`) and apply per-
artifact flag overrides; (3) resolve the fsm reference: explicit fsm-reference
flags win over the BASE-derived name; C form 1 expands STEM to `STEM.h`+
`STEM.c`; Rust appends `.rs` when the extension is omitted; (4) for each fsm
slot that will be REFERENCED by another artifact (mc/test/fuzz-target), apply
take-or-emit: if the resolved path(s) exist on disk, take them (C form 1:
BOTH must exist - a partial pair is a usage error, exit 1, no overwrite);
otherwise emit the generated content; parse line 1 of a taken fsm file for
`spec sha256:<16hex>` and warn on stderr when it differs from the current
spec's hash (D14-4); (5) write the command's own artifacts fresh (overwrite)
with `create_dir_all` on parent dirs, printing `wrote <path>` per file; (6)
stdout mode: `--out -` prints the single filled slot and is refused (exit 1)
when more than one slot would be written.

Rationale: this is where D14-2/D14-3/D14-4 become observable behaviour; keeping
all naming logic in one place means tasks 2-5 change WHAT is in the slots,
never HOW they are named or written.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/commands/codegen.rs` | change (the `Generate` arm of `run_cli`) | Full rewrite of the output handling per the six steps above. |

### Action 1.3 - Generator adaptation, content unchanged (plan subtask 1.3)

Adapt all six generator entry points to return the new slot struct with their
CURRENT content in the primary slot(s): C/Rust production -> `fsm_code` only;
C/Rust mc -> `mc_code`; C/Rust test -> `test_code` + `fuzz_target` (the fuzz
skeleton moves out of `extra_files`). No emitted byte changes. Re-point the two
`extra_files` reads in `src/cli/commands/test.rs` at the new `fuzz_target`
slot. Update `CodegenCommandKind::run`/`run_module` doc comments that mention
`extra_files`.

Rationale: keeps the tree compiling and all content-pinning tests green while
the naming layer changes underneath; tasks 2-5 then fill additional slots and
change content deliberately.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/codegen/backends/c/fsm.rs` | change (`generate_production` return) | Single .c -> fsm_code slot. |
| `src/codegen/backends/c/test.rs` | change (`generate_unit_tests` return) | Test .c -> test_code; skeleton -> fuzz_target. |
| `src/codegen/backends/c/mc.rs` | change (`generate_model_checker` return) | Executable .c -> mc_code. |
| `src/codegen/backends/rust/fsm.rs` | change (`generate_production` return) | Single .rs -> fsm_code. |
| `src/codegen/backends/rust/test.rs` | change (`generate_unit_tests` return) | Test .rs -> test_code; skeleton -> fuzz_target. |
| `src/codegen/backends/rust/mc.rs` | change (`generate_model_checker` return) | Executable .rs -> mc_code. |
| `src/cli/commands/test.rs` | change (lines ~185-194, ~338) | `extra_files` reads -> `fuzz_target` slot. |

### Action 1.4 - Call-site + test migration to the new naming (plan subtask 1.4)

Migrate every test that invokes codegen with the old `--out FILE` semantics or
reads fixed output names: `tests/codegen_test.rs` CLI-parsing tests (update
expected request shapes; add cases pinning spec-stem default naming, per-
artifact override, C form-1 expansion without `-fsm` suffix, partial-pair
usage error, `--out -` refusal on multi-artifact commands, fsm-reference flag
validation rules) and its compile legs (fixed names like `rfs_generated_fsm.c`
become explicit-flag outputs); `tests/utils_test.rs` CLI-parsing legs. Run the
full root suite until green.

Rationale: the naming change is atomic - no test may keep pinning the old
`--out FILE`-as-file-name semantics after this task.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `tests/codegen_test.rs` | change (CLI-parsing tests) | New request shapes + new contract cases. |
| `tests/codegen_test.rs` | change (compile legs, lines ~2039/2121 area) | Fixed output names -> explicit flags / new naming. |
| `tests/utils_test.rs` | change (codegen CLI-parsing legs) | Same migration. |

## Task deliverables
- `CodegenOutput` slot model + `CodegenRequest::Generate` flag family in
  `src/codegen/mod.rs`, with `parse_codegen_args` implementing D14-1..D14-3.
- The rewritten write loop in `src/cli/commands/codegen.rs` (take-or-emit,
  partial-pair error, D14-4 warning, `--out -`).
- All six generators returning the slot struct with byte-identical content.
- Migrated + extended CLI tests; full root suite green.

## Task exit criteria
- [ ] `cargo test` (root) fully green; no generated-byte changes (content-
      pinning assertions pass against the same text, re-pointed at slots).
- [ ] All 23 contract-table rows that are parse/write-level behave as specified
      (verified by the new/updated tests in codegen_test).
- [ ] `extra_files` / `ExtraFile` no longer exist anywhere in `src/`.
- [ ] `rfs test` still compiles + runs green for both backends (its pipelines
      are unchanged in this task - only the slot reads moved).
- [ ] `cargo fmt --check` clean; clippy at the pre-phase baseline (no new
      warnings).

## Non-goals
- No change to any generated artifact's CONTENT (that is tasks 2-5).
- No help-text changes (task 6) - the codegen help strings stay as-is until
  then, even though they describe the old `--out FILE` semantics.
- No documentation updates (tasks 7-8).
- No change to `generate_core`'s fused contract or the aggregate-crate path.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| `tests/smt_bounded_test.rs` line 139: `.primary_code()` -> `.primary_code().to_string()` (found at close-out, task 11) | The smt-feature leg does not compile under the default feature set, so tasks 1-10's green runs never built it: its `generate_smt_source` helper still expected `primary_code()` to return a `String`, but the slot-model rewrite changed it to `&str` (E0308). One-line fix per the compiler's own suggestion; no behaviour change. Rationale: PROTOCOL.md section 3 - close-out records green, it does not manufacture it; the defect routes back to the owning task (task 1 introduced the signature change) and is re-verified here before the phase exits. |

## Other information

- Re-check while implementing: whether any OTHER library consumer of
  `CodegenOutput.code` / `.extra_files` exists beyond `src/cli/commands/
  test.rs` (grep for both field names before deleting them).
- The spec-stem of a JSON input (`spec.json`) is the stem of the .json file -
  the policy is extension-agnostic; pin this in a test.
- Line-1 header format to parse for D14-4: `// Generated by RFS v<ver> | spec
  sha256:<16 hex chars> | module <name>` - extract the hash between
  `spec sha256:` and the next `|`.
