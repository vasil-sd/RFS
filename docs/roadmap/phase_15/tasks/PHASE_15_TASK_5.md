# Phase 15, Task 5 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Give `BUILD.md` an explicit section on building, testing and running RFS with
Nix - the canonical reference counterpart to TOOLCHAIN.md's new §10 (which
covers installation/state; this section covers day-to-day build/test/run).

## Context
- **Insertion point:** BUILD.md is 314 lines; the current last section is
  `## 7. Benchmarks in CI (Phase 6 task 6)` starting at line 252 and running to
  EOF. The new section is `## 8. Building, testing and running RFS with Nix`
  appended after line 314 - a pure append, no renumbering.
- **Content requirements (from PHASE_15.md exit criteria):** how to build the
  package (`nix build --impure --expr 'import ./nix/package.nix { }'` with
  checks; the doCheck=false variant for speed), how to pass custom tool
  versions as package arguments (the twelve-argument signature from task 2,
  with a concrete example overriding gcc and enabling the optional diagram
  tools), what the wrapper sets (the RFS_* contract table - reference it, do
  not duplicate it wholesale; point at TOOLCHAIN.md §8.2 for the full resolver
  contract) and that user redefinition always wins (`--print-script` as the
  inspection tool), how to use the dev shell (`nix develop -f shell.nix -c
  '<cmd>'` with the per-suite command list from task 4's verification matrix,
  incl. the nightly Rust fuzz pipeline), and a bare-package-vs-shell matrix
  (what works from the installed package alone vs what needs the shell: the
  Rust fuzz pipeline's nightly, and anything requiring rustup).
- **Style:** match BUILD.md's existing register - short tables + copy-paste
  command blocks, no narrative. The section must be accurate to what tasks
  2-4 actually shipped (write it AFTER they are done; if a detail differs from
  this file's expectations, follow the implemented reality and note the drift in
  the status file).
- **Cross-references:** point at TOOLCHAIN.md §10 (install/state), §8
  (portability contract), `nix/package.nix` + `shell.nix` themselves. Do NOT
  reference flake commands (non-goal - no flake.nix).
- **Docs-sync obligation:** BUILD.md is a command/toolchain doc - per START.md
  §8 it must stay synchronized; this task IS that sync for the nix surface.
  No `//! Docs:` lines or inventory rows are affected (no .rs changes), so
  `dev_docs_test` stays green by construction - re-run it in task 6 as proof.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | package.nix parameterized dependencies | the section documents the argument signature and closure behavior this task ships |
| 3. | package.nix wrapper script | the section documents the wrapper's env-var behavior and override semantics |
| 4. | shell.nix full-suite coverage | the section documents the dev-shell command surface and suite matrix |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 6. | Phase close-out | verifies the doc is accurate against the shipped state |

### Affected documentation files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `BUILD.md` | change (append after line 314) | new §8: package build, tool overrides, wrapper behavior, dev shell usage, bare-package-vs-shell matrix |

## Planned actions

### Write the new §8 section
Structure (subsections under `## 8. Building, testing and running RFS with Nix`):
1. `### 8.1 Prerequisites` - one line: nix on this sandbox per TOOLCHAIN.md §10
   (PATH export + `--impure`; single-user mode, no daemon needed).
2. `### 8.2 Building the package` - the two build commands (with/without
   checks), what the built closure contains (the five binaries + full tool
   closure per task 2), and a custom-tools example:
   `nix build --impure --expr 'import ./nix/package.nix { gcc = pkgs.gcc14; mermaid-cli = pkgs.mermaid-cli; }'`
   (or the actual argument names as shipped).
3. `### 8.3 Running the wrapped binary` - what `rfs-env.sh` sets (short table:
   variable -> purpose, one line each), the user-redefinition-wins rule with a
   `--print-script` inspection example, and the bare-install proof commands
   (`rfs test --suite unit` both backends, `--suite fuzz --backend c`).
4. `### 8.4 The dev shell (full test suite)` - `nix develop -f shell.nix`
   entry, the per-suite command table mirroring task 4's verification matrix
   (workspace tests, smt feature, generated-code, rfs test unit/fuzz both
   backends, example projects, diagram parity with external half active, docs
   gates), the nightly note (first entry downloads it; Rust fuzz pipeline needs
   it), and the mmdc-serialization gotcha pointer (TOOLCHAIN.md §6.5).
5. `### 8.5 Bare package vs dev shell` - a two-column matrix: works from the
   installed package alone (unit suites both backends, C fuzz, SMT mode,
   external diagram rendering IF the optional args were passed at build time)
   vs needs the dev shell (Rust fuzz nightly pipeline, cargo-driven test
   suites, rfs-dev).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `BUILD.md` | change (append at EOF, after line 314) | the new §8 - makes Nix a first-class documented build/test/run path per the phase goal |

## Task deliverables
- `BUILD.md` §8 with all five subsections, accurate to the shipped nix state.

## Task exit criteria
- [ ] `BUILD.md` contains `## 8. Building, testing and running RFS with Nix`
      with subsections 8.1-8.5 covering: prerequisites pointer; package build
      commands + custom-tool-argument example; wrapper env-var behavior +
      override-wins rule + `--print-script` inspection; dev-shell per-suite
      command table incl. nightly note; bare-package-vs-shell matrix.
- [ ] Every command in the section was actually executed and works as written
      (copy-paste verified).
- [ ] No flake references; cross-references to TOOLCHAIN.md §8/§10 present.
- [ ] `./target/debug/rfs-dev doc-section BUILD.md '8. Building, testing and running RFS with Nix'`
      returns the section (exit 0).
- [ ] Committed per FORMATTING.md §7.

## Non-goals
- No changes to any other BUILD.md section (pure append).
- No TOOLCHAIN.md changes (task 1's territory).
- No flake documentation (no flake.nix exists - phase non-goal).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Custom-tools example binds `pkgs` first | The plan's suggested `nix build --impure --expr 'import ./nix/package.nix { gcc = pkgs.gcc14; … }'` is not copy-paste valid: a bare `--expr` has no `pkgs` in scope ("undefined variable 'pkgs'"). The working form is `let pkgs = import <nixpkgs> {}; in import ./nix/package.nix { gcc = pkgs.gcc13; mermaid-cli = pkgs.mermaid-cli; }`. A trailing `;` after the top-level expression is also rejected by Nix (same rule as task 4's z3-unified.nix lambda). Documented the correct form + why the bare form fails. |
| One-shot dev-shell form is `--command sh -c '…'`, not `-c '<string>'` | nix execs a bare string as a single program name ("exec: … not found"). The section uses `--command sh -c '…'` throughout and says so explicitly. |

## Other information
- Write this task LAST among the doc tasks: it describes shipped behavior, so
  execute it only after tasks 2-4 are committed. If implementation drifted from
  the plan (e.g. D3 fallback duplication instead of the shared file), document
  the reality, not the plan.
