# Phase 15, Task 3 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make the RFS package export a wrapper around ALL FIVE of its binaries that sets
every required `RFS_*` environment variable (plus the z3-sys build vars and
`PKG_CONFIG_PATH`) - each set ONLY when the user has not already defined it,
so user redefinition always wins. This makes `rfs test ...` deterministic from
a bare install and eliminates the recorded "clang answering for gcc"
tool-resolution artifact.

## Context
- **The env-var contract (from `src/cli/ext_tool.rs` presets +
  `tests/common/toolchain.rs`, precedence CLI flag > `RFS_*` > standard var >
  bare PATH / pkg-config):** the wrapper must conditionally export
  `RFS_RUSTC` (std tier `RUSTC`), `RFS_GCC` (std tier `CC`), `RFS_CLANG`
  (deliberately NO std tier - `CC` must not leak into the fuzzer compiler),
  `RFS_CARGO_FUZZ` (std tier `CARGO`), `RFS_MAKE`, `RFS_Z3_DIR` (std tier
  `Z3_SYS_Z3_DIR`; discovery chain `RFS_Z3_DIR` > `Z3_SYS_Z3_DIR` >
  `pkg-config --variable=prefix z3` > `/workspace/host/z3` - the last tier is
  absent under nix, so one of the first three MUST resolve), plus for z3-sys at
  user build time `Z3_SYS_Z3_DIR=<unified>`,
  `Z3_LIBRARY_PATH_OVERRIDE=<unified>/bin` (libz3.so is symlinked into bin/ by
  unifiedZ3) and `PKG_CONFIG_PATH` PREPENDED with `<unified>/pkgconfig`. When
  the optional package arguments are non-null, additionally `RFS_DOT`,
  `RFS_PLANTUML`, `RFS_MERMAID` (each pointing at the passed package's bin).
- **Why a PATH prepend as well as RFS_* exports:** running
  `/nix/store/xxx-rfs-1.0.0/bin/rfs` directly does NOT put the propagated
  dependencies' bin dirs on PATH (that only happens in `nix develop` shells or
  assembled user profiles). The wrapper therefore (a) prepends the closure's
  tool bin dirs to PATH - which also lets `cargo fuzz` find the `cargo-fuzz`
  subcommand (cargo looks next to its own executable AND on PATH) - and (b)
  sets the RFS_* vars for deterministic resolution per the precedence contract.
  Belt and suspenders; both are store-absolute so they are stable.
- **FIVE binaries, not one:** the package's bin/ holds `rfs` plus the four
  cargo bins `build_training`, `extract_training_snippets`,
  `generate_code_tests`, `test_generated_code`. The two code-test bins invoke
  rustc/gcc themselves, so ALL FIVE get the same wrapper preamble (uniform
  mechanism - decision D2 below).
- **Mechanism decision D2 - shared preamble script:** generate ONE file
  `$out/bin/rfs-env.sh` in `postFixup` containing the PATH prepend + all
  conditional exports; replace each of the five binaries with a three-line
  wrapper (`#!/bin/sh`, source the preamble via its own directory, exec the
  real binary renamed to `<name>.real`). Rationale over makeWrapper `--run`:
  the conditional-export logic lives in exactly one visible place instead of
  being duplicated five times, and adding a variable later touches one block.
  (makeWrapper `--set` is NOT usable - it overrides unconditionally and would
  break user redefinition.)
- **Conditional-export form:** `[ -z "${RFS_GCC:-}" ] && export RFS_GCC=${gcc}/bin/gcc`
  for every variable; `PKG_CONFIG_PATH` uses the prepend idiom
  `export PKG_CONFIG_PATH="${unifiedZ3}/pkgconfig${PKG_CONFIG_PATH:+:$PKG_CONFIG_PATH}"`
  guarded by the same unset test on a marker (or simply always prepend - it is
  idempotent-harmless; decide and document in the status file).
- **DO NOT TOUCH the checkPhase z3 logic:** package.nix's checkPhase
  deliberately leaves `RFS_Z3_DIR`/`Z3_SYS_Z3_DIR` UNSET and only exports
  `PKG_CONFIG_PATH`, so z3-sys reads the real version (5.3.2) from headers with
  `explicit=false`. The wrapper is a RUNTIME concern; mixing the two would
  re-pin the solver version and break SMT parity.
