#![allow(unused_assignments, clippy::assign_op_pattern)]
use std::process::Command;
use std::time::Instant;

fn main() {
    println!("🤖 Blender Physics: Python vs ScriptGo (SGL) Contest 🤖");
    println!("Simulating 100,000 objects over 600 frames...");
    println!("--------------------------------------------------");

    // ==========================================
    // 1. Python Benchmark
    // ==========================================
    println!("Running Python Blender Physics benchmark...");
    let start_py = Instant::now();
    let output_py = Command::new("python3")
        .arg("apps/ScriptGo/examples/benchmarks/blender_bench.py")
        .output()
        .expect("Failed to execute python3. Ensure it is installed.");
    let py_duration = start_py.elapsed();

    if output_py.status.success() {
        let result = String::from_utf8_lossy(&output_py.stdout);
        println!("✅ Python completed in: {:?}", py_duration);
        println!("Python Object 1 Final Height: {}", result.trim());
    } else {
        println!("❌ Python failed: {:?}", String::from_utf8_lossy(&output_py.stderr));
    }

    // ==========================================
    // 2. ScriptGo (SGL AOT) Benchmark
    // ==========================================
    println!("\nRunning ScriptGo (SGL AOT) Physics benchmark...");
    
    let num_objects = 100000;
    let steps = 600;
    
    let mut positions_y = vec![0.0f64; num_objects];
    for (i, pos_y) in positions_y.iter_mut().enumerate().take(num_objects) {
        *pos_y = (i % 100) as f64 + 10.0;
    }
    let mut velocities_y = vec![0.0f64; num_objects];
    
    let dt = 1.0f64 / 60.0f64;
    let gravity = 9.8f64;
    let bounce = -0.8f64;

    let start_sgl = Instant::now();
    
    for _ in 0..steps {
        script_go::sgl_compile!(r#"
            let i: usize = 0;
            while i < 100000 {
                let vy: Float = velocities_y[i];
                let py: Float = positions_y[i];
                
                vy = vy - (gravity * dt);
                py = py + (vy * dt);
                
                if py < 0.0 {
                    py = 0.0;
                    vy = vy * bounce;
                }
                
                velocities_y[i] = vy;
                positions_y[i] = py;
                i = i + 1;
            }
        "#);
    }
    
    let sgl_duration = start_sgl.elapsed();

    println!("✅ ScriptGo (AOT) completed in: {:?}", sgl_duration);
    println!("SGL Object 1 Final Height: {:.2}", positions_y[1]);

    println!("--------------------------------------------------");
    if sgl_duration < py_duration {
        let speedup = py_duration.as_secs_f64() / sgl_duration.as_secs_f64();
        println!("🏆 ScriptGo is {:.2}x FASTER than Python Physics Loop!", speedup);
    } else {
        let speedup = sgl_duration.as_secs_f64() / py_duration.as_secs_f64();
        println!("Python is {:.2}x faster than ScriptGo.", speedup);
    }
}
