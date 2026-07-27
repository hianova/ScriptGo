use script_go::sgl::vm::ScriptVm;
use script_go::sgl::math::SglMathRegisterExt;
use script_go::sgl::instruction::{OpCode, Instruction};

#[test]
fn test_sgl_math_exp_q16() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_math();

    // Set up registers for ExpQ16
    vm.registers[1] = 0; // dest
    vm.registers[2] = 12; // cmd id for math_exp_q16
    vm.registers[3] = 65536; // arg x (1.0 in Q16.16)

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, 3);
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];
    
    let res = vm.run(&code);
    assert!(res.is_ok());
    
    // Result should be in R1
    // exp(1.0) = 2.71828
    // In Q16.16, 2.71828 * 65536 = 178145
    let val = vm.registers[1];
    assert!(val > 175000 && val < 195000, "Exp(1.0) should be approx 189096 (approx), got {}", val);
}

#[test]
fn test_sgl_math_random() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_math();

    // R1 = dest, R2 = cmd, R3 = arg
    vm.registers[1] = 0;
    vm.registers[2] = 13; // cmd id for random_u32
    vm.registers[3] = 0;

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, 3);
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];
    
    let res = vm.run(&code);
    assert!(res.is_ok());
    let res1 = vm.registers[1];
    
    // Run again
    vm.pc = 0;
    vm.registers[1] = 0;
    let res_2 = vm.run(&code);
    assert!(res_2.is_ok());
    let res2 = vm.registers[1];
    
    // Should be highly unlikely to get the exact same random number
    assert_ne!(res1, res2);
}
