use script_go::assembler::parse_asm;
use script_go::vm::ScriptVm;
use std::time::Instant;

fn replay_trace(path: &str) -> Result<(), Box<dyn std::error::Error>> {
    let content = std::fs::read_to_string(path)?;
    let trace: Vec<script_go::vm::TraceStep> = serde_json::from_str(&content)?;

    println!("⏱️  Replaying trace from: {}", path);
    println!("--------------------------------------------------");
    for (i, step) in trace.iter().enumerate() {
        let mut change_str = String::from("No State Mutation");
        if let Some((reg, val)) = step.reg_change {
            change_str = format!("R[{}] -> {}", reg, val);
        } else if let Some((addr, val)) = step.mem_change {
            change_str = format!("RAM[{}] -> {}", addr, val);
        }

        println!(
            "[#{}] PC: {:03} | INST: 0x{:08X} | {}",
            i, step.pc, step.inst, change_str
        );
    }
    println!("--------------------------------------------------");
    println!("✅ Trace replay completed successfully!");
    Ok(())
}

fn main() {
    let args: Vec<String> = std::env::args().collect();
    if args.len() >= 3 && args[1] == "--replay" {
        if let Err(e) = replay_trace(&args[2]) {
            eprintln!("❌ Replay failed: {}", e);
            std::process::exit(1);
        }
        return;
    }
    if args.len() >= 4 && args[1] == "--build" {
        let input = &args[2];
        let output = &args[3];
        let content = std::fs::read_to_string(input).expect("Failed to read input file");
        let code = parse_asm(&content).expect("Failed to assemble");
        let mut bytes = Vec::new();
        for inst in code {
            bytes.extend_from_slice(&inst.0.to_le_bytes());
        }
        std::fs::write(output, &bytes).expect("Failed to write output file");
        println!("Successfully built {} -> {}", input, output);
        return;
    }

    println!("🚀 Starting ScriptGo VM with Assembler...");

    let source_code = r#"
        # Calculate Fibonacci sequence
        # R[1] = a (starts at 0)
        # R[2] = b (starts at 1)
        # R[3] = temp
        # R[4] = counter (loop 10 times)
        # R[5] = 1 (constant for decrement)
        
        LOADIMM 1 0
        LOADIMM 2 1
        LOADIMM 4 10
        LOADIMM 5 1
        
        # LOOP START (PC=4)
        # temp = a + b
        ADD 3 1 2
        # a = b
        ADD 1 2 0
        # b = temp
        ADD 2 3 0
        
        # counter = counter - 1
        SUB 4 4 5
        
        # If counter == 0, jump to HALT (PC=11)
        JMPIFZERO 4 11
        
        # Jump back to LOOP START (PC=4)
        JMP 4
        
        # HALT (PC=11)
        HALT
    "#;

    let code = parse_asm(source_code).unwrap();

    let mut vm = ScriptVm::new();
    vm.print_handler = Some(|val| println!("🖨️ [PRINTREG]: {}", val));
    let start = Instant::now();
    let steps = vm.run(&code).unwrap();
    let duration = start.elapsed();

    println!("✅ Execution Finished!");
    println!("⏱️  Steps Executed: {}", steps);
    println!("⏱️  Time Taken: {:?}", duration);
    println!("--------------------------------------------------");
    println!("📊 Final Register States:");
    println!("R[1] (Fib 10) = {}", vm.registers[1]);
    println!("R[2] (Fib 11) = {}", vm.registers[2]);

    // The 10th fibonacci number starting with 0, 1 is 55.
    // Fib 0=0, 1=1, 2=1, 3=2, 4=3, 5=5, 6=8, 7=13, 8=21, 9=34, 10=55
    if vm.registers[1] == 55 {
        println!("✨ Fibonacci Verification Passed!");
    } else {
        println!("❌ Verification Failed! Got: {}", vm.registers[1]);
    }
}
