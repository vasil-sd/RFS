#!/bin/sh
# RUSTC_WRAPPER for the unsloth-studio sandbox (Phase 18 task 3K).
#
# Why this exists: in this sandbox /dev is a sparse tmpfs and /dev/null has been
# observed to be DELETED after image build; any later write to the missing path
# (a shell redirect, or a compiler '-o /dev/null') recreates it as a REGULAR
# FILE holding that data (git-status text, ELF objects). rustc's cargo target-
# probe ('rustc - ...', source from stdin) then parses that content and dies
# with E0762 / "not valid UTF-8" on every cargo build/test/clippy.
#
# What this does: rewrites the probe's bare '-' source argument to a guaranteed-
# empty file and redirects rustc's stdin from that same empty file (NOT from
# /dev/null - task 3K hardening, so a corrupted device can never feed rustc).
# For normal file-argument compiles it is a no-op passthrough (identical args,
# only stdin is redirected and there is a few-ms shell overhead per invocation).
#
# Activate: export RUSTC_WRAPPER="$PWD/scripts/rustc-wrap.sh" (the TOOLCHAIN.md
# section-2 activation prefix and the BUILD.md verification checklist do this).
COMPILER=$1
shift
RUSTC_BIN=/workspace/host/rustup/toolchains/stable-x86_64-unknown-linux-gnu/bin/rustc
printf '' > /tmp/empty_src.rs
list=$(printf '%s\n' "$@")
newlist=$(printf '%s\n' "$list" | sed -e "s|^$RUSTC_BIN\$||" -e 's/^-$/\/tmp\/empty_src.rs/')
set --
while IFS= read -r line; do
  [ -n "$line" ] && set -- "$@" "$line"
done <<EOF2
$newlist
EOF2
exec "$COMPILER" "$@" < /tmp/empty_src.rs
