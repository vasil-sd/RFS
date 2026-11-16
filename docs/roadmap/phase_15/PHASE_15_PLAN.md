# Phase 15 - Implementation plan

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation immediately without re-deriving scope.

## Context

**Nix in THIS sandbox (verified 2026-11-15, planning session).**
- Nix **2.35.6** is installed but NOT on PATH: the binary lives at
  `/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin/nix` (no
  `/usr/bin/nix` symlink, no `/etc/profile.d/nix.sh`). Prepend that store bin
  dir to PATH in every nix command.
- **No daemon is running and none is needed**: there is no
  `/nix/var/nix/daemon-socket`, yet `nix build` works - as root, Nix falls back
  to single-user (direct) mode. `/etc/nix/nix.conf` sets
  `experimental-features = nix-command flakes` and `sandbox = false`.
- The store is pre-populated from the preliminary nix work (many
  `*-rfs-1.0.0.drv` / built outputs), so rebuilds are mostly cache hits.
- Network to `cache.nixos.org` works (verified: `nix build --expr 'hello'`
  fetched and ran).
- `import <nixpkgs> {}` resolves to **nixpkgs unstable 26.11pre**
  (`lib.version = "26.11pre1109744.4fe25e12eb9f"`).

**Baseline build of the CURRENT package (verified 2026-11-15).**
`nix build --impure --expr '(import ./nix/package.nix { }).overrideAttrs (a: { doCheck = false; })'`
succeeds -> `/nix/store/xkz5s7wi67rbdiajj5indydkxsvhgk4x-rfs-1.0.0` with
binaries `rfs`, `build_training`, `extract_training_snippets`,
`generate_code_tests`, `test_generated_code`. **Its runtime closure is only 7
paths** (libunistring, libidn2, gcc-libgcc, glibc, gcc-lib, the package) - NO
tool binaries. This proves task 1's premise: today's `nativeBuildInputs`
(gcc/clang/rustc/cargo/gnumake/unifiedZ3/pkg-config) are build-time only and
vanish from the installed package, so `rfs test ...` from a bare install cannot
find any compiler.

**Preliminary nix work (the "previous work" task 0 must reference).**
- Commit **dd6e0f4** (2026-11-12) "Make generated C clang-clean and add Nix
  package + dev shell": created `nix/package.nix` (buildRustPackage,
  unifiedZ3 runCommand, doCheck over 10 MC suites + smt_bounded_test) and
  `shell.nix` (mkShell, inputsFrom the package, gcc/clang/z3/pkg-config
  buildInputs); also fixed 3 C-backend defects the nix clang toolchain exposed
  (-Wunused-function pragma, double-paren equality guards, a use-after-scope UB
  in --violate parsing).
- Commit **d1c1b31** (2026-11-12): unifiedZ3 switched from copying to plain
  `ln -s` (this nixpkgs' symlinkJoin/lndir dereferences and would copy the
  44 MB libz3.so); z3.pc moved to `${unifiedZ3}/pkgconfig`, checkPhase exports
  `PKG_CONFIG_PATH` there.
- The ONLY nix entry in `TASKS.md` (line ~2463, Phase 14 task 8 close-out):
  `nix develop -f shell.nix` failed on the rfs package build because
  `ref_mc_diff_test` panicked on a CLANG-style warning attributed to
  `gcc -Wall` - "clang answering for gcc", a tool-resolution artifact of the
  nix sandbox, not a codegen bug. **Task 3's wrapper (explicit `RFS_GCC`) is
  the fix; task 3's verification re-runs that suite under nix to confirm.**
- Reference guide: https://dev.to/misterio/how-to-package-a-rust-app-using-nix-3lh3
  (buildRustPackage happy path: minimal default.nix, `cargoLock.lockFile`,
  `cleanSource`, flake variant, `shell.nix` with `inputsFrom` + extra
  buildInputs, `importTOML` for name/version - the pattern our package.nix
  already follows).

**nixpkgs 26.11pre facts (verified by nix eval during planning).**
- `mermaid-cli` **12.0.0**: mainProgram `mmdc`; its makeWrapperArgs ALREADY set
  `PUPPETEER_EXECUTABLE_PATH` to the store's **chromium 157.0.8103.38** - so
  under nix, mmdc works with zero extra browser setup (RFS still passes its
  generated puppeteer config with `--no-sandbox`, required when running as
  root; overridable via `RFS_MERMAID_PUPPETEER_ARGS`).
