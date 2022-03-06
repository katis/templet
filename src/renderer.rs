use std::fmt::Write;

use valuable::{Valuable, Value, Visit};

use self::Segment::*;
use crate::parser::Part;

enum Segment<'a> {
    Field(&'a str),
    Idx(usize),
}

pub struct Renderer<'a, W> {
    writer: &'a mut W,
    ctx: Vec<Segment<'a>>,
}

impl<'a, W: Write> Renderer<'a, W> {
    pub fn new(writer: &'a mut W) -> Self {
        Self {
            writer,
            ctx: vec![],
        }
    }

    pub fn render(&mut self, tokens: &'a [Part<'a>], valuable: &'a dyn Valuable) {
        for token in tokens.iter() {
            match token {
                Part::Text(text) => self.writer.write_str(text).unwrap(),
                Part::Variable(name) => self.render_variable(name, valuable.as_value()),
                Part::Section(name, tokens) => {
                    self.ctx.push(Field(name));
                    self.render(tokens, valuable);
                    self.ctx.pop();
                }
                Part::Comment => {}
            }
        }
    }

    fn render_variable(&mut self, name: &'a str, root: Value<'a>) {
        let mut var = Variable::new(name, &self.ctx, self.writer);
        root.visit(&mut var);
    }
}

struct Variable<'a, W> {
    name: &'a str,
    path: &'a [Segment<'a>],
    writer: &'a mut W,
    result: Result<bool, Error>,
}

impl<'a, W: Write> Variable<'a, W> {
    fn new(name: &'a str, path: &'a [Segment<'a>], writer: &'a mut W) -> Self {
        Self {
            name,
            path,
            writer,
            result: Ok(false),
        }
    }

    fn render_value(&mut self, value: &valuable::Value<'_>) {
        let result = match value {
            Value::String(v) => self.writer.write_str(v),
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
            Value::U8(v) => write!(self.writer, "{}", v),
            Value::U16(v) => write!(self.writer, "{}", v),
            Value::U32(v) => write!(self.writer, "{}", v),
            Value::U64(v) => write!(self.writer, "{}", v),
            Value::U128(v) => write!(self.writer, "{}", v),
            Value::Usize(v) => write!(self.writer, "{}", v),
            Value::Path(v) => write!(self.writer, "{}", v.display()),
            _ => return,
        };
        self.result = result.map(|_| true).map_err(Error::Fmt);
    }
}

impl<'a, W: Write> Visit for Variable<'a, W> {
    fn visit_value(&mut self, value: valuable::Value<'_>) {
        if let Value::Structable(s) = value {
            s.visit(self);
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if !self.path.is_empty() {
            if let Field(name) = self.path[0] {
                if let Some(value) = named_values.get_by_name(name) {
                    let mut var = Variable::new(self.name, &self.path[1..], self.writer);
                    value.visit(&mut var);
                    self.result = var.result;
                }
            }
        }

        if let Ok(false) = self.result {
            if let Some(value) = named_values.get_by_name(self.name) {
                self.render_value(value);
            }
        }
    }
}

pub enum Error {
    Fmt(std::fmt::Error),
}
