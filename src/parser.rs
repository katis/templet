use std::{cell::RefCell, hash::Hasher, rc::Rc};

use fnv::FnvHasher;
use nom::{
    branch::alt,
    bytes::complete::{is_not, tag, take_till, take_until, take_while},
    combinator::{eof, rest},
    error::ErrorKind,
    multi::many0,
    sequence::delimited,
    Err, IResult,
};
use nom_locate::LocatedSpan;

type Span<'a> = LocatedSpan<&'a str, ()>;

type Result<'a, T = Token<'a>> = IResult<Span<'a>, T>;

#[derive(Debug, Eq, PartialEq)]
pub enum Token<'a> {
    Text(&'a str),
    Variable(&'a str),
}

fn hash(str: &str) -> u64 {
    let mut hasher = FnvHasher::default();
    hasher.write(str.as_bytes());
    hasher.finish()
}

#[derive(Debug)]
pub struct TempletParser {}

impl TempletParser {
    pub fn parse<'s>(s: &'s str) -> Vec<Token<'s>> {
        let span = Span::new(s); // , Rc::new(RefCell::new(self)));
        let tokens = tokenize(span);
        tokens.map(|(_, tokens)| tokens).unwrap()
    }
}

fn tokenize(input: Span) -> Result<Vec<Token>> {
    let (input, mut tokens) = many0(alt((parse_variable, parse_text)))(input)?;
    if input.len() > 0 {
        let (input, s) = rest(input)?;
        tokens.push(Token::Text(&s));
        Ok((input, tokens))
    } else {
        Ok((input, tokens))
    }
}

fn parse_variable(input: Span) -> Result {
    let (input, variable) = delimited(tag("{{"), is_not("}}"), tag("}}"))(input)?;
    dbg!(Ok((input, Token::Variable(variable.trim()))))
}

fn parse_text(s: Span) -> Result {
    let (input, text) = take_until("{{")(s)?;
    dbg!(Ok((input, Token::Text(&text))))
}

#[cfg(test)]
mod tests {
    use super::*;

    use Token::*;

    #[test]
    fn test() {
        let result = TempletParser::parse("<h1>{{title}}</h1>");
        assert_eq!(result, vec![Text("<h1>"), Variable("title"), Text("</h1>")]);
    }
}
