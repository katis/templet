use std::io::Cursor;

use bevy_reflect::{GetPath, List, ListIter, Reflect, ReflectPathError, ReflectRef};
use byteorder::{LittleEndian, ReadBytesExt};
use thiserror::Error;

pub struct Vm<'a> {
    bytecode: Cursor<&'a [u8]>,
}

impl<'a> Vm<'a> {
    pub fn new(bytecode: &'a [u8]) -> Self {
        Self {
            bytecode: Cursor::new(bytecode),
        }
    }

    pub fn write(mut self, buf: &mut String, root: &dyn Reflect) -> Result<()> {
        let mut stack = Vec::<(&dyn Reflect, Option<ReflectIter>, u64)>::new();
        stack.push((root, None, 0));

        loop {
            let pc = match self.bytecode.read_u8() {
                Ok(pc) => pc.try_into()?,
                Err(err) if err.kind() == std::io::ErrorKind::UnexpectedEof => return Ok(()),
                Err(err) => return Err(err.into()),
            };
            match pc {
                OpCode::PushStr => {
                    self.write_string(buf)?;
                }
                OpCode::PushVar => {
                    let path = self.read_string()?;
                    let (last, _, _) = stack.last().unwrap();
                    stack.push((last.reflect_path(path)?, None, self.bytecode.position()));
                }
                OpCode::WriteVar => {
                    let (last, _, _) = stack.last().unwrap();
                    self.write_value(buf, *last)?;
                    stack.pop();
                }
                OpCode::StartSection => {
                    let path = self.read_string()?;
                    let (last, _, _) = stack.last().unwrap();
                    stack.push((last.reflect_path(path)?, self.bytecode.position()));
                }
                OpCode::EndSection => todo!(),
            }
        }
    }

    fn write_string(&mut self, buf: &mut String) -> Result<()> {
        let s = self.read_string()?;
        buf.push_str(s);
        Ok(())
    }

    fn write_value(&mut self, buf: &mut String, value: &dyn Reflect) -> Result<()> {
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
                    self.write_value(buf, value)?;
                }
            } else {
                return Err(Error::UnsupportedType(value.type_name().to_owned()));
            }
        } else {
            return Err(Error::UnsupportedType(value.type_name().to_owned()));
        }
        Ok(())
    }

    fn read_string(&mut self) -> Result<&str> {
        let len = self.bytecode.read_u64::<LittleEndian>()? as usize;
        let start = self.bytecode.position() as usize;
        let end = start + len;
        let bytes = &self.bytecode.get_ref()[start..end];
        let str = unsafe { core::str::from_utf8_unchecked(bytes) };
        self.bytecode.set_position(end as u64);
        Ok(str)
    }
}

enum ReflectIter<'a> {
    List(ListIter<'a>),
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
}

impl<'a> From<ReflectPathError<'a>> for Error {
    fn from(value: ReflectPathError<'a>) -> Self {
        Error::InvalidField(value.to_string())
    }
}

#[cfg(test)]
mod tests {
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
        Vm::new(&bytecode)
            .write(
                &mut buf,
                &Props {
                    int: Some(113),
                    str: Some("qwe".into()),
                },
            )
            .unwrap();
        assert_eq!(&buf, "foobar(113, qwe)");
    }
}
