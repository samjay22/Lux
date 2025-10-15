//! Token definitions for the Lux language
//!
//! This module defines all token types used in lexical analysis.

use crate::error::SourceLocation;
use std::fmt;

/// A token in the Lux language
#[derive(Debug, Clone, PartialEq)]
pub struct Token {
    pub token_type: TokenType,
    pub lexeme: String,
    pub location: SourceLocation,
}

impl Token {
    /// Create a new token
    pub fn new(token_type: TokenType, lexeme: String, location: SourceLocation) -> Self {
        Self {
            token_type,
            lexeme,
            location,
        }
    }
}

/// Token types in the Lux language
#[derive(Debug, Clone, PartialEq)]
pub enum TokenType {
    // Literals
    Literal(Literal),

    // Identifiers and keywords
    Identifier,
    Keyword(Keyword),

    // Operators
    // Arithmetic
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %

    // Comparison
    Equal,          // ==
    NotEqual,       // !=
    Less,           // <
    LessEqual,      // <=
    Greater,        // >
    GreaterEqual,   // >=

    // Logical
    And,        // and
    Or,         // or
    Not,        // not

    // Assignment
    Assign,         // =
    ColonAssign,    // :=

    // Unary operators
    Hash,           // # (length operator, Lua-style)
    Ampersand,      // & (address-of operator)

    // Delimiters
    LeftParen,      // (
    RightParen,     // )
    LeftBrace,      // {
    RightBrace,     // }
    LeftBracket,    // [
    RightBracket,   // ]
    Comma,          // ,
    Dot,            // .
    Colon,          // :
    Semicolon,      // ;
    Arrow,          // ->

    // Special
    Newline,
    Eof,
}

/// Keywords in the Lux language
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum Keyword {
    // Variable declarations (Lua-style)
    Local,
    Const,

    // Functions
    Fn,
    Return,

    // Control flow
    If,
    Else,
    While,
    For,
    Break,
    Continue,

    // Types
    Int,
    Float,
    String,
    Bool,
    Nil,
    Table,

    // Boolean literals
    True,
    False,

    // Async/concurrency
    Async,
    Await,
    Spawn,

    // Logical operators (also keywords)
    And,
    Or,
    Not,

    // Modules
    Import,
}

impl Keyword {
    /// Get keyword from string
    pub fn from_str(s: &str) -> Option<Self> {
        match s {
            "local" => Some(Self::Local),
            "const" => Some(Self::Const),
            "fn" => Some(Self::Fn),
            "return" => Some(Self::Return),
            "if" => Some(Self::If),
            "else" => Some(Self::Else),
            "while" => Some(Self::While),
            "for" => Some(Self::For),
            "break" => Some(Self::Break),
            "continue" => Some(Self::Continue),
            "int" => Some(Self::Int),
            "float" => Some(Self::Float),
            "string" => Some(Self::String),
            "bool" => Some(Self::Bool),
            "nil" => Some(Self::Nil),
            "table" => Some(Self::Table),
            "true" => Some(Self::True),
            "false" => Some(Self::False),
            "async" => Some(Self::Async),
            "await" => Some(Self::Await),
            "spawn" => Some(Self::Spawn),
            "and" => Some(Self::And),
            "or" => Some(Self::Or),
            "not" => Some(Self::Not),
            "import" => Some(Self::Import),
            _ => None,
        }
    }

    /// Get string representation of keyword
    pub fn as_str(&self) -> &'static str {
        match self {
            Self::Local => "local",
            Self::Const => "const",
            Self::Fn => "fn",
            Self::Return => "return",
            Self::If => "if",
            Self::Else => "else",
            Self::While => "while",
            Self::For => "for",
            Self::Break => "break",
            Self::Continue => "continue",
            Self::Int => "int",
            Self::Float => "float",
            Self::String => "string",
            Self::Bool => "bool",
            Self::Nil => "nil",
            Self::Table => "table",
            Self::True => "true",
            Self::False => "false",
            Self::Async => "async",
            Self::Await => "await",
            Self::Spawn => "spawn",
            Self::And => "and",
            Self::Or => "or",
            Self::Not => "not",
            Self::Import => "import",
        }
    }
}

impl fmt::Display for Keyword {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

/// Literal token values
#[derive(Debug, Clone, PartialEq)]
pub enum Literal {
    Integer(i64),
    Float(f64),
    String(String),
}

impl fmt::Display for TokenType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Literal(lit) => write!(f, "{:?}", lit),
            Self::Identifier => write!(f, "identifier"),
            Self::Keyword(kw) => write!(f, "keyword '{}'", kw),
            Self::Plus => write!(f, "+"),
            Self::Minus => write!(f, "-"),
            Self::Star => write!(f, "*"),
            Self::Slash => write!(f, "/"),
            Self::Percent => write!(f, "%"),
            Self::Equal => write!(f, "=="),
            Self::NotEqual => write!(f, "!="),
            Self::Less => write!(f, "<"),
            Self::LessEqual => write!(f, "<="),
            Self::Greater => write!(f, ">"),
            Self::GreaterEqual => write!(f, ">="),
            Self::And => write!(f, "and"),
            Self::Or => write!(f, "or"),
            Self::Not => write!(f, "not"),
            Self::Assign => write!(f, "="),
            Self::ColonAssign => write!(f, ":="),
            Self::Hash => write!(f, "#"),
            Self::LeftParen => write!(f, "("),
            Self::RightParen => write!(f, ")"),
            Self::LeftBrace => write!(f, "{{"),
            Self::RightBrace => write!(f, "}}"),
            Self::LeftBracket => write!(f, "["),
            Self::RightBracket => write!(f, "]"),
            Self::Comma => write!(f, ","),
            Self::Dot => write!(f, "."),
            Self::Colon => write!(f, ":"),
            Self::Semicolon => write!(f, ";"),
            Self::Arrow => write!(f, "->"),
            Self::Ampersand => write!(f, "&"),
            Self::Newline => write!(f, "newline"),
            Self::Eof => write!(f, "EOF"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keyword_from_str() {
        assert_eq!(Keyword::from_str("local"), Some(Keyword::Local));
        assert_eq!(Keyword::from_str("fn"), Some(Keyword::Fn));
        assert_eq!(Keyword::from_str("async"), Some(Keyword::Async));
        assert_eq!(Keyword::from_str("table"), Some(Keyword::Table));
        assert_eq!(Keyword::from_str("invalid"), None);
        // setmetatable and getmetatable are now regular identifiers, not keywords
        assert_eq!(Keyword::from_str("setmetatable"), None);
        assert_eq!(Keyword::from_str("getmetatable"), None);
    }

    #[test]
    fn test_keyword_as_str() {
        assert_eq!(Keyword::Local.as_str(), "local");
        assert_eq!(Keyword::Async.as_str(), "async");
        assert_eq!(Keyword::Table.as_str(), "table");
    }
}

