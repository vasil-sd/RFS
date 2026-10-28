# RFS Parser (Class A)

The nom-based parser that turns RFS v0 source text into an IR document, plus
the EBNF grammar model that mirrors it for IDE/tooling output.

<a id="dev:parser-overview"></a>
## Overview

TBD (task 8.1): parser architecture - entry points `src/parser.rs::parse_source`
and `src/parser.rs::parse_file`, the nom combinator structure, error reporting
via `src/error.rs::ParseError`, and the `KEYWORDS` reserved-word table; the
relationship to `src/ebnf_grammar.rs` (EBNF model + text/md/plantuml/svg
renderers pinned against DESIGN.md section 6 by sync tests).
