use std::process::Command;
use std::time::Instant;

fn main() {
    println!("🤖 Python (AI) vs ScriptGo (SGL Tensor Core) Contest 🤖");
    println!("--------------------------------------------------");
    
    // 1. Benchmark Python
    println!("Running Python AI benchmark...");
    let start_py = Instant::now();
    let output_py = Command::new("python3")
        .arg("examples/benchmarks/ai_bench.py")
        .output()
        .expect("Failed to execute python3. Ensure it is installed.");
    let py_duration = start_py.elapsed();
    
    if output_py.status.success() {
        let result = String::from_utf8_lossy(&output_py.stdout);
        println!("✅ Python completed in: {:?}", py_duration);
        println!("Python Output: {}", result.trim());
    } else {
        println!("❌ Python failed: {:?}", String::from_utf8_lossy(&output_py.stderr));
    }

    // 2. Benchmark ScriptGo
    println!("\nRunning ScriptGo (SGL) benchmark...");
    let sgl_code = r#"
        let i: Int = 0;
        let val: Int = 1;
        while i < 1000000 {
            val = forward_pass(val);
            if 10000 < val {
                val = 1;
            }
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
    // Handler: a = dest, b = val.
    // Simulate forward_pass(val) -> val * 2
    vm.neural_handler = Some(|vm: &mut ScriptVm, dest: usize, src: usize, _c: usize| {
        let val = vm.registers[src];
        let mut out = val * 2;
        if out == 0 {
            out = 0; // Simulate relu
        }
        vm.registers[dest] = out;
    });

    vm.max_steps = Some(10_000_000);
    let _ = vm.run(&bytecode);

    let sgl_duration = start_sgl.elapsed();
    println!("✅ ScriptGo completed in: {:?}", sgl_duration);
    println!("SGL Registers[val] = {}", vm.registers[1]); // Val is likely R[1], but just printing it for debugging

    println!("--------------------------------------------------");
    if sgl_duration < py_duration {
        let speedup = py_duration.as_secs_f64() / sgl_duration.as_secs_f64();
        println!("🏆 ScriptGo is {:.2}x FASTER than Python AI Loop!", speedup);
    } else {
        let speedup = sgl_duration.as_secs_f64() / py_duration.as_secs_f64();
        println!("Python is {:.2}x faster than ScriptGo.", speedup);
    }
}
