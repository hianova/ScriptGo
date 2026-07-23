use script_go::sgl::assembler::ScriptAssembler;
use script_go::sgl::vm::{ScriptVm, VmResult};
use std::time::Instant;

fn main() {
    println!("🌟 Native 3D/DB Mmap Zero-Copy in ScriptGo VM 🌟");

    let num_floats = 100_000;
    let byte_size = num_floats * 4;
    
    // Simulate a massive 3D Geometry Buffer or Database Table mapped from OS
    // Total size: 300,000 floats (~1.2MB)
    let total_size = byte_size * 3;
    let mut mmap_buffer = vec![0u8; total_size];
    
    // Initialize src1 and src2 with some data
    unsafe {
        let ptr = mmap_buffer.as_mut_ptr() as *mut f32;
        for i in 0..num_floats {
            *ptr.add(i) = i as f32; // src1: 0, 1, 2...
            *ptr.add(i + num_floats) = (i * 2) as f32; // src2: 0, 2, 4...
            // dest will be zero
        }
    }

    let mut asm = ScriptAssembler::new();

    // R[1] = 1 (Resource ID for our geometry)
    asm.load_imm(1, 1);
    
    // Request Mmap
    asm.mmap_op(1);
    
    // R[0] = num_floats
    // num_floats is 100,000 (0x186A0)
    asm.load_imm(0, 0x01);
    asm.load_imm(10, 16);
    asm.emit(script_go::sgl::instruction::Instruction::new(15, 0, 0, 10)); // Shl R[0] = R[0] << 16
    asm.load_imm16(11, 0x86A0);
    asm.emit(script_go::sgl::instruction::Instruction::new(13, 0, 0, 11)); // Or R[0] = R[0] | R[11]
    
    // Set up addresses
    let _base_addr = 0x8000_0000u32;
    // R[2] = src1 addr
    asm.load_imm16(2, 0x8000);
    asm.emit(script_go::sgl::instruction::Instruction::new(15, 2, 2, 10)); // Shl R[2] = R[2] << 16
    // R[2] is now 0x8000_0000 (src1)
    
    // R[3] = src2 addr = R[2] + byte_size
    // Load byte_size into R[4]
    // byte_size is 400_000 = 0x61A80.
    asm.load_imm(4, 0x06);
    asm.emit(script_go::sgl::instruction::Instruction::new(15, 4, 4, 10)); // Shl R[4] = R[4] << 16
    asm.load_imm16(5, 0x1A80);
    asm.emit(script_go::sgl::instruction::Instruction::new(13, 4, 4, 5)); // Or R[4] = R[4] | R[5]
    // R[4] is now byte_size
    
    // R[3] = R[2] + R[4] (src2)
    asm.emit(script_go::sgl::instruction::Instruction::new(3, 3, 2, 4)); // Add R[3] = R[2] + R[4]
    
    // R[5] = R[3] + R[4] (dest)
    asm.emit(script_go::sgl::instruction::Instruction::new(3, 5, 3, 4)); // Add R[5] = R[3] + R[4]
    
    // Now VecAdd!
    // dest = R[5], src1 = R[2], src2 = R[3]
    asm.vec_add(5, 2, 3);
    
    asm.halt();
    
    let code = asm.build();
    let mut vm = ScriptVm::new();
    
    println!("VM Requesting execution...");
    
    let start = Instant::now();
    let mut run_result = vm.run(&code);
    
    if let Ok(VmResult::MmapRequest(_, res_id)) = run_result {
        println!("Host intercepting Mmap Request for Resource ID {}!", res_id);
        // Map the buffer!
        vm.mmap_ptr = mmap_buffer.as_ptr() as usize;
        vm.mmap_len = mmap_buffer.len();
        
        println!("Virtual Memory mapped! Resuming VM...");
        run_result = vm.run(&code); // Resume
    }
    
    let duration = start.elapsed();
    
    match run_result {
        Ok(VmResult::Halted(s)) => {
            println!("VM Halted successfully after {} steps.", s);
            
            // Verify results directly in the Host's mmap buffer
            let mut all_correct = true;
            unsafe {
                let dest_ptr = mmap_buffer.as_ptr().add(total_size - byte_size) as *const f32;
                for i in 0..10 {
                    // check first 10 elements
                    let expected = (i as f32) + ((i * 2) as f32);
                    let actual = *dest_ptr.add(i);
                    println!("Result[{}]: {} (Expected: {})", i, actual, expected);
                    if actual != expected {
                        all_correct = false;
                    }
                }
                
                // check last element
                let expected = ((num_floats - 1) as f32) + (((num_floats - 1) * 2) as f32);
                let actual = *dest_ptr.add(num_floats - 1);
                if actual != expected {
                    all_correct = false;
                }
            }
            
            if all_correct {
                println!("--------------------------------------------------");
                println!("✅ Zero-Cost Mmap SIMD Vectorization Passed!");
                println!("Processed {} vectors directly in Host memory without copying.", num_floats);
                println!("Time Taken: {:?}", duration);
            } else {
                println!("❌ Verification Failed!");
            }
        }
        e => println!("VM Error: {:?}", e),
    }
}
