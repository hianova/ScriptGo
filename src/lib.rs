#![no_std]

extern crate alloc;

pub mod instruction;
pub mod vm;
pub mod assembler;
pub mod sync;
pub mod binary;

#[cfg(test)]
extern crate std;

#[cfg(test)]
mod covopt_tests {
    use std::env;
    use super::*;

    #[test]
    fn covopt_benchmark_test() {
        let n_str = env::var("COVOPT_N").unwrap_or_else(|_| alloc::string::String::from("100"));
        let n: u32 = n_str.parse().unwrap();
        
        let mut vm = vm::ScriptVm::new();
        vm.registers[1] = n; // Loop counter
        vm.registers[2] = 1; // Constant 1
        
        // Instructions:
        // 0: JmpIfZero 1 3 0  (If R[1] == 0, jump to Halt at 3)
        // 1: Sub 1 1 2        (R[1] = R[1] - R[2])
        // 2: Jmp 0 0 0        (Jump back to 0)
        // 3: Halt 0 0 0
        let code = [
            instruction::Instruction::new(instruction::OpCode::JmpIfZero as u8, 1, 3, 0),
            instruction::Instruction::new(instruction::OpCode::Sub as u8, 1, 1, 2),
            instruction::Instruction::new(instruction::OpCode::Jmp as u8, 0, 0, 0),
            instruction::Instruction::new(instruction::OpCode::Halt as u8, 0, 0, 0),
        ];
        
        let steps = vm.run(std::hint::black_box(&code)).unwrap();
        std::hint::black_box(steps);
    }
}
