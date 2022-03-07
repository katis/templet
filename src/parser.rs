use std::{cell::RefCell, rc::Rc};

use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until, take_until1},
    combinator::{not, rest},
    error::ErrorKind,
    multi::many0,
    sequence::delimited,
    Err, IResult, Parser,
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;

type Result<'a, T = Part> = IResult<Span<'a>, T>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Part {
    Text(String),
    Variable(String),
    Section(String, Vec<Part>),
    Comment,
}

#[derive(Debug)]
pub struct TempletParser {}

impl TempletParser {
    pub fn parse(s: &str) -> Vec<Part> {
        let span = Span::new(s);
        let tokens = parse_parts(span);
        tokens.map(|(_, tokens)| tokens).unwrap()
    }
}

fn parse_parts(input: Span) -> Result<Vec<Part>> {
    let (input, tokens) = many0(alt((
        parse_comment,
        parse_section,
        parse_variable,
        parse_text,
    )))(input)?;
    Ok((input, tokens))
}

fn parse_comment(input: Span) -> Result {
    let (input, _) = delimited(tag("\\{{"), is_not("}}"), tag("}}"))(input)?;
    Ok((input, Part::Comment))
}

fn parse_variable(input: Span) -> Result {
    let (input, variable) =
        delimited(not(tag("{{/")).and(tag("{{")), is_not("}}"), tag("}}"))(input)?;
    Ok((input, Part::Variable(variable.trim().to_owned())))
}

fn parse_section(input: Span) -> Result {
    let (input, name) = start_tag("{{#")(input)?;

    let (input, contents) = parse_parts(input)?;
    let (input, _) = tag_end(name)(input)?;

    Ok((input, Part::Section(name.to_owned(), contents)))
}

fn start_tag<'a>(open: &'a str) -> impl Fn(Span<'a>) -> Result<&'a str> + 'a {
    move |input: Span| {
        let (input, _) = tag(open)(input)?;
        let (input, s) = take_until("}}")(input)?;
        let (input, _) = tag("}}")(input)?;
        Ok((input, s.trim()))
    }
}

fn tag_end<'a>(name: &'a str) -> impl Fn(Span<'a>) -> Result<()> + 'a {
    move |input: Span| {
        let (input, _) = tag("{{/")(input)?;
        let (input, _) = tag(name)(input)?;
        let (input, _) = tag("}}")(input)?;
        Ok((input, ()))
    }
}

fn parse_text(s: Span) -> Result {
    let (input, text) = alt((take_until1("\\{{"), take_until("{{"), rest))(s)?;
    if text.is_empty() {
        return Err(Err::Error(nom::error::Error::new(input, ErrorKind::Eof)));
    }
    Ok((input, Part::Text(text.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    use Part::*;

    #[test]
    fn simple_variable() {
        let result = TempletParser::parse("<h1>{{title}}</h1>");
        assert_eq!(
            result,
            vec![
                Text("<h1>".into()),
                Variable("title".into()),
                Text("</h1>".into())
            ]
        );
    }

    #[test]
    fn text_wtf() {
        let result = TempletParser::parse("{{/foobar}}");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn simple_section() {
        let result = TempletParser::parse(
            "\\{{ Cool shit here }}<ul>{{#items}}<li>{{id}}</li>{{/items}}</ul>",
        );
        assert_eq!(
            result,
            vec![
                Comment,
                Text("<ul>".into()),
                Section(
                    "items".into(),
                    vec![
                        Text("<li>".into()),
                        Variable("id".into()),
                        Text("</li>".into())
                    ]
                ),
                Text("</ul>".into())
            ]
        );
    }
}
