{ pkgs ? import <nixpkgs> { } }:

# Development shell for RFS with FULL Ubuntu-parity test-suite coverage (Phase 15
# task 4). `inputsFrom` builds the package (nix/package.nix) and pulls its whole
# closure into the shell: the exact rust toolchain + every cargo dependency used to
# build rfs, AND the built `rfs` binary itself on PATH. So after
# `nix develop -f shell.nix`, both `rfs` and `rustc`/`cargo` just work.
#
# The buildInputs add every external tool a runnable suite needs so that EVERY test
# suite that runs on Ubuntu also runs green here: gcc/clang/rustc/cargo/make compile
# generated code, unifiedZ3 + pkg-config back SMT mode (raw pkgs.z3 is dropped - its
# split out/lib/dev outputs have no single prefix with include/ + lib/ + bin/, which
# made SMT tests silently skip in the old shell), cargo-fuzz + rustup drive the Rust
# fuzz pipeline, graphviz/plantuml/mermaid-cli make the diagram parity test's external
# half ACTIVE, and python3 lets rfs-dev's port-equality call run. These land on PATH
# so RFS's tool resolvers find them (the project-local RFS_Z3_DIR default in
# .cargo/env.toml does not exist here and is skipped, falling through to PATH).
let
  # The tool arguments - bound ONCE so the shell's buildInputs and the package's
  # callPackage receive the IDENTICAL derivations: shell and package always agree on
  # which compiler/solver/diagram tools are in play. A user overrides any of them by
  # passing a different value to this file (e.g. `nix develop -f shell.nix --argstr`
  # or an import with { gcc = pkgs.gcc14; }).
  rustc = pkgs.rustc;
  cargo = pkgs.cargo;
  gcc = pkgs.gcc;
  clang = pkgs.clang;
  make = pkgs.gnumake;
  z3 = pkgs.z3;
  pkg-config = pkgs.pkg-config;
  cargo-fuzz = pkgs.cargo-fuzz;
  mermaid-cli = pkgs.mermaid-cli;
  plantuml = pkgs.plantuml;
  graphviz = pkgs.graphviz;

  # The shared unified-z3 prefix (design decision D3): the SAME derivation
  # nix/package.nix imports, so the shell's SMT mode and the package's runtime
  # closure resolve z3 identically. One definition, zero drift.
  unifiedZ3 = import ./nix/z3-unified.nix { inherit pkgs z3; };

  # doCheck is turned off for the shell: entering a dev environment should be fast,
  # not re-run the entire model-checking suite (that is what `nix build` of the
  # package is for). The built `rfs` binary and its toolchain closure are identical
  # either way - only the check phase differs. The identical tool arguments above are
  # passed through so the shell's package is byte-for-byte the default-args package.
  rfs = pkgs.callPackage ./nix/package.nix {
    inherit rustc cargo gcc clang make z3 pkg-config cargo-fuzz mermaid-cli plantuml graphviz;
  } // { doCheck = false; };
in
pkgs.mkShell {
  name = "rfs-dev";

  # The wrapped `rfs` binary on PATH (the package's output). Task 3's wrapper sources
  # bin/rfs-env.sh, so `rfs test ...` from the shell is already deterministic.
  packages = [ rfs ];
  # The rust toolchain + every cargo dependency used to build it (the package's
  # build closure) - so `cargo build`/`cargo test` work inside the shell.
  inputsFrom = [ rfs ];

  # Full Ubuntu-parity tool set on PATH. unifiedZ3 (not raw z3) backs SMT mode;
  # rustup provides the nightly for the Rust fuzz pipeline; python3 un-gates
  # rfs-dev's port-equality call. The diagram tools make the parity test's external
  # half run instead of skip.
  buildInputs = [
    rustc cargo gcc clang make unifiedZ3 pkg-config cargo-fuzz
    graphviz plantuml mermaid-cli
    pkgs.rustup pkgs.python3
  ];

  shellHook = ''
    echo "RFS dev shell (full Ubuntu-parity tool set)"
    # The unified z3 prefix keeps its generated z3.pc in <prefix>/pkgconfig - a
    # NON-standard location mkShell does not scan (it only adds lib/pkgconfig and
    # share/pkgconfig of each buildInput). Expose it explicitly so `pkg-config z3`
    # resolves to the nix store z3: this is what lets the SMT suites run with the
    # unified prefix (explicit=false, z3-sys reads the real version from headers)
    # instead of silently falling back to a host-local install. Mirrors the
    # package's checkPhase. $PKG_CONFIG_PATH is bare (no braces) so Nix leaves it
    # for the shell; ${unifiedZ3} is interpolated by Nix to the store path.
    if [ -n "$PKG_CONFIG_PATH" ]; then export PKG_CONFIG_PATH="${unifiedZ3}/pkgconfig:$PKG_CONFIG_PATH"; else export PKG_CONFIG_PATH="${unifiedZ3}/pkgconfig"; fi
    # Idempotent nightly install for the Rust fuzz pipeline: cargo-fuzz is built
    # against stable, but the scaffold's rust-toolchain.toml pin selects nightly for
    # the temp fuzz project. FIRST entry needs network to download the nightly
    # toolchain; afterwards it is cached in the profile dir and this is a no-op.
    if command -v rustup >/dev/null 2>&1; then
      if ! rustup toolchain list 2>/dev/null | grep -q '^nightly'; then
        echo "  (first entry: downloading the nightly toolchain via rustup - needs network)"
        rustup toolchain install nightly --profile minimal \
          || echo "  WARNING: nightly download failed (offline?); C-side suites still run"
      fi
    fi
    echo "  resolved tool matrix:"
    echo "    rustc:      $(rustc --version 2>/dev/null | head -1 || echo missing)"
    echo "    cargo:      $(cargo --version 2>/dev/null | head -1 || echo missing)"
    echo "    gcc:        $(gcc --version 2>/dev/null | head -1 || echo missing)"
    echo "    clang:      $(clang --version 2>/dev/null | head -1 || echo missing)"
    echo "    make:       $(make --version 2>/dev/null | head -1 || echo missing)"
    echo "    z3:         $(z3 --version 2>/dev/null || echo missing)"
    echo "    dot:        $(dot -V 2>&1 | head -1 || echo missing)"
    echo "    plantuml:   $(plantuml -version 2>&1 | head -1 || echo missing)"
    echo "    mmdc:       $(mmdc --version 2>/dev/null | head -1 || echo missing)"
    echo "    cargo-fuzz: $(cargo fuzz --version 2>/dev/null | head -1 || echo missing)"
    echo "    python3:    $(python3 --version 2>/dev/null || echo missing)"
    echo "    rfs:        $(command -v rfs || echo 'not on PATH (run: cargo build --release)')"
    echo "    rustup toolchains:"
    rustup toolchain list 2>/dev/null | sed 's/^/      /' || echo "      (rustup not available)"
  '';
}
