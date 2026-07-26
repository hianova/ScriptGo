#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::sgl::assembler::ScriptAssembler;
use script_go::sgl::vm::{ScriptVm, VmResult};
use std::time::Instant;

fn main() {
    println!("🌟 Native 3D/DB Mmap Zero-Copy in ScriptGo VM 🌟");

    let num_floats = covopt_param!("M_13_21", 100000);
    let byte_size = num_floats * covopt_param!("M_14_33", 4);
    
    // Simulate a massive 3D Geometry Buffer or Database Table mapped from OS
    // Total size: 300,000 floats (~1.2MB)
    let total_size = byte_size * covopt_param!("M_18_33", 3);
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
    asm.load_imm(covopt_param!("M_42_17", 10), covopt_param!("M_42_21", 16));
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_43_59", 15), 0, 0, covopt_param!("M_43_69", 10))); // Shl R[0] = R[0] << 16
    asm.load_imm16(covopt_param!("M_44_19", 11), covopt_param!("M_44_23", 34464));
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_45_59", 13), 0, 0, covopt_param!("M_45_69", 11))); // Or R[0] = R[0] | R[11]
    
    // Set up addresses
    let _base_addr = covopt_param!("M_48_21", 2147483648u32);
    // R[2] = src1 addr
    asm.load_imm16(2, covopt_param!("M_50_22", 32768));
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_51_59", 15), 2, 2, covopt_param!("M_51_69", 10))); // Shl R[2] = R[2] << 16
    // R[2] is now 0x8000_0000 (src1)
    
    // R[3] = src2 addr = R[2] + byte_size
    // Load byte_size into R[4]
    // byte_size is 400_000 = 0x61A80.
    asm.load_imm(covopt_param!("M_57_17", 4), covopt_param!("M_57_20", 6));
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_58_59", 15), covopt_param!("M_58_63", 4), covopt_param!("M_58_66", 4), covopt_param!("M_58_69", 10))); // Shl R[4] = R[4] << 16
    asm.load_imm16(covopt_param!("M_59_19", 5), covopt_param!("M_59_22", 6784));
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_60_59", 13), covopt_param!("M_60_63", 4), covopt_param!("M_60_66", 4), covopt_param!("M_60_69", 5))); // Or R[4] = R[4] | R[5]
    // R[4] is now byte_size
    
    // R[3] = R[2] + R[4] (src2)
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_64_59", 3), covopt_param!("M_64_62", 3), 2, covopt_param!("M_64_68", 4))); // Add R[3] = R[2] + R[4]
    
    // R[5] = R[3] + R[4] (dest)
    asm.emit(script_go::sgl::instruction::Instruction::new(covopt_param!("M_67_59", 3), covopt_param!("M_67_62", 5), covopt_param!("M_67_65", 3), covopt_param!("M_67_68", 4))); // Add R[5] = R[3] + R[4]
    
    // Now VecAdd!
    // dest = R[5], src1 = R[2], src2 = R[3]
    asm.vec_add(covopt_param!("M_71_16", 5), 2, covopt_param!("M_71_22", 3));
    
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
                for i in 0..covopt_param!("M_103_28", 10) {
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
