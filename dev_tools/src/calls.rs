//! Phase 12 task 5.6: `calls extract` / `calls classify` - the Rust port of
//! `dev_tools/analysis/extract_calls.py` + `classify_calls.py`, rule-by-rule.
//!
//! The python pair is the ORACLE: this module's output on any input must be
//! byte-identical to theirs (the port-equality test in tests/calls_test.rs
//! enforces it on a fixture AND the real 12,596-call commands.md). Every
//! function below names the python function it ports. Pattern matching uses the
//! `regex` crate (third declared dependency exception, user-authorized); the
//! single python pattern that relies on a backreference + lookahead (the
//! fenced-form close) is handled with a regex-open + same-width line scan,
//! because `regex` omits both by design.
//!
//! No serde: the JSONL rows and the edits-payload parsing are hand-rolled so
//! the byte format matches python's `json.dumps(..., ensure_ascii=False)`
//! exactly (", " / ": " separators, insertion-order keys).

use std::collections::HashMap;
use std::path::Path;
use std::sync::{Arc, Mutex, OnceLock};

use regex::Regex;

/// Python's `re` module caches compiled patterns by their source string, so
/// repeated searches with the same pattern are cheap. A faithful port must do
/// the same - recompiling per call made the 12,596-entry extract ~26x slower
/// than the oracle. The cache stores `Arc<Regex>` and hands out Arc clones:
/// a bare `Regex::clone()` would deep-copy the compiled automaton (the regex
/// crate does NOT share it via interior mutability), which is exactly the
/// per-call cost we are trying to avoid. Callers use the Arc through Deref.
fn cached_regex(pattern: &str) -> Arc<Regex> {
    static CACHE: OnceLock<Mutex<HashMap<String, Arc<Regex>>>> = OnceLock::new();
    let cache = CACHE.get_or_init(|| Mutex::new(HashMap::new()));
    let mut map = cache.lock().unwrap();
    match map.get(pattern) {
        Some(r) => r.clone(),
        None => {
            let r = Arc::new(Regex::new(pattern).expect("internal regex must compile"));
            map.insert(pattern.to_string(), r.clone());
            r
        }
    }
}

// ===========================================================================
// Shared constants (ported verbatim from the python scripts)
// ===========================================================================

/// Repo root for log-path classification (Phase 12 portability). The recorded
/// logs carry absolute paths from whatever machine produced them, so this is
/// overridable via RFS_REPO_ROOT; otherwise derived from this crate's location
/// (dev_tools lives at <repo>/dev_tools). No absolute path is baked into source.
fn repo_root() -> String {
    // Explicit override for parsing logs recorded on a different machine than
    // the one running dev_tools. Otherwise derive from this crate's location:
    // dev_tools lives at <repo>/dev_tools, so the repo root is its parent.
    if let Some(p) = std::env::var("RFS_REPO_ROOT")
        .ok()
        .filter(|s| !s.trim().is_empty())
    {
        return p;
    }
    std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .parent()
        .expect("dev_tools must live inside a repo root")
        .to_string_lossy()
        .into_owned()
}

static REPO_ROOT: std::sync::LazyLock<String> = std::sync::LazyLock::new(repo_root);

/// extract_calls.py: ROOT_DOCS - root-level living documents that count as `docs`.
const ROOT_DOCS: &[&str] = &[
    "START.md",
    "BUILD.md",
    "TOOLCHAIN.md",
    "KB.md",
    "DESIGN.md",
    "CODEGEN.md",
    "TASKS.md",
    "README.md",
    "SRC_STRUCTURE.md",
    "TEST_STRUCTURE.md",
];

/// extract_calls.py: PAYLOAD_FIELD - kind -> primary payload field name.
fn payload_field(kind: &str) -> Option<&'static str> {
    match kind {
        "terminal" => Some("command"),
        "edit_file" => Some("edits"),
        "python" => Some("code"),
        "search_conversation" => Some("query"),
        "web_search" => Some("query"),
        _ => None,
    }
}

// ===========================================================================
// Hand-rolled JSON (writer matches python json.dumps ensure_ascii=False;
// parser validates the edit_file edits payloads)
// ===========================================================================

/// Escape a string exactly like python `json.dumps(s, ensure_ascii=False)`:
/// `"` -> `\"`, `\` -> `\\`, the five short control forms, other C0 controls
/// as lowercase `\u00xx`, everything else (incl. non-ASCII) verbatim.
fn json_escape(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 2);
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{08}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{0c}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => out.push_str(&format!("\\u{:04x}", c as u32)),
            c => out.push(c),
        }
    }
    out
}

/// A parsed JSON value, just enough to inspect the edits arrays.
#[derive(Debug, Clone)]
enum Json {
    Null,
    Bool(bool),
    Num(f64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    fn get(&self, key: &str) -> Option<&Json> {
        if let Json::Obj(pairs) = self {
            pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v)
        } else {
            None
        }
    }
}

/// Recursive-descent JSON parser. Returns None on any validity error - the
/// same acceptance set as python `json.loads` for the constructs that occur in
/// edits payloads (objects, arrays, strings with escapes, bools, null, numbers).
fn parse_json(s: &str) -> Option<Json> {
    let b = s.as_bytes();
    let mut i = 0usize;
    fn skip_ws(b: &[u8], i: &mut usize) {
        while *i < b.len() && matches!(b[*i], b' ' | b'\t' | b'\n' | b'\r') {
            *i += 1;
        }
    }
    fn parse_value(b: &[u8], i: &mut usize) -> Option<Json> {
        skip_ws(b, i);
        if *i >= b.len() {
            return None;
        }
        match b[*i] {
            b'{' => parse_obj(b, i),
            b'[' => parse_arr(b, i),
            b'"' => parse_str(b, i).map(Json::Str),
            b't' => literal(b, i, "true").map(|()| Json::Bool(true)),
            b'f' => literal(b, i, "false").map(|()| Json::Bool(false)),
            b'n' => literal(b, i, "null").map(|()| Json::Null),
            c if c == b'-' || c.is_ascii_digit() => parse_num(b, i),
            _ => None,
        }
    }
    fn literal(b: &[u8], i: &mut usize, word: &str) -> Option<()> {
        if b[*i..].starts_with(word.as_bytes()) {
            *i += word.len();
            Some(())
        } else {
            None
        }
    }
    fn parse_num(b: &[u8], i: &mut usize) -> Option<Json> {
        let start = *i;
        if b[*i] == b'-' {
            *i += 1;
        }
        while *i < b.len()
            && (b[*i].is_ascii_digit() || matches!(b[*i], b'.' | b'e' | b'E' | b'+' | b'-'))
        {
            *i += 1;
        }
        std::str::from_utf8(&b[start..*i])
            .ok()?
            .parse::<f64>()
            .ok()
            .map(Json::Num)
    }
    fn parse_str(b: &[u8], i: &mut usize) -> Option<String> {
        // *i is at the opening quote
        *i += 1;
        let mut out = String::new();
        while *i < b.len() {
            match b[*i] {
                b'"' => {
                    *i += 1;
                    return Some(out);
                }
                b'\\' => {
                    *i += 1;
                    if *i >= b.len() {
                        return None;
                    }
                    match b[*i] {
                        b'"' => out.push('"'),
                        b'\\' => out.push('\\'),
                        b'/' => out.push('/'),
                        b'b' => out.push('\u{08}'),
                        b'f' => out.push('\u{0c}'),
                        b'n' => out.push('\n'),
                        b'r' => out.push('\r'),
                        b't' => out.push('\t'),
                        b'u' => {
                            if *i + 4 >= b.len() {
                                return None;
                            }
                            let hex = std::str::from_utf8(&b[*i + 1..*i + 5]).ok()?;
                            let cp = u32::from_str_radix(hex, 16).ok()?;
                            out.push(char::from_u32(cp).unwrap_or('\u{fffd}'));
                            *i += 4;
                        }
                        _ => return None,
                    }
                    *i += 1;
                }
                _ => {
                    // consume one UTF-8 char
                    let len = utf8_len(b[*i]);
                    let ch = std::str::from_utf8(&b[*i..*i + len]).ok()?;
                    out.push_str(ch);
                    *i += len;
                }
            }
        }
        None // unterminated
    }
    fn utf8_len(b: u8) -> usize {
        if b < 0x80 {
            1
        } else if b >> 5 == 0b110 {
            2
        } else if b >> 4 == 0b1110 {
            3
        } else if b >> 3 == 0b11110 {
            4
        } else {
            1
        }
    }
    fn parse_arr(b: &[u8], i: &mut usize) -> Option<Json> {
        *i += 1; // [
        let mut items = Vec::new();
        skip_ws(b, i);
        if *i < b.len() && b[*i] == b']' {
            *i += 1;
            return Some(Json::Arr(items));
        }
        loop {
            let v = parse_value(b, i)?;
            items.push(v);
            skip_ws(b, i);
            if *i >= b.len() {
                return None;
            }
            match b[*i] {
                b',' => {
                    *i += 1;
                }
                b']' => {
                    *i += 1;
                    return Some(Json::Arr(items));
                }
                _ => return None,
            }
        }
    }
    fn parse_obj(b: &[u8], i: &mut usize) -> Option<Json> {
        *i += 1; // {
        let mut pairs = Vec::new();
        skip_ws(b, i);
        if *i < b.len() && b[*i] == b'}' {
            *i += 1;
            return Some(Json::Obj(pairs));
        }
        loop {
            skip_ws(b, i);
            let k = parse_str(b, i)?;
            skip_ws(b, i);
            if *i >= b.len() || b[*i] != b':' {
                return None;
            }
            *i += 1;
            let v = parse_value(b, i)?;
            pairs.push((k, v));
            skip_ws(b, i);
            if *i >= b.len() {
                return None;
            }
            match b[*i] {
                b',' => {
                    *i += 1;
                }
                b'}' => {
                    *i += 1;
                    return Some(Json::Obj(pairs));
                }
                _ => return None,
            }
        }
    }
    parse_value(b, &mut i).and_then(|v| {
        skip_ws(b, &mut i);
        if i == b.len() {
            Some(v)
        } else {
            None // trailing garbage
        }
    })
}

