use nom::{
    branch::alt,
    bytes::complete::{escaped, is_not, tag, take_until, take_until1},
    character::complete::space0,
    combinator::{recognize, rest},
    error::ErrorKind,
    multi::{fold_many1, many0, many0_count},
    sequence::{delimited, pair},
    Err, IResult,
};
use nom_locate::LocatedSpan;
use nom_unicode::complete::{alpha1, alphanumeric1, upper1};

type Span<'a> = LocatedSpan<&'a str>;

type Result<'a, T = Part<'a>> = IResult<Span<'a>, T>;

#[derive(Debug, Eq, PartialEq)]
pub enum Part<'a> {
    Text(&'a str),
    Variable(Access<'a>),
    Section(Access<'a>, Vec<Part<'a>>),
    InvertedSection(Access<'a>, Vec<Part<'a>>),
    Include(&'a str),
    Comment,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Access<'a> {
    Variant(&'a str),
    Path(Vec<Field<'a>>),
    This,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Field<'a> {
    Index(usize),
    Nth(usize),
    Named(&'a str),
}

#[derive(Debug)]
pub enum PathPart<'a> {
    Index(usize),
    Nth(usize),
    Named(&'a str),
    Dot,
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
    let (input, field) = delimited(space0, access, space0)(input)?;
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

fn start_tag<'a>(open: &'a str) -> impl Fn(Span<'a>) -> Result<Access<'a>> + 'a {
    move |input: Span| {
        let (input, _) = tag(open)(input)?;
        let (input, tag_access) = delimited(space0, access, space0)(input)?;
        let (input, _) = tag("}}")(input)?;
        Ok((input, tag_access))
    }
}

fn tag_end(input: Span) -> Result<Access> {
    let (input, _) = tag("{{/")(input)?;
    let (input, tag_access) = delimited(space0, access, space0)(input)?;
    let (input, _) = tag("}}")(input)?;
    Ok((input, tag_access))
}

fn parse_text(s: Span) -> Result {
    let (input, text) = alt((take_until1("\\{{"), take_until("{{"), rest))(s)?;
    if text.is_empty() {
        return Err(Err::Error(nom::error::Error::new(input, ErrorKind::Eof)));
    }
    Ok((input, Part::Text(&text)))
}

fn access(input: Span) -> Result<Access> {
    alt((access_this, access_variant, access_path))(input)
}

fn access_this(input: Span) -> Result<Access> {
    let (input, _) = tag(".")(input)?;
    Ok((input, Access::This))
}

fn access_variant(input: Span) -> Result<Access> {
    let (input, name) = recognize(pair(upper1, alphanumeric1))(input)?;
    Ok((input, Access::Variant(&name)))
}

fn access_path(input: Span) -> Result<Access> {
    let (input, fields) = fold_many1(path_part, Vec::new, |mut acc, part| {
        match part {
            PathPart::Index(i) => acc.push(Field::Index(i)),
            PathPart::Nth(i) => acc.push(Field::Nth(i)),
            PathPart::Named(n) => acc.push(Field::Named(n)),
            PathPart::Dot => {}
        };
        acc
    })(input)?;
    Ok((input, Access::Path(fields)))
}

fn path_part(input: Span) -> Result<PathPart> {
    alt((field_dot, field_index, field_nth, field_identifier))(input)
}

fn field_dot(input: Span) -> Result<PathPart> {
    let (input, _) = tag(".")(input)?;
    Ok((input, PathPart::Dot))
}

fn field_index(input: Span) -> Result<PathPart> {
    let (input, number) = delimited(tag("["), nom::character::complete::u128, tag("]"))(input)?;
    Ok((input, PathPart::Index(number as usize)))
}

fn field_nth(input: Span) -> Result<PathPart> {
    let (input, number) = nom::character::complete::u128(input)?;
    Ok((input, PathPart::Nth(number as usize)))
}

fn field_identifier(input: Span) -> Result<PathPart> {
    let (input, name) = recognize(pair(
        alt((alpha1, tag("_"))),
        many0_count(alt((alphanumeric1, tag("_")))),
    ))(input)?;
    Ok((input, PathPart::Named(&name)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn variable() {
        use Field::*;

        let this_var = parse("{{ . }}");
        assert_eq!(this_var, vec![Part::Variable(Access::This)]);

        let path_var = parse("{{ foo[12].1 }}");
        assert_eq!(
            path_var,
            vec![Part::Variable(Access::Path(vec![
                Named("foo"),
                Index(12),
                Nth(1)
            ]))]
        );
    }

    #[test]
    fn access_path() {
        use Field::*;

        let (_, path) = access(LocatedSpan::new("foo.0.bar.1[12]")).unwrap();
        assert_eq!(
            path,
            Access::Path(vec![Named("foo"), Nth(0), Named("bar"), Nth(1), Index(12)])
        );
    }

    #[test]
    fn access_variant() {
        let (_, variant) = access(LocatedSpan::new("FooBar")).unwrap();
        assert_eq!(variant, Access::Variant("FooBar"));
    }

    #[test]
    fn access_this() {
        let (_, variant) = access(LocatedSpan::new(".")).unwrap();
        assert_eq!(variant, Access::This);
    }
}
