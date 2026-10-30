# RFS References

Consolidated index of external articles/books/publications with one-line
annotations on what each covers and which RFS component uses it. Algorithm
sections in `ALGORITHMS.md` link into this file.

<a id="dev:references-theory"></a>
## Parsing, automata, compiler construction

External references for the frontend (parser + validator) and the codegen
pipeline - the first two sections of `ALGORITHMS.md`. Each entry: what the
work covers, then which RFS component uses it.

### Parsing theory

- **Vaughan Pratt - "Top-Down Operator Precedence" (Communications of the ACM 16(4), 1973).** The canonical paper on parsing operator expressions by precedence without grammar explosion. RFS uses: `src/parser.rs` - the 12-level expression chain and the separate 8-level formula chain are parsed by precedence climbing, a variant of top-down operator precedence.
- **ISO/IEC 14977 - Information technology: Syntax notation - Extended Backus-Naur Form (EBNF).** The ISO standard defining EBNF grammar notation. RFS uses: `src/ebnf_grammar.rs::GRAMMAR_RULES` - the source grammar is mirrored as machine-readable EBNF data (61 rules, same names and order as DESIGN.md section 6) and pinned to the parser by sync tests.
- **nom - Rust parser combinator library (v8).** A composable parser framework built around `IResult` with a two-tier error model (`Err::Error` backtrackable, `Err::Failure` non-backtrackable). RFS uses: `src/parser.rs` - the entire source parser is written on nom 8 combinators; the two error tiers implement the syntax-error vs semantic-error classes.

### Automata theory

- **Hopcroft, Motwani, Ullman - Introduction to Automata Theory, Languages, and Computation (3rd ed., Cambridge University Press).** The standard reference on finite automata, regular languages, and NFA/DFA conversion. RFS uses: the generated production artifact is a finite-state machine over event inputs (`src/backends/rust_fsm.rs`, `src/backends/c_fsm.rs`); monitor check trees are evaluated as per-sample state machines over traces (`src/monitor.rs`).

### Compiler construction

- **Aho, Lam, Sethi, Ullman - Compilers: Principles, Techniques, and Tools ("Dragon Book", 2nd ed., Addison-Wesley, 2006).** The standard reference on compiler construction: lexical analysis, top-down parsing (recursive descent, LL(k)), operator precedence, IR design, code generation. RFS uses: the two-stage parse-then-validate frontend (`src/parser.rs`, `src/validator.rs`), recursive-descent grammar functions with single-lookahead decision points, and the IR as a single normalized form rendered by per-language backends (`src/codegen.rs`).

## Model checking, SMT/SAT solving

TBD (task 7.2). Fragment anchor added in task 7.2.
