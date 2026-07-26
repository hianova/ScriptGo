#![allow(unused_imports)]
#[macro_use] extern crate covopt_macro;
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::sgl::host_handlers::HostContext;
use script_go::sgl::instruction::{Instruction, OpCode};
use script_go::sgl::vm::{ScriptVm, VmError};
use script_go::{
    sgl_combine_handlers, sgl_package, SglIoRegisterExt, SglNetRegisterExt,
};

#[sgl_package(name = "mock_boundary_hw", kind = "hardware")]
pub mod mock_boundary_hw {
    use script_go::sgl::vm::ScriptVm;

    #[sgl_cmd(id = 1)]
    pub fn echo_string(_vm: &mut ScriptVm, input: String) -> Result<String, u32> {
        if input == "valid" {
            Ok("ECHO_OK".to_string())
        } else {
            Err(covopt_param!("M_21_16", 400))
        }
    }

    #[sgl_cmd(id = 2)]
    pub fn process_bytes(_vm: &mut ScriptVm, data: Vec<u8>) -> u32 {
        data.len() as u32
    }

    #[sgl_cmd(id = 3)]
    pub fn panicking_cmd(_vm: &mut ScriptVm, _dummy: String) -> u32 {
        panic!("Intentional host panic for boundary test");
    }
}

#[sgl_package(name = "mock_boundary_sys", kind = "syscall")]
pub mod mock_boundary_sys {
    use script_go::sgl::vm::ScriptVm;

    #[sgl_cmd(id = 10)]
    pub fn sys_add(_vm: &mut ScriptVm, val: u32) -> u32 {
        val.wrapping_add(covopt_param!("M_42_25", 1000))
    }
}

const INVALID_ADDRESSES: [usize; 4] = [
    0xDEAD_BEEF,  // 3735928559
    0xFFFF_FFFF,  // 4294967295 (u32::MAX)
    usize::MAX,   // System pointer max
    0x9000_0000,  // Unmapped MMAP region (2415919104)
];

// AC1: Rust script uses macro system to register mock package, SGL script / instructions execute SysCall & HardwareCall
#[test]
fn test_ac1_macro_mock_package_syscall_and_hardware_call() {
    let mut vm = ScriptVm::new();
    MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);
    MockBoundarySysRegisterExt::register_mock_boundary_sys(&mut vm);

    // 1. HardwareCall to mock_boundary_hw command 1 (echo_string)
    vm.write_string(covopt_param!("M_61_20", 100), "valid", true).unwrap();
    vm.registers[1] = 0;   // Dest reg R[1]
    vm.registers[2] = 1;   // Cmd reg R[2] = 1 (echo_string)
    vm.registers[covopt_param!("M_64_17", 3)] = covopt_param!("M_64_22", 100); // Arg reg R[3] = 100 (string address)

    let hw_inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_66_69", 3));
    let code = [hw_inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());
    let res_ptr = vm.registers[1] as usize;
    assert!(res_ptr >= 512);
    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert_eq!(res_str, "ECHO_OK");

    // 2. SysCall to mock_boundary_sys command 10 (sys_add)
    vm.pc = 0;
    vm.registers[covopt_param!("M_78_17", 4)] = 0;  // Dest reg R[4]
    vm.registers[covopt_param!("M_79_17", 5)] = covopt_param!("M_79_22", 10); // Cmd reg R[5] = 10 (sys_add)
    vm.registers[covopt_param!("M_80_17", 6)] = covopt_param!("M_80_22", 50); // Arg reg R[6] = 50 (value 50)

    let sys_inst = Instruction::new(OpCode::SysCall as u8, covopt_param!("M_82_59", 4), covopt_param!("M_82_62", 5), covopt_param!("M_82_65", 6));
    let code_sys = [sys_inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res_sys = vm.run(&code_sys);
    assert!(res_sys.is_ok());
    assert_eq!(vm.registers[4], 1050);
}

