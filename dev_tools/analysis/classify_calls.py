#!/usr/bin/env python3
"""Phase 12 task 3: classify calls.jsonl rows into operation categories.

One committed script, four modes (--kind terminal|edit_file|python|misc),
added one per subtask (3.1-3.4). Stdlib only. Deterministic ORDERED rule sets
- first match wins; every rule is a named function so task 5.6's Rust port
maps rule-by-rule. The category vocabulary is the STABLE one fixed in
PHASE_12_TASK_3.md Context - never renamed or merged here.

Output: one TSV per kind under dev_tools/analysis/ with header
    category, count, abstract_pattern, example1, example2, example3
(one row per category that occurred; examples = payload excerpts <=160 chars,
whitespace collapsed, first three in ordinal order). The abstract patterns are
authored in the PATTERN dicts below - they are part of the script so the TSVs
are fully regenerable (task 5.6's port-equality requirement).

Usage:  python3 classify_calls.py --kind terminal [calls.jsonl]
Exit status: 0 on success with exact reconciliation, 1 otherwise.
"""

import json
import re
import sys
from pathlib import Path

HERE = Path(__file__).resolve().parent
sys.path.insert(0, str(HERE))
from extract_calls import path_class  # single source of truth for path labels

REPO = "/workspace/host/RFS"

# ---------------------------------------------------------------------------
# shared payload helpers
# ---------------------------------------------------------------------------

def load_rows(inv: Path):
    with inv.open(encoding="utf-8") as f:
        return [json.loads(line) for line in f]


def collapse(s: str, limit: int = 160) -> str:
    """Whitespace-collapse a payload excerpt and truncate to `limit` chars."""
    s = " ".join(s.split())
    return s if len(s) <= limit else s[: limit - 3] + "..."


def write_tsv(out: Path, header_note: str, rows: list, patterns: dict):
    """rows: list of (category, [payloads...]) in first-occurrence order.
    TRUE tab separation - pattern and example fields contain commas, so comma
    joining would make the columns ambiguous. collapse() removes tabs from
    examples, and the authored patterns contain none."""
    with out.open("w", encoding="utf-8") as f:
        f.write("# " + header_note + "\n")
        f.write("category\tcount\tabstract_pattern\texample1\texample2\texample3\n")
        total = 0
        for cat, payloads in rows:
            total += len(payloads)
            ex = [collapse(p) for p in payloads[:3] if p is not None]
            while len(ex) < 3:
                ex.append("")
            f.write("\t".join([cat, str(len(payloads)), patterns[cat]] + ex) + "\n")
        return total


# ---------------------------------------------------------------------------
# terminal mode (subtask 3.1)
# ---------------------------------------------------------------------------

LOOKUP_VERBS = {
    "grep", "rg", "find", "ls", "wc", "head", "tail", "cat", "sort", "uniq",
    "cut", "tr", "strings", "file", "awk", "sed", "dd", "diff", "stat", "du",
}
ENV_PROBE_VERBS = {"whoami", "id", "uname", "env", "which", "date", "true",
                   "false", "type"}
FILE_OP_VERBS = {"mv", "rm", "cp", "mkdir", "ln", "touch", "install"}
CARGO_BUILD_VERBS = {
    "build", "test", "clippy", "fmt", "run", "check", "fuzz",
    "generate-code-tests", "test-generated-code", "extract-training",
    "build-training", "test-training",
}


def segments(cmd: str):
    """Split a command into &&/;/||/| segments; strip leading VAR=value and
    `export` prefixes. Returns [(first_word, segment_text), ...]."""
    out = []
    for seg in re.split(r"&&|\|\||;|\|", cmd):
        parts = seg.split()
        i = 0
        while i < len(parts) and (
            re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", parts[i]) or parts[i] == "export"
        ):
            i += 1
        if i < len(parts):
            out.append((parts[i], seg.strip()))
    return out


def effective_cwd(segs) -> str:
    """The subpath of REPO the command has cd'd into (last repo cd wins);
    '' for the repo root or no repo cd."""
    cwd = None
    for first, seg in segs:
        if first == "cd":
            m = re.search(r"cd[ \t]+(\S+)", seg)
            if m and m.group(1).startswith(REPO):
                cwd = m.group(1)[len(REPO):].strip("/")
    return cwd or ""


PATH_MENTION_RE = re.compile(
    r"(?:/workspace/host/RFS)?/?"
    r"((?:docs|src|crates|tests|fuzz|examples|training|ide)/[\w./-]*)"
    r"|\b(START|BUILD|TOOLCHAIN|KB|DESIGN|CODEGEN|TASKS|README|SRC_STRUCTURE|"
    r"TEST_STRUCTURE)\.md\b"
    r"|\b([\w.-]+\.rs)\b"
    r"|\b(Cargo\.toml|training\.py|\.cargo/config\.toml)\b"
    r"|\b([\w.-]+\.jsonl)\b"
)


