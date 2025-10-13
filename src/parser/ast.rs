//! Abstract Syntax Tree definitions
//!
//! This module defines the AST node types for the Lux language.

use crate::error::SourceLocation;

/// Root AST node representing a complete program
#[derive(Debug, Clone, PartialEq)]
pub struct Ast {
    pub statements: Vec<Stmt>,
}

/// Statement node
#[derive(Debug, Clone, PartialEq)]
pub enum Stmt {
    /// Variable declaration: local x: int = 42
    VarDecl {
        name: String,
        type_annotation: Option<Type>,
        initializer: Option<Expr>,
        is_const: bool,
        location: SourceLocation,
    },

    /// Function declaration
    FunctionDecl {
        name: String,
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
        body: Vec<Stmt>,
        is_async: bool,
        location: SourceLocation,
    },

    /// Expression statement
    Expression {
        expr: Expr,
        location: SourceLocation,
    },

    /// If statement
    If {
        condition: Expr,
        then_branch: Vec<Stmt>,
        else_branch: Option<Vec<Stmt>>,
        location: SourceLocation,
    },

    /// While loop
    While {
        condition: Expr,
        body: Vec<Stmt>,
        location: SourceLocation,
    },

    /// For loop
    For {
        initializer: Option<Box<Stmt>>,
        condition: Option<Expr>,
        increment: Option<Expr>,
        body: Vec<Stmt>,
        location: SourceLocation,
    },

    /// Return statement
    Return {
        value: Option<Expr>,
        location: SourceLocation,
    },

    /// Break statement
    Break {
        location: SourceLocation,
    },

    /// Continue statement
    Continue {
        location: SourceLocation,
    },

    /// Block statement
    Block {
        statements: Vec<Stmt>,
        location: SourceLocation,
    },
}

/// Expression node
#[derive(Debug, Clone, PartialEq)]
pub enum Expr {
    /// Literal value
    Literal {
        value: Literal,
        location: SourceLocation,
    },

    /// Variable reference
    Variable {
        name: String,
        location: SourceLocation,
    },

    /// Binary operation
    Binary {
        left: Box<Expr>,
        operator: BinaryOp,
        right: Box<Expr>,
        location: SourceLocation,
    },

    /// Unary operation
    Unary {
        operator: UnaryOp,
        operand: Box<Expr>,
        location: SourceLocation,
    },

    /// Assignment
    Assign {
        target: String,
        value: Box<Expr>,
        location: SourceLocation,
    },

    /// Function call
    Call {
        callee: Box<Expr>,
        arguments: Vec<Expr>,
        location: SourceLocation,
    },

    /// Table literal
    Table {
        fields: Vec<(TableKey, Expr)>,
        location: SourceLocation,
    },

    /// Table access: table.field or table[key]
    TableAccess {
        table: Box<Expr>,
        key: Box<Expr>,
        location: SourceLocation,
    },

    /// Logical operation (and, or)
    Logical {
        left: Box<Expr>,
        operator: LogicalOp,
        right: Box<Expr>,
        location: SourceLocation,
    },

    /// Function expression (anonymous function)
    Function {
        params: Vec<(String, Type)>,
        return_type: Option<Type>,
        body: Vec<Stmt>,
        location: SourceLocation,
    },
}

/// Table key (for table literals)
#[derive(Debug, Clone, PartialEq)]
pub enum TableKey {
    Identifier(String),
    Expression(Box<Expr>),
}

/// Binary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum BinaryOp {
    Add,
    Subtract,
    Multiply,
    Divide,
    Modulo,
    Equal,
    NotEqual,
    Less,
    LessEqual,
    Greater,
    GreaterEqual,
}

/// Unary operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum UnaryOp {
    Negate,
    Not,
    Length, // # operator
}

/// Logical operators
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LogicalOp {
    And,
    Or,
}

/// Literal value
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
    Boolean(bool),
    Nil,
}

/// Type annotation
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum Type {
    Int,
    Float,
    String,
    Bool,
    Nil,
    Table,
    Function {
        params: Vec<Type>,
        return_type: Box<Type>,
    },
}

