use std::{collections::HashMap, fs, io::Write, path::Path};

use bevy_reflect::{GetTypeRegistration, Reflect, TypeRegistry};
use thiserror::Error;

use crate::{errors::Error, reflect_render::Renderer, template::Template};

pub struct Templates {
    templates: HashMap<String, Template>,
    type_registry: TypeRegistry,
}

impl Templates {
    pub fn new(templates: HashMap<String, Template>) -> Self {
        let mut type_registry = TypeRegistry::default();
        type_registry.register::<String>();
        Self {
            templates,
            type_registry,
        }
    }

    pub fn register_type<T: GetTypeRegistration>(&mut self) {
        self.type_registry.register::<T>();
    }

    pub fn render<W: Write, T: Reflect + GetTypeRegistration>(
        &self,
        name: &str,
        writer: &mut W,
        data: &T,
    ) -> Result<(), Error> {
        let mut renderer = Renderer::new(&self.templates, writer);
        renderer.render(name, data)
    }

    pub fn render_to_string<T: Reflect + GetTypeRegistration>(
        &self,
        name: &str,
        data: &T,
    ) -> Result<String, Error> {
        let mut buf = Vec::new();
        let mut renderer = Renderer::new(&self.templates, &mut buf);
        renderer.render(name, data)?;
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
                let template = Template::parse(file);
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
    use crate::reflect_render::Unescaped;

    use super::*;

    use bevy_reflect::{FromReflect, Reflect};
    use pretty_assertions::assert_eq;

    #[derive(Reflect)]
    struct Page {
        head: Head,
        items: Vec<Item>,
        user: User,
    }

    #[derive(Reflect)]
    struct Head {
        title: String,
        css: Vec<String>,
    }

    #[derive(Reflect, FromReflect)]
    struct Item {
        name: String,
    }

    #[derive(Reflect)]
    enum User {
        Customer { name: String },
        Admin,
    }

    fn compile_templates(sources: Vec<(&'static str, &'static str)>) -> Templates {
        let mut map = HashMap::new();
        for (name, src) in sources.iter() {
            map.insert(name.to_string(), Template::parse(src.to_string()));
        }
        Templates::new(map)
    }

    #[test]
    fn partials() {
        let templates = compile_templates(vec![
            (
                "main",
                r#"{{>"header"}}<ul>{{#items}}{{> "item"}}{{/items}}</ul>"#,
            ),
            ("header", r#"<h1>{{head.title}}</h1>"#),
            ("item", r#"<li>{{name}}</li>"#),
        ]);

        let str = templates
            .render_to_string(
                "main",
                &Page {
                    head: Head {
                        title: "Products".into(),
                        css: vec![],
                    },
                    items: vec![
                        Item {
                            name: "Bread".into(),
                        },
                        Item {
                            name: "Milk".into(),
                        },
                    ],
                    user: User::Admin,
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
                    user: User::Admin,
                    head: Head {
                        title: "Products".into(),
                        css: vec!["/index.css".into(), "/main.css".into()],
                    },
                    items: vec![
                        Item {
                            name: "Bread".into(),
                        },
                        Item {
                            name: "Milk".into(),
                        },
                    ],
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

    #[test]
    fn test_enum_sections() {
        let templates = compile_templates(vec![(
            "main",
            "<div>{{#Customer}}Customer: {{name}}{{/Customer}}</div>",
        )]);

        let src = templates
            .render_to_string(
                "main",
                &User::Customer {
                    name: "Jane Doe".into(),
                },
            )
            .unwrap();

        assert_eq!(src, "<div>Customer: Jane Doe</div>");
    }

    #[test]
    fn test_option_this() {
        let templates = compile_templates(vec![("main", "{{.}}")]);

        let src = templates
            .render_to_string("main", &Some("FOO".to_string()))
            .unwrap();
        assert_eq!(src, "FOO");
    }

    #[derive(Reflect, FromReflect)]
    struct First {
        second: Option<Second>,
    }

    #[derive(Reflect, FromReflect)]
    struct Second {
        third: Option<usize>,
    }

    #[test]
    fn test_option_path() {
        let templates = compile_templates(vec![("main", "third: {{second.third}}.")]);

        let src = templates
            .render_to_string(
                "main",
                &Some(First {
                    second: Some(Second { third: Some(12) }),
                }),
            )
            .unwrap();
        assert_eq!(src, "third: 12.");
    }

    #[test]
    fn test_option_path_missing() {
        let templates =
            compile_templates(vec![("main", "third: {{third}}{{^third}}N/A{{/third}}.")]);

        let src = templates
            .render_to_string("main", &Some(Second { third: None }))
            .unwrap();
        assert_eq!(src, "third: N/A.");
    }

    #[test]
    fn test_option_section() {
        let templates =
            compile_templates(vec![("main", "({{#second}}Exists: {{third}}{{/second}})")]);

        let src = templates
            .render_to_string(
                "main",
                &Some(First {
                    second: Some(Second { third: Some(12) }),
                }),
            )
            .unwrap();
        assert_eq!(src, "(Exists: 12)");
    }

    #[derive(Reflect)]
    struct BoolSection {
        section: bool,
    }

    #[test]
    fn true_section() {
        let templates = compile_templates(vec![("main", "{{#section}}OK{{/section}}")]);
        let src = templates
            .render_to_string("main", &BoolSection { section: true })
            .unwrap();
        assert_eq!(src, "OK");
    }

    #[test]
    fn false_section() {
        let templates = compile_templates(vec![("main", "{{#section}}OK{{/section}}")]);
        let src = templates
            .render_to_string("main", &BoolSection { section: false })
            .unwrap();
        assert_eq!(src, "");
    }

    #[test]
    fn empty_string_section() {
        let templates = compile_templates(vec![("main", "{{#.}}CONTENT{{/.}}")]);
        let src = templates.render_to_string("main", &"".to_owned()).unwrap();
        assert_eq!(src, "");
    }

    #[test]
    fn non_empty_string_section() {
        let templates = compile_templates(vec![("main", "{{#.}}CONTENT{{/.}}")]);
        let src = templates
            .render_to_string("main", &"FOO".to_owned())
            .unwrap();
        assert_eq!(src, "CONTENT");
    }

    #[derive(Reflect)]
    struct TupleStruct(String, i32, bool);

    #[test]
    fn tuple_struct_section() {
        let templates = compile_templates(vec![("main", "{{#.}}{{.}}, {{/.}}")]);
        let src = templates
            .render_to_string("main", &TupleStruct("foo".into(), 42, true))
            .unwrap();
        assert_eq!(src, "foo, 42, true, ");
    }

    #[test]
    fn tuple_section() {
        let templates = compile_templates(vec![("main", "{{#.}}{{.}}, {{/.}}")]);
        let src = templates
            .render_to_string("main", &("foo".to_owned(), 42, true))
            .unwrap();
        assert_eq!(src, "foo, 42, true, ");
    }

    #[test]
    fn test_true_inverted_section() {
        let templates = compile_templates(vec![("main", "{{^section}}OK{{/section}}")]);
        let src = templates
            .render_to_string("main", &BoolSection { section: true })
            .unwrap();
        assert_eq!(src, "");
    }

    #[test]
    fn test_false_inverted_section() {
        let templates = compile_templates(vec![("main", "{{^section}}OK{{/section}}")]);
        let src = templates
            .render_to_string("main", &BoolSection { section: false })
            .unwrap();
        assert_eq!(src, "OK");
    }

    #[derive(Reflect)]
    struct UnitEnumTemplate {
        value: UnitEnum,
    }

    #[derive(Reflect)]
    enum UnitEnum {
        FooBar,
        Baz,
    }

    #[test]
    fn render_unit_enum() {
        let templates = compile_templates(vec![("main", "{{value}}")]);
        let src = templates
            .render_to_string(
                "main",
                &UnitEnumTemplate {
                    value: UnitEnum::FooBar,
                },
            )
            .unwrap();
        assert_eq!(src, "foo-bar");
    }

    #[derive(Reflect)]
    enum WrapperEnum {
        String(String),
        Int(i32),
    }

    #[test]
    fn render_wrapper_enum_string() {
        let templates = compile_templates(vec![("main", "{{0}}")]);
        let src = templates
            .render_to_string("main", &WrapperEnum::String("FOOBAR".to_owned()))
            .unwrap();
        assert_eq!(src, "FOOBAR");
    }

    #[test]
    fn render_wrapper_enum_int() {
        let templates = compile_templates(vec![("main", "{{0}}")]);
        let src = templates
            .render_to_string("main", &WrapperEnum::Int(42))
            .unwrap();
        assert_eq!(src, "42");
    }

    #[test]
    fn render_escaped() {
        let templates = compile_templates(vec![("main", "{{.}}")]);
        let src = templates
            .render_to_string("main", &"<div>FOOBAR</div>".to_owned())
            .unwrap();
        assert_eq!(src, "&lt;div&gt;FOOBAR&lt;&#x2f;div&gt;");
    }

    #[test]
    fn render_unescaped() {
        let templates = compile_templates(vec![("main", "{{.}}")]);
        let src = templates
            .render_to_string("main", &Unescaped("<div>FOOBAR</div>".into()))
            .unwrap();
        assert_eq!(src, "<div>FOOBAR</div>");
    }

    #[derive(Reflect)]
    struct UnitStruct(i32);

    #[test]
    fn render_unit_struct() {
        let templates = compile_templates(vec![("main", "{{.}}")]);
        let src = templates.render_to_string("main", &UnitStruct(42)).unwrap();
        assert_eq!(src, "42");
    }
}
