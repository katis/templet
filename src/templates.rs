use std::{collections::HashMap, io::Write};

use valuable::Valuable;

use crate::{errors::Error, render::Renderer, Template};

pub struct Templates {
    templates: HashMap<String, Template>,
}

impl Templates {
    pub fn new(templates: HashMap<String, Template>) -> Self {
        Self { templates }
    }

    pub fn render<W: Write>(
        &self,
        name: &str,
        writer: &mut W,
        data: &dyn Valuable,
    ) -> Result<(), Error> {
        let renderer = Renderer::new(&self.templates, data.as_value());
        renderer.render(name, writer)
    }

    pub fn render_to_string(&self, name: &str, data: &dyn Valuable) -> Result<String, Error> {
        let renderer = Renderer::new(&self.templates, data.as_value());
        let mut buf = Vec::new();
        renderer.render(name, &mut buf)?;
        Ok(String::from_utf8(buf).unwrap())
    }
}