- `plantuml` **1.2026.9** (pulls a JRE), `graphviz` **15.1.1** (provides `dot`),
  `z3` **5.3.2** (split out/lib/dev outputs - the existing unifiedZ3 runCommand
  handles it), `gcc` **16.2.0**, `clang` **21.1.8** (SINGLE output - the
  libFuzzer runtime, if built in, lives inside `out/lib/clang/<ver>/lib/linux/`
  as `libclang_rt.fuzzer-x86_64.a`; **verify presence at build time in task 1**
  - Ubuntu's clang ships it, nixpkgs builds compiler-rt with the fuzzer
  runtime by default but this must be proven, not assumed).
- `cargo-fuzz` exists but is built against STABLE rustc. **There is NO
  `rust-bin` and no nightly package in this nixpkgs** (rust-bin was removed;
  only `rustc`/`cargo`/`rustPackages*`/`rustPlatform` remain). Consequence:
  the Rust fuzz pipeline (`-Zsanitizer=address`, nightly-only) cannot be met
  hermetically from the package closure. The dev shell solves it with
  `pkgs.rustup` + a `rustup toolchain install nightly --profile minimal`
  shellHook step (idempotent, cached in the profile dir after first entry -
  same convention as TOOLCHAIN.md §7.1 on Ubuntu). The scaffold's
  `rust-toolchain.toml` pin then selects nightly for the temp project exactly
  as it does on this machine.
- `pkgs.rustup` and `pkgs.rustup-toolchain-install-master` exist as
  derivations (fallback if plain rustup misbehaves).

**The RFS_* env-var contract (from `src/cli/ext_tool.rs` presets +
`tests/common/toolchain.rs`).** Precedence everywhere: CLI flag > `RFS_*` env
var > standard Unix var > bare name on PATH (pkg-config for z3). The wrapper
must set, each ONLY when unset by the user:

| Variable | Standard tier | Notes |
|---|---|---|
| `RFS_RUSTC` (+`RFS_RUSTC_ARGS`) | `RUSTC` | rust codegen/test compile |
| `RFS_GCC` (+`RFS_GCC_ARGS`) | `CC` | C codegen/test compile; example Makefiles use `CC ?= gcc` |
| `RFS_CLANG` (+`RFS_CLANG_ARGS`) | none (deliberate) | libFuzzer host; `CC` must not leak in |
| `RFS_CARGO_FUZZ` (+`_ARGS`) | `CARGO` | the cargo that has the `fuzz` subcommand |
| `RFS_MAKE` | none | C example projects |
| `RFS_DOT` / `RFS_PLANTUML` / `RFS_MERMAID` (+`_ARGS` each) | none | external diagram rendering; mmdc also honors `RFS_MERMAID_PUPPETEER_ARGS` |
| `RFS_Z3_DIR` | `Z3_SYS_Z3_DIR` | z3 discovery chain: `RFS_Z3_DIR` > `Z3_SYS_Z3_DIR` > `pkg-config --variable=prefix z3` > `/workspace/host/z3` (last tier absent under nix - the unified prefix must be found via one of the first three) |

Plus for z3-sys at USER build time (`cargo test --features smt`):
`Z3_SYS_Z3_DIR=<unified>` and `Z3_LIBRARY_PATH_OVERRIDE=<unified>/bin`
(libz3.so is symlinked into bin/ by unifiedZ3), and `PKG_CONFIG_PATH` prepended
with `<unified>/pkgconfig`. The wrapper sets all of these conditionally.

**Do NOT touch the checkPhase z3 logic.** package.nix's checkPhase deliberately
leaves `RFS_Z3_DIR`/`Z3_SYS_Z3_DIR` UNSET and only exports `PKG_CONFIG_PATH`
so that z3-sys reads the real version (5.3.2) from headers with `explicit`
false. The wrapper is a RUNTIME concern; the checkPhase is BUILD time. Mixing
them up would re-pin the solver version and break SMT parity.

**The package ships FIVE binaries** (`rfs` + the four cargo bins
`build_training`, `extract_training_snippets`, `generate_code_tests`,
`test_generated_code`). `generate_code_tests`/`test_generated_code` also invoke
rustc/gcc, so the wrapper preamble must cover ALL five, not just `rfs`
(recommended mechanism: one generated env-preamble script in `$out/bin` that
each wrapped binary sources before exec'ing the real binary - or makeWrapper
`--run` with the conditional exports; decide in task 2, keep it uniform).

**`.cargo/env.toml`** (included by `.cargo/config.toml`) sets a NON-FORCING
`RFS_Z3_DIR = "/workspace/host/z3"` default that is existence-checked and
skipped when absent - under nix it simply falls through; no change needed.