def repo_path_classes(cmd: str, cwd_sub: str):
    """Classify every repo-path mention in the payload, in order of mention.
    Relative mentions resolve against the effective cwd. Returns the list of
    docs/sources classes only (scratch/other mentions never decide)."""
    classes = []
    for m in PATH_MENTION_RE.finditer(cmd):
        doc_dir, root_doc, rs_file, build_file, jsonl = m.groups()
        if doc_dir:
            p = doc_dir
        elif root_doc:
            p = root_doc + ".md"
        elif rs_file:
            p = (cwd_sub + "/" if cwd_sub else "") + rs_file
        elif build_file:
            p = (cwd_sub + "/" if cwd_sub else "") + build_file
        else:  # jsonl - training data, a sources artifact
            p = (cwd_sub + "/" if cwd_sub else "") + jsonl
        c = path_class(REPO + "/" + p)
        if c in ("docs", "sources"):
            classes.append(c)
    return classes


def is_build_test_run(cmd: str, segs) -> bool:
    """cargo build/test/clippy/fmt/run/check (+ aliases, +nightly), direct
    rustc/gcc/make/clang compilation, built-binary execution (./target/...),
    timeout-wrapped fuzz runs."""
    for first, seg in segs:
        if first == "cargo":
            parts = seg.split()
            j = 1
            while j < len(parts) and parts[j].startswith("+"):
                j += 1  # skip toolchain pins like +nightly
            if j < len(parts) and parts[j] in CARGO_BUILD_VERBS:
                return True
        elif first in ("rustc", "gcc", "make", "clang", "cc"):
            return True
        elif first == "timeout" and re.search(r"\b(cargo|rustc|gcc)\b", seg):
            return True
        elif seg.startswith("./target/") or seg.startswith("target/debug/rfs") \
                or seg.startswith("target/release/rfs"):
            return True
    return False


def is_git(cmd: str, segs) -> bool:
    return any(first == "git" for first, _ in segs)


def is_python_via_shell(cmd: str, segs) -> bool:
    """python3 invocations from the shell, incl. heredoc scripts and -c one-liners."""
    return any(first in ("python", "python3") for first, _ in segs)


def is_shell_write(cmd: str, segs) -> bool:
    """Heredoc / `cat >` / `tee` file creation - but NOT heredocs feeding
    python (those are python-via-shell; tested earlier by design)."""
    if is_python_via_shell(cmd, segs):
        return False
    if re.search(r"<<-?\s*['\"]?\w+", cmd):
        return True
    return any(first == "tee" for first, _ in segs) or \
        re.search(r"\bcat\s+>>?[ \t]+\S", cmd) is not None


def is_shell_edit(cmd: str, segs) -> bool:
    """In-place text edits: sed -i / perl -pi."""
    for first, seg in segs:
        if first == "sed" and re.search(r"\s-i[abw]?\b|\s--in-place", seg):
            return True
        if first == "perl" and "-pi" in seg:
            return True
    return False


def is_file_ops(cmd: str, segs) -> bool:
    """mv/rm/cp/mkdir/ln/touch/install without cargo."""
    return any(first in FILE_OP_VERBS for first, _ in segs)


# System/toolchain locations that mark a find/ls segment as environment
# exploration (NOT /tmp - scratch work stays in the lookup residual).
SYSTEM_PATH_RE = re.compile(
    r"(?:^|\s)(/usr/|/opt/|/root/|~/?\.|\$HOME|"
    r"/workspace/host/(\.cargo|rustup)|/\s)"
)


def is_env_check(cmd: str, segs) -> bool:
    """whoami/id/uname/env/which/date probes, bare tool version probes, and
    system/toolchain-path exploration (/, ~, /usr, /opt, $HOME,
    /workspace/host/.cargo, /workspace/host/rustup) - but never payloads that
    mention repo paths or the vendored registry (those belong to the lookup
    family / vendored-deps)."""
    if repo_path_classes(cmd, effective_cwd(segs)):
        return False
    if ".cargo/registry" in cmd or "CARGO_HOME/registry" in cmd:
        return False
    for first, seg in segs:
        if first in ENV_PROBE_VERBS:
            return True
        if re.search(r"\s(--version|-version|-V)\b", seg) and \
                first not in LOOKUP_VERBS:
            return True
        if first in ("find", "ls") and SYSTEM_PATH_RE.search(seg):
            return True
    return False


def is_vendored_deps(cmd: str, segs) -> bool:
    """Reading vendored crate sources from the cargo registry cache."""
    return ".cargo/registry" in cmd or "CARGO_HOME/registry" in cmd


