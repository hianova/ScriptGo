#![allow(unused_imports)]
use covopt_macro::covopt_param;
use std::io::Write;
#[cfg(test)]
mod tests {
    use crate::sgl::instruction::{Instruction, OpCode};
    use crate::sgl::vm::{ScriptVm, VmError};
    use alloc::string::String;

    // Define a test macro package using #[sgl_package]
    #[sgl_macros::sgl_package(name = "macro_stress")]
    pub mod macro_stress_pkg {
        use crate::sgl::vm::ScriptVm;
        use alloc::format;
        use alloc::string::String;

        #[sgl_cmd(1)]
        pub fn echo_string(_vm: &mut ScriptVm, text: String) -> String {
            format!("ECHO: {}", text)
        }

        #[sgl_cmd(2)]
        pub fn process_bytes(_vm: &mut ScriptVm, data: &[u8]) -> usize {
            data.len()
        }

        #[sgl_cmd(3)]
        pub fn host_panic_function(_vm: &mut ScriptVm) {
            panic!("Host panic inside sgl-macro handler!");
        }

        #[sgl_cmd(4)]
        pub fn multi_args(_vm: &mut ScriptVm, first: String, val: u32, second: String) -> String {
            format!("{}-{}-{}", first, val, second)
        }
    }

    // Define standalone #[sgl_syscall] function
    #[sgl_macros::sgl_syscall(10)]
    pub fn standalone_syscall_panic(_vm: &mut ScriptVm, _input: String) {
        panic!("Panic in standalone syscall!");
    }

    // =========================================================================
    // 1. sgl-macro Stress Tests: Invalid Addresses, Null Pointers, Invalid UTF-8, Host Panics
    // =========================================================================

    #[test]
    fn test_sgl_macro_invalid_address_0xdeadbeef() {
        let mut vm = ScriptVm::new();

        // Pass 0xDEADBEEF as the string memory pointer in R[1]
        let dest_reg = 0;
        let cmd_reg = 1;
        let arg_reg = 2;

        vm.registers[cmd_reg] = 1; // cmd 1: echo_string
        vm.registers[arg_reg] = covopt_param!("M_58_32", 3735928559);

        // Dispatch macro-generated package handler
        macro_stress_pkg::dispatch(&mut vm, dest_reg, cmd_reg, arg_reg);

        // Verification: Out-of-bounds pointer causes extractor to fail safely, setting dest_reg to 0.
        assert_eq!(
            vm.registers[dest_reg], 0,
            "Destination register must be set to 0 when passing invalid address 0xDEADBEEF"
        );
    }

    #[test]
    fn test_sgl_macro_null_pointer() {
        let mut vm = ScriptVm::new();

        // Null pointer address (0) in R[2] for byte slice reading with huge length in R[3]
        let dest_reg = 0;
        let cmd_reg = 1;
        let arg_reg = 2;

        vm.registers[cmd_reg] = 2; // cmd 2: process_bytes
        vm.registers[arg_reg] = 0; // data pointer = 0
        vm.registers[arg_reg + 1] = covopt_param!("M_81_36", 2147483647); // huge data length = 2147483647

        macro_stress_pkg::dispatch(&mut vm, dest_reg, cmd_reg, arg_reg);

        // Verification: Bounds check fails safely, dest_reg set to 0, no host panic.
        assert_eq!(
            vm.registers[dest_reg], 0,
            "Destination register must be set to 0 when passing null pointer with huge length"
        );
    }

    #[test]
    fn test_sgl_macro_invalid_utf8_string() {
        let mut vm = ScriptVm::new();

        // Write non-UTF-8 bytes at VM memory address 100
        let invalid_utf8_bytes: [u8; 5] = [covopt_param!("M_97_43", 255), covopt_param!("M_97_48", 254), covopt_param!("M_97_53", 253), covopt_param!("M_97_58", 128), 0x00];
        vm.write_bytes(covopt_param!("M_98_23", 100), &invalid_utf8_bytes)
            .expect("Failed to write invalid UTF-8 bytes to VM memory");

        let dest_reg = 0;
        let cmd_reg = 1;
        let arg_reg = 2;

        vm.registers[cmd_reg] = 1; // cmd 1: echo_string
        vm.registers[arg_reg] = covopt_param!("M_106_32", 100); // Pointer to invalid UTF-8 string

        macro_stress_pkg::dispatch(&mut vm, dest_reg, cmd_reg, arg_reg);

        // Verification: read_string fails UTF-8 decoding, dest_reg set to 0, zero host panic.
        assert_eq!(
            vm.registers[dest_reg], 0,
            "Destination register must be set to 0 when reading invalid UTF-8 string"
        );
    }

