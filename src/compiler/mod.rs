//! AOT/JIT Compiler using Cranelift
//!
//! This module compiles Lux code to native machine code

use cranelift::prelude::*;
use cranelift_jit::{JITBuilder, JITModule};
use cranelift_module::{Linkage, Module};
use std::collections::HashMap;
use crate::error::{LuxError, LuxResult};
use crate::parser::ast::{Stmt, Expr, BinaryOp, Literal};

/// Compiled function signature
type CompiledFunction = unsafe extern "C" fn() -> i64;

/// JIT Compiler for Lux
pub struct JITCompiler {
    builder_context: FunctionBuilderContext,
    ctx: codegen::Context,
    module: JITModule,
    variables: HashMap<String, Variable>,
    next_var: usize,
    /// Map of function names to their compiled function IDs
    function_ids: HashMap<String, cranelift_module::FuncId>,
    /// Map of native function names to their pointers
    native_functions: HashMap<String, *const u8>,
}

impl JITCompiler {
    pub fn new() -> Self {
        let mut flag_builder = settings::builder();
        flag_builder.set("use_colocated_libcalls", "false").unwrap();
        flag_builder.set("is_pic", "false").unwrap();
        flag_builder.set("opt_level", "speed").unwrap();
        
        let isa_builder = cranelift_native::builder().unwrap_or_else(|msg| {
            panic!("host machine is not supported: {}", msg);
        });
        let isa = isa_builder
            .finish(settings::Flags::new(flag_builder))
            .unwrap();
        
        let mut builder = JITBuilder::with_isa(isa, cranelift_module::default_libcall_names());
        
        let module = JITModule::new(builder);

        Self {
            builder_context: FunctionBuilderContext::new(),
            ctx: module.make_context(),
            module,
            variables: HashMap::new(),
            next_var: 0,
            function_ids: HashMap::new(),
            native_functions: HashMap::new(),
        }
    }

    /// Register a native function (like print) that can be called from JIT code
    pub fn register_native_function(&mut self, name: &str, ptr: *const u8) {
        self.native_functions.insert(name.to_string(), ptr);
    }

    /// Declare a function signature (for recursive calls)
    fn declare_function_signature(&mut self, name: &str, param_count: usize) -> LuxResult<cranelift_module::FuncId> {
        // Check if already declared
        if let Some(&func_id) = self.function_ids.get(name) {
            return Ok(func_id);
        }

        // Create signature: (i64, i64, i64, i64, i64) -> i64
        // We always use 5 parameters for simplicity (pad with zeros)
        let mut sig = self.module.make_signature();
        for _ in 0..5 {
            sig.params.push(AbiParam::new(types::I64));
        }
        sig.returns.push(AbiParam::new(types::I64));

        // Declare the function
        let func_id = self.module
            .declare_function(name, Linkage::Local, &sig)
            .map_err(|e| LuxError::runtime_error(format!("Failed to declare function: {}", e), None))?;

        self.function_ids.insert(name.to_string(), func_id);
        Ok(func_id)
    }

