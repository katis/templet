use std::io::BufWriter;

use criterion::{criterion_group, criterion_main, Criterion};
use ramhorns::Content;
use templet::Template;
use valuable::Valuable;

#[derive(Valuable, Content)]
struct Page<'a> {
    title: &'a str,
    products: &'a [Product<'a>],
}

#[derive(Valuable, Content)]
struct Item<'a> {
    name: &'a str,
}

#[derive(Valuable, Content)]
struct Product<'a> {
    name: &'a str,
    price: Price,
    images: &'a [Image<'a>],
}

#[derive(Valuable, Content)]
struct Price {
    price: i32,
}

#[derive(Valuable, Content)]
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

    c.bench_function("parse", |b| {
        let mut template = None;

        b.iter(|| {
            template = Some(Template::parse(PAGE));
        });
    });

    c.bench_function("parse_ramhorns", |b| {
        let mut template = None;

        b.iter(|| {
            template = ramhorns::Template::new(PAGE).ok();
        });
    });

    c.bench_function("render", |b| {
        let t = Template::parse(PAGE);
        let mut buf = String::new();
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
            t.render_to(&mut buf, &ctx).unwrap();
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
}

criterion_group!(benches, criterion_benchmark);
criterion_main!(benches);
