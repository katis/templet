use std::collections::HashMap;

use criterion::{criterion_group, criterion_main, Criterion};
use handlebars::Handlebars;
use ramhorns::Content;
use serde::Serialize;
use templet::{Template, Templates};
use valuable::Valuable;

#[derive(Valuable, Content, Serialize)]
struct Page<'a> {
    title: &'a str,
    products: &'a [Product<'a>],
}

#[derive(Valuable, Content, Serialize)]
struct Item<'a> {
    name: &'a str,
}

#[derive(Valuable, Content, Serialize)]
struct Product<'a> {
    name: &'a str,
    price: Price,
    images: &'a [Image<'a>],
}

#[derive(Valuable, Content, Serialize)]
struct Price {
    price: i32,
}

#[derive(Valuable, Content, Serialize)]
struct Image<'a> {
    title: &'a str,
    href: &'a str,
}

pub fn criterion_benchmark(c: &mut Criterion) {
    static PAGE: &str = r#"
    <!DOCTYPE html>
    <html>
      <head>
        <title>{{title}}<title>
      </head>
      <body>
        <h1>{{title}}</h1>
        <ul>
          {{#product}}
            <li>
              <h2>{{name}}</h2>
              {{#price}}
                Price: {{price}}e
              {{/price}}
              <ul>
                {{#images}}
                  <li><img alt="{{title}}" href="{{href}}"></li>
                {{/images}}
              </ul>
            </li>
          {{/product}}
        </ul>
      </body>
    </html>"#;

    static PAGE_HANDLEBARS: &str = r#"
    <!DOCTYPE html>
    <html>
      <head>
        <title>{{title}}<title>
      </head>
      <body>
        <h1>{{title}}</h1>
        <ul>
          {{#each product}}
            <li>
              <h2>{{name}}</h2>
              {{#with price}}
                Price: {{price}}e
              {{/with}}
              <ul>
                {{#each images}}
                  <li><img alt="{{title}}" href="{{href}}"></li>
                {{/each}}
              </ul>
            </li>
          {{/each}}
        </ul>
      </body>
    </html>"#;

    c.bench_function("parse", |b| {
        let mut template = None;

        b.iter(|| {
            template = Some(Template::parse(PAGE.to_string()));
        });
    });

    c.bench_function("parse_ramhorns", |b| {
        let mut template = None;

        b.iter(|| {
            template = ramhorns::Template::new(PAGE.to_string()).ok();
        });
    });

    c.bench_function("parse_handlebars", |b| {
        let mut template = None;

        b.iter(|| {
            template = Some(handlebars::Template::compile(&PAGE_HANDLEBARS.to_string()).unwrap());
        });
    });

    c.bench_function("render", |b| {
        let t = Template::parse(PAGE.to_string());
        let mut map = HashMap::new();
        map.insert("template".to_owned(), t);
        let templates = Templates::new(map);

        let mut buf = Vec::new();
        let ctx = &Page {
            title: "Weird store",
            products: &[
                Product {
                    name: "Netflix subscription",
                    images: &[Image {
                        title: "Netflix",
                        href: "/netflix.logo.svg",
                    }],
                    price: Price { price: 13 },
                },
                Product {
                    name: "Artisan Bread",
                    images: &[Image {
                        title: "Bread",
                        href: "/bread.jpg",
                    }],
                    price: Price { price: 4 },
                },
                Product {
                    name: "Orange juice",
                    images: &[Image {
                        title: "Orange juice",
                        href: "/orange-juice.jpg",
                    }],
                    price: Price { price: 4 },
                },
            ],
        };

        b.iter(|| {
            templates.render("template", &mut buf, &ctx).unwrap();
            buf.clear();
        })
    });

    c.bench_function("render_ramhorns", |b| {
        let template = ramhorns::Template::new(PAGE).unwrap();
        let mut buf = Vec::new();
        let ctx = &Page {
            title: "Weird store",
            products: &[
                Product {
                    name: "Netflix subscription",
                    images: &[Image {
                        title: "Netflix",
                        href: "/netflix.logo.svg",
                    }],
                    price: Price { price: 13 },
                },
                Product {
                    name: "Artisan Bread",
                    images: &[Image {
                        title: "Bread",
                        href: "/bread.jpg",
                    }],
                    price: Price { price: 4 },
                },
                Product {
                    name: "Orange juice",
                    images: &[Image {
                        title: "Orange juice",
                        href: "/orange-juice.jpg",
                    }],
                    price: Price { price: 4 },
                },
            ],
        };

        b.iter(|| {
            template.render_to_writer(&mut buf, ctx).unwrap();
            buf.clear();
        });
    });

    c.bench_function("render_handlebars", |b| {
        let mut buf = Vec::new();

        let template = handlebars::Template::compile(PAGE_HANDLEBARS).unwrap();
        let mut reg = Handlebars::new();
        reg.register_template("template", template);

        let ctx = &Page {
            title: "Weird store",
            products: &[
                Product {
                    name: "Netflix subscription",
                    images: &[Image {
                        title: "Netflix",
                        href: "/netflix.logo.svg",
                    }],
                    price: Price { price: 13 },
                },
                Product {
                    name: "Artisan Bread",
                    images: &[Image {
                        title: "Bread",
                        href: "/bread.jpg",
                    }],
                    price: Price { price: 4 },
                },
                Product {
                    name: "Orange juice",
                    images: &[Image {
                        title: "Orange juice",
                        href: "/orange-juice.jpg",
                    }],
                    price: Price { price: 4 },
                },
            ],
        };

        b.iter(|| {
            reg.render_to_write("template", ctx, &mut buf).unwrap();
            buf.clear();
        });
    });
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
