//! Lexer/Scanner implementation for the Lux language
//!
//! This module implements lexical analysis, converting source code into tokens.

use crate::error::{LuxError, LuxResult, SourceLocation};
use super::token::{Token, TokenType, Keyword, Literal};

/// Lexer for Lux source code
pub struct Lexer {
    source: Vec<char>,
    tokens: Vec<Token>,
    start: usize,
    current: usize,
    line: usize,
    column: usize,
    filename: Option<String>,
}

impl Lexer {
    /// Create a new lexer
    pub fn new(source: &str, filename: Option<&str>) -> Self {
        Self {
            source: source.chars().collect(),
            tokens: Vec::new(),
            start: 0,
            current: 0,
            line: 1,
            column: 1,
            filename: filename.map(|s| s.to_string()),
        }
    }

    /// Tokenize the source code
    pub fn tokenize(&mut self) -> LuxResult<Vec<Token>> {
        while !self.is_at_end() {
            self.start = self.current;
            self.scan_token()?;
        }

        // Add EOF token
        self.tokens.push(Token::new(
            TokenType::Eof,
            String::new(),
            self.current_location(),
        ));

        Ok(self.tokens.clone())
    }

    /// Scan a single token
    fn scan_token(&mut self) -> LuxResult<()> {
        let c = self.advance();

        match c {
            // Whitespace (skip)
            ' ' | '\r' | '\t' => Ok(()),

            // Newline
            '\n' => {
                self.line += 1;
                self.column = 1;
                // Optionally emit newline tokens for statement separation
                // self.add_token(TokenType::Newline);
                Ok(())
            }

            // Single-character tokens
            '(' => self.add_token(TokenType::LeftParen),
            ')' => self.add_token(TokenType::RightParen),
            '{' => self.add_token(TokenType::LeftBrace),
            '}' => self.add_token(TokenType::RightBrace),
            '[' => self.add_token(TokenType::LeftBracket),
            ']' => self.add_token(TokenType::RightBracket),
            ',' => self.add_token(TokenType::Comma),
            '.' => self.add_token(TokenType::Dot),
            ';' => self.add_token(TokenType::Semicolon),
            '+' => self.add_token(TokenType::Plus),
            '*' => self.add_token(TokenType::Star),
            '%' => self.add_token(TokenType::Percent),
            '#' => self.add_token(TokenType::Hash),

            // Two-character tokens
            '-' => {
                if self.match_char('>') {
                    self.add_token(TokenType::Arrow)
                } else {
                    self.add_token(TokenType::Minus)
                }
            }

            '=' => {
                if self.match_char('=') {
                    self.add_token(TokenType::Equal)
                } else {
                    self.add_token(TokenType::Assign)
                }
            }

            '!' => {
                if self.match_char('=') {
                    self.add_token(TokenType::NotEqual)
                } else {
                    Err(self.error("Unexpected character '!'. Did you mean '!='?"))
                }
            }

            '<' => {
                if self.match_char('=') {
                    self.add_token(TokenType::LessEqual)
                } else {
                    self.add_token(TokenType::Less)
                }
            }

            '>' => {
                if self.match_char('=') {
                    self.add_token(TokenType::GreaterEqual)
                } else {
                    self.add_token(TokenType::Greater)
                }
            }

            ':' => {
                if self.match_char('=') {
                    self.add_token(TokenType::ColonAssign)
                } else {
                    self.add_token(TokenType::Colon)
                }
            }

            // Comments
            '/' => {
                if self.match_char('/') {
                    // Single-line comment: skip until end of line
                    while self.peek() != '\n' && !self.is_at_end() {
                        self.advance();
                    }
                    Ok(())
                } else if self.match_char('*') {
                    // Multi-line comment
                    self.scan_multiline_comment()
                } else {
                    self.add_token(TokenType::Slash)
                }
            }

            // String literals
            '"' => self.scan_string(),

            // Number literals
            c if c.is_ascii_digit() => self.scan_number(),

            // Identifiers and keywords
            c if c.is_alphabetic() || c == '_' => self.scan_identifier(),

            // Unexpected character
            _ => Err(self.error(&format!("Unexpected character '{}'", c))),
        }
    }