// ===========================================================================
// The call-inventory row (extract_calls.py row schema)
// ===========================================================================

#[derive(Debug, Clone)]
struct Row {
    ordinal: usize,
    kind: Option<String>,
    payload: Option<String>,
    paths: Vec<String>,
    path_classes: Vec<String>,
    unparsed: bool,
    // edit_file rows only (appended after `unparsed`, matching python dict order)
    is_edit_file: bool,
    n_edits: Option<usize>,
    is_new_file: Option<bool>,
}

impl Row {
    /// Serialize with python `json.dumps(row, ensure_ascii=False)` byte format:
    /// insertion-order keys, ": " and ", " separators.
    fn to_json(&self) -> String {
        let mut s = String::new();
        s.push('{');
        s.push_str(&format!("\"ordinal\": {}", self.ordinal));
        s.push_str(", ");
        s.push_str("\"kind\": ");
        match &self.kind {
            Some(k) => s.push_str(&format!("\"{}\"", json_escape(k))),
            None => s.push_str("null"),
        };
        s.push_str(", ");
        s.push_str("\"payload\": ");
        match &self.payload {
            Some(p) => s.push_str(&format!("\"{}\"", json_escape(p))),
            None => s.push_str("null"),
        };
        s.push_str(", \"paths\": [");
        for (i, p) in self.paths.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("\"{}\"", json_escape(p)));
        }
        s.push_str("], \"path_classes\": [");
        for (i, p) in self.path_classes.iter().enumerate() {
            if i > 0 {
                s.push_str(", ");
            }
            s.push_str(&format!("\"{}\"", json_escape(p)));
        }
        s.push_str("], ");
        s.push_str(&format!("\"unparsed\": {}", self.unparsed));
        if self.is_edit_file {
            s.push_str(", \"n_edits\": ");
            match self.n_edits {
                Some(n) => s.push_str(&n.to_string()),
                None => s.push_str("null"),
            };
            s.push_str(", \"is_new_file\": ");
            match self.is_new_file {
                Some(b) => s.push_str(&b.to_string()),
                None => s.push_str("null"),
            };
        }
        s.push('}');
        s
    }
}

// ===========================================================================
// extract_calls.py port
// ===========================================================================

/// extract_calls.py: path_class - classify one explicit path per the fixed labels.
fn path_class(p: &str) -> String {
    let repo_prefix = format!("{}/", REPO_ROOT.as_str());
    let q = if let Some(stripped) = p.strip_prefix(&repo_prefix) {
        stripped
    } else {
        p
    };
    if ROOT_DOCS.contains(&q) || q.starts_with("docs/") {
        return "docs".into();
    }
    let src_prefixes = [
        "src/",
        "crates/",
        "tests/",
        "fuzz/",
        "examples/",
        "training/",
        "ide/",
    ];
    if src_prefixes.iter().any(|pre| q.starts_with(pre))
        || q.ends_with(".rs")
        || matches!(q, "Cargo.toml" | ".cargo/config.toml" | "training.py")
    {
        return "sources".into();
    }
    if p.starts_with("/tmp/")
        || p.starts_with("target/")
        || p.starts_with(".unsloth_tool_output/")
        || p.contains("unsloth-tmp")
        || q.starts_with("target/")
        || q.starts_with(".unsloth_tool_output/")
    {
        return "scratch".into();
    }
    "other".into()
}

/// extract_calls.py: extract_fenced - the fenced payload form. The python regex
/// uses a backreference (`^\1`) + lookahead (`(?=\n|\Z)`) for the same-width
/// closing fence, which `regex` omits; so we regex-find each opening fence (and
/// its width), then line-scan forward for the first line that is exactly that
/// many backticks followed only by spaces/tabs. Leftmost valid opening wins,
/// matching python re.search semantics.
fn extract_fenced(entry: &str, field: &str) -> (Option<String>, bool) {
    let open_re = cached_regex(&format!(r"\*\*{}:\*\*[ \t]*\r?\n+(`{{3,}})[^\n]*\n", field));
    for m in open_re.captures_iter(entry) {
        let fence_width = m[1].len();
        let payload_start = m.get(0).unwrap().end();
        // line-scan for the same-width close
        let mut p = payload_start;
        loop {
            let line_end = entry[p..].find('\n').map(|e| p + e).unwrap_or(entry.len());
            let line = &entry[p..line_end];
            if is_close_fence(line, fence_width) {
                let mut payload = entry[payload_start..p].to_string();
                if payload.ends_with('\n') {
                    payload.pop();
                }
                return (Some(payload), false);
            }
            if line_end >= entry.len() {
                break; // reached EOF without a close from this opening
            }
            p = line_end + 1;
        }
    }
    (None, true)
}

/// A closing fence line: exactly `width` backticks followed only by spaces/tabs.
fn is_close_fence(line: &str, width: usize) -> bool {
    let mut n = 0;
    for c in line.chars() {
        if c == '`' {
            n += 1;
        } else if c == ' ' || c == '\t' {
            if n == 0 {
                return false; // whitespace before any backtick
            }
        } else {
            return false;
        }
    }
    n == width
}

/// extract_calls.py: extract_inline - the inline-backticked payload form.
fn extract_inline(entry: &str, field: &str) -> (Option<String>, bool) {
    let re = cached_regex(&format!(r"\*\*{}:\*\*[ \t]*`", field));
    let Some(m) = re.find(entry) else {
        return (None, true);
    };
    let line_end = entry[m.end()..]
        .find('\n')
        .map(|e| m.end() + e)
        .unwrap_or(entry.len());
    let line = &entry[m.end()..line_end];
    if !line.ends_with('`') {
        return (None, true);
    }
    (Some(line[..line.len() - 1].to_string()), false)
}

