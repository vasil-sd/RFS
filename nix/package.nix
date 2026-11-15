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

  meta = with pkgs.lib; {
    description = "RFS - reactive finite-state machine specification + toolchain";
    license = licenses.mit;
    platforms = platforms.linux;
    mainProgram = "rfs";
  };
}
