//! Hand-rolled JSON reader/writer for the RFS toolchain.
//!
//! # Why hand-rolled instead of an external crate (decision rationale)
//!
//! This module is the ONLY JSON implementation in the toolchain. The external
//! JSON dependency stack was removed in Phase 5 task 2; every JSON document the tool
//! produces or consumes (IR round-trips, all `--json` CLI outputs, training
//! JSONL validation, benchmark reports, and - from task 10 on - the generated
//! MC engines' JSON) goes through this module. The decision to hand-roll it
//! rests on four properties no external crate can contractually provide:
//!
//! 1. **Key ordering guarantee.** Objects are `Vec<(String, Json)>` and
//!    serialize in DECLARATION order - the exact order our code writes the
//!    keys. External crates' default map-backed objects are alphabetical
//!    (BTreeMap) or insertion-order-dependent on a third-party map type;
//!    either way the key sequence is not under our control. Consumers can
//!    rely on the exact key sequence as part of the wire contract, and golden
//!    tests pin it byte-for-byte.
//! 2. **Always byte-identical text output.** The rendered text depends only
//!    on the value - never on platform, Rust version, allocator behavior, or
//!    a dependency update. External crates do change formatting details,
//!    escaping edge cases, and number rendering across versions; every such
//!    change would silently break our byte-pinned parity tests (cross-backend
//!    MC JSON identity, golden files) and any downstream consumer that diffs
//!    tool output.
//! 3. **Zero external dependencies.** Consistent with the generated-code
//!    constraint (self-contained artifacts): from task 10 on, a copy of this
//!    module is embedded VERBATIM in the generated MC engines, so toolchain
//!    and generated JSON behavior are identical by construction, not by hope.
//! 4. **Stable wire format under our control.** The IR JSON shape evolves
//!    only when we deliberately change `to_json`/`from_json` in `src/ir.rs`;
//!    a dependency upgrade can never reshape our output or break our inputs.
//!
//! # Scope limits (documented)
//!
//! - **Integers only.** The value model has no float variant. No current or
//!   planned RFS JSON contains floats; if a future need appears, extend the
//!   model deliberately (writer escaping/number formatting is pinned by tests
//!   here and in golden files).
//! - **Strict reader.** No trailing garbage, no NaN/Infinity, no leading
//!   zeros, no duplicate object keys, nesting depth capped at `MAX_DEPTH`.
//!   Error positions are byte offsets into the input.
//!
//! # Formatting contract (pinned by tests below)
//!
//! - Compact: `"key":value`, no whitespace outside strings.
//! - Pretty: 2-space indent, `"key": value`, one element per line; empty
//!   objects/arrays stay inline (`{}` / `[]`).
//! - String escaping: `"` `\` `\b` `\t` `\n` `\f` `\r`; other control chars
//!   U+0000..=U+001F as `\u00XX` (lowercase hex); everything else raw UTF-8.
//! - Integers via `i64` Display (no padding, no exponents).
//! Docs: dev:ir-json-rw-overview, dev:algorithms-ir-codegen, llm:ir-json-rw

use std::fmt;

/// Maximum nesting depth accepted by the reader (bounds recursion).
pub const MAX_DEPTH: usize = 512;

/// A JSON value. Objects are ORDERED key/value vectors: serialization order
/// is exactly declaration order, and the reader rejects duplicate keys.
#[derive(Debug, Clone, PartialEq)]
pub enum Json {
    Null,
    Bool(bool),
    Int(i64),
    Str(String),
    Arr(Vec<Json>),
    Obj(Vec<(String, Json)>),
}

impl Json {
    /// Compact constructor for objects (declaration order is preserved).
    pub fn obj(pairs: Vec<(String, Json)>) -> Self {
        Json::Obj(pairs)
    }

    /// Compact constructor for arrays.
    pub fn arr(items: Vec<Json>) -> Self {
        Json::Arr(items)
    }

    /// Compact constructor for strings (takes `impl Into<String>`).
    pub fn str(s: impl Into<String>) -> Self {
        Json::Str(s.into())
    }

