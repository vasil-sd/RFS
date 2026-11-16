# Shared unified-z3-prefix derivation (Phase 15 task 4, design decision D3).
# package.nix and shell.nix both import this so there is ONE definition and zero
# drift between the package's runtime closure and the dev shell.
#
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
{ pkgs, z3 }: pkgs.runCommand "z3-unified-${z3.version}" { } ''
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
  ''