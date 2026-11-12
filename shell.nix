{ pkgs ? import <nixpkgs> { } }:

# Development shell for RFS. `inputsFrom` builds the package (nix/package.nix) and
# pulls its whole closure into the shell: the exact rust toolchain + every cargo
# dependency used to build rfs, AND the built `rfs` binary itself on PATH. So after
# `nix develop -f shell.nix`, both `rfs` and `rustc`/`cargo` just work.
#
# The extra buildInputs (gcc, clang, z3, pkg-config) are what the model-checking
# tests need: they generate C/Rust code, compile it with gcc/clang, and drive the
# SMT solver via z3. These land on PATH so RFS's tool resolvers find them (the
# project-local RFS_Z3_DIR default in .cargo/env.toml does not exist here and is
# skipped, falling through to PATH).
let
  # doCheck is turned off for the shell: entering a dev environment should be fast,
  # not re-run the entire model-checking suite (that is what `nix build` of the
  # package is for). The built `rfs` binary and its toolchain closure are identical
  # either way - only the check phase differs.
  rfs = pkgs.callPackage ./nix/package.nix { } // { doCheck = false; };
in
pkgs.mkShell {
  name = "rfs-dev";

  # The built `rfs` binary on PATH (the package's output).
  packages = [ rfs ];
  # The rust toolchain + every cargo dependency used to build it (the package's
  # build closure) - so `cargo build`/`cargo test` work inside the shell.
  inputsFrom = [ rfs ];

  buildInputs = with pkgs; [
    gcc
    clang
    z3
    pkg-config
  ];

  shellHook = ''
    echo "RFS dev shell"
    echo "  rustc: $(rustc --version 2>/dev/null || echo missing)"
    echo "  rfs:   $(command -v rfs || echo 'not on PATH (run: cargo build --release)')"
    echo "  gcc:   $(gcc --version 2>/dev/null | head -1 || echo missing)"
    echo "  clang: $(clang --version 2>/dev/null | head -1 || echo missing)"
    echo "  z3:    $(z3 --version 2>/dev/null || echo missing)"
  '';
}
