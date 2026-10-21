# Phase 5, Task 9 - Implementation status

> Written once when all exit criteria of `PHASE_5_TASK_9.md` are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up phase plans.

## Added source files

| Name | Description |
|---|---|
| (none) | All changes land in existing files; the no_std transformation is header-only by construction. |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `src/codegen.rs` | changed | +`pub const NO_STD_PRODUCTION_ONLY_ERROR` (pinned rejection message shared by parser and backend entry points); `BackendOptions.no_std: bool` (Default false) with full doc; `CodegenRequest::Generate.no_std`; `--no-std` parsing (production only, else the pinned error); `common_help()` new option line. |
| `src/main.rs` | changed | `Generate` destructure + `BackendOptions` construction carry `no_std`. |
| `src/backends/rust_fsm.rs` | changed | `Gen.uses_vec_macro` (any transition or on_error with non-empty `emits`); `emit_prelude` no_std path: `#![no_std]` before the unchanged `#![allow(...)]` line, then `extern crate alloc;`, `use alloc::string::{String, ToString};`, `use alloc::vec::Vec;`, and `use alloc::vec;` ONLY when `uses_vec_macro` (the `vec!` macro is exported at the alloc crate root; importing it unconditionally would warn). The body is emitted by the SAME code paths as std mode - byte-identical. The two expression-only helper `Gen` constructors (`render_mc_step_all`, `render_sample_formula_evaluator`) set `uses_vec_macro: false` (they never render prelude/outputs). |
| `src/backends/c_fsm.rs` | changed | `emit_header`: when `options.no_std`, one comment line after the includes: `// --no-std accepted: the C backend is already dependency-free; the flag has no effect on this output.` (documented no-op, CLI symmetry). |
| `src/backends/rust_test.rs`, `src/backends/rust_mc.rs`, `src/backends/c_test.rs`, `src/backends/c_mc.rs` | changed | Defensive library-level guard: each test/mc entry point returns `Err([NO_STD_PRODUCTION_ONLY_ERROR])` when `options.no_std` (covers API users who bypass the CLI parser; rust_mc's check precedes its SMT branch). |
| `crates/rfs-build/src/lib.rs` | changed | T1-1 delivered: `RfsBuild.no_std: bool` field + `.no_std(bool)` builder method (off by default); `compile()` appends `--no-std` after `--with-property-monitors`; module doc + `compile()` doc updated. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `CODEGEN.md` | changed | §3.1 BackendOptions sketch gains `no_std`; note on the pinned error constant; §4.1 defaults + parse-error list mention `--no-std`; §4.3 example line. |
| `src/backends/RUST.md` | changed | §2 new paragraph: no_std header anatomy, body byte-identity rationale (alloc provides Vec/String/ToString, panic! comes from core), the sweep test that proves it. |
| `src/backends/C.md` | changed | §2 new paragraph: accepted documented no-op + the exact header comment. |
| `BUILD.md` | changed | Section 1 rfs-build row (+`.no_std(bool)`); section 3 counts actualized (306 root / 314 workspace / 8 rfs-build) + codegen suite note incl. the no_std sweep; section 4 generation rows gain `[--no-std]` + new no_std row. |
| `docs/roadmap/INTRO.md` | changed | Phase 5 row -> "tasks 1-3 + 8-9 of 16 done". |

## Other file changes

- `TASKS.md`: one new entry (2026-10-21).
- `tests/codegen_test.rs`: +7 tests (see below).

## Changed functionality

- None to existing commands: default (std) output is byte-identical for all examples (proven by the 537/537 generated-code aggregate suite regenerating+running every module, and by the body-parity test comparing std vs no_std on all 45 x 2 modes).

## Removed functionality

- None.

## New functionality

- `--no-std` for the PRODUCTION codegen command:
  - Rust: generated file becomes `#![no_std]` + `extern crate alloc;` + alloc imports; IDENTICAL public API shape (body byte-identical to std output - only the header differs); compiles as a no_std lib crate.
  - C: accepted as a documented no-op with a header comment (plain C is already dependency-free).
  - `test`/`mc`: rejected with exit 1 + pinned message `--no-std is only supported for the production command` (CLI parser AND all four backend entry points).
- `rfs-build`: `.no_std(bool)` builder option mapping to the flag (task correction T1-1 from task 1 delivered).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| `cli_no_std_flag_parsing` | new | parse level: production accepts the flag (rust+c), default off, test/mc rejected with the pinned message exactly |
| `cli_no_std_e2e_exit_codes_and_output` | new | real binary: production --no-std exit 0 + no_std header on stdout; test --no-std exit 1 + pinned diagnostic on stderr |
| `test_mc_entry_points_reject_no_std` | new | library level: all four test/mc entry points return the pinned error for no_std options |
| `no_std_header_pins_and_std_unchanged` | new | byte-pins: std header unchanged (2 lines); no_std header exact 6 lines on a no-output module; NO `use alloc::vec;` when unused |
| `no_std_header_imports_vec_macro_when_outputs_exist` | new | 10_output_payloads: line 7 is `use alloc::vec;`, body contains `let outputs = vec![` |
| `c_production_no_std_is_documented_noop` | new | C: note present iff flag set; removing the note line makes no_std output byte-equal to std output |
| `no_std_compile_sweep_and_body_parity_all_valid_examples` | new | ALL 45 valid examples x {plain, --with-property-monitors}: (1) body after header strip BYTE-IDENTICAL between std and no_std (the API-identity criterion - stronger than "same pub items", chosen as the robust variant); (2) no_std artifact compiles with `rustc --edition 2021 --crate-type lib --emit=metadata` (catches any leftover std item). Compile step gated on rustc PATH availability like the real-tool diagram tests; body parity always enforced. |
| `no_std_option_pins_argv` (rfs-build) | new | stub rfs records argv: default has no `--no-std`; `.with_property_monitors(true).no_std(true)` pins exact order `... --out <art> --with-property-monitors --no-std` |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (root package) | OK (**306/306**; codegen 26 -> 33; was 299) |
| `cargo test --workspace` | OK (**314/314**; rfs-build 7 -> 8; was 306) |
| `cargo test-generated-code` | OK (537/537 - std output byte-identical, no drift) |
| `cargo clippy --all-targets` | OK (zero new warnings; the same 12 pre-existing baseline remain: c_fsm x9, codegen from_str, validator, codegen_test &PathBuf) |
| `cargo fmt --check` | OK (clean) |

## Found problems/issues/mismatches

- C9-1 (environment quirk, not a code bug): `cargo clippy` intermittently failed in this container with "failed to run rustc to learn about target-specific information / couldn't read from stdin, as it did not contain valid UTF-8". Diagnosis: the target-info probe (`clippy-driver rustc - --crate-name ___ --print=...`) inherits cargo's stdin; a shell session whose stdin pipe delivers non-UTF-8 bytes breaks the probe. Reproduced on the PRISTINE tree (git stash), fixed by re-running in a fresh shell session. Future sessions: if clippy suddenly fails this way, retry before suspecting code changes.
- Pre-existing (unchanged): example 41's generated monitor code emits `idx as u64 >= 0` which warns `unused_comparisons` - present in BOTH std and no_std output; the task's allow-list policy is not to widen `#![allow(...)]`, so the warning stays (it predates this task and is identical in both variants).

## Other important findings/information

- The no_std transformation needed NO body changes: a full-pattern sweep of all 45 valid examples x both monitor modes found exactly four std-referencing constructs in generated production code - `Vec`/`vec!`, `String`, `.to_string()` (ToString), and `panic!`. alloc provides the first three; `panic!` comes from core. Hence "identical public API" is achieved as literal body byte-identity (stronger than the task's suggested "same pub items" structural check).
- `use alloc::vec;` (module import) also brings the `vec!` macro into scope because `#[macro_export]` places it at the alloc crate root - this is why a single line suffices and why `use alloc::vec::{Vec, vec}` does NOT compile.
- For task 11 (adapters): the no_std-compatible minimal adapter wrapping these FSMs will need `extern crate alloc` + the same three imports; the generated API it consumes is unchanged by this task.
- For task 15 (docs sync): DESIGN section 23 should record the no_std decision (D9 follow-through: core+alloc, identical API, C no-op, test/mc stay std).
