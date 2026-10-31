<!-- llm-budget: 150 -->
# RFS Codegen Core (Class B)

Compact card for LLM assistants working on the backend API or the shared
codegen planning modules.

<a id="llm:codegen-core"></a>
## Card

**Responsibility.** The language-agnostic codegen core: the backend plugin API
+ registry + CLI parsing (`src/codegen.rs`) and the shared planning/concept
modules every backend renders from - production-FSM analysis
(`src/codegen_fsm.rs`), the static unit-test planner (`src/codegen_test.rs`),
model-checker concepts (`src/codegen_mc.rs`). The parity principle is the
critical constraint: one plan, rendered identically in Rust and C.

**Key items.**

| Item | Role |
|---|---|
| `src/codegen.rs::Backend` / `src/codegen.rs::BACKENDS` | static plugin value (name, description, command descriptors, 3 fn pointers) + the registry (C + Rust); a new backend appends ONE entry |
| `src/codegen.rs::CodegenCommandKind::run` / `run_module` | standard dispatch: stamp spec hash over the WHOLE file's canonical IR JSON, generate for the FIRST module; run_module = per-module library path, caller owns the hash |
| `src/codegen.rs::BackendOptions` | shared options bag (assumptions, steps, no_std, monitors, mc mode/limits, counterexample format, protocol-fsm opt-in, spec_hash); backends ignore inapplicable fields |
| `src/codegen.rs::CodegenOutput` / `ExtraFile` | code + warnings + extra files (fuzz_target.rs/.c is separate - libFuzzer dep; CLI refuses stdout mode with extras) |
| `src/codegen.rs::parse_codegen_args` | all-errors CLI parsing; command-scoped rejections via the two pinned error constants |
| `src/codegen_fsm.rs::build_variants` | flattens event/output groups: single-variant bare name, else group_variant, numeric suffixes on duplicates; NOT yet target-mangled |
| `src/codegen_fsm.rs::representative_values` | candidate-input defaults: bool false, bit 0, enum first variant (full path) |
| `src/codegen_test.rs::build_test_plan` | the static planner entry: TestPlan of cases/expectations/labels - DATA, never code |
| `src/codegen_test.rs::expect_progress_from_initial` | EXACT positive-drive prediction (representative candidates + guard under initial vars); not a heuristic |
| `src/codegen_test.rs::plan_transitions` | per-transition taken/not-taken cases; firing order = priority desc then source order, mirroring generated fsm_step; capped static search (4096) |
| `src/codegen_test.rs::Lcg` / `RANDOM_TRACE_SEED` | deterministic random-trace sequence (same constants as the fuzz wire format; seed 12345), precomputed at plan time |
| `src/codegen_mc.rs::ExactStatus` / `exact_summary_line` | pass-complete/fail/unknown-state-limit taxonomy (exit 0/1/2) + the pinned summary-line format |
| `src/codegen_mc.rs::McJsonReport` / `JsonCounterexample` | JSON report model; KEY ORDERING IS PART OF THE BYTE-PARITY CONTRACT (ce_steps last; peak_rss_kb informational, normalized to 0 in parity) |
| `src/codegen_mc.rs::EXACT_DEFAULT_*` / `SMT_DEFAULT_BOUND` | codegen-time defaults embedded as runtime-overridable (timeout is wall-clock only, NOT parity-guaranteed) |

**Invariants that must not break.**

- Parity: a new feature lands in EVERY backend in the same change + cross-backend differential tests (tests/backend_parity_test.rs, liveness_mc_test, benchmark suite).
- The planner emits data, never code; its static evaluator mirrors the backend lowering EXACTLY (u128 wrapping + mask to type width, shifts saturate at width, bool bitwise = logical) - drift makes planned cases disagree with runtime.
- Firing order everywhere is descending effective priority then source order (mirrors generated fsm_step); unguarded earlier siblings shadow; monitor-violation drives skip transitions a sibling would take first (C5-1).
- Pinned formats: summary line, JSON key ordering, LCG constants + seed, test-name scheme + random-trace line, ViolationSpec wire format, the two option-rejection strings.
- Generated binaries embed their own copies of MC parsing helpers + JSON writer - format changes must update both backends' emitters in the same change.
- Core contains no target-language syntax; backends contain no core-level IR analysis (placement rule, CODEGEN.md section 2).

**Change impact.** Placement rule first (CODEGEN.md section 2); new backend = registry entry only (section 6 checklist). Verify: cargo test --test codegen_test, cargo test --test backend_parity_test, then the full BUILD.md section 5 checklist. Full table: llm:change-impact (task 13.2).

**Deep dive.** dev:codegen-core-overview + dev:codegen-plugin-api / dev:codegen-fsm-helpers / dev:codegen-test-plan / dev:codegen-mc-concepts in docs/dev/codegen_core.md; extension guide: CODEGEN.md (repo root); algorithm levels: dev:algorithms-ir-codegen; call graph: dev:deps-fn-codegen-rust.
