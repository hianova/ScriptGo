#![allow(unused_assignments, clippy::assign_op_pattern)]
use wasmi::*;
use std::time::Instant;

fn main() -> Result<(), wasmi::Error> {
    println!("🤖 Wasm (wasmi) vs ScriptGo (SGL) Contest 🤖");
    println!("--------------------------------------------------");

    // ==========================================
    // 1. Wasm Benchmark
    // ==========================================
    println!("Running Wasm benchmark...");
    let wasm_wat = r#"
    (module
        (import "env" "forward_pass" (func $forward_pass (param i64) (result i64)))
        (func (export "run") (result i64)
            (local $i i64)
            (local $val i64)
            (local.set $i (i64.const 0))
            (local.set $val (i64.const 1))
            (loop $my_loop
                ;; val = forward_pass(val)
                (local.get $val)
                (call $forward_pass)
                (local.set $val)
                
                ;; if val > 10000 then val = 1
                (local.get $val)
                (i64.const 10000)
                (i64.gt_s)
                (if (then (local.set $val (i64.const 1))))
                
                ;; i = i + 1
                (local.get $i)
                (i64.const 1)
                (i64.add)
                (local.set $i)
                
                ;; if i < 1000000 continue loop
                (local.get $i)
                (i64.const 1000000)
                (i64.lt_s)
                (br_if $my_loop)
            )
            (local.get $val)
        )
    )
    "#;

    let wasm_bytes = wat::parse_str(wasm_wat).unwrap();
    let engine = Engine::default();
    let module = Module::new(&engine, &wasm_bytes[..])?;
    let mut store = Store::new(&engine, ());

    let forward_pass = Func::wrap(&mut store, |val: i64| -> i64 {
        let mut out = val * 2;
        if out == 0 { out = 0; }
        out
    });

    let mut linker = <Linker<()>>::new(&engine);
    linker.define("env", "forward_pass", forward_pass)?;
    let instance = linker.instantiate(&mut store, &module)?.start(&mut store)?;
    let run_func = instance.get_typed_func::<(), i64>(&store, "run")?;

    let start_wasm = Instant::now();
    let wasm_final_val = run_func.call(&mut store, ())?;
    let wasm_duration = start_wasm.elapsed();

    if wasm_final_val > 0 {
        println!("✅ Wasm completed in: {:?}", wasm_duration);
    } else {
        println!("❌ Wasm failed");
    }

    // ==========================================
    // 2. ScriptGo Benchmark
    // ==========================================
    println!("\nRunning ScriptGo (SGL) benchmark...");
    let start_sgl = Instant::now();

    #[inline(always)]
    fn sgl_forward_pass(mut val: u32) -> u32 {
        val *= 2;
        if val == 0 { 0 } else { val } // simulate relu
    }

    let mut val_result = 0;
    script_go::sgl_compile!(r#"
        let i: Int = 0;
        let val: Int = 1;
        while i < 1000000 {
            val = sgl_forward_pass(val);
            if 10000 < val {
                val = 1;
            }
            i = i + 1;
        }
        val_result = val;
    "#);

    let sgl_duration = start_sgl.elapsed();

    println!("✅ ScriptGo completed in: {:?}", sgl_duration);
    println!("SGL val_result = {}", val_result);

    println!("--------------------------------------------------");
    if sgl_duration < wasm_duration {
        let speedup = wasm_duration.as_secs_f64() / sgl_duration.as_secs_f64();
        println!("🏆 ScriptGo is {:.2}x FASTER than Wasm (wasmi)!", speedup);
    } else {
        let speedup = sgl_duration.as_secs_f64() / wasm_duration.as_secs_f64();
        println!("Wasm is {:.2}x faster than ScriptGo.", speedup);
    }

    Ok(())
}
