# Phase 10, Task 21 - Implementation status

> Written once when all exit criteria of `PHASE_10_TASK_21.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `TEST_STRUCTURE.md` | changed | the frontend CLI test suite cluster: 137 function rows for tests/frontend_test.rs + 9 function rows for tests/library_comparison.rs appended to `## All functions`. Every row carries bidirectional project Depends (in-process tests reference the rfs library API - mapped to src/frontend.rs, src/parser.rs, src/validator.rs, src/json_rw.rs, src/diagram.rs, src/diagrams/*, src/info.rs, src/mc_ref_cmd.rs, src/gen_spec_cmd.rs, src/codegen.rs, src/ext_tool.rs, src/lib.rs - plus the tests/common/mc_json.rs helper from task 20; e2e binary tests and std-only helpers have empty Depends), a one-line description, and blank Component/Functionality/Documented-in cells. library_comparison's two same-named module_header fns get distinct `<file>::<module>::<fn>` identifiers (nom_smoke / chumsky_smoke) |
| `docs/roadmap/phase_10/PHASE_10_PLAN.md` | changed | plan correction 21a: the cluster notes' "~1 type" for frontend_test.rs is a phantom - see Found problems |

## Other file changes

| Name | Operation | Description |
|---|---|---|

## Changed functionality

None - docs-only inventory phase (no source moved or renamed).

## Removed functionality

None.

## New functionality

- The frontend CLI test suite is now enumerated in TEST_STRUCTURE.md, covering every area the task lists:
  - Conversion: all four RFS/JSON combinations, strict extension handling, output placement, the collision guard, err-file generation, and end-to-end binary exit codes.
  - Validate: the in-process validate suite plus the Phase 5 task 13 `validate --json` shape/position pins.
  - Info: the text/JSON info sweeps and e2e exit-code/log-level/usage pins.
  - Diagram: pinned outputs for all 5 formats, in-process + external rendering, internal-renderer sweeps, stub argv/env proofs, and strict error cases.
  - rfs test: both backends, byte-pinned --print-script wrappers, stub-tool precedence proofs.
  - rfs test --suite fuzz: stub argv proofs and real-tool-gated e2e.
  - mc-ref / gen-spec: flag parsing + exit-code pins, text/JSON agreement with the generated engine, seed determinism.
  - library_comparison: the nom vs chumsky parser smoke comparison (dev-dependency only).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|

## Tests run status

| Name | Status |
|---|---|
| `cargo test --test docs_md_test` (5) | OK - green; the 146 new rows parse cleanly under comrak GFM (Depends cells are plain text, not backticked paths, so no link checks are triggered) |

## Found problems/issues/mismatches

- Plan correction 21a: the cluster notes' "~1 type" for frontend_test.rs was a phantom. Its single `enum State { Idle, Active, Error }` grep hit (line 1299) is RFS spec text INSIDE the pinned test-input string `PIN_DIAGRAM_RFS`, not a toolchain definition - the same template-phantom class as corrections 7a/8a and 20a. Authoritative per-file counts: frontend_test.rs 137 fns / 0 types, library_comparison.rs 9 fns / 0 types = 146 fns / 0 types total (the ~146 fn estimate was already correct; only the type estimate was a phantom).

## Other important findings/information

- 70 of the 137 frontend_test fns have NO project Depends: the e2e_* tests spawn the compiled rfs binary via std::process::Command (checking exit codes/stdout) and the small helpers (tmp_root, cleanup, write_file, rfs_bin, tool_on_path, write_stub, the env locks/clears) use only std. The in-process tests (converts_*, validate_*, diagram_pinned_*, info_*, mc_ref_*, gen_spec_*) are the ones that reference the rfs library API.
- library_comparison.rs's 9 fns live in two nested modules (nom_smoke, chumsky_smoke); nom and chumsky are external dev-dependencies so they contribute no project Depends - only the same-module helper calls (identifier -> ident_start/ident_cont, module_header -> identifier) are recorded.
- The rfs::IR_VERSION and rfs::parser::KEYWORDS references are consts, so they get no Depends entries per the functions-and-types-only convention.
