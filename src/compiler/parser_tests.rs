#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
#[cfg(test)]
mod tests {
    use crate::compiler::ast::*;
    use crate::compiler::lexer::Lexer;
    use crate::compiler::parser::Parser;
    use alloc::boxed::Box;
    use alloc::vec;

    #[test]
    fn test_operator_precedence() {
        let input = "let res: Int = 1 + 2 * 3;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 1);
        if let Statement::LetDecl(_, _, expr) = &program.statements[0] {
            // Expected AST: 1 + (2 * 3)
            let expected = Expr::BinaryOp(
                Box::new(Expr::IntLiteral(1)),
                BinaryOperator::Add,
                Box::new(Expr::BinaryOp(
                    Box::new(Expr::IntLiteral(2)),
                    BinaryOperator::Mul,
                    Box::new(Expr::IntLiteral(covopt_param!("M_29_46", 3))),
                )),
            );
            assert_eq!(*expr, expected);
        } else {
            panic!("Expected LetDecl statement");
        }
    }

    #[test]
    fn test_operator_precedence_comparison_and_logical() {
        let input = "let cond: Int = a + b < c * d && x == y;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        if let Statement::LetDecl(_, _, expr) = &program.statements[0] {
            // Expected AST: ((a + b) < (c * d)) && (x == y)
            let left_add = Expr::BinaryOp(
                Box::new(Expr::Identifier("a".into())),
                BinaryOperator::Add,
                Box::new(Expr::Identifier("b".into())),
            );
            let right_mul = Expr::BinaryOp(
                Box::new(Expr::Identifier("c".into())),
                BinaryOperator::Mul,
                Box::new(Expr::Identifier("d".into())),
            );
            let cmp_lt = Expr::BinaryOp(
                Box::new(left_add),
                BinaryOperator::Lt,
                Box::new(right_mul),
            );
            let cmp_eq = Expr::BinaryOp(
                Box::new(Expr::Identifier("x".into())),
                BinaryOperator::Eq,
                Box::new(Expr::Identifier("y".into())),
            );
            let expected = Expr::BinaryOp(
                Box::new(cmp_lt),
                BinaryOperator::And,
                Box::new(cmp_eq),
            );
            assert_eq!(*expr, expected);
        } else {
            panic!("Expected LetDecl statement");
        }
    }

    #[test]
    fn test_enforce_mandatory_semicolons() {
        let input = "let x = 5 let y = 10;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let res = parser.parse();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Expected ';'"));
        assert!(err.contains("line 1, column 11"));
    }

    #[test]
    fn test_method_chaining_and_closing_paren_validation() {
        let input = "a.b(1).c(2);";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 1);
        if let Statement::ExprStmt(expr) = &program.statements[0] {
            let inner_call = Expr::MethodCall(
                Box::new(Expr::Identifier("a".into())),
                "b".into(),
                vec![Expr::IntLiteral(1)],
            );
            let expected = Expr::MethodCall(
                Box::new(inner_call),
                "c".into(),
                vec![Expr::IntLiteral(2)],
            );
            assert_eq!(*expr, expected);
        } else {
            panic!("Expected ExprStmt statement");
        }

        // Test unclosed parenthesis in method call
        let unclosed = "a.b(1, 2;";
        let mut lexer_unclosed = Lexer::new(unclosed);
        let tokens_unclosed = lexer_unclosed.tokenize().unwrap();
        let mut parser_unclosed = Parser::new(tokens_unclosed);
        let res = parser_unclosed.parse();
        assert!(res.is_err());
        assert!(res.unwrap_err().contains("Expected ')'"));
    }

    #[test]
    fn test_malformed_return_type_arrow_rejection() {
        let input = "fn foo() -> { return 1; }";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let res = parser.parse();
        assert!(res.is_err());
        let err = res.unwrap_err();
        assert!(err.contains("Expected return type after '->'"));
    }

    #[test]
    fn test_valid_function_declaration() {
        let input = "fn add(a: Int, b: Int) -> Int { return a + b; }";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        assert_eq!(program.statements.len(), 1);
        if let Statement::FunctionDecl(name, args, ret_ty, body) = &program.statements[0] {
            assert_eq!(name, "add");
            assert_eq!(args, &vec![("a".into(), Type::Int), ("b".into(), Type::Int)]);
            assert_eq!(*ret_ty, Type::Int);
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected FunctionDecl statement");
        }
    }

    #[test]
    fn test_function_return_type_arrow_no_double_advance() {
        let input1 = "fn test1() -> Int { let a = 10 - 1; return a; }";
        let mut lexer1 = Lexer::new(input1);
        let tokens1 = lexer1.tokenize().unwrap();
        let mut parser1 = Parser::new(tokens1);
        let program1 = parser1.parse().unwrap();

        if let Statement::FunctionDecl(name, args, ret_ty, body) = &program1.statements[0] {
            assert_eq!(name, "test1");
            assert!(args.is_empty());
            assert_eq!(*ret_ty, Type::Int);
            assert_eq!(body.len(), 2);
        } else {
            panic!("Expected FunctionDecl statement");
        }

        // Test split minus and gt tokens: - > Int
        let input2 = "fn test2() - > Int { return 42; }";
        let mut lexer2 = Lexer::new(input2);
        let tokens2 = lexer2.tokenize().unwrap();
        let mut parser2 = Parser::new(tokens2);
        let program2 = parser2.parse().unwrap();

        if let Statement::FunctionDecl(name, args, ret_ty, body) = &program2.statements[0] {
            assert_eq!(name, "test2");
            assert!(args.is_empty());
            assert_eq!(*ret_ty, Type::Int);
            assert_eq!(body.len(), 1);
        } else {
            panic!("Expected FunctionDecl statement");
        }
    }

    #[test]
    fn test_all_operators_codegen_lowering() {
        use crate::compiler::codegen::CodeGen;

        let input = r#"
            let a = 10 % 3;
            let b = 1 <= 2;
            let c = 3 >= 2;
            let d = 4 != 5;
            let e = 1 && 1;
            let f = 0 || 1;
        "#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut codegen = CodeGen::new();
        let result = codegen.compile(&program);
        assert!(result.is_ok(), "CodeGen lowering failed: {:?}", result.err());

        let bytecode = result.unwrap();
        assert!(!bytecode.is_empty());
    }

    #[test]
    fn test_ui_call_opcode_emission() {
        use crate::compiler::codegen::CodeGen;
        use crate::sgl::instruction::OpCode;

        let input = "ui_call(1, 2, 3);";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).expect("CodeGen failed");

        // First instruction in bytecode should be LoadImm for args, followed by UiCall opcode (36)
        let ui_call_inst = bytecode.iter().find(|inst| crate::opcode!(**inst) == OpCode::UiCall as u8);
        assert!(ui_call_inst.is_some(), "Expected UiCall opcode 36 in bytecode");
        
        // Ensure opcode 254 is NOT emitted anywhere
        let invalid_inst = bytecode.iter().find(|inst| crate::opcode!(**inst) == covopt_param!("M_236_81", 254));
        assert!(invalid_inst.is_none(), "Opcode 254 must not be emitted");
    }

    #[test]
    fn test_number_method_call_parsing() {
        let input = "123.foo();";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        assert_eq!(program.statements.len(), 1);
        if let Statement::ExprStmt(Expr::MethodCall(target, method_name, args)) = &program.statements[0] {
            assert_eq!(**target, Expr::IntLiteral(123));
            assert_eq!(method_name, "foo");
            assert!(args.is_empty());
        } else {
            panic!("Expected MethodCall on IntLiteral");
        }
    }
}
