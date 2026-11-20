#!/bin/sh
# mc_walkthrough (Phase 17 task 8.1): generate MC binaries for BOTH backends on
# a passing spec and a failing variant, compile them, run exact mode, and
# capture the reports - the PASS report and the FAIL counterexample in text
# AND json. The captures/ directory is the source of truth for the README's
# end-to-end walkthrough; tests/example_projects_test.rs re-runs this script
# and pins excerpts from it.
#
# Toolchain resolution mirrors rfs-build: RFS_BIN > workspace target/debug/rfs
# > bare `rfs` on PATH. A cold checkout must run `cargo build` first (produces
# target/debug/rfs) or set RFS_BIN. rustc/gcc are resolved the standard way
# (RUSTC/GCC env vars > bare name on PATH).
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$HERE"

# --- resolve the rfs toolchain -------------------------------------------------
if [ -n "${RFS_BIN:-}" ] && [ -f "$RFS_BIN" ]; then
  RFS="$RFS_BIN"
elif [ -x "$HERE/../../../target/debug/rfs" ]; then
  RFS="$HERE/../../../target/debug/rfs"
else
  RFS="rfs"
fi

# --- resolve compilers ---------------------------------------------------------
# RUSTC: standard var > the project-local rustup toolchain (a cold checkout's
# toolchain is not on PATH) > bare `rustc` on PATH. GCC: standard var > bare
# name (gcc is expected on PATH; set GCC to force a specific compiler).
WSROOT=$(CDPATH= cd -- "$HERE/../../.." && pwd)
if [ -n "${RUSTC:-}" ] && [ -f "$RUSTC" ]; then
  RUSTC="$RUSTC"
elif [ -x "$WSROOT/../rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc" ]; then
  RUSTC=$(CDPATH= cd -- "$WSROOT/.." && pwd)/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc
else
  RUSTC="rustc"
fi
GCC=${GCC:-gcc}

BUILD=build
CAP=captures
rm -rf "$BUILD" "$CAP"
mkdir -p "$BUILD" "$CAP"
# build/ holds intermediate .rs/.c sources + compiled binaries (scratch); the
# durable output is captures/. Remove build/ on exit so no generated .rs files
# linger in the source tree (the inventory audit walks crates/ for .rs).
trap 'rm -rf "$BUILD"' EXIT

echo "rfs:      $RFS"
echo "rustc:    $RUSTC"
echo "gcc:      $GCC"
echo

# build_mc <spec> <tag>: generate + compile the Rust AND C MC binaries for one
# spec. Each backend's artifacts land together in build/<tag>/ under a BARE
# stem, so the Rust mc's include!("<stem>-fsm.rs") resolves relative to its own
# directory (an --out with a subdirectory prefix would double the path).
build_mc() {
  spec=$1; tag=$2
  mkdir -p "$BUILD/$tag"
  # Rust: <stem>-fsm.rs + <stem>-mc.rs in build/<tag>/.
  ( cd "$BUILD/$tag" && "$RFS" codegen rust mc "$HERE/$spec" --out "$tag-rust" )
  "$RUSTC" --edition 2021 "$BUILD/$tag/$tag-rust-mc.rs" -o "$BUILD/$tag/$tag-rust"
  # C: <stem>-fsm.{h,c} + <stem>-mc.{h,c}; object+link build, zero-warning bar.
  ( cd "$BUILD/$tag" && "$RFS" codegen c mc "$HERE/$spec" --out "$tag-c" )
  "$GCC" -Wall -c "$BUILD/$tag/$tag-c-fsm.c" -o "$BUILD/$tag/$tag-c-fsm.o"
  "$GCC" -Wall -c "$BUILD/$tag/$tag-c-mc.c" -o "$BUILD/$tag/$tag-c-mc.o"
  "$GCC" -Wall "$BUILD/$tag/$tag-c-fsm.o" "$BUILD/$tag/$tag-c-mc.o" -o "$BUILD/$tag/$tag-c"
}

# run_mc <binary> <name> <fmt>: run one MC binary in exact mode, capturing
# stdout to captures/<name>.<ext>. Exact mode exits 0 on PASS and 1 on FAIL -
# both are EXPECTED here (the failing variant must fail); any other exit code
# is a crash and aborts the script.
run_mc() {
  bin=$1; name=$2; fmt=$3
  if [ "$fmt" = json ]; then
    set +e; "$bin" --counterexample-format json > "$CAP/$name.json"; rc=$?; set -e
  else
    set +e; "$bin" > "$CAP/$name.txt"; rc=$?; set -e
  fi
  if [ "$rc" -ne 0 ] && [ "$rc" -ne 1 ]; then
    echo "ERROR: $bin exited $rc (expected 0=PASS or 1=FAIL)" >&2
    exit "$rc"
  fi
}

echo "=== generating + compiling MC binaries (both backends, both specs) ==="
build_mc pass.rfs pass
build_mc fail.rfs fail
echo "built: build/pass/{pass-rust,pass-c} build/fail/{fail-rust,fail-c}"
echo

echo "=== running exact mode on the PASSING spec (pass.rfs) ==="
run_mc "$BUILD/pass/pass-rust" pass-rust text
cat "$CAP/pass-rust.txt"
echo
echo "--- C backend (byte-identical report expected) ---"
run_mc "$BUILD/pass/pass-c" pass-c text
cat "$CAP/pass-c.txt"
echo

echo "=== running exact mode on the FAILING variant (fail.rfs) ==="
echo "--- Rust backend, TEXT counterexample ---"
run_mc "$BUILD/fail/fail-rust" fail-rust text
cat "$CAP/fail-rust.txt"
echo
echo "--- Rust backend, JSON counterexample ---"
run_mc "$BUILD/fail/fail-rust" fail-rust json
cat "$CAP/fail-rust.json"
echo
echo "--- C backend, TEXT counterexample (byte-identical to Rust) ---"
run_mc "$BUILD/fail/fail-c" fail-c text
cat "$CAP/fail-c.txt"
echo
echo "captures written to $CAP/ :"
ls -1 "$CAP"
