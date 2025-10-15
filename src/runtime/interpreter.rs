//! Interpreter implementation
//!
//! This module implements the tree-walking interpreter for Lux.

use std::collections::HashMap;
use std::sync::Arc;
use crate::error::{LuxError, LuxResult, SourceLocation};
use crate::parser::ast::{Ast, Stmt, Expr, BinaryOp, UnaryOp, LogicalOp, Literal, TableKey};
use crate::async_runtime::{AsyncExecutor, TaskState};
use super::value::{Value, TableValue, FunctionValue, NativeFunctionValue};
use crate::lexer::Lexer;
use crate::parser::Parser;

/// Environment for variable storage
#[derive(Debug, Clone)]
struct Environment {
    scopes: Vec<HashMap<String, Value>>,
}

impl Environment {
    fn new() -> Self {
        Self {
            scopes: vec![HashMap::new()],
        }
    }

    fn push_scope(&mut self) {
        self.scopes.push(HashMap::new());
    }

    fn pop_scope(&mut self) {
        if self.scopes.len() > 1 {
            self.scopes.pop();
        }
    }

    fn define(&mut self, name: String, value: Value) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, value);
        }
    }

    fn get(&self, name: &str) -> Option<Value> {
        for scope in self.scopes.iter().rev() {
            if let Some(value) = scope.get(name) {
                return Some(value.clone());
            }
        }
        None
    }

    fn set(&mut self, name: &str, value: Value) -> bool {
        for scope in self.scopes.iter_mut().rev() {
            if scope.contains_key(name) {
                scope.insert(name.to_string(), value);
                return true;
            }
        }
        false
    }
}

/// Control flow signals
#[derive(Debug, Clone)]
enum ControlFlow {
    None,
    Return(Value),
    Break,
    Continue,
}