    /// Scan a string literal
    fn scan_string(&mut self) -> LuxResult<()> {
        let mut value = String::new();

        while self.peek() != '"' && !self.is_at_end() {
            if self.peek() == '\n' {
                self.line += 1;
                self.column = 1;
            }

            // Handle escape sequences
            if self.peek() == '\\' {
                self.advance(); // consume backslash
                let escaped = self.advance();
                match escaped {
                    'n' => value.push('\n'),
                    't' => value.push('\t'),
                    'r' => value.push('\r'),
                    '\\' => value.push('\\'),
                    '"' => value.push('"'),
                    _ => return Err(self.error(&format!("Invalid escape sequence '\\{}'", escaped))),
                }
            } else {
                value.push(self.advance());
            }
        }

        if self.is_at_end() {
            return Err(self.error("Unterminated string"));
        }

        // Consume closing quote
        self.advance();

        self.add_token(TokenType::Literal(Literal::String(value)))
    }

    /// Scan a number literal (integer or float)
    fn scan_number(&mut self) -> LuxResult<()> {
        while self.peek().is_ascii_digit() {
            self.advance();
        }

        // Check for decimal point
        let is_float = if self.peek() == '.' && self.peek_next().is_ascii_digit() {
            self.advance(); // consume '.'
            while self.peek().is_ascii_digit() {
                self.advance();
            }
            true
        } else {
            false
        };

        let lexeme: String = self.source[self.start..self.current].iter().collect();

        if is_float {
            let value = lexeme.parse::<f64>()
                .map_err(|_| self.error(&format!("Invalid float literal '{}'", lexeme)))?;
            self.add_token(TokenType::Literal(Literal::Float(value)))
        } else {
            let value = lexeme.parse::<i64>()
                .map_err(|_| self.error(&format!("Invalid integer literal '{}'", lexeme)))?;
            self.add_token(TokenType::Literal(Literal::Integer(value)))
        }
    }

    /// Scan an identifier or keyword
    fn scan_identifier(&mut self) -> LuxResult<()> {
        while self.peek().is_alphanumeric() || self.peek() == '_' {
            self.advance();
        }

        let lexeme: String = self.source[self.start..self.current].iter().collect();

        // Check if it's a keyword
        let token_type = if let Some(keyword) = Keyword::from_str(&lexeme) {
            TokenType::Keyword(keyword)
        } else {
            TokenType::Identifier
        };

        self.add_token(token_type)
    }

    /// Scan a multi-line comment
    fn scan_multiline_comment(&mut self) -> LuxResult<()> {
        let mut depth = 1;

        while depth > 0 && !self.is_at_end() {
            if self.peek() == '/' && self.peek_next() == '*' {
                self.advance();
                self.advance();
                depth += 1;
            } else if self.peek() == '*' && self.peek_next() == '/' {
                self.advance();
                self.advance();
                depth -= 1;
            } else {
                if self.peek() == '\n' {
                    self.line += 1;
                    self.column = 1;
                }
                self.advance();
            }
        }

        if depth > 0 {
            return Err(self.error("Unterminated multi-line comment"));
        }

        Ok(())
    }

    /// Add a token to the token list
    fn add_token(&mut self, token_type: TokenType) -> LuxResult<()> {
        let lexeme: String = self.source[self.start..self.current].iter().collect();
        let location = SourceLocation::new(
            self.line,
            self.column - (self.current - self.start),
            self.filename.clone(),
        );
        self.tokens.push(Token::new(token_type, lexeme, location));
        Ok(())
    }

    /// Advance to the next character
    fn advance(&mut self) -> char {
        let c = self.source[self.current];
        self.current += 1;
        self.column += 1;
        c
    }

    /// Check if the next character matches and consume it if so
    fn match_char(&mut self, expected: char) -> bool {
        if self.is_at_end() || self.source[self.current] != expected {
            false
        } else {
            self.current += 1;
            self.column += 1;
            true
        }
    }

    /// Peek at the current character without consuming it
    fn peek(&self) -> char {
        if self.is_at_end() {
            '\0'
        } else {
            self.source[self.current]
        }
    }

    /// Peek at the next character without consuming it
    fn peek_next(&self) -> char {
        if self.current + 1 >= self.source.len() {
            '\0'
        } else {
            self.source[self.current + 1]
        }
    }

