use std::borrow::Cow;

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

type Span<'a> = LocatedSpan<&'a str, ()>;

type Result<'a, T = Part<'a>> = IResult<Span<'a>, T>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Part<'a> {
    Text(Cow<'a, str>),
    Variable(Cow<'a, str>),
    Section(Cow<'a, str>, Vec<Part<'a>>),
    Comment,
}

impl<'a> Part<'a> {
    pub fn into_owned(self) -> Part<'static> {
        match self {
            Part::Text(text) => Part::Text(Cow::Owned(text.into_owned())),
            Part::Variable(var) => Part::Variable(Cow::Owned(var.into_owned())),
            Part::Section(name, tokens) => Part::Section(
                Cow::Owned(name.into_owned()),
                tokens.into_iter().map(|t| t.into_owned()).collect(),
            ),
            Part::Comment => Part::Comment,
        }
    }
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
    Ok((input, Part::Variable(Cow::Borrowed(variable.trim()))))
}

fn parse_section(input: Span) -> Result {
    let (input, name) = start_tag("{{#")(input)?;
    let (input, contents) = dbg!(parse_parts(input))?;
    let (input, _) = tag_end(name)(input)?;

    Ok((input, Part::Section(Cow::Borrowed(name), contents)))
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
    Ok((input, Part::Text(Cow::Borrowed(&text))))
}

#[cfg(test)]
mod tests {
    use super::*;

    use std::borrow::Cow::*;

    use Part::*;

    #[test]
    fn simple_variable() {
        let result = TempletParser::parse("<h1>{{title}}</h1>");
        assert_eq!(
            result,
            vec![
                Text(Borrowed("<h1>")),
                Variable(Borrowed("title")),
                Text(Borrowed("</h1>"))
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
                Text(Borrowed("<ul>")),
                Section(
                    Cow::Borrowed("items"),
                    vec![
                        Text(Borrowed("<li>")),
                        Variable(Borrowed("id")),
                        Text(Borrowed("</li>"))
                    ]
                ),
                Text(Borrowed("</ul>"))
            ]
        );
    }
}
