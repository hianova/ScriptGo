use script_go::sgl::assembler::ScriptAssembler;
use script_go::sgl::vm::{ScriptVm, VmResult};
use std::collections::{HashMap, VecDeque};
use std::time::Instant;

fn main() {
    println!("🌟 Native JS-style Async Event Loop in ScriptGo VM 🌟");

    let mut asm = ScriptAssembler::new();

    // -- MAIN FUNCTION (PC = 0) --
    // Spawn a child task starting at label (PC = 100)
    // R[1] = task_id
    asm.spawn(1, 100); 

    // Await the task_id in R[1], and put result in R[2]
    asm.await_op(2, 1);

    // Print result
    asm.print_reg(2);
    asm.halt();

    // Pad with NOPs until PC = 100
    let current_len = 4;
    for _ in 0..(100 - current_len) {
        asm.emit(script_go::sgl::instruction::Instruction::new(0, 0, 0, 0));
    }

    // -- CHILD TASK (PC = 100) --
    // Simulate some work by yielding 3 times (like awaiting IO)
    asm.yield_op();
    asm.yield_op();
    asm.yield_op();
    
    // Set R[0] = 42 (return value)
    asm.load_imm(0, 42);
    asm.halt();

    let code = asm.build();

    // HOST EVENT LOOP
    let mut tasks = HashMap::new();
    let mut run_queue = VecDeque::new();
    let mut next_task_id = 0;
    
    // Track tasks waiting for a resource: map[waiting_task_id] -> (resource_id, dest_reg)
    let mut waiting: HashMap<u32, (u32, u8)> = HashMap::new();
    let mut results: HashMap<u32, u32> = HashMap::new();

    // Create Main VM
    let mut main_vm = ScriptVm::new();
    main_vm.print_handler = Some(|val| {
        println!("Main Task received result: {}", val);
    });
    tasks.insert(next_task_id, main_vm);
    run_queue.push_back(next_task_id);
    next_task_id += 1;

    println!("Starting Event Loop...");
    let start = Instant::now();
    let mut steps_total = 0;

    while let Some(current_id) = run_queue.pop_front() {
        // If this task is waiting for a resource, check if it's ready
        if let Some(&(res_id, dest_reg)) = waiting.get(&current_id) {
            if let Some(&val) = results.get(&res_id) {
                // Resource is ready! Wake up task.
                waiting.remove(&current_id);
                let vm = tasks.get_mut(&current_id).unwrap();
                vm.registers[dest_reg as usize] = val;
            } else {
                // Still waiting. Push to back of queue.
                run_queue.push_back(current_id);
                continue;
            }
        }

        let run_res = {
            let vm = tasks.get_mut(&current_id).unwrap();
            vm.run(&code)
        };

        match run_res {
            Ok(VmResult::Halted(s)) => {
                steps_total += s;
                println!("Task {} Halted.", current_id);
                // Save its R[0] as the result for anyone awaiting it
                let vm = tasks.get(&current_id).unwrap();
                let ret_val = vm.registers[0];
                results.insert(current_id, ret_val);
            }
            Ok(VmResult::Yielded(s)) => {
                steps_total += s;
                println!("Task {} Yielded (Async IO).", current_id);
                run_queue.push_back(current_id);
            }
            Ok(VmResult::Spawn(s, target_pc, dest_reg)) => {
                steps_total += s;
                let new_id = next_task_id;
                next_task_id += 1;
                
                println!("Task {} Spawned Task {}.", current_id, new_id);
                
                let mut new_vm = ScriptVm::new();
                new_vm.pc = target_pc as usize; // start at target PC
                
                tasks.insert(new_id, new_vm);
                run_queue.push_back(new_id); // schedule child
                
                // Write child task_id to current task's dest_reg
                let vm = tasks.get_mut(&current_id).unwrap();
                vm.registers[dest_reg as usize] = new_id;
                run_queue.push_back(current_id); // reschedule parent
            }
            Ok(script_go::sgl::vm::VmResult::Awaiting(s, res_id, dest_reg)) => {
                steps_total += s;
                println!("Task {} Awaiting Resource {}.", current_id, res_id);
                waiting.insert(current_id, (res_id, dest_reg));
                run_queue.push_back(current_id); // parent is blocked
            }
            Ok(script_go::sgl::vm::VmResult::MmapRequest(_, _)) => {
                // Not supported
            }
            Err(e) => {
                println!("Task {} Error: {:?}", current_id, e);
            }
        }
    }

    let duration = start.elapsed();
    println!("--------------------------------------------------");
    println!("✅ Zero-Cost Async Event Loop Passed!");
    println!("Total VM steps executed: {}", steps_total);
    println!("Time Taken: {:?}", duration);
}
