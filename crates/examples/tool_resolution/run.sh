#!/bin/sh
# tool_resolution (Phase 17 task 8.4): explicit external-tool path passing end
# to end. The script sets EXPLICIT RFS_* paths, runs `rfs test --print-script`
# for the C and Rust backends, and captures the `# tools:` header lines that
# prove each tool resolved to the exact path given. It then demonstrates the
# precedence contract (TOOLCHAIN.md section 8.2): CLI flag > RFS_* env var >
# standard Unix var > auto-discovery - a --gcc flag beats the RFS_GCC env var,
# and with nothing set bare PATH resolution applies.
#
# `--print-script` prints the resolved invocation as a self-contained POSIX sh
# script whose header carries a `# tools: <name>=<path> ...` line - that line
# is the proof artifact (the tool is NOT invoked in print-script mode).
#
# Toolchain resolution for rfs itself mirrors rfs-build: RFS_BIN > workspace
# target/debug/rfs > bare `rfs` on PATH.
set -eu

HERE=$(CDPATH= cd -- "$(dirname -- "$0")" && pwd)
cd "$HERE"

if [ -n "${RFS_BIN:-}" ] && [ -f "$RFS_BIN" ]; then
  RFS="$RFS_BIN"
elif [ -x "$HERE/../../../target/debug/rfs" ]; then
  RFS="$HERE/../../../target/debug/rfs"
else
  RFS="rfs"
fi

# Resolve a REAL explicit rustc path for the demo: RUSTC env var (when it names
# an existing file) > the workspace toolchain's rustc > bare `rustc`. The point
# is to show an ABSOLUTE path winning, so prefer a real one over the bare name.
# (The workspace root is $HERE/../../.. ; the project-local rustup install is a
# sibling of it. Canonicalize both so the captured # tools: line carries a clean
# absolute path, not literal ../ segments.)
WSROOT=$(CDPATH= cd -- "$HERE/../../.." && pwd)
RUSTUP_BIN="$WSROOT/../rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc"
if [ -n "${RUSTC:-}" ] && [ -f "$RUSTC" ]; then
  RUSTC_EXPLICIT=$(CDPATH= cd -- "$(dirname -- "$RUSTC")" && pwd)/$(basename -- "$RUSTC")
elif [ -x "$RUSTUP_BIN" ]; then
  RUSTC_EXPLICIT=$(CDPATH= cd -- "$WSROOT/.." && pwd)/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc
elif command -v rustc >/dev/null 2>&1; then
  RUSTC_EXPLICIT=$(command -v rustc)
else
  RUSTC_EXPLICIT="rustc"
fi
GCC_EXPLICIT=${RFS_GCC_DEMO:-/usr/bin/gcc}
CLANG_EXPLICIT=${RFS_CLANG_DEMO:-/usr/bin/clang}

CAP=captures
rm -rf "$CAP"
mkdir -p "$CAP"

echo "rfs:            $RFS"
echo "gcc (explicit): $GCC_EXPLICIT"
echo "rustc (explicit): $RUSTC_EXPLICIT"
echo "clang (explicit): $CLANG_EXPLICIT"
echo

# tools_line <capture-name> <cmd...>: run a --print-script invocation, save the
# full script, and echo the `# tools:` line(s) - the proof artifact.
tools_line() {
  name=$1; shift
  "$@" > "$CAP/$name.sh"
  echo "--- $name ---"
  grep "^# tools:" "$CAP/$name.sh" || echo "(no # tools: line)"
  echo
}

echo "=== 1. EXPLICIT env vars win (RFS_GCC / RFS_RUSTC / RFS_CLANG set) ==="
tools_line explicit-c \
  env RFS_GCC="$GCC_EXPLICIT" RFS_RUSTC="$RUSTC_EXPLICIT" RFS_CLANG="$CLANG_EXPLICIT" \
      "$RFS" test --backend c --suite unit --print-script pass.rfs
tools_line explicit-rust \
  env RFS_GCC="$GCC_EXPLICIT" RFS_RUSTC="$RUSTC_EXPLICIT" RFS_CLANG="$CLANG_EXPLICIT" \
      "$RFS" test --backend rust --suite unit --print-script pass.rfs

echo "=== 2. PRECEDENCE: a CLI flag beats the env var ==="
tools_line flag-beats-env \
  env RFS_GCC="$GCC_EXPLICIT" \
      "$RFS" test --backend c --suite unit --gcc /bin/sh-fake --print-script pass.rfs

echo "=== 3. NOTHING set: bare PATH resolution applies ==="
# Explicitly clear the RFS_* vars so this scenario is independent of the
# caller's environment - the resolver must fall through to bare PATH.
tools_line bare-path \
  env -u RFS_GCC -u RFS_RUSTC -u RFS_CLANG \
      "$RFS" test --backend c --suite unit --print-script pass.rfs

echo "captures written to $CAP/ (full print-script output per scenario):"
ls -1 "$CAP"
