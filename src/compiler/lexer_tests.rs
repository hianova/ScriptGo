#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
#[cfg(test)]
mod tests {
    use alloc::vec;
    use alloc::vec::Vec;
    use crate::compiler::lexer::{Lexer, Position, Token};

    #[test]
    fn test_valid_tokens_and_positions() {
        let input = "let x = 123;\nfn foo() -> Int { return x + 4.5; }";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        assert_eq!(tokens[0].token, Token::Let);
        assert_eq!(tokens[0].pos, Position::new(1, 1));

        assert_eq!(tokens[1].token, Token::Identifier("x".into()));
        assert_eq!(tokens[1].pos, Position::new(1, 5));

        assert_eq!(tokens[2].token, Token::Equal);
        assert_eq!(tokens[3].token, Token::IntLiteral(123));
        assert_eq!(tokens[4].token, Token::Semicolon);

        assert_eq!(tokens[5].token, Token::Fn);
        assert_eq!(tokens[5].pos, Position::new(2, 1));

        assert_eq!(tokens[6].token, Token::Identifier("foo".into()));
        assert_eq!(tokens[7].token, Token::LParen);
        assert_eq!(tokens[8].token, Token::RParen);
        assert_eq!(tokens[9].token, Token::Arrow);
        assert_eq!(tokens[10].token, Token::Identifier("Int".into()));
        assert_eq!(tokens[11].token, Token::LBrace);
        assert_eq!(tokens[12].token, Token::Return);
        assert_eq!(tokens[13].token, Token::Identifier("x".into()));
        assert_eq!(tokens[14].token, Token::Plus);
        assert_eq!(tokens[15].token, Token::FloatLiteral(4.5));
        assert_eq!(tokens[16].token, Token::Semicolon);
        assert_eq!(tokens[17].token, Token::RBrace);
        assert_eq!(tokens[18].token, Token::EOF);
    }

    #[test]
    fn test_string_literals_and_escapes() {
        let input = r#""hello world" "escaped \" quotes \\ and \n newline""#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        assert_eq!(tokens[0].token, Token::StringLiteral("hello world".into()));
        assert_eq!(
            tokens[1].token,
            Token::StringLiteral("escaped \" quotes \\ and \n newline".into())
        );
    }

    #[test]
    fn test_compound_operators() {
        let input = "<= >= != && || -> += -=";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let expected_tokens = vec![
            Token::Le,
            Token::Ge,
            Token::NotEqual,
            Token::AndAnd,
            Token::OrOr,
            Token::Arrow,
            Token::PlusEqual,
            Token::MinusEqual,
            Token::EOF,
        ];

        let actual_tokens: Vec<Token> = tokens.into_iter().map(|t| t.token).collect();
        assert_eq!(actual_tokens, expected_tokens);
    }

    #[test]
    fn test_comments() {
        let input = "// line comment\nlet x = 1; /* block comment */ let y = 2;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let actual_tokens: Vec<Token> = tokens.into_iter().map(|t| t.token).collect();
        assert_eq!(
            actual_tokens,
            vec![
                Token::Let,
                Token::Identifier("x".into()),
                Token::Equal,
                Token::IntLiteral(1),
                Token::Semicolon,
                Token::Let,
                Token::Identifier("y".into()),
                Token::Equal,
                Token::IntLiteral(2),
                Token::Semicolon,
                Token::EOF,
            ]
        );
    }

    #[test]
    fn test_reject_unrecognized_char() {
        let input = "let x = @10;";
        let mut lexer = Lexer::new(input);
        let res = lexer.tokenize();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Unrecognized character '@'"));
        assert!(err.contains("line 1, column 9"));
    }

    #[test]
    fn test_reject_unterminated_string() {
        let input = "let s = \"unterminated string;";
        let mut lexer = Lexer::new(input);
        let res = lexer.tokenize();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Unterminated string literal"));
    }

    #[test]
    fn test_reject_malformed_float() {
        let input = "let f = 1.2.3;";
        let mut lexer = Lexer::new(input);
        let res = lexer.tokenize();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Malformed float literal '1.2.3'"));
    }

    #[test]
    fn test_number_method_call_disambiguation() {
        let input = "123.foo()";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let expected_tokens = vec![
            Token::IntLiteral(123),
            Token::Dot,
            Token::Identifier("foo".into()),
            Token::LParen,
            Token::RParen,
            Token::EOF,
        ];

        let actual_tokens: Vec<Token> = tokens.into_iter().map(|t| t.token).collect();
        assert_eq!(actual_tokens, expected_tokens);
    }
}
