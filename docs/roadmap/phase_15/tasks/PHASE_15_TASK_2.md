# Phase 15, Task 2 - Implementation details

> Written for a future LLM session: after reading `START.md`, the
> `docs/roadmap/` set (`INTRO.md` + `PROTOCOL.md` + `FORMATTING.md`) and this file,
> the assistant must be able to start
> implementation of the task immediately without re-deriving scope.

## Goal
Make `nix/package.nix` accept every external tool as an overridable package
argument (users can pass their own versions) and move the tools that
`rfs test ...` needs at RUN TIME from build-time-only `nativeBuildInputs` to
propagated `buildInputs`, so the built package's closure carries rustc, cargo,
gcc, clang, make, the unified z3 prefix, pkg-config and cargo-fuzz alongside
the `rfs` binary.

## Context
- **Why (verified 2026-11-15):** the current package builds to
  `/nix/store/xkz5s7wi67rbdiajj5indydkxsvhgk4x-rfs-1.0.0` whose runtime closure
  is only 7 paths (libunistring, libidn2, gcc-libgcc, glibc, gcc-lib, the
  package) - NO tool binaries. Line 95's
  `nativeBuildInputs = [ pkgs.gcc pkgs.clang pkgs.rustc pkgs.cargo pkgs.gnumake unifiedZ3 pkgs.pkg-config ]`
  is build-time only: it feeds the checkPhase but vanishes from the installed
  package, so a bare `rfs test ...` from an install finds no compiler.
- **buildRustPackage internals (nixpkgs 26.11pre, verified by reading
  `pkgs/build-support/rust/build-rust-package/default.nix` + evaluating
  `__functionArgs`):** `cargo` and `rustc` are callPackage-INJECTED arguments
  (default.nix lines 11-14) that buildRustPackage puts into its OWN
  nativeBuildInputs (lines 155-156). They are NOT in the public function args,
  so the BUILD toolchain is always nixpkgs' default rustc/cargo and cannot be
  swapped through our package arguments. Consequence (design decision D1): a
  user-passed `rustc`/`cargo` argument flows into `buildInputs` - it lands in
  the runtime closure and (task 3) in the wrapper's `RFS_RUSTC`/`RFS_CARGO_FUZZ`
  - while the package build itself keeps using nixpkgs' hermetic toolchain.
  With the default arguments both are the SAME derivation, so nothing is
  duplicated. Building with a foreign rustc is out of scope (would require
  overriding the callPackage set - future phase).
- **Moving to buildInputs keeps checkPhase working:** `buildInputs` are on PATH
  for the WHOLE build including checkPhase (propagation only additionally puts
  them in the runtime closure). So gcc/clang/make/unifiedZ3/pkg-config remain
  available to the existing checkPhase scripts unchanged.
- **nixpkgs 26.11pre tool facts (verified by nix eval):** gcc 16.2.0, clang
  21.1.8 (SINGLE output - if the libFuzzer runtime is built in it lives inside
  `out/lib/clang/<ver>/lib/linux/libclang_rt.fuzzer-x86_64.a`; PROVE presence
  with the action-3 probe, do not assume), z3 5.3.2 (split out/lib/dev - the
  existing unifiedZ3 runCommand at lines 25-47 assembles the single prefix;
  it must take the `z3` argument instead of hardcoding `pkgs.z3`), graphviz
  15.1.1 (`dot`), plantuml 1.2026.9 (pulls a JRE), mermaid-cli 12.0.0 (its
  makeWrapperArgs already set `PUPPETEER_EXECUTABLE_PATH` to store chromium
  157.0.8103.38 - zero browser setup under nix), cargo-fuzz (built against
  STABLE rustc - fine for the C fuzz pipeline; the Rust pipeline's nightly need
  is a dev-shell concern, task 4).
- **Current file anchors (nix/package.nix, 127 lines):** line 1 signature
  `{ pkgs ? import <nixpkgs> { } }:`; lines 25-47 unifiedZ3 runCommand (hardcodes
  `pkgs.z3` in its name and body); line 50 `buildRustPackage rec {`; line 95
  nativeBuildInputs; lines 97-118 doCheck/checkPhase (UNCHANGED by this task);
  line 121 meta.
- **Nix invocation in this sandbox:**
  `export PATH=/nix/store/g3m57m0jcdj84fas49lpiwpisacp20gr-nix-2.35.6/bin:$PATH`
  then `nix build --impure ...` (`--impure` required: `<nixpkgs>` resolves via
  the channel profile).

## Scope

### Depends on tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| - | none | standalone; task 3 (wrapper) builds on this task's buildInputs |

### Dependent tasks