    /// Compact constructor for integers.
    pub fn int(n: i64) -> Self {
        Json::Int(n)
    }

    /// Compact constructor for booleans.
    pub fn bool(b: bool) -> Self {
        Json::Bool(b)
    }

    // ------------------------------------------------------------------
    // Writer
    // ------------------------------------------------------------------

    /// Render as compact JSON (no whitespace outside strings).
    pub fn to_string_compact(&self) -> String {
        let mut out = String::new();
        write_compact(&mut out, self);
        out
    }

    /// Render as pretty JSON with 2-space indentation.
    pub fn to_string_pretty(&self) -> String {
        let mut out = String::new();
        write_pretty(&mut out, self, 0);
        out
    }

    // ------------------------------------------------------------------
    // Typed extraction
    // ------------------------------------------------------------------

    /// Look up an object field by key (None if not an object or absent).
    pub fn get(&self, key: &str) -> Option<&Json> {
        match self {
            Json::Obj(pairs) => pairs.iter().find(|(k, _)| k == key).map(|(_, v)| v),
            _ => None,
        }
    }

    /// The string payload if this is a `Str`.
    pub fn as_str(&self) -> Option<&str> {
        match self {
            Json::Str(s) => Some(s.as_str()),
            _ => None,
        }
    }

    /// The integer payload if this is an `Int`.
    pub fn as_int(&self) -> Option<i64> {
        match self {
            Json::Int(n) => Some(*n),
            _ => None,
        }
    }

    /// The boolean payload if this is a `Bool`.
    pub fn as_bool(&self) -> Option<bool> {
        match self {
            Json::Bool(b) => Some(*b),
            _ => None,
        }
    }

    /// The element list if this is an `Arr`.
    pub fn as_arr(&self) -> Option<&Vec<Json>> {
        match self {
            Json::Arr(items) => Some(items),
            _ => None,
        }
    }

    /// The ordered key/value pairs if this is an `Obj`.
    pub fn as_obj(&self) -> Option<&Vec<(String, Json)>> {
        match self {
            Json::Obj(pairs) => Some(pairs),
            _ => None,
        }
    }
}

impl fmt::Display for Json {
    /// Compact rendering (the `Display` form); see `to_string_pretty` for the
    /// indented layout.
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.to_string_compact())
    }
}

fn write_compact(out: &mut String, v: &Json) {
    match v {
        Json::Null => out.push_str("null"),
        Json::Bool(b) => out.push_str(if *b { "true" } else { "false" }),
        Json::Int(n) => out.push_str(&n.to_string()),
        Json::Str(s) => write_escaped(out, s),
        Json::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_compact(out, item);
            }
            out.push(']');
        }
        Json::Obj(pairs) => {
            out.push('{');
            for (i, (k, val)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                write_escaped(out, k);
                out.push(':');
                write_compact(out, val);
            }
            out.push('}');
        }
    }
}

fn write_pretty(out: &mut String, v: &Json, indent: usize) {
    match v {
        Json::Arr(items) if items.is_empty() => out.push_str("[]"),
        Json::Obj(pairs) if pairs.is_empty() => out.push_str("{}"),
        Json::Arr(items) => {
            out.push('[');
            for (i, item) in items.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&" ".repeat((indent + 1) * 2));
                write_pretty(out, item, indent + 1);
            }
            out.push('\n');
            out.push_str(&" ".repeat(indent * 2));
            out.push(']');
        }
        Json::Obj(pairs) => {
            out.push('{');
            for (i, (k, val)) in pairs.iter().enumerate() {
                if i > 0 {
                    out.push(',');
                }
                out.push('\n');
                out.push_str(&" ".repeat((indent + 1) * 2));
                write_escaped(out, k);
                out.push_str(": ");
                write_pretty(out, val, indent + 1);
            }
            out.push('\n');
            out.push_str(&" ".repeat(indent * 2));
            out.push('}');
        }
        other => write_compact(out, other),
    }
}

