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

#[cfg(test)]
mod tests {
    use super::*;

    use valuable::Valuable;

    #[derive(Valuable)]
    struct Page<'a> {
        title: &'a str,
        items: &'a [Item<'a>],
    }

    #[derive(Valuable)]
    struct Item<'a> {
        name: &'a str,
    }

    #[test]
    fn partials() {
        let mut map = HashMap::new();
        map.insert(
            "main".to_string(),
            Template::parse(r#"{{>"header"}}<ul>{{#items}}{{> "item"}}{{/items}}</ul>"#),
        );
        map.insert(
            "header".to_string(),
            Template::parse(r#"<h1>{{title}}</h1>"#),
        );
        map.insert("item".to_string(), Template::parse(r#"<li>{{name}}</li>"#));

        let templates = Templates::new(map);
        let str = templates
            .render_to_string(
                "main",
                &Page {
                    title: "Products",
                    items: &[Item { name: "Bread" }, Item { name: "Milk" }],
                },
            )
            .unwrap();
        assert_eq!(
            &str,
            "<h1>Products</h1><ul><li>Bread</li><li>Milk</li></ul>"
        );
    }
}