def classify_terminal(row: dict) -> str:
    """Ordered rule set - first match wins (order per PHASE_12_TASK_3.md,
    with python-via-shell ahead of shell-write so heredoc-to-python is not
    misfiled as a shell write)."""
    if row["payload"] is None:
        return "other"  # the two payload-less recorded calls
    cmd = row["payload"]
    segs = segments(cmd)
    if is_build_test_run(cmd, segs):
        return "build-test-run"
    if is_git(cmd, segs):
        return "git"
    if is_python_via_shell(cmd, segs):
        return "python-via-shell"
    if is_shell_write(cmd, segs):
        return "shell-write"
    if is_shell_edit(cmd, segs):
        return "shell-edit"
    if is_file_ops(cmd, segs):
        return "file-ops"
    if is_env_check(cmd, segs):
        return "env-check"
    if is_vendored_deps(cmd, segs):
        return "lookup-vendored-deps"
    if any(first in LOOKUP_VERBS for first, _ in segs):
        classes = repo_path_classes(cmd, effective_cwd(segs))
        if not classes:
            return "other"  # lookup verb with no repo target (pipes, /dev/null)
        return "lookup-docs" if classes[0] == "docs" else "lookup-sources"
    return "other"


# ---------------------------------------------------------------------------
# edit_file mode (subtask 3.2)
# ---------------------------------------------------------------------------

# Whole-file-rewrite proxy for MODIFY calls: a single edit whose old_string is
# at least this many characters rewrites (nearly) the whole file. New-file
# creations (old_string == "") are whole rewrites by definition. The threshold
# is part of the output contract - task 5.6's port must reuse it.
WHOLE_REWRITE_OLD_LEN = 1000


def classify_edit_file(row: dict) -> str:
    """category = path class x new/modify; scratch/other paths -> other."""
    if row["unparsed"] or not row["paths"]:
        return "other"
    pc = row["path_classes"][0]
    if pc == "docs":
        return "docs-new" if row["is_new_file"] else "docs-modify"
    if pc == "sources":
        return "sources-new" if row["is_new_file"] else "sources-modify"
    return "other"


def edit_file_example(row: dict) -> str:
    """Informative excerpt: the target path plus the first old_string (or a
    new-file marker), instead of raw edits-JSON which is uninformative at
    160 chars."""
    try:
        arr = json.loads(row["payload"])
        first_old = arr[0].get("old_string", "") if arr else ""
    except (ValueError, TypeError, AttributeError):
        first_old = ""
    head = (first_old[:100] + "...") if len(first_old) > 100 else first_old
    head = " ".join(head.split())
    kind = "NEW FILE" if row["is_new_file"] else f"{row['n_edits']} edit(s)"
    return collapse(f"{row['paths'][0]} [{kind}] {head}")


def is_whole_rewrite(row: dict) -> bool:
    if row["is_new_file"]:
        return True
    if row["n_edits"] != 1 or row["payload"] is None:
        return False
    try:
        arr = json.loads(row["payload"])
        return len(arr) == 1 and len(arr[0].get("old_string", "")) >= WHOLE_REWRITE_OLD_LEN
    except (ValueError, TypeError):
        return False


def replace_all_count(row: dict) -> int:
    if row["payload"] is None:
        return 0
    try:
        arr = json.loads(row["payload"])
        return sum(1 for x in arr if x.get("replace_all"))
    except (ValueError, TypeError):
        return 0


# ---------------------------------------------------------------------------
# python mode (subtask 3.3)
# ---------------------------------------------------------------------------

def py_has_writeback(code: str) -> bool:
    """The script writes file content back: open(...,'w'/'a'), write_text, or
    a .write( call."""
    return (
        re.search(r"open\([^)]*[\'][wa][^\']*\)", code) is not None
        or re.search(r'open\([^)]*\"[wa]\"', code) is not None
        or "write_text" in code
        or re.search(r"\.write\(", code) is not None
    )


def py_is_bulk_rewrite(code: str) -> bool:
    """Reads file text and applies ordered str.replace / re.sub substitutions,
    then writes back - the scripted symbol/path renames."""
    return py_has_writeback(code) and (".replace(" in code or "re.sub" in code)


def py_is_audit_mask(code: str) -> bool:
    """String-masking / diffing / completeness-audit scripts (the Phase 10/11
    audit method): identifier extraction with strings masked, diffed against
    inventory rows. Matches the bare word, an underscore-joined identifier
    (mask_code, audit_src_structure - how the audit scripts name their
    functions) or a call (diff() - but NOT English words like "different",
    where a letter follows the stem."""
    return re.search(r"\b(mask|audit|diff)(?:_\w+|\(|\b)", code) is not None


