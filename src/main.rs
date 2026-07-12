use script_go::assembler::parse_asm;
use script_go::vm::ScriptVm;
use std::time::Instant;

fn main() {
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

    let code = parse_asm(source_code);
    
    let mut vm = ScriptVm::new();
    let start = Instant::now();
    let steps = vm.run(&code);
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
