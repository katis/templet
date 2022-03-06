use valuable::Valuable;

use crate::{
    parser::{TempletParser, Token},
    renderer::Renderer,
};

pub struct Template<'a> {
    tokens: Vec<Token<'a>>,
}

impl<'a> Template<'a> {
    pub fn parse(input: &'a str) -> Self {
        let tokens = TempletParser::parse(input);
        Template { tokens }
    }

    pub fn parse_owned(input: &'a str) -> Template<'static> {
        let tokens = TempletParser::parse(input);
        Template {
            tokens: tokens.into_iter().map(|t| t.into_owned()).collect(),
        }
    }

    pub fn render(&self, ctx: &dyn Valuable) -> String {
        let mut str = String::new();
        let mut renderer = Renderer::new(&mut str);
        renderer.render(&self.tokens, ctx);
        str
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Valuable)]
    struct User<'a> {
        name: &'a str,
        address: Address<'a>,
    }

    #[derive(Valuable)]
    struct Address<'a> {
        street: &'a str,
        number: i32,
    }

    fn render(source: &str, ctx: &dyn Valuable) -> String {
        let tpl = Template::parse(source);
        tpl.render(ctx)
    }

    #[test]
    fn variable() {
        let s = render(
            "{{street}}",
            &Address {
                street: "Broadway",
                number: 10,
            },
        );
        assert_eq!(&s, "Broadway");
    }

    #[test]
    fn section() {
        let s = render(
            "- {{name}}@{{#address}}{{street}}{{/address}} -",
            &User {
                name: "Joe",
                address: Address {
                    street: "Broadway",
                    number: 10,
                },
            },
        );
        assert_eq!(&s, "- Joe@Broadway -");
    }

    #[test]
    fn section_parent_context() {
        let s = render(
            "{{#address}}{{name}}@{{street}}{{/address}}",
            &User {
                name: "Joe",
                address: Address {
                    street: "Broadway",
                    number: 10,
                },
            },
        );
        assert_eq!(&s, "Joe@Broadway");
    }

    #[test]
    fn section_missing_value() {
        let s = render(
            "{{name}}@{{#address}}{{FOOBAR}}{{/address}}",
            &User {
                name: "Joe",
                address: Address {
                    street: "Broadway",
                    number: 10,
                },
            },
        );
        assert_eq!(&s, "Joe@");
    }
}
