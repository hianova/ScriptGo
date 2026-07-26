#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use alloc::format;
use alloc::string::String;
use alloc::vec::Vec;

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Position {
    pub line: usize,
    pub column: usize,
}

impl Position {
    pub fn new(line: usize, column: usize) -> Self {
        Self { line, column }
    }
}

#[derive(Debug, Clone, PartialEq)]
pub enum Token {
    IntLiteral(i64),
    FloatLiteral(f64),
    StringLiteral(String),
    Identifier(String),
    Let,
    Fn,
    If,
    Else,
    While,
    Return,
    Equal,      // =
    EqualEqual, // ==
    Plus,       // +
    Minus,      // -
    Star,       // *
    Slash,      // /
    Percent,    // %
    Lt,         // <
    Gt,         // >
    Le,         // <=
    Ge,         // >=
    NotEqual,   // !=
    AndAnd,     // &&
    OrOr,       // ||
    Arrow,      // ->
    PlusEqual,  // +=
    MinusEqual, // -=
    LParen,     // (
    RParen,     // )
    LBrace,     // {
    RBrace,     // }
    LBracket,   // [
    RBracket,   // ]
    Comma,      // ,
    Colon,      // :
    Semicolon,  // ;
    Dot,        // .
    Bang,       // !
    EOF,
}

#[derive(Debug, Clone, PartialEq)]
pub struct SpannedToken {
    pub token: Token,
    pub pos: Position,
}

