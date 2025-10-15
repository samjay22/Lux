//! Parser implementation
//!
//! This module implements the parser for the Lux language.

use crate::error::{LuxError, LuxResult, SourceLocation};
use crate::lexer::{Token, TokenType, Keyword, Literal as TokenLiteral};
use super::ast::*;

/// Parser for Lux source code
pub struct Parser {
    tokens: Vec<Token>,
    current: usize,
}

impl Parser {
    /// Create a new parser from tokens
    pub fn new(tokens: Vec<Token>) -> Self {
        Self {
            tokens,
            current: 0,
        }
    }

    /// Parse tokens into an AST
    pub fn parse(&mut self) -> LuxResult<Ast> {
        let mut statements = Vec::new();

        while !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        Ok(Ast { statements })
    }

    // ===== Declarations =====

    fn declaration(&mut self) -> LuxResult<Stmt> {
        if self.match_keyword(Keyword::Import) {
            self.import_declaration()
        } else if self.match_keyword(Keyword::Local) {
            self.var_declaration(false)
        } else if self.match_keyword(Keyword::Const) {
            self.var_declaration(true)
        } else if self.check_keyword(Keyword::Fn) || self.check_keyword(Keyword::Async) {
            self.function_declaration()
        } else {
            self.statement()
        }
    }

    fn import_declaration(&mut self) -> LuxResult<Stmt> {
        let location = self.previous().location.clone();

        // Expect a string literal for the path
        if let TokenType::Literal(TokenLiteral::String(_)) = &self.peek().token_type {
            let token = self.advance();
            if let TokenType::Literal(TokenLiteral::String(path)) = &token.token_type {
                Ok(Stmt::Import {
                    path: path.clone(),
                    location
                })
            } else {
                unreachable!()
            }
        } else {
            Err(LuxError::parse_error(
                "Expected string path after 'import'".to_string(),
                self.peek().location.clone(),
            ))
        }
    }

    fn var_declaration(&mut self, is_const: bool) -> LuxResult<Stmt> {
        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected variable name")?;

        let type_annotation = if self.match_token(TokenType::Colon) {
            Some(self.parse_type()?)
        } else {
            None
        };

        let initializer = if self.match_token(TokenType::Assign) || self.match_token(TokenType::ColonAssign) {
            Some(self.expression()?)
        } else {
            None
        };

        Ok(Stmt::VarDecl {
            name,
            type_annotation,
            initializer,
            is_const,
            location,
        })
    }

