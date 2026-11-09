#!/usr/bin/env python3
"""Phase 12 task 2: extract commands.md into a structured call inventory.

Parses the repo-root `commands.md` (the chronological log of assistant tool
calls from the Phases 0-11 sessions) into one JSON row per recorded call:

    {"ordinal": <1-based entry number>, "kind": <str>,
     "payload": <str|null>, "paths": [<explicit field paths>],
     "path_classes": [<docs|sources|scratch|other> per path],
     "unparsed": <bool>}

plus, for edit_file rows only: "n_edits": <int|null> and
"is_new_file": <bool|null> (derived from the edits JSON; old_string == ""
means whole-new-file creation).

Entry format handled (both payload forms occur in the file):
  **tool call:** `<kind>`
  followed by kind-specific fields - terminal: **command:**, edit_file:
  **path:** + **edits:** (fenced JSON array), python: **code:**,
  search_conversation / web_search: **query:** (+ optional extra fields that
  are ignored for payload purposes). A payload is either inline-backticked on
  the field's own line, or in a fenced code block on the following lines.

Terminal/python target paths are deliberately NOT extracted here - task 3's
classifier derives them from payload text. `paths` carries only explicit
field paths (edit_file **path:**).

Stdlib only (re, json, sys) - no dependencies. The script is committed; its
output calls.jsonl is gitignored (regenerable in seconds). Task 5.6 ports this
file rule-by-rule to Rust (`rfs-dev calls extract`); keep the logic flat and
deterministic so the port maps function-by-function.

Usage:  python3 extract_calls.py [commands.md] [out.jsonl]
        (defaults: ../commands.md relative to this file's repo layout, i.e.
         <repo>/commands.md -> <repo>/dev_tools/analysis/calls.jsonl)
Exit status: 0 on success with exact internal reconciliation, 1 otherwise.
"""

import json
import re
import sys
from pathlib import Path

# The line-anchored entry marker. Splitting and counting both use this exact
# pattern so the internal reconciliation (rows vs markers) is by construction
# comparable to `grep -c '^\*\*tool call:\*\*' commands.md`.
MARKER_RE = re.compile(r"(?m)^\*\*tool call:\*\* ")

# kind -> primary payload field name
PAYLOAD_FIELD = {
    "terminal": "command",
    "edit_file": "edits",
    "python": "code",
    "search_conversation": "query",
    "web_search": "query",
}

# Root-level living documents that count as `docs` (task file Context).
ROOT_DOCS = {
    "START.md", "BUILD.md", "TOOLCHAIN.md", "KB.md", "DESIGN.md",
    "CODEGEN.md", "TASKS.md", "README.md", "SRC_STRUCTURE.md",
    "TEST_STRUCTURE.md",
}

# Repo root derived from this script's location (dev_tools/analysis/ -> repo),
# so path classification is portable - no hardcoded machine path.
REPO_PREFIX = str(Path(__file__).resolve().parent.parent.parent) + "/"


def path_class(p: str) -> str:
    """Classify one explicit path per the task file's fixed labels."""
    q = p[len(REPO_PREFIX):] if p.startswith(REPO_PREFIX) else p
    if q in ROOT_DOCS or q.startswith("docs/"):
        return "docs"
    if (
        q.startswith(("src/", "crates/", "tests/", "fuzz/", "examples/",
                      "training/", "ide/"))
        or q.endswith(".rs")
        or q in ("Cargo.toml", ".cargo/config.toml", "training.py")
    ):
        return "sources"
    if (
        p.startswith("/tmp/")
        or p.startswith("target/")
        or p.startswith(".unsloth_tool_output/")
        or "unsloth-tmp" in p
        or q.startswith(("target/", ".unsloth_tool_output/"))
    ):
        return "scratch"
    return "other"


def extract_fenced(entry: str, field: str):
    """Fenced form: the field marker line is followed (after blank lines) by a
    fence with an optional info string, content, and a closing fence of the
    SAME WIDTH. Fence width matters: when the payload itself contains triple-
    backtick lines (markdown fences inside edit_file JSON strings), the
    recorder widens the fence to four backticks - a width-matched close is the
    only unambiguous terminator. Returns (payload, unparsed)."""
    m = re.search(
        r"\*\*" + field + r":\*\*[ \t]*\r?\n+(`{3,})[^\n]*\n(.*?)^\1[ \t]*$(?=\n|\Z)",
        entry,
        re.S | re.M,
    )
    if not m:
        return None, True
    payload = m.group(2)
    if payload.endswith("\n"):
        payload = payload[:-1]
    return payload, False


