<!-- llm-budget: 150 -->
# RFS Model-Checking Engine (Class B)

Compact card for LLM assistants working on the exact/SMT MC engines or the
reference checker.

<a id="llm:mc-engine"></a>
## Card

**Responsibility.** The model-checking engine family: the codegen-time plan
(`src/mc/plan.rs`) embedded in both generated MC binaries, the brute-force
reference checker (`src/mc/ref_mc.rs`) used as ground truth behind `rfs mc-ref`
and differential testing, and the shared SMT spec + emitter for the z3-based
mode. Parity by construction (one plan/spec, both backends) is the critical
constraint.

**Key items.**

| Item | Role |
|---|---|
| `src/mc/plan.rs::McPlan` / `compute` | codegen-time plan - pure function of module + assumption selection; embedded verbatim in BOTH generated binaries so the Rust/C exact engines are identical by construction (mirrors the Phase 3 MonitorPlan pattern) |
| `src/mc/plan.rs::referenced_fields` | input abstraction D-P4-3: a payload field is concrete iff referenced by a guard / emit arg / assignment binding OR an `input_is` pattern with CONSTRAINING fields (wildcards/binds constrain nothing); abstracted fields collapse to one representative |
| `src/mc/plan.rs::ConfigEncoding` | config = state index + packed vars + monitor transition states, little-endian into total_bytes - the buffer both engines hash for the visited set; sticky failed/trigger_step deliberately EXCLUDED (invariant on all live configs) |
| `src/mc/plan.rs::TraversalSpec` | canonical input order (declaration order, values ascending) + per-state firing order (effective priority desc, ties by declaration) + assumption labels for --assumes |
| `src/mc/plan.rs::LivenessDecomposition` | D-P4-9: obligations (`eventually p` / `p until q`, boolean-over-atoms bodies) anywhere in the boolean skeleton; uncheckable shapes -> diagnostics (codegen warnings, never parse errors) |
| `src/mc/ref_mc.rs::run` | brute-force ground truth: level-synchronous BFS, per-config timeout check, assumptions-first pruning, first-violation-wins stop AFTER the per-level loop (SEMANTICS 9.5), deadlock + monitor projection capture, liveness post-BFS on completed OR truncated graph |
| `src/mc/ref_mc.rs::Mon` / `MonSt` / `mon_step` | OWN monitor compiler (independent of the shared MonitorPlan): per-sample (not sticky) failure status - essential for Or/And composition; mon_key = persistent projection (remaining packed only while pending) |
| `src/mc/ref_mc.rs::CfgKey` / `bnode_key` | STRUCTURED configuration key, no bit-packing - deliberate: behavior must not depend on the container (D-P4-2); enums as variant index, bools 0/1, bits raw |
| `src/mc/ref_mc.rs::liv_analyze` | liveness: position graph with EDGE-based labels, product automaton reachability, iterative Tarjan SCCs, falsifying limit vector search (q ascending, discovery order), lasso witness with GLOBAL visited pruning (path-only pruning spins on diamond subgraphs) |
| `src/mc/ref_mc.rs::render_text` / `to_json_report` | pinned report formats - byte-identical to the generated exact-mode binaries (text: status line + shared summary line + statistics block + "counterexample trace:"; JSON: same key order, ce_steps last) |
| `src/mc/ref_mc.rs::verify_counterexample` / `minimal_violation_labels` | ground-truth CE verifier (replay + per-kind checks; first-confirmation rule; lasso acceptance = liv_analyze's end-component condition) + two-phase minimal-step label collection - the differential suite's oracle |
| `src/mc/smt_plan.rs::build_smt_spec` | backend-neutral SMT spec from the shared plan (step variables, constraints, queries) - detail: dev:mc-engine-smt |
| `src/mc/smt_emit.rs::emit_rust_spec` / `emit_c_spec` | shared emitter rendering the spec for Rust and C - parity by construction; one libz3 build (5.3.1) - detail: dev:mc-engine-smt |

**Invariants that must not break.**

- Parity by construction: a new feature lands in EVERY backend in the same change + cross-backend differential tests (tests/backend_parity_test.rs, prod_parity_test); ref_mc vs generated engines is pinned by tests/ref_mc_diff_test.rs + ref_mc_sweep_test.rs (byte-parity, text AND json).
- Container-independence: behavior must not depend on visited-set / graph containers (D-P4-2) - CfgKey stays structured; the C engine's sorted-vector mirror and Rust's HashMap must agree.
- Pinned formats: exact summary line + statistics block, text report format, JSON key ordering (ce_steps last), lasso fields (prefix/cycle/cycle_length), formula text, config byte layout.
- Soundness rules: truncated analysis - a witness inside the explored part is sound, otherwise UNKNOWN stands; NEVER a false pass-complete; first-violation-wins stop rule (SEMANTICS 9.5); deadlock = no successor at all with the monitor projection carried for external re-derivation (SEMANTICS 8.4).
- ref_mc is GROUND TRUTH: an independent implementation of SEMANTICS.md - its own monitor compiler + liveness decomposition must stay in sync with the documented semantics; the negative control ("no false PASS") lives in the diff suite.
- Firing order everywhere = descending effective priority then source order - mirrors generated fsm_step AND the planner.

**Change impact.** Placement rule first (CODEGEN.md section 2); verify with
cargo test --test ref_mc_diff_test + ref_mc_sweep_test + backend_parity_test,
then the full BUILD.md section 5 checklist. Full table: llm:change-impact.

**Deep dive.** dev:mc-engine-overview + dev:mc-engine-plan /
dev:mc-engine-exact / dev:mc-engine-liveness / dev:mc-engine-verify in
docs/dev/mc_engine.md (dev:mc-engine-smt is task 11.2); semantics: SEMANTICS.md
sections 5.5, 6.4-6.6, 8.3, 8.4, 9.2, 9.5; shared plan data:
docs/dev/codegen_core.md; generated twins: docs/dev/backend_rust.md +
docs/dev/backend_c.md; monitor compiler: docs/dev/monitor_compiler.md (task
11.3).

