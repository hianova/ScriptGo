#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
#[cfg(test)]
mod tests {
    use crate::compiler::ast::*;
    use crate::compiler::codegen::CodeGen;
    use crate::compiler::lexer::{Lexer, Token};
    use crate::compiler::parser::Parser;
    use crate::sgl::instruction::OpCode;
    use crate::sgl::vm::ScriptVm;
    use alloc::boxed::Box;
    use alloc::vec;
    use alloc::vec::Vec;
    use core::sync::atomic::{AtomicBool, AtomicUsize, Ordering};

    // =========================================================================
    // Task 1: Operator Precedence Stress Tests
    // =========================================================================

    #[test]
    fn test_precedence_multiplication_over_addition() {
        // Test: 1 + 2 * 3 == 7
        let input = "let res: Int = 1 + 2 * 3;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        assert_eq!(program.statements.len(), 1);
        if let Statement::LetDecl(name, _ty, expr) = &program.statements[0] {
            assert_eq!(name, "res");
            // AST expected: 1 + (2 * 3)
            let expected_ast = Expr::BinaryOp(
                Box::new(Expr::IntLiteral(1)),
                BinaryOperator::Add,
                Box::new(Expr::BinaryOp(
                    Box::new(Expr::IntLiteral(2)),
                    BinaryOperator::Mul,
                    Box::new(Expr::IntLiteral(covopt_param!("M_40_46", 3))),
                )),
            );
            assert_eq!(*expr, expected_ast);
        } else {
            panic!("Expected LetDecl statement");
        }

        // Empirical evaluation in VM
        let eval_input = "let x = 1 + 2 * 3;";
        let mut lexer = Lexer::new(eval_input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).unwrap();
        
        // Find stored variable value in VM registers (reg 6)
        let x_val = vm.registers[covopt_param!("M_61_33", 6)];
        assert_eq!(x_val, 7, "1 + 2 * 3 must evaluate to 7 in VM");

        // Verification of 1 + 2 * 3 == 7
        let eval_eq_input = "let res = (1 + 2 * 3) == 7;";
        let mut lexer = Lexer::new(eval_eq_input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).unwrap();
        // Variable res stored in reg 8
        assert_eq!(vm.registers[8], 1, "1 + 2 * 3 == 7 must evaluate to 1 (true)");
    }

    #[test]
    fn test_precedence_left_associativity_subtraction() {
        // Test: 10 - 2 - 3 == 5 (Left-associative: (10 - 2) - 3 = 5, not 10 - (2 - 3) = 11)
        let input = "let res: Int = 10 - 2 - 3;";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        assert_eq!(program.statements.len(), 1);
        if let Statement::LetDecl(_, _, expr) = &program.statements[0] {
            // AST expected: (10 - 2) - 3
            let expected_ast = Expr::BinaryOp(
                Box::new(Expr::BinaryOp(
                    Box::new(Expr::IntLiteral(covopt_param!("M_93_46", 10))),
                    BinaryOperator::Sub,
                    Box::new(Expr::IntLiteral(2)),
                )),
                BinaryOperator::Sub,
                Box::new(Expr::IntLiteral(covopt_param!("M_98_42", 3))),
            );
            assert_eq!(*expr, expected_ast);
        } else {
            panic!("Expected LetDecl statement");
        }

        // Empirical evaluation in VM
        let eval_input = "let x = 10 - 2 - 3;";
        let mut lexer = Lexer::new(eval_input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).unwrap();
        // Variable x stored in reg 6
        assert_eq!(vm.registers[6], 5, "10 - 2 - 3 must evaluate to 5 in VM");

        // Verification of 10 - 2 - 3 == 5
        let eval_eq_input = "let res = (10 - 2 - 3) == 5;";
        let mut lexer = Lexer::new(eval_eq_input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).unwrap();
        // Variable res stored in reg 8
        assert_eq!(vm.registers[8], 1, "10 - 2 - 3 == 5 must evaluate to 1 (true)");
    }

    #[test]
    fn test_precedence_complex_expressions() {
        // Test: 2 + 3 * 4 == 14 && 100 / 10 / 2 == 5
        let eval_input = "let a = 2 + 3 * 4; let b = 100 / 10 / 2;";
        let mut lexer = Lexer::new(eval_input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).unwrap();
        // a stored in reg 6, b stored in reg 12
        assert_eq!(vm.registers[6], 14, "2 + 3 * 4 must evaluate to 14");
        assert_eq!(vm.registers[12], 5, "100 / 10 / 2 must evaluate to 5");
    }

    // =========================================================================
    // Task 2: Compound and Logical Operators Stress Tests
    // =========================================================================