**What "full test suite on Ubuntu" means (BUILD.md §3-5, verified baselines).**
Current baselines are the Phase 14 close-out numbers: `cargo test` **548** /
`--workspace` **676** / `--features smt` **550** / `test-generated-code`
**1683** (BUILD.md §5's "known-good baseline" text still shows the older
538/666/540 re-measured 2026-11-12, before Phase 14 added 13 tests - use the
Phase 14 numbers for verification). `rfs test --suite unit|fuzz` both backends;
`example_projects_test` (gcc+make for c_sync_handler, clang-gated fuzz_c,
cargo check for fuzz_rust); `diagram_render_parity_test` (internal half always;
external half needs dot/plantuml/mmdc - must be ACTIVE in the shell);
docs-sync gates `dev_docs_test` (9) + `docs_md_test` (5) are pure Rust; rfs-dev
calls port-equality is python3-gated (include python3 so it runs). The current
shell.nix is missing: make, graphviz, plantuml, mermaid-cli, cargo-fuzz,
rustup/nightly, python3 - and passes RAW `pkgs.z3` (split outputs) which the
resolvers cannot use for SMT mode (no single prefix with include/ + lib/ +
bin/) - so SMT tests silently skip in today's shell. All of this is task 3.

**Docs-sync obligation.** This phase changes `TOOLCHAIN.md` and `BUILD.md`
(command/toolchain docs) - per START.md §8 those must stay synchronized;
no `DESIGN.md`/`KB.md` design decisions change (packaging only). No source
files change, so the structure inventories and `dev_docs_test` are unaffected
(re-run them in close-out as proof).

**Task numbering vs the phase request.** The phase request listed tasks 0-4;
this plan renumbers them 1-5 and adds task 6 (close-out): plan task 1 =
request task 0 (TOOLCHAIN.md nix section), plan task 2 = request task 1
(package.nix dependencies + arguments), plan task 3 = request task 2 (wrapper
script), plan task 4 = request task 3 (shell.nix full-suite coverage), plan
task 5 = request task 4 (BUILD.md nix section).

**Examples / tests / training sets.** This phase changes no RFS source, no
`.rfs` examples and no training data - it is packaging + documentation only,
so FORMATTING.md's "include examples update, tests update, training-sets
update as explicit tasks" rule is satisfied vacuously: the test UPDATE that
does exist is the nix-side verification runs inside tasks 3-4 (running the
existing suites under nix), and task 6 re-proves the canonical baselines are
byte-unchanged.

## Planned tasks

