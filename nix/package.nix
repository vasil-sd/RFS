# Every external tool is an overridable argument (Phase 15 task 2): a user of
# the package passes their own versions, e.g.
#   import ./nix/package.nix { gcc = pkgs.gcc14; mermaid-cli = pkgs.mermaid-cli; }
# The defaults are the plain nixpkgs packages, so every existing no-arg call
# site (`import ./nix/package.nix { }`, `pkgs.callPackage ./nix/package.nix { }`)
# keeps working unchanged. mermaid-cli / plantuml / graphviz are OPTIONAL -
# null by default; when passed they join the runtime closure (and task 3's
# wrapper sets RFS_MERMAID / RFS_PLANTUML / RFS_DOT for them).
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
let
  manifest = (pkgs.lib.importTOML ./../Cargo.toml).package;
  # nixpkgs splits z3 into three outputs: `out` (bin/z3 CLI only), `lib`
  # (lib/libz3.so*) and `dev` (include/z3*.h + its own pkg-config file, whose
  # prefix= points at the bin-only `out`). The RFS tests assume a SINGLE install
  # dir with include/, lib/ AND bin/ - and bin/ must also hold libz3.so for the
  # Z3_LIBRARY_PATH_OVERRIDE convention: they compile generated C with
  # -I{z3}/include -L{z3}/lib, set Z3_SYS_Z3_DIR={z3} + Z3_LIBRARY_PATH_OVERRIDE=
  # {z3}/bin for z3-sys, and put {z3}/lib:{z3}/bin on LD_LIBRARY_PATH. So assemble
  # a unified prefix from the three outputs WITHOUT copying any of their files -
  # every piece is an ordinary symlink, plus one generated z3.pc whose prefix=
  # points at the unified dir. With that pc on PKG_CONFIG_PATH, the toolchain's
  # z3() resolver lands here with explicit=false, so z3-sys reads the real version
  # (5.3.2) from the headers rather than a pinned project-local one.
  #
  # Why not symlinkJoin: this nixpkgs' symlinkJoin builds its tree with `lndir`,
  # which dereferences regular files - it COPIES the 44 MB libz3.so.5.3.2.0 (into
  # both lib/ and bin/) and every header instead of linking them. Plain `ln -s`
  # keeps the whole prefix at zero copied bytes: include/ and lib/ are single
  # symlinks to the store output dirs, and bin/ is a real dir whose entries are all
  # symlinks (it must be real because it merges the z3 CLI from `out` with
  # libz3.so* from `lib`). The z3.pc is generated in the same derivation, so its
  # prefix=$out needs no self-reference cycle.
  unifiedZ3 = pkgs.runCommand "z3-unified-${z3.version}" { } ''
    mkdir -p $out/bin $out/pkgconfig
    # include/ and lib/: whole-directory symlinks to the store outputs (zero copy).
    ln -s ${z3.dev}/include $out/include
    ln -s ${z3.lib}/lib $out/lib
    # bin/: the z3 CLI from `out`, plus libz3.so* from `lib` (the project-local
    # Z3_LIBRARY_PATH_OVERRIDE convention) - every entry a symlink, nothing copied.
    for f in ${z3.out}/bin/*; do ln -s "$f" "$out/bin/$(basename "$f")"; done
    for f in ${z3.lib}/lib/libz3*; do ln -s "$f" "$out/bin/$(basename "$f")"; done
    # The one generated file: a pkg-config module whose prefix= names this unified
    # dir. Concrete $out-based paths are expanded by the shell at build time; the
    # heredoc keeps the .pc free of any dollar-brace tokens Nix would interpolate.
    cat > $out/pkgconfig/z3.pc <<PC
prefix=$out
exec_prefix=$out
libdir=$out/lib
includedir=$out/include
Name: z3
Description: Z3 Theorem Prover (unified prefix)
Version: ${z3.version}
Libs: -L$out/lib -lz3
Cflags: -I$out/include
PC
  '';
in
pkgs.rustPlatform.buildRustPackage rec {
  pname = manifest.name;        # rfs
  version = manifest.version;  # 1.0.0

  # The whole repo is the source (a cargo workspace rooted at `rfs`). Only the
  # root package `rfs` is compiled - the other workspace members just need their
  # manifests present so cargo can parse the workspace. cleanSourceWith drops .git
  # and gitignored paths, PLUS any `target/` build-artifact directory (the fuzz_rust
  # example carries ~694 files / ~50MB of libfuzzer build output under a nested
  # fuzz/target that the root-level `fuzz/target` gitignore pattern does not match)
  # and the two large regenerable data files, keeping the source small on slow I/O.
  src = pkgs.lib.cleanSourceWith {
    src = ./..;
    filter = path: type:
      let parts = builtins.split "/" (toString path);
      in
        !(builtins.elem "target" parts)
        && builtins.baseNameOf path != "commands.md"
        && builtins.baseNameOf path != "all_rfs_training.jsonl"
        # `result` is the nix build output symlink, recreated in the repo root
        # on every `nix build`. cleanSourceWith does NOT honor .gitignore when
        # a custom filter is given, so it must be excluded here explicitly -
        # otherwise the previous build's store path leaks into the source and
        # the package hash changes on every rebuild.
        && builtins.baseNameOf path != "result";
  };

  # plantuml-core (and its 8 sibling crates) come from a pinned git rev, not
  # crates.io. Nix vendors git deps hermetically and verifies each with an
  # outputHash; all nine share the hash of that one repo checkout (fetched via
  # nix-prefetch-git at rev 8066f6124a86ed5da59c6d7d570f0568891c378e).
  cargoLock = {
    lockFile = ./../Cargo.lock;
    outputHashes = {
      "plantuml-ast-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-core-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-layout-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-model-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-parser-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-preprocessor-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-renderer-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-stdlib-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
      "plantuml-themes-0.2.0" = "0dqhxrx39pw0lxr44mcql11fqcg8iyzq5fkxcn6sicrrkf2vxya2";
    };
  };

  # Goal 2: the model-checking tests take a spec, generate a C/Rust bounded-MC
  # binary via the codegen backend, compile it, and run it. They locate their
  # tools via RFS_GCC / RFS_CLANG / RFS_RUSTC / RFS_MAKE (non-forcing) with a PATH
  # fallback, so putting the toolchains on PATH is sufficient. z3 + pkg-config
  # back the SMT-bounded test (the `smt` feature links libz3).
  # Runtime tool closure (Phase 15 task 2): these are PROPAGATED into the
  # installed package so `rfs test ...` works from a bare install - rustc/cargo
  # compile generated code, gcc/clang build C artifacts (clang carries the
  # libFuzzer runtime for --suite fuzz), make drives the C example projects,
  # unifiedZ3 + pkg-config back SMT mode, cargo-fuzz scaffolds the Rust fuzz
  # pipeline. buildInputs are on PATH during checkPhase too, so the existing
  # check scripts keep working unchanged. The BUILD itself still uses
  # buildRustPackage's callPackage-injected rustc/cargo (nixpkgs defaults -
  # they are not public arguments of buildRustPackage in this nixpkgs), so a
  # user-passed rustc/cargo lands in the runtime closure + wrapper, not in the
  # build toolchain.
  buildInputs = [ rustc cargo gcc clang make unifiedZ3 pkg-config cargo-fuzz ]
    ++ pkgs.lib.optionals (mermaid-cli != null) [ mermaid-cli ]
    ++ pkgs.lib.optionals (plantuml != null) [ plantuml ]
    ++ pkgs.lib.optionals (graphviz != null) [ graphviz ];

  # This nixpkgs' stdenv does NOT auto-propagate buildInputs (verified by
  # evaluation: propagatedBuildInputs was [] with them set), so declare the
  # runtime closure explicitly - otherwise a nix-env install / inputsFrom shell
  # would put only rfs's own bin/ on PATH and none of the tools.
  propagatedBuildInputs = buildInputs;

  nativeBuildInputs = [ ];

  doCheck = true;
  checkPhase = ''
    # buildRustPackage does NOT propagate nativeBuildInputs' pkgconfig dirs to
    # PKG_CONFIG_PATH in the checkPhase (it is empty there), so neither the
    # toolchain's z3() resolver nor compile_c's `pkg-config --cflags --libs z3`
    # can find z3. Point it at the unified prefix's generated pkgconfig dir
    # (its z3.pc prefix= line names that same unified dir). RFS_Z3_DIR / Z3_SYS_
    # Z3_DIR are left unset on purpose: that keeps z3()'s `explicit` flag false, so
    # z3-sys reads the real version (5.3.2) from the headers instead of a pinned
    # project-local one.
    # (PKG_CONFIG_PATH is empty in this phase, so a plain assignment suffices.)
    export PKG_CONFIG_PATH="${unifiedZ3}/pkgconfig"
    runHook preCheck
    # Core reference/parity model checking (no smt feature): generate + compile
    # (gcc/clang/rustc/make) + run a bounded MC over the example specs.
    cargo test --test backend_parity_test --test prod_parity_test \
      --test ref_mc_diag_test --test ref_mc_diff_test --test ref_mc_sweep_test \
      --test liveness_mc_test --test monitor_ref_test --test monitor_integration_test \
      --test wide_ops_parity_test --test benchmark_mc_test
    # SMT-bounded mode: rebuild with the `smt` feature (links libz3) and check via z3.
    cargo test --features smt --test smt_bounded_test
    runHook postCheck
  '';

  # RFS_* wrapper (Phase 15 task 3, design decision D2): one shared preamble
  # script $out/bin/rfs-env.sh that every wrapped binary sources before exec'ing
  # the real one. It (a) prepends the package's propagated tool bin dirs to PATH
  # - store bins are NOT on PATH for a bare install, and `cargo fuzz` needs the
  # cargo-fuzz subcommand findable there - and (b) sets the RFS_* tool-
  # resolution contract (TOOLCHAIN.md section 8) ONLY when the user has not
  # already defined the variable: user redefinition always wins. makeWrapper
  # --set is NOT usable for this - it overrides unconditionally. The preamble is
  # generated by Nix string interpolation, so passing different tool arguments
  # yields a different preamble (no runtime detection). checkPhase's z3 logic
  # stays untouched on purpose: it is BUILD time (explicit=false so z3-sys reads
  # the real version from headers); this wrapper is RUNTIME.
  postFixup = let
    # A literal "${" for runtime shell expansions: Nix interpolates ${...} in
    # strings (and $$ is NOT an escape in this nix - verified empirically), so
    # the brace group is assembled from pieces: db = "$" + "{".
    db = "$" + "{";
  in ''
    cat > $out/bin/rfs-env.sh <<'ENVEOF'
# Generated by nix/package.nix (Phase 15 task 3). Sourced by every wrapped RFS
# binary. Prepend the closure's tool bin dirs to PATH, then set the RFS_*
# contract vars conditionally - an already-defined user value always wins.
PATH="${rustc}/bin:${cargo}/bin:${gcc}/bin:${clang}/bin:${make}/bin:${unifiedZ3}/bin:${pkg-config}/bin:${cargo-fuzz}/bin${pkgs.lib.optionalString (graphviz != null) "${graphviz}/bin"}${pkgs.lib.optionalString (plantuml != null) "${plantuml}/bin"}${pkgs.lib.optionalString (mermaid-cli != null) "${mermaid-cli}/bin"}:${db}PATH}"
export PATH
if [ -z "${db}RFS_RUSTC:-}" ]; then export RFS_RUSTC=${rustc}/bin/rustc; fi
if [ -z "${db}RFS_GCC:-}" ]; then export RFS_GCC=${gcc}/bin/gcc; fi
if [ -z "${db}RFS_CLANG:-}" ]; then export RFS_CLANG=${clang}/bin/clang; fi
if [ -z "${db}RFS_CARGO_FUZZ:-}" ]; then export RFS_CARGO_FUZZ=${cargo}/bin/cargo; fi
if [ -z "${db}RFS_MAKE:-}" ]; then export RFS_MAKE=${make}/bin/make; fi
if [ -z "${db}RFS_Z3_DIR:-}" ]; then export RFS_Z3_DIR=${unifiedZ3}; fi
if [ -z "${db}Z3_SYS_Z3_DIR:-}" ]; then export Z3_SYS_Z3_DIR=${unifiedZ3}; fi
if [ -z "${db}Z3_LIBRARY_PATH_OVERRIDE:-}" ]; then export Z3_LIBRARY_PATH_OVERRIDE=${unifiedZ3}/bin; fi
# PKG_CONFIG_PATH is always prepended (idempotent-harmless: the unified z3.pc
# just makes `pkg-config z3` resolvable; user entries are preserved after it).
if [ -n "${db}PKG_CONFIG_PATH:-}" ]; then export PKG_CONFIG_PATH=${unifiedZ3}/pkgconfig:${db}PKG_CONFIG_PATH}; else export PKG_CONFIG_PATH=${unifiedZ3}/pkgconfig; fi
${pkgs.lib.optionalString (graphviz != null) "if [ -z \"${db}RFS_DOT:-}\" ]; then export RFS_DOT=${graphviz}/bin/dot; fi\n"}${pkgs.lib.optionalString (plantuml != null) "if [ -z \"${db}RFS_PLANTUML:-}\" ]; then export RFS_PLANTUML=${plantuml}/bin/plantuml; fi\n"}${pkgs.lib.optionalString (mermaid-cli != null) "if [ -z \"${db}RFS_MERMAID:-}\" ]; then export RFS_MERMAID=${mermaid-cli}/bin/mmdc; fi\n"}
ENVEOF
    # One thin wrapper per binary: source the preamble, exec the real one. The
    # QUOTED heredoc keeps every runtime shell token ($0, $dir, $@, $(...))
    # literal - Nix only interpolates brace groups, and there are none inside;
    # the binary name is hardcoded per block (a shell loop variable would not
    # be visible to Nix, and an unquoted heredoc would let the build-time shell
    # expand $dir/$@ to empty).
    mv $out/bin/rfs $out/bin/rfs.real
    cat > $out/bin/rfs <<'WRAP'
#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Invoked via PATH, $0 is bare and dir is the CWD - re-locate through PATH.
[ -f "$dir/rfs-env.sh" ] || dir=$(dirname -- "$(command -v -- "$0")")
. "$dir/rfs-env.sh"
exec "$dir/rfs.real" "$@"
WRAP
    chmod +x $out/bin/rfs
    mv $out/bin/build_training $out/bin/build_training.real
    cat > $out/bin/build_training <<'WRAP'
#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Invoked via PATH, $0 is bare and dir is the CWD - re-locate through PATH.
[ -f "$dir/rfs-env.sh" ] || dir=$(dirname -- "$(command -v -- "$0")")
. "$dir/rfs-env.sh"
exec "$dir/build_training.real" "$@"
WRAP
    chmod +x $out/bin/build_training
    mv $out/bin/extract_training_snippets $out/bin/extract_training_snippets.real
    cat > $out/bin/extract_training_snippets <<'WRAP'
#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Invoked via PATH, $0 is bare and dir is the CWD - re-locate through PATH.
[ -f "$dir/rfs-env.sh" ] || dir=$(dirname -- "$(command -v -- "$0")")
. "$dir/rfs-env.sh"
exec "$dir/extract_training_snippets.real" "$@"
WRAP
    chmod +x $out/bin/extract_training_snippets
    mv $out/bin/generate_code_tests $out/bin/generate_code_tests.real
    cat > $out/bin/generate_code_tests <<'WRAP'
#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Invoked via PATH, $0 is bare and dir is the CWD - re-locate through PATH.
[ -f "$dir/rfs-env.sh" ] || dir=$(dirname -- "$(command -v -- "$0")")
. "$dir/rfs-env.sh"
exec "$dir/generate_code_tests.real" "$@"
WRAP
    chmod +x $out/bin/generate_code_tests
    mv $out/bin/test_generated_code $out/bin/test_generated_code.real
    cat > $out/bin/test_generated_code <<'WRAP'
#!/bin/sh
dir=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
# Invoked via PATH, $0 is bare and dir is the CWD - re-locate through PATH.
[ -f "$dir/rfs-env.sh" ] || dir=$(dirname -- "$(command -v -- "$0")")
. "$dir/rfs-env.sh"
exec "$dir/test_generated_code.real" "$@"
WRAP
    chmod +x $out/bin/test_generated_code
  '';

  meta = with pkgs.lib; {
    description = "RFS - reactive finite-state machine specification + toolchain";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "rfs";
  };
}
