# Phase 18, Task 3E - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
After tasks 3A-3D close the soundness gaps (D1/D2) and add the missing language features
(D3 assumptions, D4 on_error, D8 enums/wildcards/patterns), smt-bounded mode is a
near-drop-in analysis mode for exact-mode specs. This task makes that VISIBLE and
ACTIONABLE to users by (a) adding a dedicated **"smt-bounded vs exact mode: residual
differences"** section to `docs/user/MODEL_CHECKING.md` - one subsection per RESIDUAL
difference, each giving an exact-mode-supported RFS snippet, the smt-mode alternative (if
any), the detailed rationale for the difference, and step-by-step instructions on how to
alter a spec so that smt-bounded gets as close to exact-mode functionality as possible -
and (b) updating EVERY document under `docs/user/`, `docs/dev/` and `docs/llm/` that still
describes the now-closed limitations, so no doc contradicts the new capability. Rationale:
the whole 3A-3E arc exists so a user can write a natural spec (enums, assumptions,
on_error, temporal properties) and check it under smt-bounded with confidence; without this
documentation pass the user cannot tell what is supported, what is not, or how to rewrite
an unsupported snippet - which defeats the purpose.

## Context

### What 3A-3D change (the new ground truth this task documents)
- **D1 CLOSED (3A):** a `next`/shifted property body is weak at trace death - an unhandled
  input that freezes the successor config no longer false-fails it. smt-bounded now matches
  exact mode §8.3.
- **D2 CLOSED (3A):** a top-level `bounded_eventually[N]` is unconfirmable when the path
  dies before the deadline - matching exact mode §5.5. The in-process and generated
  encodings are now identical.
- **D3 SUPPORTED (3C):** assumptions work. Cheap input-guard assumptions
  (`assume always (input_is(...))`) are default; expensive liveness/temporal assumptions are
  `--full-assumptions`-gated (a flag DISTINCT from `--full-properties`, so the two axes are
  independent) with per-assumption omission warnings +
  `assumptions_omitted=M`. `--assumes all|none|labels` is honored on both smt surfaces.
- **D4 SUPPORTED (3C):** `on_error` blocks work - an unmatched input takes the fallback
  (§3.6-faithful), so specs with a safe-state fallback no longer need rewriting.
- **D8 SUPPORTED (3D):** enum-typed variables, enum payload fields, multi-variant wildcard
  transitions, enum paths in expressions, and enum-variant input/output patterns all encode
  as declaration-order indices (faithful to exact mode). Large-group wildcards may be
  `--full-properties`-gated if profiling triggered it.
