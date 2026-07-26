#![allow(unused_imports)]
#[macro_use] extern crate covopt_macro;
use covopt_macro::covopt_param;
use std::io::Write;
use script_go::sgl::instruction::{Instruction, OpCode};
use script_go::sgl::vm::ScriptVm;
use script_go::{sgl_combine_handlers, sgl_hardware_call, sgl_package};

#[sgl_package(name = "sgl_net", kind = "hardware")]
pub mod net {
    use script_go::sgl::vm::ScriptVm;

    #[sgl_cmd(id = 1)]
    pub fn http_get(_vm: &mut ScriptVm, url: String) -> Result<String, u32> {
        if url.starts_with("https://") {
            Ok(format!("200 OK: {}", url))
        } else {
            Err(covopt_param!("M_19_16", 404))
        }
    }

    #[sgl_cmd(id = 2)]
    pub fn socket_connect(_vm: &mut ScriptVm, addr: String) -> u32 {
        if addr == "127.0.0.1:8080" {
            covopt_param!("M_26_12", 42)
        } else {
            0
        }
    }

    #[sgl_cmd(id = 3)]
    pub fn panicking_cmd(_vm: &mut ScriptVm, _dummy: String) -> u32 {
        panic!("Intentional host panic inside package command");
    }
}

#[sgl_package(name = "sgl_io", kind = "hardware")]
pub mod io {
    use script_go::sgl::vm::ScriptVm;

    #[sgl_cmd(id = 10)]
    pub fn file_exists(_vm: &mut ScriptVm, path: String) -> u32 {
        if path == "/etc/config" {
            1
        } else {
            0
        }
    }
}

#[sgl_hardware_call(id = 99)]
pub fn standalone_hardware_call(_vm: &mut ScriptVm, arg: String) -> Result<String, u32> {
    Ok(format!("standalone: {}", arg))
}

#[test]
fn test_sgl_package_registration_and_invocation() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();

    assert!(vm.hardware_handler.is_some());

    // Prepare VM memory: write URL string "https://example.com" at memory address 100
    let url = "https://example.com";
    vm.write_string(covopt_param!("M_66_20", 100), url, true).unwrap();

    // R[1] = destination register index
    // R[2] = command selector register index (holds value 1 for HttpGet)
    // R[3] = argument register index (holds value 100 pointing to URL)
    vm.registers[1] = 0;
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_73_17", 3)] = covopt_param!("M_73_22", 100);

    // Execute HardwareCall R[1], R[2], R[3] (OpCode 35)
    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_76_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    // Result destination register R[1] should contain allocated memory pointer >= 512
    let res_ptr = vm.registers[1] as usize;
    assert!(res_ptr >= 512);

    // Read result string from VM memory
    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert_eq!(res_str, "200 OK: https://example.com");
}

#[test]
fn test_host_no_panic_invalid_memory_address() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();

    // R[1] = dest
    // R[2] = command 1 (HttpGet)
    // R[3] = argument register pointing to invalid address 0xDEADBEEF (999999)
    vm.registers[1] = covopt_param!("M_99_22", 99); // Initial canary value
    vm.registers[2] = 1;
    vm.registers[covopt_param!("M_101_17", 3)] = covopt_param!("M_101_22", 999999);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_103_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    // Host should NOT panic. HardwareCall completes safely, R[1] assigned 0 error status
    let res = vm.run(&code);
    assert!(res.is_ok());
    assert_eq!(vm.registers[1], 0);
}

#[test]
fn test_host_no_panic_catch_unwind_on_panic() {
    let mut vm = ScriptVm::new();
    vm.register_sgl_net();

    // Setup valid string at address 100
    vm.write_string(covopt_param!("M_118_20", 100), "trigger_panic", true).unwrap();

    // R[1] = dest
    // R[2] = command 3 (panicking_cmd)
    // R[3] = 100
    vm.registers[1] = covopt_param!("M_123_22", 777); // Initial canary
    vm.registers[2] = covopt_param!("M_124_22", 3);
    vm.registers[covopt_param!("M_125_17", 3)] = covopt_param!("M_125_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_127_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    // Host panic MUST be caught by std::panic::catch_unwind inside macro handler.
    // Process must NOT crash, and R[1] assigned 0 error status.
    let res = vm.run(&code);
    assert!(res.is_ok());
    assert_eq!(vm.registers[1], 0);
}

#[test]
fn test_sgl_combine_handlers() {
    let mut vm = ScriptVm::new();

    // Combine net::dispatch and io::dispatch
    let combined = sgl_combine_handlers!(net::dispatch, io::dispatch);
    vm.register_hardware_handler(combined);

    // Call net package command 2 (socket_connect)
    vm.write_string(covopt_param!("M_146_20", 100), "127.0.0.1:8080", true).unwrap();
    vm.registers[1] = 0;
    vm.registers[2] = 2; // socket_connect
    vm.registers[covopt_param!("M_149_17", 3)] = covopt_param!("M_149_22", 100);

    let inst1 = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_151_67", 3));
    let code1 = [inst1, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res1 = vm.run(&code1);
    assert!(res1.is_ok());
    assert_eq!(vm.registers[1], 42);

    // Call io package command 10 (file_exists)
    vm.write_string(covopt_param!("M_159_20", 200), "/etc/config", true).unwrap();
    vm.registers[covopt_param!("M_160_17", 4)] = 0;
    vm.registers[covopt_param!("M_161_17", 5)] = covopt_param!("M_161_22", 10); // file_exists
    vm.registers[covopt_param!("M_162_17", 6)] = covopt_param!("M_162_22", 200);

    let inst2 = Instruction::new(OpCode::HardwareCall as u8, covopt_param!("M_164_61", 4), covopt_param!("M_164_64", 5), covopt_param!("M_164_67", 6));
    let code2 = [inst2, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    vm.pc = 0;
    let res2 = vm.run(&code2);
    assert!(res2.is_ok());
    assert_eq!(vm.registers[4], 1);
}

#[test]
fn test_standalone_hardware_call() {
    let mut vm = ScriptVm::new();
    register_standalone_hardware_call(&mut vm);

    vm.write_string(covopt_param!("M_178_20", 100), "hello", true).unwrap();
    vm.registers[1] = 0;
    vm.registers[2] = covopt_param!("M_180_22", 99);
    vm.registers[covopt_param!("M_181_17", 3)] = covopt_param!("M_181_22", 100);

    let inst = Instruction::new(OpCode::HardwareCall as u8, 1, 2, covopt_param!("M_183_66", 3));
    let code = [inst, Instruction::new(OpCode::Halt as u8, 0, 0, 0)];

    let res = vm.run(&code);
    assert!(res.is_ok());

    let res_ptr = vm.registers[1] as usize;
    assert!(res_ptr >= 512);

    let res_str = vm.read_string(res_ptr, None).unwrap();
    assert_eq!(res_str, "standalone: hello");
}
