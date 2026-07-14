use crate::compiler::ast::*;
use crate::compiler::lexer::*;
use alloc::boxed::Box;
use alloc::string::String;
use alloc::vec::Vec;

pub struct Parser {
    tokens: Vec<Token>,
    position: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Token>) -> Self {
        Self { tokens, position: 0 }
    }

    fn current(&self) -> Option<&Token> {
        self.tokens.get(self.position)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn match_token(&mut self, expected: &Token) -> bool {
        if let Some(token) = self.current() {
            if token == expected {
                self.advance();
                return true;
            }
        }
        false
    }

    pub fn parse(&mut self) -> Result<Program, String> {
        let mut statements = Vec::new();
        while self.current() != Some(&Token::EOF) && self.current().is_some() {
            statements.push(self.parse_statement()?);
        }
        Ok(Program { statements })
    }

    fn parse_statement(&mut self) -> Result<Statement, String> {
        if self.match_token(&Token::Let) {
            let ident = match self.current() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err("Expected identifier after 'let'".into()),
            };
            self.advance();
            
            let mut ty = Type::Int;
            if self.match_token(&Token::Colon) {
                if let Some(Token::Identifier(tname)) = self.current() {
                    ty = match tname.as_str() {
                        "Int" => Type::Int,
                        "Float" => Type::Float,
                        "String" => Type::String,
                        "Tensor" => Type::Tensor,
                        "DynamicArray" => Type::DynamicArray,
                        _ => return Err("Unknown type".into()),
                    };
                    self.advance();
                }
            }

            if !self.match_token(&Token::Equal) {
                return Err("Expected '=' in let declaration".into());
            }

            let expr = self.parse_expr()?;
            self.match_token(&Token::Semicolon);
            return Ok(Statement::LetDecl(ident, ty, expr));
        }

        if self.match_token(&Token::If) {
            let condition = self.parse_expr()?;
            if !self.match_token(&Token::LBrace) {
                return Err("Expected '{' after if condition".into());
            }
            let mut then_branch = Vec::new();
            while !self.match_token(&Token::RBrace) {
                then_branch.push(self.parse_statement()?);
            }
            let mut else_branch = Vec::new();
            if self.match_token(&Token::Else) {
                if !self.match_token(&Token::LBrace) {
                    return Err("Expected '{' after else".into());
                }
                while !self.match_token(&Token::RBrace) {
                    else_branch.push(self.parse_statement()?);
                }
            }
            return Ok(Statement::If(condition, then_branch, else_branch));
        }

        if self.match_token(&Token::While) {
            let condition = self.parse_expr()?;
            if !self.match_token(&Token::LBrace) {
                return Err("Expected '{' after while condition".into());
            }
            let mut body = Vec::new();
            while !self.match_token(&Token::RBrace) {
                body.push(self.parse_statement()?);
            }
            return Ok(Statement::While(condition, body));
        }

        if self.match_token(&Token::Return) {
            if self.match_token(&Token::Semicolon) {
                return Ok(Statement::Return(None));
            }
            let expr = self.parse_expr()?;
            self.match_token(&Token::Semicolon);
            return Ok(Statement::Return(Some(expr)));
        }

        if self.match_token(&Token::Fn) {
            let name = match self.current() {
                Some(Token::Identifier(name)) => name.clone(),
                _ => return Err("Expected function name".into()),
            };
            self.advance();

            if !self.match_token(&Token::LParen) {
                return Err("Expected '(' after function name".into());
            }

            let mut args = Vec::new();
            if !self.match_token(&Token::RParen) {
                loop {
                    let arg_name = match self.current() {
                        Some(Token::Identifier(name)) => name.clone(),
                        _ => return Err("Expected argument name".into()),
                    };
                    self.advance();
                    if !self.match_token(&Token::Colon) {
                        return Err("Expected ':' after argument name".into());
                    }
                    let ty = match self.current() {
                        Some(Token::Identifier(tname)) => match tname.as_str() {
                            "Int" => Type::Int,
                            "Float" => Type::Float,
                            "String" => Type::String,
                            "Tensor" => Type::Tensor,
                            "DynamicArray" => Type::DynamicArray,
                            _ => return Err("Unknown type".into()),
                        },
                        _ => return Err("Expected type for argument".into()),
                    };
                    self.advance();
                    args.push((arg_name, ty));

                    if !self.match_token(&Token::Comma) {
                        break;
                    }
                }
                if !self.match_token(&Token::RParen) {
                    return Err("Expected ')'".into());
                }
            }

            let mut ret_ty = Type::Void;
            if self.match_token(&Token::Minus) && self.match_token(&Token::Gt) { // ->
                if let Some(Token::Identifier(tname)) = self.current() {
                    ret_ty = match tname.as_str() {
                        "Int" => Type::Int,
                        "Float" => Type::Float,
                        "String" => Type::String,
                        "Tensor" => Type::Tensor,
                        "DynamicArray" => Type::DynamicArray,
                        _ => return Err("Unknown return type".into()),
                    };
                    self.advance();
                }
            }

            if !self.match_token(&Token::LBrace) {
                return Err("Expected '{' after function signature".into());
            }

            let mut body = Vec::new();
            while !self.match_token(&Token::RBrace) {
                body.push(self.parse_statement()?);
            }

            return Ok(Statement::FunctionDecl(name, args, ret_ty, body));
        }

        // It could be an assignment or just an expr stmt
        let expr = self.parse_expr()?;
        if self.match_token(&Token::Equal) {
            if let Expr::Identifier(ident) = expr {
                let right = self.parse_expr()?;
                self.match_token(&Token::Semicolon);
                return Ok(Statement::Assign(ident, right));
            } else {
                return Err("Invalid assignment target".into());
            }
        }
        
        self.match_token(&Token::Semicolon);
        Ok(Statement::ExprStmt(expr))
    }

    fn parse_expr(&mut self) -> Result<Expr, String> {
        let mut left = self.parse_primary()?;

        while let Some(tok) = self.current() {
            let op = match tok {
                Token::Plus => BinaryOperator::Add,
                Token::Minus => BinaryOperator::Sub,
                Token::Star => BinaryOperator::Mul,
                Token::Slash => BinaryOperator::Div,
                Token::EqualEqual => BinaryOperator::Eq,
                Token::Lt => BinaryOperator::Lt,
                Token::Gt => BinaryOperator::Gt,
                _ => break,
            };
            self.advance();
            let right = self.parse_primary()?;
            left = Expr::BinaryOp(Box::new(left), op, Box::new(right));
        }

        Ok(left)
    }

    fn parse_primary(&mut self) -> Result<Expr, String> {
        let token = self.current().cloned();
        match token {
            Some(Token::IntLiteral(v)) => {
                self.advance();
                Ok(Expr::IntLiteral(v))
            }
            Some(Token::FloatLiteral(v)) => {
                self.advance();
                Ok(Expr::FloatLiteral(v))
            }
            Some(Token::StringLiteral(v)) => {
                self.advance();
                Ok(Expr::StringLiteral(v))
            }
            Some(Token::Identifier(v)) => {
                self.advance();
                if self.match_token(&Token::LParen) {
                    let mut args = Vec::new();
                    if !self.match_token(&Token::RParen) {
                        loop {
                            args.push(self.parse_expr()?);
                            if !self.match_token(&Token::Comma) {
                                break;
                            }
                        }
                        if !self.match_token(&Token::RParen) {
                            return Err("Expected ')'".into());
                        }
                    }
                    Ok(Expr::Call(v, args))
                } else if self.match_token(&Token::Dot) {
                    // Method chaining support
                    if let Some(Token::Identifier(method_name)) = self.current().cloned() {
                        self.advance();
                        if self.match_token(&Token::LParen) {
                            let mut args = Vec::new();
                            if !self.match_token(&Token::RParen) {
                                loop {
                                    args.push(self.parse_expr()?);
                                    if !self.match_token(&Token::Comma) {
                                        break;
                                    }
                                }
                                self.match_token(&Token::RParen);
                            }
                            Ok(Expr::MethodCall(Box::new(Expr::Identifier(v)), method_name, args))
                        } else {
                            Err("Expected '(' after method name".into())
                        }
                    } else {
                        Err("Expected method name".into())
                    }
                } else {
                    Ok(Expr::Identifier(v))
                }
            }
            Some(Token::LParen) => {
                self.advance();
                let expr = self.parse_expr()?;
                if !self.match_token(&Token::RParen) {
                    return Err("Expected ')'".into());
                }
                Ok(expr)
            }
            _ => Err("Unexpected token".into()),
        }
    }
}
