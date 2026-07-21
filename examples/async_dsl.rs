use script_go::compiler::codegen::CodeGen;
use script_go::compiler::lexer::Lexer;
use script_go::compiler::parser::Parser;

fn main() {
    println!("🌟 SGL Async Event Loop Macro Test 🌟");

    let script = r#"
        let task_id = spawn!(100);
        yield!();
        let result = await!(task_id);
    "#;

    println!("Parsing SGL Script:\n{}", script);

    let mut lexer = Lexer::new(script);
    let tokens = lexer.tokenize();

    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Parser Error: {:?}", e);
            return;
        }
    };

    let mut codegen = CodeGen::new();
    match codegen.compile(&ast) {
        Ok(bytecode) => {
            println!("✅ Compiled to {} OpCodes!", bytecode.len());
            for (i, inst) in bytecode.iter().enumerate() {
                println!("{:04x}: {:?}", i, inst);
            }
        }
        Err(e) => {
            println!("CodeGen Error: {:?}", e);
        }
    }
}
