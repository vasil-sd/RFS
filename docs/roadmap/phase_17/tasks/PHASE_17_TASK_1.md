# Phase 17, Task 1 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Bring the external user documentation in line with the Phase 16 mc-ref
unification: `docs/user/MODEL_CHECKING.md` sections 1-2 must describe all three
mc-ref modes with per-mode flags and exit codes, and `docs/user/INTEGRATION.md`
gains the one-line Nix installation pointer. This is the foundation task -
USAGE.md (task 10) must not contradict what lands here.

## Context
- Current state of `docs/user/MODEL_CHECKING.md` section 2 (verified 2026-11-19):
  an exact-only flag table (`--max-states`, `--timeout-sec`, `--assumes`,
  `--counterexample-format`, `--log-level`), a "NOT applicable" paragraph listing
  `--seed`, `--steps`, `--violate` (fuzz-mode concepts), `--threads`/`--fast`, and
  `--nondet` ("the reference engine explores ALL enabled transitions by
  construction"), a parity paragraph scoped to exact mode, and the closing line
  "Use a generated binary when you need ... fuzz mode". ALL of that is stale.
- The ACTUAL current mc-ref CLI (from `rfs mc-ref`'s own usage text):
  `--mode exact|fuzz|smt-bounded`; exact: `--max-states N` (1000000),
  `--timeout-sec N` (0 = off), `--nondet`, `--counterexample-format json|text`
  (default text); fuzz: `--bound N` (5), `--runs N` (20), `--seed N` (12345),
  `--violate label:prob:percent%,...`, `--unexpected-prob P` (0.0); smt-bounded:
  `--bound N` (10) ONLY, specs with assumptions/on_error rejected, requires the
  `smt` cargo feature; shared `--assumes all|none|labels` + `--log-level`. Exit
  codes: exact 0 pass-complete / 1 fail / 2 unknown-state-limit; fuzz 0 no
  violation (summary line) / 1 violation (per-run report, NO summary line);
  smt-bounded 0 pass-within-bound / 1 fail / 2 unknown. Not applicable:
  `--threads`/`--fast` ONLY.
- Section 1's mode table lists mc-ref only under the Exact row, and the
  following paragraph claims "All three share ... exit codes: 0 = pass-complete,
  1 = fail, 2 = unknown-state-limit" - wrong for fuzz (0/1, no 2) and
  smt-bounded (different status names).
- Byte-parity of mc-ref vs BOTH generated backends holds in ALL three modes
  (Phase 16: `tests/ref_mc_fuzz_test.rs`, `tests/ref_mc_smt_test.rs`) - the
  parity statement can now cover all modes.
- `docs/user/INTEGRATION.md` around line 59 says a cold checkout must run
  `cargo build`; the Nix pointer goes next to it (`nix build` per BUILD.md
  section 8 produces an installable package with the full tool closure).
- The file is machine-checked by `tests/docs_md_test.rs` (mermaid must parse,
  links must resolve) - keep the existing mermaid flowchart in section 1 valid.

## Scope

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `docs/user/MODEL_CHECKING.md` | change | section 2 rewritten for mode dispatch; section 1 table + exit-code paragraph corrected |
| `docs/user/INTEGRATION.md` | change | one-line Nix installation pointer next to the from-source note |

## Planned actions

### Rewrite section 2 (rfs mc-ref) for mode dispatch
Replace the exact-only flag table and "NOT applicable" paragraph with: a short
intro (in-process reference MC, no compilation, all three modes); per-mode flag
tables (exact / fuzz / smt-bounded) with defaults copied from the actual usage
text; per-mode exit codes; `--nondet` documented under exact (fires EVERY enabled
transition as its own successor; liveness then requires strongly-fair bad
cycles); not-applicable reduced to `--threads`/`--fast` with the reason
(single-threaded in-process). Update the parity paragraph: byte-comparable with
BOTH generated backends in ALL three modes where deterministic (modulo the
informational `peak_rss_kb` field; mc-ref reports 0). Replace the closing
"use a generated binary when you need fuzz mode" line: the generated binary's
unique value is now `--threads`/`--fast` + the production-identical engine.
Update the section-1 mermaid flowchart if it implies mc-ref is exact-only (it
currently does not - verify, do not break it).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/MODEL_CHECKING.md` | change (section 2, ~lines 45-83) | the rewrite above - the core staleness fix of this task |

### Fix section 1 (mode table + exit-code paragraph)
The Fuzz and SMT-bounded rows' Command column gains the mc-ref form (`rfs
mc-ref spec.rfs --mode fuzz` / `--mode smt-bounded`). The "All three share ..." 
paragraph becomes per-mode exit codes: exact 0/1/2; fuzz 0 no-violation /
1 violation (no 2); smt-bounded 0 pass-within-bound / 1 fail / 2 unknown.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/MODEL_CHECKING.md` | change (section 1, ~lines 23-43) | table rows + exit-code paragraph accuracy |

### Add the Nix installation pointer to INTEGRATION.md
One line next to the existing "A cold checkout must run `cargo build`" note:
RFS can also be installed via `nix build` (BUILD.md section 8) - the package
carries the full external-tool closure, so `rfs test ...` works from a bare
install. No new section.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `docs/user/INTEGRATION.md` | change (~line 59) | the pointer - users now have two documented installation paths |

## Task deliverables
- `docs/user/MODEL_CHECKING.md` sections 1-2 describing all three mc-ref modes
  accurately (flags, defaults, exit codes, parity).
- `docs/user/INTEGRATION.md` Nix installation pointer.

## Task exit criteria
- [ ] No claim remains that `--seed`/`--violate`/`--nondet` are not applicable
      to `rfs mc-ref`, or that fuzz mode requires a generated binary.
- [ ] Section 2 carries per-mode flag tables with defaults matching the actual
      usage text (spot-check by running `cargo run -q -- mc-ref` with no spec).
- [ ] Section 1's exit-code statement is per-mode and correct (fuzz has no 2).
- [ ] INTEGRATION.md mentions `nix build` as an installation path.
- [ ] `cargo test --test docs_md_test` green; `cargo fmt --check` clean.

## Non-goals
- No changes to other docs/user/ pages (LANGUAGE, PROPERTIES, EXAMPLES, FAQ,
  MIGRATION, README) - task 10 wires USAGE.md into the index separately.
- No source changes.

## Other information
- If the section-2 rewrite changes any anchor that other docs link to (e.g.
  `#2-rfs-mc-ref---in-process-reference-model-checker`), keep the heading text
  stable so existing links (INTEGRATION.md section 8 links it) do not break;
  `docs_md_test` catches broken links.