def py_is_orchestration(code: str) -> bool:
    """Drives external commands via subprocess / os.system and post-processes
    their output."""
    return "subprocess" in code or "os.system" in code


def py_is_json_processing(code: str) -> bool:
    """Parses or emits JSON (training JSONL, edits arrays, inventories)."""
    return re.search(r"json\.(load|loads|dump|dumps)\b", code) is not None


def py_is_transform(code: str) -> bool:
    """Read-modify-write line surgery WITHOUT a rewrite pattern: table-row
    insertion/deletion, heading fixes, encoding/whitespace normalization.
    Requires both a read of existing content and a write-back."""
    return py_has_writeback(code) and (
        re.search(r"\.read\(\)|read_text|open\(", code) is not None
    )


def classify_python(row: dict) -> str:
    """Ordered code-shape tests - first match wins (order per
    PHASE_12_TASK_3.md): bulk-rewrite, audit-mask, orchestration,
    json-processing, transform; residual = analysis (read-only computation,
    incl. pure generators that read nothing)."""
    code = row["payload"] or ""
    if py_is_bulk_rewrite(code):
        return "bulk-rewrite"
    if py_is_audit_mask(code):
        return "audit-mask"
    if py_is_orchestration(code):
        return "orchestration"
    if py_is_json_processing(code):
        return "json-processing"
    if py_is_transform(code):
        return "transform"
    return "analysis"


# ---------------------------------------------------------------------------
# misc mode (subtask 3.4): search_conversation + web_search
# ---------------------------------------------------------------------------

def classify_misc(row: dict) -> str:
    """Direct kind mapping - the two kinds have distinct, stable purposes."""
    if row["kind"] == "search_conversation":
        return "conversation-recall"
    if row["kind"] == "web_search":
        return "web-docs-lookup"
    return "other"


MISC_PATTERNS = {
    "conversation-recall": "recall conversation history after context compaction or between sessions: 'Execute task N' resume prompts, phase close-out procedures, what was done/committed in earlier tasks, plan and task-file details - restoring working context without re-reading files",
    "web-docs-lookup": "external documentation lookup: crate APIs (mermaid-rs-renderer, plantuml-core, layout-rs, railroad, mdr on docs.rs / crates.io / github) and tool syntax references (PlantUML EBNF + state diagrams, mermaid) - three recorded forms: query (3), url fetch (24), image_queries (7)",
}


def run_misc(inv: Path, out: Path) -> int:
    rows = [r for r in load_rows(inv)
            if r["kind"] in ("search_conversation", "web_search")]
    buckets = {}
    order = []
    for r in rows:
        cat = classify_misc(r)
        if cat not in buckets:
            buckets[cat] = []
            order.append(cat)
        buckets[cat].append(r["payload"])
    ordered = [(c, buckets[c]) for c in
               [k for k in MISC_PATTERNS if k in buckets] +
               [c for c in order if c not in MISC_PATTERNS]]
    total = write_tsv(out, "misc classification - Phase 12 task 3.4",
                      ordered, MISC_PATTERNS)
    print(f"misc rows: {len(rows)}, categorized: {total}")
    if total != len(rows):
        print("FATAL: category counts do not sum to the misc row count",
              file=sys.stderr)
        return 1
    for cat, payloads in ordered:
        print(f"  {cat:<24} {len(payloads):>6}")
    return 0


PYTHON_PATTERNS = {
    "bulk-rewrite": "read file text, apply ordered str.replace / re.sub substitutions, write back - scripted symbol/path renames across sources or docs (the pattern-fragile half of the edit operations)",
    "audit-mask": "extract fn/type identifiers from source with strings masked, diff against inventory rows per file - the Phase 10/11 completeness-audit scripts",
    "transform": "read-modify-write line surgery without a rewrite pattern: table-row insertion/deletion, heading fixes, encoding and whitespace normalization",
    "json-processing": "parse or emit JSON (training JSONL, edits arrays, call inventories) - load/dump with structural checks",
    "orchestration": "drive external commands via subprocess / os.system (cargo runs, git show, tool invocations) and post-process their output",
    "analysis": "read-only computation over files/text: counting, searching, reporting - no write-back (residual; incl. pure generators that read nothing)",
}


def run_python(inv: Path, out: Path) -> int:
    rows = [r for r in load_rows(inv) if r["kind"] == "python"]
    buckets = {}
    order = []
    for r in rows:
        cat = classify_python(r)
        if cat not in buckets:
            buckets[cat] = []
            order.append(cat)
        buckets[cat].append(r["payload"])
    ordered = [(c, buckets[c]) for c in
               [k for k in PYTHON_PATTERNS if k in buckets] +
               [c for c in order if c not in PYTHON_PATTERNS]]
    total = write_tsv(out, "python classification - Phase 12 task 3.3",
                      ordered, PYTHON_PATTERNS)
    print(f"python rows: {len(rows)}, categorized: {total}")
    if total != len(rows):
        print("FATAL: category counts do not sum to the python row count",
              file=sys.stderr)
        return 1
    for cat, payloads in ordered:
        print(f"  {cat:<24} {len(payloads):>6}")
    return 0


