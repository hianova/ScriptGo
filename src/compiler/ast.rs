use alloc::string::String;
use alloc::vec::Vec;
use alloc::boxed::Box;

#[derive(Debug, Clone, PartialEq)]
pub enum Type {
    Int,
    Float,
    String,
    Tensor, // For R replacement
    DynamicArray,
    Void,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    Identifier(String),
    BinaryOp(Box<Expr>, BinaryOperator, Box<Expr>),
    MethodCall(Box<Expr>, String, Vec<Expr>), // For method chaining (SQL replacement)
    Call(String, Vec<Expr>),
    ArrayLit(Vec<Expr>),
}

#[derive(Debug, Clone, PartialEq)]
pub enum BinaryOperator {
    Add, Sub, Mul, Div, Mod, Eq, Lt, Gt,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Statement {
    LetDecl(String, Type, Expr),
    Assign(String, Expr),
    ExprStmt(Expr),
    If(Expr, Vec<Statement>, Vec<Statement>),
    While(Expr, Vec<Statement>),
    Return(Option<Expr>),
    FunctionDecl(String, Vec<(String, Type)>, Type, Vec<Statement>),
}

#[derive(Debug, Clone, PartialEq)]
pub struct Program {
    pub statements: Vec<Statement>,
}
