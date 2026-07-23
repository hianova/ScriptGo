use script_go::sgl::vm::{ScriptVm, VmResult};
use script_go::compiler::codegen::CodeGen;
use script_go::compiler::lexer::Lexer;
use script_go::compiler::parser::Parser;
use std::time::Instant;

fn main() {
    println!("🌟 SGL Macro-ASM Expander (Zero-Cost DSL) 🌟");

    let script = r#"
        let port = 8080;
        let table = 1234;
        let condition = 18;
        
        // Extreme Encapsulation: Macro expansions!
        server.start!(port);
        db.filter!(table, condition);
    "#;

    println!("Parsing SGL Script:\n{}", script);

    let mut lexer = Lexer::new(script);
    let tokens = lexer.tokenize();
    println!("Tokens: {:?}", tokens);

    let mut parser = Parser::new(tokens);
    let ast = match parser.parse() {
        Ok(ast) => ast,
        Err(e) => {
            println!("Parser Error: {:?}", e);
            return;
        }
    };

    let mut codegen = CodeGen::new();
    let bytecode = match codegen.compile(&ast) {
        Ok(b) => b,
        Err(e) => {
            println!("CodeGen Error: {:?}", e);
            return;
        }
    };

    println!("✅ Compiled to {} OpCodes without abstract overhead!", bytecode.len());
    for (i, inst) in bytecode.iter().enumerate() {
        println!("{:04x}: {:?}", i, inst);
    }

    let mut vm = ScriptVm::new();
    
    // Set up Handlers to mock Host interception
    vm.syscall_handler = Some(|id, r1, _r2| {
        println!("VM intercepted SysCall {} with arg {}", id, r1);
        if id == 2 {
            println!("🚀 Host Action: Starting Web Server on port {}...", r1);
        }
    });
    
    vm.hardware_handler = Some(|vm_ref, id, r1, r2| {
        println!("VM intercepted HardwareCall {} with arg1 {}, arg2 {}", id, vm_ref.registers[r1], vm_ref.registers[r2]);
        if id == 3 {
            println!("🚀 Host Action: Zero-Copy DB Filter on table {}, condition {}", vm_ref.registers[r1], vm_ref.registers[r2]);
        }
    });

    println!("\nExecuting VM...");
    
    let start = Instant::now();
    let run_result = vm.run(&bytecode);
    
    match run_result {
        Ok(VmResult::Halted(s)) => {
            println!("VM Halted normally after {} steps.", s);
        }
        e => {
            println!("VM Error: {:?}", e);
        }
    }
    
    let duration = start.elapsed();
    println!("Time Taken: {:?}", duration);
    println!("--------------------------------------------------");
}
