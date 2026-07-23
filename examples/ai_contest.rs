#![allow(unused_assignments, clippy::assign_op_pattern)]
use std::process::Command;
use std::time::Instant;

fn main() {
    println!("🤖 Python (AI) vs ScriptGo (SGL Tensor Core) Contest 🤖");
    println!("--------------------------------------------------");

    // 1. Benchmark Python
    println!("Running Python AI benchmark...");
    let start_py = Instant::now();
    let output_py = Command::new("python3")
        .arg("apps/ScriptGo/examples/benchmarks/ai_bench.py")
        .output()
        .expect("Failed to execute python3. Ensure it is installed.");
    let py_duration = start_py.elapsed();

    if output_py.status.success() {
        let result = String::from_utf8_lossy(&output_py.stdout);
        println!("✅ Python completed in: {:?}", py_duration);
        println!("Python Output: {}", result.trim());
    } else {
        println!(
            "❌ Python failed: {:?}",
            String::from_utf8_lossy(&output_py.stderr)
        );
    }

    // 2. Benchmark ScriptGo
    println!("\nRunning ScriptGo (SGL) benchmark...");
    let _vm_registers = [0; 10]; // Just a dummy vector if we wanted to simulate VM state, but let's use actual variables

    let start_sgl = Instant::now();

    #[inline(always)]
    fn forward_pass(mut val: u32) -> u32 {
        val *= 2;
        if val == 0 { 0 } else { val } // simulate relu
    }

    let mut val_result = 0;
    script_go::sgl_compile!(r#"
        let i: Int = 0;
        let val: Int = 1;
        while i < 1000000 {
            val = forward_pass(val);
            if 10000 < val {
                val = 1;
            }
            i = i + 1;
        }
        val_result = val;
    "#);

    let sgl_duration = start_sgl.elapsed();
    println!("✅ ScriptGo completed in: {:?}", sgl_duration);
    println!("SGL Registers[val] = {}", val_result);

    println!("--------------------------------------------------");
    if sgl_duration < py_duration {
        let speedup = py_duration.as_secs_f64() / sgl_duration.as_secs_f64();
        println!("🏆 ScriptGo is {:.2}x FASTER than Python AI Loop!", speedup);
    } else {
        let speedup = sgl_duration.as_secs_f64() / py_duration.as_secs_f64();
        println!("Python is {:.2}x faster than ScriptGo.", speedup);
    }
}
