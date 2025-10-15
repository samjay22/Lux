//! # Lux Programming Language
//!
//! A custom programming language with:
//! - Lua-like syntax (simple and clean)
//! - Go-like static typing (explicit types with inference)
//! - Built-in async/await support (goroutine-style concurrency)
//!
//! ## Architecture
//!
//! The language implementation is organized into several modules:
//! - `lexer`: Tokenization of source code
//! - `parser`: Parsing tokens into an Abstract Syntax Tree (AST)
//! - `types`: Type system and type checking
//! - `semantic`: Semantic analysis and validation
//! - `runtime`: Interpreter/execution engine
//! - `async_runtime`: Async task execution (future)
//! - `error`: Error handling and diagnostics

pub mod error;
pub mod lexer;
pub mod parser;
pub mod types;
pub mod runtime;
pub mod async_runtime;

// Re-export commonly used types
pub use error::{LuxError, LuxResult, SourceLocation};
pub use lexer::{Token, TokenType, Lexer};
pub use parser::{Parser, Ast};

/// Version of the Lux language
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

/// Compile and run a Lux program from source code
///
/// This is the main entry point for executing Lux programs.
/// It performs lexical analysis, parsing, type checking, semantic analysis,
/// and finally interpretation.
///
/// # Arguments
///
/// * `source` - The source code to compile and run
/// * `filename` - Optional filename for error reporting
///
/// # Returns
///
/// Returns `Ok(())` if the program executes successfully, or a `LuxError` if
/// any stage of compilation or execution fails.
pub fn run(source: &str, filename: Option<&str>) -> LuxResult<()> {
    // Phase 1: Lexical Analysis
    let mut lexer = Lexer::new(source, filename);
    let tokens = lexer.tokenize()?;

    // Phase 2: Parsing
    let ast = Parser::new(tokens).parse()?;

    // Phase 3: Type Checking
    let mut type_checker = types::TypeChecker::new();
    type_checker.check(&ast)?;

    // Phase 4: Semantic Analysis (to be implemented)
    // let validated_ast = SemanticAnalyzer::analyze(typed_ast)?;

    // Phase 5: Interpretation
    let mut interpreter = runtime::Interpreter::new();
    interpreter.interpret(&ast)?;

    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_version() {
        assert!(!VERSION.is_empty());
    }
}

