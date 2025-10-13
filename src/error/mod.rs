//! Error handling and diagnostics for the Lux language
//!
//! This module provides comprehensive error types and diagnostic formatting
//! for all stages of compilation and execution.

use std::fmt;

pub mod diagnostic;

pub use diagnostic::Diagnostic;

/// Result type alias for Lux operations
pub type LuxResult<T> = Result<T, LuxError>;

/// Source location information for error reporting
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SourceLocation {
    /// Line number (1-based)
    pub line: usize,
    /// Column number (1-based)
    pub column: usize,
    /// Optional filename
    pub filename: Option<String>,
}

impl SourceLocation {
    /// Create a new source location
    pub fn new(line: usize, column: usize, filename: Option<String>) -> Self {
        Self {
            line,
            column,
            filename,
        }
    }

    /// Create a source location without a filename
    pub fn at(line: usize, column: usize) -> Self {
        Self::new(line, column, None)
    }
}

impl fmt::Display for SourceLocation {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(ref filename) = self.filename {
            write!(f, "{}:{}:{}", filename, self.line, self.column)
        } else {
            write!(f, "{}:{}", self.line, self.column)
        }
    }
}

/// Main error type for the Lux language
#[derive(Debug, Clone)]
pub enum LuxError {
    /// Lexical analysis error
    LexerError {
        message: String,
        location: SourceLocation,
    },
    /// Parsing error
    ParseError {
        message: String,
        location: SourceLocation,
    },
    /// Type checking error
    TypeError {
        message: String,
        location: SourceLocation,
    },
    /// Semantic analysis error
    SemanticError {
        message: String,
        location: SourceLocation,
    },
    /// Runtime error
    RuntimeError {
        message: String,
        location: Option<SourceLocation>,
    },
    /// Internal compiler error (should not happen in normal operation)
    InternalError {
        message: String,
    },
}

impl LuxError {
    /// Create a new lexer error
    pub fn lexer_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::LexerError {
            message: message.into(),
            location,
        }
    }

    /// Create a new parse error
    pub fn parse_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::ParseError {
            message: message.into(),
            location,
        }
    }

    /// Create a new type error
    pub fn type_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::TypeError {
            message: message.into(),
            location,
        }
    }

    /// Create a new semantic error
    pub fn semantic_error(message: impl Into<String>, location: SourceLocation) -> Self {
        Self::SemanticError {
            message: message.into(),
            location,
        }
    }

    /// Create a new runtime error
    pub fn runtime_error(message: impl Into<String>, location: Option<SourceLocation>) -> Self {
        Self::RuntimeError {
            message: message.into(),
            location,
        }
    }

    /// Create a new internal error
    pub fn internal_error(message: impl Into<String>) -> Self {
        Self::InternalError {
            message: message.into(),
        }
    }

    /// Get the error kind as a string
    pub fn kind(&self) -> &str {
        match self {
            Self::LexerError { .. } => "Lexer Error",
            Self::ParseError { .. } => "Parse Error",
            Self::TypeError { .. } => "Type Error",
            Self::SemanticError { .. } => "Semantic Error",
            Self::RuntimeError { .. } => "Runtime Error",
            Self::InternalError { .. } => "Internal Error",
        }
    }

    /// Get the error message
    pub fn message(&self) -> &str {
        match self {
            Self::LexerError { message, .. }
            | Self::ParseError { message, .. }
            | Self::TypeError { message, .. }
            | Self::SemanticError { message, .. }
            | Self::RuntimeError { message, .. }
            | Self::InternalError { message } => message,
        }
    }

    /// Get the source location if available
    pub fn location(&self) -> Option<&SourceLocation> {
        match self {
            Self::LexerError { location, .. }
            | Self::ParseError { location, .. }
            | Self::TypeError { location, .. }
            | Self::SemanticError { location, .. } => Some(location),
            Self::RuntimeError { location, .. } => location.as_ref(),
            Self::InternalError { .. } => None,
        }
    }
}

impl fmt::Display for LuxError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if let Some(location) = self.location() {
            write!(f, "{}: {} at {}", self.kind(), self.message(), location)
        } else {
            write!(f, "{}: {}", self.kind(), self.message())
        }
    }
}

impl std::error::Error for LuxError {}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_source_location_display() {
        let loc = SourceLocation::at(10, 5);
        assert_eq!(loc.to_string(), "10:5");

        let loc_with_file = SourceLocation::new(10, 5, Some("test.lux".to_string()));
        assert_eq!(loc_with_file.to_string(), "test.lux:10:5");
    }

    #[test]
    fn test_error_creation() {
        let loc = SourceLocation::at(1, 1);
        let err = LuxError::lexer_error("unexpected character", loc.clone());
        
        assert_eq!(err.kind(), "Lexer Error");
        assert_eq!(err.message(), "unexpected character");
        assert_eq!(err.location(), Some(&loc));
    }

    #[test]
    fn test_error_display() {
        let loc = SourceLocation::at(5, 10);
        let err = LuxError::parse_error("expected ';'", loc);
        
        assert_eq!(err.to_string(), "Parse Error: expected ';' at 5:10");
    }
}

