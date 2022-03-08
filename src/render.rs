use std::fmt::Write;

use valuable::{Valuable, Value, Visit};

use crate::parser::Part;

pub fn render<W: Write>(
    writer: &mut W,
    parts: &[Part],
    value: Value,
) -> Result<(), std::fmt::Error> {
    let ctx = Context::new(value);
    ctx.render_parts(writer, parts)?;
    Ok(())
}

#[derive(Debug, Clone)]
struct Context<'v> {
    stack: Vec<Value<'v>>,
}

impl<'v> Context<'v> {
    fn new(initial: Value<'v>) -> Self {
        Self {
            stack: vec![initial],
        }
    }

    fn append(&self, value: Value<'v>) -> Context {
        let mut ctx = self.clone();
        ctx.stack.push(value);
        ctx
    }

    fn render_parts<W: Write>(
        &self,
        writer: &mut W,
        parts: &[Part],
    ) -> Result<(), std::fmt::Error> {
        for part in parts.iter() {
            self.render_part(writer, part)?;
        }
        Ok(())
    }

    fn render_part<W: Write>(&self, writer: &mut W, part: &Part) -> Result<(), std::fmt::Error> {
        match part {
            Part::Variable(name) => {
                for value in self.stack.iter().rev() {
                    let mut var = Variable::new(name.clone(), writer);
                    value.visit(&mut var);
                    if var.render_result()? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Part::Section(name, parts) => {
                if let Some(last) = self.stack.last() {
                    let mut section = Section::new(name.clone(), parts, writer, self.clone());
                    last.visit(&mut section);
                    section.result?;
                }
                Ok(())
            }
            Part::Text(text) => writer.write_str(text.as_str()),
            Part::Comment => Ok(()),
        }
    }
}

struct Variable<'a, W> {
    name: String,
    writer: &'a mut W,
    render_result: Result<bool, std::fmt::Error>,
}

impl<'a, W: Write> Variable<'a, W> {
    fn new(name: String, writer: &'a mut W) -> Self {
        Self {
            name,
            writer,
            render_result: Ok(false),
        }
    }

    fn render_result(&self) -> Result<bool, std::fmt::Error> {
        self.render_result
    }

    fn render_value(&mut self, value: Value) -> Result<bool, std::fmt::Error> {
        let result = match value {
            Value::String(v) => self.writer.write_str(v),
            Value::Char(v) => self.writer.write_char(v),
            Value::Bool(v) => write!(self.writer, "{}", v),
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
            _ => return Ok(false),
        };
        result.map(|_| true)
    }
}

impl<'a, W: Write> Visit for Variable<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        match value {
            Value::Structable(s) => {
                s.visit(self);
            }
            Value::Mappable(m) => {
                m.visit(self);
            }
            Value::Enumerable(e) => {
                e.visit(self);
            }
            _ => (),
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if let Some(value) = named_values.get_by_name(&self.name) {
            self.render_result = self.render_value(*value);
        }
    }

    fn visit_entry(&mut self, key: Value<'_>, value: Value<'_>) {
        if let Value::String(key) = key {
            if key == self.name {
                self.render_result = self.render_value(value);
            }
        }
    }
}

struct Section<'a, W> {
    name: String,
    parts: &'a [Part],
    writer: &'a mut W,
    context: Context<'a>,
    result: Result<(), std::fmt::Error>,
}

impl<'a, W> Section<'a, W> {
    fn new(name: String, parts: &'a [Part], writer: &'a mut W, context: Context<'a>) -> Self {
        Self {
            name,
            parts,
            writer,
            context,
            result: Ok(()),
        }
    }
}

impl<'a, W: Write> Visit for Section<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        match value {
            Value::Structable(s) => s.visit(self),
            Value::Enumerable(e) => {
                if e.variant().name() == self.name {
                    let ctx = self.context.append(e.as_value());
                    self.result = ctx.render_parts(self.writer, self.parts);
                }
            }
            _ => (),
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if self.result.is_err() {
            return;
        }
        if let Some(&value) = named_values.get_by_name(&self.name) {
            match value {
                Value::Listable(l) => {
                    let mut list = ListSection::new(self.parts, self.writer, self.context.clone());
                    l.visit(&mut list);
                    self.result = list.result;
                }
                Value::Bool(true) => {
                    self.result = self.context.render_parts(self.writer, self.parts);
                }
                Value::Bool(false) | Value::Unit => {}
                _ => {
                    let ctx = self.context.append(value);
                    self.result = ctx.render_parts(self.writer, self.parts);
                }
            }
        }
    }
}

struct ListSection<'a, W> {
    parts: &'a [Part],
    writer: &'a mut W,
    context: Context<'a>,
    result: Result<(), std::fmt::Error>,
}

impl<'a, W> ListSection<'a, W> {
    fn new(parts: &'a [Part], writer: &'a mut W, context: Context<'a>) -> Self {
        Self {
            parts,
            writer,
            context,
            result: Ok(()),
        }
    }
}

impl<'a, W: Write> Visit for ListSection<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        if self.result.is_err() {
            return;
        } else if let Value::Unit = value {
            return;
        }

        let ctx = self.context.append(value);
        self.result = ctx.render_parts(self.writer, self.parts);
    }
}