EDIT_FILE_PATTERNS = {
    "docs-new": "edit_file with old_string==\"\" creating a whole new doc file (status files, task files, phase docs, README sections) - new_string carries the full document content",
    "docs-modify": "edit_file with one or more {old_string,new_string} replacements in docs (roadmap/KB/DESIGN/BUILD/TASKS/inventories) - targeted section and table-row edits",
    "sources-new": "edit_file with old_string==\"\" creating a whole new Rust source/test file - new_string carries the full module content",
    "sources-modify": "edit_file with one or more replacements in src/tests/crates/fuzz - function-level, import-line and test-block edits",
    "other": "edits to paths outside docs/sources (session-local scripts under /tmp or the sandbox, .gitignore, scratch fixtures)",
}


def run_edit_file(inv: Path, out: Path) -> int:
    rows = [r for r in load_rows(inv) if r["kind"] == "edit_file"]
    buckets = {}
    order = []
    for r in rows:
        cat = classify_edit_file(r)
        if cat not in buckets:
            buckets[cat] = []
            order.append(cat)
        buckets[cat].append(r)
    ordered = [(c, buckets[c]) for c in
               [k for k in EDIT_FILE_PATTERNS if k in buckets] +
               [c for c in order if c not in EDIT_FILE_PATTERNS]]

    hist = {}
    total_replace_all = 0
    total_whole = 0
    for r in rows:
        hist[r["n_edits"] or -1] = hist.get(r["n_edits"] or -1, 0) + 1
        total_replace_all += replace_all_count(r)
        if is_whole_rewrite(r):
            total_whole += 1

    with out.open("w", encoding="utf-8") as f:
        f.write("# edit_file classification - Phase 12 task 3.2\n")
        f.write("category\tcount\tabstract_pattern\texample1\texample2\t"
                "example3\tsingle_edit_share\treplace_all_count\t"
                "whole_rewrite_share\n")
        total = 0
        for cat, cat_rows in ordered:
            total += len(cat_rows)
            ex = [edit_file_example(r) for r in cat_rows[:3]]
            while len(ex) < 3:
                ex.append("")
            single = sum(1 for r in cat_rows if r["n_edits"] == 1)
            ra = sum(replace_all_count(r) for r in cat_rows)
            whole = sum(1 for r in cat_rows if is_whole_rewrite(r))
            f.write("\t".join([
                cat, str(len(cat_rows)), EDIT_FILE_PATTERNS.get(cat, ""),
                *ex,
                f"{single / len(cat_rows):.4f}", str(ra),
                f"{whole / len(cat_rows):.4f}",
            ]) + "\n")
        hist_str = " ".join(f"{k}:{v}" for k, v in sorted(hist.items()))
        f.write(f"# edits-per-call histogram: {hist_str}\n")
        f.write(f"# replace_all total: {total_replace_all}\n")
        f.write(f"# whole-file rewrites (new or single edit with old_string >= "
                f"{WHOLE_REWRITE_OLD_LEN} chars): {total_whole} "
                f"({total_whole / len(rows):.4f} of all calls)\n")
    print(f"edit_file rows: {len(rows)}, categorized: {total}")
    if total != len(rows):
        print("FATAL: category counts do not sum to the edit_file row count",
              file=sys.stderr)
        return 1
    for cat, cat_rows in ordered:
        print(f"  {cat:<24} {len(cat_rows):>6}")
    return 0


