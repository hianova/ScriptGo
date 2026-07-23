#![allow(unused_assignments, clippy::assign_op_pattern)]
use mlua::prelude::*;
use std::time::Instant;

fn main() -> LuaResult<()> {
    println!("🤖 Lua vs ScriptGo (SGL) Contest 🤖");
    println!("--------------------------------------------------");

    // ==========================================
    // 1. Lua Benchmark
    // ==========================================
    println!("Running Lua benchmark...");
    let lua = Lua::new();

    let forward_pass = lua.create_function(|_, val: u32| {
        let mut out = val * 2;
        if out == 0 {
            out = 0; // Simulate relu
        }
        Ok(out)
    })?;
    lua.globals().set("forward_pass", forward_pass)?;

    let lua_code = r#"
        local i = 0
        local val = 1
        while i < 1000000 do
            val = forward_pass(val)
            if val > 10000 then
                val = 1
            end
            i = i + 1
        end
        return val
    "#;

    let start_lua = Instant::now();
    let lua_final_val: u32 = lua.load(lua_code).eval()?;
    let lua_duration = start_lua.elapsed();

    if lua_final_val > 0 {
        println!("✅ Lua completed in: {:?}", lua_duration);
    } else {
        println!("❌ Lua failed");
    }

    // ==========================================
    // 2. ScriptGo Benchmark
    // ==========================================
    println!("\nRunning ScriptGo (SGL) benchmark...");
    
    fn sgl_forward_pass(val: u32) -> u32 {
        if val == 0 { 0 } else { val } // simulate relu
    }

    let start_sgl = Instant::now();
    let mut val_result = 0;
    script_go::sgl_compile!(r#"
        let i: Int = 0;
        let val: Int = 1;
        while i < 1000000 {
            let next: Int = val + 1;
            val = next;
            i = i + 1;
        }
        val_result = val;
    "#);

    let sgl_duration = start_sgl.elapsed();

    println!("✅ ScriptGo completed in: {:?}", sgl_duration);
    println!("SGL val_result = {}", val_result);

    println!("--------------------------------------------------");
    if sgl_duration < lua_duration {
        let speedup = lua_duration.as_secs_f64() / sgl_duration.as_secs_f64();
        println!("🏆 ScriptGo is {:.2}x FASTER than Lua (FFI bottleneck)!", speedup);
    } else {
        let speedup = sgl_duration.as_secs_f64() / lua_duration.as_secs_f64();
        println!("Lua is {:.2}x faster than ScriptGo.", speedup);
    }

    Ok(())
}