/// extract_calls.py: extract_payload - fenced first, then inline.
fn extract_payload(entry: &str, field: &str) -> (Option<String>, bool) {
    let (payload, unparsed) = extract_fenced(entry, field);
    if unparsed {
        return extract_inline(entry, field);
    }
    (payload, unparsed)
}

/// extract_calls.py: extract_edit_file - explicit **path:** + fenced edits JSON.
fn extract_edit_file(entry: &str) -> (RowFields, bool) {
    let path_re = Regex::new(r"\*\*path:\*\*[ \t]*`([^`]*)`").unwrap();
    let paths: Vec<String> = path_re
        .captures(entry)
        .map(|c| vec![c[1].to_string()])
        .unwrap_or_default();
    let (payload, mut unparsed) = extract_payload(entry, "edits");
    let (mut n_edits, mut is_new) = (None, None);
    if let Some(pl) = &payload {
        match parse_json(pl) {
            Some(Json::Arr(arr)) => {
                n_edits = Some(arr.len());
                is_new = Some(
                    arr.iter()
                        .any(|x| matches!(x.get("old_string"), Some(Json::Str(s)) if s.is_empty())),
                );
            }
            _ => unparsed = true, // not a valid JSON array
        }
    }
    (
        RowFields {
            paths: paths.clone(),
            path_classes: paths.iter().map(|p| path_class(p)).collect(),
            payload,
            unparsed: unparsed || paths.is_empty(),
            n_edits,
            is_new_file: is_new,
        },
        unparsed,
    )
}

struct RowFields {
    paths: Vec<String>,
    path_classes: Vec<String>,
    payload: Option<String>,
    unparsed: bool,
    n_edits: Option<usize>,
    is_new_file: Option<bool>,
}

/// extract_calls.py: parse_entry - one recorded call into a Row.
fn parse_entry(ordinal: usize, entry: &str) -> Row {
    let kind = if entry.starts_with('`') && entry[1..entry.len().min(40)].contains('`') {
        let rest = &entry[1..];
        rest.find('`').map(|end| rest[..end].to_string())
    } else {
        None
    };
    let mut row = Row {
        ordinal,
        kind: kind.clone(),
        payload: None,
        paths: Vec::new(),
        path_classes: Vec::new(),
        unparsed: true,
        is_edit_file: false,
        n_edits: None,
        is_new_file: None,
    };
    let Some(kind) = &kind else {
        return row; // unknown/absent kind: keep the row, flag unparsed
    };
    if !payload_field(kind).is_some() {
        return row;
    }
    if kind == "edit_file" {
        let (f, _) = extract_edit_file(entry);
        row.payload = f.payload;
        row.paths = f.paths;
        row.path_classes = f.path_classes;
        row.unparsed = f.unparsed;
        row.is_edit_file = true;
        row.n_edits = f.n_edits;
        row.is_new_file = f.is_new_file;
        return row;
    }
    let field = payload_field(kind).unwrap();
    let (mut payload, mut unparsed) = extract_payload(entry, field);
    if unparsed && kind == "web_search" {
        for alt in ["url", "image_queries"] {
            (payload, unparsed) = extract_payload(entry, alt);
            if !unparsed {
                break;
            }
        }
    }
    row.payload = payload;
    row.unparsed = unparsed;
    row
}

/// extract_calls.py: main - split on the line-anchored marker, parse every
/// entry, write the JSONL, print the reconciliation summary. Returns the stdout
/// text (the caller also gets the written file on disk).
pub fn run_extract(src: &Path, out: &Path) -> Result<String, String> {
    let text = std::fs::read_to_string(src).map_err(|e| format!("cannot read {:?}: {e}", src))?;

    // MARKER_RE = (?m)^\*\*tool call:\*\*  - line-anchored occurrences.
    let marker = "**tool call:** ";
    let mut starts: Vec<usize> = Vec::new();
    if text.starts_with(marker) {
        starts.push(0);
    }
    let mut search_from = 0;
    while let Some(rel) = text[search_from..].find('\n') {
        let after_nl = search_from + rel + 1;
        if text[after_nl..].starts_with(marker) {
            starts.push(after_nl);
        }
        search_from = after_nl;
    }
    let marker_count = starts.len();

    // entries = MARKER_RE.split(text)[1:] - the text between consecutive markers.
    let mut rows: Vec<Row> = Vec::with_capacity(marker_count);
    for (idx, &st) in starts.iter().enumerate() {
        let entry_start = st + marker.len();
        let entry_end = starts.get(idx + 1).copied().unwrap_or(text.len());
        rows.push(parse_entry(idx + 1, &text[entry_start..entry_end]));
    }
    if rows.len() != marker_count {
        return Err(format!(
            "FATAL: split produced {} entries but the file has {} marker lines",
            rows.len(),
            marker_count
        ));
    }

    let mut jsonl = String::new();
    for row in &rows {
        jsonl.push_str(&row.to_json());
        jsonl.push('\n');
    }
    std::fs::write(out, jsonl).map_err(|e| format!("cannot write {:?}: {e}", out))?;

    // Reconciliation summary (python main): per-kind parsed/unparsed/total + TOTAL.
    let mut stats: HashMap<String, (usize, usize)> = HashMap::new();
    for row in &rows {
        let key = row.kind.clone().unwrap_or_default();
        let e = stats.entry(key).or_insert((0, 0));
        if row.unparsed {
            e.1 += 1;
        } else {
            e.0 += 1;
        }
    }
    let mut kinds: Vec<String> = stats.keys().cloned().collect();
    // python sort key (k is None, k or "") - no None kinds occur; alphabetical.
    kinds.sort();
    let mut out_text = String::new();
    out_text.push_str(&format!(
        "{:<22} {:>7} {:>9} {:>7}\n",
        "kind", "parsed", "unparsed", "total"
    ));
    let mut total = 0usize;
    for kind in &kinds {
        let (parsed, unparsed) = stats[kind];
        let t = parsed + unparsed;
        total += t;
        out_text.push_str(&format!(
            "{:<22} {:>7} {:>9} {:>7}\n",
            kind, parsed, unparsed, t
        ));
    }
    out_text.push_str(&format!(
        "{:<22} {:>7} {:>9} {:>7}\n",
        "TOTAL", "", "", total
    ));
    if total != marker_count {
        return Err(format!(
            "FATAL: rows ({total}) != marker lines ({marker_count})"
        ));
    }
    out_text.push_str(&format!("wrote {}\n", out.display()));
    Ok(out_text)
}

// ===========================================================================
// classify_calls.py port - shared helpers
// ===========================================================================

/// classify_calls.py: collapse - whitespace-collapse + truncate to `limit` chars.
fn collapse(s: &str, limit: usize) -> String {
    let collapsed: String = s.split_whitespace().collect::<Vec<_>>().join(" ");
    if collapsed.chars().count() <= limit {
        collapsed
    } else {
        collapsed.chars().take(limit - 3).collect::<String>() + "..."
    }
}

/// Load a calls.jsonl inventory into Rows (re-parsing the python-written JSON).
fn load_rows(inv: &Path) -> Result<Vec<Row>, String> {
    let text = std::fs::read_to_string(inv).map_err(|e| format!("cannot read {:?}: {e}", inv))?;
    let mut rows = Vec::new();
    for line in text.lines() {
        if line.is_empty() {
            continue;
        }
        rows.push(row_from_json(line)?);
    }
    Ok(rows)
}