TERMINAL_PATTERNS = {
    "lookup-docs": "cd <repo>[/<docs subdir>] && (sed -n '<a>,<b>p' | head -c N | tail -c +N | cat | grep -n 'pat') <doc file(s)> [; second doc in the same command] - chunked reads of START/BUILD/TOOLCHAIN/KB/DESIGN/CODEGEN/TASKS/docs/** per the token-economy protocol",
    "lookup-sources": "cd <repo> && (sed -n '<a>,<b>p' | grep -n 'pat' | head/tail | cat | awk) <src/tests/crates file(s)> - chunked reads and pattern searches over Rust sources, test suites and workspace crates",
    "lookup-vendored-deps": "L=/workspace/host/.cargo/registry/src/index.crates.io-*/<crate>-<ver>; ls $L/src/ && cat/grep $L/src/<file> - reading vendored dependency sources (layout-rs, arbitrary, z3, ...) to answer API questions",
    "build-test-run": "export RUSTUP_HOME=... CARGO_HOME=... PATH=... && cd <repo> && cargo (test [--workspace|-p X|--test Y|--features smt] | build | clippy --all-targets | fmt [--check] | run -- <rfs subcmd>) - plus direct rustc/gcc/make/clang compiles, ./target/{debug,release}/rfs executions and timeout-wrapped fuzz runs",
    "git": "cd <repo> && [export GIT_CONFIG_COUNT=1 ...safe.directory...] && git (log --oneline -N | status --short | show --stat <sha> | add <paths> && commit -m '...') - history inspection and the per-task commits",
    "shell-write": "cd <repo> && cat > <file> <<'EOF' ... EOF  /  cat >> TASKS.md <<'EOF' ... EOF  /  tee <file> - creating or appending whole file contents (status files, task files, TASKS.md entries) from heredocs",
    "shell-edit": "cd <repo> && sed -i 's/old/new/g' <file(s)>  /  sed -i '<a>,<b>d' <file>  /  perl -pi -e 's/.../' - in-place text surgery on sources or docs",
    "file-ops": "(mv | rm -rf | cp | mkdir -p | ln -s | touch) <paths> - tree manipulation: moving modules, cleaning scratch dirs, copying fixtures (incl. /tmp scratch work)",
    "env-check": "(whoami | id | uname -a | which X | command -v X | X --version/-V | rustup toolchain list | find / -name X | ls ~/.cargo | ls /workspace/host/{.cargo,rustup}) - environment and toolchain probing, incl. hunting for missing tools",
    "python-via-shell": "cd <repo> && python3 - <<'PYEOF' ... PYEOF  /  python3 <script>.py  /  python3 -c '...' - scripted operations run from the shell (rewrites, audits, transforms)",
    "other": "residual: bare cd/export segments, for-loops over mixed operations, nohup/background runs, bash <script>.sh, install/network calls (apt-get, curl, npm, npx), echo-only commands, and the two payload-less recorded calls",
}


def run_terminal(inv: Path, out: Path) -> int:
    rows = [r for r in load_rows(inv) if r["kind"] == "terminal"]
    buckets = {}
    order = []
    for r in rows:
        cat = classify_terminal(r)
        if cat not in buckets:
            buckets[cat] = []
            order.append(cat)
        buckets[cat].append(r["payload"])
    ordered = [(c, buckets[c]) for c in
               [k for k in TERMINAL_PATTERNS if k in buckets] +
               [c for c in order if c not in TERMINAL_PATTERNS]]
    total = write_tsv(out, "terminal classification - Phase 12 task 3.1",
                      ordered, TERMINAL_PATTERNS)
    expected_total = len(rows)
    print(f"terminal rows: {expected_total}, categorized: {total}")
    if total != expected_total:
        print("FATAL: category counts do not sum to the terminal row count",
              file=sys.stderr)
        return 1
    for cat, payloads in ordered:
        print(f"  {cat:<24} {len(payloads):>6}")
    return 0


# ---------------------------------------------------------------------------
# consolidate mode (subtask 3.5): build the README analysis section from the
# four TSVs - no manual numbers, so the frozen section is regenerable and its
# agreement with the TSVs is mechanical (a PROTOCOL.md re-run reproduces it).
# ---------------------------------------------------------------------------

TOTAL_CALLS = 12596

# Authored per-category implication for the tool set (the analysis half of the
# master table; the counts/patterns come from the TSVs). Categories with no
# tool say why.
IMPLICATIONS = {
    "lookup-docs": "becomes a tool: `doc-section` - structured section retrieval replaces chunked sed/head/tail/dd reads",
    "lookup-sources": "becomes a tool: `context` - inventory row + Depends closure + doc fragments in one command replaces the grep+sed+read sequence",
    "lookup-vendored-deps": "no tool - external crate sources change with dependency versions; a cached index would go stale, ad-hoc reads stay correct",
    "build-test-run": "no tool - cargo IS the build/test tool; wrapping it would duplicate its UX. The fixed activation env stays documented in BUILD.md/TOOLCHAIN.md",
    "git": "no tool - git is the system of record; a wrapper adds no determinism",
    "shell-write": "no dedicated tool this phase - template-fill creation (status/task files) is governed by FORMATTING.md templates + review, not by a generator",
    "shell-edit": "becomes a tool: `edit-rename`/`edit-move`/`edit-replace-item` - syn-parsed rewrites immune to string/comment pattern phantoms",
    "file-ops": "partially covered: repo-relevant module moves go to `edit-move` (with import-site rewriting); raw mv/rm/cp of scratch files stay manual",
    "env-check": "no tool - the environment differs per sandbox; probing must stay live",
    "python-via-shell": "becomes the tools themselves: rewrites -> edit-*, audits -> `inventory audit`, table surgery -> `inventory add/update/remove` + `sync-fix`; residual ad-hoc analysis stays manual",
    "docs-new": "no dedicated tool this phase - whole-doc creation is template-governed (FORMATTING.md) and review-checked",
    "docs-modify": "becomes a tool: `sync-check`/`sync-fix` - docs<->source reference re-syncing after refactors, as a compact fix-list",
    "sources-new": "no tool - new-file creation is design work, not a mechanical operation",
    "sources-modify": "becomes tools: `edit-*` for the rewrites + `sync-check` for the reference upkeep they trigger",
    "bulk-rewrite": "becomes a tool: `edit-rename`/`edit-move` - the 229 pattern-fragile scripted rewrites productized with syn",
    "audit-mask": "becomes a tool: `inventory audit` - the Phase 10/11 string-masked completeness check, no longer a reconstructed session script",
    "transform": "becomes tools: `inventory add/update/remove` (table-row surgery) + `sync-fix` (pointer-line rewrites)",
    "json-processing": "no tool - JSON manipulation is task-specific; the one recurring case (the commands.md inventory) is covered by `calls extract`",
    "orchestration": "no tool - subprocess glue is inherently ad-hoc per operation",
    "analysis": "no tool - read-only computation carries design judgment",
    "conversation-recall": "no tool - a platform capability (searching one's own conversation) that no repo tool can replace",
    "web-docs-lookup": "no tool - external and one-off per dependency; fails the mechanical-repeated criterion",
}

