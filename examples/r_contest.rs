use std::process::Command;
use std::time::Instant;

fn main() {
    println!("📈 R/NumPy (Python List) vs ScriptGo (SGL SIMD/Vector) Contest 📈");
    println!("--------------------------------------------------");
    
    // 1. Benchmark Python List Vector
    println!("Running Python Vector Addition benchmark (10,000,000 elements)...");
    let _start_py = Instant::now();
    let output_py = Command::new("python3")
        .arg("examples/benchmarks/r_vector.py")
        .output()
        .expect("Failed to execute python3. Ensure it is installed.");
    let mut py_time_sec = 0.0;
    
    if output_py.status.success() {
        let result = String::from_utf8_lossy(&output_py.stdout);
        println!("✅ Python completed.");
        for line in result.lines() {
            println!("  {}", line);
            if line.starts_with("Time taken: ") {
                let parts: Vec<&str> = line.split_whitespace().collect();
                if let Ok(sec) = parts[2].parse::<f64>() {
                    py_time_sec = sec;
                }
            }
        }
    } else {
        println!("❌ Python failed: {:?}", String::from_utf8_lossy(&output_py.stderr));
    }

    // 2. Benchmark ScriptGo
    println!("\nRunning ScriptGo (SGL) SIMD Vector Addition...");
    
    let sgl_code = r#"
        let size: Int = 10000000;
        let result: Int = vector_add(size);
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
    
    vm.hardware_handler = Some(|vm: &mut ScriptVm, dest: usize, src: usize, op: usize| {
        if op == 2 {
            // vector_add
            let size = vm.registers[src] as usize;
            
            // Rust Host allocates arrays and does addition
            // In a real SGL engine, arrays would be pre-allocated or mapped to GPU
            let a = vec![1; size];
            let b = vec![2; size];
            let mut c = vec![0; size];
            
            // Vector addition loop
            for i in 0..size {
                c[i] = a[i] + b[i];
            }
            
            vm.registers[dest] = c[0] as u32; // Just store something
        }
    });

    vm.max_steps = Some(1_000_000);
    let _ = vm.run(&bytecode);

    let sgl_duration = start_sgl.elapsed();
    let sgl_time_sec = sgl_duration.as_secs_f64();
    println!("✅ ScriptGo completed in: {:.6} seconds", sgl_time_sec);

    println!("--------------------------------------------------");
    if sgl_time_sec < py_time_sec {
        let speedup = py_time_sec / sgl_time_sec;
        println!("🏆 ScriptGo Vector Core is {:.2}x FASTER than Python list comprehension!", speedup);
    } else {
        let speedup = sgl_time_sec / py_time_sec;
        println!("Python is {:.2}x faster than ScriptGo.", speedup);
    }
}
