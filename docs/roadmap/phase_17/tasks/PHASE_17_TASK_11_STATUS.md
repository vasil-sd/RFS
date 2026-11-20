# Phase 17, Task 11 - Implementation status

> Written once when all exit criteria of `PHASE_17_TASK_11.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Rewritten stale entries (3)

| File | Entry (user question) | What changed |
|---|---|---|
| `training/frontend_cli/rfs_training.jsonl` | "What does the rfs mc-ref subcommand do? Explain its flags and exit codes." | was exact-only, listed `--seed`/`--violate`/`--nondet` as NOT applicable. Now a mode dispatcher over all three engines: per-mode flag tables (exact / fuzz / smt-bounded) with defaults, per-mode exit codes, `--nondet` applicable under exact, NOT-applicable reduced to `--threads`/`--fast`, parity with BOTH backends in all three modes. Real captured runs for exact-pass, fuzz-violation (per-run report, no summary line), and fuzz-no-violation (summary line). |
| `training/frontend_cli/rfs_training.jsonl` | "When should I use rfs mc-ref versus a generated model-checking binary?" | claimed "it has no fuzz mode at all" and that the generated binary is "the only way to get fuzz mode". Now: mc-ref covers exact/fuzz/smt-bounded in-process; the generated binary's UNIQUE value after Phase 16 is `--threads`/`--fast` + the production-identical engine. Rule of thumb updated accordingly (the old claim is explicitly refuted, not silently dropped). |
| `training/model_checking/rfs_training.jsonl` | "How do I model-check an RFS spec quickly without compiling any generated code?" | was exact-only with the same NOT-applicable list. Now a mode dispatcher: per-mode flag table, NOT-applicable reduced to `--threads`/`--fast`, per-mode exit codes, unified reports byte-comparable in all three modes. |

## New coverage entries (4)

| File | Entry (user question) | Category rationale |
|---|---|---|
| `training/model_checking/rfs_training.jsonl` | "How do I read a model-checking counterexample from RFS - what do the fields mean for a property violation versus a deadlock, and how do I turn the trace into a fix?" | CE-trace reading - the skill the negative examples teach. Walks task 6's REAL captured traces: the property-violation PREFIX (HTTP/2 RstStream retarget, `label`/`step`/`vars at violation step`) and the deadlock dead-end (TCP-handshake terminal state, `label=-`), plus the lasso shape for liveness. The JSON `counterexample.kind` discriminator (`trace`/`lasso`/`deadlock-config`) is explained with a real `deadlock-config` document. The fix discipline (derive from trace, re-run to prove) closes it. The property-violation spec is fenced as an rfs block (a complete valid module -> lands in snippets/valid). |
| `training/codegen_rust/rfs_training.jsonl` | "How does `rfs codegen` decide which files to write - what is the take-or-emit behavior for the FSM artifact, and when do I get a usage error versus a warning?" | Artifact take-or-emit semantics - the most error-prone CLI surface. BASE derivation, per-artifact override, taken-vs-emitted (rust mc run twice), the C partial-pair USAGE ERROR, the stale-reuse WARNING (not an error), and `--out -` single-artifact-only. A small valid spec is fenced (-> snippets/valid). Consistent with USAGE.md section 6. |
| `training/frontend_cli/rfs_training.jsonl` | "How does rfs find external tools like gcc and rustc - what is the RFS_* environment contract, and how do I inspect which binary it will actually call?" | RFS_* tool-resolution precedence: flag > env var > standard var > PATH, the nine-tool table, `--print-script` `# tools:` inspection using task 8.4's REAL captures (explicit env, flag-beats-env, bare-PATH fallback), and the clang-no-standard-tier rule. No rfs fence (the proof lines are sh). |
| `training/project_tooling/rfs_training.jsonl` | "What example projects does RFS ship under crates/examples, and what does each one demonstrate?" | The ten Phase 17 example projects - every one named and grouped (negative MC examples / cargo-member crates / plain-dir walkthroughs) with a one-line description + how to run each, plus the five originals. Points at USAGE.md section 11 as the index. No rfs fence. |

## Regenerated artifacts

| Artifact | Operation | New totals |
|---|---|---|
| `training/<cat>/snippets/` | regenerated via `cargo extract-training-snippets` | **140 valid + 128 invalid** (baseline was 138 valid + 128 invalid). The +2 valid are the two intentionally-fenced complete specs: the HTTP/2 rst spec (model_checking) and the timeout_retry spec (codegen_rust). No new invalid snippets - both fences parse+validate, so neither is an accidental partial-syntax fragment. |
| `all_rfs_training.jsonl` | regenerated via `cargo build-training` | **271 lines** (baseline 267; +4 = the four new entries; the three rewrites are in-place so they add no lines). |

## Tests run status

| Name | Status |
|---|---|
| `cargo test-training` (4) | OK - `committed_all_rfs_training_file_is_fresh`, `training_categories_exist_and_have_valid_jsonl`, `aggregate_training_sets_matches_category_files`, `extracted_training_snippets_parse_as_expected` all pass with the new entries + regenerated snippets/aggregate. |
| `cargo test` (root pkg, 584) | OK - 0 failures across every test binary. |
| `cargo clippy --all-targets` | 0 errors (only pre-existing duplicate warnings in untouched test helpers). |
| `cargo fmt --check` | clean. |

## Exit-criteria verification
- **No stale mc-ref claim remains.** Grep for "no fuzz mode" returns exactly ONE hit - entry B's deliberate refutation ("'mc-ref has no fuzz mode' is no longer true"). A regex scan for any `NOT applicable` sentence listing `--seed`/`--violate`/`--nondet` returns ZERO. The only NOT-applicable flags anywhere are `--threads`/`--fast`.
- **Every new entry follows the format rules** (two messages, `< think> ... < /think>` block) - enforced structurally by `cargo test-training` (green).
- **Snippets + aggregate regenerated; new totals recorded** above (140 valid + 128 invalid / 271 lines).
- **Full checklist green:** `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check`.

## Found problems/issues/mismatches
- None. The two fenced specs were verified to be complete, valid modules BEFORE fencing (they are pinned example specs), so both landed in snippets/valid as intended - no accidental invalid snippet was produced, and no fence needed fixing.

## Other important findings/information
- Every nuanced claim was verified by RUNNING the command against the built `target/debug/rfs`, not from memory: the exact-pass report (spec 43), a fuzz violation (spec 13, per-run report with NO summary line, exit 1), a fuzz no-violation (spec 43, summary line present, exit 0), the exact-only-flag-under-fuzz usage error, the smt-bounded extra-flag usage error, and the smt-feature-missing diagnostic. The `# tools:` proof lines in the tool-resolution entry are task 8.4's real captures from `crates/examples/tool_resolution/captures/`.
- The three rewrites keep their user messages byte-identical (including the trailing newline) and replace only the assistant answer, per the task instruction to locate by question text and leave the question untouched.
