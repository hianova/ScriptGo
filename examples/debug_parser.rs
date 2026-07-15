use script_go::compiler::lexer::Lexer;
use script_go::compiler::parser::Parser;

fn main() {
    let sgl_code = r#"
        let val: Int = 0;
        let i: Int = 0;
        while i < 1000000 {
            val = (500 * 1000) / 2;
            i = i + 1;
        }
    "#;

    let mut lexer = Lexer::new(sgl_code);
    let tokens = lexer.tokenize();
    println!("Tokens: {:?}", tokens);

    let mut parser = Parser::new(tokens);
    match parser.parse() {
        Ok(ast) => println!("AST: {:#?}", ast),
        Err(e) => println!("Parser error: {}", e),
    }
}
