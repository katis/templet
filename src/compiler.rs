use crate::{vm::OpCode, Expr};

#[derive(Default)]
pub struct Compiler {
    bytecode: Vec<u8>,
}

impl Compiler {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn compile(mut self, exprs: Vec<Expr>) -> Vec<u8> {
        for expr in exprs.into_iter() {
            self.compile_expr(expr);
        }
        self.bytecode
    }

    fn compile_expr(&mut self, expr: Expr) {
        match expr {
            Expr::String(string) => {
                self.compile_push_str(&string);
            }
            Expr::Section { var, body } => {
                self.compile_section(var);
                for expr in body.into_iter() {
                    self.compile_expr(expr);
                }
                self.bytecode.push(OpCode::EndSection as u8);
            }
            Expr::Var(var_name) => {
                self.compile_push_var(var_name);
                self.bytecode.push(OpCode::WriteVar as u8);
            }
        }
    }

    fn compile_section(&mut self, var_name: String) {
        self.bytecode.push(OpCode::StartSection as u8);
        self.compile_bytes(var_name.as_bytes());
    }

    fn compile_push_str(&mut self, value: &str) {
        self.bytecode.push(OpCode::PushStr as u8);
        self.compile_bytes(value.as_bytes());
    }

    fn compile_push_var(&mut self, var_name: String) {
        self.bytecode.push(OpCode::PushVar as u8);
        self.compile_bytes(var_name.as_bytes());
    }

    fn compile_bytes(&mut self, value: &[u8]) {
        self.compile_u64(value.len() as u64);
        self.bytecode.extend_from_slice(value);
    }

    fn compile_u64(&mut self, value: u64) {
        self.bytecode.extend_from_slice(&value.to_le_bytes());
    }
}