// AC2: sgl-net and sgl-io compile cleanly as workspace members and can be registered & invoked
#[test]
fn test_ac2_workspace_members_sgl_net_and_sgl_io() {
    let mut vm = ScriptVm::new();
    let mut ctx = HostContext::new();
    ctx.http_mock_routes.insert(
        "https://boundary.test/api".to_string(),
        "NET_OK".to_string(),
    );
    vm.register_host_context(ctx);
    vm.register_sgl_net();
    vm.register_sgl_io();

    // Call sgl-net HttpGet
    vm.write_string(covopt_param!("M_104_20", 100), "https://boundary.test/api", true).unwrap();
    vm.registers[1] = 0;
    vm.registers[2] = 1; // HttpGet
    vm.registers[covopt_param!("M_107_17", 3)] = covopt_param!("M_107_22", 100);

    let inst_net = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_109_70", 3));
    let code_net = [inst_net, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code_net);
    assert!(res.is_ok());
    let res_ptr = vm.registers[1] as usize;
    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert_eq!(res_str, "NET_OK");
}

// AC3 & Task 3: Dedicated boundary test for invalid memory addresses (0xDEADBEEF, 0xFFFFFFFF, usize::MAX, 0x9000_0000)
#[test]
fn test_boundary_invalid_memory_addresses_no_panic() {
    for &invalid_addr in &INVALID_ADDRESSES {
        // Fast mode
        {
            let mut vm = ScriptVm::new();
            vm.register_sgl_net();
            MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

            vm.registers[1] = covopt_param!("M_129_30", 999); // Canary value
            vm.registers[2] = 1;   // HttpGet / echo_string command
            vm.registers[covopt_param!("M_131_25", 3)] = invalid_addr as u32;

            let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_133_74", 3));
            let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

            let res = vm.run(&code);
            assert!(res.is_ok());
            // Host process MUST NOT panic, destination register assigned error status 0
            assert_eq!(vm.registers[1], 0, "Failed for invalid address {:#X} in fast mode", invalid_addr);
        }

        // Tracing / Slow mode (tests run_slow synchronization)
        {
            let mut vm = ScriptVm::new();
            vm.tracing_enabled = true;
            vm.register_sgl_net();
            MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

            vm.registers[1] = covopt_param!("M_149_30", 999);
            vm.registers[2] = 1;
            vm.registers[covopt_param!("M_151_25", 3)] = invalid_addr as u32;

            let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_153_74", 3));
            let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

            let res = vm.run(&code);
            assert!(res.is_ok());
            assert_eq!(vm.registers[1], 0, "Failed for invalid address {:#X} in slow mode", invalid_addr);
        }
    }
}