- **The artifact this fixes:** TASKS.md (Phase 14 task 8 close-out, line ~2463)
  records `nix develop -f shell.nix` failing because `ref_mc_diff_test`
  panicked on a CLANG-style warning attributed to `gcc -Wall` - "clang
  answering for gcc". With `RFS_GCC` explicitly set to the real gcc store path,
  resolution is deterministic; action 4 re-runs that suite under nix to prove
  it.
- **mmdc as root** needs `--no-sandbox`: RFS always passes its generated
  puppeteer config (overridable via `RFS_MERMAID_PUPPETEER_ARGS`), so nothing
  extra is needed in the wrapper for mermaid-cli's wired chromium.
- **Nix invocation:** `export PATH=/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin:$PATH`
  + `--impure` (see task 2 Context).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | package.nix parameterized dependencies | the wrapper references the buildInputs store paths (gcc/clang/rustc/cargo/make/unifiedZ3/pkg-config/cargo-fuzz + optional diagram tools) that task 2 puts in place |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 4. | shell.nix full-suite coverage | the shell inherits the wrapped package via inputsFrom/packages and relies on the wrapper's user-redefinition semantics |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `nix/package.nix` | change | postFixup generating `rfs-env.sh` + five thin wrappers replacing the raw binaries |

## Planned actions

### 1. Generate the shared preamble and wrap all five binaries
Add a `postFixup` (after the existing install/fixup of buildRustPackage) that:
1. writes `$out/bin/rfs-env.sh` - PATH prepend of the bin dirs of rustc, cargo,
   gcc, clang, make, unifiedZ3, pkg-config, cargo-fuzz (each
   `${tool}/bin`, deduplicated), then the conditional exports from Context for
   `RFS_RUSTC`, `RFS_GCC`, `RFS_CLANG`, `RFS_CARGO_FUZZ`, `RFS_MAKE`,
   `RFS_Z3_DIR`, `Z3_SYS_Z3_DIR`, `Z3_LIBRARY_PATH_OVERRIDE`, and the
   `PKG_CONFIG_PATH` prepend; plus - emitted ONLY when the corresponding
   argument is non-null - `RFS_DOT=${graphviz}/bin/dot`,
   `RFS_PLANTUML=${plantuml}/bin/plantuml`,
   `RFS_MERMAID=${mermaid-cli}/bin/mmdc` (each conditional on unset);
2. for each of the five binaries: `mv $out/bin/<name> $out/bin/<name>.real`,
   then writes a three-line wrapper sourcing `$out/bin/rfs-env.sh` (located via
   its own path, so the package works from any store location) and exec'ing
   `<name>.real` with all arguments.

The preamble is generated by the Nix expression (string interpolation of the
store paths at build time), so passing different tool arguments yields a
different preamble - no runtime detection needed.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `nix/package.nix` | change (after line 118, before `meta`) | new postFixup block: rfs-env.sh generation + five wrapper replacements; comment records D2 and the checkPhase-untouched rule |

### 2. Verify: bare-install operation with no env preset
From the built package alone (no shell, no exported vars):
1. `result/bin/rfs test --suite unit --backend rust examples/valid/01_*.rfs`
   and `--backend c` both exit 0 - proof that PATH prepend + RFS_* exports find
   rustc/gcc from the closure.
2. `result/bin/rfs test --suite fuzz --backend c <spec>` exits 0 with the
   closure's clang (libFuzzer runtime per task 2's probe).
3. SMT: compile a generated C MC artifact against the unified z3 using the
   wrapper's environment (`RFS_Z3_DIR` set, `pkg-config --variable=prefix z3`
   returns the unified dir) - mirrors TOOLCHAIN.md §5.3's recipe.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### 3. Verify: user redefinition always wins
1. Pre-set `RFS_GCC` to a DIFFERENT store gcc (e.g. the nixpkgs default if the
   package was built with a custom one, or vice versa), run
   `result/bin/rfs test --suite unit --backend c <spec> --print-script` - the
   printed script must show the USER's gcc path, not the wrapper's.
2. Same check for at least `RFS_RUSTC` and `RFS_Z3_DIR`.
3. Unset everything: the wrapper's values appear in `--print-script` (the
   default path).
4. Optional-args matrix: default build leaves `RFS_DOT`/`RFS_PLANTUML`/
   `RFS_MERMAID` unset (`result/bin/rfs diagram --format dot --render svg
   --renderer external` fails with the strict missing-tool diagnostic);
   a build with all three passed sets them and the same command renders.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only |

