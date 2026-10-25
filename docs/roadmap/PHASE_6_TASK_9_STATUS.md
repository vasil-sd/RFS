# Phase 6, Task 9 - Implementation status

> Written once when all exit criteria of PHASE_6_TASK_9.md are reached. Not
> rewritten afterwards; later fixes go into TASKS.md entries and follow-up
> phase plans.

## Added source files

None (documentation-only task).

## Changed/removed source files

None - no code was touched, per the task's non-goals.

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `DESIGN.md` | changed | NEW section 24 "Phase 6 production-hardening decisions (as shipped)": D-P6-1..D-P6-11 in the section-23 format + an "execution notes" subsection recording plan corrections 2a/4b/5a/5b/6a/6b/6c, the final example counts (50 valid / 13 invalid), and the 2026-10-25 mermaid/mdr rule change. Task-1 cross-reference pointers from sections 2/3/4 to `docs/SEMANTICS.md` verified in place. |
| `KB.md` | changed | section 1 state table (crate 1.0.0, EIGHT-command CLI, Phase 5 row -> COMPLETE, new Phase 6 row, examples 50/13, re-measured test/training counts, clippy note) + pipeline diagram (+mc-ref/gen-spec); section 2 gained the D-P6 pointer block; section 4.2 source map (+5 new modules with responsibilities, main.rs -> 8 subcommands, codegen_mc.rs report fields, tests/common/ row); section 7 "Resolved during Phase 6" paragraph (provenance, C well-definedness, machine-checked MC trustworthiness, SEMANTICS.md, docs/user/); section 8 roadmap (Phase 5 -> COMPLETE, Phase 6 -> tasks 1-9 complete + close-out pending, post-v1 candidates); section 12 next work rewritten. |
| `BUILD.md` | changed | section 3: root row 453 pass (238 lib) with stale sub-counts corrected, workspace row 477, generated-code row 1683/192, nine NEW suite rows (ref_mc_sweep/ref_mc_diag/ref_mc_diff/prod_parity/monitor_ref/psl_translation/toolchain_fuzz/wide_ops_parity/docs_md) with measured counts; section 4: new `mc-ref` + `gen-spec` command rows, training/generated alias counts refreshed; section 5: known-good baseline rewritten with re-measured numbers (2026-10-25); section 1 Cargo.toml row v1.0.0 + dev-deps note. Sections 6/7 (fuzzing, benchmark CI) verified consistent - unchanged. |
| `CODEGEN.md` | changed | section 5.3: JSON report model updated (`statistics.peak_rss_kb` self-reported/informational/zeroed before byte comparisons; counterexample `ce_steps`); section 9: new invariant 7 - the line-1 provenance header contract (fields stable, wording may move in MINOR bumps; pin fields not wording). |
| `src/backends/RUST.md` | changed | section 2: line-1 provenance header bullet + no_std sweep count 45 -> 50; section 4: VmHWM peak-rss self-reporting note on the exact engine. |
| `src/backends/C.md` | changed | section 2: SAME line-1 header as Rust (verified against a live capture - `//` comment, identical fields so the hash matches across backends); section 5: getrusage(RUSAGE_SELF) peak-rss note. |
| `START.md` | changed | section 1: Phase 6 summary bullet + re-measured baseline; section 2 doc map: new `docs/user/` row; section 3 source tree: 8 subcommands, +5 new src modules, fuzz/ workspace row, Phase 6 test suites, examples 50/13, aggregate 261; section 7 code block count refreshed. |

## Other file changes

None.

## Changed functionality

None - documentation-only; no runtime behavior changed.

## Removed functionality

None.

## New functionality

None (the deliverable is the synced documentation itself).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| none | - | docs-only task; no test changes |

## Tests run status

| Name | Status |
|---|---|
| `cargo build` (exit 0) | OK |
| `cargo test` (453/453) | OK |
| `cargo clippy --all-targets` (exit 0; 36 pre-existing warnings, all in Phase 6 tasks 4-6 files) | OK |
| `cargo fmt --check` | OK |
| re-measurement runs: `cargo test --workspace` (477/477), `cargo test --features smt` (455/455, z3 env per TOOLCHAIN.md 5.2) | OK |

## Found problems/issues/mismatches

- BUILD.md was INTERNALLY inconsistent before this task: section 3's generated-code row said 1601/189 (task 6's update) while the section 5 baseline block still carried the Phase 5 numbers (1501/184, 376/400/378) - resolved by rewriting both from live re-measurements.
- `src/backends/RUST.md`'s no_std sweep note said "all 45 valid examples" - stale after tasks 2a/6 added examples; corrected to 50 (the suite itself iterates dynamically, confirmed green).
- The C artifact's line-1 header was drafted as a `/* ... */` block comment in this task's first pass; a live capture of `rfs codegen c production` showed it is a `//` line comment - the doc was corrected before commit (docs must match shipped output byte-for-byte in claims).

## Other important findings/information

- Stale-sweep grep list + fixed/kept-as-history hits: recorded in `PHASE_6_TASK_9.md` "Other information" (two-pass sweep over all live docs; roadmap PHASE_* files left untouched as frozen historical records).
- Final re-measured baseline for task 10's status file: `cargo test` 453/453; `--workspace` 477/477; `--features smt` 455/455; `test-generated-code` 1683/1683 (192 modules); examples 50 valid / 13 invalid; training aggregate 261 lines (snippets 138 valid + 128 invalid); clippy exit 0 with 36 pre-existing warnings (tests/common/prod_driver.rs x14, src/backends/c_fsm.rs x9, single hits in tests/common/mc_json.rs, src/validator.rs, src/test_cmd.rs, src/codegen_test.rs, src/codegen.rs); fmt clean.
- The clippy drift (13 -> 36 pre-existing warnings since the Phase 5 close-out record) comes from cfg-gated dead-code helpers in `tests/common/prod_driver.rs` and monitor-rendering/from_str naming in `src/backends/c_fsm.rs` - all in files this task did not touch; fixing them is out of scope here (docs-only) and can be a post-v1.0 cleanup.
