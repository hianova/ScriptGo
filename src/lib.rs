#![allow(unused_imports)]
#![no_std]
#[macro_use] extern crate covopt_macro;
use covopt_macro::covopt_param;
use std::io::Write;
extern crate alloc;
extern crate self as script_go;
pub mod sgl;
pub use sgl::assembler::ScriptAssembler;
pub use sgl::host_handlers;
pub use sgl::instruction;
pub use sgl::ui_engine;
pub use sgl::vm;
pub use sgl::io::{self, SglIoRegisterExt};
pub use sgl::net::{self, SglNetRegisterExt};
pub use sgl_macros::{
    sgl_cmd, sgl_combine_handlers, sgl_compile, sgl_hardware_call, sgl_package, sgl_syscall,
};
pub mod assembler;
pub mod binary;
pub mod compiler;
pub mod sync;
#[cfg(feature = "std")]
extern crate std;

#[cfg(test)]
mod covopt_tests {
    use super::*;
    use std::env;
    #[test]
    fn covopt_benchmark_test() {
        let n_str = env::var("COVOPT_N").unwrap_or_else(|_| alloc::string::String::from("100"));
        let n: u32 = n_str.parse().unwrap();
        let mut vm = vm::ScriptVm::new();
        vm.tracing_enabled = true;
        vm.registers[1] = n; // Loop counter
        vm.registers[2] = 1; // Constant 1
                             // Instructions:
                             // 0: JmpIfZero 1 3 0  (If R[1] == 0, jump to Halt at 3)
                             // 1: Sub 1 1 2        (R[1] = R[1] - R[2])
                             // 2: Jmp 0 0 0        (Jump back to 0)
                             // 3: Halt 0 0 0
        let code = [
            instruction::Instruction::new(instruction::OpCode::JmpIfZero as u8, 1, covopt_param!("M_44_83", 3), 0),
            instruction::Instruction::new(instruction::OpCode::Sub as u8, 1, 1, 2),
            instruction::Instruction::new(instruction::OpCode::Jmp as u8, 0, 0, 0),
            instruction::Instruction::new(instruction::OpCode::Halt as u8, 0, 0, 0),
        ];
        let steps = vm.run(std::hint::black_box(&code)).unwrap();
        std::hint::black_box(steps);
    }
}
pub mod cli;