/// Inverse of Row::to_json - parse one python-written JSONL row back into a Row.
fn row_from_json(line: &str) -> Result<Row, String> {
    let v = parse_json(line).ok_or_else(|| format!("bad JSONL row: {line}"))?;
    let ordinal = match v.get("ordinal") {
        Some(Json::Num(n)) => *n as usize,
        _ => return Err(format!("row missing ordinal: {line}")),
    };
    let kind = match v.get("kind") {
        Some(Json::Str(s)) => Some(s.clone()),
        Some(Json::Null) => None,
        _ => return Err(format!("row bad kind: {line}")),
    };
    let payload = match v.get("payload") {
        Some(Json::Str(s)) => Some(s.clone()),
        Some(Json::Null) => None,
        _ => return Err(format!("row bad payload: {line}")),
    };
    let str_arr = |key: &str| -> Result<Vec<String>, String> {
        match v.get(key) {
            Some(Json::Arr(items)) => items
                .iter()
                .map(|x| match x {
                    Json::Str(s) => Ok(s.clone()),
                    _ => Err(format!("row bad {key} element: {line}")),
                })
                .collect(),
            _ => Ok(Vec::new()),
        }
    };
    let paths = str_arr("paths")?;
    let path_classes = str_arr("path_classes")?;
    let unparsed = match v.get("unparsed") {
        Some(Json::Bool(b)) => *b,
        _ => return Err(format!("row bad unparsed: {line}")),
    };
    let is_edit_file = v.get("n_edits").is_some();
    let n_edits = match v.get("n_edits") {
        Some(Json::Num(n)) => Some(*n as usize),
        Some(Json::Null) | None => None,
        _ => return Err(format!("row bad n_edits: {line}")),
    };
    let is_new_file = match v.get("is_new_file") {
        Some(Json::Bool(b)) => Some(*b),
        Some(Json::Null) | None => None,
        _ => return Err(format!("row bad is_new_file: {line}")),
    };
    Ok(Row {
        ordinal,
        kind,
        payload,
        paths,
        path_classes,
        unparsed,
        is_edit_file,
        n_edits,
        is_new_file,
    })
}

// ===========================================================================
// terminal mode (subtask 3.1)
// ===========================================================================

const LOOKUP_VERBS: &[&str] = &[
    "grep", "rg", "find", "ls", "wc", "head", "tail", "cat", "sort", "uniq", "cut", "tr",
    "strings", "file", "awk", "sed", "dd", "diff", "stat", "du",
];
const ENV_PROBE_VERBS: &[&str] = &[
    "whoami", "id", "uname", "env", "which", "date", "true", "false", "type",
];
const FILE_OP_VERBS: &[&str] = &["mv", "rm", "cp", "mkdir", "ln", "touch", "install"];
const CARGO_BUILD_VERBS: &[&str] = &[
    "build",
    "test",
    "clippy",
    "fmt",
    "run",
    "check",
    "fuzz",
    "generate-code-tests",
    "test-generated-code",
    "extract-training",
    "build-training",
    "test-training",
];

/// classify_calls.py: segments - split on &&/||/;/|, strip leading VAR=value and
/// `export` prefixes. Returns (first_word, segment_text) pairs.
fn segments(cmd: &str) -> Vec<(String, String)> {
    let re = cached_regex(r"&&|\|\||;|\|");
    let mut out = Vec::new();
    for seg in re.split(cmd) {
        let parts: Vec<&str> = seg.split_whitespace().collect();
        let mut i = 0;
        while i < parts.len() && (var_value_token(parts[i]) || parts[i] == "export") {
            // re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", token) or token == "export"
            i += 1;
        }
        if i < parts.len() {
            out.push((parts[i].to_string(), seg.trim().to_string()));
        }
    }
    out
}

/// re.match(r"^[A-Za-z_][A-Za-z0-9_]*=", token) - a VAR=value assignment token.
fn var_value_token(tok: &str) -> bool {
    let b = tok.as_bytes();
    if b.is_empty() || !(b[0].is_ascii_alphabetic() || b[0] == b'_') {
        return false;
    }
    for (idx, &c) in b.iter().enumerate() {
        if c == b'=' {
            return idx >= 1; // at least one name char before '='
        }
        if !(c.is_ascii_alphanumeric() || c == b'_') {
            return false;
        }
    }
    false // no '=' found
}

/// classify_calls.py: effective_cwd - the subpath of REPO the command has cd'd into.
fn effective_cwd(segs: &[(String, String)]) -> String {
    let re = cached_regex(r"cd[ \t]+(\S+)");
    let mut cwd: Option<String> = None;
    for (first, seg) in segs {
        if first == "cd" {
            if let Some(m) = re.captures(seg) {
                let target = &m[1];
                if let Some(sub) = target.strip_prefix(REPO_ROOT.as_str()) {
                    cwd = Some(sub.trim_matches('/').to_string());
                }
            }
        }
    }
    cwd.unwrap_or_default()
}

/// classify_calls.py: PATH_MENTION_RE - the five-alternative repo-path matcher.
/// The optional absolute prefix is derived from REPO_ROOT (not hardcoded), so
/// the classifier works on whatever machine dev_tools runs on.
fn path_mention_re() -> Arc<Regex> {
    let repo = regex::escape(REPO_ROOT.as_str());
    let first =
        format!(r"(?:{repo})?/?((?:docs|src|crates|tests|fuzz|examples|training|ide)/[\w./-]*)");
    let pattern = [
        first.as_str(),
        r"|\b(START|BUILD|TOOLCHAIN|KB|DESIGN|CODEGEN|TASKS|README|SRC_STRUCTURE|TEST_STRUCTURE)\.md\b",
        r"|\b([\w.-]+\.rs)\b",
        r"|\b(Cargo\.toml|training\.py|\.cargo/config\.toml)\b",
        r"|\b([\w.-]+\.jsonl)\b",
    ]
    .concat();
    cached_regex(&pattern)
}

/// classify_calls.py: repo_path_classes - classify every repo-path mention, in
/// order of mention; relative mentions resolve against the effective cwd.
fn repo_path_classes(cmd: &str, cwd_sub: &str) -> Vec<String> {
    let re = path_mention_re();
    let mut classes = Vec::new();
    for m in re.captures_iter(cmd) {
        let p = if let Some(doc_dir) = m.get(1).map(|x| x.as_str()) {
            doc_dir.to_string()
        } else if let Some(root_doc) = m.get(2).map(|x| x.as_str()) {
            format!("{root_doc}.md")
        } else if let Some(rs_file) = m.get(3).map(|x| x.as_str()) {
            if cwd_sub.is_empty() {
                rs_file.to_string()
            } else {
                format!("{cwd_sub}/{rs_file}")
            }
        } else if let Some(build_file) = m.get(4).map(|x| x.as_str()) {
            if cwd_sub.is_empty() {
                build_file.to_string()
            } else {
                format!("{cwd_sub}/{build_file}")
            }
        } else if let Some(jsonl) = m.get(5).map(|x| x.as_str()) {
            if cwd_sub.is_empty() {
                jsonl.to_string()
            } else {
                format!("{cwd_sub}/{jsonl}")
            }
        } else {
            continue;
        };
        let c = path_class(&format!("{}/{}", REPO_ROOT.as_str(), p));
        if c == "docs" || c == "sources" {
            classes.push(c);
        }
    }
    classes
}

/// classify_calls.py: is_build_test_run. (The python takes `cmd` but only
/// iterates `segs`; kept for rule-by-rule fidelity, hence the underscore.)
fn is_build_test_run(_cmd: &str, segs: &[(String, String)]) -> bool {
    let timeout_re = cached_regex(r"\b(cargo|rustc|gcc)\b");
    for (first, seg) in segs {
        if first == "cargo" {
            let parts: Vec<&str> = seg.split_whitespace().collect();
            let mut j = 1;
            while j < parts.len() && parts[j].starts_with('+') {
                j += 1; // skip toolchain pins like +nightly
            }
            if j < parts.len() && CARGO_BUILD_VERBS.contains(&parts[j]) {
                return true;
            }
        } else if matches!(first.as_str(), "rustc" | "gcc" | "make" | "clang" | "cc")
            || (first == "timeout" && timeout_re.is_match(seg))
            || seg.starts_with("./target/")
            || seg.starts_with("target/debug/rfs")
            || seg.starts_with("target/release/rfs")
        {
            return true;
        }
    }
    false
}

fn is_git(segs: &[(String, String)]) -> bool {
    segs.iter().any(|(first, _)| first == "git")
}

fn is_python_via_shell(segs: &[(String, String)]) -> bool {
    segs.iter()
        .any(|(first, _)| first == "python" || first == "python3")
}

