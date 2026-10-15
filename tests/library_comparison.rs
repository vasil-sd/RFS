//! Parser-library comparison result.
//!
//! Both `chumsky` and `nom` were installed and evaluated.
//! `nom` was selected for the RFS Phase 0 rewrite because its complete parsers
//! integrate cleanly with the existing IR and error model.
//! Chumsky remains a possible future upgrade for richer diagnostics/recovery.

mod nom_smoke {
    use nom::{
        bytes::complete::tag,
        character::complete::{multispace0, multispace1, satisfy},
        multi::many0,
        IResult, Parser,
    };

    fn ident_start(input: &str) -> IResult<&str, char> {
        satisfy(|c: char| c.is_ascii_alphabetic() || c == '_')(input)
    }

    fn ident_cont(input: &str) -> IResult<&str, char> {
        satisfy(|c: char| c.is_ascii_alphanumeric() || c == '_')(input)
    }

    fn identifier(input: &str) -> IResult<&str, String> {
        let (input, first) = ident_start(input)?;
        let (input, rest) = many0(ident_cont).parse_complete(input)?;
        let mut s = String::new();
        s.push(first);
        s.extend(rest);
        Ok((input, s))
    }

    fn module_header(input: &str) -> IResult<&str, String> {
        let (input, _) = multispace0(input).map(|(i, _)| (i, ()))?;
        let (input, _) = tag("module")(input).map(|(i, _)| (i, ()))?;
        let (input, _) = multispace1(input).map(|(i, _)| (i, ()))?;
        identifier(input)
    }

    #[test]
    fn nom_can_parse_module_header() {
        let result = module_header("   module foo ");
        assert!(result.is_ok(), "nom smoke test failed: {:?}", result.err());
        assert_eq!(result.unwrap().1, "foo");
    }

    #[test]
    fn nom_reports_errors_for_bad_input() {
        let result = module_header("not a module");
        assert!(result.is_err());
    }
}

mod chumsky_smoke {
    use chumsky::prelude::*;
    use chumsky::text::unicode::{ident, keyword};

    fn module_header<'a>() -> impl Parser<'a, &'a str, String> {
        let ws = one_of(" \t\n\r").repeated();
        keyword("module")
            .then_ignore(ws)
            .then(ident())
            .map(|(_, name): (&str, &str)| name.to_string())
    }

    #[test]
    fn chumsky_can_parse_module_header() {
        let result = module_header().parse("module foo").into_result();
        assert!(
            result.is_ok(),
            "chumsky smoke test failed: {:?}",
            result.err()
        );
        assert_eq!(result.unwrap(), "foo");
    }

    #[test]
    fn chumsky_reports_errors_for_bad_input() {
        let result = module_header().parse("not a module").into_result();
        assert!(result.is_err());
    }
}