### 4. Verify: the clang-answering-for-gcc artifact is gone
Re-run the previously failing suite under nix with the wrapped environment:
`ref_mc_diff_test` (the Phase 14 task 8 finding) must pass 4/4 - no CLANG-style
diagnostic attributed to `gcc -Wall`. Run it via the package's checkPhase
(`nix build --impure --expr 'import ./nix/package.nix { }'`) and/or directly in
a nix shell with the wrapper's env.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only; outcome recorded in the status file |

## Task deliverables
- Wrapped package: five binaries each exec'ing through `rfs-env.sh`, which sets
  the full RFS_* contract conditionally and prepends the closure's tool bin
dirs to PATH.
- Verification matrix results (bare-install operation, override precedence,
  optional-args behavior, ref_mc_diff_test green) recorded in the status file.

## Task exit criteria
- [ ] `result/bin/rfs-env.sh` exists and contains conditional exports for
      RFS_RUSTC, RFS_GCC, RFS_CLANG, RFS_CARGO_FUZZ, RFS_MAKE, RFS_Z3_DIR,
      Z3_SYS_Z3_DIR, Z3_LIBRARY_PATH_OVERRIDE + PKG_CONFIG_PATH prepend; all
      five binaries are thin wrappers sourcing it.
- [ ] With NO environment preset: `rfs test --suite unit` passes for BOTH
      backends from the bare package; `--suite fuzz --backend c` passes;
      SMT-mode C compilation works against the unified z3.
- [ ] A pre-set user `RFS_GCC`/`RFS_RUSTC`/`RFS_Z3_DIR` is NOT overridden
      (proven via `--print-script`).
- [ ] Default build: RFS_DOT/RFS_PLANTUML/RFS_MERMAID unset; build with the
      optional args passed: all three set and external rendering works.
- [ ] `ref_mc_diff_test` passes under nix (artifact resolved).
- [ ] checkPhase z3 logic byte-unchanged; full doCheck build still green.
- [ ] Committed per FORMATTING.md §7.

## Non-goals
- No changes to the resolver source (`src/cli/ext_tool.rs`,
  `tests/common/toolchain.rs`) - the wrapper satisfies the existing contract,
  nothing is extended.
- No nightly toolchain in the package (Rust fuzz pipeline's nightly need stays
  a dev-shell concern - task 4).
- No `RFS_*_ARGS` variables set by the wrapper (args are user configuration,
  not tool locations).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Nix string-escaping idiom | The planned `$${VAR}` escaping does NOT exist in this nix (2.35.6): `$$` is not an escape sequence in indented OR regular strings (verified empirically - `$${x}` stays literal, while `${x}` interpolates and `\$` only protects a bare dollar). Runtime brace groups are therefore assembled with `db = "$" + "{"` and written as `${db}VAR...` - the interpolation emits `$` and the following text supplies the brace group. |
| Wrapper generation: printf -> quoted heredocs | The first draft used `printf '%s\n' ... "exec \"$dir/$name.real\" \"$@\""` - the double-quoted argument let the BUILD-time shell expand $dir (unset -> empty) and $@ (the derivation's own args), producing a one-line wrapper with literal \n and a mangled exec line. Replaced by five explicit `cat <<'WRAP'` blocks with the binary name hardcoded per block: a quoted heredoc keeps every runtime token ($0, $dir, $@, $(...)) literal, and a shell loop variable would be invisible to Nix anyway. |
| Missing closing braces in generated expansions | Four generated brace groups lost their `}` (the `${db}VAR` idiom makes the closing brace easy to drop): `${PATH"`, `${PKG_CONFIG_PATH`, and the three optionalString diagram lines (`${RFS_DOT:-"` etc.). Each caught by `sh -n` on the generated preamble after a rebuild; all fixed. |
| PATH-invocation fallback in wrappers | The planned wrapper computed `dir=$(dirname -- "$0")` only - but when invoked via PATH (the common case: nix-env profile / nix develop), $0 is bare, dirname yields `.`, and the wrapper sourced rfs-env.sh from the CWD. Added `[ -f "$dir/rfs-env.sh" ] || dir=$(dirname -- "$(command -v -- "$0")")` to all five wrappers; verified both backends pass via PATH invocation from /tmp and /. |

## Other information
- If postFixup turns out to run before some fixup that rewrites bin/ entries
  (unlikely for buildRustPackage), move the wrapping to a `postInstall` or a
  dedicated installPhase override - record whichever was used.
- The `.real` suffix is invisible to users; document it in the status file so
  a future session does not "clean up" the apparently-duplicate binaries.