def extract_inline(entry: str, field: str):
    """Inline form: a single backticked value on the field's own line. The
    closing backtick is the LAST backtick on that line (payloads never contain
    backticks in the inline form - multi-line content uses the fenced form).
    Returns (payload, unparsed)."""
    m = re.search(r"\*\*" + field + r":\*\*[ \t]*`", entry)
    if not m:
        return None, True
    line_end = entry.find("\n", m.end())
    line = entry[m.end(): line_end if line_end != -1 else len(entry)]
    if not line.endswith("`"):
        return None, True
    return line[:-1], False


def extract_payload(entry: str, field: str):
    """Try the fenced form first (an inline value can never start a fence),
    then the inline form."""
    payload, unparsed = extract_fenced(entry, field)
    if unparsed:
        payload, unparsed = extract_inline(entry, field)
    return payload, unparsed


def extract_edit_file(entry: str):
    """edit_file rows carry an explicit **path:** (inline) plus the fenced
    **edits:** JSON. Returns (row_fields_dict, unparsed)."""
    pm = re.search(r"\*\*path:\*\*[ \t]*`([^`]*)`", entry)
    paths = [pm.group(1)] if pm else []
    payload, unparsed = extract_payload(entry, "edits")
    n_edits, is_new = None, None
    if payload is not None:
        try:
            arr = json.loads(payload)
            assert isinstance(arr, list)
            n_edits = len(arr)
            is_new = any(x.get("old_string") == "" for x in arr)
        except (ValueError, AssertionError):
            unparsed = True
    return {
        "paths": paths,
        "path_classes": [path_class(p) for p in paths],
        "payload": payload,
        "unparsed": unparsed or not paths,
        "n_edits": n_edits,
        "is_new_file": is_new,
    }, unparsed


def parse_entry(ordinal: int, entry: str) -> dict:
    kind = entry.split("`", 2)[1] if entry.startswith("`") and "`" in entry[1:40] else None
    row = {"ordinal": ordinal, "kind": kind, "payload": None, "paths": [],
           "path_classes": [], "unparsed": True}
    if kind not in PAYLOAD_FIELD:
        return row  # unknown kind: keep the row, flag unparsed
    if kind == "edit_file":
        fields, _ = extract_edit_file(entry)
        row.update(fields)
        return row
    field = PAYLOAD_FIELD[kind]
    payload, unparsed = extract_payload(entry, field)
    if unparsed and kind == "web_search":
        # Alternate recorded forms: URL-fetch entries carry **url:** instead of
        # **query:**; image-search entries carry only **image_queries:**.
        for alt in ("url", "image_queries"):
            payload, unparsed = extract_payload(entry, alt)
            if not unparsed:
                break
    row["payload"] = payload
    row["unparsed"] = unparsed
    return row


def main() -> int:
    here = Path(__file__).resolve().parent
    src = Path(sys.argv[1]) if len(sys.argv) > 1 else here.parent.parent / "commands.md"
    out = Path(sys.argv[2]) if len(sys.argv) > 2 else here / "calls.jsonl"
    text = src.read_text(encoding="utf-8")

    marker_count = len(MARKER_RE.findall(text))
    entries = MARKER_RE.split(text)[1:]  # drop the preamble before entry 1
    if len(entries) != marker_count:
        print(f"FATAL: split produced {len(entries)} entries but the file has "
              f"{marker_count} marker lines - a payload contains a line-anchored "
              f"marker; fix the splitter before trusting any output", file=sys.stderr)
        return 1

    rows = [parse_entry(i, e) for i, e in enumerate(entries, start=1)]
    with out.open("w", encoding="utf-8") as f:
        for row in rows:
            f.write(json.dumps(row, ensure_ascii=False) + "\n")

    # Reconciliation summary: per-kind parsed/unparsed/total + grand total.
    stats = {}
    for row in rows:
        s = stats.setdefault(row["kind"], {"parsed": 0, "unparsed": 0})
        s["unparsed" if row["unparsed"] else "parsed"] += 1
    print(f"{'kind':<22} {'parsed':>7} {'unparsed':>9} {'total':>7}")
    total = 0
    for kind in sorted(stats, key=lambda k: (k is None, k or "")):
        s = stats[kind]
        t = s["parsed"] + s["unparsed"]
        total += t
        print(f"{str(kind):<22} {s['parsed']:>7} {s['unparsed']:>9} {t:>7}")
    print(f"{'TOTAL':<22} {'':>7} {'':>9} {total:>7}")
    if total != marker_count:
        print(f"FATAL: rows ({total}) != marker lines ({marker_count})",
              file=sys.stderr)
        return 1
    print(f"wrote {out}")
    return 0


if __name__ == "__main__":
    sys.exit(main())
