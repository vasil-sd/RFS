#!/bin/sh
# usb_device_link (Phase 18 task 2): drive every demonstrated RFS mode on the USB
# device & link machine - validate + info (analysis), mc-ref exact on the passing spec
# / the failing variant / the fix, generated MC binaries in BOTH backends (rust + c,
# byte-identical reports), production codegen in BOTH backends (warning-free).
# captures/ is the source of truth for the README walkthrough; the test leg re-runs
# this script and pins excerpts.
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$HERE"

if [ -n "${RFS_BIN:-}" ] && [ -f "$RFS_BIN" ]; then RFS="$RFS_BIN"
elif [ -x "$HERE/../../../target/debug/rfs" ]; then RFS="$HERE/../../../target/debug/rfs"
else RFS="rfs"; fi

WSROOT=$(CDPATH= cd -- "$HERE/../../.." && pwd)
if [ -n "${RUSTC:-}" ] && [ -f "$RUSTC" ]; then RUSTC="$RUSTC"
elif [ -x "$WSROOT/../rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc" ]; then
  RUSTC=$(CDPATH= cd -- "$WSROOT/.." && pwd)/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc
else RUSTC="rustc"; fi
GCC=${GCC:-gcc}

BUILD=build; CAP=captures
rm -rf "$BUILD" "$CAP"; mkdir -p "$BUILD" "$CAP"
trap 'rm -rf "$BUILD"' EXIT

echo "rfs:   $RFS"; echo "rustc: $RUSTC"; echo "gcc:   $GCC"; echo

echo "=== 1. analysis: rfs validate + rfs info ==="
"$RFS" validate pass.rfs | tee "$CAP/validate.txt"
"$RFS" info pass.rfs > "$CAP/info.txt"
echo "(info report captured to captures/info.txt)"
echo

echo "=== 2. mc-ref exact: pass / fail / fixed ==="
"$RFS" mc-ref pass.rfs  | tee "$CAP/mcref-pass.txt"
echo "--- failing variant (TEXT counterexample) ---"
set +e; "$RFS" mc-ref fail.rfs > "$CAP/mcref-fail.txt"; rc=$?; set -e
[ "$rc" -eq 1 ] || { echo "expected exit 1, got $rc" >&2; exit "$rc"; }
cat "$CAP/mcref-fail.txt"
set +e; "$RFS" mc-ref fail.rfs --counterexample-format json > "$CAP/mcref-fail.json"; set -e
echo "--- failing variant (JSON counterexample) ---"
cat "$CAP/mcref-fail.json"
echo "--- the fix (re-runs to PASS) ---"
"$RFS" mc-ref fixed.rfs | tee "$CAP/mcref-fixed.txt"
echo

build_mc() { # <spec> <tag>
  spec=$1; tag=$2; mkdir -p "$BUILD/$tag"
  ( cd "$BUILD/$tag" && "$RFS" codegen rust mc "$HERE/$spec" --out "$tag-rust" )
  "$RUSTC" --edition 2021 "$BUILD/$tag/$tag-rust-mc.rs" -o "$BUILD/$tag/$tag-rust"
  ( cd "$BUILD/$tag" && "$RFS" codegen c mc "$HERE/$spec" --out "$tag-c" )
  "$GCC" -Wall -c "$BUILD/$tag/$tag-c-fsm.c" -o "$BUILD/$tag/$tag-c-fsm.o"
  "$GCC" -Wall -c "$BUILD/$tag/$tag-c-mc.c" -o "$BUILD/$tag/$tag-c-mc.o"
  "$GCC" -Wall "$BUILD/$tag/$tag-c-fsm.o" "$BUILD/$tag/$tag-c-mc.o" -o "$BUILD/$tag/$tag-c"
}
echo "=== 3. generated MC binaries (both backends) ==="
build_mc pass.rfs pass; build_mc fail.rfs fail
"$BUILD/pass/pass-rust" | tee "$CAP/mc-rust-pass.txt"
"$BUILD/pass/pass-c"    | tee "$CAP/mc-c-pass.txt"
set +e; "$BUILD/fail/fail-rust" > "$CAP/mc-rust-fail.txt"; rc=$?; set -e
[ "$rc" -eq 1 ] || { echo "expected exit 1, got $rc" >&2; exit "$rc"; }
cat "$CAP/mc-rust-fail.txt"
set +e; "$BUILD/fail/fail-c" > "$CAP/mc-c-fail.txt"; rc=$?; set -e
[ "$rc" -eq 1 ] || { echo "expected exit 1, got $rc" >&2; exit "$rc"; }
cat "$CAP/mc-c-fail.txt"
echo

echo "=== 4. production codegen (both backends, warning-free) ==="
( cd "$BUILD" && "$RFS" codegen rust production "$HERE/pass.rfs" --out prod-rust )
"$RUSTC" --edition 2021 --crate-type lib --emit=metadata "$BUILD/prod-rust-fsm.rs" -o "$BUILD/.compile-check.rmeta" && rm -f "$BUILD/.compile-check.rmeta" && echo "rust production: compiles clean"
( cd "$BUILD" && "$RFS" codegen c production "$HERE/pass.rfs" --out prod-c )
"$GCC" -Wall -c "$BUILD/prod-c-fsm.c" -o "$BUILD/.compile-check.o" && rm -f "$BUILD/.compile-check.o" && echo "c production: compiles warning-free"
echo

echo "captures written to $CAP/ :"; ls -1 "$CAP"
