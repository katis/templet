use std::io::Cursor;

use bevy_reflect::{GetPath, ListIter, Reflect, ReflectPathError, ReflectRef};
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
    loop {
        let pc = match bytecode.read_u8() {
            Ok(pc) => pc.try_into()?,
            Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
            Err(err) => return Err(err.into()),
        };
        match pc {
            OpCode::PushStr => {
                write_string(buf, bytecode)?;
            }
            OpCode::PushVar => {
                let path = read_string(bytecode)?;
                let last = stack.last().unwrap().reflect_path(path)?;
                stack.push(last);
            }
            OpCode::WriteVar => {
                let last = stack.last().unwrap();
                write_value(buf, *last)?;
                stack.pop();
            }
            OpCode::StartSection => {
                let path = read_string(bytecode)?;
                let start_pos = bytecode.position();
                let list = stack.last().unwrap().reflect_path(path)?;
                for item in ReflectIter::new(list)? {
                    stack.push(item);
                    bytecode.set_position(start_pos);
                    execute(buf, bytecode, stack)?;
                    stack.pop();
                }
            }
            OpCode::EndSection => {
                return Ok(());
            }
        }
    }
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

fn write_string(buf: &mut String, bytecode: &mut ByteCode) -> Result<()> {
    let s = read_string(bytecode)?;
    buf.push_str(s);
    Ok(())
}

fn read_string<'a>(bytecode: &mut ByteCode<'a>) -> Result<&'a str> {
    let len = bytecode.read_u64::<LittleEndian>()? as usize;
    let start = bytecode.position() as usize;
    let end = start + len;
    let bytes = &bytecode.get_ref()[start..end];
    let str = unsafe { core::str::from_utf8_unchecked(bytes) };
    bytecode.set_position(end as u64);
    Ok(str)
}

enum ReflectIter<'a> {
    List(ListIter<'a>),
}

impl<'a> ReflectIter<'a> {
    fn new(value: &'a dyn Reflect) -> Result<Self> {
        match value.reflect_ref() {
            ReflectRef::List(list) => Ok(Self::List(list.iter())),
            _ => Err(Error::UnsupportedSectionVar(value.type_name().to_owned())),
        }
    }
}

impl<'a> Iterator for ReflectIter<'a> {
    type Item = &'a dyn Reflect;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            ReflectIter::List(ref mut iter) => iter.next(),
        }
    }
}

#[repr(u8)]
pub enum OpCode {
    PushStr = 0x01,
    PushVar = 0x02,
    StartSection = 0x03,
    EndSection = 0x04,
    WriteVar = 0x05,
}

impl TryFrom<u8> for OpCode {
    type Error = Error;

    fn try_from(value: u8) -> std::result::Result<Self, Self::Error> {
        match value {
            0x01 => Ok(OpCode::PushStr),
            0x02 => Ok(OpCode::PushVar),
            0x03 => Ok(OpCode::StartSection),
            0x04 => Ok(OpCode::EndSection),
            0x05 => Ok(OpCode::WriteVar),
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
                Expr::String("!".into()),
            ]
            .into(),
        }];
        let bytecode = compiler.compile(ast.into());

        println!("ByteCode:");
        println!("{:?}", bytecode);

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
        assert_eq!(&buf, "foobar(113, qwe)");
    }
}