| # | Task | Description + explanation |
|---|------|----------------------------|
| 3. | package.nix wrapper script | wraps the binaries and references the same tool store paths this task puts in buildInputs |
| 4. | shell.nix full-suite coverage | passes the identical tool arguments to the parameterized package |

### Other affected files

| File | Operation | Brief description |
|------|-----------|-------------------|
| `nix/package.nix` | change | new argument signature, parameterized unifiedZ3, buildInputs propagation, optional diagram tools |

## Planned actions

### 1. Parameterize the package arguments
Replace line 1's signature with the full argument list, each defaulting to the
nixpkgs package so existing call sites (`import ./nix/package.nix { }`,
`pkgs.callPackage ./nix/package.nix { }`) keep working unchanged:

```nix
{ pkgs ? import <nixpkgs> { }
, rustc ? pkgs.rustc
, cargo ? pkgs.cargo
, gcc ? pkgs.gcc
, clang ? pkgs.clang
, make ? pkgs.gnumake
, z3 ? pkgs.z3
, pkg-config ? pkgs.pkg-config
, cargo-fuzz ? pkgs.cargo-fuzz
, mermaid-cli ? null
, plantuml ? null
, graphviz ? null
}:
```

Rationale: phase request task 1 - users pass their own tool versions;
`mermaid-cli`/`plantuml`/`graphviz` are OPTIONAL (default null) per the request.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `nix/package.nix` | change (`1:1`) | new 12-argument signature with nixpkgs defaults and null optional diagram tools |

### 2. Parameterize unifiedZ3 on the z3 argument
Change the runCommand (lines 25-47) to use the `z3` argument instead of
`pkgs.z3` everywhere (derivation name `z3-unified-${z3.version}` and all six
`${pkgs.z3.*}` references). Keep the whole explanatory comment block - it
records why plain `ln -s` instead of symlinkJoin/lndir.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `nix/package.nix` | change (`25:47`) | unifiedZ3 takes the overridable `z3` argument; comment preserved |

### 3. Move runtime tools to buildInputs, add optional diagram tools
Replace line 95 with:

```nix
  # Runtime tool closure (Phase 15 task 2): these are propagated into the
  # installed package so `rfs test ...` works from a bare install - rustc/cargo
  # compile generated code, gcc/clang build C artifacts (clang carries the
  # libFuzzer runtime for --suite fuzz), make drives the C example projects,
  # unifiedZ3 + pkg-config back SMT mode, cargo-fuzz scaffolds the Rust fuzz
  # pipeline. buildInputs are on PATH during checkPhase too, so the existing
  # check scripts keep working unchanged. The BUILD itself still uses
  # buildRustPackage's callPackage-injected rustc/cargo (nixpkgs defaults).
  buildInputs = [ rustc cargo gcc clang make unifiedZ3 pkg-config cargo-fuzz ]
    ++ lib.optionals (mermaid-cli != null) [ mermaid-cli ]
    ++ lib.optionals (plantuml != null) [ plantuml ]
    ++ lib.optionals (graphviz != null) [ graphviz ];

  nativeBuildInputs = [ ];
```

(`lib` is available as `pkgs.lib`; use `pkgs.lib.optionals` or bind `lib =
pkgs.lib;` in the let - decide by what reads best, keep it uniform.)
Rationale: this is the core of phase request task 1 - "normal dependencies,
not only nativeBuildInputs". Empty nativeBuildInputs is correct because
buildRustPackage injects its own rustc/cargo + cargo hooks internally (Context).

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| `nix/package.nix` | change (`95:95`) | tools moved to propagated buildInputs; optional diagram tools via optionals; nativeBuildInputs emptied with an explanatory comment |

### 4. Verify: closure proof, full check build, libFuzzer probe
1. `nix build --impure --expr '(import ./nix/package.nix { }).overrideAttrs (a: { doCheck = false; })'`
   succeeds.
2. `nix path-info -r <result>` lists the tool binaries: gcc, clang, rustc,
   cargo, gnumake/make, the unified z3 prefix, pkg-config, cargo-fuzz (and
   chromium + nodejs ONLY when mermaid-cli is passed - prove both directions:
   default build has no chromium, `import ./nix/package.nix { mermaid-cli =
   pkgs.mermaid-cli; }` does).
3. Full `nix build --impure --expr 'import ./nix/package.nix { }'` (doCheck=true)
   passes all checkPhase suites - proof that moving the tools to buildInputs
   broke nothing in the check environment.
4. libFuzzer probe (settle the open question from the plan): inside a nix
   shell with the store clang, compile + bounded-run a trivial
   `LLVMFuzzerTestOneInput` target with `-fsanitize=fuzzer,address`. Record the
   outcome (runtime present at `lib/clang/<ver>/lib/linux/`) in the status file.
   If ABSENT: stop and record a plan correction - the C fuzz pipeline would
   need a clang override enabling compiler-rt's fuzzer component.

