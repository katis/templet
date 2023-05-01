use std::io::Cursor;

use bevy_reflect::{GetPath, Reflect, ReflectPathError, ReflectRef};
use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

type Stack<'a> = Vec<&'a dyn Reflect>;
type ByteCode<'a> = Cursor<&'a [u8]>;

fn execute_to(buf: &mut String, bytecode: &[u8], root: &dyn Reflect) -> Result<()> {
    let mut stack: Stack = vec![root];
    let mut bytecode = Cursor::new(bytecode);
    execute(buf, &mut bytecode, &mut stack)?;
    Ok(())
}

fn execute(buf: &mut String, bytecode: &mut ByteCode, stack: &mut Stack) -> Result<()> {
    while let Some(pc) = read_opcode(bytecode)? {
        match pc {
            OpCode::PushStr => {
                write_str(buf, bytecode)?;
            }
            OpCode::PushVar => {
                let path = read_str(bytecode)?;
                let last = stack.last().unwrap().reflect_path(path)?;
                stack.push(last);
            }
            OpCode::WriteVar => {
                let last = stack.last().unwrap();
                write_value(buf, *last)?;
                stack.pop();
            }
            OpCode::StartSection => {
                let path = read_str(bytecode)?;
                let last = stack.last().unwrap();
                if let ReflectRef::Enum(enm) = last.reflect_ref() {
                    if enm.variant_name() == path {
                        execute(buf, bytecode, stack)?;
                    } else {
                        execute_skip(buf, bytecode)?;
                    }
                } else {
                    let value = last.reflect_path(path)?;
                    match value.reflect_ref() {
                        ReflectRef::Tuple(tuple) => {
                            execute_iter(buf, bytecode, stack, tuple.iter_fields())?;
                        }
                        ReflectRef::List(list) => {
                            execute_iter(buf, bytecode, stack, list.iter())?;
                        }
                        ReflectRef::Array(array) => {
                            execute_iter(buf, bytecode, stack, array.iter())?;
                        }
                        ReflectRef::Map(map) => {
                            execute_iter(buf, bytecode, stack, map.iter().map(|(k, _)| k))?;
                        }
                        _ => {
                            return Err(Error::UnsupportedSectionVar(value.type_name().to_owned()))
                        }
                    }
                }
            }
            OpCode::EndSection => {
                return Ok(());
            }
        }
    }
    Ok(())
}

fn execute_skip(buf: &mut String, bytecode: &mut ByteCode) -> Result<()> {
    while let Some(pc) = read_opcode(bytecode)? {
        let mut depth = 0;
        match pc {
            OpCode::StartSection => {
                depth += 1;
            }
            OpCode::EndSection => {
                depth -= 1;
                if depth == 0 {
                    return Ok(());
                }
            }
            _ => (),
        }
    }
    Ok(())
}

fn read_opcode(bytecode: &mut ByteCode) -> Result<Option<OpCode>> {
    match bytecode.read_u8() {
        Ok(pc) => pc.try_into().map(Some),
        Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => Ok(None),
        Err(err) => Err(err.into()),
    }
}

fn execute_iter<'a, I: Iterator<Item = &'a dyn Reflect>>(
    buf: &mut String,
    bytecode: &mut ByteCode,
    stack: &mut Stack<'a>,
    iter: I,
) -> Result<()> {
    let start_pos = bytecode.position();
    for item in iter {
        stack.push(item);
        bytecode.set_position(start_pos);
        execute(buf, bytecode, stack)?;
        stack.pop();
    }
    Ok(())
}

fn write_value(buf: &mut String, value: &dyn Reflect) -> Result<()> {
    use std::fmt::Write;

    if let Some(s) = value.downcast_ref::<String>() {
        buf.push_str(s);
    } else if let Some(s) = value.downcast_ref::<bool>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<u8>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<i8>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<u16>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<i16>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<u32>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<i32>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<u64>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<i64>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<u128>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<i128>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<f32>() {
        write!(buf, "{}", s).ok();
    } else if let Some(s) = value.downcast_ref::<f64>() {
        write!(buf, "{}", s).ok();
    } else if let ReflectRef::Enum(enm) = value.reflect_ref() {
        if enm.type_name().starts_with("core::option::Option<") {
            if let Some(value) = enm.field_at(0) {
                write_value(buf, value)?;
            }
        } else {
            return Err(Error::UnsupportedType(value.type_name().to_owned()));
        }
    } else {
        return Err(Error::UnsupportedType(value.type_name().to_owned()));
    }
    Ok(())
}