/// classify_calls.py: is_shell_write - heredoc / `cat >` / `tee`, but NOT
/// heredocs feeding python (those are python-via-shell; tested earlier).
fn is_shell_write(cmd: &str, segs: &[(String, String)]) -> bool {
    if is_python_via_shell(segs) {
        return false;
    }
    let heredoc = cached_regex(r#"<<-?\s*['"]?\w+"#);
    if heredoc.is_match(cmd) {
        return true;
    }
    let cat_re = cached_regex(r"\bcat\s+>>?[ \t]+\S");
    segs.iter().any(|(first, _)| first == "tee") || cat_re.is_match(cmd)
}

/// classify_calls.py: is_shell_edit - sed -i / perl -pi.
fn is_shell_edit(segs: &[(String, String)]) -> bool {
    let sed_re = cached_regex(r"\s-i[abw]?\b|\s--in-place");
    for (first, seg) in segs {
        if first == "sed" && sed_re.is_match(seg) {
            return true;
        }
        if first == "perl" && seg.contains("-pi") {
            return true;
        }
    }
    false
}

fn is_file_ops(segs: &[(String, String)]) -> bool {
    segs.iter()
        .any(|(first, _)| FILE_OP_VERBS.contains(&first.as_str()))
}

/// classify_calls.py: SYSTEM_PATH_RE. The toolchain-probing prefix is derived
/// from REPO_ROOT's parent (the dir holding .cargo / rustup), not hardcoded.
fn system_path_re() -> Arc<Regex> {
    let host = std::path::Path::new(REPO_ROOT.as_str())
        .parent()
        .map(|p| regex::escape(p.to_string_lossy().as_ref()))
        .unwrap_or_default();
    cached_regex(&format!(
        r"(?:^|\s)(/usr/|/opt/|/root/|~/?\.|\$HOME|{host}/(\.cargo|rustup)|/\s)"
    ))
}

/// classify_calls.py: is_env_check.
fn is_env_check(cmd: &str, segs: &[(String, String)]) -> bool {
    if !repo_path_classes(cmd, &effective_cwd(segs)).is_empty() {
        return false;
    }
    if cmd.contains(".cargo/registry") || cmd.contains("CARGO_HOME/registry") {
        return false;
    }
    let ver_re = cached_regex(r"\s(--version|-version|-V)\b");
    let sys_re = system_path_re();
    for (first, seg) in segs {
        if ENV_PROBE_VERBS.contains(&first.as_str()) {
            return true;
        }
        if ver_re.is_match(seg) && !LOOKUP_VERBS.contains(&first.as_str()) {
            return true;
        }
        if (first == "find" || first == "ls") && sys_re.is_match(seg) {
            return true;
        }
    }
    false
}

fn is_vendored_deps(cmd: &str) -> bool {
    cmd.contains(".cargo/registry") || cmd.contains("CARGO_HOME/registry")
}

/// classify_calls.py: classify_terminal - ordered rule set, first match wins.
fn classify_terminal(row: &Row) -> String {
    let Some(cmd) = &row.payload else {
        return "other".into(); // the two payload-less recorded calls
    };
    let segs = segments(cmd);
    if is_build_test_run(cmd, &segs) {
        return "build-test-run".into();
    }
    if is_git(&segs) {
        return "git".into();
    }
    if is_python_via_shell(&segs) {
        return "python-via-shell".into();
    }
    if is_shell_write(cmd, &segs) {
        return "shell-write".into();
    }
    if is_shell_edit(&segs) {
        return "shell-edit".into();
    }
    if is_file_ops(&segs) {
        return "file-ops".into();
    }
    if is_env_check(cmd, &segs) {
        return "env-check".into();
    }
    if is_vendored_deps(cmd) {
        return "lookup-vendored-deps".into();
    }
    if segs
        .iter()
        .any(|(first, _)| LOOKUP_VERBS.contains(&first.as_str()))
    {
        let classes = repo_path_classes(cmd, &effective_cwd(&segs));
        if classes.is_empty() {
            return "other".into(); // lookup verb with no repo target
        }
        return if classes[0] == "docs" {
            "lookup-docs"
        } else {
            "lookup-sources"
        }
        .into();
    }
    "other".into()
}

// ===========================================================================
// edit_file mode (subtask 3.2)
// ===========================================================================

/// classify_calls.py: WHOLE_REWRITE_OLD_LEN - part of the output contract.
const WHOLE_REWRITE_OLD_LEN: usize = 1000;

fn classify_edit_file(row: &Row) -> String {
    if row.unparsed || row.paths.is_empty() {
        return "other".into();
    }
    let pc = &row.path_classes[0];
    match pc.as_str() {
        "docs" => if row.is_new_file == Some(true) {
            "docs-new"
        } else {
            "docs-modify"
        }
        .into(),
        "sources" => if row.is_new_file == Some(true) {
            "sources-new"
        } else {
            "sources-modify"
        }
        .into(),
        _ => "other".into(),
    }
}

/// classify_calls.py: edit_file_example - path + [NEW FILE|N edit(s)] + first old_string head.
fn edit_file_example(row: &Row) -> String {
    let mut first_old = String::new();
    if let Some(pl) = &row.payload {
        if let Some(Json::Arr(arr)) = parse_json(pl) {
            if let Some(x) = arr.first() {
                if let Some(Json::Str(s)) = x.get("old_string") {
                    first_old = s.clone();
                }
            }
        }
    }
    let head = if first_old.chars().count() > 100 {
        first_old.chars().take(100).collect::<String>() + "..."
    } else {
        first_old.clone()
    };
    let head = head.split_whitespace().collect::<Vec<_>>().join(" ");
    let kind = if row.is_new_file == Some(true) {
        "NEW FILE".to_string()
    } else {
        format!("{} edit(s)", row.n_edits.unwrap_or(0))
    };
    collapse(&format!("{} [{kind}] {head}", row.paths[0]), 160)
}

fn is_whole_rewrite(row: &Row) -> bool {
    if row.is_new_file == Some(true) {
        return true;
    }
    if row.n_edits != Some(1) || row.payload.is_none() {
        return false;
    }
    match parse_json(row.payload.as_ref().unwrap()) {
        Some(Json::Arr(arr)) => {
            arr.len() == 1
                && matches!(arr[0].get("old_string"), Some(Json::Str(s)) if s.chars().count() >= WHOLE_REWRITE_OLD_LEN)
        }
        _ => false,
    }
}

fn replace_all_count(row: &Row) -> usize {
    let Some(pl) = &row.payload else { return 0 };
    match parse_json(pl) {
        Some(Json::Arr(arr)) => arr
            .iter()
            .filter(|x| matches!(x.get("replace_all"), Some(Json::Bool(true))))
            .count(),
        _ => 0,
    }
}

// ===========================================================================
// python mode (subtask 3.3)
// ===========================================================================

fn py_has_writeback(code: &str) -> bool {
    let re1 = cached_regex(r"open\([^)]*'[wa][^']*\)");
    let re2 = cached_regex(r#"open\([^)]*"[wa]""#);
    let re3 = cached_regex(r"\.write\(");
    re1.is_match(code) || re2.is_match(code) || code.contains("write_text") || re3.is_match(code)
}

fn py_is_bulk_rewrite(code: &str) -> bool {
    py_has_writeback(code) && (code.contains(".replace(") || code.contains("re.sub"))
}

fn py_is_audit_mask(code: &str) -> bool {
    cached_regex(r"\b(mask|audit|diff)(?:_\w+|\(|\b)").is_match(code)
}

fn py_is_orchestration(code: &str) -> bool {
    code.contains("subprocess") || code.contains("os.system")
}

fn py_is_json_processing(code: &str) -> bool {
    cached_regex(r"json\.(load|loads|dump|dumps)\b").is_match(code)
}

fn py_is_transform(code: &str) -> bool {
    py_has_writeback(code) && cached_regex(r"\.read\(\)|read_text|open\(").is_match(code)
}

/// classify_calls.py: classify_python - ordered code-shape tests, first match wins.
fn classify_python(row: &Row) -> String {
    let code = row.payload.as_deref().unwrap_or("");
    if py_is_bulk_rewrite(code) {
        return "bulk-rewrite".into();
    }
    if py_is_audit_mask(code) {
        return "audit-mask".into();
    }
    if py_is_orchestration(code) {
        return "orchestration".into();
    }
    if py_is_json_processing(code) {
        return "json-processing".into();
    }
    if py_is_transform(code) {
        return "transform".into();
    }
    "analysis".into()
}

// ===========================================================================
// misc mode (subtask 3.4): search_conversation + web_search
// ===========================================================================

fn classify_misc(row: &Row) -> String {
    match row.kind.as_deref() {
        Some("search_conversation") => "conversation-recall".into(),
        Some("web_search") => "web-docs-lookup".into(),
        _ => "other".into(),
    }
}

// ===========================================================================
// Authored abstract patterns (verbatim from classify_calls.py - part of the
// output contract; the TSVs are regenerable only because these live here).
// ===========================================================================

const TERMINAL_PATTERNS: &[(&str, &str)] = &[
    ("lookup-docs", "cd <repo>[/<docs subdir>] && (sed -n '<a>,<b>p' | head -c N | tail -c +N | cat | grep -n 'pat') <doc file(s)> [; second doc in the same command] - chunked reads of START/BUILD/TOOLCHAIN/KB/DESIGN/CODEGEN/TASKS/docs/** per the token-economy protocol"),
    ("lookup-sources", "cd <repo> && (sed -n '<a>,<b>p' | grep -n 'pat' | head/tail | cat | awk) <src/tests/crates file(s)> - chunked reads and pattern searches over Rust sources, test suites and workspace crates"),
    ("lookup-vendored-deps", "L=$CARGO_HOME/registry/src/index.crates.io-*/<crate>-<ver>; ls $L/src/ && cat/grep $L/src/<file> - reading vendored dependency sources (layout-rs, arbitrary, z3, ...) to answer API questions"),
    ("build-test-run", "export RUSTUP_HOME=... CARGO_HOME=... PATH=... && cd <repo> && cargo (test [--workspace|-p X|--test Y|--features smt] | build | clippy --all-targets | fmt [--check] | run -- <rfs subcmd>) - plus direct rustc/gcc/make/clang compiles, ./target/{debug,release}/rfs executions and timeout-wrapped fuzz runs"),
    ("git", "cd <repo> && [export GIT_CONFIG_COUNT=1 ...safe.directory...] && git (log --oneline -N | status --short | show --stat <sha> | add <paths> && commit -m '...') - history inspection and the per-task commits"),
    ("shell-write", "cd <repo> && cat > <file> <<'EOF' ... EOF  /  cat >> TASKS.md <<'EOF' ... EOF  /  tee <file> - creating or appending whole file contents (status files, task files, TASKS.md entries) from heredocs"),
    ("shell-edit", "cd <repo> && sed -i 's/old/new/g' <file(s)>  /  sed -i '<a>,<b>d' <file>  /  perl -pi -e 's/.../' - in-place text surgery on sources or docs"),
    ("file-ops", "(mv | rm -rf | cp | mkdir -p | ln -s | touch) <paths> - tree manipulation: moving modules, cleaning scratch dirs, copying fixtures (incl. /tmp scratch work)"),
    ("env-check", "(whoami | id | uname -a | which X | command -v X | X --version/-V | rustup toolchain list | find / -name X | ls ~/.cargo | ls $CARGO_HOME | ls $RUSTUP_HOME) - environment and toolchain probing, incl. hunting for missing tools"),
    ("python-via-shell", "cd <repo> && python3 - <<'PYEOF' ... PYEOF  /  python3 <script>.py  /  python3 -c '...' - scripted operations run from the shell (rewrites, audits, transforms)"),
    ("other", "residual: bare cd/export segments, for-loops over mixed operations, nohup/background runs, bash <script>.sh, install/network calls (apt-get, curl, npm, npx), echo-only commands, and the two payload-less recorded calls"),
];

const EDIT_FILE_PATTERNS: &[(&str, &str)] = &[
    ("docs-new", "edit_file with old_string==\"\" creating a whole new doc file (status files, task files, phase docs, README sections) - new_string carries the full document content"),
    ("docs-modify", "edit_file with one or more {old_string,new_string} replacements in docs (roadmap/KB/DESIGN/BUILD/TASKS/inventories) - targeted section and table-row edits"),
    ("sources-new", "edit_file with old_string==\"\" creating a whole new Rust source/test file - new_string carries the full module content"),
    ("sources-modify", "edit_file with one or more replacements in src/tests/crates/fuzz - function-level, import-line and test-block edits"),
    ("other", "edits to paths outside docs/sources (session-local scripts under /tmp or the sandbox, .gitignore, scratch fixtures)"),
];

const PYTHON_PATTERNS: &[(&str, &str)] = &[
    ("bulk-rewrite", "read file text, apply ordered str.replace / re.sub substitutions, write back - scripted symbol/path renames across sources or docs (the pattern-fragile half of the edit operations)"),
    ("audit-mask", "extract fn/type identifiers from source with strings masked, diff against inventory rows per file - the Phase 10/11 completeness-audit scripts"),
    ("transform", "read-modify-write line surgery without a rewrite pattern: table-row insertion/deletion, heading fixes, encoding and whitespace normalization"),
    ("json-processing", "parse or emit JSON (training JSONL, edits arrays, call inventories) - load/dump with structural checks"),
    ("orchestration", "drive external commands via subprocess / os.system (cargo runs, git show, tool invocations) and post-process their output"),
    ("analysis", "read-only computation over files/text: counting, searching, reporting - no write-back (residual; incl. pure generators that read nothing)"),
];

const MISC_PATTERNS: &[(&str, &str)] = &[
    ("conversation-recall", "recall conversation history after context compaction or between sessions: 'Execute task N' resume prompts, phase close-out procedures, what was done/committed in earlier tasks, plan and task-file details - restoring working context without re-reading files"),
    ("web-docs-lookup", "external documentation lookup: crate APIs (mermaid-rs-renderer, plantuml-core, layout-rs, railroad, mdr on docs.rs / crates.io / github) and tool syntax references (PlantUML EBNF + state diagrams, mermaid) - three recorded forms: query (3), url fetch (24), image_queries (7)"),
];

fn pattern_lookup<'a>(patterns: &'a [(&'a str, &'a str)], cat: &str) -> &'a str {
    patterns
        .iter()
        .find(|(k, _)| *k == cat)
        .map(|(_, v)| *v)
        .unwrap_or("")
}