    /// Compile a Lux function to native code
    /// params: list of (param_name, param_type) - currently only supports int parameters
    /// jit_functions: map of already-compiled functions that can be called
    pub fn compile_function(&mut self, name: &str, params: &[(String, String)], body: &[Stmt], jit_functions: &std::collections::HashMap<String, (*const u8, usize, bool)>) -> LuxResult<*const u8> {
        // First, declare the function so it can be called recursively
        let func_id = self.declare_function_signature(name, params.len())?;

        // Clear previous function signature
        self.ctx.func.signature.params.clear();
        self.ctx.func.signature.returns.clear();

        // Create function signature
        // Add parameters (all i64 for now)
        for _ in params {
            self.ctx.func.signature.params.push(AbiParam::new(types::I64));
        }
        // Return type: i64
        self.ctx.func.signature.returns.push(AbiParam::new(types::I64));

        // Create function builder
        let mut builder = FunctionBuilder::new(&mut self.ctx.func, &mut self.builder_context);

        // Create entry block and exit block
        let entry_block = builder.create_block();
        let exit_block = builder.create_block();

        // Add block parameters for function parameters
        for _ in params {
            builder.append_block_param(entry_block, types::I64);
        }

        // Exit block has one parameter: the return value
        builder.append_block_param(exit_block, types::I64);

        builder.switch_to_block(entry_block);
        builder.seal_block(entry_block);

        // Map parameters to variables
        let mut vars = HashMap::new();
        let mut next_var_idx = 0;

        for (i, (param_name, _)) in params.iter().enumerate() {
            let var = Variable::new(i);
            builder.declare_var(var, types::I64);
            let param_value = builder.block_params(entry_block)[i];
            builder.def_var(var, param_value);
            vars.insert(param_name.clone(), var);
            next_var_idx = i + 1;
        }

        // Compile function body
        let mut current_block_sealed = false;
        for stmt in body {
            if current_block_sealed {
                // Previous statement was a return, create new unreachable block
                let unreachable_block = builder.create_block();
                builder.switch_to_block(unreachable_block);
                builder.seal_block(unreachable_block);
                current_block_sealed = false;
            }
            let is_return = matches!(stmt, Stmt::Return { .. });
            Self::compile_stmt(&mut builder, stmt, &mut vars, &mut next_var_idx, exit_block, &self.module, &self.function_ids, jit_functions)?;
            if is_return {
                current_block_sealed = true;
            }
        }

        // If we reach here without returning, return 0
        if !current_block_sealed {
            let default_return = builder.ins().iconst(types::I64, 0);
            builder.ins().jump(exit_block, &[default_return]);
        }

        // Exit block: return the value
        builder.switch_to_block(exit_block);
        builder.seal_block(exit_block);
        let return_value = builder.block_params(exit_block)[0];
        builder.ins().return_(&[return_value]);
        
        // Finalize function
        builder.finalize();
        
        // Declare function in module
        let id = self.module
            .declare_function(name, Linkage::Export, &self.ctx.func.signature)
            .map_err(|e| LuxError::runtime_error(format!("Failed to declare function: {}", e), None))?;
        
        // Define function
        self.module
            .define_function(id, &mut self.ctx)
            .map_err(|e| LuxError::runtime_error(format!("Failed to define function: {}", e), None))?;
        
        // Clear context for next function
        self.module.clear_context(&mut self.ctx);

        // Create fresh context and builder context for next function
        self.ctx = self.module.make_context();
        self.builder_context = FunctionBuilderContext::new();
        
        // Finalize module
        self.module.finalize_definitions().unwrap();
        
        // Get function pointer
        let code = self.module.get_finalized_function(id);
        
        Ok(code)
    }
    