/// Escape a string exactly per the module-level formatting contract.
fn write_escaped(out: &mut String, s: &str) {
    out.push('"');
    for c in s.chars() {
        match c {
            '"' => out.push_str("\\\""),
            '\\' => out.push_str("\\\\"),
            '\u{0008}' => out.push_str("\\b"),
            '\t' => out.push_str("\\t"),
            '\n' => out.push_str("\\n"),
            '\u{000C}' => out.push_str("\\f"),
            '\r' => out.push_str("\\r"),
            c if (c as u32) < 0x20 => {
                // Remaining control chars: \u00XX, lowercase hex.
                out.push_str(&format!("\\u{:04x}", c as u32));
            }
            c => out.push(c),
        }
    }
    out.push('"');
}

// ---------------------------------------------------------------------------
// Reader
// ---------------------------------------------------------------------------

/// A JSON parse error with a byte offset into the input.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct JsonError {
    pub message: String,
    pub offset: usize,
}

impl fmt::Display for JsonError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{} (byte offset {})", self.message, self.offset)
    }
}

impl std::error::Error for JsonError {}

/// Parse a strict JSON document. See the module docs for scope limits:
/// integers only, no trailing garbage, no duplicate keys, depth <= `MAX_DEPTH`.
pub fn parse(input: &str) -> Result<Json, JsonError> {
    let mut r = Reader { s: input, pos: 0 };
    r.skip_ws();
    let v = r.parse_value(0)?;
    r.skip_ws();
    if !r.at_end() {
        return Err(r.err("trailing characters after JSON document"));
    }
    Ok(v)
}

struct Reader<'a> {
    s: &'a str,
    pos: usize,
}

impl<'a> Reader<'a> {
    fn at_end(&self) -> bool {
        self.pos >= self.s.len()
    }

    /// The character at the cursor (input is `&str`, so always valid UTF-8).
    fn peek(&self) -> Option<char> {
        self.s.get(self.pos..).and_then(|t| t.chars().next())
    }

    fn next_char(&mut self) -> Result<char, JsonError> {
        let Some(c) = self.peek() else {
            return Err(self.err("unexpected end of input"));
        };
        self.pos += c.len_utf8();
        Ok(c)
    }

    fn err(&self, message: &str) -> JsonError {
        JsonError {
            message: message.to_string(),
            offset: self.pos,
        }
    }

    /// An error whose offset was captured earlier (e.g. at the start of a
    /// number or escape sequence).
    fn err_at(&self, message: &str, offset: usize) -> JsonError {
        JsonError {
            message: message.to_string(),
            offset,
        }
    }

    fn skip_ws(&mut self) {
        while let Some(c) = self.peek() {
            match c {
                ' ' | '\t' | '\n' | '\r' => self.pos += 1,
                _ => break,
            }
        }
    }

    fn parse_value(&mut self, depth: usize) -> Result<Json, JsonError> {
        if depth > MAX_DEPTH {
            return Err(self.err("maximum nesting depth exceeded"));
        }
        match self.peek() {
            None => Err(self.err("unexpected end of input")),
            Some('{') => self.parse_object(depth),
            Some('[') => self.parse_array(depth),
            Some('"') => Ok(Json::Str(self.parse_string()?)),
            Some('t') => self.parse_literal("true", Json::Bool(true)),
            Some('f') => self.parse_literal("false", Json::Bool(false)),
            Some('n') => self.parse_literal("null", Json::Null),
            Some(c) if c == '-' || c.is_ascii_digit() => self.parse_number(),
            Some(c) => Err(self.err(&format!("unexpected character '{c}'"))),
        }
    }

    fn parse_literal(&mut self, lit: &str, value: Json) -> Result<Json, JsonError> {
        if self
            .s
            .get(self.pos..)
            .is_some_and(|rest| rest.starts_with(lit))
        {
            self.pos += lit.len();
            return Ok(value);
        }
        Err(self.err(&format!("invalid literal (expected '{lit}')")))
    }

