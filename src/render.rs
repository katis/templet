use std::collections::HashMap;
use std::io::Write;

use v_htmlescape::escape;
use valuable::{Valuable, Value, Visit};

use crate::errors::Error;
use crate::parse::{Field, Part};
use crate::Template;

#[derive(Clone)]
pub(crate) struct Renderer<'v> {
    templates: &'v HashMap<String, Template>,
    stack: Vec<Value<'v>>,
}

impl<'v> Renderer<'v> {
    pub fn new(templates: &'v HashMap<String, Template>, initial: Value<'v>) -> Self {
        Self {
            templates,
            stack: vec![initial],
        }
    }

    fn append(&self, value: Value<'v>) -> Renderer {
        let mut rnd = self.clone();
        rnd.stack.push(value);
        rnd
    }

    pub fn render<W: Write>(&self, template_name: &str, writer: &mut W) -> Result<(), Error> {
        if let Some(tpl) = self.templates.get(template_name) {
            self.render_parts(writer, tpl.parts())?;
        }
        Ok(())
    }

    fn render_parts<W: Write>(&self, writer: &mut W, parts: &[Part]) -> Result<(), Error> {
        for part in parts.iter() {
            self.render_part(writer, part)?;
        }
        Ok(())
    }

    fn render_part<W: Write>(&self, writer: &mut W, part: &Part) -> Result<(), Error> {
        match part {
            Part::Text(text) => writer.write_all(text.as_bytes()),
            Part::Variable(name) => {
                for value in self.stack.iter().rev() {
                    let mut var = Variable::new(name.clone(), writer);
                    value.visit(&mut var);
                    if var.render_result? {
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
            Part::InvertedSection(name, parts) => {
                if let Some(last) = self.stack.last() {
                    let mut section =
                        InvertedSection::new(name.clone(), parts, writer, self.clone());
                    last.visit(&mut section);
                    section.result?;
                }
                Ok(())
            }
            Part::Include(path) => {
                if let Some(template) = self.templates.get(*path) {
                    self.render_parts(writer, template.parts())?;
                }
                Ok(())
            }
            Part::Comment => Ok(()),
        }
    }
}

struct Variable<'a, W> {
    field: Field<'a>,
    writer: &'a mut W,
    render_result: Result<bool, Error>,
}

impl<'a, W: Write> Variable<'a, W> {
    fn new(field: Field<'a>, writer: &'a mut W) -> Self {
        Self {
            field,
            writer,
            render_result: Ok(false),
        }
    }

    fn render_value(&mut self, value: Value) -> Result<bool, Error> {
        let mut buf = Vec::new();

        let result = match value {
            Value::String(v) => {
                let esc = escape(v);
                write!(self.writer, "{}", esc)?;
                return Ok(true);
            }
            Value::Char(v) => write!(&mut buf, "{}", v),
            Value::Bool(v) => write!(&mut buf, "{}", v),
            Value::F32(v) => write!(&mut buf, "{}", v),
            Value::F64(v) => write!(&mut buf, "{}", v),
            Value::I8(v) => write!(&mut buf, "{}", v),
            Value::I16(v) => write!(&mut buf, "{}", v),
            Value::I32(v) => write!(&mut buf, "{}", v),
            Value::I64(v) => write!(&mut buf, "{}", v),
            Value::I128(v) => write!(&mut buf, "{}", v),
            Value::Isize(v) => write!(&mut buf, "{}", v),
            Value::U8(v) => write!(&mut buf, "{}", v),
            Value::U16(v) => write!(&mut buf, "{}", v),
            Value::U32(v) => write!(&mut buf, "{}", v),
            Value::U64(v) => write!(&mut buf, "{}", v),
            Value::U128(v) => write!(&mut buf, "{}", v),
            Value::Usize(v) => write!(&mut buf, "{}", v),
            Value::Path(v) => write!(&mut buf, "{}", v.display()),
            _ => return Ok(false),
        };
        if !buf.is_empty() {
            if let Ok(str) = String::from_utf8(buf) {
                let esc = escape(&str);
                write!(self.writer, "{}", esc)?;
            }
        }
        result.map(|_| true)
    }
}

impl<'a, W: Write> Visit for Variable<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        if self.field == Field::This {
            self.render_result = self.render_value(value);
            return;
        }

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
        if let Field::Named(name) = &self.field {
            if let Some(value) = named_values.get_by_name(&name) {
                self.render_result = self.render_value(*value);
            }
        }
    }

    fn visit_entry(&mut self, key: Value<'_>, value: Value<'_>) {
        match (&self.field, key) {
            (Field::Named(name), Value::String(key)) if *name == key => {
                self.render_result = self.render_value(value);
            }
            _ => {}
        }
    }

    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if let Field::Index(i) = &self.field {
            if let Some(&value) = values.get(*i as usize) {
                self.render_result = self.render_value(value);
            }
        }
    }
}

