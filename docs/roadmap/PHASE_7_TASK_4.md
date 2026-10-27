# Phase 7, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`,
> `docs/roadmap/INTRO.md` and this file, the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal

The cross-renderer syntax parity suite (D-P7-10): the user-required tests
against external tools, consolidated in a dedicated file
`tests/diagram_render_parity_test.rs` mirroring the `backend_parity_test.rs`
naming pattern. Comparison is SYNTAX CORRECTNESS ONLY (parse/render success),
never visual output.

## Context

- Phase 7 plan: rows 4, 4.1-4.2; decision D-P7-10.
- Tasks 1-3 landed in-process rendering for all three formats (mmd/puml/dot);
  this task consolidates the internal-vs-external comparison in one dedicated
  suite.
- External tools are availability-gated: the external half runs only when the
  real tools are on PATH (installed per TOOLCHAIN.md 6.1 in the execution
  session: dot 2.43.0, plantuml 1.2020.02, mmdc 12.0.0 bundling mermaid 12.2.0).
- All external-tool invocations (and every env mutation) serialize under a
  static mutex - mmdc/Chrome must not run concurrently (TOOLCHAIN.md 6.4), same
  pattern as frontend_test's DIAG_ENV_LOCK.

## Scope

### In scope
- `tests/diagram_render_parity_test.rs` (new): the full parity matrix +
  unresolvable-tools e2e + print-script internal pins + negative controls.
- BUILD.md: section 3 new row for the parity suite; section 5 checklist note
  that the real-tool tests are availability-gated opt-in checks.
- TOOLCHAIN.md section 6: in-process default for all three formats, tools now
  optional, crate-dependency notes (incl. the plantuml-core pinned rev and
  offline-rebuild behaviour).

### Out of scope
- Any visual-output comparison (explicitly D-P7-10).
- Format documentation entries (tasks 5-14).

## Planned actions

1. **Parity test matrix** (`tests/diagram_render_parity_test.rs`): per format x
   {internal: all 50 valid examples; external: pinned ~4-spec subset exercising
   guards/emits/on_error/payloads, availability-gated, serialized under a static
   mutex} - assert parse/render success only. Plus: unresolvable-tools e2e for
   all three formats in one place (zero-external-dep proof), `--print-script`
   internal-mode pin (heredoc + comment, no tool line), and the three
   library-level negative controls.
2. **Docs** (`BUILD.md`): section 3 new row for the parity suite; section 5
   checklist note that the real-tool tests are now availability-gated opt-in
   checks.

#### Affected files

| File | Operation |
|---|---|
| `tests/diagram_render_parity_test.rs` | new - 13-test parity suite (internal sweep, external pinned subset, unresolvable-tools e2e, print-script pins, negative controls) |
| `BUILD.md` | changed - section 3 diagram row names all three in-process renderers + new parity-suite row; counts re-pinned (root 479 / workspace 503); section 5 checklist note |
| `TOOLCHAIN.md` | changed - section 6: in-process default for all three formats, tools now optional, new 6.5 crate-dependency notes incl. the plantuml-core pinned rev and offline-rebuild behaviour |

## Task deliverables

- Dedicated parity suite proving internal and external renderers parse the SAME
  generated text (syntax correctness only).
- Zero-external-dep proof consolidated in one place.
- BUILD.md / TOOLCHAIN.md describe the final rendering architecture.

## Task exit criteria

- [x] Internal half: ALL 50 valid examples x dot/puml/mmd rendered in-process (DONE: `parity_internal_{dot,puml,mmd}_all_valid_examples`)
- [x] External half: pinned 4-spec subset (specs 04/19/40/48) x dot/plantuml/mmdc via `--renderer external`, availability-gated + serialized; source file rfs writes next to the SVG asserted byte-identical to the in-process generated text (DONE: `parity_external_{dot,puml,mmd}_pinned_subset`)
- [x] Unresolvable-tools e2e for all three formats (zero-external-dep proof) (DONE: `parity_unresolvable_tools_internal_all_formats_exit_zero`)
- [x] `--print-script` internal-mode structural pins (heredoc + crate comment, no tool line) (DONE: `parity_print_script_internal_{dot,puml,mmd}_pinned`)
- [x] Three library-level garbage-rejection negative controls (DONE: `parity_negative_control_{dot,puml,mmd}_rejects_garbage`)
- [x] Full test matrix green; BUILD.md baseline re-pinned (DONE: root 479 / workspace 503, parity suite 13/13 with the external half running against the real tools)

## Non-goals

- No visual-output comparison (D-P7-10).
- No generator changes - this task is test + docs only.

## Task corrections

None recorded.

## Other information

- **Pinned external-parity subset** (D-P7-10):
  `examples/valid/{04_tcp_handshake_like, 19_guard_complex, 40_on_error_recovery,
  48_usb_control_transfer}.rfs` - collectively exercising guards, emits,
  on_error, and event/output payloads.
- Task 16 closeout re-runs this suite as part of the full verification matrix.
- This file was reconstructed retroactively on 2026-10-27 after a protocol
  violation (work notes existed as `docs/roadmap/tasks/task_4.md`, since removed).
  Sources: commit `696339c` message, TASKS.md entry for task 4,
  `PHASE_7_PLAN.md`, BUILD.md baseline re-pins.
- Committed bundled with tasks 1-3 in `696339c` (2026-10-26). Verification at
  commit time: cargo build exit 0; root suite 479 pass / 0 fail; workspace 503
  pass / 0 fail; diagram_render_parity_test 13/13 with the external half running
  against the real tools; clippy zero warnings in every file the commit touched;
  fmt clean.
- Deliberately NOT committed (recorded in the commit message): `docs/TODO.txt`
  (pre-existing user notes, not a phase artifact) and `docs/.#TODO.txt` (broken
  vim swap symlink).