    fn parse_number(&mut self) -> Result<Json, JsonError> {
        let start = self.pos;
        let neg = if self.peek() == Some('-') {
            self.next_char()?;
            true
        } else {
            false
        };
        let Some(first) = self.peek() else {
            return Err(self.err("expected digit after '-'"));
        };
        if !first.is_ascii_digit() {
            return Err(self.err("expected digit"));
        }
        let mut mag: u64 = (first as u64) - ('0' as u64);
        self.next_char()?;
        if first == '0' {
            // JSON forbids leading zeros: `0` must stand alone.
            if let Some(c) = self.peek() {
                if c.is_ascii_digit() {
                    return Err(self.err("invalid leading zero in number"));
                }
            }
        } else {
            while let Some(c) = self.peek() {
                if !c.is_ascii_digit() {
                    break;
                }
                mag = mag
                    .checked_mul(10)
                    .and_then(|m| m.checked_add((c as u64) - ('0' as u64)))
                    .ok_or_else(|| self.err_at("number out of i64 range", start))?;
                self.next_char()?;
            }
        }
        // Integer-only scope: a decimal point or exponent is a syntax error
        // for us (documented in the module docs).
        if let Some(c) = self.peek() {
            if c == '.' || c == 'e' || c == 'E' {
                return Err(self.err(
                    "fractional/exponent numbers are not supported (integer-only JSON scope)",
                ));
            }
        }
        // 2^63 is the largest magnitude a negative i64 can hold.
        let limit: u64 = if neg {
            9_223_372_036_854_775_808
        } else {
            9_223_372_036_854_775_807
        };
        if mag > limit {
            return Err(self.err_at("number out of i64 range", start));
        }
        let v: i64 = if neg {
            -(mag as i128) as i64
        } else {
            mag as i64
        };
        Ok(Json::Int(v))
    }

    fn parse_string(&mut self) -> Result<String, JsonError> {
        // Cursor is at the opening '"'.
        self.next_char()?;
        let mut out = String::new();
        loop {
            let Some(c) = self.peek() else {
                return Err(self.err("unterminated string"));
            };
            match c {
                '"' => {
                    self.next_char()?;
                    return Ok(out);
                }
                '\\' => {
                    // All escape diagnostics point at the backslash.
                    let esc_start = self.pos;
                    self.next_char()?;
                    let Some(e) = self.peek() else {
                        return Err(self.err("unterminated escape sequence"));
                    };
                    match e {
                        '"' => out.push('"'),
                        '\\' => out.push('\\'),
                        '/' => out.push('/'),
                        'b' => out.push('\u{0008}'),
                        'f' => out.push('\u{000C}'),
                        'n' => out.push('\n'),
                        'r' => out.push('\r'),
                        't' => out.push('\t'),
                        'u' => {
                            self.next_char()?;
                            let hi = self.parse_hex4(esc_start)?;
                            if (0xD800..=0xDBFF).contains(&hi) {
                                // High surrogate: a low one must follow.
                                if !self
                                    .s
                                    .get(self.pos..)
                                    .is_some_and(|rest| rest.starts_with("\\u"))
                                {
                                    return Err(self.err_at(
                                        "high surrogate not followed by \\u low surrogate",
                                        esc_start,
                                    ));
                                }
                                self.pos += 2;
                                let lo = self.parse_hex4(esc_start)?;
                                if !(0xDC00..=0xDFFF).contains(&lo) {
                                    return Err(self.err_at("invalid low surrogate", esc_start));
                                }
                                let cp = (0x1_0000 + (((hi - 0xD800) as u32) << 10))
                                    | ((lo - 0xDC00) as u32);
                                out.push(
                                    char::from_u32(cp).ok_or_else(|| {
                                        self.err_at("invalid code point", esc_start)
                                    })?,
                                );
                            } else if (0xDC00..=0xDFFF).contains(&hi) {
                                return Err(self.err_at("unexpected low surrogate", esc_start));
                            } else {
                                out.push(
                                    char::from_u32(hi as u32).ok_or_else(|| {
                                        self.err_at("invalid code point", esc_start)
                                    })?,
                                );
                            }
                        }
                        _ => {
                            return Err(
                                self.err_at(&format!("invalid escape sequence '\\{e}'"), esc_start)
                            );
                        }
                    }
                    if e != 'u' {
                        self.next_char()?;
                    }
                }
                c if (c as u32) < 0x20 => {
                    return Err(self.err("unescaped control character in string"));
                }
                _ => {
                    out.push(c);
                    self.next_char()?;
                }
            }
        }
    }