    fn compile_stmt(builder: &mut FunctionBuilder, stmt: &Stmt, vars: &mut HashMap<String, Variable>, next_var_idx: &mut usize, exit_block: Block, module: &JITModule, function_ids: &HashMap<String, cranelift_module::FuncId>, jit_functions: &std::collections::HashMap<String, (*const u8, usize, bool)>) -> LuxResult<()> {
        match stmt {
            Stmt::Return { value, .. } => {
                let val = if let Some(expr) = value {
                    Self::compile_expr(builder, expr, vars, module, function_ids, jit_functions)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };
                // Jump to exit block with return value
                builder.ins().jump(exit_block, &[val]);
                Ok(())
            }
            Stmt::Expression { expr, .. } => {
                Self::compile_expr(builder, expr, vars, module, function_ids, jit_functions)?;
                Ok(())
            }
            Stmt::VarDecl { name, initializer, .. } => {
                let value = if let Some(init) = initializer {
                    Self::compile_expr(builder, init, vars, module, function_ids, jit_functions)?
                } else {
                    builder.ins().iconst(types::I64, 0)
                };

                // Create variable with unique index
                let var = Variable::new(*next_var_idx);
                *next_var_idx += 1;
                builder.declare_var(var, types::I64);
                builder.def_var(var, value);
                vars.insert(name.clone(), var);

                Ok(())
            }
            Stmt::While { condition, body, .. } => {
                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let while_exit_block = builder.create_block();

                // Jump to header
                builder.ins().jump(header_block, &[]);

                // Header: check condition
                builder.switch_to_block(header_block);
                let cond = Self::compile_expr(builder, condition, vars, module, function_ids, jit_functions)?;
                builder.ins().brif(cond, body_block, &[], while_exit_block, &[]);

                // Body: execute statements
                builder.switch_to_block(body_block);
                for stmt in body {
                    Self::compile_stmt(builder, stmt, vars, next_var_idx, exit_block, module, function_ids, jit_functions)?;
                }
                builder.ins().jump(header_block, &[]);

                // Exit
                builder.switch_to_block(while_exit_block);
                builder.seal_block(header_block);
                builder.seal_block(body_block);
                builder.seal_block(while_exit_block);

                Ok(())
            }
            Stmt::If { condition, then_branch, else_branch, .. } => {
                let then_block = builder.create_block();
                let else_block = builder.create_block();
                let merge_block = builder.create_block();

                let cond = Self::compile_expr(builder, condition, vars, module, function_ids, jit_functions)?;
                builder.ins().brif(cond, then_block, &[], else_block, &[]);

                // Then branch
                builder.switch_to_block(then_block);
                let mut then_has_return = false;
                for stmt in then_branch {
                    if matches!(stmt, Stmt::Return { .. }) {
                        then_has_return = true;
                    }
                    Self::compile_stmt(builder, stmt, vars, next_var_idx, exit_block, module, function_ids, jit_functions)?;
                }
                if !then_has_return {
                    builder.ins().jump(merge_block, &[]);
                }

                // Else branch
                builder.switch_to_block(else_block);
                let mut else_has_return = false;
                if let Some(else_stmts) = else_branch {
                    for stmt in else_stmts {
                        if matches!(stmt, Stmt::Return { .. }) {
                            else_has_return = true;
                        }
                        Self::compile_stmt(builder, stmt, vars, next_var_idx, exit_block, module, function_ids, jit_functions)?;
                    }
                }
                if !else_has_return {
                    builder.ins().jump(merge_block, &[]);
                }

                // Merge
                builder.switch_to_block(merge_block);
                builder.seal_block(then_block);
                builder.seal_block(else_block);
                builder.seal_block(merge_block);

                Ok(())
            }
            Stmt::For { initializer, condition, increment, body, .. } => {
                // For loop is: initializer; while(condition) { body; increment; }

                // Execute initializer
                if let Some(init) = initializer {
                    Self::compile_stmt(builder, init, vars, next_var_idx, exit_block, module, function_ids, jit_functions)?;
                }

                let header_block = builder.create_block();
                let body_block = builder.create_block();
                let for_exit_block = builder.create_block();

                // Jump to header
                builder.ins().jump(header_block, &[]);

                // Header: check condition
                builder.switch_to_block(header_block);
                if let Some(cond_expr) = condition {
                    let cond = Self::compile_expr(builder, cond_expr, vars, module, function_ids, jit_functions)?;
                    builder.ins().brif(cond, body_block, &[], for_exit_block, &[]);
                } else {
                    // No condition means infinite loop
                    builder.ins().jump(body_block, &[]);
                }

                // Body: execute statements
                builder.switch_to_block(body_block);
                for stmt in body {
                    Self::compile_stmt(builder, stmt, vars, next_var_idx, exit_block, module, function_ids, jit_functions)?;
                }

                // Increment
                if let Some(inc_expr) = increment {
                    Self::compile_expr(builder, inc_expr, vars, module, function_ids, jit_functions)?;
                }

                builder.ins().jump(header_block, &[]);

                // Exit
                builder.switch_to_block(for_exit_block);
                builder.seal_block(header_block);
                builder.seal_block(body_block);
                builder.seal_block(for_exit_block);

                Ok(())
            }
            _ => Err(LuxError::runtime_error(format!("Unsupported statement in JIT compilation: {:?}", stmt), None)),
        }
    }

