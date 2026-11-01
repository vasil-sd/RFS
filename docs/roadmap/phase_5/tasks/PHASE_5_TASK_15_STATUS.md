# Phase 5, Task 15 - Implementation status

Completed 2026-10-23. All design-relevant documentation synced to the FINAL Phase 5 state: DESIGN.md sections 2/8/9/10/14 updated + NEW section 23 "Phase 5 toolchain decisions (as shipped)" recording D-P5-1..D-P5-14 with plan-correction pointers; KB.md, BUILD.md, CODEGEN.md (verified current - no changes needed), src/backends/RUST.md, src/backends/C.md, START.md updated per the scope tables; stale sweep clean.

## Added source files

| File | Description |
|---|---|
| `docs/roadmap/PHASE_5_TASK_15_STATUS.md` | this file |

## Changed/removed source files

None (docs-only task; no `src/` changes).

## Added/changed/extended documentation files

| File | Change |
|---|---|
| `DESIGN.md` | §2 architecture diagram rewritten for the Phase 5 state (hand-rolled json_rw, six commands incl. info/test/diagram, C backend shipped, workspace crates + ide/ note) + "Rust now, C later" -> both shipped; §8.2 keyword list annotated with the `parser::KEYWORDS` single source of truth (28 words, shared with `ide/`, drift-guard test); §9.8 production-integration table updated (trait-based adapters = **Shipped in Phase 5** D-P5-11; tokio-only row = No); §10 rewritten around the SHIPPED production surface (flat enums + trace/assumption runtimes + monitors, `--no-std` D-P5-9, `--with-protocol-fsm` D-P5-11) with the original Phase 0/1 sketch kept as historical + codegen rules marked shipped/superseded; §14 "Implemented so far" rewritten to the six-command final state (info/test/diagram semantics, external-tool parameterization D-P5-7, `--print-script` D-P5-8, exit-code convention); NEW §23 "Phase 5 toolchain decisions (as shipped)" - D-P5-1 workspace layout, D-P5-2 hand-rolled JSON reader-writer (full rationale + C3), D-P5-3 rfs-build + RFS_BIN (+ C11-1 include! pattern), D-P5-4 rfs info, D-P5-5 rfs test incl. both fuzz pipelines (C17-1..C17-5), D-P5-6 rfs diagram plugin registry + strict rendering, D-P5-7 external-tool parameterization (env-preferred, flag > env > PATH, RFS_* names, subcommand extension), D-P5-8 --print-script format, D-P5-9 no_std scope, D-P5-10 counterexample JSON document shape + embedded-writer constraint, D-P5-11 ProtocolFsm approach (optional impl emission, generic adapters, Rust-ecosystem-only), D-P5-12 extended test generation (one plan both backends, C5-1, fuzz skeletons as extra files), D-P5-13 IDE support + validate --json (C13-1..C13-3, D13-1), D-P5-14 example-crate matrix; plan-correction pointers C1/C2/C3/C4 + task-level corrections; §20 D9 gains a "Phase 5 resolution" pointer (no_std shipped) |
| `KB.md` | §1 state table: JSON round-trip row (serde -> hand-rolled json_rw), CLI row (three -> SIX commands with the Phase 5 three named), Phase 4 row (close-out pending -> complete), NEW Phase 5 full-toolchain row, LLM training set row (231 -> 244 + task 14 coverage note); §1 high-level ASCII diagram rewritten (json_rw, six commands, C backend, workspace crates + ide/); §2.1 targets list updated (C automata, text|json MC reports, extended tests + fuzz skeletons, rfs diagram, IDE support with full-LSP non-goal); §2.4 IR serialization row (serde -> hand-rolled json_rw); §4.2 source map: ir.rs row fixed, NEW rows for json_rw.rs / info.rs / ext_tool.rs / test_cmd.rs / diagram.rs+diagrams/, parser.rs + KEYWORDS note, codegen_test.rs extended-planner note, main.rs logic-file list completed; §4.5 CLI behavior: validate section gains `--json` (document shape, line/col semantics incl. the 1:1 whole-module case, no .err side files), NEW `info` subcommand section, NEW `diagram` subcommand section (formats, pure-text primary behaviour, strict --render svg, resolver, --print-script); §4.6 category coverage updated for the Phase 5 entries; §7 known limitations: "Spans in IR" row updated (parse errors DO carry line/col via validate --json; semantic still span-free) and "Feature flags" row marked RESOLVED (--no-std + monitors shipped); §8 roadmap: Phase 3/4/5 rows -> complete, Phase 6 = next; §12 what-to-work-on-next updated (Phase 5 implementation done, task 16 close-out next) |
| `BUILD.md` | Training counts re-pinned to the task 14 state: extract-training-snippets 130 -> **135** valid / 127 invalid, build-training 231 -> **244** examples (command table + alias re-verification note). Everything else verified current (workspace notes, six-command table, final suite counts, env-var coverage, dot/clang availability notes all present from tasks 1-14) |
| `CODEGEN.md` | Verified current - NO changes needed: §4.1 already documents the full Phase 5 flag surface (--no-std, --with-protocol-fsm, --counterexample-format with pinned errors), §5.2 the extended test planning + both fuzz-skeleton emissions, §9 invariant 1 the ProtocolFsm exception |
| `src/backends/RUST.md` | §2.1 gains the optional ProtocolFsm impl emission (D-P5-11: placement after inherent impl Fsm / before #[cfg(test)], explicit forwarding, mc rejection, C ignore, byte-pin); §4 gains the counterexample report format paragraph (D-P5-10: text default unchanged, json document shape, embedded hand-rolled writer for the dependency-free constraint, cross-backend byte parity, exact-mode-only) |
| `src/backends/C.md` | §5 gains two bullets: the JSON renderer mirror (same document via the embedded writer, byte-identical across backends, exact-mode-only) and the `--with-protocol-fsm` accept-and-ignore (byte-identical, Rust-ecosystem-only per parity principle) |
| `START.md` | §0 system prompt: "C backend planned" -> both backends shipped; §1 snapshot rewritten (2026-10-23): Phases 0-4 complete + Phase 5 COMPLETE at implementation level (tasks 1-14 of 16 + task 17) with the full Phase 5 feature list, next work = task 16 close-out, known-good baseline re-pinned (375/399/377 main tests, 1453 generated, 45+12 examples, 244 training / 135+127 snippets); §3 source tree updated (json_rw/info/ext_tool/test_cmd/diagram rows, crates/, ide/, fixed serde reference, 45/12 example counts, 244-line aggregate, full test-suite list); §5 DESIGN.md section map extended to sections 19-23; §6 task table: IR row (serde attrs -> hand-written to_json/from_json per D-P5-2), phase-implementation row (next: Phase 5 task 16 close-out then Phase 6); §7 generated-test count 513 -> 1453; "Before changing IR" workflow step 1 updated |

## Changed functionality

None - documentation only; no tool behavior changed.

## Removed functionality

None.

## New functionality

- DESIGN.md section 23: the first complete record of the shipped Phase 5 design (D-P5-1..D-P5-14 as shipped, incl. plan corrections C1-C4 and task-level correction pointers) - the behavioral twin of the task 14 training data.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no test-code changes; docs-only task |

## Tests run status

| Name | Status |
|---|---|
| `cargo test` (375) | OK (unchanged from the task 14 baseline - docs add no tests) |
| `cargo fmt --check` | OK (clean) |

## Stale-sweep results (exit criteria)

- No live doc describes the CLI as three-command (all six named where the command set is listed; "three command kinds" in CODEGEN.md correctly refers to production/test/mc).
- No live doc describes the repo as a single package (workspace layout current in DESIGN §2/§23, KB §1/§4.2, BUILD §1, START §3).
- No pre-Phase-5 counts remain (230/231/537/513/219 all gone; 375/399/377 + 1453 + 244/135+127 current).
- "Later" markers resolved by Phase 5 updated: DESIGN §9.8 (adapters shipped), §20 D9 (resolution pointer added), KB §2.1 targets (diagrams + IDE shipped, full LSP explicit non-goal), KB §7 feature-flags row (RESOLVED).
- Historical decision records (DESIGN §20/§21 serde-mechanism parentheticals, KB §8 Phase 0 "IR v0 with serde JSON") intentionally left as history per protocol - the current mechanism is documented at every live location and in D-P5-2.
- Cross-references verified programmatically: every `DESIGN.md/KB.md/BUILD.md/CODEGEN.md/RUST.md/C.md section N` citation and every TOOLCHAIN.md section citation resolves to an existing header; all 14 D-P5-x identifiers defined in §23 and only those are cited.

## Task corrections / decisions

- **D15-1**: CODEGEN.md required no changes - tasks 4/5/6/9/10/11 already kept it current (flag surface in §4.1, extended planning + fuzz skeletons in §5.2, ProtocolFsm exception in §9). Recorded so the scope table's CODEGEN.md row is accounted for.
- **D15-2**: the "multiple Rust targets" core-principle phrasing (START.md x2, KB.md x2) was left unchanged - it survived all prior docs-syncs as the architectural principle statement, and the concrete target lists below it are now current; changing the principle wording is a Phase 6/v1.0 decision.
- **D15-3**: DESIGN §15 (testing strategy) and §14's "Planned commands" block kept as historical planning text - the shipped state is documented in the adjacent "Implemented so far" paragraph and section 23, matching the Phase 4 precedent (frozen plan blocks + as-shipped sections).

## Verification

- `cargo test` 375/375 root unchanged; `cargo fmt --check` clean.
- All four exit criteria met: DESIGN §8/9/10 current + §23 present (D-P5-1..14 as shipped incl. corrections); KB/BUILD/CODEGEN/RUST/C/START updated per the scope tables; stale sweep clean (grep-verified above); cross-references resolve (script-checked).