| # | Task | Affected files (operation) | Description + rationale |
|---|------|----------------------------|--------------------------|
| 1. | TOOLCHAIN.md: Nix-on-Ubuntu-sandbox section | `TOOLCHAIN.md` (change) | New final section (numbered after the current §9): how to install Nix on Ubuntu (official installer one-liner; multi-user daemon mode vs single-user `--no-daemon`; what the nix daemon is, when it is required - non-root use / remote builds - and how to run `nix-daemon` manually in a container without systemd; this sandbox's actual state: 2.35.6 at its store path, no PATH entry, no daemon, single-user-as-root works, `sandbox = false`, flakes on, pre-populated store, cache.nixos.org reachable); the dev.to buildRustPackage guide as the packaging reference; the previous preliminary work (commits dd6e0f4/d1c1b31, what they added, the TASKS.md clang-answering-for-gcc finding) and how to use nix here (PATH export + `nix build --impure` / `nix develop -f shell.nix`). Rationale: task 0 of the phase request - future sessions must be able to set up and drive Nix without re-discovering any of this. |
| 2. | package.nix: parameterized tools as propagated dependencies | `nix/package.nix` (change) | New argument signature `{ pkgs ? import <nixpkgs> {}, rustc ? pkgs.rustc, cargo ? pkgs.cargo, gcc ? pkgs.gcc, clang ? pkgs.clang, make ? pkgs.gnumake, z3 ? pkgs.z3, pkg-config ? pkgs.pkg-config, cargo-fuzz ? pkgs.cargo-fuzz, mermaid-cli ? null, plantuml ? null, graphviz ? null }`; unifiedZ3 takes the `z3` argument; move rustc/cargo/gcc/clang/make/unifiedZ3/pkg-config/cargo-fuzz from `nativeBuildInputs` to `buildInputs` (propagated into the runtime closure) keeping only true build-time needs in nativeBuildInputs; optional diagram tools via `lib.optionals (x != null)` appended to buildInputs. Verify: `nix build` (doCheck=false) succeeds, `nix path-info -r` shows every tool binary in the closure, and a probe compile proves the closure's clang carries the libFuzzer runtime (`clang -fsanitize=fuzzer` on a trivial target). Rationale: phase task 1 - the installed package must make `rfs test ...` fully operational with user-overridable tool versions. |
| 3. | package.nix: RFS_* wrapper script honoring user redefinition | `nix/package.nix` (change) | Wrap ALL FIVE package binaries so each, before exec'ing the real binary, conditionally exports (only when unset): `RFS_RUSTC`, `RFS_GCC`, `RFS_CLANG`, `RFS_CARGO_FUZZ`, `RFS_MAKE`, `RFS_Z3_DIR`, `Z3_SYS_Z3_DIR`, `Z3_LIBRARY_PATH_OVERRIDE`, and prepends the unified z3 pkgconfig dir to `PKG_CONFIG_PATH`; when the optional args are non-null additionally `RFS_DOT`/`RFS_PLANTUML`/`RFS_MERMAID`. Mechanism: makeWrapper `--run` conditional-export preamble (or one shared generated preamble script sourced by each wrapper - decide here, keep uniform). checkPhase z3 logic stays untouched. Verify: wrapped `rfs test --suite unit` passes both backends with NO env preset; with `RFS_GCC` pre-set to a different store gcc, `--print-script` shows the user's value winning; optional-args build sets the three diagram vars, default build leaves them unset; re-run `ref_mc_diff_test` under nix to confirm the clang-answering-for-gcc artifact is gone. Rationale: phase task 2 - deterministic tool resolution from the installed package, user overrides always win. |
| 4. | shell.nix: full Ubuntu-parity test-suite coverage | `shell.nix` (change) | Extend buildInputs with make, graphviz, plantuml, mermaid-cli, cargo-fuzz, rustup, python3; replace raw `pkgs.z3` with the same unified-prefix derivation (factor it so package.nix and shell.nix share one definition - e.g. move unifiedZ3 into `nix/` as its own file both import, or duplicate with a comment); pass the identical tool arguments to the `callPackage` of package.nix so shell and package agree; shellHook installs the nightly toolchain via rustup (idempotent) and prints the resolved tool matrix. Verify inside `nix develop`: `cargo test --workspace`, `cargo test --features smt`, `cargo test-generated-code`, `rfs test --suite unit` + `--suite fuzz` (BOTH backends - C via closure clang, Rust via rustup nightly + cargo-fuzz), `cargo test --test example_projects_test`, `cargo test --test diagram_render_parity_test` with the external half ACTIVE (all three tools resolve, mmdc renders via wired chromium), `cargo test --test dev_docs_test` + `--test docs_md_test`. Rationale: phase task 3 - every suite that runs on Ubuntu must run green under nix. |
| 5. | BUILD.md: explicit Nix build/test/run section | `BUILD.md` (change) | New section (after §7, before or as §8 - follow the file's numbering): installing/locating nix on this sandbox (pointer to TOOLCHAIN.md's new section), building the package (`nix build --impure --expr 'import ./nix/package.nix { }'`, with/without doCheck, passing custom tool arguments), running the wrapped binary (what the wrapper sets, how user `RFS_*` redefinition wins, `--print-script` as the inspection tool), the dev shell (`nix develop -f shell.nix -c ...` per-suite commands incl. the nightly fuzz pipeline), and what works from the bare package vs only from the shell (Rust fuzz nightly). Rationale: phase task 4 - BUILD.md is the canonical build/test/run reference and must document Nix explicitly. |
| 6. | Phase close-out: full verification + bookkeeping | `docs/roadmap/phase_15/PHASE_15_STATUS.md` (new), `TASKS.md` (change - append entry per its first-paragraph template), `docs/roadmap/CURRENT_STATE.md` (change - phase 15 row) | Run the complete exit-criteria sweep: canonical toolchain baselines unchanged (`cargo test` 548 / `--workspace` 676 / `--features smt` 550 / generated-code 1683 - the Phase 14 close-out baselines; fmt clean, clippy at baseline - proof that no source changed), the nix-side proofs from tasks 2-4 re-confirmed end to end, then write PHASE_15_STATUS.md per FORMATTING.md §4 (files, functionality deltas, tests run with exact counts, findings incl. the clang-for-gcc resolution and any libFuzzer-runtime probe result), append the TASKS.md entry, update CURRENT_STATE.md. Rationale: PROTOCOL.md §3 steps r-u - the phase exits only with all criteria met and the record kept. |

## Plan corrections (during execution)

| # | Task | Correction type | Description + rationale |
|---|------|-----------------|--------------------------|
| 2a. | Make generated C warning-free under the nixpkgs toolchain (checkPhase blocker) | new | Task 2's doCheck criterion exposed that the CURRENT source fails the package checkPhase under BOTH nix compilers - pre-existing, not caused by task 2 (the original pre-task-2 package.nix fails the same checkPhase). Two distinct zero-warning-gate violations: (1) gcc 16.2.0 `-Wmaybe-uninitialized` on `uint64_t fields[64]` in the test driver template at tests/common/prod_driver.rs:615 (fails prod_parity_test); (2) clang 21.1.8 `-Wtautological-overlap-compare` on an always-true disjunction `(s != B) || (s == B)` in generated FSM C (fails ref_mc_diff_test - the artifact recorded in TASKS.md, whose mechanism task 2 additionally pinned down: stdenv sets CC=clang when clang is a nativeBuildInput, and RFS's resolver honors CC over bare gcc; moving the tools to buildInputs left CC empty so real gcc answers). Fixing both requires source changes (a test helper + the C codegen emitter), which the phase non-goals route through plan corrections. Task 2's doCheck criterion is PENDING on 2a; all other task-2 criteria are met and committed. |
| 4. | Rust fuzz pipeline through the WRAPPED binary needs the proxy, not the store cargo | adjustment | Task 4's verification item 5 assumed (per Context) that "with rustup on PATH, `cargo fuzz build` inside the scaffold runs under nightly" via the scaffold's `rust-toolchain.toml` pin. That holds only when bare `cargo` is the RUSTUP PROXY (as on Ubuntu). Under nix the shell's `rfs` is task 3's WRAPPED binary, whose `rfs-env.sh` prepends the nix store stable `cargo`/`rustc` to PATH; cargo-fuzz re-invokes bare `cargo` for its build, so that internal call resolves to store-stable and rejects `-Zsanitizer` (the pin is honoured only by the proxy). The machinery itself is fully functional under nix - invoking the UNWRAPPED `rfs.real` with the rustup proxy bin dir first on PATH gives exit 0 (nightly + cargo-fuzz + libFuzzer all present and working), and the C fuzz backend works fully through the wrapped binary. So item 5 is met for the C backend via the wrapped `rfs` and for the Rust backend via the unwrapped binary + proxy; making the WRAPPED binary route its fuzz pipeline's internal cargo through the proxy is a task-3 wrapper follow-up (out of task 4's shell.nix scope). |
| 2b. | Fix the task-2a SMT-bounded backend-parity regression (host z3 5.3.1) | new | Task 6's canonical sweep found `cargo test --features smt` fails on `smt_bounded_backend_parity`: for the fail variant (property `flag_or_counter : always (flag == true || counter > 0)`, violated at the initial state flag=false/counter=0) the C backend correctly reports FAIL but the Rust backend reports PASS WITHIN BOUND. It passes at pre-task-2a source (17d22bc) and under nix z3 5.3.2 (task 2a's doCheck 42/42), so task 2a's boolean-folding change introduced a z3-version-sensitive divergence that only manifests with the host z3 5.3.1. New task 2b diagnoses + fixes it so both backends agree across solver versions; the phase close-out is PENDING on 2b. |

