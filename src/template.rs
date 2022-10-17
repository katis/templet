use ouroboros::self_referencing;

use crate::parse::{parse, Part};

pub struct Template(TemplateData);

impl Template {
    pub fn parse(input: String) -> Self {
        Template(
            TemplateDataBuilder {
                source: input,
                parts_builder: |str| parse(str.as_str()),
            }
            .build(),
        )
    }

    pub(crate) fn parts(&self) -> &[Part] {
        self.0.borrow_parts()
    }
}

#[self_referencing]
struct TemplateData {
    source: String,
    #[borrows(source)]
    #[covariant]
    parts: Vec<Part<'this>>,
}