    fn function_declaration(&mut self) -> LuxResult<Stmt> {
        let is_async = self.match_keyword(Keyword::Async);
        self.consume_keyword(Keyword::Fn, "Expected 'fn'")?;

        let location = self.previous().location.clone();
        let name = self.consume_identifier("Expected function name")?;

        self.consume(TokenType::LeftParen, "Expected '(' after function name")?;

        let mut params = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                let param_name = self.consume_identifier("Expected parameter name")?;

                // Type annotation is optional
                let param_type = if self.match_token(TokenType::Colon) {
                    self.parse_type()?
                } else {
                    Type::Nil // Use Nil to represent "any" type
                };

                params.push((param_name, param_type));

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after parameters")?;

        let return_type = if self.match_token(TokenType::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expected '{' before function body")?;
        let body = self.block_statements()?;

        Ok(Stmt::FunctionDecl {
            name,
            params,
            return_type,
            body,
            is_async,
            location,
        })
    }

    // ===== Statements =====

    fn statement(&mut self) -> LuxResult<Stmt> {
        if self.match_keyword(Keyword::If) {
            self.if_statement()
        } else if self.match_keyword(Keyword::While) {
            self.while_statement()
        } else if self.match_keyword(Keyword::For) {
            self.for_statement()
        } else if self.match_keyword(Keyword::Return) {
            self.return_statement()
        } else if self.match_keyword(Keyword::Break) {
            Ok(Stmt::Break {
                location: self.previous().location.clone(),
            })
        } else if self.match_keyword(Keyword::Continue) {
            Ok(Stmt::Continue {
                location: self.previous().location.clone(),
            })
        } else if self.match_token(TokenType::LeftBrace) {
            let location = self.previous().location.clone();
            let statements = self.block_statements()?;
            Ok(Stmt::Block { statements, location })
        } else {
            self.expression_statement()
        }
    }

    fn if_statement(&mut self) -> LuxResult<Stmt> {
        let location = self.previous().location.clone();
        let condition = self.expression()?;

        self.consume(TokenType::LeftBrace, "Expected '{' after if condition")?;
        let then_branch = self.block_statements()?;

        let else_branch = if self.match_keyword(Keyword::Else) {
            if self.match_keyword(Keyword::If) {
                // else if
                Some(vec![self.if_statement()?])
            } else {
                self.consume(TokenType::LeftBrace, "Expected '{' after else")?;
                Some(self.block_statements()?)
            }
        } else {
            None
        };

        Ok(Stmt::If {
            condition,
            then_branch,
            else_branch,
            location,
        })
    }

    fn while_statement(&mut self) -> LuxResult<Stmt> {
        let location = self.previous().location.clone();
        let condition = self.expression()?;

        self.consume(TokenType::LeftBrace, "Expected '{' after while condition")?;
        let body = self.block_statements()?;

        Ok(Stmt::While {
            condition,
            body,
            location,
        })
    }

    fn for_statement(&mut self) -> LuxResult<Stmt> {
        let location = self.previous().location.clone();

        // Initializer
        let initializer = if self.match_keyword(Keyword::Local) {
            Some(Box::new(self.var_declaration(false)?))
        } else if !self.check(TokenType::Semicolon) {
            Some(Box::new(self.expression_statement()?))
        } else {
            None
        };

        self.consume(TokenType::Semicolon, "Expected ';' after for initializer")?;

        // Condition
        let condition = if !self.check(TokenType::Semicolon) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(TokenType::Semicolon, "Expected ';' after for condition")?;

        // Increment
        let increment = if !self.check(TokenType::LeftBrace) {
            Some(self.expression()?)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expected '{' after for clauses")?;
        let body = self.block_statements()?;

        Ok(Stmt::For {
            initializer,
            condition,
            increment,
            body,
            location,
        })
    }

    fn return_statement(&mut self) -> LuxResult<Stmt> {
        let location = self.previous().location.clone();

        let value = if !self.check(TokenType::RightBrace) && !self.is_at_end() {
            Some(self.expression()?)
        } else {
            None
        };

        Ok(Stmt::Return { value, location })
    }

    fn expression_statement(&mut self) -> LuxResult<Stmt> {
        let expr = self.expression()?;
        let location = expr.location().clone();
        Ok(Stmt::Expression { expr, location })
    }

    fn block_statements(&mut self) -> LuxResult<Vec<Stmt>> {
        let mut statements = Vec::new();

        while !self.check(TokenType::RightBrace) && !self.is_at_end() {
            statements.push(self.declaration()?);
        }

        self.consume(TokenType::RightBrace, "Expected '}' after block")?;
        Ok(statements)
    }

    // ===== Expressions =====

    fn expression(&mut self) -> LuxResult<Expr> {
        self.assignment()
    }

    fn assignment(&mut self) -> LuxResult<Expr> {
        let expr = self.logical_or()?;

        if self.match_token(TokenType::Assign) {
            let location = self.previous().location.clone();
            let value = Box::new(self.assignment()?);

            // Check if the target is a valid assignment target
            match &expr {
                Expr::Variable { .. } | Expr::TableAccess { .. } => {
                    return Ok(Expr::Assign {
                        target: Box::new(expr),
                        value,
                        location,
                    });
                }
                _ => {
                    return Err(LuxError::parse_error(
                        "Invalid assignment target",
                        location,
                    ));
                }
            }
        }

        Ok(expr)
    }

    fn logical_or(&mut self) -> LuxResult<Expr> {
        let mut expr = self.logical_and()?;

        while self.match_keyword(Keyword::Or) {
            let location = self.previous().location.clone();
            let right = Box::new(self.logical_and()?);
            expr = Expr::Logical {
                left: Box::new(expr),
                operator: LogicalOp::Or,
                right,
                location,
            };
        }

        Ok(expr)
    }

    fn logical_and(&mut self) -> LuxResult<Expr> {
        let mut expr = self.equality()?;

        while self.match_keyword(Keyword::And) {
            let location = self.previous().location.clone();
            let right = Box::new(self.equality()?);
            expr = Expr::Logical {
                left: Box::new(expr),
                operator: LogicalOp::And,
                right,
                location,
            };
        }

        Ok(expr)
    }

    fn equality(&mut self) -> LuxResult<Expr> {
        let mut expr = self.comparison()?;

        while self.match_tokens(&[TokenType::Equal, TokenType::NotEqual]) {
            let location = self.previous().location.clone();
            let operator = match &self.previous().token_type {
                TokenType::Equal => BinaryOp::Equal,
                TokenType::NotEqual => BinaryOp::NotEqual,
                _ => unreachable!(),
            };
            let right = Box::new(self.comparison()?);
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right,
                location,
            };
        }

        Ok(expr)
    }

    fn comparison(&mut self) -> LuxResult<Expr> {
        let mut expr = self.term()?;

        while self.match_tokens(&[
            TokenType::Greater,
            TokenType::GreaterEqual,
            TokenType::Less,
            TokenType::LessEqual,
        ]) {
            let location = self.previous().location.clone();
            let operator = match &self.previous().token_type {
                TokenType::Greater => BinaryOp::Greater,
                TokenType::GreaterEqual => BinaryOp::GreaterEqual,
                TokenType::Less => BinaryOp::Less,
                TokenType::LessEqual => BinaryOp::LessEqual,
                _ => unreachable!(),
            };
            let right = Box::new(self.term()?);
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right,
                location,
            };
        }

        Ok(expr)
    }

