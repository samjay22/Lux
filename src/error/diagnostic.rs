//! Diagnostic formatting for better error messages
//!
//! This module provides utilities for formatting error messages with
//! source code context and helpful suggestions.

use super::{LuxError, SourceLocation};
use colored::Colorize;

/// Diagnostic information for displaying errors with context
pub struct Diagnostic {
    error: LuxError,
    source: Option<String>,
}

impl Diagnostic {
    /// Create a new diagnostic from an error
    pub fn new(error: LuxError) -> Self {
        Self {
            error,
            source: None,
        }
    }

    /// Create a diagnostic with source code context
    pub fn with_source(error: LuxError, source: &str) -> Self {
        Self {
            error,
            source: Some(source.to_string()),
        }
    }

    /// Format the diagnostic with color and context
    pub fn format(&self) -> String {
        let mut output = String::new();

        // Error header
        let kind = self.error.kind().red().bold();
        output.push_str(&format!("{}: ", kind));
        output.push_str(self.error.message());
        output.push('\n');

        // Location and source context
        if let Some(location) = self.error.location() {
            output.push_str(&format!("  {} {}\n", "-->".blue().bold(), location));

            if let Some(ref source) = self.source {
                output.push_str(&self.format_source_context(source, location));
            }
        }

        output
    }

    /// Format source code context around the error location
    fn format_source_context(&self, source: &str, location: &SourceLocation) -> String {
        let mut output = String::new();
        let lines: Vec<&str> = source.lines().collect();

        if location.line == 0 || location.line > lines.len() {
            return output;
        }

        let line_idx = location.line - 1;
        let line_num_width = location.line.to_string().len();

        // Show previous line if available
        if line_idx > 0 {
            output.push_str(&format!(
                "  {} {}\n",
                format!("{:width$}", line_idx, width = line_num_width).blue(),
                lines[line_idx - 1]
            ));
        }

        // Show error line
        output.push_str(&format!(
            "  {} {}\n",
            format!("{:width$}", location.line, width = line_num_width)
                .blue()
                .bold(),
            lines[line_idx]
        ));

        // Show error indicator
        let indicator_padding = " ".repeat(line_num_width + 2 + location.column - 1);
        output.push_str(&format!("{}{}\n", indicator_padding, "^".red().bold()));

        // Show next line if available
        if line_idx + 1 < lines.len() {
            output.push_str(&format!(
                "  {} {}\n",
                format!("{:width$}", line_idx + 2, width = line_num_width).blue(),
                lines[line_idx + 1]
            ));
        }

        output
    }
}

impl std::fmt::Display for Diagnostic {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.format())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_diagnostic_without_source() {
        let loc = SourceLocation::at(1, 1);
        let err = LuxError::lexer_error("unexpected character", loc);
        let diag = Diagnostic::new(err);
        
        let formatted = diag.format();
        assert!(formatted.contains("Lexer Error"));
        assert!(formatted.contains("unexpected character"));
    }

    #[test]
    fn test_diagnostic_with_source() {
        let source = "let x = 42\nlet y = @\nlet z = 10";
        let loc = SourceLocation::at(2, 9);
        let err = LuxError::lexer_error("unexpected character '@'", loc);
        let diag = Diagnostic::with_source(err, source);
        
        let formatted = diag.format();
        assert!(formatted.contains("Lexer Error"));
        assert!(formatted.contains("let y = @"));
    }
}

