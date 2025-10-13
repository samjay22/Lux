//! Lexical analysis module
//!
//! This module handles tokenization of Lux source code.

pub mod token;
pub mod scanner;

pub use token::{Token, TokenType, Keyword, Literal};
pub use scanner::Lexer;