    /// Check if we've reached the end of the source
    fn is_at_end(&self) -> bool {
        self.current >= self.source.len()
    }

    /// Get the current source location
    fn current_location(&self) -> SourceLocation {
        SourceLocation::new(self.line, self.column, self.filename.clone())
    }

    /// Create an error at the current location
    fn error(&self, message: &str) -> LuxError {
        LuxError::lexer_error(message, self.current_location())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tokenize_source(source: &str) -> LuxResult<Vec<Token>> {
        let mut lexer = Lexer::new(source, None);
        lexer.tokenize()
    }

    #[test]
    fn test_empty_source() {
        let tokens = tokenize_source("").unwrap();
        assert_eq!(tokens.len(), 1); // Just EOF
        assert_eq!(tokens[0].token_type, TokenType::Eof);
    }

    #[test]
    fn test_single_character_tokens() {
        let tokens = tokenize_source("(){}[],;.+-*/%").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::LeftParen);
        assert_eq!(tokens[1].token_type, TokenType::RightParen);
        assert_eq!(tokens[2].token_type, TokenType::LeftBrace);
        assert_eq!(tokens[3].token_type, TokenType::RightBrace);
        assert_eq!(tokens[4].token_type, TokenType::LeftBracket);
        assert_eq!(tokens[5].token_type, TokenType::RightBracket);
        assert_eq!(tokens[6].token_type, TokenType::Comma);
        assert_eq!(tokens[7].token_type, TokenType::Semicolon);
        assert_eq!(tokens[8].token_type, TokenType::Dot);
        assert_eq!(tokens[9].token_type, TokenType::Plus);
        assert_eq!(tokens[10].token_type, TokenType::Minus);
        assert_eq!(tokens[11].token_type, TokenType::Star);
        assert_eq!(tokens[12].token_type, TokenType::Slash);
        assert_eq!(tokens[13].token_type, TokenType::Percent);
    }

