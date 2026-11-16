# Phase 15, Task 4 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Extend `shell.nix` so that EVERY test suite runnable on Ubuntu also runs green
under `nix develop -f shell.nix`: add the missing tools (make, graphviz,
plantuml, mermaid-cli, cargo-fuzz, rustup + nightly, python3), fix the z3
provisioning (raw split `pkgs.z3` today -> the unified prefix), and pass the
identical tool arguments to the parameterized package so shell and package
agree.

## Context
- **Current shell.nix (44 lines) and its gaps:** line 18 builds the package via
  `pkgs.callPackage ./nix/package.nix { } // { doCheck = false; }`; lines 24-27
  put the built `rfs` binary + full build closure on PATH (`packages` +
  `inputsFrom`); lines 29-33 add ONLY `gcc clang z3 pkg-config`. Missing for
  full-suite parity: **make** (c_sync_handler example), **graphviz/plantuml/
  mermaid-cli** (diagram_render_parity_test's external half - today it skips),
  **cargo-fuzz + a nightly toolchain** (`rfs test --suite fuzz --backend rust`),
  **python3** (rfs-dev calls port-equality, availability-gated). And the z3 it
  passes is RAW `pkgs.z3` - nixpkgs splits z3 into out/lib/dev outputs with no
  single prefix holding include/ + lib/ + bin/, so the resolvers' pkg-config
  tier finds a bin-only prefix and SMT-mode tests silently SKIP in today's
  shell. The unified-prefix derivation (package.nix lines 25-47) is the fix.
- **Design decision D3 - share one unifiedZ3 definition:** factor the runCommand
  out of package.nix into a NEW file `nix/z3-unified.nix` exporting
  `{ pkgs, z3 }: <the runCommand verbatim, comment block included>`; package.nix
  imports and calls it (replacing its inline copy), and shell.nix does the same.
  Rationale: one definition, zero drift between package and shell. Fallback if
  the import path proves awkward: duplicate with a "keep in sync with
  nix/package.nix" comment - record which was used.
- **Nightly for the Rust fuzz pipeline (verified constraint):** this nixpkgs
  has NO `rust-bin` and no nightly package (rust-bin was removed; only
  `rustc`/`cargo`/`rustPackages*`/`rustPlatform` remain). The working approach
  mirrors TOOLCHAIN.md §7.1 on Ubuntu: add `pkgs.rustup` to buildInputs and let
  the shellHook install nightly idempotently:
  `command -v rustup >/dev/null && ! rustup toolchain list 2>/dev/null | grep -q '^nightly' && rustup toolchain install nightly --profile minimal`
  (first entry needs network; afterwards it is cached in the profile dir). The
  scaffold's `rust-toolchain.toml` pin then selects nightly for the temp fuzz
  project exactly as on this machine. `pkgs.cargo-fuzz` (stable-built) provides
  the `fuzz` subcommand; with rustup on PATH, `cargo fuzz build` inside the
  scaffold runs under nightly.
- **mermaid-cli under nix needs no browser setup:** its makeWrapperArgs already
  set `PUPPETEER_EXECUTABLE_PATH` to store chromium 157.0.8103.38 (verified by
  nix eval). RFS passes its generated puppeteer config with `--no-sandbox`
  (required as root) - works unchanged.
- **mmdc serialization:** two mmdc processes must not run concurrently in this
  container (TOOLCHAIN.md §6.5); the diagram parity suite already serializes
  real-tool invocations under a static mutex - keep it that way when verifying.
- **The shell inherits the wrapped package** (task 3): `packages = [ rfs ]` puts
  the wrapper binaries on PATH, so `rfs test ...` inside the shell is already
  deterministic; the shell's own buildInputs additionally cover cargo-driven
  suites that resolve tools via bare PATH / standard vars.
- **Nix invocation:** `export PATH=/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin:$PATH`
  + `--impure`.

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 2. | package.nix parameterized dependencies | the shell passes the same tool arguments to the parameterized package |
| 3. | package.nix wrapper script | the shell's `packages = [ rfs ]` carries the wrapped binaries; user-redefinition semantics must hold inside the shell too |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 5. | BUILD.md Nix section | documents the finished shell's command surface |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `shell.nix` | change | full buildInputs set, unified z3, tool args to callPackage, nightly shellHook + tool-matrix echo |
| `nix/z3-unified.nix` | new (D3) | the shared unified-z3-prefix derivation extracted from package.nix |
| `nix/package.nix` | change | import and use the shared z3-unified.nix instead of the inline runCommand |

## Planned actions

### 1. Extract the shared unifiedZ3 derivation (D3)
Create `nix/z3-unified.nix`: `{ pkgs, z3 }: pkgs.runCommand ...` with the
runCommand body + comment block moved VERBATIM from package.nix lines 25-47
(only `pkgs.z3` -> `z3` already done by task 2). In package.nix replace the
inline definition with `unifiedZ3 = import ./z3-unified.nix { inherit pkgs z3; };`
in the let block. Verify package.nix still evaluates and builds identically
(same store path for unchanged inputs - or a new one only if the derivation
name changed; record it).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `nix/z3-unified.nix` | new | single shared definition of the unified z3 prefix (D3) |
| `nix/package.nix` | change (`25:47`) | inline runCommand replaced by the import; behavior identical |

### 2. Extend the shell to full-suite coverage
Rewrite shell.nix's buildInputs to:
`with pkgs; [ gcc clang rustc cargo make unifiedZ3 pkg-config cargo-fuzz graphviz plantuml mermaid-cli rustup python3 ]`
where `unifiedZ3 = import ./nix/z3-unified.nix { inherit pkgs z3; };` (raw
`pkgs.z3` dropped). Pass the identical tool arguments to the package:
`rfs = pkgs.callPackage ./nix/package.nix { inherit rustc cargo gcc clang make z3 pkg-config cargo-fuzz mermaid-cli plantuml graphviz; } // { doCheck = false; };`
- keep `packages = [ rfs ]` + `inputsFrom = [ rfs ]` (wrapped binary + build
  closure on PATH);
- shellHook: the idempotent nightly install from Context, then an echo of the
  resolved tool matrix (rustc/cargo/gcc/clang/make/z3/dot/plantuml/mmdc/
  cargo-fuzz versions + `rustup toolchain list`) and a note that FIRST entry
  needs network for the nightly download.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `shell.nix` | change (`13:44`) | full buildInputs, unified z3, tool args to callPackage, nightly shellHook + matrix echo |

### 3. Verify every Ubuntu-runnable suite inside nix develop
Run each under `nix develop -f shell.nix -c '<cmd>'` (or one interactive entry
running them in sequence) and record exact pass counts:
1. `cargo test --workspace` -> 676/0.
2. `cargo test --features smt` -> 550/0 (proves the unified-z3 fix: SMT tests
   RUN instead of skipping - compare against a pre-fix shell where they skip).
3. `cargo test-generated-code` -> 1683/1683.
4. `rfs test --suite unit --backend rust|c` on an example spec -> exit 0 both.
5. `rfs test --suite fuzz --backend c` -> exit 0 (closure clang + libFuzzer
   runtime per task 2's probe); `--backend rust` -> exit 0 via rustup nightly +
   cargo-fuzz.
6. `cargo test --test example_projects_test` -> 3/0 (gcc+make c_sync_handler,
   clang-gated fuzz_c, cargo-check fuzz_rust).
7. `cargo test --test diagram_render_parity_test` -> 13/0 with the EXTERNAL
   half ACTIVE (dot/plantuml/mmdc all resolve; mmdc renders via wired chromium;
   serialized - no concurrent mmdc).
8. `cargo test --test dev_docs_test` (9/0) + `--test docs_md_test` (5/0) -
   pure Rust, must pass unconditionally.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only; counts go to the status file |

## Task deliverables
- `shell.nix` covering every Ubuntu-runnable suite; `nix/z3-unified.nix` shared
  derivation; package.nix consuming it.
- The full verification matrix with exact counts recorded in the status file.

## Task exit criteria
- [ ] `nix develop -f shell.nix` enters cleanly; the tool-matrix echo shows all
      tools resolved (incl. nightly present after first entry).
- [ ] All eight verification commands pass with the documented counts; SMT
      tests RUN (not skip); diagram parity external half ACTIVE.
- [ ] `nix/z3-unified.nix` is the single definition - package.nix and shell.nix
      both import it (or the documented fallback duplication, flagged).
- [ ] Package build via the shell's callPackage still succeeds (doCheck=false
      path) with identical tool arguments.
- [ ] Committed per FORMATTING.md §7.

## Non-goals
- No hermetic nightly in the package closure (rustup-based, documented).
- No changes to test sources - suites that skip on absent tools keep their
  availability gates; this task makes the tools PRESENT so nothing skips.
- No CI integration.

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| Rust fuzz verified via the unwrapped binary + proxy, not the wrapped `rfs` | Verification item 5's "`--backend rust` -> exit 0" is met by invoking `rfs.real` with the rustup proxy bin dir first on PATH (exit 0: nightly + cargo-fuzz + libFuzzer all work under nix). The WRAPPED `rfs`'s `rfs-env.sh` prepends the nix store stable `cargo`/`rustc`, so cargo-fuzz's internal bare-`cargo` build call resolves to store-stable and rejects `-Zsanitizer` (the scaffold's nightly pin is honoured only by the rustup proxy). This is a task-3 wrapper concern, not a shell.nix defect; routed to a plan correction + follow-up. The C fuzz backend works fully through the wrapped binary. |
| `TMPDIR=/tmp` for mmdc-invoking suites | This sandbox's nix-shell temp dir is deeply nested, so chromium's SingletonSocket path exceeds 108 chars and mmdc fails to launch. A normal Ubuntu nix uses a short `/tmp/nix-shell.XXXX` and is unaffected; left out of the shellHook so the definition stays correct for normal systems. |
| Isolated `CARGO_TARGET_DIR` for the workspace run | The shared repo `target/` held mixed host/nix rlibs (four `librfs_adapters-*.rlib`), and `nostd_minimal.rs`'s non-deterministic rlib selection hit a mismatched one -> E0514. A clean `CARGO_TARGET_DIR=/tmp/rfs-nix-target` gives a deterministic 685/0. `smt_bounded_test` runs with the DEFAULT target dir instead (its scratch project hardcodes `<proj>/target/release/smt_gen`, which `CARGO_TARGET_DIR` would redirect). |

## Other information
- First `nix develop` entry may take a while (nightly download + any missing
  store paths); subsequent entries are fast. If the nightly download fails
  offline, record it - the C-side suites still run.
- If `cargo test --features smt` under the shell shows z3-sys pinning surprises
  (the checkPhase `explicit=false` subtlety is BUILD-time only), remember the
  shell's cargo builds are USER builds: setting Z3_SYS_Z3_DIR there is correct
  and expected - do not "fix" it by unsetting.