    fn term(&mut self) -> LuxResult<Expr> {
        let mut expr = self.factor()?;

        while self.match_tokens(&[TokenType::Plus, TokenType::Minus]) {
            let location = self.previous().location.clone();
            let operator = match &self.previous().token_type {
                TokenType::Plus => BinaryOp::Add,
                TokenType::Minus => BinaryOp::Subtract,
                _ => unreachable!(),
            };
            let right = Box::new(self.factor()?);
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right,
                location,
            };
        }

        Ok(expr)
    }

    fn factor(&mut self) -> LuxResult<Expr> {
        let mut expr = self.unary()?;

        while self.match_tokens(&[TokenType::Star, TokenType::Slash, TokenType::Percent]) {
            let location = self.previous().location.clone();
            let operator = match &self.previous().token_type {
                TokenType::Star => BinaryOp::Multiply,
                TokenType::Slash => BinaryOp::Divide,
                TokenType::Percent => BinaryOp::Modulo,
                _ => unreachable!(),
            };
            let right = Box::new(self.unary()?);
            expr = Expr::Binary {
                left: Box::new(expr),
                operator,
                right,
                location,
            };
        }

        Ok(expr)
    }

    fn unary(&mut self) -> LuxResult<Expr> {
        if self.match_tokens(&[TokenType::Minus, TokenType::Hash, TokenType::Ampersand, TokenType::Star]) || self.match_keyword(Keyword::Not) {
            let location = self.previous().location.clone();
            let operator = match &self.previous().token_type {
                TokenType::Minus => UnaryOp::Negate,
                TokenType::Hash => UnaryOp::Length,
                TokenType::Ampersand => UnaryOp::AddressOf,
                TokenType::Star => UnaryOp::Dereference,
                TokenType::Keyword(Keyword::Not) => UnaryOp::Not,
                _ => unreachable!(),
            };
            let operand = Box::new(self.unary()?);
            return Ok(Expr::Unary {
                operator,
                operand,
                location,
            });
        }

        self.call()
    }

    fn call(&mut self) -> LuxResult<Expr> {
        let mut expr = self.primary()?;

        loop {
            if self.match_token(TokenType::LeftParen) {
                expr = self.finish_call(expr)?;
            } else if self.match_token(TokenType::Dot) {
                let location = self.previous().location.clone();
                let field = self.consume_identifier("Expected property name after '.'")?;
                expr = Expr::TableAccess {
                    table: Box::new(expr),
                    key: Box::new(Expr::Literal {
                        value: Literal::String(field),
                        location: location.clone(),
                    }),
                    location,
                };
            } else if self.match_token(TokenType::LeftBracket) {
                let location = self.previous().location.clone();
                let key = Box::new(self.expression()?);
                self.consume(TokenType::RightBracket, "Expected ']' after table index")?;
                expr = Expr::TableAccess {
                    table: Box::new(expr),
                    key,
                    location,
                };
            } else {
                break;
            }
        }

        Ok(expr)
    }

    fn finish_call(&mut self, callee: Expr) -> LuxResult<Expr> {
        let location = self.previous().location.clone();
        let mut arguments = Vec::new();

        if !self.check(TokenType::RightParen) {
            loop {
                arguments.push(self.expression()?);
                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after arguments")?;

        Ok(Expr::Call {
            callee: Box::new(callee),
            arguments,
            location,
        })
    }

    fn primary(&mut self) -> LuxResult<Expr> {
        let location = self.peek().location.clone();

        // Literals
        if let TokenType::Literal(lit) = &self.peek().token_type {
            let value = match lit {
                TokenLiteral::Integer(n) => Literal::Integer(*n),
                TokenLiteral::Float(f) => Literal::Float(*f),
                TokenLiteral::String(s) => Literal::String(s.clone()),
            };
            self.advance();
            return Ok(Expr::Literal { value, location });
        }

        // Boolean literals
        if self.match_keyword(Keyword::True) {
            return Ok(Expr::Literal {
                value: Literal::Boolean(true),
                location,
            });
        }

        if self.match_keyword(Keyword::False) {
            return Ok(Expr::Literal {
                value: Literal::Boolean(false),
                location,
            });
        }

        // Nil
        if self.match_keyword(Keyword::Nil) {
            return Ok(Expr::Literal {
                value: Literal::Nil,
                location,
            });
        }

        // Identifiers
        if self.check(TokenType::Identifier) {
            let name = self.advance().lexeme.clone();
            return Ok(Expr::Variable { name, location });
        }

        // Parenthesized expression
        if self.match_token(TokenType::LeftParen) {
            let expr = self.expression()?;
            self.consume(TokenType::RightParen, "Expected ')' after expression")?;
            return Ok(expr);
        }

        // Table literal
        if self.match_token(TokenType::LeftBrace) {
            return self.table_literal(location);
        }

        // Function expression
        if self.match_keyword(Keyword::Fn) {
            return self.function_expression(location);
        }

        // Spawn expression
        if self.match_keyword(Keyword::Spawn) {
            let call = Box::new(self.unary()?);
            return Ok(Expr::Spawn { call, location });
        }

        // Await expression
        if self.match_keyword(Keyword::Await) {
            let task = Box::new(self.unary()?);
            return Ok(Expr::Await { task, location });
        }

        Err(LuxError::parse_error(
            "Expected expression",
            self.peek().location.clone(),
        ))
    }

    fn function_expression(&mut self, location: SourceLocation) -> LuxResult<Expr> {
        self.consume(TokenType::LeftParen, "Expected '(' after 'fn'")?;

        let mut params = Vec::new();
        if !self.check(TokenType::RightParen) {
            loop {
                let param_name = self.consume_identifier("Expected parameter name")?;

                // Type annotation is optional
                let param_type = if self.match_token(TokenType::Colon) {
                    self.parse_type()?
                } else {
                    Type::Nil // Use Nil to represent "any" type
                };

                params.push((param_name, param_type));

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightParen, "Expected ')' after parameters")?;

        let return_type = if self.match_token(TokenType::Arrow) {
            Some(self.parse_type()?)
        } else {
            None
        };

        self.consume(TokenType::LeftBrace, "Expected '{' before function body")?;
        let body = self.block_statements()?;

        Ok(Expr::Function {
            params,
            return_type,
            body,
            location,
        })
    }

    fn table_literal(&mut self, location: SourceLocation) -> LuxResult<Expr> {
        let mut fields = Vec::new();

        if !self.check(TokenType::RightBrace) {
            loop {
                // Check for key = value or just value
                if self.check(TokenType::Identifier) {
                    let checkpoint = self.current;
                    let name = self.advance().lexeme.clone();

                    if self.match_token(TokenType::Assign) {
                        // key = value
                        let value = self.expression()?;
                        fields.push((TableKey::Identifier(name), value));
                    } else {
                        // Just a value, backtrack
                        self.current = checkpoint;
                        let value = self.expression()?;
                        fields.push((TableKey::Expression(Box::new(Expr::Literal {
                            value: Literal::Integer(fields.len() as i64 + 1),
                            location: location.clone(),
                        })), value));
                    }
                } else if self.match_token(TokenType::LeftBracket) {
                    // [expr] = value
                    let key_expr = self.expression()?;
                    self.consume(TokenType::RightBracket, "Expected ']' after table key")?;
                    self.consume(TokenType::Assign, "Expected '=' after table key")?;
                    let value = self.expression()?;
                    fields.push((TableKey::Expression(Box::new(key_expr)), value));
                } else {
                    // Just a value
                    let value = self.expression()?;
                    fields.push((TableKey::Expression(Box::new(Expr::Literal {
                        value: Literal::Integer(fields.len() as i64 + 1),
                        location: location.clone(),
                    })), value));
                }

                if !self.match_token(TokenType::Comma) {
                    break;
                }
            }
        }

        self.consume(TokenType::RightBrace, "Expected '}' after table literal")?;

        Ok(Expr::Table { fields, location })
    }

    // ===== Type Parsing =====

    fn parse_type(&mut self) -> LuxResult<Type> {
        // Check for pointer type: *T
        if self.match_token(TokenType::Star) {
            let inner_type = self.parse_type()?;
            return Ok(Type::Pointer(Box::new(inner_type)));
        }

        if self.match_keyword(Keyword::Int) {
            Ok(Type::Int)
        } else if self.match_keyword(Keyword::Float) {
            Ok(Type::Float)
        } else if self.match_keyword(Keyword::String) {
            Ok(Type::String)
        } else if self.match_keyword(Keyword::Bool) {
            Ok(Type::Bool)
        } else if self.match_keyword(Keyword::Nil) {
            Ok(Type::Nil)
        } else if self.match_keyword(Keyword::Table) {
            Ok(Type::Table)
        } else {
            Err(LuxError::parse_error(
                "Expected type",
                self.peek().location.clone(),
            ))
        }
    }

    // ===== Helper Methods =====

    fn match_token(&mut self, token_type: TokenType) -> bool {
        if self.check(token_type) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn match_tokens(&mut self, types: &[TokenType]) -> bool {
        for t in types {
            if self.check(t.clone()) {
                self.advance();
                return true;
            }
        }
        false
    }

    fn match_keyword(&mut self, keyword: Keyword) -> bool {
        if self.check_keyword(keyword) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn check(&self, token_type: TokenType) -> bool {
        if self.is_at_end() {
            false
        } else {
            std::mem::discriminant(&self.peek().token_type) == std::mem::discriminant(&token_type)
        }
    }

    fn check_keyword(&self, keyword: Keyword) -> bool {
        if self.is_at_end() {
            false
        } else {
            matches!(&self.peek().token_type, TokenType::Keyword(k) if k == &keyword)
        }
    }

    fn advance(&mut self) -> &Token {
        if !self.is_at_end() {
            self.current += 1;
        }
        self.previous()
    }

    fn is_at_end(&self) -> bool {
        matches!(self.peek().token_type, TokenType::Eof)
    }

    fn peek(&self) -> &Token {
        &self.tokens[self.current]
    }

    fn previous(&self) -> &Token {
        &self.tokens[self.current - 1]
    }

    fn consume(&mut self, token_type: TokenType, message: &str) -> LuxResult<&Token> {
        if self.check(token_type) {
            Ok(self.advance())
        } else {
            Err(LuxError::parse_error(message, self.peek().location.clone()))
        }
    }

    fn consume_keyword(&mut self, keyword: Keyword, message: &str) -> LuxResult<&Token> {
        if self.check_keyword(keyword) {
            Ok(self.advance())
        } else {
            Err(LuxError::parse_error(message, self.peek().location.clone()))
        }
    }

    fn consume_identifier(&mut self, message: &str) -> LuxResult<String> {
        if self.check(TokenType::Identifier) {
            Ok(self.advance().lexeme.clone())
        } else {
            Err(LuxError::parse_error(message, self.peek().location.clone()))
        }
    }
}

// Helper method for Expr to get location
impl Expr {
    pub fn location(&self) -> &SourceLocation {
        match self {
            Expr::Literal { location, .. }
            | Expr::Variable { location, .. }
            | Expr::Binary { location, .. }
            | Expr::Unary { location, .. }
            | Expr::Assign { location, .. }
            | Expr::Call { location, .. }
            | Expr::Table { location, .. }
            | Expr::TableAccess { location, .. }
            | Expr::Logical { location, .. }
            | Expr::Function { location, .. }
            | Expr::Spawn { location, .. }
            | Expr::Await { location, .. } => location,
        }
    }
}