    fn compile_expr(builder: &mut FunctionBuilder, expr: &Expr, vars: &HashMap<String, Variable>, module: &JITModule, function_ids: &HashMap<String, cranelift_module::FuncId>, jit_functions: &std::collections::HashMap<String, (*const u8, usize, bool)>) -> LuxResult<cranelift::prelude::Value> {
        match expr {
            Expr::Literal { value, .. } => {
                match value {
                    Literal::Integer(n) => Ok(builder.ins().iconst(types::I64, *n)),
                    Literal::Boolean(b) => Ok(builder.ins().iconst(types::I64, if *b { 1 } else { 0 })),
                    _ => Err(LuxError::runtime_error("Unsupported literal type in AOT compilation".to_string(), None)),
                }
            }

            Expr::Variable { name, .. } => {
                if let Some(&var) = vars.get(name) {
                    Ok(builder.use_var(var))
                } else {
                    Err(LuxError::runtime_error(format!("Undefined variable: {}", name), None))
                }
            }

            Expr::Assign { target, value, .. } => {
                if let Expr::Variable { name, .. } = target.as_ref() {
                    let val = Self::compile_expr(builder, value, vars, module, function_ids, jit_functions)?;
                    if let Some(&var) = vars.get(name) {
                        builder.def_var(var, val);
                    }
                    Ok(val)
                } else {
                    Err(LuxError::runtime_error("Unsupported assignment target in AOT compilation".to_string(), None))
                }
            }

            Expr::Binary { left, operator, right, .. } => {
                let lhs = Self::compile_expr(builder, left, vars, module, function_ids, jit_functions)?;
                let rhs = Self::compile_expr(builder, right, vars, module, function_ids, jit_functions)?;

                Ok(match operator {
                    BinaryOp::Add => builder.ins().iadd(lhs, rhs),
                    BinaryOp::Subtract => builder.ins().isub(lhs, rhs),
                    BinaryOp::Multiply => builder.ins().imul(lhs, rhs),
                    BinaryOp::Divide => builder.ins().sdiv(lhs, rhs),
                    BinaryOp::Modulo => builder.ins().srem(lhs, rhs),
                    BinaryOp::Equal => {
                        let cmp = builder.ins().icmp(IntCC::Equal, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::NotEqual => {
                        let cmp = builder.ins().icmp(IntCC::NotEqual, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::Less => {
                        let cmp = builder.ins().icmp(IntCC::SignedLessThan, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::LessEqual => {
                        let cmp = builder.ins().icmp(IntCC::SignedLessThanOrEqual, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::Greater => {
                        let cmp = builder.ins().icmp(IntCC::SignedGreaterThan, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    BinaryOp::GreaterEqual => {
                        let cmp = builder.ins().icmp(IntCC::SignedGreaterThanOrEqual, lhs, rhs);
                        builder.ins().uextend(types::I64, cmp)
                    }
                    _ => return Err(LuxError::runtime_error("Unsupported binary operator in AOT compilation".to_string(), None)),
                })
            }

            Expr::Call { callee, arguments, .. } => {
                // For now, we'll handle specific built-in functions
                if let Expr::Variable { name, .. } = callee.as_ref() {
                    match name.as_str() {
                        "print" => {
                            // For print, we'll just evaluate the argument and return it
                            // (actual printing would require FFI which is complex)
                            if arguments.len() == 1 {
                                Self::compile_expr(builder, &arguments[0], vars, module, function_ids, jit_functions)
                            } else {
                                Ok(builder.ins().iconst(types::I64, 0))
                            }
                        }
                        _ => {
                            // Recursive calls not yet supported
                            Err(LuxError::runtime_error(format!("Function calls to '{}' not yet supported in JIT (recursive calls coming soon)", name), None))
                        }
                    }
                } else {
                    Err(LuxError::runtime_error("Complex function calls not yet supported in JIT".to_string(), None))
                }
            }

            _ => Err(LuxError::runtime_error("Unsupported expression in AOT compilation".to_string(), None)),
        }
    }
}

