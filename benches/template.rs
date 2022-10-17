use std::collections::HashMap;

use bevy_reflect::{FromReflect, Reflect};
use criterion::{criterion_group, criterion_main, Criterion};
use handlebars::Handlebars;
use ramhorns::Content;
use serde::Serialize;
use templet::{Template, Templates};

#[derive(Reflect, FromReflect, Content, Serialize)]
struct Page {
    title: String,
    products: Vec<Product>,
}

#[derive(Reflect, FromReflect, Content, Serialize)]
struct Item {
    name: String,
}

#[derive(Reflect, FromReflect, Content, Serialize)]
struct Product {
    name: String,
    price: Price,
    images: Vec<Image>,
}

#[derive(Reflect, FromReflect, Content, Serialize)]
struct Price {
    price: i32,
}

#[derive(Reflect, FromReflect, Content, Serialize)]
struct Image {
    title: String,
    href: String,
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
            title: "Weird store".into(),
            products: vec![
                Product {
                    name: "Netflix subscription".to_owned(),
                    images: vec![Image {
                        title: "Netflix".to_owned(),
                        href: "/netflix.logo.svg".to_owned(),
                    }],
                    price: Price { price: 13 },
                },
                Product {
                    name: "Artisan Bread".to_owned(),
                    images: vec![Image {
                        title: "Bread".to_owned(),
                        href: "/bread.jpg".to_owned(),
                    }],
                    price: Price { price: 4 },
                },
                Product {
                    name: "Orange juice".to_owned(),
                    images: vec![Image {
                        title: "Orange juice".to_owned(),
                        href: "/orange-juice.jpg".to_owned(),
                    }],
                    price: Price { price: 4 },
                },
            ],
        };

        b.iter(|| {
            templates.render("template", &mut buf, ctx);
            buf.clear();
        })
    });

    c.bench_function("render_ramhorns", |b| {
        let template = ramhorns::Template::new(PAGE).unwrap();
        let mut buf = Vec::new();
        let ctx = &Page {
            title: "Weird store".to_owned(),
            products: vec![
                Product {
                    name: "Netflix subscription".to_owned(),
                    images: vec![Image {
                        title: "Netflix".to_owned(),
                        href: "/netflix.logo.svg".to_owned(),
                    }],
                    price: Price { price: 13 },
                },
                Product {
                    name: "Artisan Bread".to_owned(),
                    images: vec![Image {
                        title: "Bread".to_owned(),
                        href: "/bread.jpg".to_owned(),
                    }],
                    price: Price { price: 4 },
                },
                Product {
                    name: "Orange juice".to_owned(),
                    images: vec![Image {
                        title: "Orange juice".to_owned(),
                        href: "/orange-juice.jpg".to_owned(),
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
            title: "Weird store".to_owned(),
            products: vec![
                Product {
                    name: "Netflix subscription".to_owned(),
                    images: vec![Image {
                        title: "Netflix".to_owned(),
                        href: "/netflix.logo.svg".to_owned(),
                    }],
                    price: Price { price: 13 },
                },
                Product {
                    name: "Artisan Bread".to_owned(),
                    images: vec![Image {
                        title: "Bread".to_owned(),
                        href: "/bread.jpg".to_owned(),
                    }],
                    price: Price { price: 4 },
                },
                Product {
                    name: "Orange juice".to_owned(),
                    images: vec![Image {
                        title: "Orange juice".to_owned(),
                        href: "/orange-juice.jpg".to_owned(),
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
