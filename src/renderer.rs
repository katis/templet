use std::fmt::Write;

use valuable::{Valuable, Value, Visit};

use crate::parser::Token;

pub struct Renderer<'a> {
    tokens: Vec<Token<'a>>,
}

impl<'a> Renderer<'a> {
    pub fn new(tokens: Vec<Token<'a>>) -> Self {
        Self { tokens }
    }

    pub fn render<W: Write>(&self, writer: &mut W, ctx: &dyn Valuable) {
        for token in self.tokens.iter() {
            match token {
                Token::Text(text) => writer.write_str(text).unwrap(),
                Token::Variable(name) => self.render_variable(writer, ctx, name),
            }
        }
    }

    fn render_variable<W: Write>(&self, writer: &mut W, ctx: &dyn Valuable, name: &str) {
        let mut variable = Variable::new(name, writer);
        ctx.visit(&mut variable);
    }
}

struct Variable<'a, W> {
    name: &'a str,
    writer: &'a mut W,
    result: Result<(), std::fmt::Error>,
}

impl<'a, W> Variable<'a, W> {
    fn new(name: &'a str, writer: &'a mut W) -> Self {
        Self {
            name,
            writer,
            result: Ok(()),
        }
    }
}

impl<'a, W: Write> Visit for Variable<'a, W> {
    fn visit_value(&mut self, value: valuable::Value<'_>) {
        if let Value::Structable(st) = value {
            if let valuable::StructDef::Static { .. } = st.definition() {
                st.visit(self)
            }
        };
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if let Some(value) = named_values.get_by_name(self.name) {
            self.result = match value {
                Value::Bool(v) => write!(self.writer, "{}", v),
                Value::Char(v) => write!(self.writer, "{}", v),
                Value::F32(v) => write!(self.writer, "{}", v),
                Value::F64(v) => write!(self.writer, "{}", v),
                Value::I8(v) => write!(self.writer, "{}", v),
                Value::I16(v) => write!(self.writer, "{}", v),
                Value::I32(v) => write!(self.writer, "{}", v),
                Value::I64(v) => write!(self.writer, "{}", v),
                Value::I128(v) => write!(self.writer, "{}", v),
                Value::Isize(v) => write!(self.writer, "{}", v),
                Value::String(v) => write!(self.writer, "{}", v),
                Value::U8(v) => write!(self.writer, "{}", v),
                Value::U16(v) => write!(self.writer, "{}", v),
                Value::U32(v) => write!(self.writer, "{}", v),
                Value::U64(v) => write!(self.writer, "{}", v),
                Value::U128(v) => write!(self.writer, "{}", v),
                Value::Usize(v) => write!(self.writer, "{}", v),
                Value::Path(v) => write!(self.writer, "{}", v.display()),
                _ => return,
            };
        }
    }
}

#[cfg(test)]
mod tests {
    use crate::parser::TempletParser;

    use super::*;

    #[derive(Valuable)]
    struct Titled<'a> {
        title: &'a str,
    }

    #[test]
    fn test() {
        let tokens = TempletParser::parse(r#"<h1>{{title}}</h1>"#);
        let mut s = String::new();
        let renderer = Renderer::new(tokens);
        renderer.render(
            &mut s,
            &Titled {
                title: "Hello, world!",
            },
        );
        assert_eq!(s.trim(), "<h1>Hello, world!</h1>");
    }
}
