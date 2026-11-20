#!/bin/sh
# json_schema_ci (Phase 17 task 8.2): prove the five self-describing JSON
# document formats are CI-usable WITHOUT the rfs source. For each format the
# script (1) emits its schema via `rfs info format <name> --output schema`,
# (2) generates a REAL document of that kind with the toolchain, and (3)
# validates the document against the schema with `python3 -m jsonschema`.
# Availability-gated: if python3 + the jsonschema module are absent the script
# prints a note and exits 0 (the test leg skips rather than fails).
#
# The five formats (rfs info format --list):
#   spec-json     IR v1 JSON document          <- rfs parse --output json
#   mc-report     full exact-mode MC report    <- rfs mc-ref --counterexample-format json
#   ce-trace      the report's counterexample  <- the mc-report's "counterexample" object
#   validate-json rfs validate --json output   <- rfs validate --json on an invalid spec
#   info-json     rfs info --json introspection<- rfs info --json
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

echo "rfs: $RFS"

# --- availability gate: python3 + jsonschema ---------------------------------
if ! command -v python3 >/dev/null 2>&1; then
  echo "note: python3 not available - json_schema_ci check skipped"
  exit 0
fi
if ! python3 -c 'import jsonschema' >/dev/null 2>&1; then
  echo "note: python3 jsonschema module not available - json_schema_ci check skipped"
  exit 0
fi
echo "jsonschema: $(python3 -c 'import jsonschema; print(jsonschema.__version__)')"
echo

WORK=work
rm -rf "$WORK"
mkdir -p "$WORK/schemas" "$WORK/docs"

# --- step 1: emit all five schemas -------------------------------------------
echo "=== emitting schemas (rfs info format <name> --output schema) ==="
for fmt in spec-json ce-trace mc-report validate-json info-json; do
  "$RFS" info format "$fmt" --output schema > "$WORK/schemas/$fmt.json"
  echo "  $fmt: $(wc -c < "$WORK/schemas/$fmt.json") bytes"
done
echo

# --- step 2: generate one REAL document per format ---------------------------
echo "=== generating real documents ==="

# spec-json: parse the valid spec through the IR to JSON.
"$RFS" parse --input rfs --output json --target-dir "$WORK/docs" pass.rfs >/dev/null
mv "$WORK/docs/pass.json" "$WORK/docs/spec-json.json"
echo "  spec-json: from rfs parse (pass.rfs)"

# mc-report: exact-mode reference MC on the FAILING variant, json format.
# mc-ref exits 1 on FAIL - expected here; capture stdout regardless.
set +e; "$RFS" mc-ref fail.rfs --counterexample-format json > "$WORK/docs/mc-report.json"; rc=$?; set -e
[ "$rc" -eq 1 ] || { echo "ERROR: mc-ref exited $rc (expected 1=FAIL)" >&2; exit "$rc"; }
echo "  mc-report: from rfs mc-ref (fail.rfs, FAIL as intended)"

# ce-trace: the counterexample object extracted from the mc-report document.
python3 - "$WORK/docs/mc-report.json" "$WORK/docs/ce-trace.json" <<'PY'
import json, sys
doc = json.load(open(sys.argv[1]))
json.dump(doc["counterexample"], open(sys.argv[2], "w"))
PY
echo "  ce-trace: the mc-report's counterexample object"

# validate-json: rfs validate --json on an INVALID spec (file mode: one object).
set +e; "$RFS" validate --json invalid_always_zero_bound.rfs > "$WORK/docs/validate-json.json"; rc=$?; set -e
[ "$rc" -eq 1 ] || { echo "ERROR: validate exited $rc (expected 1=invalid)" >&2; exit "$rc"; }
echo "  validate-json: from rfs validate --json (invalid spec, rejected as intended)"

# info-json: rfs info --json introspection of the valid spec.
"$RFS" info --json pass.rfs > "$WORK/docs/info-json.json"
echo "  info-json: from rfs info --json (pass.rfs)"
echo

# --- step 3: validate every document against its schema ----------------------
echo "=== validating documents against schemas (python3 -m jsonschema) ==="
fail=0
for fmt in spec-json ce-trace mc-report validate-json info-json; do
  if python3 -m jsonschema -i "$WORK/docs/$fmt.json" "$WORK/schemas/$fmt.json" >/dev/null 2>&1; then
    echo "  $fmt: VALID"
  else
    echo "  $fmt: INVALID" >&2
    python3 -m jsonschema -i "$WORK/docs/$fmt.json" "$WORK/schemas/$fmt.json" 2>&1 | head -5 >&2 || true
    fail=1
  fi
done

if [ "$fail" -ne 0 ]; then
  echo "ERROR: one or more documents failed schema validation" >&2
  exit 1
fi
echo
echo "all five formats validated against their schemas"