- **RESIDUAL (cannot be unified - inherent to bounded MC):**
  - **D5 Completeness of PASS:** exact mode's `PASS (complete)` is an exhaustive proof over
    ALL reachable configurations; smt-bounded's `PASS WITHIN BOUND k (incomplete - no
    completeness claim)` only covers paths of length <= k. No bound makes bounded MC
    complete for an unbounded system. This is the FUNDAMENTAL difference and the reason the
    mode exists (it analyzes specs too large for exact MC).
  - **D6 Counterexample form:** exact mode's counterexample is a REAL execution trace
    (every step an enabled transition, inputs are concrete). smt-bounded's witness is a
    SOLVER MODEL: the state/var sequence is a valid unrolled path, but payload field values
    at each step are solver-chosen free variables (consistent with the fired transitions,
    not "real" input data). After 3A/3C the violation-relevant PREFIX of the witness is
    executable (every step an enabled transition - or the on_error fallback), but steps past
    a path death in a spec WITHOUT on_error are frozen and non-executable, and the payload
    values are still model-chosen.
  - **Declared omissions (not silent divergences):** FULL properties without
    `--full-properties` are omitted with a per-property warning + `properties_omitted=M`
    (D7); expensive assumptions / large-group wildcards without the flag are likewise
    omitted with warnings. These are HONEST omissions the user can lift by passing the flag.

### The MODEL_CHECKING.md section structure (the centerpiece deliverable)
Add a NEW top-level section (number it after the existing sections, e.g. "## 10. smt-bounded
vs exact mode: residual differences" - renumber any trailing cross-reference section) with
an intro paragraph stating that 3A-3D closed D1/D2/D3/D4/D8 and listing what remains, then
ONE SUBSECTION PER RESIDUAL DIFFERENCE. Each subsection follows this EXACT shape so a user
can scan it uniformly:
1. **The difference** - one or two sentences: what exact mode does that smt-bounded does
   not (or does differently).
2. **Exact-mode snippet** - a small, self-contained RFS snippet (a `prop` / `assume` /
   `on_error` / enum var) that a user would naturally write and that exact mode fully
   supports.
3. **smt-bounded alternative** - what smt-bounded does with that snippet today: checks it
   identically (for the closed items, to reassure), omits it with a warning (for the
   flag-gated items, with the flag that lifts the omission), or approximates it (for D5/D6,
   with the approximation stated).
4. **Rationale** - WHY the difference exists: inherent to k-step bounded unrolling (D5), to
   solver-model witnesses (D6), or a deliberate performance gate (the flag-gated omissions).
   Cite the SEMANTICS.md section.
5. **How to convert / how to get closer** - concrete, step-by-step instructions to alter the
   spec so smt-bounded's result is as trustworthy as exact mode's: e.g. for D5, "raise
   `--bound` until the verdict stabilizes across two consecutive bound increases, and run
   `--mode fuzz` as a complement; a PASS WITHIN BOUND that holds at bound k with no new
   violation found at k+1..k+m is strong (though not complete) evidence"; for D6, "add an
   `on_error` block so every input takes a real transition (the witness prefix becomes fully
   executable); read payload values in the witness as 'some consistent assignment', not
   measured data"; for a flag-gated omission, "pass the relevant flag (`--full-properties`
   for properties, `--full-assumptions` for assumptions)".

The subsections, in order: (1) D5 completeness of PASS; (2) D6 counterexample form;
(3) flag-gated FULL-property omission (D7); (4) flag-gated expensive-assumption / large-
group-wildcard omission (if 3C/3D triggered those gates). A short closing table summarizes:
construct | exact mode | smt-bounded | how to close the gap. This table is the quick-reference a user actually keeps open.

### The docs that must be brought into line
Every file below currently states a limitation that 3A-3D closed, or omits a capability
that now exists. Each must be edited so it neither contradicts the new behavior nor leaves a
stale "rejected" / "unsupported" claim. (The roadmap `phase_N/` files are HISTORICAL records
- do NOT rewrite them; only the live `docs/user/`, `docs/dev/`, `docs/llm/` and
`docs/SEMANTICS.md` change.)

## Scope

### Depends on tasks (optional)

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3A. | Fix D1+D2 | closes the trace-death soundness gaps this task documents as resolved |
| 3C. | Implement D3+D4 | adds assumptions + on_error this task documents as supported (with the flag-gated expensive class) |
| 3D. | Maximize language support | adds enums/wildcards/patterns this task documents as supported |

### Dependent tasks (optional)

None - this is the documentation close-out of the 3A-3E arc; task 10 (phase close-out) reads
the final docs state.

### Affected source files (optional)

None - documentation only; no `src/` change. (If a doc edit reveals a BEHAVIOR that
contradicts the code, STOP and record it as a task correction - do not silently "fix" the
doc to match a suspected bug.)

### Affected documentation files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/MODEL_CHECKING.md` | change | the centerpiece: new "smt-bounded vs exact mode: residual differences" section (intro + one subsection per residual difference D5/D6/flag-gated omissions, each with exact snippet / smt alternative / rationale / conversion instructions + a closing summary table); retire the "specs with assumptions or on_error are rejected" line and the D8 limitation notes in the smt-bounded mode description |
| `docs/user/USAGE.md` | change | flag tables: `--assumes` now applies to smt-bounded (remove the not-applicable note); the TWO flag scopes - `--full-properties` (FULL properties + large-group wildcards) and `--full-assumptions` (expensive assumptions), independent axes - per what 3C/3D actually gated |
| `docs/user/LANGUAGE.md` | change | any statement that smt-bounded rejects enum vars / wildcard groups / enum patterns is retired; the language is now uniformly checkable in both modes |
| `docs/user/PROPERTIES.md` | change | the property-language section: note that temporal properties (next / bounded_eventually / until) are checkable under smt-bounded (weak at trace death per 3A), with the FULL-class flag gate |
| `docs/user/FAQ.md` | change | the abstraction-and-assumptions Q&A: assumptions now work under smt-bounded (cheap default, expensive flag-gated); add/adjust a Q on "why is my smt-bounded PASS not 'complete'" pointing at the new MODEL_CHECKING section |
| `docs/SEMANTICS.md` | change | §9.9: rewrite the "v1 scope rejections" paragraph - D1/D2 closed (temporal obligations weak at trace death, matching §8.3/§5.5), D3/D4/D8 supported (with the flag-gated expensive classes); keep the D5 incompleteness + D6 solver-model witness as the stated residual differences |
| `docs/dev/mc_engine.md` | change | the SMT engine description: on_error catch-all + per-step assumption pruning + enum index-encoding + the liveness-gated next / deadline-gated bounded_eventually; retire the "rejects assumptions/on_error/enums" notes |
| `docs/dev/ALGORITHMS.md` | change | the k-step unrolling algorithm: the alive[] liveness gate on temporal shifts, the on_error fallback in the next-state ITE, the per-step assumption base-constraints, the enum index encoding |
| `docs/dev/cli_commands.md` | change | flag reference: `--assumes` under smt-bounded, the TWO flag scopes (`--full-properties` for properties, `--full-assumptions` for assumptions), the omission-warning + summary-suffix behavior (`properties_omitted=M`, `assumptions_omitted=M`) |
| `docs/dev/codegen_core.md` | change | if it describes the SMT spec model (`smt_plan.rs`): the new `SmtSpec.on_error` / `.assumptions` fields, `SmtTrans.classes`, the enum index encoding |
| `docs/dev/backend_rust.md` / `docs/dev/backend_c.md` | change | the emitted SMT runtime description: the multi-class enabled predicate, on_error fallback, assumption constraints, enum indices - both backends (parity) |
| `docs/dev/DECISIONS.md` | change | if a decision record declared the smt-bounded v1 scope (assumptions/on_error/enums rejected), annotate it as SUPERSEDED by the Phase 18 3A-3D corrections (do not delete the historical record) |
| `docs/llm/mc_engine.md` | change | LLM-facing SMT engine overview: same capability updates as dev/mc_engine.md, in the llm doc's style/budget |
| `docs/llm/cli_commands.md` | change | LLM-facing flag reference: `--assuses`-free: `--assumes` + the two flag scopes (`--full-properties`, `--full-assumptions`) under smt-bounded |
| `docs/llm/SRC_STRUCTURE.md` | change | if it lists the smt_plan.rs / ref_mc_smt.rs responsibilities: the new fields/functions (on_error, assumptions, enum encoding, liveness gate) |
| `docs/llm/TASK_MAP.md` | change | map the 3A-3E tasks to the source files they touch (so a future session finds the right code from the task number) |
| `docs/llm/CHANGE_IMPACT.md` | change | the impact table: changing the SMT encoder/engine now affects assumption/on_error/enum specs, not just bit/bool ones |

