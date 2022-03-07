use std::fmt::Write;

use valuable::Valuable;

use crate::{
    parser::{Part, TempletParser},
    render::render,
};

#[derive(Clone)]
pub struct Template {
    parts: Vec<Part>,
}

impl Template {
    pub fn parse(input: &str) -> Self {
        let parts = TempletParser::parse(input);
        Template { parts }
    }

    pub fn render(&self, ctx: &dyn Valuable) -> Result<String, std::fmt::Error> {
        let mut str = String::new();
        self.render_to(&mut str, ctx)?;
        Ok(str)
    }

    pub fn render_to<W: Write>(
        &self,
        writer: &mut W,
        ctx: &dyn Valuable,
    ) -> Result<(), std::fmt::Error> {
        render(writer, &self.parts, ctx.as_value())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[derive(Valuable)]
    struct User<'a> {
        name: &'a str,
        address: Address<'a>,
        roles: Vec<Role<'a>>,
    }

    #[derive(Valuable)]
    struct Address<'a> {
        street: &'a str,
        number: i32,
    }

    #[derive(Valuable)]
    struct Role<'a> {
        name: &'a str,
    }

    fn render(source: &str, ctx: &dyn Valuable) -> String {
        let tpl = Template::parse(source);
        tpl.render(ctx).unwrap()
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
                roles: vec![],
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
                roles: vec![],
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
                roles: vec![],
            },
        );
        assert_eq!(&s, "Joe@");
    }

    #[test]
    fn list() {
        let s = render(
            "{{#roles}}{{name}}, {{/roles}}",
            &User {
                name: "Joe",
                address: Address {
                    street: "Broadway",
                    number: 10,
                },
                roles: vec![
                    Role { name: "SALES" },
                    Role { name: "SUPPORT" },
                    Role { name: "BASIC" },
                ],
            },
        );
        assert_eq!(&s, "SALES, SUPPORT, BASIC, ");
    }
}
