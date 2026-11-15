# Phase 15, Task 3 - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_3.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up
> phase plans.

## Added source files

| Name | Description |
|---|---|
| - | none (the wrapper is generated at build time by package.nix's postFixup) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `nix/package.nix` | changed | new `postFixup` (after checkPhase, before meta): generates `$out/bin/rfs-env.sh` (PATH prepend of the eight propagated tool bin dirs + optional diagram bins, then conditional exports of RFS_RUSTC/RFS_GCC/RFS_CLANG/RFS_CARGO_FUZZ/RFS_MAKE/RFS_Z3_DIR/Z3_SYS_Z3_DIR/Z3_LIBRARY_PATH_OVERRIDE + always-prepended PKG_CONFIG_PATH + optional RFS_DOT/RFS_PLANTUML/RFS_MERMAID) and replaces all five binaries with thin wrappers that source the preamble and exec `<name>.real`. Runtime brace groups use the `db = "$" + "{"` idiom (see corrections). checkPhase untouched. |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| - | - | none (BUILD.md's nix section is task 5) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| - | - | none |

## Changed functionality
- The installed package's five binaries (`rfs`, `build_training`,
  `extract_training_snippets`, `generate_code_tests`, `test_generated_code`)
  are now wrappers: they source `bin/rfs-env.sh` (which prepends the closure's
  tool bin dirs to PATH and sets the RFS_* contract vars ONLY when unset) and
  exec the real binary at `<name>.real`. A bare install therefore resolves
  every external tool deterministically; user-defined RFS_*/standard vars always
  win over the wrapper's values.

## Removed functionality
- None.

## New functionality
- Runtime tool environment for the installed package: PATH prepend (store bins
  are not on PATH otherwise; `cargo fuzz` needs the cargo-fuzz subcommand
  findable there) + conditional RFS_* exports + PKG_CONFIG_PATH prepend making
  the unified z3.pc resolvable. Optional diagram arguments (task 2) now also
  surface as RFS_DOT/RFS_PLANTUML/RFS_MERMAID in the preamble - only when
  passed.

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| - | - | no test files changed; verification is the nix-side matrix below |

## Tests run status

| Name | Status |
|---|---|
| `nix build` (doCheck=false, default args) | OK - wrapper + preamble generated, `sh -n` clean on all six scripts |
| bare install, no env presets: `rfs test --suite unit` rust + c (env -i) | OK - both backends pass |
| bare install: `rfs test --suite fuzz --backend c` (5 s budget) | OK - 772,735 runs, no crash |
| wrapper env only: `cargo test --features smt --test smt_bounded_test` | OK - 2/2; RFS_Z3_DIR + pkg-config resolve to the unified prefix |
| PATH invocation (bare $0) from /tmp and / | OK - both backends pass (fallback re-locates the preamble) |
| user preset RFS_GCC=/usr/bin/gcc -> `--print-script` | OK - shows /usr/bin/gcc (user wins); no preset -> store path; CC preset correctly loses to the wrapper's RFS_GCC (contract precedence) |
| optional-args build: preamble carries exactly RFS_DOT/RFS_PLANTUML/RFS_MERMAID | OK - default build has 0 diagram-var lines |
| optional-args build: `rfs diagram --render svg --renderer external` (mmd, env -i) | OK - 185,970-byte SVG via nixpkgs mermaid-cli + wired chromium |
| full `nix build` with doCheck=true | OK - 42/42 checkPhase tests pass; ref_mc_diff_test green (the recorded clang-answering-for-gcc artifact is gone) |

## Found problems/issues/mismatches
- Nix 2.35.6 string escaping differs from the task file's assumption: `$$` is
  NOT an escape in indented or regular strings (verified empirically), so the
  planned `$${VAR}` form would have emitted literal `$${VAR}` into the script.
  The `db = "$" + "{"` concatenation idiom was used instead (correction 1).
- Four generated brace groups lost their closing `}` during drafting
  (`${PATH"`, `${PKG_CONFIG_PATH`, three `${RFS_*:-"` in the optionalString
  lines) - each caught by `sh -n` after a rebuild (correction 3).
- The printf-based wrapper draft expanded $dir/$@ at BUILD time (double-quoted
  argument), producing a broken one-line wrapper - replaced with quoted
  heredocs (correction 2).
- PATH invocation (bare $0) initially sourced rfs-env.sh from the CWD - fixed
  with the command -v fallback (correction 4).

## Other important findings/information
- PKG_CONFIG_PATH is always prepended (not conditional): idempotent-harmless -
  the unified z3.pc only makes `pkg-config z3` resolvable, and user entries are
  preserved after it. Documented in the generated preamble.
- The PATH prepend means the package's tools win bare-name resolution even when
  the user's shell has other compilers earlier on PATH; RFS_* redefinition
  still wins for the resolver (contract: flag > RFS_* > standard var > PATH).
- checkPhase is unaffected by the wrapper by construction (it runs cargo test
  against the library, never through the wrapped binaries) - proven by the
  green 42/42 doCheck build.
