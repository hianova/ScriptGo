use script_go::sgl::assembler::ScriptAssembler;
use script_go::sgl::vm::{ScriptVm, VmResult};
use std::time::Instant;

fn main() {
    println!("📈 Native SIMD (Tensor Ops) in ScriptGo VM 📈");

    // We will allocate memory for two input vectors and one output vector
    // Vector size = 50
    // src1 starts at memory index 0
    // src2 starts at memory index 400
    // dest starts at memory index 800

    let mut asm = ScriptAssembler::new();
    // R[0] is the length (VLEN). We set it to 50
    asm.load_imm(0, 50);
    // R[1] = dest addr (800)
    asm.load_imm16(1, 800);
    // R[2] = src1 addr (0)
    asm.load_imm16(2, 0);
    // R[3] = src2 addr (400)
    asm.load_imm16(3, 400);

    // Call VecAdd: dest=R[1], src1=R[2], src2=R[3]
    asm.vec_add(1, 2, 3);
    
    // Call VecDot: dest_reg=R[4], src1=R[2], src2=R[3]
    asm.vec_dot(4, 2, 3);

    asm.print_reg(4); // Print the dot product sum
    asm.halt();

    let code = asm.build();

    let mut vm = ScriptVm::new();
    
    // Initialize memory with float data
    for i in 0..50 {
        let val1 = (i as f32).to_le_bytes();
        let val2 = (2.0f32).to_le_bytes(); // Every element is 2.0
        vm.memory[i * 4..i * 4 + 4].copy_from_slice(&val1);
        vm.memory[400 + i * 4..400 + i * 4 + 4].copy_from_slice(&val2);
    }

    vm.print_handler = Some(|val| {
        println!("Dot Product Result: {}", f32::from_bits(val));
    });

    let start = Instant::now();
    match vm.run(&code) {
        Ok(VmResult::Halted(steps)) => {
            let duration = start.elapsed();
            println!("--------------------------------------------------");
            println!("✅ Zero-Cost Native SIMD Test Passed!");
            println!("Total VM steps: {}", steps); // Should be very few steps!
            println!("Time Taken: {:?}", duration);
            
            // Check dest memory (first element 0.0 + 2.0 = 2.0)
            let out_bytes: [u8; 4] = vm.memory[800..804].try_into().unwrap();
            println!("First element of VecAdd result: {}", f32::from_le_bytes(out_bytes));
            
            // Check dest memory (last element 49.0 + 2.0 = 51.0)
            let out_bytes_last: [u8; 4] = vm.memory[800 + 49 * 4..804 + 49 * 4].try_into().unwrap();
            println!("Last element of VecAdd result: {}", f32::from_le_bytes(out_bytes_last));
        }
        Err(e) => {
            println!("VM Error: {:?}", e);
        }
        _ => {}
    }
}
