# Phase 18, Task 8A - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start implementation of the task immediately without
> re-deriving scope.

## Goal
Add to `docs/user/PROPERTIES.md` a new section of EDUCATIONAL examples: temporal
formulae paired with TRACE VISUALIZATIONS in the style of
https://matrix.ai/learn/blog/programming-with-linear-temporal-logic-operators - a
formula on top, a discrete time axis below, per-sample truth values, and clearly marked
regions where the formula HOLDS and where it is VIOLATED. For each selected property or
assumption there must be SEVERAL trace visualizations (hold cases AND violation cases),
each with a detailed step-by-step description of what happens at every sample. Rationale:
give a user a strong understanding and intuition about temporal operators, their
combinations, and the assumption/property distinction - the single biggest usability gap
of the property language.

## Context
- **The reference style (matrix.ai LTL blog):** each operator is illustrated with a
  diagram showing a proposition's truth values over discrete samples t0, t1, t2, ... and
  shading/markers for where `G p`, `F p`, `X p`, `p U q` etc. hold or fail. The intuition
  being taught: temporal operators are statements about WINDOWS of the trace, not about
  single states.
- **Rendering constraints (binding):** binary images are PROHIBITED by START.md rule 7;
  `docs_md_test` (the mdr-stack gate) machine-checks only markdown + mermaid. The
  visualization format is therefore: annotated ASCII timelines in code fences (a row per
  proposition/property, a column per sample, T/F marks, with the hold/violate region
  marked), plus mermaid `stateDiagram-v2` for the machines where one adds clarity. This
  reproduces the matrix.ai intuition in text form and stays machine-checkable.
- **Traces must be REAL.** Every trace shown is either (a) an actual pinned mc-ref output
  already captured in a Phase 18 example's `captures/` (the fail.rfs counterexamples), or
  (b) the output of running a small teaching spec through `rfs mc-ref` during this task,
  pinned verbatim in the doc. No hand-drawn traces: a visualization that disagrees with
  what the engine prints is worse than none. Where a hold-case trace is needed (mc-ref
  only prints FAIL traces), run the spec and quote the PASS summary + construct the
  timeline from the spec's transition table, marking it as an illustrative walk-through.
- **Existing material to build on:** PROPERTIES.md section 10 already carries four worked
  examples (specs 13/15/16/42) with real mc-ref output - the new section extends that
  discipline with the timeline visualizations and the hold/violate case pairs. The
  operator reference is sections 3-7 (atoms, operators one by one, bounded-eventually
  obligation model, liveness, monitor subset).
