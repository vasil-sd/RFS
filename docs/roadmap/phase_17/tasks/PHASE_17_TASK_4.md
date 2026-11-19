# Phase 17, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make the global usage text in `src/cli/args.rs::usage()` match the shipped
three-mode mc-ref CLI. Today the global usage (printed by no-argument `rfs` and
on unknown subcommand) shows the old exact-only mc-ref signature and help
paragraph, contradicting `rfs mc-ref`'s own usage text.

## Context
- The stale content lives in ONE string literal: `src/cli/args.rs::usage()`
  (starts ~line 15). Two parts are stale: the mc-ref USAGE LINE (`rfs mc-ref
  <spec.rfs|spec.json> [--max-states N] [--timeout-sec N] [--assumes all|none|
  labels] [--counterexample-format json|text] [--log-level ...]`) and the
  mc-ref HELP PARAGRAPH (~line 63: "Exhaustive exact search per SEMANTICS.md
  section 9 ... NOT applicable (documented in `rfs mc-ref --help`): --seed/
  --steps/--violate (fuzz-mode concepts), --threads/--fast ..., --nondet ...").
- The authoritative replacement content is the usage text already shipped in
  `src/cli/commands/mc_ref.rs` (~line 285): the three-mode signature line + the
  per-mode paragraphs. Re-flow it to fit the global usage's indentation style
  (continuation lines aligned under the command name) - do not invent new
  wording where the command's own text exists.
- VERIFIED: no test pins this string or its mc-ref "NOT applicable" fragment
  (grep over `tests/` found no assertion on the global usage body; only
  per-command behaviour pins exist, e.g. `mc_ref_flag_errors_are_usage_failures`
  in `tests/utils_test.rs`, which tests flag-error exit codes - preserved).
- This is a printed-text-only change: no parsing, dispatch or exit-code
  behaviour may change.

## Scope

### Affected source files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `src/cli/args.rs` | change | the usage() string literal: mc-ref line + help paragraph |

## Planned actions

### Update the mc-ref usage line and help paragraph
In `usage()`: replace the mc-ref signature line with the three-mode form
(`rfs mc-ref <spec.rfs|spec.json> [--mode exact|fuzz|smt-bounded]` + the
per-mode flag groups, matching `src/cli/commands/mc_ref.rs`'s own usage text
and re-flowed to the global style). Replace the mc-ref help paragraph with the
per-mode description: exact (default) with `--nondet`; fuzz with its flags and
0/1 exit codes; smt-bounded with `--bound` only, the assumptions/on_error
rejection and the `smt` feature requirement; shared `--assumes` + `--log-level`;
not-applicable reduced to `--threads`/`--fast` (single-threaded in-process).
Leave every other command's line and paragraph byte-for-byte untouched.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `src/cli/args.rs` | change (~line 15 usage line; ~line 63 help paragraph) | the global usage is user-facing documentation embedded in the binary - it must not contradict the command's own usage |

### Verify behaviour preservation
Run `cargo run -q` (no args) and `cargo run -q -- definitely-not-a-command`:
both print the updated usage and exit 1. Run `cargo run -q -- mc-ref` (no
spec): the command's OWN usage is unchanged. Then the full suite.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (verification only) | - | proves the edit changed printed text and nothing else |

## Task deliverables
- `src/cli/args.rs::usage()` showing the three-mode mc-ref signature + help,
  consistent with `rfs mc-ref`'s own usage text.

## Task exit criteria
- [ ] No-argument `rfs` output shows the three-mode mc-ref line (spot-check by
      running it).
- [ ] The mc-ref help paragraph no longer lists `--seed`/`--violate`/`--nondet`
      as not applicable.
- [ ] All other commands' usage lines are byte-identical to before (diff the
      string literal).
- [ ] Full checklist green: `cargo build && cargo test && cargo clippy
      --all-targets && cargo fmt --check`.

## Non-goals
- No change to `src/cli/commands/mc_ref.rs`'s own usage text (it is the source
  of truth).
- No test additions or migrations (none pin this string - verified).
