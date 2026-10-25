use std::fmt;

/// A byte span in the source text.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Span {
    pub start: usize,
    pub end: usize,
}

impl Span {
    pub fn new(start: usize, end: usize) -> Self {
        Self { start, end }
    }

    pub fn point(pos: usize) -> Self {
        Self {
            start: pos,
            end: pos,
        }
    }
}

/// Parser/lexer error with an optional source copy for rich diagnostics.
#[derive(Debug, Clone, PartialEq)]
pub struct ParseError {
    pub message: String,
    pub span: Span,
    pub source: Option<String>,
}

impl ParseError {
    pub fn new(message: impl Into<String>, span: Span) -> Self {
        Self {
            message: message.into(),
            span,
            source: None,
        }
    }

    pub fn with_source(mut self, source: String) -> Self {
        self.source = Some(source);
        self
    }

    pub fn line_col(&self) -> (usize, usize) {
        let src = self.source.as_deref().unwrap_or("");
        line_col_in(src, self.span.start)
    }

    pub fn rich_display(&self) -> String {
        use std::fmt::Write as _;
        let (line, col) = self.line_col();
        let mut out = String::new();

        if let Some(src) = &self.source {
            let byte = self.span.start.min(src.len());
            let line_start = src[..byte].rfind('\n').map(|i| i + 1).unwrap_or(0);
            let line_end = src[byte..]
                .find('\n')
                .map(|i| byte + i)
                .unwrap_or(src.len());
            let snippet = src[line_start..line_end].trim_end();

            let _ = writeln!(out, "{}:{}: {}", line, col, self.message);
            let _ = writeln!(out, "  |");
            let _ = writeln!(out, "{} |{}", line, snippet);
            let caret = format!("{}^", " ".repeat(col.saturating_sub(1)));
            let _ = writeln!(out, "  |{}", caret);
        } else {
            let _ = write!(out, "{}: {}", self.message, self.span.start);
        }

        out
    }
}

fn line_col_in(src: &str, idx: usize) -> (usize, usize) {
    let idx = idx.min(src.len());
    let mut line = 1;
    let mut col = 1;
    for (i, b) in src.bytes().enumerate() {
        if i >= idx {
            break;
        }
        if b == b'\n' {
            line += 1;
            col = 1;
        } else {
            col += 1;
        }
    }
    (line, col)
}

impl fmt::Display for ParseError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(&self.rich_display())
    }
}

impl std::error::Error for ParseError {}