    #[test]
    fn test_sgl_macro_host_panicking_function() {
        let mut vm = ScriptVm::new();

        let dest_reg = 0;
        let cmd_reg = 1;
        let arg_reg = 2;

        vm.registers[cmd_reg] = covopt_param!("M_125_32", 3); // cmd 3: host_panic_function

        // Empirical check: Verify that calling panicking macro handler catches panic via catch_unwind
        macro_stress_pkg::dispatch(&mut vm, dest_reg, cmd_reg, arg_reg);

        // Verification: Dest reg set to 0, host process survived intact!
        assert_eq!(
            vm.registers[dest_reg], 0,
            "Destination register must be 0 after catching host function panic"
        );
    }

    #[test]
    fn test_sgl_syscall_panicking_function() {
        let mut vm = ScriptVm::new();

        let dest_reg = 0;
        let cmd_reg = 1;
        let arg_reg = 2;

        vm.registers[arg_reg] = covopt_param!("M_145_32", 3735928559);

        // Call generated standalone syscall handler directly
        standalone_syscall_panic_handler(&mut vm, dest_reg, cmd_reg, arg_reg);

        assert_eq!(
            vm.registers[dest_reg], 0,
            "Destination register must be 0 after catching standalone syscall panic"
        );
    }

    #[test]
    fn test_sgl_macro_multi_args_invalid_inputs() {
        let mut vm = ScriptVm::new();

        // Setup R[2] = 100 (valid string "hello"), R[3] = 42, R[4] = 0xDEADBEEF (invalid string)
        let valid_str = "hello\0";
        vm.write_bytes(covopt_param!("M_162_23", 100), valid_str.as_bytes()).unwrap();

        vm.registers[1] = covopt_param!("M_164_26", 4); // cmd 4: multi_args
        vm.registers[2] = covopt_param!("M_165_26", 100);
        vm.registers[covopt_param!("M_166_21", 3)] = covopt_param!("M_166_26", 42);
        vm.registers[covopt_param!("M_167_21", 4)] = covopt_param!("M_167_26", 3735928559);

        macro_stress_pkg::dispatch(&mut vm, 0, 1, 2);

        assert_eq!(
            vm.registers[0], 0,
            "Multi-arg call with any invalid parameter must return 0"
        );
    }

    // =========================================================================
    // 2. SIMD Overflow Fix Stress Tests (OpCodes 39, 40, 41)
    // =========================================================================

    #[test]
    fn test_simd_integer_overflow_length_usize_max() {
        let mut vm = ScriptVm::new();

        // R[0] holds SIMD vector length
        vm.registers[0] = u32::MAX; // 0xFFFFFFFF, multiplication by 4 overflows usize/u32!
        vm.registers[1] = covopt_param!("M_187_26", 100); // dest
        vm.registers[2] = covopt_param!("M_188_26", 200); // src1
        vm.registers[covopt_param!("M_189_21", 3)] = covopt_param!("M_189_26", 300); // src2

        // OpCode 39 (VecAdd)
        let code_add = [Instruction::new(OpCode::VecAdd as u8, 1, 2, covopt_param!("M_192_69", 3))];
        let res_add = vm.run(&code_add);

        assert!(
            matches!(res_add, Err(VmError::MemoryAccessOutOfBounds { .. })),
            "VecAdd with overflow length (u32::MAX) must return MemoryAccessOutOfBounds, got: {:?}",
            res_add
        );

        // OpCode 40 (VecMul)
        vm.pc = 0;
        let code_mul = [Instruction::new(OpCode::VecMul as u8, 1, 2, covopt_param!("M_203_69", 3))];
        let res_mul = vm.run(&code_mul);

        assert!(
            matches!(res_mul, Err(VmError::MemoryAccessOutOfBounds { .. })),
            "VecMul with overflow length (u32::MAX) must return MemoryAccessOutOfBounds, got: {:?}",
            res_mul
        );

        // OpCode 41 (VecDot)
        vm.pc = 0;
        let code_dot = [Instruction::new(OpCode::VecDot as u8, 1, 2, covopt_param!("M_214_69", 3))];
        let res_dot = vm.run(&code_dot);

        assert!(
            matches!(res_dot, Err(VmError::MemoryAccessOutOfBounds { .. })),
            "VecDot with overflow length (u32::MAX) must return MemoryAccessOutOfBounds, got: {:?}",
            res_dot
        );
    }

