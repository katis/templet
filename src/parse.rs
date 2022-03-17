use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag, take_until, take_until1},
    character::complete::space0,
    combinator::{recognize, rest},
    error::ErrorKind,
    multi::{many0, many0_count, separated_list1},
    sequence::{delimited, pair},
    Err, IResult,
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str>;

type Result<'a, T = Part<'a>> = IResult<Span<'a>, T>;

#[derive(Debug, Eq, PartialEq)]
pub enum Part<'a> {
    Text(&'a str),
    Variable(Vec<Field<'a>>),
    Section(Vec<Field<'a>>, Vec<Part<'a>>),
    InvertedSection(Vec<Field<'a>>, Vec<Part<'a>>),
    Include(&'a str),
    Comment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field<'a> {
    Index(u8),
    Named(&'a str),
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
        parse_include,
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
    let (input, start_field) = start_tag("{{#")(input)?;

    let (input, contents) = parse_parts(input)?;
    let (input, end_field) = tag_end(input)?;

    if start_field != end_field {
        return Err(Err::Error(nom::error::Error::new(input, ErrorKind::Many1)));
    }

    Ok((input, Part::Section(start_field, contents)))
}

fn parse_inverted_section(input: Span) -> Result {
    let (input, start_field) = start_tag("{{^")(input)?;

    let (input, contents) = parse_parts(input)?;
    let (input, end_field) = tag_end(input)?;

    if start_field != end_field {
        return Err(Err::Error(nom::error::Error::new(input, ErrorKind::Many1)));
    }

    Ok((input, Part::InvertedSection(start_field, contents)))
}

fn parse_include(input: Span) -> Result {
    let (input, _) = tag("{{>")(input)?;
    let (input, path) = delimited(space0, file_path, space0)(input)?;
    let (input, _) = tag("}}")(input)?;
    Ok((input, Part::Include(&path)))
}

fn file_path(input: Span) -> Result<Span> {
    delimited(
        tag("\""),
        escaped(is_not("\"\\"), '\\', tag("\"")),
        tag("\""),
    )(input)
}

fn start_tag<'a>(open: &'a str) -> impl Fn(Span<'a>) -> Result<Vec<Field>> + 'a {
    move |input: Span| {
        let (input, _) = tag(open)(input)?;
        let (input, field) = delimited(space0, field, space0)(input)?;
        let (input, _) = tag("}}")(input)?;
        Ok((input, field))
    }
}

fn tag_end<'a>(input: Span<'a>) -> Result<Vec<Field<'a>>> {
    let (input, _) = tag("{{/")(input)?;
    let (input, end_field) = delimited(space0, field, space0)(input)?;
    let (input, _) = tag("}}")(input)?;
    Ok((input, end_field))
}

fn parse_text(s: Span) -> Result {
    let (input, text) = alt((take_until1("\\{{"), take_until("{{"), rest))(s)?;
    if text.is_empty() {
        return Err(Err::Error(nom::error::Error::new(input, ErrorKind::Eof)));
    }
    Ok((input, Part::Text(&text)))
}

fn path(input: Span) -> Result<Vec<Field>> {
    separated_list1(tag("."), alt((named, index)))(input)
}

fn field(input: Span) -> Result<Vec<Field>> {
    alt((path, this))(input)
}

fn index(input: Span) -> Result<Field> {
    let (input, i) = nom::character::complete::u8(input)?;
    Ok((input, Field::Index(i)))
}

fn this(input: Span) -> Result<Vec<Field>> {
    let (input, _) = tag(".")(input)?;
    Ok((input, vec![Field::This]))
}

fn named(input: Span) -> Result<Field> {
    let is_begin = alt((tag("_"), nom_unicode::complete::alpha1));
    let is_rest = many0_count(alt((tag("_"), nom_unicode::complete::alphanumeric1)));

    let (input, ident) = recognize(pair(is_begin, is_rest))(input)?;
    Ok((input, Field::Named(&ident)))
}

#[cfg(test)]
mod tests {
    use super::*;

    use Field::*;
    use Part::*;

    #[test]
    fn simple_variable() {
        let result = parse("<h1>{{head.title}}</h1>");
        assert_eq!(
            result,
            vec![
                Text("<h1>"),
                Variable(vec![Named("head"), Named("title")]),
                Text("</h1>")
            ]
        );
    }

    #[test]
    fn simple_variable_index() {
        let result = parse("<h1>{{ 0 }}</h1>");
        assert_eq!(
            result,
            vec![Text("<h1>"), Variable(vec![Index(0)]), Text("</h1>")]
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
                Text("<ul>"),
                Section(
                    vec![Named("items")],
                    vec![Text("<li>"), Variable(vec![Named("id")]), Text("</li>")]
                ),
                Text("</ul>")
            ]
        );
    }

    #[test]
    fn simple_section_index() {
        let result = parse("{{#0.foo}}{{foobar}}{{/0.foo}}");
        assert_eq!(
            result,
            vec![Section(
                vec![Index(0), Named("foo")],
                vec![Variable(vec![Named("foobar")])]
            )]
        );
    }

    #[test]
    fn simple_include() {
        let result = parse(r#"{{>  "/users/jane doe/templates/index.\"temp\".html" }}"#);
        assert_eq!(
            result,
            vec![Include(r#"/users/jane doe/templates/index.\"temp\".html"#)]
        );
    }
}