## Other information

- **Open question to settle in task 2 (prove, don't assume):** whether this
  nixpkgs' single-output `clang` 21.1.8 actually contains
  `libclang_rt.fuzzer-x86_64.a`. Probe: build a trivial libFuzzer target with
  the store clang inside a `nix develop`/`nix eval` shell. If absent, the C
  fuzz pipeline needs `pkgs.clang-wrapper` or an override enabling
  compiler-rt's fuzzer component - record the outcome in task 2's status file
  and adjust task 4's verification accordingly.
- **Open question for task 3:** makeWrapper `--run` vs a shared generated
  preamble script. Prefer whichever keeps the five wrappers uniform and makes
  the conditional-export logic visible in one place; document the choice in
  the status file.
- **First `nix develop` entry needs network** (rustup nightly download +
  possibly store fetches); subsequent entries are cached. Note this in the
  shellHook echo output so a future session is not surprised.
- **Do not run two mmdc processes concurrently** (TOOLCHAIN.md §6.5) - the
  diagram parity suite already serializes them; keep it that way when
  verifying under nix.
- **Commit discipline:** one commit per task per FORMATTING.md §7 (subject +
  Changes/Issues and findings/Verification results). The phase touches no
  `.rs` file, so `rfs-dev inventory audit` / `sync-check` are expected to stay
  EXACT/clean - run them in task 6 as proof.
- **Token budget:** every task above is a single-file (or two-file) change plus
  nix verification runs - comfortably inside the 200k per-task budget; task 4
  is the largest (iterative `nix develop` suite runs) and may need several
  build iterations if a tool is missing from the shell.
