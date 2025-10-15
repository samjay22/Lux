//! Type checker implementation
//!
//! This module implements type checking for Lux.

use std::collections::HashMap;
use crate::error::{LuxError, LuxResult};
use crate::parser::ast::{Ast, Stmt, Expr, Type, BinaryOp, UnaryOp, Literal};

/// Type environment for tracking variable types
#[derive(Debug, Clone)]
struct TypeEnvironment {
    scopes: Vec<HashMap<String, Type>>,
}

impl TypeEnvironment {
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

    fn define(&mut self, name: String, typ: Type) {
        if let Some(scope) = self.scopes.last_mut() {
            scope.insert(name, typ);
        }
    }

    fn get(&self, name: &str) -> Option<Type> {
        for scope in self.scopes.iter().rev() {
            if let Some(typ) = scope.get(name) {
                return Some(typ.clone());
            }
        }
        None
    }
}

/// Type checker
pub struct TypeChecker {
    env: TypeEnvironment,
    current_function_return_type: Option<Type>,
}

impl TypeChecker {
    pub fn new() -> Self {
        let mut env = TypeEnvironment::new();

        // Register built-in functions
        // print(value) -> nil
        env.define(
            "print".to_string(),
            Type::Function {
                params: vec![Type::Nil], // Accept any type (we use Nil as placeholder)
                return_type: Box::new(Type::Nil),
            },
        );

        // setmetatable(table, metatable) -> table
        env.define(
            "setmetatable".to_string(),
            Type::Function {
                params: vec![Type::Table, Type::Table],
                return_type: Box::new(Type::Table),
            },
        );

        // getmetatable(table) -> table | nil
        env.define(
            "getmetatable".to_string(),
            Type::Function {
                params: vec![Type::Table],
                return_type: Box::new(Type::Nil), // Can return table or nil
            },
        );

        Self {
            env,
            current_function_return_type: None,
        }
    }

    /// Type check an entire AST
    pub fn check(&mut self, ast: &Ast) -> LuxResult<()> {
        for stmt in &ast.statements {
            self.check_stmt(stmt)?;
        }
        Ok(())
    }

