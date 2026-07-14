use alloc::string::String;
use alloc::vec::Vec;

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
    Equal,       // =
    EqualEqual,  // ==
    Plus,        // +
    Minus,       // -
    Star,        // *
    Slash,       // /
    Percent,     // %
    Lt,          // <
    Gt,          // >
    LParen,      // (
    RParen,      // )
    LBrace,      // {
    RBrace,      // }
    LBracket,    // [
    RBracket,    // ]
    Comma,       // ,
    Colon,       // :
    Semicolon,   // ;
    Dot,         // .
    EOF,
}

pub struct Lexer<'a> {
    input: &'a str,
    position: usize,
}

impl<'a> Lexer<'a> {
    pub fn new(input: &'a str) -> Self {
        Self { input, position: 0 }
    }

    fn current_char(&self) -> Option<char> {
        self.input.chars().nth(self.position)
    }

    fn advance(&mut self) {
        self.position += 1;
    }

    fn skip_whitespace(&mut self) {
        while let Some(c) = self.current_char() {
            if c.is_whitespace() {
                self.advance();
            } else {
                break;
            }
        }
    }

    pub fn tokenize(&mut self) -> Vec<Token> {
        let mut tokens = Vec::new();

        self.skip_whitespace();
        while self.position < self.input.len() {
            let Some(c) = self.current_char() else { break; };

            match c {
                '+' => { tokens.push(Token::Plus); self.advance(); },
                '-' => { tokens.push(Token::Minus); self.advance(); },
                '*' => { tokens.push(Token::Star); self.advance(); },
                '/' => { tokens.push(Token::Slash); self.advance(); },
                '%' => { tokens.push(Token::Percent); self.advance(); },
                '<' => { tokens.push(Token::Lt); self.advance(); },
                '>' => { tokens.push(Token::Gt); self.advance(); },
                '(' => { tokens.push(Token::LParen); self.advance(); },
                ')' => { tokens.push(Token::RParen); self.advance(); },
                '{' => { tokens.push(Token::LBrace); self.advance(); },
                '}' => { tokens.push(Token::RBrace); self.advance(); },
                '[' => { tokens.push(Token::LBracket); self.advance(); },
                ']' => { tokens.push(Token::RBracket); self.advance(); },
                ',' => { tokens.push(Token::Comma); self.advance(); },
                ':' => { tokens.push(Token::Colon); self.advance(); },
                ';' => { tokens.push(Token::Semicolon); self.advance(); },
                '.' => { tokens.push(Token::Dot); self.advance(); },
                '=' => {
                    self.advance();
                    if self.current_char() == Some('=') {
                        tokens.push(Token::EqualEqual);
                        self.advance();
                    } else {
                        tokens.push(Token::Equal);
                    }
                }
                '"' => {
                    self.advance();
                    let mut s = String::new();
                    while let Some(ch) = self.current_char() {
                        if ch == '"' {
                            self.advance();
                            break;
                        }
                        s.push(ch);
                        self.advance();
                    }
                    tokens.push(Token::StringLiteral(s));
                }
                _ if c.is_ascii_digit() => {
                    let mut num = String::new();
                    let mut is_float = false;
                    while let Some(ch) = self.current_char() {
                        if ch.is_ascii_digit() {
                            num.push(ch);
                            self.advance();
                        } else if ch == '.' {
                            is_float = true;
                            num.push(ch);
                            self.advance();
                        } else {
                            break;
                        }
                    }
                    if is_float {
                        tokens.push(Token::FloatLiteral(num.parse().unwrap_or(0.0)));
                    } else {
                        tokens.push(Token::IntLiteral(num.parse().unwrap_or(0)));
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
                    match ident.as_str() {
                        "let" => tokens.push(Token::Let),
                        "fn" => tokens.push(Token::Fn),
                        "if" => tokens.push(Token::If),
                        "else" => tokens.push(Token::Else),
                        "while" => tokens.push(Token::While),
                        "return" => tokens.push(Token::Return),
                        _ => tokens.push(Token::Identifier(ident)),
                    }
                }
                _ => { self.advance(); }
            }
            self.skip_whitespace();
        }
        tokens.push(Token::EOF);
        tokens
    }
}