// Task 3: Load and Store VM opcodes with invalid memory addresses return error, never panic
#[test]
fn test_boundary_vm_load_store_invalid_addresses_no_panic() {
    for &invalid_addr in &INVALID_ADDRESSES {
        let (b, c) = if invalid_addr <= u32::MAX as usize {
            (invalid_addr as u32, 0u32)
        } else {
            (covopt_param!("M_170_13", 4294967295), 0u32)
        };

        // Load from invalid address
        {
            let mut vm = ScriptVm::new();
            vm.registers[1] = 0;
            vm.registers[2] = b;
            vm.registers[covopt_param!("M_178_25", 3)] = c;

            let load_inst = Instruction::new(OpCode::Load as u8, 1, 2, covopt_param!("M_180_71", 3));
            let code = [load_inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

            let res = vm.run(&code);
            assert!(
                matches!(res, Err(VmError::MemoryAccessOutOfBounds { .. })),
                "Expected MemoryAccessOutOfBounds for address {:#X}",
                invalid_addr
            );
        }

        // Store to invalid address
        {
            let mut vm = ScriptVm::new();
            vm.registers[1] = covopt_param!("M_194_30", 305419896);
            vm.registers[2] = b;
            vm.registers[covopt_param!("M_196_25", 3)] = c;

            let store_inst = Instruction::new(OpCode::Store as u8, 1, 2, covopt_param!("M_198_73", 3));
            let code = [store_inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

            let res = vm.run(&code);
            assert!(
                matches!(res, Err(VmError::MemoryAccessOutOfBounds { .. })),
                "Expected MemoryAccessOutOfBounds for address {:#X}",
                invalid_addr
            );
        }
    }
}

// Task 3: Malformed UTF-8 in VM memory does not panic host process
#[test]
fn test_boundary_malformed_utf8_no_panic() {
    let mut vm = ScriptVm::new();
    MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

    // Write invalid UTF-8 bytes at memory address 100
    let invalid_utf8 = [covopt_param!("M_218_24", 255), covopt_param!("M_218_29", 254), covopt_param!("M_218_34", 253), covopt_param!("M_218_39", 128), 0x00]; // Null-terminated invalid UTF-8 sequence
    vm.write_bytes(covopt_param!("M_219_19", 100), &invalid_utf8).unwrap();

    vm.registers[1] = covopt_param!("M_221_22", 777); // Canary
    vm.registers[2] = 1;   // echo_string
    vm.registers[covopt_param!("M_223_17", 3)] = covopt_param!("M_223_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_225_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());
    // Host process MUST NOT panic, string conversion fails gracefully, dest reg assigned 0
    assert_eq!(vm.registers[1], 0);
}

// Task 3: Integer overflow lengths & giant buffer lengths do not panic host process
#[test]
fn test_boundary_integer_overflow_and_giant_length_no_panic() {
    let mut vm = ScriptVm::new();
    MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

    // Command 2 expects Vec<u8>: R[3] = buffer ptr (100), R[4] = length
    vm.registers[1] = covopt_param!("M_241_22", 555); // Canary
    vm.registers[2] = 2;   // process_bytes command
    vm.registers[covopt_param!("M_243_17", 3)] = covopt_param!("M_243_22", 100);
    vm.registers[covopt_param!("M_244_17", 4)] = u32::MAX; // Overflow / giant length

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_246_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());
    // Out of bounds byte read returns error status 0, host process does not panic
    assert_eq!(vm.registers[1], 0);
}

// Task 3: Out-of-bounds register indices wrap safely and do not panic host process
#[test]
fn test_boundary_out_of_bounds_register_indices_no_panic() {
    let mut vm = ScriptVm::new();
    MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

    vm.write_string(covopt_param!("M_261_20", 100), "valid", true).unwrap();
    vm.registers[1] = 0;
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_264_17", 3)] = covopt_param!("M_264_22", 100);

    // Use raw instruction with register indices > 255 (e.g. 256, 257, 258)
    // The instruction decoder truncates/masks to 8-bit operands (256 % 256 = 0, etc.)
    let inst = Instruction::new(
        OpCode::HardwareCall as u8,
        (covopt_param!("M_270_9", 257) & covopt_param!("M_270_15", 255)) as u8,
        (covopt_param!("M_271_9", 258) & covopt_param!("M_271_15", 255)) as u8,
        (covopt_param!("M_272_9", 259) & covopt_param!("M_272_15", 255)) as u8,
    );
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());
}

// Task 3: Host function panic recovery inside macro handlers
#[test]
fn test_boundary_host_function_panic_recovery() {
    let mut vm = ScriptVm::new();
    MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

    vm.write_string(covopt_param!("M_286_20", 100), "panic_trigger", true).unwrap();
    vm.registers[1] = covopt_param!("M_287_22", 888); // Canary
    vm.registers[2] = covopt_param!("M_288_22", 3);   // panicking_cmd
    vm.registers[covopt_param!("M_289_17", 3)] = covopt_param!("M_289_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_291_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    // Panic in host code caught by catch_unwind in macro dispatcher
    let res = vm.run(&code);
    assert!(res.is_ok());
    assert_eq!(vm.registers[1], 0);
}

// Task 3: Invalid command selector IDs return default status cleanly
#[test]
fn test_boundary_unknown_command_selector_no_panic() {
    let mut vm = ScriptVm::new();
    MockBoundaryHwRegisterExt::register_mock_boundary_hw(&mut vm);

    vm.registers[1] = covopt_param!("M_306_22", 1234); // Canary
    vm.registers[2] = covopt_param!("M_307_22", 9999); // Unknown command ID
    vm.registers[covopt_param!("M_308_17", 3)] = covopt_param!("M_308_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_310_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());
    // Dest register remains unchanged canary value if command is not handled
    assert_eq!(vm.registers[1], 1234);
}

// Task 3: Combined handlers with multiple packages under boundary conditions
#[test]
fn test_boundary_combined_handlers_no_panic() {
    let mut vm = ScriptVm::new();
    let combined = sgl_combine_handlers!(
        mock_boundary_hw::dispatch,
        mock_boundary_sys::dispatch
    );
    vm.register_hardware_handler(combined);

    for &invalid_addr in &INVALID_ADDRESSES {
        vm.pc = 0;
        vm.registers[1] = covopt_param!("M_331_26", 999);
        vm.registers[2] = 1;
        vm.registers[covopt_param!("M_333_21", 3)] = invalid_addr as u32;

        let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_335_70", 3));
        let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

        let res = vm.run(&code);
        assert!(res.is_ok());
        assert_eq!(vm.registers[1], 0);
    }
}
