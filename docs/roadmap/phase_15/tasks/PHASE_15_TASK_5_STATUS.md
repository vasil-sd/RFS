# Phase 15, Task 5 - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_5.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| - | none (documentation only) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| - | - | none (no .rs changes - the nix surface is packaging + docs only) |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `BUILD.md` | extended (pure append after line 314) | new `## 8. Building, testing and running RFS with Nix` with subsections 8.1 Prerequisites (TOOLCHAIN.md §10 pointer + PATH export), 8.2 Building the package (with/without-checks commands, what the closure carries, the 12-argument signature + a working custom-tools example), 8.3 Running the wrapped binary (rfs-env.sh variable table, override-wins rule with --print-script inspection, env -i bare-install proofs), 8.4 The dev shell (per-suite command table mirroring task 4's matrix, the nightly note, the Rust-fuzz-via-unwrapped+proxy invocation, the sandbox TMPDIR/CARGO_TARGET_DIR gotchas, mmdc-serialization pointer to TOOLCHAIN.md §6.5), 8.5 Bare package vs dev shell (two-column matrix). |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Changed functionality
- None (documentation only).

## Removed functionality
- None.

## New functionality
- BUILD.md now documents Nix as a first-class build/test/run path, the canonical
  counterpart to TOOLCHAIN.md §10 (install/state) and §8/§8.2 (portability contract).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no test files changed; `dev_docs_test` stays green by construction (no .rs changes, no new //! Docs: lines) - re-run in task 6 as proof |

## Tests run status

| Name | Status |
|---|---|
| `./target/debug/rfs-dev doc-section BUILD.md '8. Building, testing and running RFS with Nix'` | OK - returns the section, exit 0 |
| §8.2 build commands (with checks / doCheck=false) | OK - both build; doCheck=false is the fast path |
| §8.2 custom-tools example (`let pkgs = …; in import ./nix/package.nix { gcc = pkgs.gcc13; mermaid-cli = pkgs.mermaid-cli; }`) | OK - RFS_GCC points at gcc-13.5.0, only RFS_MERMAID set (only mermaid-cli passed) |
| §8.3 --print-script override-wins | OK - no preset shows the store gcc; RFS_GCC=/usr/bin/gcc preset shows /usr/bin/gcc (user wins) |
| §8.3 env -i bare-install proofs (unit rust, unit c, fuzz c) | OK - all exit 0 with a clean environment |
| §8.4 per-suite commands (workspace, smt, generated-code, unit both backends, fuzz c, example projects, diagram parity external-active, docs gates) | OK - all green under nix develop (task 4 matrix); docs gates re-confirmed 9/0 + 5/0 |
| §8.4 Rust fuzz via unwrapped rfs.real + proxy on PATH | OK - exit 0, "fuzz target ran within its budget without crash (rust backend)" |

## Found problems/issues/mismatches
- **The task's suggested custom-tools command is not copy-paste valid.** The plan
  wrote `nix build --impure --expr 'import ./nix/package.nix { gcc = pkgs.gcc14; … }'`,
  but a bare `--expr` has no `pkgs` in scope, so that is an "undefined variable 'pkgs'"
  error. The working form binds it first: `let pkgs = import <nixpkgs> {}; in import
  ./nix/package.nix { gcc = pkgs.gcc13; mermaid-cli = pkgs.mermaid-cli; }`. Documented
  the correct form (and noted why the bare form fails). Also dropped a trailing `;`
  after the top-level expression - Nix rejects it (same rule as task 4's z3-unified.nix
  lambda).
- **`nix develop -c '<string>'` execs the string as one program name** ("exec: … not
  found"); the working one-shot form is `--command sh -c '…'`. Documented that explicitly
  so a reader does not hit the same wall.

## Other important findings/information
- The section documents SHIPPED behavior from tasks 2-4, verified by execution: the
  12-argument signature, the wrapper's rfs-env.sh variable set (default build has zero
  diagram vars; optional-args builds set exactly the passed ones), and the dev-shell
  suite matrix. The Rust-fuzz-via-wrapped-binary limitation (task 4's plan correction)
  is carried into §8.4/§8.5 accurately - the pipeline works through the unwrapped binary
  + rustup proxy, not the wrapped one.
- No flake references anywhere in the section (the `--impure` rationale is given as
  "channel-profile nixpkgs" instead). Cross-references to TOOLCHAIN.md §10 (install),
  §8/§8.2 (resolver contract) and §6.5 (mmdc serialization) are present.
