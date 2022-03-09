use crate::parse::{parse, Part};

pub struct Template {
    parts: Vec<Part>,
}

impl Template {
    pub fn parse(input: &str) -> Self {
        Template {
            parts: parse(input),
        }
    }

    pub(crate) fn parts(&self) -> &[Part] {
        &self.parts
    }
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use valuable::Valuable;

    use crate::templates::Templates;

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
        let mut map = HashMap::new();
        map.insert("template".into(), tpl);
        let templates = Templates::new(map);
        templates.render_to_string("template", ctx).unwrap()
    }

    #[test]
    fn escapes_variable() {
        let s = render("{{.}}", &"foo > \"12\"");
        assert_eq!(&s, "foo &gt; &quot;12&quot;");
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
        product: &'a str,
        cost: Cost,
    }

    #[test]
    fn enum_in_section() {
        let s = render(
            "Name: {{product}}{{#cost}}{{#Purchase}}, Price: {{price}}e{{/Purchase}}{{/cost}}",
            &Product {
                product: "Book",
                cost: Cost::Purchase { price: 33 },
            },
        );
        assert_eq!(&s, "Name: Book, Price: 33e")
    }

    #[derive(Valuable)]
    struct User2<'a> {
        name: Option<&'a str>,
        address: Option<Address<'a>>,
        products: Vec<Option<Product<'a>>>,
    }

    #[test]
    fn optional_section_none() {
        let s = render(
            "{{#address}}Street: {{street}}{{/address}}",
            &User2 {
                name: None,
                address: None,
                products: vec![],
            },
        );
        assert_eq!(&s, "");
    }

    #[test]
    fn optional_section_some() {
        let s = render(
            "{{#address}}Street: {{street}}{{/address}}",
            &User2 {
                name: None,
                address: Some(Address {
                    street: "Baker Street",
                    number: 221,
                }),
                products: vec![],
            },
        );
        assert_eq!(&s, "Street: Baker Street");
    }

    #[test]
    fn optional_field() {
        let s = render(
            "Name: {{name}}.",
            &User2 {
                name: Some("Joe Mama"),
                address: None,
                products: vec![],
            },
        );
        assert_eq!(&s, "Name: Joe Mama.");
    }

    #[test]
    fn optional_list_items() {
        let s = render(
            "Products: {{#products}}{{product}}, {{/products}}",
            &User2 {
                name: Some("Joe Mama"),
                address: None,
                products: vec![
                    Some(Product {
                        product: "Butter",
                        cost: Cost::Purchase { price: 300 },
                    }),
                    None,
                    Some(Product {
                        product: "Bread",
                        cost: Cost::Purchase { price: 200 },
                    }),
                ],
            },
        );
        assert_eq!(&s, "Products: Butter, Bread, ");
    }

    #[test]
    fn list_items_inverted() {
        let s = render(
            "Products: {{#products}}{{product}}, {{/products}}{{^products}}none{{/products}}",
            &User2 {
                name: Some("Joe Mama"),
                address: None,
                products: vec![],
            },
        );
        assert_eq!(&s, "Products: none");
    }

    #[derive(Valuable)]
    struct ErrorMsg<'a> {
        has_error: bool,
        msg: &'a str,
    }

    #[test]
    fn bool_section_true() {
        let s = render(
            "{{#has_error}}Error: {{msg}}.{{/has_error}}{{^has_error}}OK{{/has_error}}",
            &ErrorMsg {
                has_error: true,
                msg: "invalid email",
            },
        );
        assert_eq!(&s, "Error: invalid email.")
    }

    #[test]
    fn bool_section_false() {
        let s = render(
            "{{#has_error}}Error: {{msg}}.{{/has_error}}{{^has_error}}OK{{/has_error}}",
            &ErrorMsg {
                has_error: false,
                msg: "invalid email",
            },
        );
        assert_eq!(&s, "OK")
    }

    #[derive(Valuable)]
    struct Data<'a>(ErrorMsg<'a>);

    #[test]
    fn positional_section() {
        let s = render(
            "{{#0}}Error: {{msg}}.{{/0}}",
            &Data(ErrorMsg {
                has_error: true,
                msg: "invalid email",
            }),
        );
        assert_eq!(&s, "Error: invalid email.")
    }

    #[test]
    fn positional_section_missing() {
        let s = render(
            "{{#1}}Error: {{msg}}.{{/1}}",
            &Data(ErrorMsg {
                has_error: true,
                msg: "invalid email",
            }),
        );
        assert_eq!(&s, "")
    }

    #[derive(Valuable)]
    struct OptData<'a>(Option<ErrorMsg<'a>>);

    #[test]
    fn opt_positional_section_some() {
        let s = render(
            "{{#0}}Error: {{msg}}.{{/0}}",
            &OptData(Some(ErrorMsg {
                has_error: true,
                msg: "invalid email",
            })),
        );
        assert_eq!(&s, "Error: invalid email.")
    }

    #[test]
    fn opt_positional_section_none() {
        let s = render("{{#0}}Error: {{msg}}.{{/0}}{{^0}}OK{{/0}}", &OptData(None));
        assert_eq!(&s, "OK")
    }

    #[derive(Valuable)]
    struct Positional<'a>(&'a str, i32);

    #[test]
    fn positional_variable() {
        let s = render("Name: {{0}}, age: {{1}}", &Positional("Joe", 23));
        assert_eq!(&s, "Name: Joe, age: 23")
    }

    #[derive(Valuable)]
    struct Tags<'a> {
        tags: &'a [&'a str],
    }

    #[test]
    fn this_list() {
        let s = render(
            "Tags: {{#tags}}{{.}}, {{/tags}}",
            &Tags {
                tags: &["work", "buzzwords", "agile"],
            },
        );
        assert_eq!(&s, "Tags: work, buzzwords, agile, ");
    }
}