    #[test]
    fn test_all_compound_and_logical_operators_tokenization() {
        let input = "<= >= != && || % + - * / == < > += -=";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");

        let expected_tokens = vec![
            Token::Le,
            Token::Ge,
            Token::NotEqual,
            Token::AndAnd,
            Token::OrOr,
            Token::Percent,
            Token::Plus,
            Token::Minus,
            Token::Star,
            Token::Slash,
            Token::EqualEqual,
            Token::Lt,
            Token::Gt,
            Token::PlusEqual,
            Token::MinusEqual,
            Token::EOF,
        ];

        let actual_tokens: Vec<Token> = tokens.into_iter().map(|t| t.token).collect();
        assert_eq!(actual_tokens, expected_tokens);
    }

    #[test]
    fn test_all_compound_and_logical_operators_execution() {
        let input = r#"
            let mod_val = 10 % 3;
            let le_val = 5 <= 5;
            let ge_val = 6 >= 2;
            let ne_val = 7 != 8;
            let and_val = 1 && 1;
            let or_val = 0 || 1;
            let mut_val = 10;
            mut_val += 5;
            mut_val -= 3;
        "#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).expect("CodeGen failed");

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).expect("VM execution failed");

        // Verify values stored in destination registers:
        // mod_val -> reg 4
        // le_val -> reg 10
        // ge_val -> reg 16
        // ne_val -> reg 22
        // and_val -> reg 26
        // or_val -> reg 30
        // mut_val -> reg 32
        assert_eq!(vm.registers[4], 1, "10 % 3 must be 1");
        assert_eq!(vm.registers[10], 1, "5 <= 5 must be 1");
        assert_eq!(vm.registers[16], 1, "6 >= 2 must be 1");
        assert_eq!(vm.registers[22], 1, "7 != 8 must be 1");
        assert_eq!(vm.registers[26], 1, "1 && 1 must be 1");
        assert_eq!(vm.registers[30], 1, "0 || 1 must be 1");
        assert_eq!(vm.registers[32], 12, "10 + 5 - 3 must be 12");
    }

    #[test]
    fn test_logical_and_or_false_cases() {
        let input = r#"
            let and_false = 1 && 0;
            let or_false = 0 || 0;
            let ne_false = 5 != 5;
            let le_false = 6 <= 5;
            let ge_false = 2 >= 6;
        "#;
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();
        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.run(&bytecode).unwrap();

        // and_false -> reg 4
        // or_false -> reg 8
        // ne_false -> reg 14
        // le_false -> reg 20
        // ge_false -> reg 26
        assert_eq!(vm.registers[4], 0, "1 && 0 must be 0");
        assert_eq!(vm.registers[8], 0, "0 || 0 must be 0");
        assert_eq!(vm.registers[14], 0, "5 != 5 must be 0");
        assert_eq!(vm.registers[20], 0, "6 <= 5 must be 0");
        assert_eq!(vm.registers[26], 0, "2 >= 6 must be 0");
    }

    // =========================================================================
    // Task 3: Syntax Error Rejection Stress Tests
    // =========================================================================

    #[test]
    fn test_syntax_error_unrecognized_chars() {
        let invalid_inputs = [
            ("let x = @10;", "@"),
            ("let y = #foo;", "#"),
            ("let z = $bar;", "$"),
            ("let w = ^2;", "^"),
            ("let q = ?;", "?"),
        ];

        for (input, ch) in invalid_inputs {
            let mut lexer = Lexer::new(input);
            let res = lexer.tokenize();
            assert!(res.is_err(), "Expected error for unrecognized char '{}'", ch);
            let err = res.unwrap_err();
            assert!(
                err.contains("Unrecognized character"),
                "Error message should mention Unrecognized character: {}",
                err
            );
        }
    }

    #[test]
    fn test_syntax_error_unterminated_strings() {
        let invalid_strings = [
            "let s = \"unterminated string;",
            "let s = \"missing closing quote\n;",
            "let s = \"escaped quote at end \\\"",
        ];

        for input in invalid_strings {
            let mut lexer = Lexer::new(input);
            let res = lexer.tokenize();
            assert!(res.is_err(), "Expected error for unterminated string in: {}", input);
            let err = res.unwrap_err();
            assert!(
                err.contains("Unterminated string literal"),
                "Error message should mention Unterminated string literal: {}",
                err
            );
        }
    }

    #[test]
    fn test_syntax_error_bad_floats() {
        let bad_floats = [
            "let f = 1.2.3;",
            "let f = 10.20.30.40;",
            "let f = 0.1.2;",
        ];

        for input in bad_floats {
            let mut lexer = Lexer::new(input);
            let res = lexer.tokenize();
            assert!(res.is_err(), "Expected error for bad float in: {}", input);
            let err = res.unwrap_err();
            assert!(
                err.contains("Malformed float literal"),
                "Error message should mention Malformed float literal: {}",
                err
            );
        }
    }

    #[test]
    fn test_syntax_error_missing_semicolons() {
        let missing_semi_inputs = [
            "let x = 5 let y = 10;",
            "let x = 5",
            "x = 10",
            "return 42",
        ];

        for input in missing_semi_inputs {
            let mut lexer = Lexer::new(input);
            if let Ok(tokens) = lexer.tokenize() {
                let mut parser = Parser::new(tokens);
                let res = parser.parse();
                assert!(res.is_err(), "Expected parser error for missing semicolon in: {}", input);
                let err = res.unwrap_err();
                assert!(
                    err.contains("Expected ';'"),
                    "Error message should mention Expected ';': {}",
                    err
                );
            }
        }
    }

    // =========================================================================
    // Task 4: Method Call Tokenization Stress Tests
    // =========================================================================

    #[test]
    fn test_method_call_tokenization_number_disambiguation() {
        // Test: 123.foo()
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
        assert_eq!(actual_tokens, expected_tokens, "123.foo() must tokenize as IntLiteral(123), Dot, Identifier('foo'), LParen, RParen");
    }

    #[test]
    fn test_method_call_parsing_and_chaining() {
        // Test parsing 123.foo() and chained method calls 123.foo().bar(456)
        let input = "123.foo().bar(456);";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        assert_eq!(program.statements.len(), 1);
        if let Statement::ExprStmt(Expr::MethodCall(target, method_name, args)) = &program.statements[0] {
            assert_eq!(method_name, "bar");
            assert_eq!(args.len(), 1);
            assert_eq!(args[0], Expr::IntLiteral(456));

            // Inner target should be MethodCall 123.foo()
            if let Expr::MethodCall(inner_target, inner_method, inner_args) = &**target {
                assert_eq!(**inner_target, Expr::IntLiteral(123));
                assert_eq!(inner_method, "foo");
                assert!(inner_args.is_empty());
            } else {
                panic!("Expected inner MethodCall target");
            }
        } else {
            panic!("Expected MethodCall ExprStmt");
        }
    }

    // =========================================================================
    // Task 5: ui_call Opcode Emission Stress Tests (OpCode 36)
    // =========================================================================

    #[test]
    fn test_ui_call_opcode_emission_and_verification() {
        let input = "ui_call(1, 2, 3);";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().expect("Lexing failed");
        let mut parser = Parser::new(tokens);
        let program = parser.parse().expect("Parsing failed");

        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).expect("CodeGen failed");

        // Verify OpCode::UiCall as u8 == 36
        assert_eq!(OpCode::UiCall as u8, 36, "OpCode::UiCall numeric value must be 36");

        // Find instruction with opcode 36
        let ui_call_inst = bytecode.iter().find(|inst| crate::opcode!(**inst) == covopt_param!("M_422_81", 36));
        assert!(ui_call_inst.is_some(), "Bytecode must contain OpCode 36 (UiCall)");

        // Verify invalid/undefined opcode 254 is NOT in bytecode
        let invalid_inst = bytecode.iter().find(|inst| crate::opcode!(**inst) == covopt_param!("M_426_81", 254));
        assert!(invalid_inst.is_none(), "Opcode 254 must not be present in compiled bytecode");
    }

    #[test]
    fn test_ui_call_execution_in_vm() {
        static UI_INVOKED: AtomicBool = AtomicBool::new(false);
        static UI_REG_A: AtomicUsize = AtomicUsize::new(0);
        static UI_REG_B: AtomicUsize = AtomicUsize::new(0);
        static UI_REG_C: AtomicUsize = AtomicUsize::new(0);

        fn mock_ui_handler(reg_a: usize, reg_b: usize, reg_c: usize) {
            UI_INVOKED.store(true, Ordering::SeqCst);
            UI_REG_A.store(reg_a, Ordering::SeqCst);
            UI_REG_B.store(reg_b, Ordering::SeqCst);
            UI_REG_C.store(reg_c, Ordering::SeqCst);
        }

        let input = "ui_call(100, 200, 300);";
        let mut lexer = Lexer::new(input);
        let tokens = lexer.tokenize().unwrap();
        let mut parser = Parser::new(tokens);
        let program = parser.parse().unwrap();

        let mut codegen = CodeGen::new();
        let bytecode = codegen.compile(&program).unwrap();

        let mut vm = ScriptVm::new();
        vm.ui_handler = Some(mock_ui_handler);
        vm.run(&bytecode).unwrap();

        assert!(UI_INVOKED.load(Ordering::SeqCst), "ui_handler must be invoked by OpCode 36");
        let reg_a = UI_REG_A.load(Ordering::SeqCst);
        let reg_b = UI_REG_B.load(Ordering::SeqCst);
        let reg_c = UI_REG_C.load(Ordering::SeqCst);

        // Registers passed to ui_handler hold values 100, 200, 300
        assert_eq!(vm.registers[reg_a], 100, "Value in reg_a must be 100");
        assert_eq!(vm.registers[reg_b], 200, "Value in reg_b must be 200");
        assert_eq!(vm.registers[reg_c], 300, "Value in reg_c must be 300");
    }
}