/// Interpreter
pub struct Interpreter {
    env: Environment,
    control_flow: ControlFlow,
    executor: Arc<AsyncExecutor>,
    loaded_modules: HashMap<String, bool>,
    current_file_dir: Option<String>,
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            env: Environment::new(),
            control_flow: ControlFlow::None,
            executor: Arc::new(AsyncExecutor::new()),
            loaded_modules: HashMap::new(),
            current_file_dir: None,
        };
        interpreter.register_builtins();
        interpreter
    }

    fn register_builtins(&mut self) {
        // print function
        self.env.define(
            "print".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "print".to_string(),
                arity: 1,
                func: |args| {
                    println!("{}", args[0]);
                    Ok(Value::Nil)
                },
            }),
        );

        // setmetatable function
        self.env.define(
            "setmetatable".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "setmetatable".to_string(),
                arity: 2,
                func: |args| {
                    if let (Value::Table(mut table), Value::Table(meta)) = (args[0].clone(), args[1].clone()) {
                        table.metatable = Some(Box::new(meta));
                        Ok(Value::Table(table))
                    } else {
                        Err("setmetatable expects two tables".to_string())
                    }
                },
            }),
        );

        // getmetatable function
        self.env.define(
            "getmetatable".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "getmetatable".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::Table(table) = &args[0] {
                        if let Some(meta) = &table.metatable {
                            Ok(Value::Table((**meta).clone()))
                        } else {
                            Ok(Value::Nil)
                        }
                    } else {
                        Err("getmetatable expects a table".to_string())
                    }
                },
            }),
        );

        // read_file function
        self.env.define(
            "read_file".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "read_file".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::String(path) = &args[0] {
                        match std::fs::read_to_string(path) {
                            Ok(content) => Ok(Value::String(content)),
                            Err(e) => Err(format!("Failed to read file '{}': {}", path, e)),
                        }
                    } else {
                        Err("read_file expects a string path".to_string())
                    }
                },
            }),
        );

        // write_file function
        self.env.define(
            "write_file".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "write_file".to_string(),
                arity: 2,
                func: |args| {
                    if let (Value::String(path), Value::String(content)) = (&args[0], &args[1]) {
                        match std::fs::write(path, content) {
                            Ok(_) => Ok(Value::Nil),
                            Err(e) => Err(format!("Failed to write file '{}': {}", path, e)),
                        }
                    } else {
                        Err("write_file expects two strings (path, content)".to_string())
                    }
                },
            }),
        );

        // string_split function
        self.env.define(
            "string_split".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_split".to_string(),
                arity: 2,
                func: |args| {
                    if let (Value::String(text), Value::String(delimiter)) = (&args[0], &args[1]) {
                        let parts: Vec<Value> = text
                            .split(delimiter.as_str())
                            .map(|s| Value::String(s.to_string()))
                            .collect();
                        let mut table = TableValue::new();
                        table.array = parts;
                        Ok(Value::Table(table))
                    } else {
                        Err("string_split expects two strings (text, delimiter)".to_string())
                    }
                },
            }),
        );

        // string_contains function
        self.env.define(
            "string_contains".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_contains".to_string(),
                arity: 2,
                func: |args| {
                    if let (Value::String(text), Value::String(pattern)) = (&args[0], &args[1]) {
                        Ok(Value::Bool(text.contains(pattern.as_str())))
                    } else {
                        Err("string_contains expects two strings (text, pattern)".to_string())
                    }
                },
            }),
        );

        // string_starts_with function
        self.env.define(
            "string_starts_with".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_starts_with".to_string(),
                arity: 2,
                func: |args| {
                    if let (Value::String(text), Value::String(prefix)) = (&args[0], &args[1]) {
                        Ok(Value::Bool(text.starts_with(prefix.as_str())))
                    } else {
                        Err("string_starts_with expects two strings (text, prefix)".to_string())
                    }
                },
            }),
        );

        // string_trim function
        self.env.define(
            "string_trim".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_trim".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::String(text) = &args[0] {
                        Ok(Value::String(text.trim().to_string()))
                    } else {
                        Err("string_trim expects a string".to_string())
                    }
                },
            }),
        );

        // string_length function
        self.env.define(
            "string_length".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_length".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::String(text) = &args[0] {
                        Ok(Value::Int(text.len() as i64))
                    } else {
                        Err("string_length expects a string".to_string())
                    }
                },
            }),
        );

        // table_length function (for arrays)
        self.env.define(
            "table_length".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "table_length".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::Table(table) = &args[0] {
                        Ok(Value::Int(table.array.len() as i64))
                    } else {
                        Err("table_length expects a table".to_string())
                    }
                },
            }),
        );

        // table_push function
        self.env.define(
            "table_push".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "table_push".to_string(),
                arity: 2,
                func: |args| {
                    if let Value::Table(mut table) = args[0].clone() {
                        table.array.push(args[1].clone());
                        Ok(Value::Table(table))
                    } else {
                        Err("table_push expects a table as first argument".to_string())
                    }
                },
            }),
        );

        // parse_lux function - parses Lux source code and returns AST as table
        self.env.define(
            "parse_lux".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "parse_lux".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::String(source) = &args[0] {
                        // Tokenize
                        let mut lexer = Lexer::new(source.as_str(), None);
                        let tokens = match lexer.tokenize() {
                            Ok(t) => t,
                            Err(e) => return Err(format!("Lexer error: {}", e)),
                        };

                        // Parse
                        let mut parser = Parser::new(tokens);
                        let ast = match parser.parse() {
                            Ok(a) => a,
                            Err(e) => return Err(format!("Parser error: {}", e)),
                        };

                        // Convert AST to table structure
                        Ok(Interpreter::ast_to_value(&ast))
                    } else {
                        Err("parse_lux expects a string (source code)".to_string())
                    }
                },
            }),
        );

        // type_of(value) -> string
        self.env.define(
            "type_of".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "type_of".to_string(),
                arity: 1,
                func: |args| {
                    let type_name = match &args[0] {
                        Value::Int(_) => "int",
                        Value::Float(_) => "float",
                        Value::String(_) => "string",
                        Value::Bool(_) => "bool",
                        Value::Nil => "nil",
                        Value::Table(_) => "table",
                        Value::Function(_) => "function",
                        Value::NativeFunction(_) => "function",
                        Value::Pointer(_) => "pointer",
                    };
                    Ok(Value::String(type_name.to_string()))
                },
            }),
        );

        // to_string(value) -> string
        self.env.define(
            "to_string".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "to_string".to_string(),
                arity: 1,
                func: |args| {
                    let s = match &args[0] {
                        Value::Int(i) => i.to_string(),
                        Value::Float(f) => f.to_string(),
                        Value::String(s) => s.clone(),
                        Value::Bool(b) => b.to_string(),
                        Value::Nil => "nil".to_string(),
                        _ => format!("{:?}", args[0]),
                    };
                    Ok(Value::String(s))
                },
            }),
        );

        // to_int(value) -> int
        self.env.define(
            "to_int".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "to_int".to_string(),
                arity: 1,
                func: |args| {
                    match &args[0] {
                        Value::Int(i) => Ok(Value::Int(*i)),
                        Value::Float(f) => Ok(Value::Int(*f as i64)),
                        Value::String(s) => {
                            s.parse::<i64>()
                                .map(Value::Int)
                                .map_err(|_| format!("Cannot convert '{}' to int", s))
                        }
                        Value::Bool(b) => Ok(Value::Int(if *b { 1 } else { 0 })),
                        _ => Err("Cannot convert to int".to_string()),
                    }
                },
            }),
        );

        // to_float(value) -> float
        self.env.define(
            "to_float".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "to_float".to_string(),
                arity: 1,
                func: |args| {
                    match &args[0] {
                        Value::Int(i) => Ok(Value::Float(*i as f64)),
                        Value::Float(f) => Ok(Value::Float(*f)),
                        Value::String(s) => {
                            s.parse::<f64>()
                                .map(Value::Float)
                                .map_err(|_| format!("Cannot convert '{}' to float", s))
                        }
                        _ => Err("Cannot convert to float".to_string()),
                    }
                },
            }),
        );

        // substring(text: string, start: int, length: int) -> string
        self.env.define(
            "substring".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "substring".to_string(),
                arity: 3,
                func: |args| {
                    if let (Value::String(text), Value::Int(start), Value::Int(length)) = (&args[0], &args[1], &args[2]) {
                        let start = *start as usize;
                        let length = *length as usize;
                        let chars: Vec<char> = text.chars().collect();

                        if start >= chars.len() {
                            return Ok(Value::String(String::new()));
                        }

                        let end = std::cmp::min(start + length, chars.len());
                        let result: String = chars[start..end].iter().collect();
                        Ok(Value::String(result))
                    } else {
                        Err("substring expects (string, int, int)".to_string())
                    }
                },
            }),
        );

        // string_replace(text: string, from: string, to: string) -> string
        self.env.define(
            "string_replace".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_replace".to_string(),
                arity: 3,
                func: |args| {
                    if let (Value::String(text), Value::String(from), Value::String(to)) = (&args[0], &args[1], &args[2]) {
                        Ok(Value::String(text.replace(from, to)))
                    } else {
                        Err("string_replace expects (string, string, string)".to_string())
                    }
                },
            }),
        );

        // string_upper(text: string) -> string
        self.env.define(
            "string_upper".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_upper".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::String(text) = &args[0] {
                        Ok(Value::String(text.to_uppercase()))
                    } else {
                        Err("string_upper expects a string".to_string())
                    }
                },
            }),
        );

        // string_lower(text: string) -> string
        self.env.define(
            "string_lower".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_lower".to_string(),
                arity: 1,
                func: |args| {
                    if let Value::String(text) = &args[0] {
                        Ok(Value::String(text.to_lowercase()))
                    } else {
                        Err("string_lower expects a string".to_string())
                    }
                },
            }),
        );

        // string_ends_with(text: string, suffix: string) -> bool
        self.env.define(
            "string_ends_with".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "string_ends_with".to_string(),
                arity: 2,
                func: |args| {
                    if let (Value::String(text), Value::String(suffix)) = (&args[0], &args[1]) {
                        Ok(Value::Bool(text.ends_with(suffix)))
                    } else {
                        Err("string_ends_with expects (string, string)".to_string())
                    }
                },
            }),
        );

        // Math functions
        // sqrt(x: float) -> float
        self.env.define(
            "sqrt".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "sqrt".to_string(),
                arity: 1,
                func: |args| {
                    let num = match &args[0] {
                        Value::Float(f) => *f,
                        Value::Int(i) => *i as f64,
                        _ => return Err("sqrt expects a number".to_string()),
                    };
                    Ok(Value::Float(num.sqrt()))
                },
            }),
        );

        // pow(base: float, exp: float) -> float
        self.env.define(
            "pow".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "pow".to_string(),
                arity: 2,
                func: |args| {
                    let base = match &args[0] {
                        Value::Float(f) => *f,
                        Value::Int(i) => *i as f64,
                        _ => return Err("pow expects numbers".to_string()),
                    };
                    let exp = match &args[1] {
                        Value::Float(f) => *f,
                        Value::Int(i) => *i as f64,
                        _ => return Err("pow expects numbers".to_string()),
                    };
                    Ok(Value::Float(base.powf(exp)))
                },
            }),
        );

        // abs(x: number) -> number
        self.env.define(
            "abs".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "abs".to_string(),
                arity: 1,
                func: |args| {
                    match &args[0] {
                        Value::Int(i) => Ok(Value::Int(i.abs())),
                        Value::Float(f) => Ok(Value::Float(f.abs())),
                        _ => Err("abs expects a number".to_string()),
                    }
                },
            }),
        );

        // floor(x: float) -> int
        self.env.define(
            "floor".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "floor".to_string(),
                arity: 1,
                func: |args| {
                    let num = match &args[0] {
                        Value::Float(f) => *f,
                        Value::Int(i) => return Ok(Value::Int(*i)),
                        _ => return Err("floor expects a number".to_string()),
                    };
                    Ok(Value::Int(num.floor() as i64))
                },
            }),
        );

        // ceil(x: float) -> int
        self.env.define(
            "ceil".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "ceil".to_string(),
                arity: 1,
                func: |args| {
                    let num = match &args[0] {
                        Value::Float(f) => *f,
                        Value::Int(i) => return Ok(Value::Int(*i)),
                        _ => return Err("ceil expects a number".to_string()),
                    };
                    Ok(Value::Int(num.ceil() as i64))
                },
            }),
        );

        // min(a: number, b: number) -> number
        self.env.define(
            "min".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "min".to_string(),
                arity: 2,
                func: |args| {
                    match (&args[0], &args[1]) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.min(b))),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.min(*b))),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).min(*b))),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.min(*b as f64))),
                        _ => Err("min expects two numbers".to_string()),
                    }
                },
            }),
        );

        // max(a: number, b: number) -> number
        self.env.define(
            "max".to_string(),
            Value::NativeFunction(NativeFunctionValue {
                name: "max".to_string(),
                arity: 2,
                func: |args| {
                    match (&args[0], &args[1]) {
                        (Value::Int(a), Value::Int(b)) => Ok(Value::Int(*a.max(b))),
                        (Value::Float(a), Value::Float(b)) => Ok(Value::Float(a.max(*b))),
                        (Value::Int(a), Value::Float(b)) => Ok(Value::Float((*a as f64).max(*b))),
                        (Value::Float(a), Value::Int(b)) => Ok(Value::Float(a.max(*b as f64))),
                        _ => Err("max expects two numbers".to_string()),
                    }
                },
            }),
        );
    }

    /// Convert AST to a Value (table structure) that Lux code can work with
    fn ast_to_value(ast: &Ast) -> Value {
        let mut table = TableValue::new();

        // Convert statements to array
        for stmt in &ast.statements {
            table.array.push(Self::stmt_to_value(stmt));
        }

        Value::Table(table)
    }

    fn stmt_to_value(stmt: &Stmt) -> Value {
        let mut table = TableValue::new();

        match stmt {
            Stmt::VarDecl { name, type_annotation, initializer, .. } => {
                table.fields.insert("type".to_string(), Value::String("VarDecl".to_string()));
                table.fields.insert("name".to_string(), Value::String(name.clone()));
                if let Some(vt) = type_annotation {
                    table.fields.insert("type_annotation".to_string(), Value::String(format!("{:?}", vt)));
                }
                if let Some(init) = initializer {
                    table.fields.insert("initializer".to_string(), Self::expr_to_value(init));
                }
            }
            Stmt::FunctionDecl { name, params, return_type, body, is_async, .. } => {
                table.fields.insert("type".to_string(), Value::String("FunctionDecl".to_string()));
                table.fields.insert("name".to_string(), Value::String(name.clone()));
                table.fields.insert("is_async".to_string(), Value::Bool(*is_async));

                let mut params_table = TableValue::new();
                for (param_name, param_type) in params {
                    let mut param_table = TableValue::new();
                    param_table.fields.insert("name".to_string(), Value::String(param_name.clone()));
                    param_table.fields.insert("type".to_string(), Value::String(format!("{:?}", param_type)));
                    params_table.array.push(Value::Table(param_table));
                }
                table.fields.insert("params".to_string(), Value::Table(params_table));

                if let Some(rt) = return_type {
                    table.fields.insert("return_type".to_string(), Value::String(format!("{:?}", rt)));
                }

                let mut body_table = TableValue::new();
                for s in body {
                    body_table.array.push(Self::stmt_to_value(s));
                }
                table.fields.insert("body".to_string(), Value::Table(body_table));
            }
            Stmt::Return { value, .. } => {
                table.fields.insert("type".to_string(), Value::String("Return".to_string()));
                if let Some(v) = value {
                    table.fields.insert("value".to_string(), Self::expr_to_value(v));
                }
            }
            Stmt::Expression { expr, .. } => {
                table.fields.insert("type".to_string(), Value::String("Expression".to_string()));
                table.fields.insert("expr".to_string(), Self::expr_to_value(expr));
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                table.fields.insert("type".to_string(), Value::String("If".to_string()));
                table.fields.insert("condition".to_string(), Self::expr_to_value(condition));

                let mut then_table = TableValue::new();
                for s in then_branch {
                    then_table.array.push(Self::stmt_to_value(s));
                }
                table.fields.insert("then_branch".to_string(), Value::Table(then_table));

                if let Some(else_b) = else_branch {
                    let mut else_table = TableValue::new();
                    for s in else_b {
                        else_table.array.push(Self::stmt_to_value(s));
                    }
                    table.fields.insert("else_branch".to_string(), Value::Table(else_table));
                }
            }
            Stmt::While { condition, body, .. } => {
                table.fields.insert("type".to_string(), Value::String("While".to_string()));
                table.fields.insert("condition".to_string(), Self::expr_to_value(condition));

                let mut body_table = TableValue::new();
                for s in body {
                    body_table.array.push(Self::stmt_to_value(s));
                }
                table.fields.insert("body".to_string(), Value::Table(body_table));
            }
            Stmt::For { initializer, condition, increment, body, .. } => {
                table.fields.insert("type".to_string(), Value::String("For".to_string()));
                if let Some(i) = initializer {
                    table.fields.insert("initializer".to_string(), Self::stmt_to_value(i));
                }
                if let Some(c) = condition {
                    table.fields.insert("condition".to_string(), Self::expr_to_value(c));
                }
                if let Some(inc) = increment {
                    table.fields.insert("increment".to_string(), Self::expr_to_value(inc));
                }

                let mut body_table = TableValue::new();
                for s in body {
                    body_table.array.push(Self::stmt_to_value(s));
                }
                table.fields.insert("body".to_string(), Value::Table(body_table));
            }
            _ => {
                table.fields.insert("type".to_string(), Value::String(format!("{:?}", stmt)));
            }
        }

        Value::Table(table)
    }

    fn expr_to_value(expr: &Expr) -> Value {
        let mut table = TableValue::new();

        match expr {
            Expr::Literal { value, .. } => {
                table.fields.insert("type".to_string(), Value::String("Literal".to_string()));
                match value {
                    Literal::Integer(i) => table.fields.insert("value".to_string(), Value::Int(*i)),
                    Literal::Float(f) => table.fields.insert("value".to_string(), Value::Float(*f)),
                    Literal::String(s) => table.fields.insert("value".to_string(), Value::String(s.clone())),
                    Literal::Boolean(b) => table.fields.insert("value".to_string(), Value::Bool(*b)),
                    Literal::Nil => table.fields.insert("value".to_string(), Value::Nil),
                };
            }
            Expr::Variable { name, .. } => {
                table.fields.insert("type".to_string(), Value::String("Variable".to_string()));
                table.fields.insert("name".to_string(), Value::String(name.clone()));
            }
            Expr::Binary { left, operator, right, .. } => {
                table.fields.insert("type".to_string(), Value::String("Binary".to_string()));
                table.fields.insert("operator".to_string(), Value::String(format!("{:?}", operator)));
                table.fields.insert("left".to_string(), Self::expr_to_value(left));
                table.fields.insert("right".to_string(), Self::expr_to_value(right));
            }
            Expr::Call { callee, arguments, .. } => {
                table.fields.insert("type".to_string(), Value::String("Call".to_string()));
                table.fields.insert("callee".to_string(), Self::expr_to_value(callee));

                let mut args_table = TableValue::new();
                for arg in arguments {
                    args_table.array.push(Self::expr_to_value(arg));
                }
                table.fields.insert("arguments".to_string(), Value::Table(args_table));
            }
            _ => {
                table.fields.insert("type".to_string(), Value::String(format!("{:?}", expr)));
            }
        }

        Value::Table(table)
    }

    pub fn interpret(&mut self, ast: &Ast) -> LuxResult<()> {
        for stmt in &ast.statements {
            self.execute_stmt(stmt)?;

            // Check for early return at top level
            if matches!(self.control_flow, ControlFlow::Return(_)) {
                break;
            }
        }
        Ok(())
    }

    /// Execute a task (function with arguments)
    fn execute_task(&mut self, task_id: usize, func: FunctionValue, args: Vec<Value>) -> LuxResult<Value> {
        // Push a new scope for the function
        self.env.push_scope();

        // Bind parameters
        for (param, arg) in func.params.iter().zip(args.iter()) {
            self.env.define(param.clone(), arg.clone());
        }

        // Execute the function body
        for stmt in &func.body {
            if let Err(e) = self.execute_stmt(stmt) {
                self.executor.update_task_state(task_id, TaskState::Failed(e.to_string()));
                self.env.pop_scope();
                return Err(e);
            }

            // Check for early return
            if matches!(self.control_flow, ControlFlow::Return(_)) {
                break;
            }
        }

        let return_value = match &self.control_flow {
            ControlFlow::Return(v) => v.clone(),
            _ => Value::Nil,
        };

        // Reset control flow
        self.control_flow = ControlFlow::None;

        self.executor.update_task_state(task_id, TaskState::Completed(return_value.clone()));
        self.env.pop_scope();

        Ok(return_value)
    }

    fn import_module(&mut self, path: &str, location: &SourceLocation) -> LuxResult<()> {
        // Check if already loaded
        if self.loaded_modules.contains_key(path) {
            return Ok(());
        }

        // Resolve the module path
        let resolved_path = self.resolve_module_path(path, location)?;

        // Read the file
        let source = std::fs::read_to_string(&resolved_path)
            .map_err(|e| LuxError::runtime_error(
                format!("Failed to read module '{}': {}", path, e),
                Some(location.clone()),
            ))?;

        // Parse the module
        let mut lexer = Lexer::new(&source, Some(&resolved_path));
        let tokens = lexer.tokenize()?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse()?;

        // Execute the module in the current environment
        for stmt in &ast.statements {
            self.execute_stmt(stmt)?;
        }

        // Mark as loaded
        self.loaded_modules.insert(path.to_string(), true);

        Ok(())
    }

    fn resolve_module_path(&self, path: &str, location: &SourceLocation) -> LuxResult<String> {
        use std::path::Path;

        // Try different locations:
        // 1. Relative to current file directory
        if let Some(ref current_dir) = self.current_file_dir {
            let candidate = Path::new(current_dir).join(format!("{}.lux", path));
            if candidate.exists() {
                return Ok(candidate.to_string_lossy().to_string());
            }
        }

        // 2. In lib/ directory
        let lib_path = Path::new("lib").join(format!("{}.lux", path));
        if lib_path.exists() {
            return Ok(lib_path.to_string_lossy().to_string());
        }

        // 3. In tools/ directory
        let tools_path = Path::new("tools").join(format!("{}.lux", path));
        if tools_path.exists() {
            return Ok(tools_path.to_string_lossy().to_string());
        }

        // 4. As absolute or relative path with .lux extension
        let direct_path_str = format!("{}.lux", path);
        let direct_path = Path::new(&direct_path_str);
        if direct_path.exists() {
            return Ok(direct_path.to_string_lossy().to_string());
        }

        Err(LuxError::runtime_error(
            format!("Module '{}' not found", path),
            Some(location.clone()),
        ))
    }

    fn execute_stmt(&mut self, stmt: &Stmt) -> LuxResult<()> {
        match stmt {
            Stmt::Import { path, location } => {
                self.import_module(path, location)?;
                Ok(())
            }

            Stmt::VarDecl { name, initializer, location, .. } => {
                let value = if let Some(init) = initializer {
                    self.eval_expr(init)?
                } else {
                    Value::Nil
                };
                self.env.define(name.clone(), value);
                Ok(())
            }

            Stmt::FunctionDecl { name, params, body, is_async, .. } => {
                let func = FunctionValue {
                    name: name.clone(),
                    params: params.iter().map(|(n, _)| n.clone()).collect(),
                    body: body.clone(),
                    is_async: *is_async,
                };
                self.env.define(name.clone(), Value::Function(func));
                Ok(())
            }

            Stmt::Expression { expr, .. } => {
                self.eval_expr(expr)?;
                Ok(())
            }

            Stmt::If { condition, then_branch, else_branch, location } => {
                let cond_value = self.eval_expr(condition)?;

                if cond_value.is_truthy() {
                    for stmt in then_branch {
                        self.execute_stmt(stmt)?;
                        if !matches!(self.control_flow, ControlFlow::None) {
                            return Ok(());
                        }
                    }
                } else if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        self.execute_stmt(stmt)?;
                        if !matches!(self.control_flow, ControlFlow::None) {
                            return Ok(());
                        }
                    }
                }
                Ok(())
            }

            Stmt::While { condition, body, location } => {
                loop {
                    let cond_value = self.eval_expr(condition)?;
                    if !cond_value.is_truthy() {
                        break;
                    }

                    for stmt in body {
                        self.execute_stmt(stmt)?;

                        match &self.control_flow {
                            ControlFlow::Break => {
                                self.control_flow = ControlFlow::None;
                                return Ok(());
                            }
                            ControlFlow::Continue => {
                                self.control_flow = ControlFlow::None;
                                break;
                            }
                            ControlFlow::Return(_) => return Ok(()),
                            ControlFlow::None => {}
                        }
                    }
                }
                Ok(())
            }

            Stmt::For { initializer, condition, increment, body, location } => {
                self.env.push_scope();

                if let Some(init) = initializer {
                    self.execute_stmt(init)?;
                }

                loop {
                    if let Some(cond) = condition {
                        let cond_value = self.eval_expr(cond)?;
                        if !cond_value.is_truthy() {
                            break;
                        }
                    }

                    for stmt in body {
                        self.execute_stmt(stmt)?;

                        match &self.control_flow {
                            ControlFlow::Break => {
                                self.control_flow = ControlFlow::None;
                                self.env.pop_scope();
                                return Ok(());
                            }
                            ControlFlow::Continue => {
                                self.control_flow = ControlFlow::None;
                                break;
                            }
                            ControlFlow::Return(_) => {
                                self.env.pop_scope();
                                return Ok(());
                            }
                            ControlFlow::None => {}
                        }
                    }

                    if let Some(inc) = increment {
                        self.eval_expr(inc)?;
                    }
                }

                self.env.pop_scope();
                Ok(())
            }

            Stmt::Return { value, location } => {
                let return_value = if let Some(v) = value {
                    self.eval_expr(v)?
                } else {
                    Value::Nil
                };
                self.control_flow = ControlFlow::Return(return_value);
                Ok(())
            }

            Stmt::Break { .. } => {
                self.control_flow = ControlFlow::Break;
                Ok(())
            }

            Stmt::Continue { .. } => {
                self.control_flow = ControlFlow::Continue;
                Ok(())
            }

            Stmt::Block { statements, location } => {
                self.env.push_scope();
                for stmt in statements {
                    self.execute_stmt(stmt)?;
                    if !matches!(self.control_flow, ControlFlow::None) {
                        self.env.pop_scope();
                        return Ok(());
                    }
                }
                self.env.pop_scope();
                Ok(())
            }
        }
    }

    fn eval_expr(&mut self, expr: &Expr) -> LuxResult<Value> {
        match expr {
            Expr::Literal { value, .. } => {
                Ok(match value {
                    Literal::Integer(n) => Value::Int(*n),
                    Literal::Float(f) => Value::Float(*f),
                    Literal::String(s) => Value::String(s.clone()),
                    Literal::Boolean(b) => Value::Bool(*b),
                    Literal::Nil => Value::Nil,
                })
            }

            Expr::Variable { name, location } => {
                self.env.get(name).ok_or_else(|| {
                    LuxError::runtime_error(
                        format!("Undefined variable '{}'", name),
                        Some(location.clone()),
                    )
                })
            }

            Expr::Binary { left, operator, right, location } => {
                let left_val = self.eval_expr(left)?;
                let right_val = self.eval_expr(right)?;
                self.eval_binary(left_val, operator, right_val, location)
            }

            Expr::Unary { operator, operand, location } => {
                let operand_val = self.eval_expr(operand)?;
                self.eval_unary(operator, operand_val, location)
            }

            Expr::Assign { target, value, location } => {
                let val = self.eval_expr(value)?;
                if self.env.set(target, val.clone()) {
                    Ok(val)
                } else {
                    Err(LuxError::runtime_error(
                        format!("Undefined variable '{}'", target),
                        Some(location.clone()),
                    ))
                }
            }

            Expr::Call { callee, arguments, location } => {
                let func = self.eval_expr(callee)?;
                let mut args = Vec::new();
                for arg in arguments {
                    args.push(self.eval_expr(arg)?);
                }
                self.call_function(func, args, location)
            }

            Expr::Table { fields, location } => {
                let mut table = TableValue::new();

                for (key, value_expr) in fields {
                    let value = self.eval_expr(value_expr)?;
                    match key {
                        TableKey::Identifier(name) => {
                            table.fields.insert(name.clone(), value);
                        }
                        TableKey::Expression(key_expr) => {
                            let key_val = self.eval_expr(key_expr)?;
                            table.set(key_val, value);
                        }
                    }
                }

                Ok(Value::Table(table))
            }

            Expr::TableAccess { table, key, location } => {
                let table_val = self.eval_expr(table)?;
                let key_val = self.eval_expr(key)?;

                if let Value::Table(t) = table_val {
                    Ok(t.get(&key_val).unwrap_or(Value::Nil))
                } else {
                    Err(LuxError::runtime_error(
                        "Can only index tables",
                        Some(location.clone()),
                    ))
                }
            }

            Expr::Logical { left, operator, right, location } => {
                let left_val = self.eval_expr(left)?;

                match operator {
                    LogicalOp::And => {
                        if !left_val.is_truthy() {
                            Ok(left_val)
                        } else {
                            self.eval_expr(right)
                        }
                    }
                    LogicalOp::Or => {
                        if left_val.is_truthy() {
                            Ok(left_val)
                        } else {
                            self.eval_expr(right)
                        }
                    }
                }
            }

            Expr::Function { params, body, .. } => {
                // Create an anonymous function value
                let func = FunctionValue {
                    name: "<anonymous>".to_string(),
                    params: params.iter().map(|(n, _)| n.clone()).collect(),
                    body: body.clone(),
                    is_async: false,
                };
                Ok(Value::Function(func))
            }

            Expr::Spawn { call, location } => {
                // Spawn expects a function call expression
                match call.as_ref() {
                    Expr::Call { callee, arguments, .. } => {
                        // Evaluate the callee to get the function
                        let func_value = self.eval_expr(callee)?;

                        match func_value {
                            Value::Function(func) => {
                                // Evaluate arguments
                                let mut args = Vec::new();
                                for arg in arguments {
                                    args.push(self.eval_expr(arg)?);
                                }

                                // Spawn the task (don't execute yet - will execute in parallel when awaited)
                                let task_id = self.executor.spawn_function(func, args);

                                // Return the task ID
                                Ok(Value::Int(task_id as i64))
                            }
                            _ => Err(LuxError::runtime_error(
                                "spawn expects a function call",
                                Some(location.clone()),
                            )),
                        }
                    }
                    _ => Err(LuxError::runtime_error(
                        "spawn expects a function call expression",
                        Some(location.clone()),
                    )),
                }
            }

            Expr::Await { task, location } => {
                // Await expects a task ID (integer) or a table of task IDs
                let task_value = self.eval_expr(task)?;

                match task_value {
                    Value::Int(task_id) => {
                        // Single task await - execute the task if not already done
                        if let Some(task) = self.executor.get_task(task_id as usize) {
                            match task.state {
                                TaskState::Completed(value) => Ok(value),
                                TaskState::Failed(msg) => Err(LuxError::runtime_error(
                                    &format!("Task {} failed: {}", task_id, msg),
                                    Some(location.clone()),
                                )),
                                TaskState::Pending => {
                                    // Execute the task now
                                    if let Some(func) = task.function {
                                        let result = self.execute_task(task_id as usize, func, task.arguments)?;
                                        Ok(result)
                                    } else {
                                        Err(LuxError::runtime_error(
                                            &format!("Task {} has no function to execute", task_id),
                                            Some(location.clone()),
                                        ))
                                    }
                                }
                                _ => Err(LuxError::runtime_error(
                                    &format!("Task {} is in invalid state", task_id),
                                    Some(location.clone()),
                                )),
                            }
                        } else {
                            Err(LuxError::runtime_error(
                                &format!("Task {} not found", task_id),
                                Some(location.clone()),
                            ))
                        }
                    }
                    Value::Table(table) => {
                        // Multiple tasks await - execute all tasks in parallel using threads
                        use std::thread;

                        let mut handles = Vec::new();
                        let mut task_ids_array = Vec::new();
                        let mut task_ids_fields = HashMap::new();

                        // Collect array task IDs and spawn threads
                        for value in table.array.iter() {
                            match value {
                                Value::Int(task_id) => {
                                    let tid = *task_id as usize;
                                    task_ids_array.push(tid);

                                    if let Some(task) = self.executor.get_task(tid) {
                                        if matches!(task.state, TaskState::Pending) {
                                            if let Some(func) = task.function {
                                                let args = task.arguments.clone();
                                                let env = self.env.clone();
                                                let executor = self.executor.clone();

                                                let handle = thread::spawn(move || {
                                                    let mut task_interp = Interpreter {
                                                        env,
                                                        control_flow: ControlFlow::None,
                                                        executor: executor.clone(),
                                                        loaded_modules: HashMap::new(),
                                                        current_file_dir: None,
                                                    };
                                                    task_interp.execute_task(tid, func, args)
                                                });
                                                handles.push((tid, handle));
                                            }
                                        }
                                    } else {
                                        return Err(LuxError::runtime_error(
                                            &format!("Task {} not found", task_id),
                                            Some(location.clone()),
                                        ));
                                    }
                                }
                                _ => {
                                    return Err(LuxError::runtime_error(
                                        "await table must contain only task IDs (integers)",
                                        Some(location.clone()),
                                    ));
                                }
                            }
                        }

                        // Collect field task IDs and spawn threads
                        for (key, value) in table.fields.iter() {
                            match value {
                                Value::Int(task_id) => {
                                    let tid = *task_id as usize;
                                    task_ids_fields.insert(key.clone(), tid);

                                    if let Some(task) = self.executor.get_task(tid) {
                                        if matches!(task.state, TaskState::Pending) {
                                            if let Some(func) = task.function {
                                                let args = task.arguments.clone();
                                                let env = self.env.clone();
                                                let executor = self.executor.clone();

                                                let handle = thread::spawn(move || {
                                                    let mut task_interp = Interpreter {
                                                        env,
                                                        control_flow: ControlFlow::None,
                                                        executor: executor.clone(),
                                                        loaded_modules: HashMap::new(),
                                                        current_file_dir: None,
                                                    };
                                                    task_interp.execute_task(tid, func, args)
                                                });
                                                handles.push((tid, handle));
                                            }
                                        }
                                    } else {
                                        return Err(LuxError::runtime_error(
                                            &format!("Task {} not found", task_id),
                                            Some(location.clone()),
                                        ));
                                    }
                                }
                                _ => {
                                    return Err(LuxError::runtime_error(
                                        "await table must contain only task IDs (integers)",
                                        Some(location.clone()),
                                    ));
                                }
                            }
                        }

                        // Wait for all threads to complete
                        for (_tid, handle) in handles {
                            if let Err(e) = handle.join() {
                                return Err(LuxError::runtime_error(
                                    &format!("Task thread panicked: {:?}", e),
                                    Some(location.clone()),
                                ));
                            }
                        }

                        // Collect results
                        let mut result_table = TableValue::new();

                        for tid in task_ids_array {
                            if let Some(task) = self.executor.get_task(tid) {
                                match task.state {
                                    TaskState::Completed(result) => {
                                        result_table.array.push(result);
                                    }
                                    TaskState::Failed(msg) => {
                                        return Err(LuxError::runtime_error(
                                            &format!("Task {} failed: {}", tid, msg),
                                            Some(location.clone()),
                                        ));
                                    }
                                    _ => {
                                        return Err(LuxError::runtime_error(
                                            &format!("Task {} did not complete", tid),
                                            Some(location.clone()),
                                        ));
                                    }
                                }
                            }
                        }

                        for (key, tid) in task_ids_fields {
                            if let Some(task) = self.executor.get_task(tid) {
                                match task.state {
                                    TaskState::Completed(result) => {
                                        result_table.fields.insert(key, result);
                                    }
                                    TaskState::Failed(msg) => {
                                        return Err(LuxError::runtime_error(
                                            &format!("Task {} failed: {}", tid, msg),
                                            Some(location.clone()),
                                        ));
                                    }
                                    _ => {
                                        return Err(LuxError::runtime_error(
                                            &format!("Task {} did not complete", tid),
                                            Some(location.clone()),
                                        ));
                                    }
                                }
                            }
                        }

                        // Return table of results
                        Ok(Value::Table(result_table))
                    }
                    _ => Err(LuxError::runtime_error(
                        "await expects a task ID (integer) or table of task IDs",
                        Some(location.clone()),
                    )),
                }
            }
        }
    }

    fn eval_binary(&self, left: Value, op: &BinaryOp, right: Value, location: &SourceLocation) -> LuxResult<Value> {
        match (left, right) {
            (Value::Int(a), Value::Int(b)) => {
                Ok(match op {
                    BinaryOp::Add => Value::Int(a + b),
                    BinaryOp::Subtract => Value::Int(a - b),
                    BinaryOp::Multiply => Value::Int(a * b),
                    BinaryOp::Divide => {
                        if b == 0 {
                            return Err(LuxError::runtime_error("Division by zero", Some(location.clone())));
                        }
                        Value::Int(a / b)
                    }
                    BinaryOp::Modulo => Value::Int(a % b),
                    BinaryOp::Equal => Value::Bool(a == b),
                    BinaryOp::NotEqual => Value::Bool(a != b),
                    BinaryOp::Less => Value::Bool(a < b),
                    BinaryOp::LessEqual => Value::Bool(a <= b),
                    BinaryOp::Greater => Value::Bool(a > b),
                    BinaryOp::GreaterEqual => Value::Bool(a >= b),
                })
            }
            (Value::Float(a), Value::Float(b)) => {
                Ok(match op {
                    BinaryOp::Add => Value::Float(a + b),
                    BinaryOp::Subtract => Value::Float(a - b),
                    BinaryOp::Multiply => Value::Float(a * b),
                    BinaryOp::Divide => Value::Float(a / b),
                    BinaryOp::Modulo => Value::Float(a % b),
                    BinaryOp::Equal => Value::Bool(a == b),
                    BinaryOp::NotEqual => Value::Bool(a != b),
                    BinaryOp::Less => Value::Bool(a < b),
                    BinaryOp::LessEqual => Value::Bool(a <= b),
                    BinaryOp::Greater => Value::Bool(a > b),
                    BinaryOp::GreaterEqual => Value::Bool(a >= b),
                })
            }
            (Value::String(a), Value::String(b)) => {
                Ok(match op {
                    BinaryOp::Add => Value::String(format!("{}{}", a, b)),
                    BinaryOp::Equal => Value::Bool(a == b),
                    BinaryOp::NotEqual => Value::Bool(a != b),
                    _ => return Err(LuxError::runtime_error(
                        format!("Unsupported operation {:?} for strings", op),
                        Some(location.clone()),
                    )),
                })
            }
            (a, b) => {
                if matches!(op, BinaryOp::Equal) {
                    Ok(Value::Bool(a == b))
                } else if matches!(op, BinaryOp::NotEqual) {
                    Ok(Value::Bool(a != b))
                } else {
                    Err(LuxError::runtime_error(
                        format!("Type mismatch: cannot apply {:?} to {} and {}", op, a.type_name(), b.type_name()),
                        Some(location.clone()),
                    ))
                }
            }
        }
    }

    fn eval_unary(&self, op: &UnaryOp, operand: Value, location: &SourceLocation) -> LuxResult<Value> {
        match op {
            UnaryOp::Negate => {
                match operand {
                    Value::Int(n) => Ok(Value::Int(-n)),
                    Value::Float(f) => Ok(Value::Float(-f)),
                    _ => Err(LuxError::runtime_error(
                        format!("Cannot negate {}", operand.type_name()),
                        Some(location.clone()),
                    )),
                }
            }
            UnaryOp::Not => Ok(Value::Bool(!operand.is_truthy())),
            UnaryOp::Length => {
                match operand {
                    Value::Table(t) => Ok(Value::Int(t.len() as i64)),
                    Value::String(s) => Ok(Value::Int(s.len() as i64)),
                    _ => Err(LuxError::runtime_error(
                        format!("Cannot get length of {}", operand.type_name()),
                        Some(location.clone()),
                    )),
                }
            }
            UnaryOp::AddressOf => {
                // Create a pointer to the value
                use std::sync::{Arc, Mutex};
                Ok(Value::Pointer(Arc::new(Mutex::new(operand))))
            }
            UnaryOp::Dereference => {
                // Dereference a pointer
                match operand {
                    Value::Pointer(ptr) => {
                        let guard = ptr.lock().map_err(|_| LuxError::runtime_error(
                            "Failed to lock pointer (poisoned mutex)".to_string(),
                            Some(location.clone()),
                        ))?;
                        Ok(guard.clone())
                    }
                    _ => Err(LuxError::runtime_error(
                        format!("Cannot dereference non-pointer type {}", operand.type_name()),
                        Some(location.clone()),
                    )),
                }
            }
        }
    }

    fn call_function(&mut self, func: Value, args: Vec<Value>, location: &SourceLocation) -> LuxResult<Value> {
        match func {
            Value::NativeFunction(native) => {
                if args.len() != native.arity {
                    return Err(LuxError::runtime_error(
                        format!("Expected {} arguments but got {}", native.arity, args.len()),
                        Some(location.clone()),
                    ));
                }
                (native.func)(&args).map_err(|e| {
                    LuxError::runtime_error(e, Some(location.clone()))
                })
            }
            Value::Function(user_func) => {
                if args.len() != user_func.params.len() {
                    return Err(LuxError::runtime_error(
                        format!("Expected {} arguments but got {}", user_func.params.len(), args.len()),
                        Some(location.clone()),
                    ));
                }

                // Create new scope for function
                self.env.push_scope();

                // Bind parameters
                for (param, arg) in user_func.params.iter().zip(args.iter()) {
                    self.env.define(param.clone(), arg.clone());
                }

                // Execute function body
                for stmt in &user_func.body {
                    self.execute_stmt(stmt)?;

                    if let ControlFlow::Return(value) = &self.control_flow {
                        let return_value = value.clone();
                        self.control_flow = ControlFlow::None;
                        self.env.pop_scope();
                        return Ok(return_value);
                    }
                }

                self.env.pop_scope();
                self.control_flow = ControlFlow::None;
                Ok(Value::Nil)
            }
            _ => Err(LuxError::runtime_error(
                format!("Cannot call {}", func.type_name()),
                Some(location.clone()),
            )),
        }
    }
}
