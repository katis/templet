use std::{collections::HashMap, fs, io::Write, path::Path};

use thiserror::Error;
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

    pub fn load_dir(
        dir_path: impl AsRef<Path>,
        extensions: &[&str],
    ) -> Result<Templates, TemplateLoadError> {
        let dir_path: &Path = dir_path.as_ref();

        let mut templates: HashMap<String, Template> = HashMap::new();
        for entry in walkdir::WalkDir::new(dir_path) {
            let entry = entry?;
            if entry.file_type().is_file()
                && entry
                    .path()
                    .extension()
                    .map(|ext| extensions.iter().any(|s| ext == *s))
                    .unwrap_or(false)
            {
                let file = fs::read_to_string(entry.path())?;
                let path = entry.path().strip_prefix(dir_path)?.to_string_lossy();
                let template = Template::parse(&file);
                templates.insert(path.into_owned(), template);
            }
        }
        Ok(Templates::new(templates))
    }
}

#[derive(Debug, Error)]
pub enum TemplateLoadError {
    #[error("failed to load template files")]
    Io(#[from] std::io::Error),
    #[error("could not strip path prefix")]
    StripPrefix(#[from] std::path::StripPrefixError),
    #[error("could not walk template directory")]
    WalkDir(#[from] walkdir::Error),
}

#[cfg(test)]
mod tests {
    use super::*;

    use valuable::Valuable;

    #[derive(Valuable)]
    struct Page<'a> {
        title: &'a str,
        css: &'a [&'a str],
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
                    css: &[],
                    items: &[Item { name: "Bread" }, Item { name: "Milk" }],
                },
            )
            .unwrap();
        assert_eq!(
            &str,
            "<h1>Products</h1><ul><li>Bread</li><li>Milk</li></ul>"
        );
    }

    #[test]
    fn load_dir() {
        let templates = Templates::load_dir("./templates", &["html"]).unwrap();
        let str = templates
            .render_to_string(
                "index.html",
                &Page {
                    title: "Products",
                    css: &["/index.css", "/main.css"],
                    items: &[Item { name: "Bread" }, Item { name: "Milk" }],
                },
            )
            .unwrap();
        println!("{}", &str);

        let expected = r#"
<!DOCTYPE html>
<html>
  <head>
  
  <link rel="stylesheet" href="&#x2f;index.css" />
  
  <link rel="stylesheet" href="&#x2f;main.css" />
  
  <title>Products</title>
</head>

  <body>
    <h1>Products</h1>
    <ul>
      
      <li>Bread</li>
      
      <li>Milk</li>
      
    </ul>
    <div class="footer">
  <a href="/">Home</a>
</div>

  </body>
</html>
"#
        .trim_start();
        assert_eq!(&str, expected);
    }
}