struct Section<'a, W> {
    field: Field<'a>,
    parts: &'a [Part<'a>],
    writer: &'a mut W,
    renderer: Renderer<'a>,
    result: Result<(), Error>,
}

impl<'a, W: Write> Section<'a, W> {
    fn new(field: Field<'a>, parts: &'a [Part], writer: &'a mut W, renderer: Renderer<'a>) -> Self {
        Self {
            field,
            parts,
            writer,
            renderer,
            result: Ok(()),
        }
    }

    fn render_value(&mut self, value: Value<'_>) {
        match value {
            Value::Listable(l) => {
                let mut list = ListSection::new(self.parts, self.writer, self.renderer.clone());
                l.visit(&mut list);
                self.result = list.result;
            }
            Value::Bool(true) => {
                self.result = self.renderer.render_parts(self.writer, self.parts);
            }
            Value::Bool(false) | Value::Unit => {}
            _ => {
                let rnd = self.renderer.append(value);
                self.result = rnd.render_parts(self.writer, self.parts);
            }
        }
    }
}

impl<'a, W: Write> Visit for Section<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        match (&self.field, value) {
            (_, Value::Structable(s)) => s.visit(self),
            (Field::Named(name), Value::Enumerable(e)) if *name == e.variant().name() => {
                let rnd = self.renderer.append(e.as_value());
                self.result = rnd.render_parts(self.writer, self.parts);
            }
            _ => (),
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if self.result.is_err() {
            return;
        }

        if let Field::Named(name) = &self.field {
            if let Some(&value) = named_values.get_by_name(&name) {
                self.render_value(value);
            }
        }
    }

    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if let Field::Index(i) = &self.field {
            if let Some(&value) = values.get(*i as usize) {
                self.render_value(value);
            }
        }
    }
}

struct ListSection<'a, W> {
    parts: &'a [Part<'a>],
    writer: &'a mut W,
    renderer: Renderer<'a>,
    result: Result<(), Error>,
}

impl<'a, W> ListSection<'a, W> {
    fn new(parts: &'a [Part], writer: &'a mut W, renderer: Renderer<'a>) -> Self {
        Self {
            parts,
            writer,
            renderer,
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

        let rnd = self.renderer.append(value);
        self.result = rnd.render_parts(self.writer, self.parts);
    }
}

struct InvertedSection<'a, W> {
    field: Field<'a>,
    parts: &'a [Part<'a>],
    writer: &'a mut W,
    renderer: Renderer<'a>,
    result: Result<(), Error>,
}

impl<'a, W: Write> InvertedSection<'a, W> {
    fn new(field: Field<'a>, parts: &'a [Part], writer: &'a mut W, renderer: Renderer<'a>) -> Self {
        Self {
            field,
            parts,
            writer,
            renderer,
            result: Ok(()),
        }
    }

    fn render_parts(&mut self, value: Option<&Value<'_>>) {
        self.result = match value {
            None | Some(Value::Bool(false) | Value::Unit) => {
                self.renderer.render_parts(self.writer, self.parts)
            }
            Some(Value::Listable(l)) if (0, Some(0)) == l.size_hint() => {
                self.renderer.render_parts(self.writer, self.parts)
            }
            Some(Value::Mappable(m)) if (0, Some(0)) == m.size_hint() => {
                self.renderer.render_parts(self.writer, self.parts)
            }
            _ => return,
        };
    }
}

impl<'a, W: Write> Visit for InvertedSection<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        if let Value::Structable(s) = value {
            s.visit(self)
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if self.result.is_err() {
            return;
        }

        if let Field::Named(name) = &self.field {
            let field = named_values.get_by_name(&name);
            self.render_parts(field);
        }
    }

    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if self.result.is_err() {
            return;
        }

        if let Field::Index(i) = &self.field {
            let field = values.get(*i as usize);
            self.render_parts(field);
        }
    }
}
