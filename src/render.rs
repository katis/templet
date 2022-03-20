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
            Part::Variable(path) => {
                for value in self.stack.iter().rev() {
                    let mut var = Variable::new(path.as_slice(), writer);
                    value.visit(&mut var);
                    if var.render_result? {
                        return Ok(());
                    }
                }
                Ok(())
            }
            Part::Section(path, parts) => {
                if let Some(last) = self.stack.last() {
                    let mut section = Section::new(path.as_slice(), parts, writer, self.clone());
                    last.visit(&mut section);
                    section.result?;
                }
                Ok(())
            }
            Part::InvertedSection(path, parts) => {
                if let Some(last) = self.stack.last() {
                    let mut section =
                        InvertedSection::new(path.as_slice(), parts, writer, self.clone());
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
    path: &'a [Field<'a>],
    writer: &'a mut W,
    render_result: Result<bool, Error>,
}

impl<'a, W: Write> Variable<'a, W> {
    fn new(path: &'a [Field<'a>], writer: &'a mut W) -> Self {
        Self {
            path,
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

    fn handle_value(&mut self, value: Value) {
        match self.path {
            [_] => {
                self.render_result = self.render_value(value);
            }
            [_, path @ ..] => {
                let mut variable = Variable::new(path, self.writer);
                value.visit(&mut variable);
                self.render_result = variable.render_result;
            }
            _ => {}
        }
    }
}

impl<'a, W: Write> Visit for Variable<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        if let &[Field::This] = self.path {
            self.render_result = self.render_value(value);
            return;
        }

        match value {
            Value::Structable(s) => s.visit(self),
            Value::Mappable(m) => m.visit(self),
            Value::Enumerable(e) if self.path.len() > 1 => {
                if Field::Named(e.variant().name()) == self.path[0] {
                    let mut variable = Variable::new(&self.path[1..], self.writer);
                    e.visit(&mut variable);
                    self.render_result = variable.render_result;
                }
            }
            Value::Enumerable(e) => {
                e.visit(self);
            }
            Value::Tuplable(t) => t.visit(self),
            _ => (),
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if let Field::Named(name) = &self.path[0] {
            if let Some(value) = named_values.get_by_name(&name) {
                self.handle_value(*value);
            }
        }
    }

    fn visit_entry(&mut self, key: Value<'_>, value: Value<'_>) {
        match (&self.path[0], key) {
            (Field::Named(name), Value::String(key)) if *name == key => {
                self.handle_value(value);
            }
            _ => {}
        }
    }

    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if let Field::Index(i) = &self.path[0] {
            if let Some(&value) = values.get(*i as usize) {
                self.handle_value(value);
            }
        }
    }
}

struct Section<'a, W> {
    path: &'a [Field<'a>],
    parts: &'a [Part<'a>],
    writer: &'a mut W,
    renderer: Renderer<'a>,
    result: Result<(), Error>,
}

impl<'a, W: Write> Section<'a, W> {
    fn new(
        path: &'a [Field<'a>],
        parts: &'a [Part],
        writer: &'a mut W,
        renderer: Renderer<'a>,
    ) -> Self {
        Self {
            path,
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

    fn handle_value(&mut self, value: Value<'_>) {
        match self.path {
            [_] => self.render_value(value),
            [_, path @ ..] => {
                let renderer = self.renderer.append(value);
                let mut section = Section::new(path, self.parts, self.writer, renderer);
                value.visit(&mut section);
                self.result = section.result;
            }
            [] => {}
        }
    }
}

impl<'a, W: Write> Visit for Section<'a, W> {
    fn visit_value(&mut self, value: Value<'_>) {
        match (&self.path[0], value) {
            (_, Value::Structable(s)) => s.visit(self),
            (Field::Named(name), Value::Enumerable(e)) if *name == e.variant().name() => {
                let rnd = self.renderer.append(e.as_value());
                self.result = rnd.render_parts(self.writer, self.parts);
            }
            (_, Value::Mappable(m)) => m.visit(self),
            (_, Value::Tuplable(t)) => t.visit(self),
            _ => (),
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        if self.result.is_err() {
            return;
        }

        if let Field::Named(name) = &self.path[0] {
            if let Some(&value) = named_values.get_by_name(&name) {
                self.handle_value(value);
            }
        }
    }

    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if let Field::Index(i) = &self.path[0] {
            if let Some(&value) = values.get(*i as usize) {
                self.handle_value(value);
            }
        }
    }

    fn visit_entry(&mut self, key: Value<'_>, value: Value<'_>) {
        if self.result.is_err() {
            return;
        }

        if let Field::Named(name) = &self.path[0] {
            match key {
                Value::String(s) if s == *name => {
                    self.handle_value(value);
                }
                _ => {}
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
    path: &'a [Field<'a>],
    parts: &'a [Part<'a>],
    writer: &'a mut W,
    renderer: Renderer<'a>,
    result: Result<(), Error>,
}

impl<'a, W: Write> InvertedSection<'a, W> {
    fn new(
        path: &'a [Field<'a>],
        parts: &'a [Part],
        writer: &'a mut W,
        renderer: Renderer<'a>,
    ) -> Self {
        Self {
            path,
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

    fn handle_value(&mut self, value: Option<&Value<'_>>) {
        match (self.path, value) {
            ([_], value) => self.render_parts(value),
            ([_, path @ ..], Some(value)) => {
                let renderer = self.renderer.append(*value);
                let mut section = Section::new(path, self.parts, self.writer, renderer);
                value.visit(&mut section);
                self.result = section.result;
            }
            (_, None) => {}
            _ => unreachable!("path must never be empty"),
        }
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

        if let Field::Named(name) = &self.path[0] {
            let value = named_values.get_by_name(&name);
            self.handle_value(value);
        }
    }

    fn visit_unnamed_fields(&mut self, values: &[Value<'_>]) {
        if self.result.is_err() {
            return;
        }

        if let Field::Index(i) = &self.path[0] {
            let value = values.get(*i as usize);
            self.handle_value(value);
        }
    }
}
