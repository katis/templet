use std::{collections::HashMap, io::Write};

use bevy_reflect::{reflect_trait, Enum, GetTypeRegistration, TypeRegistry, VariantType};
use bevy_reflect::{
    Reflect,
    ReflectRef::{self, *},
};
use v_htmlescape::escape;

use crate::parse::Access;
use crate::{
    parse::{Field, Part},
    template::Template,
};

pub struct Renderer<'a, W> {
    registry: &'a TypeRegistry,
    templates: &'a HashMap<String, Template>,
    writer: &'a mut W,
}

impl<'a, W: Write> Renderer<'a, W> {
    pub fn new(
        registry: &'a TypeRegistry,
        templates: &'a HashMap<String, Template>,
        writer: &'a mut W,
    ) -> Self {
        Self {
            templates,
            writer,
            registry,
        }
    }

    pub fn render<T: Reflect + GetTypeRegistration>(
        &mut self,
        template: &str,
        data: &T,
    ) -> Result<(), std::io::Error> {
        if let Some(template) = self.templates.get(template) {
            let parts = template.parts();
            self.render_parts(parts, data)?;
        }
        Ok(())
    }

    fn render_parts(&mut self, parts: &[Part], data: &dyn Reflect) -> Result<(), std::io::Error> {
        for part in parts.iter() {
            match part {
                Part::Text(text) => write!(self.writer, "{}", text)?,
                Part::Variable(access) => {
                    if let Some(data) = get_path(data, access) {
                        self.render_value(data)?;
                    }
                }
                Part::Section(access, parts) => {
                    if let Some(data) = get_path(data, access) {
                        match data.reflect_ref() {
                            ReflectRef::List(list) => {
                                for item in list.iter() {
                                    self.render_parts(parts, item)?;
                                }
                            }
                            ReflectRef::Array(arr) => {
                                for item in arr.iter() {
                                    self.render_parts(parts, item)?;
                                }
                            }
                            ReflectRef::Value(val) if val.is::<bool>() => {
                                if let Some(true) = val.downcast_ref::<bool>() {
                                    self.render_parts(parts, data)?;
                                }
                            }
                            _ => self.render_parts(parts, data)?,
                        }
                    }
                }
                Part::InvertedSection(access, parts) => {
                    let path_data = get_path(data, access);
                    match path_data.map(|data| data.reflect_ref()) {
                        None => {
                            self.render_parts(parts, data)?;
                        }
                        Some(ReflectRef::List(list)) if list.len() == 0 => {
                            self.render_parts(parts, data)?;
                        }
                        Some(ReflectRef::Array(arr)) if arr.len() == 0 => {
                            self.render_parts(parts, data)?;
                        }
                        Some(ReflectRef::Enum(enm))
                            if is_option(enm) && enm.variant_name() == "None" =>
                        {
                            self.render_parts(parts, data)?;
                        }
                        Some(ReflectRef::Value(val)) if val.is::<bool>() => {
                            if let Some(false) = val.downcast_ref::<bool>() {
                                self.render_parts(parts, data)?;
                            }
                        }
                        _ => {}
                    }
                }
                Part::Include(name) => {
                    if let Some(template) = &self.templates.get(*name) {
                        self.render_parts(template.parts(), data)?;
                    }
                }
                Part::Comment => {}
            }
        }
        Ok(())
    }

    fn render_value(&mut self, value: &dyn Reflect) -> Result<(), std::io::Error> {
        if let Some(b) = value.downcast_ref::<bool>() {
            write!(self.writer, "{}", b)
        } else if let Some(n) = value.downcast_ref::<u8>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<u16>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<u32>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<u64>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<u128>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<usize>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<i8>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<i16>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<i32>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<i64>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<i128>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<isize>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<f32>() {
            write!(self.writer, "{}", n)
        } else if let Some(n) = value.downcast_ref::<f64>() {
            write!(self.writer, "{}", n)
        } else if let Some(s) = value.downcast_ref::<String>() {
            let escaped = escape(s.as_str());
            write!(self.writer, "{}", escaped)
        } else {
            if let ReflectRef::Enum(enm) = value.reflect_ref() {
                if is_option(enm) {
                    if let Some(value) = option_value(enm) {
                        self.render_value(value)?;
                    }
                    return Ok(());
                }
            }

            if let Some(type_data) = self
                .registry
                .get_type_data::<ReflectTemplateDisplay>(value.type_id())
            {
                let display_value = type_data.get(value).expect("value to implement Display");
                write!(self.writer, "{}", display_value)
            } else {
                write!(
                    self.writer,
                    "UNSUPPORTED_VARIABLE_TYPE({})",
                    value.type_name()
                )
            }
        }
    }
}

fn get_path<'r, 'a>(reflect: &'r dyn Reflect, access: &'a Access<'a>) -> Option<&'r dyn Reflect> {
    match access {
        Access::Variant(variant) => match reflect.reflect_ref() {
            ReflectRef::Enum(enm) if is_option(enm) => option_value(enm),
            ReflectRef::Enum(enm) if enm.variant_name() == *variant => Some(reflect),
            _ => None,
        },
        Access::Path(fields) => get_fields(reflect, &fields),
        Access::This => match reflect.reflect_ref() {
            ReflectRef::Enum(enm) if is_option(enm) => option_value(enm),
            _ => Some(reflect),
        },
    }
}

fn get_fields<'r, 'f>(
    reflect: &'r dyn Reflect,
    fields: &'f [Field<'f>],
) -> Option<&'r dyn Reflect> {
    let mut value = reflect;
    for field in fields.iter() {
        value = get_field(value, field)?;
    }
    Some(value)
}

fn get_field<'r, 'f>(reflect: &'r dyn Reflect, field: &'f Field<'f>) -> Option<&'r dyn Reflect> {
    match (field, reflect.reflect_ref()) {
        (field, Enum(enm)) if is_option(enm) => {
            option_value(enm).and_then(|value| get_field(value, field))
        }
        (Field::Index(i), List(list)) => list.get(*i),
        (Field::Index(i), Array(arr)) => arr.get(*i),
        (Field::Nth(n), TupleStruct(ts)) => ts.field(*n),
        (Field::Nth(n), Tuple(t)) => t.field(*n),
        (Field::Nth(n), Enum(enm)) if enm.is_variant(VariantType::Tuple) => enm.field_at(*n),
        (Field::Named(name), Struct(s)) => s.field(name),
        (Field::Named(name), Enum(enm)) if enm.is_variant(VariantType::Struct) => enm.field(name),
        _ => None,
    }
}

fn is_option(enm: &dyn Enum) -> bool {
    enm.type_name().starts_with("core::option::Option<")
}

fn option_value(enm: &dyn Enum) -> Option<&dyn Reflect> {
    if enm.variant_name() == "Some" {
        enm.field_at(0)
    } else {
        None
    }
}

#[reflect_trait]
pub trait TemplateDisplay: std::fmt::Display {}

impl<T: std::fmt::Display> TemplateDisplay for T {}