/// classify_calls.py ordering rule: pattern-dict keys (in authored order) that
/// have data first, then any extra categories in first-occurrence order.
fn ordered_keys(
    order: &[String],
    buckets: &HashMap<String, Vec<Option<String>>>,
    patterns: &[(&str, &str)],
) -> Vec<(String, Vec<Option<String>>)> {
    let mut keys: Vec<String> = Vec::new();
    for (k, _) in patterns {
        if buckets.contains_key(*k) {
            keys.push(k.to_string());
        }
    }
    for c in order {
        if !patterns.iter().any(|(pk, _)| *pk == c.as_str()) {
            keys.push(c.clone());
        }
    }
    keys.into_iter()
        .map(|c| (c.clone(), buckets[&c].clone()))
        .collect()
}

/// Same ordering rule for the edit_file mode, whose buckets hold whole Rows.
fn ordered_rows(
    order: &[String],
    buckets: &HashMap<String, Vec<Row>>,
    patterns: &[(&str, &str)],
) -> Vec<(String, Vec<Row>)> {
    let mut keys: Vec<String> = Vec::new();
    for (k, _) in patterns {
        if buckets.contains_key(*k) {
            keys.push(k.to_string());
        }
    }
    for c in order {
        if !patterns.iter().any(|(pk, _)| *pk == c.as_str()) {
            keys.push(c.clone());
        }
    }
    keys.into_iter()
        .map(|c| (c.clone(), buckets[&c].clone()))
        .collect()
}