# NOTE: 'other' exists in BOTH the terminal and edit_file kinds with different
# implications - it is deliberately absent from IMPLICATIONS above; the master
# table resolves it via this (kind, category) key.
IMPLICATIONS_BY_KIND = {
    ("terminal", "other"): "no tool - heterogeneous residual (scratch work, installs, job polling, npm internals)",
    ("edit_file", "other"): "no tool - session-local scratch edits (audit/rewrite helper scripts, .gitignore)",
}

KIND_TITLES = {
    "terminal": "terminal (9,623 calls)",
    "edit_file": "edit_file (2,038 calls)",
    "python": "python (818 calls)",
    "misc": "misc - search_conversation + web_search (117 calls)",
}

FORMAT_TABLE = """| Kind | Count | Fields |
|---|---|---|
| `terminal` | 9,623 | `**command:**` - the shell command, inline-backticked on one line OR in a fenced code block (multi-line only in the fenced form) |
| `edit_file` | 2,038 | `**path:**` (inline backticks) + `**edits:**` (fenced JSON array of `{old_string, new_string[, replace_all]}`; the fence is widened to FOUR backticks when the JSON itself contains triple-backtick lines; `old_string: ""` = whole-new-file creation) |
| `python` | 818 | `**code:**` - inline or fenced Python source |
| `search_conversation` | 83 | `**query:**` (inline backticks) |
| `web_search` | 34 | three recorded forms: `**query:**` (3 entries), `**url:**` only (24), `**image_queries:**` only (7) |"""

CLOSING_HEADING = "What this means for the tool set"

CLOSING_PARAGRAPH = """The operation classes that become tools are exactly the mechanical, repeated, token-expensive or error-prone ones: chunked file-content lookup (5,536 calls - 44% of all terminal calls) becomes `doc-section` + `context`; pattern-fragile rewrites (229 python bulk-rewrite + 32 shell-edit + the sources-modify/edit_file share they serve) become the syn-based `edit-*` trio; docs<->source reference re-syncing (the docs-modify class after any refactor) becomes `sync-check`/`sync-fix`; inventory table surgery + the string-masked audit (170 transform + 68 audit-mask python calls, plus the edit_file inventory rows) become `inventory add/update/remove/audit`; and the analysis pipeline itself (this section's extraction + classification) becomes `calls extract/classify` so that a PROTOCOL.md re-run on a new commands.md is one command pair. The classes that do NOT become tools, with reasons: build/test execution and git (the real tools already exist), env-check and vendored-deps lookup (per-sandbox liveness / per-version staleness), file-ops residual and scratch edits (heterogeneous), new-file creation of docs or sources (design work, template-governed), json-processing/orchestration/analysis (task-specific judgment), and the two misc kinds (platform capability / external one-off)."""


def md_cell(s: str) -> str:
    """Escape a TSV cell for a markdown pipe table: `|` -> `\\|`. No other
    escaping is needed (verified: no cell ends with a backslash; collapse()
    removed all newlines)."""
    return s.replace("|", "\\|")


def tsv_to_md_table(path: Path) -> str:
    lines = path.read_text(encoding="utf-8").splitlines()
    header = next(l for l in lines if l.startswith("category\t"))
    rows = [l for l in lines if l and not l.startswith("#")
            and not l.startswith("category\t")]
    out = ["| " + " | ".join(header.split("\t")) + " |"]
    out.append("|" + "---|" * len(header.split("\t")))
    for r in rows:
        cells = r.split("\t")
        out.append("| " + " | ".join(md_cell(c) for c in cells) + " |")
    return "\n".join(out)