pub struct Lexer<'a> {
    _input: &'a str,
    chars: Vec<(usize, char)>, // (byte_offset, char)
    index: usize,
    line: usize,
    column: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        let chars: Vec<(usize, char)> = input.char_indices().collect();
        Self {
            _input: input,
            chars,
            index: 0,
            line: 1,
            column: 1,
        }
    }

    fn current_char(&self) -> Option<char> {
        self.chars.get(self.index).map(|&(_, c)| c)
    }

    fn peek_char(&self) -> Option<char> {
        self.chars.get(self.index + 1).map(|&(_, c)| c)
    }

    fn current_pos(&self) -> Position {
        Position::new(self.line, self.column)
    }

    fn advance(&mut self) -> Option<char> {
        if let Some(&(_, c)) = self.chars.get(self.index) {
            self.index += 1;
            if c == '\n' {
                self.line += 1;
                self.column = 1;
            } else {
                self.column += 1;
            }
            Some(c)
        } else {
            None
        }
    }

    fn skip_whitespace_and_comments(&mut self) -> Result<(), String> {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
            } else if c == '/' && self.peek_char() == Some('/') {
                // Line comment
                self.advance(); // consume '/'
                self.advance(); // consume '/'
                while let Some(ch) = self.current_char() {
                    if ch == '\n' {
                        self.advance();
                        break;
                    }
                    self.advance();
                }
            } else if c == '/' && self.peek_char() == Some('*') {
                // Block comment
                let start_pos = self.current_pos();
                self.advance(); // consume '/'
                self.advance(); // consume '*'
                let mut closed = false;
                while let Some(ch) = self.current_char() {
                    if ch == '*' && self.peek_char() == Some('/') {
                        self.advance(); // consume '*'
                        self.advance(); // consume '/'
                        closed = true;
                        break;
                    }
                    self.advance();
                }
                if !closed {
                    return Err(format!(
                        "Syntax error at line {}, column {}: Unterminated block comment",
                        start_pos.line, start_pos.column
                    ));
                }
            } else {
                break;
            }
        }
        Ok(())
    }

    pub fn tokenize(&mut self) -> Result<Vec<SpannedToken>, String> {
        let mut tokens = Vec::new();

        self.skip_whitespace_and_comments()?;
        while self.index < self.chars.len() {
            let pos = self.current_pos();
            let c = match self.current_char() {
                Some(ch) => ch,
                None => break,
            };

            match c {
                '+' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::PlusEqual, pos });
                    } else {
                        tokens.push(SpannedToken { token: Token::Plus, pos });
                    }
                }
                '-' => {
                    self.advance();
                    if self.current_char() == Some('>') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::Arrow, pos });
                    } else if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::MinusEqual, pos });
                    } else {
                        tokens.push(SpannedToken { token: Token::Minus, pos });
                    }
                }
                '*' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Star, pos });
                }
                '/' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Slash, pos });
                }
                '%' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Percent, pos });
                }
                '<' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::Le, pos });
                    } else {
                        tokens.push(SpannedToken { token: Token::Lt, pos });
                    }
                }
                '>' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::Ge, pos });
                    } else {
                        tokens.push(SpannedToken { token: Token::Gt, pos });
                    }
                }
                '=' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::EqualEqual, pos });
                    } else {
                        tokens.push(SpannedToken { token: Token::Equal, pos });
                    }
                }
                '!' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::NotEqual, pos });
                    } else {
                        tokens.push(SpannedToken { token: Token::Bang, pos });
                    }
                }
                '&' => {
                    self.advance();
                    if self.current_char() == Some('&') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::AndAnd, pos });
                    } else {
                        return Err(format!(
                            "Syntax error at line {}, column {}: Unrecognized character '&'",
                            pos.line, pos.column
                        ));
                    }
                }
                '|' => {
                    self.advance();
                    if self.current_char() == Some('|') {
                        self.advance();
                        tokens.push(SpannedToken { token: Token::OrOr, pos });
                    } else {
                        return Err(format!(
                            "Syntax error at line {}, column {}: Unrecognized character '|'",
                            pos.line, pos.column
                        ));
                    }
                }
                '(' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::LParen, pos });
                }
                ')' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::RParen, pos });
                }
                '{' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::LBrace, pos });
                }
                '}' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::RBrace, pos });
                }
                '[' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::LBracket, pos });
                }
                ']' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::RBracket, pos });
                }
                ',' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Comma, pos });
                }
                ':' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Colon, pos });
                }
                ';' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Semicolon, pos });
                }
                '.' => {
                    self.advance();
                    tokens.push(SpannedToken { token: Token::Dot, pos });
                }
                '"' => {
                    self.advance(); // consume opening quote
                    let mut s = String::new();
                    let mut terminated = false;
                    while let Some(ch) = self.current_char() {
                        if ch == '"' {
                            self.advance();
                            terminated = true;
                            break;
                        } else if ch == '\\' {
                            self.advance();
                            match self.current_char() {
                                Some('"') => {
                                    s.push('"');
                                    self.advance();
                                }
                                Some('\\') => {
                                    s.push('\\');
                                    self.advance();
                                }
                                Some('n') => {
                                    s.push('\n');
                                    self.advance();
                                }
                                Some('t') => {
                                    s.push('\t');
                                    self.advance();
                                }
                                Some('r') => {
                                    s.push('\r');
                                    self.advance();
                                }
                                Some('0') => {
                                    s.push('\0');
                                    self.advance();
                                }
                                Some(other) => {
                                    return Err(format!(
                                        "Syntax error at line {}, column {}: Invalid escape sequence '\\{}'",
                                        self.line, self.column, other
                                    ));
                                }
                                None => {
                                    return Err(format!(
                                        "Syntax error at line {}, column {}: Unterminated string literal",
                                        pos.line, pos.column
                                    ));
                                }
                            }
                        } else {
                            s.push(ch);
                            self.advance();
                        }
                    }
                    if !terminated {
                        return Err(format!(
                            "Syntax error at line {}, column {}: Unterminated string literal",
                            pos.line, pos.column
                        ));
                    }
                    tokens.push(SpannedToken {
                        token: Token::StringLiteral(s),
                        pos,
                    });
                }
                _ if c.is_ascii_digit() => {
                    let mut num_str = String::new();
                    let mut dot_count = 0;

                    while let Some(ch) = self.current_char() {
                        if ch.is_ascii_digit() {
                            num_str.push(ch);
                            self.advance();
                        } else if ch == '.' {
                            if self.peek_char().is_some_and(|next_c| next_c.is_ascii_digit()) {
                                dot_count += 1;
                                num_str.push(ch);
                                self.advance();
                            } else {
                                break;
                            }
                        } else {
                            break;
                        }
                    }

                    if dot_count == 0 {
                        match num_str.parse::<i64>() {
                            Ok(v) => tokens.push(SpannedToken {
                                token: Token::IntLiteral(v),
                                pos,
                            }),
                            Err(_) => {
                                return Err(format!(
                                    "Syntax error at line {}, column {}: Invalid integer literal '{}'",
                                    pos.line, pos.column, num_str
                                ));
                            }
                        }
                    } else if dot_count == 1 {
                        match num_str.parse::<f64>() {
                            Ok(v) => tokens.push(SpannedToken {
                                token: Token::FloatLiteral(v),
                                pos,
                            }),
                            Err(_) => {
                                return Err(format!(
                                    "Syntax error at line {}, column {}: Malformed float literal '{}'",
                                    pos.line, pos.column, num_str
                                ));
                            }
                        }
                    } else {
                        return Err(format!(
                            "Syntax error at line {}, column {}: Malformed float literal '{}'",
                            pos.line, pos.column, num_str
                        ));
                    }
                }
                _ if c.is_alphabetic() || c == '_' => {
                    let mut ident = String::new();
                    while let Some(ch) = self.current_char() {
                        if ch.is_alphanumeric() || ch == '_' {
                            ident.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    let tok = match ident.as_str() {
                        "let" => Token::Let,
                        "fn" => Token::Fn,
                        "if" => Token::If,
                        "else" => Token::Else,
                        "while" => Token::While,
                        "return" => Token::Return,
                        _ => Token::Identifier(ident),
                    };
                    tokens.push(SpannedToken { token: tok, pos });
                }
                _ => {
                    return Err(format!(
                        "Syntax error at line {}, column {}: Unrecognized character '{}'",
                        pos.line, pos.column, c
                    ));
                }
            }
            self.skip_whitespace_and_comments()?;
        }

        let eof_pos = self.current_pos();
        tokens.push(SpannedToken {
            token: Token::EOF,
            pos: eof_pos,
        });
        Ok(tokens)
    }
}
