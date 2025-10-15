//! Runtime value representation
//!
//! This module defines runtime values for Lux.

use std::collections::HashMap;
use std::fmt;
use std::sync::{Arc, Mutex};

/// Runtime value
#[derive(Debug, Clone)]
pub enum Value {
    Int(i64),
    Float(f64),
    String(String),
    Bool(bool),
    Nil,
    Table(TableValue),
    Function(FunctionValue),
    NativeFunction(NativeFunctionValue),
    Pointer(Arc<Mutex<Value>>),
}

/// Table value (Lua-style associative array)
#[derive(Debug, Clone)]
pub struct TableValue {
    pub fields: HashMap<String, Value>,
    pub array: Vec<Value>,
    pub metatable: Option<Box<TableValue>>,
}

impl TableValue {
    pub fn new() -> Self {
        Self {
            fields: HashMap::new(),
            array: Vec::new(),
            metatable: None,
        }
    }

    pub fn get(&self, key: &Value) -> Option<Value> {
        match key {
            Value::Int(n) if *n > 0 => {
                let index = (*n - 1) as usize;
                self.array.get(index).cloned()
            }
            Value::String(s) => self.fields.get(s).cloned(),
            _ => None,
        }
    }

    pub fn set(&mut self, key: Value, value: Value) {
        match key {
            Value::Int(n) if n > 0 => {
                let index = (n - 1) as usize;
                if index >= self.array.len() {
                    self.array.resize(index + 1, Value::Nil);
                }
                self.array[index] = value;
            }
            Value::String(s) => {
                self.fields.insert(s, value);
            }
            _ => {}
        }
    }

    pub fn len(&self) -> usize {
        self.array.len()
    }
}

/// Function value
#[derive(Debug, Clone)]
pub struct FunctionValue {
    pub name: String,
    pub params: Vec<String>,
    pub body: Vec<crate::parser::Stmt>,
    pub is_async: bool,
}

/// Native function value (built-in functions)
#[derive(Clone)]
pub struct NativeFunctionValue {
    pub name: String,
    pub arity: usize,
    pub func: fn(&[Value]) -> Result<Value, String>,
}

impl fmt::Debug for NativeFunctionValue {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "<native fn {}>", self.name)
    }
}

impl Value {
    pub fn is_truthy(&self) -> bool {
        match self {
            Value::Nil => false,
            Value::Bool(b) => *b,
            _ => true,
        }
    }

    pub fn type_name(&self) -> &str {
        match self {
            Value::Int(_) => "int",
            Value::Float(_) => "float",
            Value::String(_) => "string",
            Value::Bool(_) => "bool",
            Value::Nil => "nil",
            Value::Table(_) => "table",
            Value::Function(_) => "function",
            Value::NativeFunction(_) => "function",
            Value::Pointer(_) => "pointer",
        }
    }
}

impl fmt::Display for Value {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Value::Int(n) => write!(f, "{}", n),
            Value::Float(fl) => write!(f, "{}", fl),
            Value::String(s) => write!(f, "{}", s),
            Value::Bool(b) => write!(f, "{}", b),
            Value::Nil => write!(f, "nil"),
            Value::Table(t) => {
                if t.array.is_empty() && t.fields.is_empty() {
                    write!(f, "{{}}")
                } else if t.fields.is_empty() {
                    write!(f, "[")?;
                    for (i, v) in t.array.iter().enumerate() {
                        if i > 0 {
                            write!(f, ", ")?;
                        }
                        write!(f, "{}", v)?;
                    }
                    write!(f, "]")
                } else {
                    write!(f, "{{...}}")
                }
            }
            Value::Function(func) => write!(f, "<fn {}>", func.name),
            Value::NativeFunction(func) => write!(f, "<native fn {}>", func.name),
            Value::Pointer(ptr) => {
                if let Ok(guard) = ptr.lock() {
                    write!(f, "<pointer to {}>", guard.type_name())
                } else {
                    write!(f, "<pointer (locked)>")
                }
            }
        }
    }
}

impl PartialEq for Value {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Value::Int(a), Value::Int(b)) => a == b,
            (Value::Float(a), Value::Float(b)) => a == b,
            (Value::String(a), Value::String(b)) => a == b,
            (Value::Bool(a), Value::Bool(b)) => a == b,
            (Value::Nil, Value::Nil) => true,
            _ => false,
        }
    }
}

