use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_until, take_until1},
    character::complete::space0,
    combinator::{recognize, rest},
    error::ErrorKind,
    multi::{many0, many0_count},
    sequence::{delimited, pair},
    Err, IResult,
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;

type Result<'a, T = Part> = IResult<Span<'a>, T>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum Part {
    Text(String),
    Variable(Field),
    Section(Field, Vec<Part>),
    InvertedSection(Field, Vec<Part>),
    Comment,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Field {
    Index(u8),
    Named(String),
    This,
}

pub fn parse(s: &str) -> Vec<Part> {
    let span = Span::new(s);
    let tokens = parse_parts(span);
    tokens.map(|(_, tokens)| tokens).unwrap()
}

fn parse_parts(input: Span) -> Result<Vec<Part>> {
    let (input, tokens) = many0(alt((
        parse_comment,
        parse_section,
        parse_inverted_section,
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
    let (input, _) = tag("{{")(input)?;
    let (input, field) = delimited(space0, field, space0)(input)?;
    let (input, _) = tag("}}")(input)?;
    Ok((input, Part::Variable(field)))
}

fn parse_section(input: Span) -> Result {
    let (input, field) = start_tag("{{#")(input)?;

    let (input, contents) = parse_parts(input)?;
    let (input, _) = tag_end(field.clone())(input)?;

    Ok((input, Part::Section(field, contents)))
}

fn parse_inverted_section(input: Span) -> Result {
    let (input, field) = start_tag("{{^")(input)?;

    let (input, contents) = parse_parts(input)?;
    let (input, _) = tag_end(field.clone())(input)?;

    Ok((input, Part::InvertedSection(field, contents)))
}

fn start_tag<'a>(open: &'a str) -> impl Fn(Span<'a>) -> Result<Field> + 'a {
    move |input: Span| {
        let (input, _) = tag(open)(input)?;
        let (input, field) = delimited(space0, field, space0)(input)?;
        let (input, _) = tag("}}")(input)?;
        Ok((input, field))
    }
}

fn tag_end<'a>(start_field: Field) -> impl Fn(Span<'a>) -> Result<()> + 'a {
    move |input: Span| {
        let (input, _) = tag("{{/")(input)?;
        let (input, end_field) = delimited(space0, field, space0)(input)?;
        if start_field != end_field {
            return Err(Err::Error(nom::error::Error::new(input, ErrorKind::Many1)));
        }
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

fn field(input: Span) -> Result<Field> {
    alt((named, index, this))(input)
}

fn index(input: Span) -> Result<Field> {
    let (input, i) = nom::character::complete::u8(input)?;
    Ok((input, Field::Index(i)))
}

fn this(input: Span) -> Result<Field> {
    let (input, _) = tag(".")(input)?;
    Ok((input, Field::This))
}

fn named(input: Span) -> Result<Field> {
    let is_begin = alt((tag("_"), nom_unicode::complete::alpha1));
    let is_rest = many0_count(alt((tag("_"), nom_unicode::complete::alphanumeric1)));

    let (input, ident) = recognize(pair(is_begin, is_rest))(input)?;
    Ok((input, Field::Named(ident.to_string())))
}

#[cfg(test)]
mod tests {
    use super::*;

    use Field::*;
    use Part::*;

    #[test]
    fn simple_variable() {
        let result = parse("<h1>{{title}}</h1>");
        assert_eq!(
            result,
            vec![
                Text("<h1>".into()),
                Variable(Named("title".into())),
                Text("</h1>".into())
            ]
        );
    }

    #[test]
    fn simple_variable_index() {
        let result = parse("<h1>{{ 0 }}</h1>");
        assert_eq!(
            result,
            vec![
                Text("<h1>".into()),
                Variable(Index(0)),
                Text("</h1>".into())
            ]
        );
    }

    #[test]
    fn text_wtf() {
        let result = parse("{{/foobar}}");
        assert_eq!(result, vec![]);
    }

    #[test]
    fn simple_section() {
        let result = parse("\\{{ Cool shit here }}<ul>{{#items}}<li>{{id}}</li>{{/items}}</ul>");
        assert_eq!(
            result,
            vec![
                Comment,
                Text("<ul>".into()),
                Section(
                    Named("items".into()),
                    vec![
                        Text("<li>".into()),
                        Variable(Named("id".into())),
                        Text("</li>".into())
                    ]
                ),
                Text("</ul>".into())
            ]
        );
    }

    #[test]
    fn simple_section_index() {
        let result = parse("{{#0}}{{foobar}}{{/0}}");
        assert_eq!(
            result,
            vec![Section(Index(0), vec![Variable(Named("foobar".into()))])]
        );
    }
}