- **Candidate educational material (select from these; each was chosen in Phase 18 for
  demonstrating a distinct temporal concept):**
  - `coap_retransmit` (crates/examples): `ack_or_abort : always (state == AwaitingAck ->
    bounded_eventually[16] (state == Idle || state == Aborted))` + the assumption
    `env_timeout : always (bounded_eventually[3] (input_is(Timeout)))` - the flagship
    reactive obligation + environment-contract pair.
  - `vending_machine`: `jam_resets_to_idle : always (input_is(Jam) -> next (state == Idle))`
    - the next-form step semantic, including its weak-at-trace-death behavior (task 3A/3B).
  - `uav_flight_modes`: `geofence_held : always (input_is(GeoBreach) -> state == Rtl ||
    state == Failsafe)` - input-anchored same-sample form (the post-transition sampling
    rule made concrete).
  - `watchdog_reset`: `window_enforced` with its `last_feed_rem` witness - how a witness
    variable makes an otherwise-unobservable pre-step fact checkable.
  - `examples/valid/15_bounded_eventually.rfs` + `42_concurrent_obligations.rfs` (or
    similar from section 10's set) - the obligation window mechanics and concurrent
    obligations.
  - A small NEW teaching spec if a gap remains (e.g. `until` has no Phase 18 example -
    `p until q` needs one to be visualized honestly).
- **Operator coverage requirement:** across the selected examples, the section must
  visualize at minimum: `always`, `always[N]` (if any candidate uses it), `next`,
  `bounded_eventually[N]` (reactive + one-shot forms), `until` or `eventually` (MC-only
  liveness - with the monitor-skip caveat from section 7), and the `input_is` /
  `output_has` anchoring atoms. Assumptions must appear at least once as a first-class
  citizen (what an assumption prunes vs what a property checks - section 1's distinction,
  made visual).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 8. | EXAMPLES.md "Other examples" section | same docs/user pass; 8A runs right after it in the plan order (user-scheduled) and reuses the example projects' captures |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/PROPERTIES.md` | change | new section "Temporal operators, visualized" (numbered after the existing sections): one subsection per selected property/assumption with formula + operator explanation + hold/violate trace visualizations + per-case descriptions |
| `docs/user/README.md` | change (if it indexes PROPERTIES.md content) | pointer to the new section if the index lists sections |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| scratch teaching specs (e.g. an `until` example) | new, under `/tmp` or a throwaway dir - NOT committed unless fenced into the doc as a code block | only if the candidate set has a coverage gap; committed ONLY if the doc references it by path, in which case it goes to `examples/valid/` with the corpus test implications checked |

## Planned actions

### 8A.1 - Select the teaching set
Pick 5-7 properties/assumptions from the Context candidate list (adding a small teaching
spec only for a genuine coverage gap, e.g. `until`). For each, write one line of
selection rationale: which temporal concept it teaches that no other selected example
covers. The set must satisfy the operator-coverage requirement in Context.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/PROPERTIES.md` | change | the selection (with rationales) is stated in the new section's intro |

### 8A.2 - Per-example subsections: formula + operator explanation
For each selected property/assumption: quote the RFS formula verbatim (from the example
spec, linked), then explain EVERY temporal operator and atom it uses in plain language -
what window of the trace it quantifies over, what re-arms it, when it is weak/vacuous.
This is the "formula on top" half of the matrix.ai style.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/PROPERTIES.md` | change | one subsection per selected example, formula + explanation first |

### 8A.3 - Trace visualizations: hold cases AND violation cases
For each selected example produce at least TWO trace visualizations: one where the
formula HOLDS and one where it is VIOLATED (for assumptions: one trace satisfying the
assumption and one violating it - showing which paths get pruned). Format: an annotated
ASCII timeline in a code fence - a header row of sample indices t0..tN, one row per
relevant proposition/state/atom with T/F marks, and a final row marking the formula's
verdict per sample (HOLD / VIOLATED from this point on), plus the real mc-ref output
(PASS summary or pinned FAIL report + counterexample trace) directly below it. Each
visualization gets a detailed description: walk every sample, state which sub-formula is
evaluated, why it holds or fails there, and how the verdict follows. Violation traces
must be the REAL pinned counterexamples (from `captures/` or a fresh mc-ref run quoted
verbatim); hold-case timelines are constructed from the spec's transition table and
marked as illustrative walk-throughs.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/PROPERTIES.md` | change | the timelines + real outputs + per-sample descriptions - the core of the section |

### 8A.4 - Gate verification
Run `cargo test --test docs_md_test` (the new section is in its file set - every fence
and mermaid block must parse/render) and `cargo test --test dev_docs_test` (no doc->code
reference may dangle). Re-check that every quoted mc-ref output matches a fresh run of
the referenced spec byte-for-byte on the relevant lines (state counts, labels, steps).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none - verification only) | - | docs gates green; quoted outputs re-verified against live runs |

## Task deliverables
- A new PROPERTIES.md section: 5-7 selected properties/assumptions, each with its formula,
  a plain-language operator explanation, and >=2 trace visualizations (hold + violate)
  with per-sample descriptions - covering always / next / bounded_eventually[N] /
  until-or-eventually / input_is-output_has anchoring / the assumption distinction.

## Task exit criteria
- [ ] The new section exists in `docs/user/PROPERTIES.md`, numbered after the existing sections, with an intro stating the selection + rationales.
- [ ] Every selected example has >=2 trace visualizations (at least one hold case, at least one violation/pruning case), each with a per-sample description; violation traces are REAL pinned mc-ref outputs (re-verified byte-for-byte on the quoted lines).
- [ ] The operator-coverage requirement from Context is met (always, next, bounded_eventually[N], until-or-eventually, anchoring atoms, assumptions as first-class).
- [ ] No binary images; all visualizations are code-fence ASCII timelines (+ mermaid stateDiagrams where used); `cargo test --test docs_md_test` and `cargo test --test dev_docs_test` green.
- [ ] If a new teaching spec was added to `examples/valid/`: the corpus tests account for it (example_corpus_test count moves by exactly that, recorded) - otherwise no example changes at all.

## Non-goals
- No property-language, parser, validator, or MC-engine change - documentation only.
- No PNG/SVG binary assets (START.md rule 7); the matrix.ai style is reproduced in text.
- No restructuring of PROPERTIES.md's existing sections 1-10 (the new section is appended;
  at most a one-line pointer from section 10 to it).
