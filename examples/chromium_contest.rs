use std::process::Command;
use std::time::Instant;

fn main() {
    println!("🏎️  Chromium (V8) vs ScriptGo (SGL) Contest 🏎️");
    println!("--------------------------------------------------");
    
    // 1. Benchmark Node.js (V8)
    println!("Running V8 (Node.js) benchmark...");
    let start_node = Instant::now();
    let output_node = Command::new("node")
        .arg("examples/benchmarks/bench.js")
        .output()
        .expect("Failed to execute node. Ensure Node.js is installed.");
    let node_duration = start_node.elapsed();
    
    if output_node.status.success() {
        println!("✅ Node.js (V8) completed in: {:?}", node_duration);
    } else {
        println!("❌ Node.js (V8) failed: {:?}", String::from_utf8_lossy(&output_node.stderr));
    }

    // 2. Benchmark ScriptGo
    println!("\nRunning ScriptGo (SGL) benchmark...");
    let sgl_code = r#"
        let i: Int = 0;
        let val: Int = 0;
        while i < 1000000 {
            val = (500 * 1000) / 2;
            i = i + 1;
        }
    "#;

    let start_sgl = Instant::now();
    
    use script_go::compiler::lexer::Lexer;
    use script_go::compiler::parser::Parser;
    use script_go::compiler::codegen::CodeGen;
    use script_go::vm::ScriptVm;
    
    let mut lexer = Lexer::new(sgl_code);
    let tokens = lexer.tokenize();
    let mut parser = Parser::new(tokens);
    let ast = parser.parse().unwrap();
    let mut codegen = CodeGen::new();
    let bytecode = codegen.compile(&ast).unwrap();
    
    let mut vm = ScriptVm::new();
    vm.max_steps = Some(10_000_000); // Allow enough steps
    let _ = vm.run(&bytecode);

    let sgl_duration = start_sgl.elapsed();
    println!("✅ ScriptGo completed in: {:?}", sgl_duration);

    println!("--------------------------------------------------");
    if sgl_duration < node_duration {
        let speedup = node_duration.as_secs_f64() / sgl_duration.as_secs_f64();
        println!("🏆 ScriptGo is {:.2}x FASTER than Chrome V8!", speedup);
    } else {
        let speedup = sgl_duration.as_secs_f64() / node_duration.as_secs_f64();
        println!("Chrome V8 is {:.2}x faster than ScriptGo.", speedup);
    }
}
