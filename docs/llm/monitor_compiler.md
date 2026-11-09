<!-- llm-budget: 120 -->
# RFS Monitor Compiler (Class B)

Compact card for LLM assistants working on PSL-to-monitor compilation.

<a id="llm:monitor-compiler"></a>
## Card

**Responsibility.** The language-agnostic PSL-to-monitor compiler
(`src/codegen/monitor.rs`): at codegen time each in-subset prop/assume formula is
compiled into a `MonitorPlan` node tree that every backend renders with
identical semantics. The plan is the single source of truth - changing it
changes ALL generated monitors (production FSM, MC state layout, test
harness) in both backends at once.

**Key items.**

| Item | Role |
|---|---|
| `src/codegen/monitor.rs::compile_formula` | entry: formula -> MonitorPlan or Err (outside the R1-R4 monitor subset - no monitor generated, identical skip warning in every backend) |
| `src/codegen/monitor.rs::compile_check` | the recursive pass: anchor range + in_temporal threaded down; per-arm dispatch (be[N] one-shot vs recurring anchors, implication trigger extraction, next = shifted range, always[N] window [0,N-1], R1-R4 rejections) |
| `src/codegen/monitor.rs::compile_signal` / `formula_to_signal` | temporal-operator-free subformula -> per-sample SignalExpr; Option (None = temporal present); formula_to_signal is the public wrapper used by the Phase 4 liveness engine + test planner |
| `src/codegen/monitor.rs::AnchorRange` | [from, to] evaluation window (to=None unbounded); all() = top-level [0,inf); shifted() = +1 (next); bare signals anchor at the FULL range - invariants, not one-shots (sample 0 has no I/O) |
| `src/codegen/monitor.rs::MonNode` | five node kinds: DelayedCheck (one-sample delay register, vacuous first sample), RangeCheck (stateless per-sample), Obligation (D-P3-3 pending model: pending/remaining/trigger_step; re-trigger while pending ignored), And (any child fails), Or (ALL children fail at the SAME sample) |
| `src/codegen/monitor.rs::MonitorPlan` | { label, is_assumption, root } - one per prop/assume; MC config encoding walks the same tree in preorder (n{id} node ids) |
| `src/codegen/monitor.rs::ReferenceMonitor` / `step_node` | tests-only reference interpreter mirroring backend-generated state node for node; step returns NEWLY-confirmed failure; per-sample (not sticky) child status is essential for Or/And composition |

**Invariants that must not break.**

- D-P3-3 deadline semantics: obligation triggered at i with bound N confirms
  the violation at sample i + N - 1 (countdown reaches 0), NOT one sample
  past; be[0] = body at the trigger sample itself; always[N] window = samples
  0..N-1 (N >= 1); next = vacuous at sample 0.
- R1-R4 gate: no be under negation; be body temporal-operator-free + anchor
  rules; always[N] never under a temporal operator; next body temporal-
  operator-free. ANY eventually/until is outside the subset (MC-only, D-P4-8).
- Sticky failed flag per node; root failure = property violation; first
  failing sample wins; obligation failures are terminal.
- Parity: backends render the SAME plan identically - a plan change must be
  verified in BOTH backends + ref_mc ground truth (tests/ref_mc_diff_test.rs,
  monitor_ref_test, psl_translation_test).

**Change impact.** Placement rule first (src/codegen/CODEGEN.md section 2); verify with
cargo test --lib (in-module pins) + monitor_integration_test +
backend_parity_test, then the full BUILD.md section 5 checklist. Full table:
llm:change-impact.

**Deep dive.** dev:monitor-compiler-overview + dev:monitor-compiler-signals /
dev:monitor-compiler-nodes / dev:monitor-compiler-compile /
dev:monitor-compiler-reference in docs/dev/monitor_compiler.md; algorithm
views L1-L3: dev:algorithms-monitor-compile; semantics: SEMANTICS.md section
5.5 + DESIGN.md section 21 (D-P3-1..6); backend rendering:
docs/dev/backend_rust.md + docs/dev/backend_c.md; MC state layout:
dev:mc-engine-plan.