### Affected test files (optional)

| File | Operation | Brief description |
|------|-----------|-------------------|
| `tests/docs_md_test.rs` (via its file set) | change (indirect) | the edited markdown must still parse/render clean and all links resolve - the docs_md_test gate catches a broken link or malformed table introduced by this task |

## Planned actions

### 3E.1 - Write the MODEL_CHECKING.md residual-differences section
Add the new section per the Context structure: intro (3A-3D closed D1/D2/D3/D4/D8; what
remains), then subsections for D5 (completeness of PASS), D6 (counterexample form), the
flag-gated FULL-property omission (D7), and the flag-gated expensive-assumption /
large-group-wildcard omission (only if 3C/3D actually gated them - check their STATUS files
first). Each subsection: the difference / exact-mode snippet / smt-bounded alternative /
rationale (cite SEMANTICS.md) / how to convert or get closer. End with the closing summary
table (construct | exact mode | smt-bounded | how to close the gap). Retire the stale
"rejects assumptions/on_error" and D8-limitation lines in the existing smt-bounded mode
description earlier in the file.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/MODEL_CHECKING.md` | change | the centerpiece section - gives the user the detailed supported/unsupported/conversion picture the arc exists to provide |

### 3E.2 - Bring the user docs into line
Edit `docs/user/USAGE.md` (flag tables), `docs/user/LANGUAGE.md` (retire enum/wildcard
rejection claims), `docs/user/PROPERTIES.md` (temporal properties checkable under
smt-bounded, weak at trace death), and `docs/user/FAQ.md` (assumptions now work; a Q on
incomplete PASS pointing at the new section). Each edit removes a stale limitation or adds
the new capability - no doc may still say smt-bounded "rejects" assumptions/on_error/enums.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/USAGE.md` | change | flag tables reflect `--assumes` + the two flag scopes (`--full-properties`, `--full-assumptions`) |
| `docs/user/LANGUAGE.md` | change | retire the enum/wildcard/pattern rejection claims |
| `docs/user/PROPERTIES.md` | change | temporal properties under smt-bounded (weak at trace death) + the flag gate |
| `docs/user/FAQ.md` | change | assumptions Q&A updated; incomplete-PASS Q added |