    #[test]
    fn test_two_character_tokens() {
        let tokens = tokenize_source("== != <= >= := ->").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Equal);
        assert_eq!(tokens[1].token_type, TokenType::NotEqual);
        assert_eq!(tokens[2].token_type, TokenType::LessEqual);
        assert_eq!(tokens[3].token_type, TokenType::GreaterEqual);
        assert_eq!(tokens[4].token_type, TokenType::ColonAssign);
        assert_eq!(tokens[5].token_type, TokenType::Arrow);
    }

    #[test]
    fn test_keywords() {
        let tokens = tokenize_source("local const fn return if else while for").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Local));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::Const));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::Fn));
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::Return));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::If));
        assert_eq!(tokens[5].token_type, TokenType::Keyword(Keyword::Else));
        assert_eq!(tokens[6].token_type, TokenType::Keyword(Keyword::While));
        assert_eq!(tokens[7].token_type, TokenType::Keyword(Keyword::For));
    }

    #[test]
    fn test_metatable_keywords() {
        let tokens = tokenize_source("table setmetatable getmetatable").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Table));
        // setmetatable and getmetatable are now regular identifiers, not keywords
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "setmetatable");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "getmetatable");
    }

    #[test]
    fn test_async_keywords() {
        let tokens = tokenize_source("async await spawn").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Async));
        assert_eq!(tokens[1].token_type, TokenType::Keyword(Keyword::Await));
        assert_eq!(tokens[2].token_type, TokenType::Keyword(Keyword::Spawn));
    }

    #[test]
    fn test_identifiers() {
        let tokens = tokenize_source("foo bar_baz _private myVar123").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "foo");
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "bar_baz");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "_private");
        assert_eq!(tokens[3].token_type, TokenType::Identifier);
        assert_eq!(tokens[3].lexeme, "myVar123");
    }

    #[test]
    fn test_integer_literals() {
        let tokens = tokenize_source("0 42 123456").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Literal(Literal::Integer(0)));
        assert_eq!(tokens[1].token_type, TokenType::Literal(Literal::Integer(42)));
        assert_eq!(tokens[2].token_type, TokenType::Literal(Literal::Integer(123456)));
    }

    #[test]
    fn test_float_literals() {
        let tokens = tokenize_source("3.14 0.5 123.456").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Literal(Literal::Float(3.14)));
        assert_eq!(tokens[1].token_type, TokenType::Literal(Literal::Float(0.5)));
        assert_eq!(tokens[2].token_type, TokenType::Literal(Literal::Float(123.456)));
    }

    #[test]
    fn test_string_literals() {
        let tokens = tokenize_source(r#""hello" "world" "foo bar""#).unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Literal(Literal::String("hello".to_string())));
        assert_eq!(tokens[1].token_type, TokenType::Literal(Literal::String("world".to_string())));
        assert_eq!(tokens[2].token_type, TokenType::Literal(Literal::String("foo bar".to_string())));
    }

    #[test]
    fn test_string_escape_sequences() {
        let tokens = tokenize_source(r#""hello\nworld" "tab\there" "quote\"test""#).unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Literal(Literal::String("hello\nworld".to_string())));
        assert_eq!(tokens[1].token_type, TokenType::Literal(Literal::String("tab\there".to_string())));
        assert_eq!(tokens[2].token_type, TokenType::Literal(Literal::String("quote\"test".to_string())));
    }

    #[test]
    fn test_single_line_comment() {
        let tokens = tokenize_source("local x = 42 // this is a comment\nlocal y = 10").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Local));
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].token_type, TokenType::Assign);
        assert_eq!(tokens[3].token_type, TokenType::Literal(Literal::Integer(42)));
        assert_eq!(tokens[4].token_type, TokenType::Keyword(Keyword::Local));
    }

    #[test]
    fn test_multiline_comment() {
        let tokens = tokenize_source("local x /* comment */ = 42").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Local));
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].token_type, TokenType::Assign);
        assert_eq!(tokens[3].token_type, TokenType::Literal(Literal::Integer(42)));
    }

    #[test]
    fn test_nested_multiline_comment() {
        let tokens = tokenize_source("local x /* outer /* inner */ outer */ = 42").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Local));
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].token_type, TokenType::Assign);
        assert_eq!(tokens[3].token_type, TokenType::Literal(Literal::Integer(42)));
    }

    #[test]
    fn test_complete_statement() {
        let tokens = tokenize_source("local x: int = 42").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Local));
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "x");
        assert_eq!(tokens[2].token_type, TokenType::Colon);
        assert_eq!(tokens[3].token_type, TokenType::Keyword(Keyword::Int));
        assert_eq!(tokens[4].token_type, TokenType::Assign);
        assert_eq!(tokens[5].token_type, TokenType::Literal(Literal::Integer(42)));
    }

    #[test]
    fn test_hash_operator() {
        let tokens = tokenize_source("#myTable").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Hash);
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "myTable");
    }

    #[test]
    fn test_metamethod_identifiers() {
        let tokens = tokenize_source("__index __add __call").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Identifier);
        assert_eq!(tokens[0].lexeme, "__index");
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "__add");
        assert_eq!(tokens[2].token_type, TokenType::Identifier);
        assert_eq!(tokens[2].lexeme, "__call");
    }

    #[test]
    fn test_function_declaration() {
        let tokens = tokenize_source("fn add(a: int, b: int) -> int { return a + b }").unwrap();
        assert_eq!(tokens[0].token_type, TokenType::Keyword(Keyword::Fn));
        assert_eq!(tokens[1].token_type, TokenType::Identifier);
        assert_eq!(tokens[1].lexeme, "add");
        assert_eq!(tokens[2].token_type, TokenType::LeftParen);
    }

    #[test]
    fn test_unterminated_string() {
        let result = tokenize_source(r#""unterminated"#);
        assert!(result.is_err());
        if let Err(LuxError::LexerError { message, .. }) = result {
            assert!(message.contains("Unterminated string"));
        }
    }

    #[test]
    fn test_invalid_character() {
        let result = tokenize_source("let x = @");
        assert!(result.is_err());
        if let Err(LuxError::LexerError { message, .. }) = result {
            assert!(message.contains("Unexpected character"));
        }
    }

    #[test]
    fn test_source_location() {
        let tokens = tokenize_source("let\nx").unwrap();
        assert_eq!(tokens[0].location.line, 1);
        assert_eq!(tokens[1].location.line, 2);
    }
}