/// classify_calls.py: write_tsv - the six-column TSV for terminal/python/misc.
fn write_tsv(
    out: &Path,
    header_note: &str,
    ordered: &[(String, Vec<Option<String>>)],
    patterns: &[(&str, &str)],
) -> Result<usize, String> {
    let mut text = String::new();
    text.push_str(&format!("# {header_note}\n"));
    text.push_str("category\tcount\tabstract_pattern\texample1\texample2\texample3\n");
    let mut total = 0usize;
    for (cat, payloads) in ordered {
        total += payloads.len();
        let mut ex: Vec<String> = payloads
            .iter()
            .take(3)
            .filter_map(|p| p.as_ref().map(|s| collapse(s, 160)))
            .collect();
        while ex.len() < 3 {
            ex.push(String::new());
        }
        let mut line = vec![
            cat.clone(),
            payloads.len().to_string(),
            pattern_lookup(patterns, cat).to_string(),
        ];
        line.extend(ex);
        text.push_str(&line.join("\t"));
        text.push('\n');
    }
    std::fs::write(out, text).map_err(|e| format!("cannot write {:?}: {e}", out))?;
    Ok(total)
}

/// classify_calls.py: run_terminal.
fn run_terminal(rows: &[Row], out: &Path) -> Result<String, String> {
    let rows: Vec<Row> = rows
        .iter()
        .filter(|r| r.kind.as_deref() == Some("terminal"))
        .cloned()
        .collect();
    let mut buckets: HashMap<String, Vec<Option<String>>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in &rows {
        let cat = classify_terminal(r);
        if !buckets.contains_key(&cat) {
            order.push(cat.clone());
        }
        buckets.entry(cat).or_default().push(r.payload.clone());
    }
    let ordered = ordered_keys(&order, &buckets, TERMINAL_PATTERNS);
    let total = write_tsv(
        out,
        "terminal classification - Phase 12 task 3.1",
        &ordered,
        TERMINAL_PATTERNS,
    )?;
    let mut text = format!("terminal rows: {}, categorized: {total}\n", rows.len());
    if total != rows.len() {
        return Err("FATAL: category counts do not sum to the terminal row count".into());
    }
    for (cat, payloads) in &ordered {
        text.push_str(&format!("  {:<24} {:>6}\n", cat, payloads.len()));
    }
    Ok(text)
}

/// classify_calls.py: run_python.
fn run_python(rows: &[Row], out: &Path) -> Result<String, String> {
    let rows: Vec<Row> = rows
        .iter()
        .filter(|r| r.kind.as_deref() == Some("python"))
        .cloned()
        .collect();
    let mut buckets: HashMap<String, Vec<Option<String>>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in &rows {
        let cat = classify_python(r);
        if !buckets.contains_key(&cat) {
            order.push(cat.clone());
        }
        buckets.entry(cat).or_default().push(r.payload.clone());
    }
    let ordered = ordered_keys(&order, &buckets, PYTHON_PATTERNS);
    let total = write_tsv(
        out,
        "python classification - Phase 12 task 3.3",
        &ordered,
        PYTHON_PATTERNS,
    )?;
    let mut text = format!("python rows: {}, categorized: {total}\n", rows.len());
    if total != rows.len() {
        return Err("FATAL: category counts do not sum to the python row count".into());
    }
    for (cat, payloads) in &ordered {
        text.push_str(&format!("  {:<24} {:>6}\n", cat, payloads.len()));
    }
    Ok(text)
}

/// classify_calls.py: run_misc.
fn run_misc(rows: &[Row], out: &Path) -> Result<String, String> {
    let rows: Vec<Row> = rows
        .iter()
        .filter(|r| {
            matches!(
                r.kind.as_deref(),
                Some("search_conversation") | Some("web_search")
            )
        })
        .cloned()
        .collect();
    let mut buckets: HashMap<String, Vec<Option<String>>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in &rows {
        let cat = classify_misc(r);
        if !buckets.contains_key(&cat) {
            order.push(cat.clone());
        }
        buckets.entry(cat).or_default().push(r.payload.clone());
    }
    let ordered = ordered_keys(&order, &buckets, MISC_PATTERNS);
    let total = write_tsv(
        out,
        "misc classification - Phase 12 task 3.4",
        &ordered,
        MISC_PATTERNS,
    )?;
    let mut text = format!("misc rows: {}, categorized: {total}\n", rows.len());
    if total != rows.len() {
        return Err("FATAL: category counts do not sum to the misc row count".into());
    }
    for (cat, payloads) in &ordered {
        text.push_str(&format!("  {:<24} {:>6}\n", cat, payloads.len()));
    }
    Ok(text)
}

/// classify_calls.py: run_edit_file - the nine-column TSV + footer lines.
fn run_edit_file(rows: &[Row], out: &Path) -> Result<String, String> {
    let rows: Vec<Row> = rows
        .iter()
        .filter(|r| r.kind.as_deref() == Some("edit_file"))
        .cloned()
        .collect();
    let mut buckets: HashMap<String, Vec<Row>> = HashMap::new();
    let mut order: Vec<String> = Vec::new();
    for r in &rows {
        let cat = classify_edit_file(r);
        if !buckets.contains_key(&cat) {
            order.push(cat.clone());
        }
        buckets.entry(cat).or_default().push(r.clone());
    }
    let ordered = ordered_rows(&order, &buckets, EDIT_FILE_PATTERNS);

    let mut hist: HashMap<i64, usize> = HashMap::new();
    let mut total_replace_all = 0usize;
    let mut total_whole = 0usize;
    for r in &rows {
        let key = r.n_edits.map(|n| n as i64).unwrap_or(-1);
        *hist.entry(key).or_insert(0) += 1;
        total_replace_all += replace_all_count(r);
        if is_whole_rewrite(r) {
            total_whole += 1;
        }
    }

    let mut text_file = String::new();
    text_file.push_str("# edit_file classification - Phase 12 task 3.2\n");
    text_file.push_str("category\tcount\tabstract_pattern\texample1\texample2\texample3\tsingle_edit_share\treplace_all_count\twhole_rewrite_share\n");
    let mut total = 0usize;
    for (cat, cat_rows) in &ordered {
        total += cat_rows.len();
        let mut ex: Vec<String> = cat_rows.iter().take(3).map(edit_file_example).collect();
        while ex.len() < 3 {
            ex.push(String::new());
        }
        let single = cat_rows.iter().filter(|r| r.n_edits == Some(1)).count();
        let ra: usize = cat_rows.iter().map(replace_all_count).sum();
        let whole = cat_rows.iter().filter(|r| is_whole_rewrite(r)).count();
        let mut line = vec![
            cat.clone(),
            cat_rows.len().to_string(),
            pattern_lookup(EDIT_FILE_PATTERNS, cat).to_string(),
        ];
        line.extend(ex);
        line.push(format!("{:.4}", single as f64 / cat_rows.len() as f64));
        line.push(ra.to_string());
        line.push(format!("{:.4}", whole as f64 / cat_rows.len() as f64));
        text_file.push_str(&line.join("\t"));
        text_file.push('\n');
    }
    // python sorts the histogram by key (numeric); rebuild in sorted-key order.
    let mut hist_items: Vec<(i64, usize)> = hist.into_iter().collect();
    hist_items.sort_by_key(|(k, _)| *k);
    let hist_str = hist_items
        .iter()
        .map(|(k, v)| format!("{k}:{v}"))
        .collect::<Vec<_>>()
        .join(" ");
    text_file.push_str(&format!("# edits-per-call histogram: {hist_str}\n"));
    text_file.push_str(&format!("# replace_all total: {total_replace_all}\n"));
    text_file.push_str(&format!(
        "# whole-file rewrites (new or single edit with old_string >= {WHOLE_REWRITE_OLD_LEN} chars): {total_whole} ({:.4} of all calls)\n",
        total_whole as f64 / rows.len() as f64
    ));
    std::fs::write(out, text_file).map_err(|e| format!("cannot write {:?}: {e}", out))?;

    let mut text = format!("edit_file rows: {}, categorized: {total}\n", rows.len());
    if total != rows.len() {
        return Err("FATAL: category counts do not sum to the edit_file row count".into());
    }
    for (cat, cat_rows) in &ordered {
        text.push_str(&format!("  {:<24} {:>6}\n", cat, cat_rows.len()));
    }
    Ok(text)
}