    #[test]
    fn test_simd_integer_overflow_length_boundary_values() {
        let mut vm = ScriptVm::new();

        let overflow_lengths = [
            covopt_param!("M_229_12", 1073741824), // 1073741824 * 4 = 4294967296 (32-bit overflow)
            covopt_param!("M_230_12", 2147483648), // 2147483648 * 4 = 8589934592
            covopt_param!("M_231_12", 1073741823), // 1073741823 * 4 = 4294967292 (larger than VM memory 1024)
            u32::MAX / 2,
        ];

        for &len in &overflow_lengths {
            vm.pc = 0;
            vm.registers[0] = len;
            vm.registers[1] = 0;
            vm.registers[2] = 0;
            vm.registers[covopt_param!("M_240_25", 3)] = 0;

            let code_add = [Instruction::new(OpCode::VecAdd as u8, 1, 2, covopt_param!("M_242_73", 3))];
            let res = vm.run(&code_add);

            assert!(
                matches!(res, Err(VmError::MemoryAccessOutOfBounds { .. })),
                "VecAdd with boundary length {} must return MemoryAccessOutOfBounds, got: {:?}",
                len,
                res
            );
        }
    }

    #[test]
    fn test_simd_invalid_vm_addresses_0xdeadbeef() {
        let mut vm = ScriptVm::new();

        vm.registers[0] = covopt_param!("M_258_26", 4); // Length 4 floats (16 bytes)

        // Case 1: Dest address is 0xDEADBEEF
        vm.registers[1] = covopt_param!("M_261_26", 3735928559);
        vm.registers[2] = 0;
        vm.registers[covopt_param!("M_263_21", 3)] = covopt_param!("M_263_26", 16);
        let code1 = [Instruction::new(OpCode::VecAdd as u8, 1, 2, covopt_param!("M_264_66", 3))];
        let res1 = vm.run(&code1);
        assert!(
            matches!(res1, Err(VmError::MemoryAccessOutOfBounds { addr: 0xDEADBEEF, .. })),
            "VecAdd with dest=0xDEADBEEF must return MemoryAccessOutOfBounds"
        );

        // Case 2: Src1 address is 0xDEADBEEF
        vm.pc = 0;
        vm.registers[1] = 0;
        vm.registers[2] = covopt_param!("M_274_26", 3735928559);
        vm.registers[covopt_param!("M_275_21", 3)] = covopt_param!("M_275_26", 16);
        let code2 = [Instruction::new(OpCode::VecMul as u8, 1, 2, covopt_param!("M_276_66", 3))];
        let res2 = vm.run(&code2);
        assert!(
            matches!(res2, Err(VmError::MemoryAccessOutOfBounds { addr: 0xDEADBEEF, .. })),
            "VecMul with src1=0xDEADBEEF must return MemoryAccessOutOfBounds"
        );

        // Case 3: Src2 address is 0xDEADBEEF
        vm.pc = 0;
        vm.registers[1] = 0;
        vm.registers[2] = covopt_param!("M_286_26", 16);
        vm.registers[covopt_param!("M_287_21", 3)] = covopt_param!("M_287_26", 3735928559);
        let code3 = [Instruction::new(OpCode::VecDot as u8, 1, 2, covopt_param!("M_288_66", 3))];
        let res3 = vm.run(&code3);
        assert!(
            matches!(res3, Err(VmError::MemoryAccessOutOfBounds { addr: 0xDEADBEEF, .. })),
            "VecDot with src2=0xDEADBEEF must return MemoryAccessOutOfBounds"
        );
    }

    #[test]
    fn test_simd_out_of_bounds_ram_address() {
        let mut vm = ScriptVm::new();

        vm.registers[0] = covopt_param!("M_300_26", 10); // 40 bytes
        vm.registers[1] = covopt_param!("M_301_26", 1000); // 1000 + 40 = 1040 > 1024 RAM size!
        vm.registers[2] = 0;
        vm.registers[covopt_param!("M_303_21", 3)] = 0;

        let code = [Instruction::new(OpCode::VecAdd as u8, 1, 2, covopt_param!("M_305_65", 3))];
        let res = vm.run(&code);

        assert!(
            matches!(res, Err(VmError::MemoryAccessOutOfBounds { addr: 1000, .. })),
            "VecAdd crossing RAM boundary 1024 must return MemoryAccessOutOfBounds"
        );
    }
}
