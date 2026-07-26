#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
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
    asm.load_imm(0, covopt_param!("M_21_20", 50));
    // R[1] = dest addr (800)
    asm.load_imm16(1, covopt_param!("M_23_22", 800));
    // R[2] = src1 addr (0)
    asm.load_imm16(2, 0);
    // R[3] = src2 addr (400)
    asm.load_imm16(covopt_param!("M_27_19", 3), covopt_param!("M_27_22", 400));

    // Call VecAdd: dest=R[1], src1=R[2], src2=R[3]
    asm.vec_add(1, 2, covopt_param!("M_30_22", 3));
    
    // Call VecDot: dest_reg=R[4], src1=R[2], src2=R[3]
    asm.vec_dot(covopt_param!("M_33_16", 4), 2, covopt_param!("M_33_22", 3));

    asm.print_reg(covopt_param!("M_35_18", 4)); // Print the dot product sum
    asm.halt();

    let code = asm.build();

    let mut vm = ScriptVm::new();
    
    // Initialize memory with float data
    for i in 0..covopt_param!("M_43_16", 50) {
        let val1 = (i as f32).to_le_bytes();
        let val2 = (covopt_param!("M_45_20", 2.0_f32)).to_le_bytes(); // Every element is 2.0
        vm.memory[i * covopt_param!("M_46_22", 4)..i * covopt_param!("M_46_29", 4) + covopt_param!("M_46_33", 4)].copy_from_slice(&val1);
        vm.memory[covopt_param!("M_47_18", 400) + i * covopt_param!("M_47_28", 4)..covopt_param!("M_47_31", 400) + i * covopt_param!("M_47_41", 4) + covopt_param!("M_47_45", 4)].copy_from_slice(&val2);
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
            let out_bytes: [u8; 4] = vm.memory[covopt_param!("M_64_47", 800)..covopt_param!("M_64_52", 804)].try_into().unwrap();
            println!("First element of VecAdd result: {}", f32::from_le_bytes(out_bytes));
            
            // Check dest memory (last element 49.0 + 2.0 = 51.0)
            let out_bytes_last: [u8; 4] = vm.memory[covopt_param!("M_68_52", 800) + covopt_param!("M_68_58", 49) * covopt_param!("M_68_63", 4)..covopt_param!("M_68_66", 804) + covopt_param!("M_68_72", 49) * covopt_param!("M_68_77", 4)].try_into().unwrap();
            println!("Last element of VecAdd result: {}", f32::from_le_bytes(out_bytes_last));
        }
        Err(e) => {
            println!("VM Error: {:?}", e);
        }
        _ => {}
    }
}