/// classify_calls.py: main dispatch - run the requested kind's classifier.
pub fn run_classify(inv: &Path, kind: &str, out: &Path) -> Result<String, String> {
    let rows = load_rows(inv)?;
    match kind {
        "terminal" => run_terminal(&rows, out),
        "edit_file" => run_edit_file(&rows, out),
        "python" => run_python(&rows, out),
        "misc" => run_misc(&rows, out),
        other => Err(format!(
            "mode `{other}` not yet implemented (added by its subtask)"
        )),
    }
}

// ===========================================================================
// Unit tests - the ported functions in isolation (the integration test in
// tests/calls_test.rs is the byte-equality oracle; these pin individual rules)
// ===========================================================================

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn path_class_labels_match_the_python_rules() {
        let repo = REPO_ROOT.as_str();
        assert_eq!(path_class(&format!("{repo}/KB.md")), "docs");
        assert_eq!(path_class("KB.md"), "docs");
        assert_eq!(path_class(&format!("{repo}/docs/dev/parser.md")), "docs");
        assert_eq!(path_class(&format!("{repo}/src/lang/parser.rs")), "sources");
        assert_eq!(path_class("parser.rs"), "sources");
        assert_eq!(path_class(&format!("{repo}/Cargo.toml")), "sources");
        assert_eq!(path_class("/tmp/scratch.py"), "scratch");
        assert_eq!(path_class("target/debug/rfs"), "scratch");
        assert_eq!(
            path_class(&format!("{repo}/.unsloth_tool_output/x")),
            "scratch"
        );
        assert_eq!(path_class("/etc/hosts"), "other");
    }

    #[test]
    fn collapse_collapses_whitespace_and_truncates_at_160() {
        assert_eq!(collapse("a\n\t b   c", 160), "a b c");
        let long = "x".repeat(200);
        let c = collapse(&long, 160);
        assert_eq!(c.chars().count(), 160);
        assert!(c.ends_with("..."));
    }

    #[test]
    fn json_escape_matches_python_ensure_ascii_false() {
        assert_eq!(json_escape("plain"), "plain");
        assert_eq!(json_escape("a\"b"), "a\\\"b");
        assert_eq!(json_escape("a\\b"), "a\\\\b");
        assert_eq!(json_escape("line\nbreak"), "line\\nbreak");
        // non-ASCII stays verbatim (ensure_ascii=False)
        assert_eq!(json_escape("caf\u{e9}"), "caf\u{e9}");
        // other C0 controls use lowercase \u00xx
        assert_eq!(json_escape("\u{01}"), "\\u0001");
    }

    #[test]
    fn parse_json_round_trips_the_edits_shapes() {
        let v = parse_json(r#"[{"old_string":"","new_string":"x"},{"old_string":"a","new_string":"b","replace_all":true}]"#).unwrap();
        match v {
            Json::Arr(items) => {
                assert_eq!(items.len(), 2);
                assert!(matches!(items[0].get("old_string"), Some(Json::Str(s)) if s.is_empty()));
                assert!(matches!(
                    items[1].get("replace_all"),
                    Some(Json::Bool(true))
                ));
            }
            other => panic!("expected array, got {other:?}"),
        }
        // invalid JSON -> None (drives unparsed=True)
        assert!(parse_json("{not json").is_none());
        // valid JSON but not an array -> parses, caller rejects the non-arr
        assert!(matches!(parse_json(r#"{"a":1}"#), Some(Json::Obj(_))));
    }

    #[test]
    fn extract_fenced_uses_width_matched_close() {
        // triple-backtick fence
        let entry = "**code:**\n\n```\nline one\nline two\n```\ntrailer";
        let (p, unp) = extract_fenced(entry, "code");
        assert!(!unp);
        assert_eq!(p.as_deref(), Some("line one\nline two"));
        // widened four-backtick fence (payload contains triple backticks)
        let entry4 = "**edits:**\n\n````json\n[\n  {\"old_string\": \"```rust\\nfn main() {}\\n```\"}\n]\n````\ntrailer";
        let (p, unp) = extract_fenced(entry4, "edits");
        assert!(!unp, "widened fence must parse");
        assert!(
            p.as_ref().unwrap().contains("```rust"),
            "payload kept the inner triple backticks"
        );
    }

    #[test]
    fn extract_inline_takes_to_line_end_minus_closing_backtick() {
        let repo = REPO_ROOT.as_str();
        let entry = format!("**command:** `ls -la {repo}/`\nnext");
        let (p, unp) = extract_inline(&entry, "command");
        assert!(!unp);
        let expected = format!("ls -la {repo}/");
        assert_eq!(p.as_deref(), Some(expected.as_str()));
    }

    #[test]
    fn segments_split_and_strip_env_prefixes() {
        let segs = segments("export A=1 B=2 && cd /x && cargo test | tee log");
        let firsts: Vec<&str> = segs.iter().map(|(f, _)| f.as_str()).collect();
        assert_eq!(firsts, vec!["cd", "cargo", "tee"]);
    }

    #[test]
    fn is_close_fence_requires_exact_width_then_only_ws() {
        assert!(is_close_fence("```", 3));
        assert!(is_close_fence("````   ", 4));
        assert!(!is_close_fence("```, ", 3)); // trailing non-ws
        assert!(!is_close_fence("````", 3)); // wrong width
        assert!(!is_close_fence(" ```", 3)); // leading ws before backticks
    }

    #[test]
    fn classify_terminal_orders_the_rules() {
        let mk = |payload: &str| Row {
            ordinal: 1,
            kind: Some("terminal".into()),
            payload: Some(payload.into()),
            paths: vec![],
            path_classes: vec![],
            unparsed: false,
            is_edit_file: false,
            n_edits: None,
            is_new_file: None,
        };
        // build-test-run beats env-check even when a version probe is present
        let repo = REPO_ROOT.as_str();
        assert_eq!(classify_terminal(&mk("rustc --version")), "build-test-run");
        assert_eq!(classify_terminal(&mk("whoami && uname -a")), "env-check");
        assert_eq!(
            classify_terminal(&mk(&format!("sed -n '1,5p' {repo}/KB.md"))),
            "lookup-docs"
        );
        assert_eq!(
            classify_terminal(&mk(&format!("grep -n x {repo}/src/a.rs"))),
            "lookup-sources"
        );
        assert_eq!(classify_terminal(&mk("echo hi")), "other");
    }

    #[test]
    fn classify_python_orders_the_code_shape_tests() {
        let mk = |payload: &str| Row {
            ordinal: 1,
            kind: Some("python".into()),
            payload: Some(payload.into()),
            paths: vec![],
            path_classes: vec![],
            unparsed: false,
            is_edit_file: false,
            n_edits: None,
            is_new_file: None,
        };
        assert_eq!(
            classify_python(&mk("open('f','w').write(t.replace('a','b'))")),
            "bulk-rewrite"
        );
        assert_eq!(
            classify_python(&mk("def mask_code(s): return s\ndef diff(a,b): pass")),
            "audit-mask"
        );
        assert_eq!(
            classify_python(&mk("import subprocess; subprocess.run(['x'])")),
            "orchestration"
        );
        assert_eq!(
            classify_python(&mk("import json; json.loads(s)")),
            "json-processing"
        );
        assert_eq!(
            classify_python(&mk("open('f','w').write(open('f').read())")),
            "transform"
        );
        assert_eq!(
            classify_python(&mk("print(sum(1 for _ in range(3)))")),
            "analysis"
        );
    }
}
