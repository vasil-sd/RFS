{ pkgs ? import <nixpkgs> { } }:
let
  manifest = (pkgs.lib.importTOML ./../Cargo.toml).package;
  # nixpkgs splits z3 into three outputs: `out` (bin/ only), `lib` (libz3.so) and
  # `dev` (include/z3.h + the pkg-config file, but no .so). The RFS tests assume a
  # SINGLE install dir with include/, lib/ and bin/: they compile generated C with
  # -I{z3}/include -L{z3}/lib, set Z3_SYS_Z3_DIR={z3} for z3-sys, and put {z3}/bin:
  # {z3}/lib on LD_LIBRARY_PATH. `pkg-config --variable=prefix z3` returns only the
  # bin-only `out`, whose include/ does not exist - so assemble a unified prefix by
  # copying the three outputs together and shipping a z3.pc that points at it. With
  # that pc first on PKG_CONFIG_PATH, the toolchain's z3() resolver lands here with
  # explicit=false, so z3-sys reads the real version (5.3.2) from the headers rather
  # than a pinned project-local one.
  unifiedZ3 = pkgs.runCommand "z3-unified-${pkgs.z3.version}" { } ''
    mkdir -p $out/include $out/lib $out/bin
    cp -r ${pkgs.z3.dev}/include/. $out/include/
    cp -r ${pkgs.z3.lib}/lib/. $out/lib/
    cp -r ${pkgs.z3.out}/bin/. $out/bin/ 2>/dev/null || true
    # project-local convention: libz3 also reachable under bin/ (Z3_LIBRARY_PATH_OVERRIDE)
    cp -r ${pkgs.z3.lib}/lib/libz3* $out/bin/ 2>/dev/null || true
    mkdir -p $out/lib/pkgconfig
    # Concrete $out-based paths (expanded by the shell at build time). The
    # heredoc keeps the pkg-config file free of any dollar-brace tokens that
    # Nix would otherwise try to interpolate inside this string.
    cat > $out/lib/pkgconfig/z3.pc <<PC
prefix=$out
exec_prefix=$out
libdir=$out/lib
includedir=$out/include
Name: z3
Description: Z3 Theorem Prover (unified prefix)
Version: ${pkgs.z3.version}
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
        && builtins.baseNameOf path != "all_rfs_training.jsonl";
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
  nativeBuildInputs = [ pkgs.gcc pkgs.clang pkgs.rustc pkgs.cargo pkgs.gnumake unifiedZ3 pkgs.pkg-config ];

  doCheck = true;
  checkPhase = ''
    # buildRustPackage does NOT propagate nativeBuildInputs' pkgconfig dirs to
    # PKG_CONFIG_PATH in the checkPhase (it is empty there), so neither the
    # toolchain's z3() resolver nor compile_c's `pkg-config --cflags --libs z3`
    # can find z3. Point it at the unified prefix explicitly. RFS_Z3_DIR / Z3_SYS_
    # Z3_DIR are left unset on purpose: that keeps z3()'s `explicit` flag false, so
    # z3-sys reads the real version (5.3.2) from the headers instead of a pinned
    # project-local one.
    # (PKG_CONFIG_PATH is empty in this phase, so a plain assignment suffices.)
    export PKG_CONFIG_PATH="${unifiedZ3}/lib/pkgconfig"
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
