mod compiler;
mod parser;
mod vm;

pub enum Expr {
    String(String),
    Var(String),
    Section { var: String, body: Vec<Expr> },
}