    /// Check a statement
    fn check_stmt(&mut self, stmt: &Stmt) -> LuxResult<()> {
        match stmt {
            Stmt::VarDecl { name, type_annotation, initializer, location, .. } => {
                let init_type = if let Some(init) = initializer {
                    Some(self.check_expr(init)?)
                } else {
                    None
                };

                let var_type = match (type_annotation, init_type) {
                    (Some(annotated), Some(init)) => {
                        // Both annotation and initializer - check compatibility
                        if !self.types_compatible(annotated, &init) {
                            return Err(LuxError::type_error(
                                format!(
                                    "Type mismatch: variable '{}' declared as {:?} but initialized with {:?}",
                                    name, annotated, init
                                ),
                                location.clone(),
                            ));
                        }
                        annotated.clone()
                    }
                    (Some(annotated), None) => {
                        // Only annotation
                        annotated.clone()
                    }
                    (None, Some(init)) => {
                        // Only initializer - infer type
                        init
                    }
                    (None, None) => {
                        return Err(LuxError::type_error(
                            format!("Variable '{}' must have either a type annotation or an initializer", name),
                            location.clone(),
                        ));
                    }
                };

                self.env.define(name.clone(), var_type);
                Ok(())
            }

            Stmt::FunctionDecl { name, params, return_type, body, location, .. } => {
                // Define function type in environment
                let func_type = Type::Function {
                    params: params.iter().map(|(_, t)| t.clone()).collect(),
                    return_type: Box::new(return_type.clone().unwrap_or(Type::Nil)),
                };
                self.env.define(name.clone(), func_type);

                // Check function body in new scope
                self.env.push_scope();

                // Define parameters
                for (param_name, param_type) in params {
                    self.env.define(param_name.clone(), param_type.clone());
                }

                // Set current function return type
                let prev_return_type = self.current_function_return_type.clone();
                self.current_function_return_type = return_type.clone();

                // Check body
                for stmt in body {
                    self.check_stmt(stmt)?;
                }

                // Restore previous return type
                self.current_function_return_type = prev_return_type;

                self.env.pop_scope();
                Ok(())
            }

            Stmt::Expression { expr, .. } => {
                self.check_expr(expr)?;
                Ok(())
            }

            Stmt::If { condition, then_branch, else_branch, location } => {
                let cond_type = self.check_expr(condition)?;
                // Condition can be any type (truthy/falsy semantics)

                // Check then branch
                self.env.push_scope();
                for stmt in then_branch {
                    self.check_stmt(stmt)?;
                }
                self.env.pop_scope();

                // Check else branch
                if let Some(else_stmts) = else_branch {
                    self.env.push_scope();
                    for stmt in else_stmts {
                        self.check_stmt(stmt)?;
                    }
                    self.env.pop_scope();
                }

                Ok(())
            }

            Stmt::While { condition, body, .. } => {
                self.check_expr(condition)?;

                self.env.push_scope();
                for stmt in body {
                    self.check_stmt(stmt)?;
                }
                self.env.pop_scope();

                Ok(())
            }

            Stmt::For { initializer, condition, increment, body, .. } => {
                self.env.push_scope();

                if let Some(init) = initializer {
                    self.check_stmt(init)?;
                }

                if let Some(cond) = condition {
                    self.check_expr(cond)?;
                }

                if let Some(inc) = increment {
                    self.check_expr(inc)?;
                }

                for stmt in body {
                    self.check_stmt(stmt)?;
                }

                self.env.pop_scope();
                Ok(())
            }

            Stmt::Return { value, location } => {
                let return_type = if let Some(val) = value {
                    self.check_expr(val)?
                } else {
                    Type::Nil
                };

                if let Some(expected) = &self.current_function_return_type {
                    // Allow Nil (unknown type) to match any expected return type
                    if !matches!(return_type, Type::Nil) && !self.types_compatible(expected, &return_type) {
                        return Err(LuxError::type_error(
                            format!(
                                "Return type mismatch: expected {:?}, got {:?}",
                                expected, return_type
                            ),
                            location.clone(),
                        ));
                    }
                }

                Ok(())
            }

            Stmt::Break { .. } | Stmt::Continue { .. } => Ok(()),

            Stmt::Block { statements, .. } => {
                self.env.push_scope();
                for stmt in statements {
                    self.check_stmt(stmt)?;
                }
                self.env.pop_scope();
                Ok(())
            }
        }
    }

