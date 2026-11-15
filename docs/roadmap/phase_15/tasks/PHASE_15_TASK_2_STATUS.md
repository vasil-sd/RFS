# Phase 15, Task 2 - Implementation status

> Written once when all exit criteria of `PHASE_15_TASK_2.md` are reached. Not
> rewritten afterwards; later fixes go into `TASKS.md` entries and follow-up phase plans.
>
> The doCheck criterion was pending on plan-correction task 2a (pre-existing
> zero-warning-gate violations in the current source); it is written now that
> 2a has landed and the full nix checkPhase build passes 42/42.

## Added source files

| Name | Description |
|---|---|
| (none) | packaging-only task - no Rust source files (task 2a, which this task unblocked, carries the phase's source changes) |

## Changed/removed source files

| Name | Operation | Description |
|---|---|---|
| `nix/package.nix` | changed | 12-argument signature (rustc/cargo/gcc/clang/make/z3/pkg-config/cargo-fuzz defaulting to nixpkgs packages; mermaid-cli/plantuml/graphviz optional null) with a header comment showing override usage; unifiedZ3 parameterized on the z3 argument (derivation name + all six references, explanatory comment preserved); tools moved from nativeBuildInputs to buildInputs with explicit `propagatedBuildInputs = buildInputs` (this stdenv does not auto-propagate - verified by evaluation); nativeBuildInputs emptied (buildRustPackage injects its own hermetic rustc/cargo 1.99.0 + cargo hooks internally - they are callPackage arguments, not public ones, so the build toolchain stays nixpkgs-default while user-passed tools land in the runtime closure - design decision D1); source filter excludes the nix `result` symlink (custom cleanSourceWith filters do not honor .gitignore) |
| `.gitignore` | changed | `result` ignored (one line + comment) for humans and other tools |

## Added/changed/extended documentation files

| Name | Type | Description |
|---|---|---|
| `docs/roadmap/phase_15/tasks/PHASE_15_TASK_2.md` | changed | four task corrections recorded (explicit propagatedBuildInputs; closure-proof method replaced; result-symlink filter fix; doCheck pending on 2a) |

## Other file changes

| Name | Operation | Description |
|---|---|---|
| (none) | - | - |

## Changed functionality
- The installed RFS package now carries its complete external-tool closure: a nix-env install or an inputsFrom shell puts rustc, cargo, gcc, clang, make, the unified z3 prefix, pkg-config and cargo-fuzz on PATH alongside the five rfs binaries. Before this task the runtime closure held zero tool binaries (7 paths total - libgcc/glibc/runtime only), so `rfs test ...` from a bare install found no compiler.
- Package arguments are user-overridable: passing e.g. `gcc = pkgs.gcc14` or the optional diagram tools changes the propagated closure accordingly.

## Removed functionality
- None (the build-time availability of the tools in checkPhase is unchanged - buildInputs are on PATH during the whole build).

## New functionality
- Parameterized, propagated tool dependencies with optional diagram-tool arguments; deterministic package hash across rebuilds (result-symlink exclusion).

## Tests add/change/remove

| Name | Operation | Description |
|---|---|---|
| (none) | - | verification was nix builds + closure evaluations + a functional installed-environment simulation; no test files touched |

## Tests run status

| Name | Status |
|---|---|
| `nix build` doCheck=false, twice consecutively with `result` present | OK - identical store path both times (hash stable) |
| propagatedBuildInputs evaluation (default) | OK - exactly rustc, cargo, gcc-wrapper, clang-wrapper, gnumake, z3-unified-5.3.2, pkg-config-wrapper, cargo-fuzz; NO chromium/nodejs |
| propagatedBuildInputs evaluation (mermaid-cli + plantuml + graphviz passed) | OK - adds exactly those three packages |
| Functional installed-environment simulation (`env -i`, PATH = package + propagated inputs only): `rfs test --suite unit` both backends on examples/valid/01_trivial_two_state.rfs | OK - rust backend "all generated rust tests passed", C backend "all generated c tests passed" (exit 0 each) |
| libFuzzer probe with the closure's clang 21.1.8 | OK - trivial fuzzer target compiled with `-fsanitize=fuzzer,address` and bounded-ran 4,412,612 runs in 3 s, exit 0 (the C fuzz pipeline works from the closure) |
| Full `nix build --impure --expr 'import ./nix/package.nix { }'` (doCheck=true) | OK - **42/42** checkPhase tests pass across the 11 suites (after task 2a fixed the pre-existing source violations that blocked this criterion) |

## Found problems/issues/mismatches
- This nixpkgs' stdenv does NOT auto-propagate buildInputs (evaluated `propagatedBuildInputs = []` with them set) - an explicit declaration was required; recorded as a task correction.
- The planned `nix path-info -r` closure proof was flawed: it traces file references inside the store output, and propagated inputs are not file-referenced - it showed the same 7 paths before and after. Replaced with Nix-level evaluation + the functional simulation; recorded as a task correction.
- The nix `result` output symlink leaked into cleanSourceWith's source (custom filters ignore .gitignore), changing the package hash on every rebuild - excluded in the filter + .gitignore; recorded as a task correction.
- The doCheck criterion was blocked by PRE-EXISTING source violations (proven by building the original pre-task-2 package.nix from git, which fails the same checkPhase) - routed to plan-correction task 2a; recorded as a task correction.

## Other important findings/information
- Root-caused the recorded "clang answering for gcc" artifact: stdenv sets `CC=clang` when clang is a nativeBuildInput (probed: CC=[clang] with gcc+clang in nativeBuildInputs, CC=[] with them in buildInputs), and RFS's resolver honors the CC standard tier over bare gcc. This task's buildInputs layout leaves CC empty, so real gcc answers during checkPhase - the artifact's mechanism is fixed at the packaging level.
- The `.real`-suffix wrapper work (task 3) builds on this task's propagated store paths; the wrapper will reference exactly these buildInputs derivations.
