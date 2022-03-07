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

    #[derive(Valuable)]
    enum Cost {
        Subscription { monthly_price: i32 },
        Purchase { price: i32 },
    }

    static SIMPLE_ENUM_TEMPLATE: &str = "{{#Subscription}}Monthly price: {{monthly_price}}{{/Subscription}}{{#Purchase}}Price: {{price}}{{/Purchase}}";

    #[test]
    fn simple_enum_section1() {
        let s = render(
            SIMPLE_ENUM_TEMPLATE,
            &Cost::Subscription { monthly_price: 12 },
        );
        assert_eq!(&s, "Monthly price: 12")
    }

    #[test]
    fn simple_enum_section2() {
        let s = render(SIMPLE_ENUM_TEMPLATE, &Cost::Purchase { price: 33 });
        assert_eq!(&s, "Price: 33")
    }

    #[derive(Valuable)]
    struct Product<'a> {
        name: &'a str,
        cost: Cost,
    }

    #[test]
    fn enum_in_section() {
        let s = render(
            "Name: {{name}}{{#cost}}{{#Purchase}}, Price: {{price}}e{{/Purchase}}{{/cost}}",
            &Product {
                name: "Book",
                cost: Cost::Purchase { price: 33 },
            },
        );
        assert_eq!(&s, "Name: Book, Price: 33e")
    }
}
