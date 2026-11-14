# Phase 14, Task 7 - Implementation status

> Written once when all exit criteria of `PHASE_14_TASK_7.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

(none.)

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `crates/rfs-build/src/lib.rs` | changed | **Pulled-forward fix (task correction)**: `compile()` invoked the CLI with the retired `--out <exact file>` semantics, so under Phase 14's BASE naming every rfs-build consumer broke (the artifact landed at `<path>-fsm.<ext>` and the post-invoke existence check failed). The invocation now passes per-slot flags: production rust `--out-fsm-code OUT_DIR/<stem>.rs`; production C `--out-fsm-header` + `--out-fsm-code` (the C source includes its own header); test rust `--out-test-code` + `--out-fuzz-target-code OUT_DIR/fuzz_target.rs` + `--fsm-file OUT_DIR/<stem>` (take-or-emit picks up a prior production run's file); test C the same with the form-1 stem `OUT_DIR/<stem>_fsm`. `cargo_directives` now emits one `cargo:include` per generated file; the post-invoke check verifies every written file. The stub `rfs` script honours the new output flags (incl. the `.rs`-appending take-or-emit for `--fsm-file`); all five argv pins + the two real-binary e2e tests updated. Consumer-visible behaviour is UNCHANGED: the main artifact still lands at `OUT_DIR/<stem>.<ext>` and the skeleton at `OUT_DIR/fuzz_target.{rs,c}` - no build.rs in the workspace needed editing. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `KB.md` | changed | Section 4.5: all 22 `cargo run -- <command>` example lines -> `rfs <command>`; the codegen block rewritten to the new CLI (spec-stem default, `--out BASE`, per-artifact flags, take-or-emit, `--out -`) with a new "Output options (Phase 14)" bullet; the stale "`--out FILE` (default stdout) / test REQUIRES --out" wording and the "generated source goes to stdout or --out" line replaced; the `test` subcommand's pipeline prose updated to the tasks 4-5 reality (C unit = object+link, C fuzz = both sources compiled together, Rust unit = two-file include! layout); the rfs-build row's "extra file next to the main artifact" wording -> named-slot description. |
| `BUILD.md` | changed | Command table: 14 `cargo run -- <cmd>` rows -> `rfs <cmd>`; the two codegen rows rewritten to the new flag family (Rust: `[--out BASE \| --out-fsm-code FILE ...] [--fsm-file NAME[.rs]]` + spec-stem default note; C: header flags + form-1/form-2 fsm reference + "compile each .c with gcc -Wall -c and link"). Legitimate cargo usages untouched. |
| `README.md` | changed | Mermaid node labels `cargo run -- codegen {rust,c} production/test/mc` -> `rfs codegen ...`. |
| `START.md` | changed | Section 7 sanity line -> `rfs validate --input rfs examples/valid`. |
| `docs/user/README.md` | changed | Mermaid labels; the usage example is now bare `rfs`; two-minute tour rewritten (`--out /tmp/arq` -> `/tmp/arq-fsm.rs` with a comment explaining BASE naming); the conventions note inverted (examples ARE the installed-binary form; building from source is a maintainer concern). |
| `docs/user/INTEGRATION.md` | changed | 9 examples -> `rfs ...`; the no_std and artifact-drift examples use `--out BASE` with the resulting file name commented; the C Makefile pattern rewritten to the header/source pair model (explicit `--out-fsm-header/--out-fsm-code`, object files, link) with the stale "fuzz pipeline compiles ONLY the skeleton" note replaced by the both-sources shape; the rfs-build mermaid node + `.test_command()` table row updated to the named-slot description. |
| `docs/user/LANGUAGE.md` | changed | 4 mermaid labels; quick-start conventions note inverted; the three tour examples rewritten (codegen shows `--out /tmp/trivial` -> `/tmp/trivial-fsm.rs`); the beacon walkthrough examples -> `rfs ...`. |
| `docs/user/MODEL_CHECKING.md` | changed | mc-ref example -> `rfs mc-ref`; section 3's generate-and-run block rewritten to show BOTH backends under the new CLI: Rust (`--out /tmp/mc` -> mc-fsm.rs + mc-mc.rs, compile the mc file - include! resolves) and C (four files, `gcc -Wall -c` each source, link). |
| `docs/user/PROPERTIES.md` | changed | The worked-examples intro line -> `rfs mc-ref <spec>`. |
| `docs/user/EXAMPLES.md` | changed | The three section-1 examples -> `rfs ...`. |
| `docs/roadmap/CURRENT_STATE.md` | changed | Phase-14 row: tasks 1-7 complete, tasks 8-11 pending. |

## Other file changes

| Name | Operation | Description |
|---|---|---|

(none.)

## Changed functionality
- rfs-build's CLI invocation (the only source change - see the task correction):
  from the retired `--out <exact path>` to per-slot output flags, with one
  `cargo:include` directive per generated file and a post-invoke existence
  check over every written file. Consumer-visible paths are unchanged.
- Living documentation now teaches ONLY the `rfs` binary for rfs commands and
  describes the Phase 14 CLI (flag family, spec-stem default naming,
  take-or-emit, artifact pairs, object+link builds) wherever codegen is shown.

## Removed functionality
- The `cargo run -- <rfs-subcommand>` invocation pattern from every living
  document (71 occurrences across 10 files).
- Stale CLI descriptions: "`--out FILE` (default stdout)", "the test command
  REQUIRES `--out`", "generated source goes to stdout or `--out`", "the fuzz
  pipeline compiles ONLY the skeleton (it #includes the production source)".

## New functionality
- Documented, execution-verified walkthroughs of the new CLI: every codegen
  example in the user set was run against the real binary during this task and
  shows the actual resulting file names (`BASE-fsm.rs`, `BASE-mc.{h,c}` +
  `BASE-fsm.{h,c}`, `BASE-test.c` + `BASE-fuzz-target.c`, ...).
- The C MC workflow in MODEL_CHECKING.md now documents the object+link build
  shape (compile each `.c` with `gcc -Wall -c`, link) that tasks 3-4 made real.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `crates/rfs-build/src/lib.rs` stub argv pins (5) | changed | Re-pinned to the per-slot flag invocations (`--out-fsm-code`, `--out-test-code --out-fuzz-target-code --fsm-file`, ...). |
| `crates/rfs-build` stub `rfs` script | changed | Honours the new output flags; mimics take-or-emit for `--fsm-file` (appends `.rs`, emits only if absent). |
| `crates/rfs-build` real-binary e2e tests (2) | unchanged (now passing) | They had been RED since task 1's CLI change (root `cargo test` never builds the example crates, so the breakage was invisible); they pass again with the fixed invocation. |

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` | OK - 5 passed / 0 failed (all mermaid blocks valid, incl. the edited INTEGRATION.md node; run AFTER the final doc edits) |
| `cargo test --workspace` | OK - 57 suites, 676 passed / 0 failed (includes all 32 root suites + rfs-build's 10 + the example crates whose build.rs exercise the fixed rfs-build against the real binary) |
| `cargo fmt --check` | OK |
| `cargo clippy -p rfs-build` | OK - 0 warnings (a doc-list-indentation warning my first rewrite introduced was reworded away; HEAD had none) |

## Found problems/issues/mismatches
- **rfs-build was functionally broken by task 1's CLI change** (pre-existing, not introduced by this task): `compile()` passed `--out OUT_DIR/<stem>.<ext>`, which under BASE semantics writes `<path>-fsm.<ext>` - the post-invoke check then failed with "artifact was not written". Root `cargo test` stayed green because it never builds the example crates; `cargo test --workspace` (and `-p rfs-build`) was red. Fixed here as a pulled-forward correction (phase precedent: corrections 2a/4b) because INTEGRATION.md documents this workflow as working and the phase invariant is a green tree.
- The first draft of the INTEGRATION.md Makefile pattern used `--out $(BUILD_DIR)/arq_fsm` expecting `arq_fsm.h/.c` - actually `--out BASE` writes `BASE-fsm.h/.c` (`arq_fsm-fsm.*`). Caught by RUNNING the documented command; the pattern now uses explicit `--out-fsm-header/--out-fsm-code`, which was re-verified end-to-end (generate, header-only include, object compile warning-free, link, run).

## Other important findings/information
- **Verified-untouched legitimate cargo usages** (the grep bar's allowed residue - all `-p` workspace-member or workspace-note forms, none teaching the rfs invocation pattern): BUILD.md:13 (`cargo run -p <crate>` demo binaries), BUILD.md:42 + 142 (workspace notes "`cargo run` keeps working via..."), BUILD.md:139 + 160 (`cargo run -p rfs-dev -- ...`), BUILD.md:187 (`cargo run -p sync_handler && cargo run -p async_handler`), docs/user/INTEGRATION.md:110-111 (`cargo run -p sync_handler` / `-p async_handler`), docs/user/EXAMPLES.md:184 (`cargo run -p sync_handler`).
- Every codegen example written into the user set was executed against the real binary during this task (Rust MC two-file compile, C MC object+link, C production pair + header-only consumer, `--out BASE` naming) - the docs describe observed behaviour, not intended behaviour.
- The c_sync_handler example Makefile still carries its task-2 minimal shape (handler.c includes the whole .c); the INTEGRATION.md pattern now documents the final pair/object-link model that task 9 will bring the example in line with.
