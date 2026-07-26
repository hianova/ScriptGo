#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use crate::compiler::ast::*;
use crate::compiler::lexer::*;
use alloc::boxed::Box;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

pub struct Parser {
    tokens: Vec<SpannedToken>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<SpannedToken>) -> Self {
        Self {
            tokens,
            position: 0,
        }
    }

    fn current(&self) -> Option<&SpannedToken> {
        self.tokens.get(self.position)
    }

    fn current_token(&self) -> Option<&Token> {
        self.current().map(|st| &st.token)
    }

    fn peek_token(&self) -> Option<&Token> {
        self.tokens.get(self.position + 1).map(|st| &st.token)
    }

    fn current_pos(&self) -> Position {
        self.current()
            .map(|st| st.pos.clone())
            .unwrap_or_else(|| Position::new(1, 1))
    }

    fn advance(&mut self) {
        if self.position < self.tokens.len() {
            self.position += 1;
        }
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if self.current_token() == Some(expected) {
            self.advance();
            true
        } else {
            false
        }
    }

    fn expect_token(&mut self, expected: &Token, err_msg: &str) -> Result<(), String> {
        if self.current_token() == Some(expected) {
            self.advance();
            Ok(())
        } else {
            let pos = self.current_pos();
            Err(format!(
                "Syntax error at line {}, column {}: {}",
                pos.line, pos.column, err_msg
            ))
        }
    }

    fn error(&self, msg: &str) -> String {
        let pos = self.current_pos();
        format!("Syntax error at line {}, column {}: {}", pos.line, pos.column, msg)
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while self.current_token() != Some(&Token::EOF) && self.current_token().is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(&Token::Let) {
            let ident = match self.current_token() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err(self.error("Expected identifier after 'let'")),
            };
            self.advance();

            let mut ty = Type::Int;
            if self.match_token(&Token::Colon) {
                if let Some(Token::Identifier(tname)) = self.current_token() {
                    ty = match tname.as_str() {
                        "Int" => Type::Int,
                        "Float" => Type::Float,
                        "String" => Type::String,
                        "Tensor" => Type::Tensor,
                        "DynamicArray" => Type::DynamicArray,
                        _ => return Err(self.error("Unknown type")),
                    };
                    self.advance();
                } else {
                    return Err(self.error("Expected type name after ':'"));
                }
            }

            self.expect_token(&Token::Equal, "Expected '=' in let declaration")?;

            let expr = self.parse_expr()?;
            self.expect_token(&Token::Semicolon, "Expected ';' after let declaration")?;
            return Ok(Statement::LetDecl(ident, ty, expr));
        }

        if self.match_token(&Token::If) {
            let condition = self.parse_expr()?;
            self.expect_token(&Token::LBrace, "Expected '{' after if condition")?;

            let mut then_branch = Vec::new();
            while self.current_token() != Some(&Token::RBrace) && self.current_token() != Some(&Token::EOF) {
                then_branch.push(self.parse_statement()?);
            }
            self.expect_token(&Token::RBrace, "Expected '}' at end of if block")?;

            let mut else_branch = Vec::new();
            if self.match_token(&Token::Else) {
                self.expect_token(&Token::LBrace, "Expected '{' after else")?;
                while self.current_token() != Some(&Token::RBrace) && self.current_token() != Some(&Token::EOF) {
                    else_branch.push(self.parse_statement()?);
                }
                self.expect_token(&Token::RBrace, "Expected '}' at end of else block")?;
            }
            return Ok(Statement::If(condition, then_branch, else_branch));
        }

        if self.match_token(&Token::While) {
            let condition = self.parse_expr()?;
            self.expect_token(&Token::LBrace, "Expected '{' after while condition")?;

            let mut body = Vec::new();
            while self.current_token() != Some(&Token::RBrace) && self.current_token() != Some(&Token::EOF) {
                body.push(self.parse_statement()?);
            }
            self.expect_token(&Token::RBrace, "Expected '}' at end of while block")?;
            return Ok(Statement::While(condition, body));
        }

        if self.match_token(&Token::Return) {
            if self.match_token(&Token::Semicolon) {
                return Ok(Statement::Return(None));
            }
            let expr = self.parse_expr()?;
            self.expect_token(&Token::Semicolon, "Expected ';' after return value")?;
            return Ok(Statement::Return(Some(expr)));
        }

        if self.match_token(&Token::Fn) {
            let name = match self.current_token() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err(self.error("Expected function name")),
            };
            self.advance();

            self.expect_token(&Token::LParen, "Expected '(' after function name")?;

            let mut args = Vec::new();
            if !self.match_token(&Token::RParen) {
                loop {
                    let arg_name = match self.current_token() {
                        Some(Token::Identifier(name)) => name.clone(),
                        _ => return Err(self.error("Expected argument name")),
                    };
                    self.advance();
                    self.expect_token(&Token::Colon, "Expected ':' after argument name")?;
                    let ty = match self.current_token() {
                        Some(Token::Identifier(tname)) => match tname.as_str() {
                            "Int" => Type::Int,
                            "Float" => Type::Float,
                            "String" => Type::String,
                            "Tensor" => Type::Tensor,
                            "DynamicArray" => Type::DynamicArray,
                            _ => return Err(self.error("Unknown type")),
                        },
                        _ => return Err(self.error("Expected type for argument")),
                    };
                    self.advance();
                    args.push((arg_name, ty));

                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                self.expect_token(&Token::RParen, "Expected ')' after parameters")?;
            }

            let mut ret_ty = Type::Void;
            let has_arrow = if self.match_token(&Token::Arrow) {
                true
            } else if self.current_token() == Some(&Token::Minus) && self.peek_token() == Some(&Token::Gt) {
                self.advance(); // consume '-'
                self.advance(); // consume '>'
                true
            } else {
                false
            };

            if has_arrow {
                if let Some(Token::Identifier(tname)) = self.current_token() {
                    ret_ty = match tname.as_str() {
                        "Int" => Type::Int,
                        "Float" => Type::Float,
                        "String" => Type::String,
                        "Tensor" => Type::Tensor,
                        "DynamicArray" => Type::DynamicArray,
                        _ => return Err(self.error("Unknown return type")),
                    };
                    self.advance();
                } else {
                    return Err(self.error("Expected return type after '->'"));
                }
            }

            self.expect_token(&Token::LBrace, "Expected '{' after function signature")?;

            let mut body = Vec::new();
            while self.current_token() != Some(&Token::RBrace) && self.current_token() != Some(&Token::EOF) {
                body.push(self.parse_statement()?);
            }
            self.expect_token(&Token::RBrace, "Expected '}' at end of function body")?;

            return Ok(Statement::FunctionDecl(name, args, ret_ty, body));
        }

        let expr = self.parse_expr()?;

        if self.match_token(&Token::Equal) {
            if let Expr::Identifier(ident) = expr {
                let right = self.parse_expr()?;
                self.expect_token(&Token::Semicolon, "Expected ';' after assignment")?;
                return Ok(Statement::Assign(ident, right));
            } else {
                return Err(self.error("Invalid assignment target"));
            }
        } else if self.match_token(&Token::PlusEqual) {
            if let Expr::Identifier(ident) = &expr {
                let right = self.parse_expr()?;
                self.expect_token(&Token::Semicolon, "Expected ';' after assignment")?;
                let val = Expr::BinaryOp(Box::new(expr.clone()), BinaryOperator::Add, Box::new(right));
                return Ok(Statement::Assign(ident.clone(), val));
            } else {
                return Err(self.error("Invalid assignment target"));
            }
        } else if self.match_token(&Token::MinusEqual) {
            if let Expr::Identifier(ident) = &expr {
                let right = self.parse_expr()?;
                self.expect_token(&Token::Semicolon, "Expected ';' after assignment")?;
                let val = Expr::BinaryOp(Box::new(expr.clone()), BinaryOperator::Sub, Box::new(right));
                return Ok(Statement::Assign(ident.clone(), val));
            } else {
                return Err(self.error("Invalid assignment target"));
            }
        }

        self.expect_token(&Token::Semicolon, "Expected ';' after expression")?;
        Ok(Statement::ExprStmt(expr))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        self.parse_expr_with_precedence(0)
    }

    fn parse_expr_with_precedence(&mut self, min_prec: u8) -> Result<Expr, String> {
        let mut left = self.parse_primary()?;

        while let Some(spanned) = self.current() {
            let (op, prec) = match &spanned.token {
                Token::OrOr => (BinaryOperator::Or, 1),
                Token::AndAnd => (BinaryOperator::And, 2),
                Token::EqualEqual => (BinaryOperator::Eq, covopt_param!("M_279_58", 3)),
                Token::NotEqual => (BinaryOperator::Ne, covopt_param!("M_280_56", 3)),
                Token::Lt => (BinaryOperator::Lt, covopt_param!("M_281_50", 3)),
                Token::Gt => (BinaryOperator::Gt, covopt_param!("M_282_50", 3)),
                Token::Le => (BinaryOperator::Le, covopt_param!("M_283_50", 3)),
                Token::Ge => (BinaryOperator::Ge, covopt_param!("M_284_50", 3)),
                Token::Plus => (BinaryOperator::Add, covopt_param!("M_285_53", 4)),
                Token::Minus => (BinaryOperator::Sub, covopt_param!("M_286_54", 4)),
                Token::Star => (BinaryOperator::Mul, covopt_param!("M_287_53", 5)),
                Token::Slash => (BinaryOperator::Div, covopt_param!("M_288_54", 5)),
                Token::Percent => (BinaryOperator::Mod, covopt_param!("M_289_56", 5)),
                _ => break,
            };

            if prec < min_prec {
                break;
            }

            self.advance();
            let next_min_prec = prec + 1; // Left-associative
            let right = self.parse_expr_with_precedence(next_min_prec)?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let mut expr = match self.current_token().cloned() {
            Some(Token::IntLiteral(v)) => {
                self.advance();
                Expr::IntLiteral(v)
            }
            Some(Token::FloatLiteral(v)) => {
                self.advance();
                Expr::FloatLiteral(v)
            }
            Some(Token::StringLiteral(v)) => {
                self.advance();
                Expr::StringLiteral(v)
            }
            Some(Token::Identifier(v)) => {
                self.advance();
                let is_macro = self.match_token(&Token::Bang);

                if self.match_token(&Token::LParen) {
                    let mut args = Vec::new();
                    if !self.match_token(&Token::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                        self.expect_token(&Token::RParen, "Expected ')' after function arguments")?;
                    }
                    if is_macro {
                        Expr::MacroCall(v, args)
                    } else {
                        Expr::Call(v, args)
                    }
                } else {
                    Expr::Identifier(v)
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let inner = self.parse_expr()?;
                self.expect_token(&Token::RParen, "Expected ')' after parenthesized expression")?;
                inner
            }
            _ => return Err(self.error("Unexpected token in expression")),
        };

        // Handle method calls `.method(...)` on any primary expression
        while let Some(tok) = self.current_token() {
            if tok == &Token::Dot {
                self.advance();
                let method_name = match self.current_token().cloned() {
                    Some(Token::Identifier(name)) => name,
                    _ => return Err(self.error("Expected method name after '.'")),
                };
                self.advance();
                let is_macro = self.match_token(&Token::Bang);

                if !self.match_token(&Token::LParen) {
                    return Err(self.error("Expected '(' after method name"));
                }

                let mut args = Vec::new();
                if !self.match_token(&Token::RParen) {
                    loop {
                        args.push(self.parse_expr()?);
                        if !self.match_token(&Token::Comma) {
                            break;
                        }
                    }
                    self.expect_token(&Token::RParen, "Expected ')' after method arguments")?;
                }

                if is_macro {
                    let base_name = match &expr {
                        Expr::Identifier(name) => format!("{}.{}", name, method_name),
                        _ => format!("<expr>.{}", method_name),
                    };
                    expr = Expr::MacroCall(base_name, args);
                } else {
                    expr = Expr::MethodCall(Box::new(expr), method_name, args);
                }
            } else {
                break;
            }
        }

        Ok(expr)
    }
}