### 3E.3 - Bring the normative + dev docs into line
Rewrite `docs/SEMANTICS.md` §9.9's "v1 scope rejections" paragraph (D1/D2 closed, D3/D4/D8
supported with flag-gated expensive classes, D5/D6 residual). Update `docs/dev/mc_engine.md`,
`docs/dev/ALGORITHMS.md`, `docs/dev/cli_commands.md`, `docs/dev/codegen_core.md`,
`docs/dev/backend_rust.md`, `docs/dev/backend_c.md` to describe the new engine behavior
(on_error catch-all, per-step assumption pruning, enum index-encoding, liveness-gated
temporal shifts). Annotate any `docs/dev/DECISIONS.md` record that declared the old v1 scope
as SUPERSEDED (keep the historical text).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/SEMANTICS.md` | change | normative §9.9 rewrite - the single source of truth the user docs derive from |
| `docs/dev/mc_engine.md` | change | SMT engine capability description |
| `docs/dev/ALGORITHMS.md` | change | unrolling algorithm: liveness gate, on_error, assumptions, enum encoding |
| `docs/dev/cli_commands.md` | change | flag reference |
| `docs/dev/codegen_core.md` | change | SMT spec model fields |
| `docs/dev/backend_rust.md` | change | emitted Rust runtime description |
| `docs/dev/backend_c.md` | change | emitted C runtime description |
| `docs/dev/DECISIONS.md` | change | supersede-annotation on the old v1-scope decision (if present) |

### 3E.4 - Bring the llm docs into line
Update `docs/llm/mc_engine.md`, `docs/llm/cli_commands.md`, `docs/llm/SRC_STRUCTURE.md`,
`docs/llm/TASK_MAP.md`, and `docs/llm/CHANGE_IMPACT.md` per the Scope table, in the llm
docs' style and within their token budgets (the dev_docs_test budget gate applies - raise
it only if a file genuinely grows, per the established convention). Map the 3A-3E tasks to
their source files in TASK_MAP.md so a future session navigates from task number to code.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/llm/mc_engine.md` | change | LLM-facing engine overview |
| `docs/llm/cli_commands.md` | change | LLM-facing flag reference |
| `docs/llm/SRC_STRUCTURE.md` | change | source-file responsibilities for the new SMT fields/functions |
| `docs/llm/TASK_MAP.md` | change | 3A-3E task-to-source mapping |
| `docs/llm/CHANGE_IMPACT.md` | change | impact table widened to assumption/on_error/enum specs |