    /// Check an expression and return its type
    fn check_expr(&mut self, expr: &Expr) -> LuxResult<Type> {
        match expr {
            Expr::Literal { value, .. } => {
                Ok(match value {
                    Literal::Integer(_) => Type::Int,
                    Literal::Float(_) => Type::Float,
                    Literal::String(_) => Type::String,
                    Literal::Boolean(_) => Type::Bool,
                    Literal::Nil => Type::Nil,
                })
            }

            Expr::Variable { name, location } => {
                self.env.get(name).ok_or_else(|| {
                    LuxError::type_error(
                        format!("Undefined variable '{}'", name),
                        location.clone(),
                    )
                })
            }

            Expr::Binary { left, operator, right, location } => {
                let left_type = self.check_expr(left)?;
                let right_type = self.check_expr(right)?;

                // If either operand is Nil (unknown type from table access), be lenient
                if matches!(left_type, Type::Nil) || matches!(right_type, Type::Nil) {
                    // Unknown type - allow operation and infer result type
                    return Ok(match operator {
                        BinaryOp::Equal | BinaryOp::NotEqual |
                        BinaryOp::Less | BinaryOp::LessEqual |
                        BinaryOp::Greater | BinaryOp::GreaterEqual => Type::Bool,
                        _ => Type::Nil, // Unknown result type
                    });
                }

                match operator {
                    BinaryOp::Add => {
                        // Add works for int + int, float + float, string + string
                        if self.types_compatible(&left_type, &right_type) {
                            match left_type {
                                Type::Int | Type::Float | Type::String => Ok(left_type),
                                _ => Err(LuxError::type_error(
                                    format!("Cannot add {:?} and {:?}", left_type, right_type),
                                    location.clone(),
                                )),
                            }
                        } else {
                            Err(LuxError::type_error(
                                format!("Type mismatch: cannot add {:?} and {:?}", left_type, right_type),
                                location.clone(),
                            ))
                        }
                    }

                    BinaryOp::Subtract | BinaryOp::Multiply | BinaryOp::Divide | BinaryOp::Modulo => {
                        // Arithmetic operations work for int and float
                        if !matches!(left_type, Type::Int | Type::Float) {
                            return Err(LuxError::type_error(
                                format!("Cannot apply {:?} to {:?}", operator, left_type),
                                location.clone(),
                            ));
                        }
                        if !matches!(right_type, Type::Int | Type::Float) {
                            return Err(LuxError::type_error(
                                format!("Cannot apply {:?} to {:?}", operator, right_type),
                                location.clone(),
                            ));
                        }
                        if self.types_compatible(&left_type, &right_type) {
                            Ok(left_type)
                        } else {
                            Err(LuxError::type_error(
                                format!("Type mismatch: {:?} and {:?}", left_type, right_type),
                                location.clone(),
                            ))
                        }
                    }

                    BinaryOp::Equal | BinaryOp::NotEqual => {
                        // Comparison works for any types
                        Ok(Type::Bool)
                    }

                    BinaryOp::Less | BinaryOp::LessEqual | BinaryOp::Greater | BinaryOp::GreaterEqual => {
                        // Ordering comparisons work for int and float
                        if !matches!(left_type, Type::Int | Type::Float) {
                            return Err(LuxError::type_error(
                                format!("Cannot compare {:?}", left_type),
                                location.clone(),
                            ));
                        }
                        if !matches!(right_type, Type::Int | Type::Float) {
                            return Err(LuxError::type_error(
                                format!("Cannot compare {:?}", right_type),
                                location.clone(),
                            ));
                        }
                        Ok(Type::Bool)
                    }
                }
            }

            Expr::Unary { operator, operand, location } => {
                let operand_type = self.check_expr(operand)?;

                match operator {
                    UnaryOp::Negate => {
                        if matches!(operand_type, Type::Int | Type::Float) {
                            Ok(operand_type)
                        } else {
                            Err(LuxError::type_error(
                                format!("Cannot negate {:?}", operand_type),
                                location.clone(),
                            ))
                        }
                    }
                    UnaryOp::Not => {
                        // Not works on any type (truthy/falsy)
                        Ok(Type::Bool)
                    }
                    UnaryOp::Length => {
                        // Length works on strings and tables
                        if matches!(operand_type, Type::String | Type::Table) {
                            Ok(Type::Int)
                        } else {
                            Err(LuxError::type_error(
                                format!("Cannot get length of {:?}", operand_type),
                                location.clone(),
                            ))
                        }
                    }
                }
            }

            Expr::Logical { left, operator, right, .. } => {
                self.check_expr(left)?;
                self.check_expr(right)?;
                // Logical operators work on any type (truthy/falsy)
                // Return type is bool
                Ok(Type::Bool)
            }

            Expr::Assign { target, value, location } => {
                let name = target;
                let var_type = self.env.get(name).ok_or_else(|| {
                    LuxError::type_error(
                        format!("Undefined variable '{}'", name),
                        location.clone(),
                    )
                })?;

                let value_type = self.check_expr(value)?;

                // Allow Nil (unknown type) to be assigned to any variable
                if !matches!(value_type, Type::Nil) && !self.types_compatible(&var_type, &value_type) {
                    return Err(LuxError::type_error(
                        format!(
                            "Type mismatch: cannot assign {:?} to variable of type {:?}",
                            value_type, var_type
                        ),
                        location.clone(),
                    ));
                }

                Ok(value_type)
            }

            Expr::Call { callee, arguments, location } => {
                let func_type = self.check_expr(callee)?;

                match func_type {
                    Type::Function { params, return_type } => {
                        // Check argument count (but be lenient for built-ins that use Nil as "any")
                        // If params has a single Nil, it means "accepts any number of any type" (built-in)
                        let is_builtin = params.len() == 1 && params[0] == Type::Nil;

                        if !is_builtin && arguments.len() != params.len() {
                            return Err(LuxError::type_error(
                                format!(
                                    "Function expects {} arguments, got {}",
                                    params.len(),
                                    arguments.len()
                                ),
                                location.clone(),
                            ));
                        }

                        // Check argument types (skip for built-ins)
                        if !is_builtin {
                            for (i, (arg, expected_type)) in arguments.iter().zip(params.iter()).enumerate() {
                                let arg_type = self.check_expr(arg)?;
                                // Allow Nil (unknown type) to match any expected type
                                if !matches!(arg_type, Type::Nil) && !self.types_compatible(expected_type, &arg_type) {
                                    return Err(LuxError::type_error(
                                        format!(
                                            "Argument {} type mismatch: expected {:?}, got {:?}",
                                            i + 1,
                                            expected_type,
                                            arg_type
                                        ),
                                        location.clone(),
                                    ));
                                }
                            }
                        } else {
                            // For built-ins, just check that arguments are valid expressions
                            for arg in arguments {
                                self.check_expr(arg)?;
                            }
                        }

                        Ok(*return_type)
                    }
                    _ => {
                        // For now, allow calling non-function types (built-ins, etc.)
                        // Return unknown type as Nil
                        Ok(Type::Nil)
                    }
                }
            }

            Expr::Table { fields, .. } => {
                // Check all field values
                for (key, value) in fields {
                    self.check_expr(value)?;
                }
                Ok(Type::Table)
            }

            Expr::TableAccess { table, key, location } => {
                let table_type = self.check_expr(table)?;
                self.check_expr(key)?;

                // Allow indexing on Table or Nil (unknown type)
                if !matches!(table_type, Type::Table | Type::Nil) {
                    return Err(LuxError::type_error(
                        format!("Cannot index {:?}", table_type),
                        location.clone(),
                    ));
                }

                // Table indexing can return any type
                Ok(Type::Nil)
            }

            Expr::Function { params, return_type, body, .. } => {
                // Function expression type
                let func_type = Type::Function {
                    params: params.iter().map(|(_, t)| t.clone()).collect(),
                    return_type: Box::new(return_type.clone().unwrap_or(Type::Nil)),
                };

                // Check function body
                self.env.push_scope();

                for (param_name, param_type) in params {
                    self.env.define(param_name.clone(), param_type.clone());
                }

                let prev_return_type = self.current_function_return_type.clone();
                self.current_function_return_type = return_type.clone();

                for stmt in body {
                    self.check_stmt(stmt)?;
                }

                self.current_function_return_type = prev_return_type;
                self.env.pop_scope();

                Ok(func_type)
            }

            Expr::Spawn { call, location } => {
                // Spawn expects a function call
                self.check_expr(call)?;
                // Returns task ID (int)
                Ok(Type::Int)
            }

            Expr::Await { task, location } => {
                let task_type = self.check_expr(task)?;
                // Await accepts either a single task ID (int) or a table of task IDs
                if !matches!(task_type, Type::Int | Type::Table | Type::Nil) {
                    return Err(LuxError::type_error(
                        format!("await expects task ID (int) or table of task IDs, got {:?}", task_type),
                        location.clone(),
                    ));
                }
                // Await can return any type (we don't know the task's return type)
                // If awaiting a table, it returns a table of results
                // If awaiting a single task, it returns the task's result
                Ok(Type::Nil)
            }
        }
    }

    /// Check if two types are compatible
    fn types_compatible(&self, expected: &Type, actual: &Type) -> bool {
        match (expected, actual) {
            (Type::Int, Type::Int) => true,
            (Type::Float, Type::Float) => true,
            (Type::String, Type::String) => true,
            (Type::Bool, Type::Bool) => true,
            (Type::Nil, Type::Nil) => true,
            (Type::Table, Type::Table) => true,
            (Type::Function { .. }, Type::Function { .. }) => {
                // For now, accept any function type
                // TODO: Check parameter and return types
                true
            }
            _ => false,
        }
    }
}

