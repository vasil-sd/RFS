# Phase 12: LLM assistant development tooling

## Goal
Build a small set of Rust tools (`dev_tools/`) that perform the routine
maintenance operations an LLM assistant otherwise does by hand with shell
commands - sources and docs manipulation, context gathering, reference
re-syncing. Purpose: speed up repeated routine operations, save tokens (tools
emit compact structured results instead of the assistant reading whole files),
and secure the process (a compiled Rust tool is deterministic and robust where
an assistant-driven awk/sed/python one-liner is not).

## Rationale
Phases 10-11 demonstrated the cost pattern: inventory audits, import-site
rewrites, manifest re-syncs and doc-reference sweeps are all mechanical,
repeated, and error-prone when done as ad-hoc shell pipelines (the Phase 10
close-out audit needed a hand-written character-level masker; the Phase 11
re-sync is a scripted path mapping). Each of those operations becomes a named,
testable tool with a stable CLI - the assistant invokes one command and reads a
compact result instead of burning context on greps, scripts and full-file reads.

## Scope
1. Create the `dev_tools/` directory as a Rust crate (workspace member or
   standalone - decided at planning) whose dependency set is a SUBSET of the
   RFS project's dependencies (no new deps), with exactly two exceptions:
   - Rust-source parsing: `syn` (+ its `proc-macro2` token layer) - the standard
     library for extracting function names/signatures, types and use-declarations
     from Rust sources; required by the docs<->sources sync tools and the
     context-extraction tool. (RFS's own nom/chumsky parsers parse RFS specs,
     not Rust - they do not serve this purpose.)
   - Markdown parsing: `pulldown-cmark` - the standard Rust CommonMark parser;
     required by the docs-section extraction and doc-structure tools. (The
     planning prompt's `wooorm/markdown-rs` is a JavaScript package; its Rust
     counterpart is pulldown-cmark, with `comrak` as the fallback if a mutable
     GFM AST becomes necessary - decide at planning.)
2. Propose the needed categories of tools and their functionalities (the
   proposal is this phase's design deliverable; implementations follow the
   later plan):
   - **Docs<->sources sync**: check and update `//! Docs:` pointer lines,
     `tests/dev_docs_manifest.txt`, and every backticked source-path reference
     in `docs/**/*.md` against the actual tree; report stale/missing pointers as
     a compact fix-list (the Phase 11 task-10 operation, productized).
   - **Context extraction**: given a function/type name and optionally a source
     file name, pull from the inventories + docs: the row itself, its Depends
     closure (depth-bounded), the Documented-in fragments' content, related
     DECISIONS.md entries and rationale - formatted as a fixed markdown template
     (the template is part of the tool's output contract). This operationalizes
     the Phase 11 impact-calculation protocol rule.
   - **Token-efficient source editing**: Rust-aware structured edits - rename a
     symbol across files, move a file and rewrite its import sites, replace an
     item by name - parsed with syn instead of text patterns, so they cannot be
     broken by strings/comments containing the pattern (the exact failure class
     that produced Phase 10's template-phantom corrections).
   - **Docs section extraction**: retrieve any doc file's content by section
     name (heading path), optionally across all docs; backed by a proposed
     `DOCS_STRUCTURE.md` - a section-level inventory of the docs tree analogous
     to SRC_STRUCTURE.md (whether to build it is decided at planning).
   - **Inventory maintenance**: add/update/remove rows in SRC_STRUCTURE.md /
     TEST_STRUCTURE.md keeping the tables mechanically valid, and re-run the
     string-masked completeness audit as a built-in check (the Phase 10/11
     audit, productized).
3. Write `dev_tools/README.md` with a table of the proposed tools: name, CLI
   pattern, functionality, rationale, and concrete usage scenarios (which
   phase-operation each one replaces).
4. Implement the proposed tools in Rust per the later implementation plan; each
   tool is exercised on a real maintenance operation as its acceptance test.

## Deliverables
- `dev_tools/` Rust crate with the dependency constraint above.
- The tool-category proposal (this file's section 2, refined at planning).
- `dev_tools/README.md`: table of proposed tools - CLI pattern, rationale,
  usage scenarios.
- Working implementations per the (later) implementation plan, each demonstrated
  on a real operation.

## Exit criteria
- [ ] `dev_tools/` builds with no dependency outside the RFS set + syn +
      pulldown-cmark (or the planning-declared fallback).
- [ ] `dev_tools/README.md` carries the full proposed-tools table (CLI pattern,
      rationale, scenarios) and matches what is implemented.
- [ ] Every implemented tool has a passing demonstration on a real maintenance
      operation (e.g. context extraction for a named function yields the
      markdown template; the sync checker reports clean on the current tree).
- [ ] Full verification green: `cargo test` baseline, `cargo clippy
      --all-targets`, `cargo fmt --check` - dev_tools included.
- [ ] No RFS toolchain functionality, CLI or codegen output changed (dev_tools
      is maintenance-only; nothing in the rfs build/test path invokes it).

## Non-goals
- No changes to the RFS toolchain itself (parser/validator/codegen/MC/CLI).
- No GUI, no network access from any tool, no shell-out where a Rust library
  suffices.
- No restructuring of docs/ or src/ - tools operate on them, they do not move
  them (that is the refactoring phase's job, consuming these tools).
- The implementation plan (`PHASE_12_PLAN.md` + task files) is written later,
  at the Phase 12 planning trigger; this file is the frozen phase description.