### 3E.5 - Verify the docs gate
Run `cargo test --test docs_md_test` (every edited file parses/renders clean, all links
resolve) and `cargo test --test dev_docs_test` (the llm-doc budgets hold). Fix any broken
link / malformed table / budget overrun this task introduced.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (no source change) | verify | the docs gates confirm the edits are well-formed and linked |

## Task deliverables
- `docs/user/MODEL_CHECKING.md` carries the new "smt-bounded vs exact mode: residual
differences" section: intro + one subsection per residual difference (D5, D6, flag-gated
omissions) each with exact-mode snippet / smt alternative / rationale / conversion
instructions, plus a closing summary table.
- No document under `docs/user/`, `docs/dev/` or `docs/llm/` still claims smt-bounded
  "rejects" assumptions, on_error, enum vars, wildcard groups, or enum patterns; the
  flag-gated expensive classes are described with their flags and omission warnings.
- `docs/SEMANTICS.md` §9.9 is the normative statement of what is closed (D1-D4, D8) and
  what remains (D5, D6, declared omissions).
- The docs gates (`docs_md_test`, `dev_docs_test`) are green.

## Task exit criteria
- [ ] MODEL_CHECKING.md has the new residual-differences section with a subsection per residual difference (D5 completeness, D6 counterexample form, flag-gated omissions), each containing an exact-mode snippet, the smt-bounded alternative, the rationale (citing SEMANTICS.md), and concrete conversion/get-closer instructions; a closing summary table is present.
- [ ] No file under docs/user/, docs/dev/ or docs/llm/ still states that smt-bounded rejects assumptions, on_error blocks, enum-typed variables, multi-variant wildcard transitions, or enum patterns (grep for the old rejection phrasing returns only historical roadmap files).
- [ ] The flag-gated expensive classes (FULL properties; expensive assumptions and/or large-group wildcards if 3C/3D gated them) are documented with their flag and omission-warning behavior in USAGE.md, MODEL_CHECKING.md, cli_commands.md (dev + llm).
- [ ] SEMANTICS.md §9.9 "v1 scope rejections" paragraph is rewritten to the closed/residual split.
- [ ] `cargo test --test docs_md_test` green (all edited files parse/render, links resolve); `cargo test --test dev_docs_test` green (llm budgets hold).
- [ ] Full checklist `cargo build && cargo test && cargo clippy --all-targets && cargo fmt --check` green.

## Non-goals
- No source-code change - documentation only. A suspected code/doc mismatch is recorded as a
task correction, not silently resolved.
- No rewriting of historical roadmap files (`docs/roadmap/phase_N/...`) - they are records
  of what was done, including the old limitations that motivated 3A-3D.
- No new examples or test legs - the B6/B7 rework is task 3B; this task only documents.
- No change to the exact-mode or fuzz-mode documentation beyond cross-references to the new
  section.

## Task corrections (optional)

| Correction | Description + rationale |
|------------|-------------------------|

## Other information
- Check the 3C and 3D STATUS files BEFORE writing the flag-gated-omission subsections: if
  3C did not gate expensive assumptions (all default) or 3D did not gate large-group
  wildcards, those subsections collapse into the "supported by default" statement - do not
  document a flag that does not exist.
- The closing summary table is the highest-value artifact: a user deciding "can I check my
  spec under smt-bounded, and how close will it be to exact mode?" reads that table. Keep
  every row's "how to close the gap" cell actionable (a flag, a bound increase, an on_error
  addition, or "inherent - use fuzz as a complement").
- The D5 conversion guidance must NOT overclaim: bounded MC is never complete. Phrase it as
  "stronger evidence with a larger bound + fuzz complement", never "equivalent to exact
  mode".
- Renumbering: if the new section is inserted before an existing numbered section (e.g. the
  cross-references section), renumber the trailing sections and fix any intra-doc anchors
  that reference them - docs_md_test catches a dangling anchor.
