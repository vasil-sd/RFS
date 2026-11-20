#!/bin/sh
# specgen_mutation_loop (Phase 17 task 8.3): the rapid-prototyping loop from
# docs/user/INTEGRATION.md section 8 as a runnable thing. It generates a
# DETERMINISTIC base spec (pinned seed), proves byte-identity across two runs,
# validates it, model-checks it (PASS), then applies K mutations and shows the
# mutated variant FAILING - a captured counterexample is the teaching moment.
# The whole transcript is written to transcript.txt; the README pins excerpts
# of it and tests/example_projects_test.rs re-runs this script and asserts the
# pinned lines are present.
#
# Pinned parameters (a REAL run, seed-deterministic):
#   base:    --seed 17            -> PASS (complete)
#   mutate:  --mutations 8 --seed 1 -> FAIL, property p2 violated at the
#                                        initial configuration (step 0)
#
# Toolchain resolution mirrors rfs-build: RFS_BIN > workspace target/debug/rfs
# > bare `rfs` on PATH.
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

BASE_SEED=17
MUT_K=8
MUT_SEED=1

WORK=work
rm -rf "$WORK"
mkdir -p "$WORK"
T="$WORK/transcript.txt"
: > "$T"

# step <title> <cmd...>: run a command, appending its title + output to the
# transcript. mc-ref may exit 1 (FAIL) - expected on the mutated variant;
# any other nonzero aborts.
step() {
  title=$1; shift
  { echo "\$ $*"; } >> "$T"
  set +e; "$@" >> "$T" 2>&1; rc=$?; set -e
  echo "exit=$rc" >> "$T"
  if [ "$rc" -ne 0 ] && [ "$rc" -ne 1 ]; then
    echo "ERROR: step '$title' exited $rc" >&2
    exit "$rc"
  fi
  echo "--- $title (exit $rc) ---" >&2
}

echo "rfs: $RFS"
echo "pinned: base seed=$BASE_SEED, mutations K=$MUT_K seed=$MUT_SEED"
echo

# 1. generate the base spec (deterministic from the seed)
step "generate base spec (seed $BASE_SEED)" \
  "$RFS" gen-spec --out "$WORK/base.rfs" --seed "$BASE_SEED"

# 2. prove determinism: a second generation with the same seed is byte-identical
step "regenerate base spec (same seed, must be byte-identical)" \
  "$RFS" gen-spec --out "$WORK/base_again.rfs" --seed "$BASE_SEED"
if cmp -s "$WORK/base.rfs" "$WORK/base_again.rfs"; then
  echo "DETERMINISM: base.rfs and base_again.rfs are byte-identical" >> "$T"
else
  echo "DETERMINISM: FAILED - the two files differ" >> "$T"
  echo "ERROR: seed determinism broken" >&2
  exit 1
fi

# 3. validate the base (gen-spec output is always valid by construction)
step "validate base spec" "$RFS" validate "$WORK/base.rfs"

# 4. model-check the base (exact mode) - PASS
step "model-check base spec (exact)" "$RFS" mc-ref "$WORK/base.rfs"

# 5. mutate the base K times (seeded, so the mutation is reproducible)
step "mutate base spec (K=$MUT_K, seed $MUT_SEED)" \
  "$RFS" gen-spec --base "$WORK/base.rfs" --mutations "$MUT_K" --seed "$MUT_SEED" --out "$WORK/mut.rfs"

# 6. validate the mutated variant (mutation preserves validity)
step "validate mutated spec" "$RFS" validate "$WORK/mut.rfs"

# 7. model-check the mutated variant - FAIL with a captured counterexample
step "model-check mutated spec (exact) - expect FAIL" "$RFS" mc-ref "$WORK/mut.rfs"

echo
echo "transcript written to $T :"
cat "$T"