fn write_str(buf: &mut String, bytecode: &mut ByteCode) -> Result<()> {
    let s = read_str(bytecode)?;
    buf.push_str(s);
    Ok(())
}

fn read_str<'a>(bytecode: &mut ByteCode<'a>) -> Result<&'a str> {
    let len = bytecode.read_u64::<LittleEndian>()? as usize;
    let start = bytecode.position() as usize;
    let end = start + len;
    let bytes = &bytecode.get_ref()[start..end];
    let str = unsafe { core::str::from_utf8_unchecked(bytes) };
    bytecode.set_position(end as u64);
    Ok(str)
}

#[repr(u8)]
pub enum OpCode {
    PushStr = 0x01,
    PushVar = 0x02,
    StartSection = 0x03,
    EndSection = 0x04,
    WriteVar = 0x07,
}

impl TryFrom<u8> for OpCode {
    type Error = Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x01 => Ok(OpCode::PushStr),
            0x02 => Ok(OpCode::PushVar),
            0x03 => Ok(OpCode::StartSection),
            0x04 => Ok(OpCode::EndSection),
            0x07 => Ok(OpCode::WriteVar),
            byte => Err(Error::InvalidOpCode(byte)),
        }
    }
}

pub type Result<T> = std::result::Result<T, Error>;

#[derive(Debug, Error)]
pub enum Error {
    #[error("IO error: {}", 0)]
    Io(#[from] std::io::Error),
    #[error("invalid opcode: {:#04x}", 0)]
    InvalidOpCode(u8),
    #[error("invalid field access: {}", 0)]
    InvalidField(String),
    #[error("unsupported value type: {}", 0)]
    UnsupportedType(String),
    #[error("unsupported value for section: {}", 0)]
    UnsupportedSectionVar(String),
}

impl<'a> From<ReflectPathError<'a>> for Error {
    fn from(value: ReflectPathError<'a>) -> Self {
        Error::InvalidField(value.to_string())
    }
}

#[cfg(test)]
mod tests {
    use bevy_reflect::FromReflect;

    use crate::{compiler::Compiler, Expr};

    use super::*;

    #[test]
    fn test_write() {
        let compiler = Compiler::new();
        let bytecode = compiler.compile(vec![
            Expr::String("foobar(".into()),
            Expr::Var("int".into()),
            Expr::String(", ".into()),
            Expr::Var("str".into()),
            Expr::String(");".into()),
        ]);

        #[derive(Reflect)]
        struct Props {
            int: Option<i32>,
            str: Option<String>,
        }

        let mut buf = String::new();
        let props = Props {
            int: Some(113),
            str: Some("qwe".into()),
        };
        execute_to(&mut buf, &bytecode, &props).unwrap();
        assert_eq!(&buf, "foobar(113, qwe);");
    }

    #[test]
    fn test_section() {
        let compiler = Compiler::new();
        let ast = [Expr::Section {
            var: "items".into(),
            body: [
                Expr::String("Item: ".into()),
                Expr::Var("value".into()),
                Expr::String("! ".into()),
            ]
            .into(),
        }];
        let bytecode = compiler.compile(ast.into());

        #[derive(Reflect)]
        struct Props {
            items: Vec<Item>,
        }

        #[derive(Reflect, FromReflect)]
        struct Item {
            value: i32,
        }

        let mut buf = String::new();
        let props = Props {
            items: [Item { value: 42 }, Item { value: 666 }].into(),
        };
        execute_to(&mut buf, &bytecode, &props).unwrap();
        assert_eq!(&buf, "Item: 42! Item: 666! ");
    }
}
