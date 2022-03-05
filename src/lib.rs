mod parser;
mod renderer;

use valuable::{Valuable, Visit};

pub struct Template {}

impl Template {}

impl Visit for Template {
    fn visit_value(&mut self, value: valuable::Value<'_>) {
        match value {
            valuable::Value::Structable(v) => {
                v.visit(self);
            }
            _ => todo!(),
        }
    }

    fn visit_named_fields(&mut self, named_values: &valuable::NamedValues<'_>) {
        let _ = named_values;
        for (field, value) in named_values {}
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    use valuable::Valuable;

    #[derive(Valuable)]
    struct Ctx {
        name: String,
    }

    #[test]
    fn test() {}
}
