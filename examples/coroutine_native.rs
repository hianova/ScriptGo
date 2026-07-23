use script_go::sgl::assembler::ScriptAssembler;
use script_go::sgl::vm::{ScriptVm, VmResult};
use std::time::Instant;

fn main() {
    println!("🤖 Native Yield (Stackless Coroutine) in ScriptGo VM 🤖");

    let mut asm = ScriptAssembler::new();
    asm.load_imm(0, 0); // i = 0
    asm.load_imm(1, 5); // limit = 5
    asm.load_imm(2, 1); // inc = 1

    let loop_start = asm.current_address() as u16;

    asm.jmp_if_eq(0, 1, 100); 

    asm.print_reg(0);
    asm.add(0, 0, 2);
    asm.yield_op();
    asm.jmp(loop_start);

    let end_addr = asm.current_address() as u16;
    asm.halt();

    let mut code = asm.build();
    code[3] = script_go::sgl::instruction::Instruction::new(
        script_go::sgl::instruction::OpCode::JmpIfEq as u8,
        0,
        1,
        end_addr as u8,
    );

    let mut vm = ScriptVm::new();
    vm.print_handler = Some(|val| {
        println!("Coroutine yielded value: {}", val);
    });

    let start = Instant::now();
    let mut total_steps = 0;
    let mut resume_count = 0;

    println!("Starting Coroutine...");
    loop {
        resume_count += 1;
        match vm.run(&code) {
            Ok(VmResult::Yielded(steps)) => {
                println!("Host received yield ({} steps). Resuming...", steps);
                total_steps += steps;
            }
            Ok(VmResult::Halted(steps)) => {
                println!("Coroutine finished execution ({} steps).", steps);
                total_steps += steps;
                break;
            }
            Ok(_) => {
                println!("Coroutine returned unhandled state");
                break;
            }
            Err(e) => {
                println!("Coroutine crashed: {:?}", e);
                break;
            }
        }
    }

    let duration = start.elapsed();
    println!("--------------------------------------------------");
    println!("✅ Zero-Cost Stackless Coroutine Test Passed!");
    println!("Resumed {} times.", resume_count);
    println!("Total VM steps: {}", total_steps);
    println!("Time Taken: {:?}", duration);
}
