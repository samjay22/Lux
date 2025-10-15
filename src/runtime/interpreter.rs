//! Interpreter implementation
//!
//! This module implements the tree-walking interpreter for Lux.

use std::collections::HashMap;
use std::sync::Arc;
use crate::error::{LuxError, LuxResult, SourceLocation};
use crate::parser::ast::{Ast, Stmt, Expr, BinaryOp, UnaryOp, LogicalOp, Literal, TableKey};
use crate::async_runtime::{AsyncExecutor, TaskState};
use super::value::{Value, TableValue, FunctionValue, NativeFunctionValue};

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
}

impl Interpreter {
    pub fn new() -> Self {
        let mut interpreter = Self {
            env: Environment::new(),
            control_flow: ControlFlow::None,
            executor: Arc::new(AsyncExecutor::new()),
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

    fn execute_stmt(&mut self, stmt: &Stmt) -> LuxResult<()> {
        match stmt {
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
