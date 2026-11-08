# Phase 12: LLM assistant development tooling

## Goal
Build a small set of Rust tools (`dev_tools/`) that perform the routine
maintenance operations an LLM assistant otherwise does by hand with shell
commands - sources and docs manipulation, context gathering, reference
re-syncing. The tool set is DERIVED FROM EVIDENCE: a systematic analysis of
`commands.md`, the recorded log of 12,596 assistant tool calls from the
Phases 0-11 sessions, identifies which operations are mechanical, repeated and
token-expensive; each such operation becomes a named, testable tool with a
stable CLI. Purpose: speed up repeated routine operations, save tokens (tools
emit compact structured results instead of the assistant reading whole files),
and secure the process (a compiled Rust tool is deterministic and robust where
an assistant-driven awk/sed/python one-liner is not).

## Rationale
Phases 10-11 demonstrated the cost pattern: inventory audits, import-site
rewrites, manifest re-syncs and doc-reference sweeps are all mechanical,
repeated, and error-prone when done as ad-hoc shell pipelines (the Phase 10
close-out audit needed a hand-written character-level masker; the Phase 11
re-sync is a scripted path mapping). The `commands.md` analysis quantifies the
pattern across the whole project history: of 12,596 recorded calls, ~61% of all
terminal calls are file-content lookup (grep/ls/find/head/tail/cat plus
`sed -n 'a,bp'` range reads and `dd` chunked reads), ~24% are cargo
build/test/run invocations, edit_file calls split roughly half sources / half
docs (with ~290 whole-new-file creations), and the 818 python calls are
dominated by scripted bulk rewrites (231), ad-hoc analysis (228), file
transforms (181) and audit/mask scripts (99). Each of those operation classes
becomes a tool - the assistant invokes one command and reads a compact result
instead of burning context on greps, scripts and full-file reads.

## Scope
1. Create the `dev_tools/` directory as a Rust crate - decided at planning:
   WORKSPACE MEMBER (root `Cargo.toml` members list), binary name `rfs-dev`,
   whose dependency set is a SUBSET of the RFS project's dependencies (no new
   deps), with exactly two exceptions:
   - Rust-source parsing: `syn` (+ its `proc-macro2` token layer) - the standard
     library for extracting function names/signatures, types and use-declarations
     from Rust sources; required by the docs<->sources sync tools and the
     context-extraction tool. (RFS's own nom/chumsky parsers parse RFS specs,
     not Rust - they do not serve this purpose.)
   - Markdown parsing: `pulldown-cmark` - the standard Rust CommonMark parser;
     required by the docs-section extraction and doc-structure tools. (The
     planning prompt's `wooorm/markdown-rs` is a JavaScript package; its Rust
     counterpart is pulldown-cmark, with `comrak` as the fallback if a mutable
     GFM AST becomes necessary - decided at planning: pulldown-cmark.)
2. ANALYZE `commands.md` (the recorded log of all 12,596 assistant tool calls
   from the Phases 0-11 sessions) and store the result in
   `dev_tools/README.md`, section `## Analysis of tool calls patterns`: a
   table of tool-call CATEGORIES with the ABSTRACT PATTERNS of the calls in
   each category (counts, shares, representative examples, and the implication
   of each category for the maintenance tooling needed and the shape of its
   CLI). Categories must at minimum separate: file-content lookup (docs vs
   sources), edits (docs vs sources), test/build execution, file operations
   (new/move/delete/...), plus whatever further categories the data shows
   (git, shell-writes, env-checks, scripted python operations, ...). The file
   is 13 MB - the analysis is split into smaller tasks: extraction of a
   structured call inventory, per-call-kind classification, consolidation.
3. Propose the needed categories of tools and their functionalities, elaborated
   in `dev_tools/README.md`, section `## Proposed tools` (table: Tool name | CLI
   | Purpose | Detailed description and rationale). The proposal builds on the
   analysis (step 2) and covers at least:
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
     to SRC_STRUCTURE.md (whether to build it is decided at planning, task 4).
   - **Inventory maintenance**: add/update/remove rows in SRC_STRUCTURE.md /
     TEST_STRUCTURE.md keeping the tables mechanically valid, and re-run the
     string-masked completeness audit as a built-in check (the Phase 10/11
     audit, productized).
   - plus any further tool the analysis (step 2) shows to be needed (e.g. the
     call-inventory extractor itself, so that `dev_tools/PROTOCOL.md` future
     re-runs are one command).
4. Implement the proposed tools in Rust per the implementation plan
   (`PHASE_12_PLAN.md`); each tool is exercised on a real maintenance operation
   as its acceptance test.

## Deliverables
- `dev_tools/` Rust crate (workspace member, binary `rfs-dev`) with the
  dependency constraint above.
- `dev_tools/analysis/extract_calls.py`: the committed commands.md extractor;
  its output `dev_tools/analysis/calls.jsonl` is gitignored (regenerable).
- `dev_tools/README.md` with BOTH sections filled: `## Analysis of tool calls
  patterns` (category table covering all 12,596 recorded calls) and
  `## Proposed tools` (Tool name | CLI | Purpose | Detailed description and
  rationale), matching what is implemented.
- Working implementations + tests per `PHASE_12_PLAN.md`, each tool
  demonstrated on a real maintenance operation.

## Exit criteria
- [ ] `dev_tools/` builds with no dependency outside the RFS set + syn +
      pulldown-cmark (or the planning-declared fallback).
- [ ] `dev_tools/README.md` carries the full `## Analysis of tool calls
      patterns` section: every one of the 12,596 recorded calls is accounted
      for in a category (counts sum to the total), each category has its
      abstract pattern and its implication for the tooling.
- [ ] `dev_tools/README.md` carries the full `## Proposed tools` table (Tool
      name | CLI | Purpose | Detailed description and rationale) and matches
      what is implemented.
- [ ] Every implemented tool has a passing demonstration on a real maintenance
      operation (e.g. context extraction for a named function yields the
      markdown template; the sync checker reports clean on the current tree).
- [ ] Full verification green: `cargo test` baseline, `cargo clippy
      --all-targets`, `cargo fmt --check` - dev_tools included (new baseline
      counts recorded in the phase status report).
- [ ] No RFS toolchain functionality, CLI or codegen output changed (dev_tools
      is maintenance-only; nothing in the rfs build/test path invokes it).

## Non-goals
- No changes to the RFS toolchain itself (parser/validator/codegen/MC/CLI).
- No GUI, no network access from any tool, no shell-out where a Rust library
  suffices.
- No restructuring of docs/ or src/ - tools operate on them, they do not move
  them (that is the refactoring phase's job, consuming these tools).
- `dev_tools/PROTOCOL.md` (the protocol for FUTURE analysis and update of the
  tool set, re-run on new `commands.md` versions) is written after planning and
  is NOT part of this phase's task set.
- This file is the frozen phase description; the implementation plan is
  `PHASE_12_PLAN.md` (written 2026-11-07, pending user review).