#### Affected files

| File | Operation | Description + rationale |
|------|-----------|-------------------------|
| (none) | - | verification only; findings go to the status file |

## Task deliverables
- `nix/package.nix` with the 12-argument signature, parameterized unifiedZ3,
  and propagated runtime tool dependencies (incl. optional diagram tools).
- Closure proof + green doCheck build + libFuzzer probe result recorded in the
  status file.

## Task exit criteria
- [ ] `nix/package.nix` accepts all twelve arguments with the documented
      defaults; existing no-arg call sites still evaluate.
- [ ] Default `nix build` (doCheck=false) succeeds and `nix path-info -r`
      shows rustc, cargo, gcc, clang, make, unified z3, pkg-config and
      cargo-fuzz binaries in the closure; NO chromium/nodejs by default.
- [ ] Passing `mermaid-cli = pkgs.mermaid-cli` (likewise plantuml/graphviz)
      adds exactly that tool (+ its chromium for mermaid-cli) to the closure.
- [ ] Full `nix build` with doCheck=true passes every checkPhase suite
      (10 MC suites + smt_bounded_test).
- [ ] libFuzzer probe outcome recorded (runtime present/absent + path).
- [ ] Committed per FORMATTING.md §7.

## Non-goals
- No wrapper script yet (task 3) - after this task the closure carries the
  tools but a bare `rfs test` still relies on PATH; the wrapper makes it
  deterministic.
- No checkPhase changes (the z3 `explicit=false` subtlety stays untouched).
- No shell.nix changes (task 4).
- No building-with-foreign-rustc support (D1 above).

## Task corrections

| Correction | Description + rationale |
|------------|-------------------------|
| doCheck criterion pending on new task 2a | The exit criterion "full nix build with doCheck=true passes every checkPhase suite" is NOT met by this task's own changes: the CURRENT source fails the zero-warning gate under both nix compilers (gcc 16 -Wmaybe-uninitialized in tests/common/prod_driver.rs:615; clang 21 -Wtautological-overlap-compare on a generated always-true disjunction). Proven pre-existing by building the ORIGINAL pre-task-2 package.nix from git (commit 2bb53fb) - it fails the same checkPhase (at ref_mc_diff_test, the recorded clang-as-gcc artifact). Plan correction 2a adds the source-fix task; this task's status file is written after 2a lands and the doCheck build goes green. All OTHER exit criteria of this task are met and verified. Bonus finding: moving the tools from nativeBuildInputs to buildInputs incidentally FIXED the clang-as-gcc artifact's mechanism - stdenv sets CC=clang only when clang is a nativeBuildInput (probed empirically), and RFS's resolver honors CC over bare gcc; with the tools in buildInputs, CC stays empty and real gcc answers. |
| Explicit propagatedBuildInputs | Action 3's premise that moving tools to `buildInputs` propagates them into the runtime closure was WRONG for this nixpkgs: evaluation showed `propagatedBuildInputs = []` with buildInputs set (this stdenv does not auto-propagate). Added an explicit `propagatedBuildInputs = buildInputs;` with a comment recording the verification. Without it, a nix-env install or an inputsFrom shell would put only rfs's own bin/ on PATH. |
| Closure-proof method | Action 4 / exit criteria specified `nix path-info -r` as the closure proof, but that command traces FILE references inside the store output - propagated inputs are not file-referenced, so it shows the same 7 paths (libgcc/glibc/runtime) regardless of propagation. Replaced with two proofs: (1) Nix-level - evaluate `propagatedBuildInputs` and assert all eight tools are listed; (2) functional - assemble a PATH from the package + its propagated inputs' bin dirs and run the built `rfs test --suite unit` from it, proving the tools are reachable exactly as an installed user would reach them. |
| .gitignore + filter: result symlink | Discovered during verification: the store path of the built package CHANGED between two consecutive identical builds - the `nix build` output symlink `result` (repo root, recreated on every build) leaked into the cleanSourceWith'd source and the hash moved every rebuild. First fix attempt (adding `result` to `.gitignore`) was INSUFFICIENT: cleanSourceWith does NOT honor .gitignore when a custom filter is given - it walks the tree. Complete fix: (a) `result` added to `.gitignore` (one line + comment, for humans and other tools), AND (b) an explicit `builtins.baseNameOf path != "result"` clause in package.nix's source filter with a comment explaining why. Two consecutive builds with `result` present now yield identical store paths. `.gitignore` was not in the task's affected-files list; it is a prerequisite for the package being deterministic at all. |

## Other information
- If `pkgs.cargo-fuzz` turns out to be broken/absent in a future nixpkgs, the
  argument default can move to a locally built cargo-fuzz - but do not preempt;
  it evaluates fine in 26.11pre (verified).