    fn parse_hex4(&mut self, esc_start: usize) -> Result<u16, JsonError> {
        let Some(rest) = self.s.get(self.pos..).filter(|t| t.len() >= 4) else {
            return Err(self.err_at("truncated \\u escape", esc_start));
        };
        let hex = &rest[..4];
        if !hex.chars().all(|c| c.is_ascii_hexdigit()) {
            return Err(self.err_at("invalid \\u escape (expected 4 hex digits)", esc_start));
        }
        let v = u16::from_str_radix(hex, 16).expect("validated hex digits fit in u16");
        self.pos += 4;
        Ok(v)
    }

    fn parse_object(&mut self, depth: usize) -> Result<Json, JsonError> {
        // Cursor is at '{'.
        self.next_char()?;
        let mut pairs: Vec<(String, Json)> = Vec::new();
        self.skip_ws();
        if self.peek() == Some('}') {
            self.next_char()?;
            return Ok(Json::Obj(pairs));
        }
        loop {
            // Strict JSON: no trailing comma - after ',' a key must follow.
            self.skip_ws();
            if self.peek() != Some('"') {
                return Err(self.err("expected object key string"));
            }
            let key_start = self.pos;
            let key = self.parse_string()?;
            self.skip_ws();
            if self.peek() != Some(':') {
                return Err(self.err("expected ':' after object key"));
            }
            self.next_char()?;
            self.skip_ws();
            if pairs.iter().any(|(k, _)| *k == key) {
                return Err(self.err_at("duplicate object key", key_start));
            }
            let value = self.parse_value(depth + 1)?;
            pairs.push((key, value));
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.next_char()?;
                    continue;
                }
                Some('}') => {
                    self.next_char()?;
                    return Ok(Json::Obj(pairs));
                }
                Some(c) => {
                    return Err(self.err(&format!(
                        "expected ',' or '}}' after object entry (got '{c}')"
                    )));
                }
                None => return Err(self.err("unexpected end of input")),
            }
        }
    }

    fn parse_array(&mut self, depth: usize) -> Result<Json, JsonError> {
        // Cursor is at '['.
        self.next_char()?;
        let mut items: Vec<Json> = Vec::new();
        self.skip_ws();
        if self.peek() == Some(']') {
            self.next_char()?;
            return Ok(Json::Arr(items));
        }
        loop {
            // Pretty layout puts a newline after commas - skip it.
            self.skip_ws();
            let value = self.parse_value(depth + 1)?;
            items.push(value);
            self.skip_ws();
            match self.peek() {
                Some(',') => {
                    self.next_char()?;
                    continue;
                }
                Some(']') => {
                    self.next_char()?;
                    return Ok(Json::Arr(items));
                }
                Some(c) => {
                    return Err(self.err(&format!(
                        "expected ',' or ']' after array element (got '{c}')"
                    )));
                }
                None => return Err(self.err("unexpected end of input")),
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // -- Writer: escaping pins -------------------------------------------

    #[test]
    fn writer_escapes_specials() {
        let s = Json::str("\"quote\\back/slash\u{8}\t\n\u{c}\r end");
        assert_eq!(
            s.to_string(),
            "\"\\\"quote\\\\back/slash\\b\\t\\n\\f\\r end\""
        );
    }

    #[test]
    fn writer_escapes_control_chars_as_u00xx() {
        // 0x01 and 0x1B (ESC) become \u00XX; DEL 0x7F stays raw (JSON only
        // mandates escaping U+0000..=U+001F).
        let s = Json::str("\u{1}\u{1b}\u{7f}");
        assert_eq!(s.to_string(), "\"\\u0001\\u001b\u{7f}\"");
    }

    #[test]
    fn writer_keeps_non_ascii_raw() {
        let s = Json::str("héllo → 世界");
        assert_eq!(s.to_string(), "\"héllo → 世界\"");
    }

    // -- Writer: layout pins ---------------------------------------------

    #[test]
    fn writer_compact_layout() {
        let v = Json::obj(vec![
            ("a".into(), Json::int(1)),
            (
                "b".into(),
                Json::arr(vec![Json::Bool(true), Json::Null, Json::Str("x".into())]),
            ),
            ("c".into(), Json::obj(vec![])),
        ]);
        assert_eq!(v.to_string(), r#"{"a":1,"b":[true,null,"x"],"c":{}}"#);
    }

    #[test]
    fn writer_pretty_layout() {
        let v = Json::obj(vec![
            ("a".into(), Json::int(1)),
            ("b".into(), Json::arr(vec![Json::int(2), Json::arr(vec![])])),
            ("c".into(), Json::obj(vec![])),
        ]);
        assert_eq!(
            v.to_string_pretty(),
            "{\n  \"a\": 1,\n  \"b\": [\n    2,\n    []\n  ],\n  \"c\": {}\n}"
        );
    }

    #[test]
    fn writer_empty_containers_inline() {
        assert_eq!(Json::obj(vec![]).to_string_pretty(), "{}");
        assert_eq!(Json::arr(vec![]).to_string_pretty(), "[]");
        assert_eq!(Json::obj(vec![]).to_string(), "{}");
        assert_eq!(Json::arr(vec![]).to_string(), "[]");
    }

    #[test]
    fn writer_int_rendering() {
        assert_eq!(Json::int(0).to_string(), "0");
        assert_eq!(Json::int(-42).to_string(), "-42");
        assert_eq!(Json::int(i64::MAX).to_string(), "9223372036854775807");
        assert_eq!(Json::int(i64::MIN).to_string(), "-9223372036854775808");
    }

    // -- Reader: success cases --------------------------------------------

    #[test]
    fn reader_parses_all_value_kinds() {
        let v = parse(r#"{"null":null,"bool":[true,false],"int":-17,"str":"s","arr":[],"obj":{}}"#)
            .expect("parse");
        assert_eq!(
            v,
            Json::obj(vec![
                ("null".into(), Json::Null),
                (
                    "bool".into(),
                    Json::arr(vec![Json::Bool(true), Json::Bool(false)])
                ),
                ("int".into(), Json::Int(-17)),
                ("str".into(), Json::Str("s".into())),
                ("arr".into(), Json::Arr(vec![])),
                ("obj".into(), Json::Obj(vec![])),
            ])
        );
    }

    #[test]
    fn reader_tolerates_surrounding_whitespace() {
        assert_eq!(
            parse("  \n\t {\"a\" : 1 } \r\n ").expect("parse"),
            Json::obj(vec![("a".into(), Json::int(1))])
        );
    }

    #[test]
    fn reader_string_escapes() {
        let doc = "\"\\\"\\\\\\/\\b\\f\\n\\r\\t\\u0041\\u0000x\"";
        let v = parse(doc).expect("parse");
        assert_eq!(v, Json::Str("\"\\/\u{8}\u{c}\n\r\tA\u{0}x".into()));
    }

    #[test]
    fn reader_surrogate_pair_becomes_non_bmp_char() {
        // U+1F600 = \uD83D\uDE00.
        let v = parse(r#""\ud83d\ude00""#).expect("parse");
        assert_eq!(v, Json::Str("\u{1F600}".into()));
    }

    #[test]
    fn reader_number_boundaries() {
        assert_eq!(parse("0").unwrap(), Json::Int(0));
        assert_eq!(parse("-0").unwrap(), Json::Int(0));
        assert_eq!(parse("9223372036854775807").unwrap(), Json::Int(i64::MAX));
        assert_eq!(parse("-9223372036854775808").unwrap(), Json::Int(i64::MIN));
    }

    // -- Reader: error classes with offsets --------------------------------

    fn expect_err(input: &str, offset: usize, msg_part: &str) {
        let err = match parse(input) {
            Err(e) => e,
            Ok(v) => panic!("expected error for {input:?}, got {v:?}"),
        };
        assert_eq!(err.offset, offset, "offset for {input:?}: {}", err.message);
        assert!(
            err.message.contains(msg_part),
            "message '{}' should contain {:?} (input {input:?})",
            err.message,
            msg_part
        );
    }

    #[test]
    fn reader_rejects_trailing_garbage() {
        expect_err("{} x", 3, "trailing characters");
        expect_err("[1] ]", 4, "trailing characters");
    }

    #[test]
    fn reader_rejects_duplicate_keys() {
        // Offset points at the opening quote of the second occurrence.
        expect_err(r#"{"a":1,"a":2}"#, 7, "duplicate object key");
        // Duplicates in different objects are fine.
        assert!(parse(r#"{"x":{"a":1},"y":{"a":2}}"#).is_ok());
    }

    #[test]
    fn reader_rejects_number_syntax() {
        expect_err("01", 1, "leading zero");
        expect_err("-01", 2, "leading zero");
        expect_err("1.5", 1, "fractional/exponent");
        expect_err("1e3", 1, "fractional/exponent");
        expect_err("1E-3", 1, "fractional/exponent");
        // Range errors point at the start of the number.
        expect_err("9223372036854775808", 0, "out of i64 range");
        expect_err("-9223372036854775809", 0, "out of i64 range");
        expect_err("-", 1, "expected digit");
    }

    #[test]
    fn reader_rejects_nan_and_infinity() {
        expect_err("NaN", 0, "unexpected character");
        expect_err("Infinity", 0, "unexpected character");
        expect_err("-Infinity", 1, "expected digit");
    }

    #[test]
    fn reader_rejects_bad_strings() {
        // Escape errors point at the backslash of the escape.
        expect_err("\"abc", 4, "unterminated string");
        expect_err("\"a\\q\"", 2, "invalid escape");
        expect_err("\"a\\u12\"", 2, "truncated");
        expect_err("\"a\\uZZZ1\"", 2, "4 hex digits");
        // Raw control character inside a string.
        expect_err("\"a\nb\"", 2, "unescaped control character");
    }

    #[test]
    fn reader_rejects_bad_surrogates() {
        // All point at the backslash of the offending escape (offset 1).
        // High surrogate not followed by a low one.
        expect_err(r#""\ud83dX""#, 1, "low surrogate");
        // Low surrogate without a high one.
        expect_err(r#""\ude00""#, 1, "unexpected low surrogate");
        // High followed by a non-surrogate \u escape.
        expect_err(r#""\ud83d\u0041""#, 1, "invalid low surrogate");
    }

    #[test]
    fn reader_rejects_structural_errors() {
        expect_err("", 0, "unexpected end of input");
        expect_err("{", 1, "expected object key string");
        expect_err("{\"a\"}", 4, "expected ':'");
        expect_err("[1 2]", 3, "expected ',' or ']'");
        expect_err("{\"a\":1,}", 7, "expected object key string");
        expect_err("nul", 0, "invalid literal");
        expect_err("tru", 0, "invalid literal");
    }

    #[test]
    fn reader_rejects_trailing_commas() {
        // Strict JSON: no trailing commas.
        expect_err(r#"{"a":1,}"#, 7, "expected object key string");
        expect_err("[1,]", 3, "unexpected character");
    }

    #[test]
    fn reader_enforces_depth_limit() {
        // MAX_DEPTH + 1 nested arrays: the innermost value (offset
        // MAX_DEPTH + 1) is rejected; exactly MAX_DEPTH is fine.
        let deep = format!(
            "{}1{}",
            "[".repeat(MAX_DEPTH + 1),
            "]".repeat(MAX_DEPTH + 1)
        );
        expect_err(&deep, MAX_DEPTH + 1, "maximum nesting depth");
        let ok = format!("{}1{}", "[".repeat(MAX_DEPTH), "]".repeat(MAX_DEPTH));
        assert!(parse(&ok).is_ok());
    }

    // -- Typed extraction ---------------------------------------------------

    #[test]
    fn typed_extraction_helpers() {
        let v = Json::obj(vec![
            ("s".into(), Json::Str("hi".into())),
            ("i".into(), Json::Int(7)),
            ("b".into(), Json::Bool(true)),
            ("a".into(), Json::arr(vec![Json::Null])),
            ("o".into(), Json::obj(vec![("k".into(), Json::int(1))])),
        ]);
        assert_eq!(v.get("s").and_then(Json::as_str), Some("hi"));
        assert_eq!(v.get("i").and_then(Json::as_int), Some(7));
        assert_eq!(v.get("b").and_then(Json::as_bool), Some(true));
        assert_eq!(v.get("a").and_then(Json::as_arr).map(Vec::len), Some(1));
        assert_eq!(v.get("o").and_then(Json::as_obj).map(Vec::len), Some(1));
        assert_eq!(v.get("missing"), None);
        assert_eq!(Json::int(3).get("x"), None);
        assert_eq!(Json::Str("s".into()).as_int(), None);
        assert_eq!(Json::Null.as_bool(), None);
    }

    // -- Round-trip property -------------------------------------------------

    /// Corpus of IR-shaped and CLI-shaped documents covering every value kind,
    /// escaping edge cases, empty containers, and deep nesting.
    fn corpus() -> Vec<Json> {
        vec![
            Json::Null,
            Json::Bool(true),
            Json::Bool(false),
            Json::Int(0),
            Json::Int(-1),
            Json::Int(i64::MAX),
            Json::Int(i64::MIN),
            Json::Str(String::new()),
            Json::Str("plain".into()),
            Json::Str("\"quotes\\back/slash\u{8}\t\n\u{c}\r\u{1}\u{1f} end".into()),
            Json::Str("héllo → 世界 🚀".into()),
            Json::arr(vec![]),
            Json::obj(vec![]),
            Json::arr(vec![
                Json::Null,
                Json::Bool(false),
                Json::Int(-5),
                Json::Str("x".into()),
            ]),
            // IR-shaped document.
            Json::obj(vec![
                ("ir_version".into(), Json::int(0)),
                (
                    "modules".into(),
                    Json::arr(vec![Json::obj(vec![
                        ("name".into(), Json::Str("m".into())),
                        ("enums".into(), Json::arr(vec![])),
                        ("initial_state".into(), Json::Null),
                        (
                            "events".into(),
                            Json::arr(vec![Json::obj(vec![
                                ("name".into(), Json::Str("Go".into())),
                                (
                                    "variants".into(),
                                    Json::arr(vec![Json::obj(vec![
                                        ("kind".into(), Json::Str("unit".into())),
                                        ("name".into(), Json::Str("Go".into())),
                                    ])]),
                                ),
                            ])]),
                        ),
                    ])]),
                ),
            ]),
            // CLI-shaped document with a long string (think-block style).
            Json::obj(vec![(
                "messages".into(),
                Json::arr(vec![Json::obj(vec![
                    ("role".into(), Json::Str("user".into())),
                    (
                        "content".into(),
                        Json::Str("line1\nline2\tindented \"quoted\" end".into()),
                    ),
                ])]),
            )]),
        ]
    }

    #[test]
    fn roundtrip_property_compact_and_pretty() {
        for v in corpus() {
            let compact = v.to_string();
            assert_eq!(parse(&compact).expect("compact re-parse"), v);
            let pretty = v.to_string_pretty();
            assert_eq!(parse(&pretty).expect("pretty re-parse"), v);
            // Compact and pretty render the same value.
            assert_eq!(parse(&pretty).unwrap(), parse(&compact).unwrap());
        }
    }

    #[test]
    fn roundtrip_property_key_order_preserved() {
        let v = Json::obj(vec![
            ("zeta".into(), Json::int(1)),
            ("alpha".into(), Json::int(2)),
            ("mid".into(), Json::int(3)),
        ]);
        // Declaration order survives parse + re-render in BOTH layouts -
        // never alphabetized.
        assert_eq!(v.to_string(), r#"{"zeta":1,"alpha":2,"mid":3}"#);
        let reparsed = parse(&v.to_string_pretty()).expect("parse");
        assert_eq!(reparsed, v);
        assert_eq!(reparsed.get("alpha").and_then(Json::as_int), Some(2));
    }
}
