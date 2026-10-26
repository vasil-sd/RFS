# Task 4 - cross-renderer syntax suite (internal vs external tools)

## Subtasks

### 4.1 Parity test matrix (`tests/diagram_render_parity_test.rs`, new)
- [x] File created mirroring the `backend_parity_test.rs` naming pattern; self-contained helpers (tmp root, tool-on-PATH gate, env set/clear, static `TOOL_LOCK` mutex - same serialization pattern as frontend_test's DIAG_ENV_LOCK, required for mmdc/Chrome per TOOLCHAIN.md 6.4)
- [x] Internal half: ALL 50 valid examples (dynamic enumeration incl. `tla_adapted/`) x dot/puml/mmd - generated text fed directly to each in-process renderer; assert render success + `<svg` (syntax correctness only, D-P7-10)
- [x] External half: pinned 4-spec subset x dot/plantuml/mmdc via `rfs diagram --renderer external`, availability-gated + serialized. Pinned subset (guards/emits/on_error/payloads): `04_tcp_handshake_like` (guards + event payloads), `19_guard_complex` (compound guard over payload fields + emit + set), `40_on_error_recovery` (on_error fallback with emits), `48_usb_control_transfer` (rich payloads + many outputs)
- [x] Same-text guarantee: the source file rfs writes next to the SVG is asserted BYTE-IDENTICAL to the in-process generated text - both renderers provably parsed the same text
- [x] Unresolvable-tools e2e for all three formats in one place (RFS_DOT/RFS_PLANTUML/RFS_MERMAID all bogus -> default internal render still exit 0 + SVG)
- [x] `--print-script` internal-mode structural pins per format (heredoc marker + "in-process, no external tool" comment naming the crate + absence of the tool invocation line)
- [x] Three library-level negative controls consolidated (corrupted text rejected by each in-process renderer with non-empty diagnostics; mmd targets a missing header per plan correction 1.3/1.4)

### 4.2 Docs (`BUILD.md`)
- [x] Section 3: new row for `cargo test --test diagram_render_parity_test` (13 pass + matrix description); root count re-pinned **479 pass** (+13) with the parity suite added to the breakdown; workspace re-pinned **503 pass**
- [x] Section 5 checklist: new bullet - touched diagram generation/rendering -> run the parity suite; external half documented as an availability-gated opt-in check (dot/plantuml/mmdc per TOOLCHAIN.md section 6)

## Status

COMPLETE (2026-10-26).

**Pinned external-parity subset:** `examples/valid/{04_tcp_handshake_like, 19_guard_complex, 40_on_error_recovery, 48_usb_control_transfer}.rfs` - collectively exercising guards, emits, on_error, and event/output payloads (D-P7-10).

Verification: new suite **13 pass / 0 fail** with all three external tools installed (dot 2.43.0, plantuml 1.2020.02, mmdc 12.0.0/mermaid 12.2.0 - the external half RAN, not skipped); full root suite **479 pass / 0 fail**; workspace **503 pass / 0 fail**; clippy zero warnings in the new file; `cargo fmt --check` clean.

Notes for later tasks:
- The user-required "tests against external tools" (Phase 7 item 1) are now consolidated here: the same generated text must parse in BOTH the internal library and the external tool, syntax success only - never visual comparison (D-P7-10).
- Tasks 5+ (`rfs info format`) do not touch this file; task 16 closeout should re-run it as part of the full matrix.