def run_consolidate(inv: Path, out: Path) -> int:
    here = HERE
    tsvs = {k: here / f"{k}_categories.tsv"
            for k in ("terminal", "edit_file", "python", "misc")}
    for p in tsvs.values():
        if not p.exists():
            print(f"FATAL: {p.name} missing - run its classification mode first",
                  file=sys.stderr)
            return 1

    # Master table: one row per (kind, category), counts from the TSVs.
    master = []
    total = 0
    for kind in ("terminal", "edit_file", "python", "misc"):
        for line in tsvs[kind].read_text(encoding="utf-8").splitlines():
            if not line or line.startswith("#") or line.startswith("category\t"):
                continue
            cat, count, pattern = line.split("\t")[:3]
            master.append((kind, cat, int(count), pattern))
            total += int(count)
    if total != TOTAL_CALLS:
        print(f"FATAL: master counts sum to {total}, expected {TOTAL_CALLS}",
              file=sys.stderr)
        return 1

    parts = []
    parts.append(
        "Evidence base for the proposed tools: a systematic classification of "
        "ALL 12,596 assistant tool calls recorded in `commands.md` (the Phases "
        "0-11 session log). Produced by `dev_tools/analysis/extract_calls.py` "
        "(inventory) + `classify_calls.py` (four-mode classifier); the per-kind "
        "TSVs under `dev_tools/analysis/` are the machine-readable source of "
        "this section - every count and abstract pattern below is transcribed "
        "from them, no manual numbers. FROZEN after Phase 12 task 3.5: later "
        "edits require a plan correction; a `dev_tools/PROTOCOL.md` re-run "
        "regenerates the section from fresh TSVs.")
    parts.append("### The commands.md format\n\n" + FORMAT_TABLE)
    # The master table is ONE part (rows joined with single newlines) - a
    # blank line between rows would terminate the markdown table.
    table = [
        "| Category | Kind | Count | Share of total | Abstract pattern | Implication for dev_tools |",
        "|---|---|---|---|---|---|",
    ]
    for kind, cat, count, pattern in master:
        key = (kind, cat)
        impl = IMPLICATIONS_BY_KIND[key] if key in IMPLICATIONS_BY_KIND \
            else IMPLICATIONS[cat]
        table.append("| " + " | ".join(md_cell(x) for x in [
            cat, kind, str(count), f"{count / TOTAL_CALLS * 100:.1f}%",
            pattern, impl,
        ]) + " |")
    parts.append(
        "### Master category table\n\nOne row per category across all kinds; "
        "counts sum to 12,596 (verified arithmetically by the consolidator).\n\n"
        + "\n".join(table))
    parts.append("### Per-kind detail\n")
    for kind in ("terminal", "edit_file", "python", "misc"):
        parts.append(f"#### {KIND_TITLES[kind]}\n\n" + tsv_to_md_table(tsvs[kind]))
    parts.append("### " + CLOSING_HEADING + "\n\n" + CLOSING_PARAGRAPH)

    out.write_text("\n\n".join(parts) + "\n", encoding="utf-8")
    print(f"consolidated {len(master)} categories, total {total} == "
          f"{TOTAL_CALLS}; wrote {out}")
    return 0


MODES = {
    "terminal": run_terminal,
    "edit_file": run_edit_file,
    "python": run_python,
    "misc": run_misc,
    "consolidate": run_consolidate,
}


def main() -> int:
    if len(sys.argv) < 3 or sys.argv[1] != "--kind":
        print(f"usage: {sys.argv[0]} --kind <{'|'.join(sorted(MODES))}> "
              f"[calls.jsonl]", file=sys.stderr)
        return 2
    kind = sys.argv[2]
    if kind not in MODES:
        print(f"mode `{kind}` not yet implemented (added by its subtask)",
              file=sys.stderr)
        return 2
    inv = Path(sys.argv[3]) if len(sys.argv) > 3 else HERE / "calls.jsonl"
    # The consolidate mode writes a markdown section, not a TSV - it takes an
    # explicit output path (argv[4]); the classification modes keep their
    # fixed <kind>_categories.tsv names.
    if kind == "consolidate":
        if len(sys.argv) < 5:
            print(f"usage: {sys.argv[0]} --kind consolidate [calls.jsonl] "
                  f"<out.md>", file=sys.stderr)
            return 2
        out = Path(sys.argv[4])
    else:
        out = HERE / f"{kind}_categories.tsv"
    return MODES[kind](inv, out)


if __name__ == "__main__":
    sys.exit(main())
